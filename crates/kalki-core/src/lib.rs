//! Kalki Code harness core. No UI in here — the TUI talks to this over the protocol.

#![forbid(unsafe_code)]

pub mod blob;
/// Policy engine (deny-first, TOML).
pub mod policy;
/// Provider wires (OpenAI Responses / Anthropic Messages / OpenAI-compatible).
pub mod provider;
/// Session FSM (one-shot headless runs).
pub mod session;
/// Snapshot checkpoint anchors.
pub mod snapshot;
/// Built-in tools (bash, apply_patch).
pub mod tools;
pub mod wal;

pub use kalki_protocol::Event;

/// Core crate errors.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// The event log is corrupt (bad length-prefix or CRC mismatch).
    #[error("wal corruption detected at offset {offset}: {reason}")]
    WalCorruption {
        /// Byte offset at which corruption was detected.
        offset: u64,
        /// Human-readable reason.
        reason: String,
    },
    /// I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Provider layer failure (config, wire, stream). Never the WAL's fault.
    #[error("provider error: {0}")]
    Provider(String),
    /// Invalid or missing user configuration (policy file, env vars).
    #[error("configuration error: {0}")]
    Config(String),
    /// JSON (de)serialization failure.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result alias for the core crate.
pub type Result<T> = std::result::Result<T, CoreError>;
