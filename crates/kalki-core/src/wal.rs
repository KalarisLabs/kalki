//! Append-only, CRC32C-framed event log — the only source of durable truth.
//!
//! # Record layout (little-endian)
//!
//! ```text
//! [u32 crc32c][u64 len][payload]
//! ```
//!
//! `crc32c` covers `len || payload`; `payload` is the `serde_json` encoding of a
//! [`kalki_protocol::Event`]. Records pack into segment files named
//! `seg-<seq_first:020>.wal` (`seq_first` = sequence number of the first record).
//! A segment rotates once appending the next record would grow it past
//! [`RotationPolicy::max_segment_bytes`].
//!
//! Only the *active* (latest) segment is repaired on open: [`WalWriter::open`]
//! scans it, verifies every record's CRC and truncates at the first bad or torn
//! record. Older segments are treated as sealed; [`replay_segment`] stops at the
//! first corruption instead of failing the whole read.

// v0.3: hash chain — each record will carry the SHA-256 of the previous
// v0.3: record's canonical bytes; GENESIS_PREV_HASH anchors the first record.
// v0.3: Batched fsync (group commit) lands with the chain; v0.2 syncs per record.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use kalki_protocol::Event;

use crate::{CoreError, Result};

/// Size of the CRC32C prefix, in bytes.
pub const CRC_BYTES: usize = 4;
/// Size of the little-endian length prefix, in bytes.
pub const LEN_BYTES: usize = 8;
/// Total framing overhead per record, in bytes.
pub const FRAME_HEADER_BYTES: usize = CRC_BYTES + LEN_BYTES;
/// Default rotation threshold: 64 MiB.
pub const DEFAULT_SEGMENT_MAX_BYTES: u64 = 64 * 1024 * 1024;

const SEGMENT_PREFIX: &str = "seg-";
const SEGMENT_EXT: &str = "wal";
const SEQ_FIRST_DIGITS: usize = 20;

/// Encode `event` into a CRC-framed record.
///
/// # Errors
/// Returns [`CoreError::Json`] if serialization fails. Structurally impossible for
/// [`Event`] today (string keys only, no custom serializers), but the audit log
/// must never silently emit an empty frame — the error propagates instead.
pub fn encode_record(event: &Event) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(event)?;
    let len = payload.len() as u64;
    let mut crc_input = Vec::with_capacity(LEN_BYTES + payload.len());
    crc_input.extend_from_slice(&len.to_le_bytes());
    crc_input.extend_from_slice(&payload);
    // The CRC covers the length prefix as well as the payload, so a torn or
    // garbage length field is detected, not just a damaged payload.
    let crc = crc32c::crc32c(&crc_input);

    let mut out = Vec::with_capacity(FRAME_HEADER_BYTES + payload.len());
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Decode one record from the head of `bytes`.
///
/// Returns the decoded event and the number of bytes consumed, so callers can
/// step through concatenated records.
///
/// # Errors
/// [`CoreError::WalCorruption`] on a short buffer, an oversized length prefix, a
/// truncated payload or a CRC mismatch; [`CoreError::Json`] if the payload is not
/// a valid [`Event`] despite a matching CRC.
pub fn decode_record(bytes: &[u8]) -> Result<(Event, usize)> {
    if bytes.len() < FRAME_HEADER_BYTES {
        return Err(CoreError::WalCorruption {
            offset: 0,
            reason: format!("record shorter than frame header ({} bytes)", bytes.len()),
        });
    }
    let mut crc_bytes = [0u8; CRC_BYTES];
    crc_bytes.copy_from_slice(&bytes[..CRC_BYTES]);
    let stored_crc = u32::from_le_bytes(crc_bytes);
    let mut len_bytes = [0u8; LEN_BYTES];
    len_bytes.copy_from_slice(&bytes[CRC_BYTES..FRAME_HEADER_BYTES]);
    let payload_len =
        usize::try_from(u64::from_le_bytes(len_bytes)).map_err(|_| CoreError::WalCorruption {
            offset: 0,
            reason: "record length exceeds address space".into(),
        })?;
    let frame_len =
        FRAME_HEADER_BYTES
            .checked_add(payload_len)
            .ok_or_else(|| CoreError::WalCorruption {
                offset: 0,
                reason: "record length overflows usize".into(),
            })?;
    if bytes.len() < frame_len {
        return Err(CoreError::WalCorruption {
            offset: 0,
            reason: format!(
                "truncated payload: need {payload_len} bytes, have {}",
                bytes.len() - FRAME_HEADER_BYTES
            ),
        });
    }
    let computed_crc = crc32c::crc32c(&bytes[CRC_BYTES..frame_len]);
    if computed_crc != stored_crc {
        return Err(CoreError::WalCorruption {
            offset: 0,
            reason: format!(
                "crc mismatch: stored {stored_crc:#010x}, computed {computed_crc:#010x}"
            ),
        });
    }
    let event: Event = serde_json::from_slice(&bytes[FRAME_HEADER_BYTES..frame_len])?;
    Ok((event, frame_len))
}

