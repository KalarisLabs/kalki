//! Integration tests for the WAL segment layer: framing, rotation, replay,
//! torn-write recovery and the kill -9 drill.

use std::error::Error;
use std::path::{Path, PathBuf};

use kalki_core::wal::{
    self, RotationPolicy, WalWriter, decode_record, encode_record, replay_segment,
};
use kalki_protocol::Event;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn Error>>;

fn sample_message(seq: u64) -> Event {
    Event::Message {
        ts_ms: 1_700_000_000_000,
        seq,
        role: "user".into(),
        content: format!("message {seq}"),
        payload_ref: None,
    }
}

fn sample_events() -> Vec<Event> {
    vec![
        Event::Message {
            ts_ms: 1,
            seq: 1,
            role: "user".into(),
            content: "hello".into(),
            payload_ref: Some("blob-ref-1".into()),
        },
        Event::Compaction {
            ts_ms: 2,
            upto_seq: 1,
            summary: "summary so far".into(),
        },
        Event::Lifecycle {
            ts_ms: 3,
            state: "created".into(),
        },
        Event::ToolCall {
            ts_ms: 4,
            seq: 4,
            id: "call_1".into(),
            name: "bash".into(),
            arguments_json: r#"{"command":"ls"}"#.into(),
        },
        Event::ToolResult {
            ts_ms: 5,
            seq: 5,
            id: "call_1".into(),
            handle: "deadbeef".into(),
            exit_code: Some(0),
            bytes: 42,
        },
        Event::Usage {
            ts_ms: 6,
            seq: 6,
            provider: "openai_responses".into(),
            input_tokens: 100,
            output_tokens: 20,
            cached_tokens: Some(80),
        },
        Event::Anchor {
            ts_ms: 7,
            seq: 7,
            anchor_hash: "abc123def456".into(),
        },
    ]
}

fn segment_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut segs: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("wal"))
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("seg-"))
        })
        .collect();
    segs.sort();
    Ok(segs)
}

fn single_segment(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let mut segs = segment_files(dir)?;
    if segs.len() != 1 {
        return Err(format!("expected exactly one segment, found {}", segs.len()).into());
    }
    Ok(segs.remove(0))
}

fn assert_segment_name(path: &Path) -> Result<u64, Box<dyn Error>> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("segment file name is not utf-8")?;
    let stem = name
        .strip_prefix("seg-")
        .and_then(|s| s.strip_suffix(".wal"))
        .ok_or_else(|| format!("segment name {name} does not match seg-<seq_first:020>.wal"))?;
    if stem.len() != 20 || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("segment name {name} is not seg-<seq_first:020>.wal").into());
    }
    Ok(stem.parse()?)
}

fn replay_all(dir: &Path) -> Result<Vec<Event>, Box<dyn Error>> {
    let mut events = Vec::new();
    for seg in segment_files(dir)? {
        events.extend(replay_segment(&seg)?);
    }
    Ok(events)
}

#[test]
fn roundtrips_every_event_variant_through_framing() -> TestResult {
    for event in sample_events() {
        let frame = encode_record(&event)?;
        let (decoded, consumed) = decode_record(&frame)?;
        assert_eq!(decoded, event);
        assert_eq!(consumed, frame.len());
    }
    Ok(())
}

#[test]
fn decodes_concatenated_records_at_successive_offsets() -> TestResult {
    let events = sample_events();
    let mut blob = Vec::new();
    for event in &events {
        blob.extend_from_slice(&encode_record(event)?);
    }
    let mut offset = 0usize;
    for expected in &events {
        let (decoded, consumed) = decode_record(&blob[offset..])?;
        assert_eq!(&decoded, expected);
        offset += consumed;
    }
    assert_eq!(offset, blob.len());
    Ok(())
}

#[test]
fn rejects_record_when_length_prefix_exceeds_buffer() -> TestResult {
    let frame = encode_record(&sample_message(1))?;
    let truncated = &frame[..frame.len() - 5];
    assert!(decode_record(truncated).is_err());
    Ok(())
}

#[test]
fn rejects_record_when_payload_crc_mismatches() -> TestResult {
    let mut frame = encode_record(&sample_message(1))?;
    let last = frame.len() - 1;
    frame[last] ^= 0x01;
    assert!(decode_record(&frame).is_err());

    let mut bad_crc = frame.clone();
    bad_crc[0] ^= 0xff;
    assert!(decode_record(&bad_crc).is_err());
    Ok(())
}

#[test]
fn truncates_torn_tail_when_last_record_byte_flips() -> TestResult {
    let dir = tempdir()?;
    let events: Vec<Event> = (1..=5).map(sample_message).collect();
    {
        let mut writer = WalWriter::open(dir.path())?;
        for event in &events {
            writer.append(event)?;
        }
    }
    let seg = single_segment(dir.path())?;
    let mut bytes = std::fs::read(&seg)?;
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    std::fs::write(&seg, &bytes)?;

    {
        let _writer = WalWriter::open(dir.path())?; // scan + truncate on open
    }
    let replayed: Vec<Event> = replay_segment(&seg)?.collect();
    assert_eq!(replayed, events[..4]);
    let expected_len: u64 = events[..4]
        .iter()
        .map(|e| encode_record(e).map(|f| f.len() as u64))
        .sum::<Result<u64, _>>()?;
    assert_eq!(std::fs::metadata(&seg)?.len(), expected_len);
    Ok(())
}

