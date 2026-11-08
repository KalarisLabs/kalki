//! Append-only event log (hash-chained, length-prefixed, CRC-checked).
//!
//! v0.1 ships the record framing; segment rotation, snapshots and replay land with the
//! WAL design in `docs/ARCHITECTURE.md` before the first tagged release.

use kalki_protocol::{Event, WalHeader};

use crate::{CoreError, Result};

/// Append a single event to a WAL segment writer.
///
/// Framing: `[8-byte WalHeader.len LE][32-byte prev_hash][8-byte ts_ms LE][payload][4-byte CRC32C]`.
/// This prototype writes the header + payload; CRC and hash-chain are wired in the next milestone.
///
/// # Errors
/// Returns an error if serialization or writing fails.
pub fn encode_record(event: &Event, prev_hash: [u8; 32]) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(event)?;
    let header = WalHeader {
        prev_hash,
        len: payload.len() as u64,
    };
    let mut out = Vec::with_capacity(32 + 8 + payload.len());
    out.extend_from_slice(&header.prev_hash);
    out.extend_from_slice(&header.len.to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Decode a record produced by [`encode_record`]. Returns the event and the next `prev_hash`.
///
/// # Errors
/// Returns an error if the bytes are truncated or the payload fails to deserialize.
pub fn decode_record(bytes: &[u8]) -> Result<(Event, [u8; 32])> {
    if bytes.len() < 40 {
        return Err(CoreError::WalCorruption {
            offset: 0,
            reason: "record shorter than header".into(),
        });
    }
    let mut prev_hash = [0u8; 32];
    prev_hash.copy_from_slice(&bytes[..32]);
    let raw_len = u64::from_le_bytes(bytes[32..40].try_into().expect("slice is 8 bytes"));
    let len = usize::try_from(raw_len).map_err(|_| CoreError::WalCorruption {
        offset: 0,
        reason: "record length exceeds address space".into(),
    })?;
    if bytes.len() < 40 + len {
        return Err(CoreError::WalCorruption {
            offset: 0,
            reason: "truncated payload".into(),
        });
    }
    let event: Event = serde_json::from_slice(&bytes[40..40 + len])?;
    Ok((event, prev_hash))
}