/// Sequence number carried by `Message`, `ToolCall`, `ToolResult`, `Usage` and
/// `Anchor`. `Lifecycle` and `Compaction` carry none.
fn event_seq(event: &Event) -> Option<u64> {
    match event {
        Event::Message { seq, .. }
        | Event::ToolCall { seq, .. }
        | Event::ToolResult { seq, .. }
        | Event::Usage { seq, .. }
        | Event::Anchor { seq, .. } => Some(*seq),
        Event::Compaction { .. } | Event::Lifecycle { .. } => None,
    }
}

fn segment_file_name(seq_first: u64) -> String {
    format!("{SEGMENT_PREFIX}{seq_first:020}.{SEGMENT_EXT}")
}

fn parse_segment_file_name(name: &str) -> Option<u64> {
    let stem = name
        .strip_prefix(SEGMENT_PREFIX)?
        .strip_suffix(&format!(".{SEGMENT_EXT}"))?;
    if stem.len() != SEQ_FIRST_DIGITS || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

fn list_segments(dir: &Path) -> Result<Vec<(u64, PathBuf)>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if let Some(seq) = parse_segment_file_name(name) {
            out.push((seq, path));
        }
    }
    Ok(out)
}

struct ScanOutcome {
    valid_len: u64,
    max_seq: u64,
    record_count: u64,
    seen_seq: bool,
}

/// Verify every record in `path`; truncate at the first bad or torn record.
fn scan_segment(path: &Path) -> Result<ScanOutcome> {
    let bytes = std::fs::read(path)?;
    let mut offset = 0usize;
    let mut outcome = ScanOutcome {
        valid_len: 0,
        max_seq: 0,
        record_count: 0,
        seen_seq: false,
    };
    while offset < bytes.len() {
        match decode_record(&bytes[offset..]) {
            Ok((event, consumed)) => {
                if let Some(seq) = event_seq(&event) {
                    outcome.max_seq = outcome.max_seq.max(seq);
                    outcome.seen_seq = true;
                }
                offset += consumed;
                outcome.record_count += 1;
            }
            Err(err) => {
                tracing::warn!(
                    path = %path.display(),
                    offset,
                    error = %err,
                    "wal: truncating torn tail at offset"
                );
                let file = OpenOptions::new().write(true).open(path)?;
                file.set_len(offset as u64)?;
                file.sync_all()?;
                break;
            }
        }
    }
    outcome.valid_len = offset as u64;
    Ok(outcome)
}

/// Rotation threshold for WAL segments.
///
/// Construct explicitly (or via [`Default`]) to override the 64 MiB default;
/// tests use a tiny `max_segment_bytes` to exercise rotation cheaply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationPolicy {
    /// Rotate the active segment once appending the next record would grow it
    /// past this many bytes. A single record larger than the threshold still
    /// lands in a fresh segment; segments never go empty by rotation.
    pub max_segment_bytes: u64,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            max_segment_bytes: DEFAULT_SEGMENT_MAX_BYTES,
        }
    }
}

struct ActiveSegment {
    path: PathBuf,
    size: u64,
    file: File,
}