#[test]
fn rotates_segment_when_size_exceeds_policy_threshold() -> TestResult {
    let dir = tempdir()?;
    let policy = RotationPolicy {
        max_segment_bytes: 128,
    };
    let events: Vec<Event> = (1..=8).map(sample_message).collect();
    let mut writer = WalWriter::open_with_policy(dir.path(), policy)?;
    for event in &events {
        writer.append(event)?;
    }
    drop(writer);

    let segs = segment_files(dir.path())?;
    assert!(segs.len() >= 2, "expected rotation into multiple segments");
    let seq_firsts: Vec<u64> = segs
        .iter()
        .map(|p| assert_segment_name(p))
        .collect::<Result<_, _>>()?;
    assert!(seq_firsts.windows(2).all(|w| w[0] < w[1]));

    let replayed = replay_all(dir.path())?;
    assert_eq!(replayed, events);
    Ok(())
}

#[test]
fn replay_stops_at_first_corruption_when_tail_garbage_appended() -> TestResult {
    let dir = tempdir()?;
    let events: Vec<Event> = (1..=3).map(sample_message).collect();
    let mut bytes = Vec::new();
    for event in &events {
        bytes.extend_from_slice(&encode_record(event)?);
    }
    let valid_len = u64::try_from(bytes.len())?;
    bytes.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0x00]);
    let seg = dir.path().join("seg-00000000000000000000.wal");
    std::fs::write(&seg, &bytes)?;

    let replayed: Vec<Event> = replay_segment(&seg)?.collect();
    assert_eq!(replayed, events);
    // Replay is read-only; the garbage is still on disk for the open-time repair.
    assert_eq!(std::fs::metadata(&seg)?.len(), valid_len + 5);
    Ok(())
}

#[test]
fn reopen_recovers_written_records_when_writer_dropped_without_close() -> TestResult {
    let dir = tempdir()?;
    let events: Vec<Event> = (1..=4).map(sample_message).collect();
    {
        let mut writer = WalWriter::open(dir.path())?;
        for event in &events {
            writer.append(event)?;
        }
    } // drop without any close; fsync happens per append
    let reopened = WalWriter::open(dir.path())?;
    drop(reopened);
    let replayed = replay_all(dir.path())?;
    assert_eq!(replayed, events);
    Ok(())
}

#[test]
fn kill_minus9_drill_recovers_valid_prefix_when_tail_torn() -> TestResult {
    let dir = tempdir()?;
    let events: Vec<Event> = (1..=10).map(sample_message).collect();
    {
        let mut writer = WalWriter::open(dir.path())?;
        for event in &events {
            writer.append(event)?;
        }
    } // simulated kill -9: no clean shutdown, no extra sync ceremony

    // Simulate the torn write: only half of an 11th record survives.
    let torn = encode_record(&sample_message(11))?;
    let seg = single_segment(dir.path())?;
    let mut bytes = std::fs::read(&seg)?;
    bytes.extend_from_slice(&torn[..torn.len() / 2]);
    std::fs::write(&seg, &bytes)?;

    let extra: Vec<Event> = (0..2).map(|i| sample_message(100 + i)).collect();
    {
        let mut writer = WalWriter::open(dir.path())?; // repair: truncate the torn tail
        for event in &extra {
            writer.append(event)?;
        }
    }
    let mut expected = events;
    expected.extend(extra);
    let replayed = replay_all(dir.path())?;
    assert_eq!(replayed, expected);
    Ok(())
}

#[test]
fn replay_yields_committed_prefix_across_segments_when_active_tail_corrupts() -> TestResult {
    let dir = tempdir()?;
    let policy = RotationPolicy {
        max_segment_bytes: 128,
    };
    let events: Vec<Event> = (1..=8).map(sample_message).collect();
    {
        let mut writer = WalWriter::open_with_policy(dir.path(), policy)?;
        for event in &events {
            writer.append(event)?;
        }
    }
    // Corrupt the final record of the final (active) segment.
    let last_seg = segment_files(dir.path())?
        .pop()
        .ok_or("no segments written")?;
    let mut bytes = std::fs::read(&last_seg)?;
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    std::fs::write(&last_seg, &bytes)?;

    let _writer = WalWriter::open(dir.path())?; // repairs only the active segment
    let replayed = replay_all(dir.path())?;
    assert_eq!(replayed, events[..events.len() - 1]);
    Ok(())
}

#[test]
fn wal_module_exposes_documented_format_constants() {
    // Guards the on-disk contract so accidental layout changes fail loudly.
    assert_eq!(wal::CRC_BYTES, 4);
    assert_eq!(wal::LEN_BYTES, 8);
    assert_eq!(wal::FRAME_HEADER_BYTES, 12);
    assert_eq!(wal::DEFAULT_SEGMENT_MAX_BYTES, 64 * 1024 * 1024);
}
