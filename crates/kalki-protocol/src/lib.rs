//! Wire and event types shared by every Kalki Code process.
//!
//! Invariants:
//! - Every durable event is an [`Event`], serialized with `#[serde(tag = "v")]`.
//! - Content addressing: large payloads live in the blob store; events carry `payload_ref`.
//! - This crate has **no** internal dependents and depends only on `serde`/`thiserror`.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Versioned envelope for every append-only log record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "v")]
pub enum Event {
    /// A user or system message entered the trajectory.
    #[serde(rename = "1")]
    Message {
        /// Milliseconds since epoch, UTC.
        ts_ms: u64,
        /// Monotonic sequence number within the session.
        seq: u64,
        /// "user" | "assistant" | "tool" | "system"
        role: String,
        /// UTF-8 content as sent; large payloads use `payload_ref` instead.
        content: String,
        /// Content-addressed blob reference for large payloads (out-of-band storage).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_ref: Option<String>,
    },
    /// A compaction boundary. The full-fidelity record still exists in the log.
    #[serde(rename = "2")]
    Compaction {
        /// Milliseconds since epoch, UTC.
        ts_ms: u64,
        /// Sequence number the compaction covered up to.
        upto_seq: u64,
        /// Opaque summary produced for the model-facing cache.
        summary: String,
    },
    /// Session lifecycle marker.
    #[serde(rename = "3")]
    Lifecycle {
        /// Milliseconds since epoch, UTC.
        ts_ms: u64,
        /// "created" | "attached" | "detached" | `crash_recovered` | "closed"
        state: String,
    },
}

/// Hash-chained record header for the audit/WAL layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalHeader {
    /// SHA-256 of the previous record's canonical bytes (`prev_hash` of genesis = all-zero).
    pub prev_hash: [u8; 32],
    /// Length in bytes of the payload that follows the header.
    pub len: u64,
}

/// Events are hashed over their canonical JSON bytes; see `kalki-core::wal`.
pub const GENESIS_PREV_HASH: [u8; 32] = [0u8; 32];