/// Append-only writer over a WAL directory.
///
/// fsyncs after every record (v0.2). The active segment is scanned and any torn
/// tail truncated on open, so a crash between `write` and `fsync` never leaves
/// unverifiable bytes visible to replay.
pub struct WalWriter {
    dir: PathBuf,
    policy: RotationPolicy,
    active: Option<ActiveSegment>,
    next_seq_hint: u64,
}

impl WalWriter {
    /// Open (or create) the WAL directory at `dir` with the default policy.
    ///
    /// # Errors
    /// I/O failures on directory listing, segment scan/repair or file open.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_policy(dir, RotationPolicy::default())
    }

    /// Like [`WalWriter::open`], with an explicit [`RotationPolicy`].
    ///
    /// # Errors
    /// I/O failures on directory listing, segment scan/repair or file open.
    pub fn open_with_policy(dir: impl AsRef<Path>, policy: RotationPolicy) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        let mut segments = list_segments(&dir)?;
        segments.sort_by_key(|(seq, _)| *seq);
        let Some((_, path)) = segments.pop() else {
            return Ok(Self {
                dir,
                policy,
                active: None,
                next_seq_hint: 0,
            });
        };
        let scan = scan_segment(&path)?;
        let file = OpenOptions::new().append(true).open(&path)?;
        let next_seq_hint = if scan.seen_seq {
            scan.max_seq.saturating_add(1)
        } else {
            scan.record_count
        };
        Ok(Self {
            dir,
            policy,
            active: Some(ActiveSegment {
                path,
                size: scan.valid_len,
                file,
            }),
            next_seq_hint,
        })
    }

    /// Append `event` to the active segment, fsyncing before returning.
    ///
    /// Rotates to a fresh segment when the current one would exceed the policy.
    ///
    /// # Errors
    /// I/O, serialization or rotation failures.
    pub fn append(&mut self, event: &Event) -> Result<()> {
        let encoded = encode_record(event)?;
        let record_len = encoded.len() as u64;
        let seq = event_seq(event).unwrap_or(self.next_seq_hint);
        self.next_seq_hint = self.next_seq_hint.max(seq.saturating_add(1));

        self.rotate_if_needed(seq, record_len)?;
        if self.active.is_none() {
            self.start_segment(seq)?;
        }
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| CoreError::WalCorruption {
                offset: 0,
                reason: "failed to establish active segment".into(),
            })?;
        active.file.write_all(&encoded)?;
        active.file.sync_all()?;
        active.size += record_len;
        Ok(())
    }

    /// Path of the segment records are currently appended to, if any.
    pub fn current_segment(&self) -> Option<&Path> {
        self.active.as_ref().map(|active| active.path.as_path())
    }

    fn rotate_if_needed(&mut self, next_seq: u64, record_len: u64) -> Result<()> {
        let rotate = self.active.as_ref().is_some_and(|active| {
            active.size > 0
                && active.size.saturating_add(record_len) > self.policy.max_segment_bytes
        });
        if rotate {
            self.start_segment(next_seq)?;
        }
        Ok(())
    }

    fn start_segment(&mut self, seq_first: u64) -> Result<()> {
        let path = self.dir.join(segment_file_name(seq_first));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        // An existing file from a crashed run is adopted, never truncated.
        let size = std::fs::metadata(&path)?.len();
        self.active = Some(ActiveSegment { path, size, file });
        Ok(())
    }
}

/// Replay `path`, yielding decoded events until the first corruption or end of file.
///
/// Corruption is not an error: the iterator simply ends after the valid prefix,
/// which is what crash recovery wants.
///
/// # Errors
/// Returns an error if the file cannot be read.
pub fn replay_segment(path: &Path) -> Result<impl Iterator<Item = Event>> {
    let bytes = std::fs::read(path)?;
    let mut events = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        match decode_record(&bytes[offset..]) {
            Ok((event, consumed)) => {
                events.push(event);
                offset += consumed;
            }
            Err(err) => {
                tracing::warn!(
                    path = %path.display(),
                    offset,
                    error = %err,
                    "wal: replay stopped at corruption"
                );
                break;
            }
        }
    }
    Ok(events.into_iter())
}
