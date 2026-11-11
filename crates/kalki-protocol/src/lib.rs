//! Wire and event types shared by every Kalki process.
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
    /// A tool-call intent issued by the model.
    #[serde(rename = "4")]
    ToolCall {
        /// Milliseconds since epoch, UTC.
        ts_ms: u64,
        /// Monotonic sequence number within the session.
        seq: u64,
        /// Call id (provider-assigned or generated).
        id: String,
        /// Tool name (e.g. `bash`, `apply_patch`).
        name: String,
        /// Raw JSON arguments as sent by the model.
        arguments_json: String,
    },
    /// The result of a tool call (or its outbound blob reference).
    #[serde(rename = "5")]
    ToolResult {
        /// Milliseconds since epoch, UTC.
        ts_ms: u64,
        /// Monotonic sequence number within the session.
        seq: u64,
        /// Matching `ToolCall::id`.
        id: String,
        /// Content-addressed handle for the full output (blob store).
        handle: String,
        /// Process exit code when applicable (shell tools).
        exit_code: Option<i64>,
        /// Total output size in bytes.
        bytes: u64,
    },
    /// Provider token usage for a model call (cost accounting).
    #[serde(rename = "6")]
    Usage {
        /// Milliseconds since epoch, UTC.
        ts_ms: u64,
        /// Monotonic sequence number within the session.
        seq: u64,
        /// Provider wire used (`openai_responses` | `anthropic_messages` | `openai_compat`).
        provider: String,
        /// Prompt/input tokens consumed (post-breakpoint for cached models).
        input_tokens: u64,
        /// Completion/output tokens generated.
        output_tokens: u64,
        /// Tokens served from the provider prompt cache, when reported.
        cached_tokens: Option<u64>,
    },
    /// Snapshot anchor marker written when a checkpoint snapshot lands.
    #[serde(rename = "7")]
    Anchor {
        /// Milliseconds since epoch, UTC.
        ts_ms: u64,
        /// Sequence number covered by the snapshot at this point.
        seq: u64,
        /// Hex SHA-256 of the snapshot's canonical bytes (see `kalki-core::snapshot`).
        anchor_hash: String,
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
