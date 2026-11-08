//! Built-in tools (bash, apply_patch). Untrusted execution behind the policy gate.
//! v0.2: bash with argv-array spawn (no shell interpolation), 4 KiB inline cap, output to blob store.

use serde::{Deserialize, Serialize};

/// The execution context a tool needs (workspace root, blob dir, policy).
#[derive(Clone)]
pub struct ToolContext {
    /// Absolute path of the session workspace (tools may not escape it).
    pub workspace: std::path::PathBuf,
    /// Blob store directory.
    pub blob_dir: std::path::PathBuf,
    /// Inline cap for tool output; overflow goes to the blob store with a `payload_ref`.
    pub inline_cap: usize,
}

/// Arguments to the bash tool (argv array; no shell string by design).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashArgs {
    /// Program + args; argv[0] is the program. Never a shell string — models must not get implicit `sh -c`.
    pub argv: Vec<String>,
    /// Timeout in seconds (hard cap 120; default 30).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

/// Default timeout for a bash invocation.
fn default_timeout() -> u64 {
    30
}

/// Result of a tool invocation (content destiny: inline or blob).
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Inline text (truncated if large).
    pub inline: String,
    /// Blob handle when `bytes` exceeds the inline cap.
    pub blob_handle: Option<String>,
    /// Exit code for processes.
    pub exit_code: Option<i64>,
    /// Total bytes produced.
    pub bytes: u64,
    /// Wall time spent.
    pub elapsed_ms: u64,
}

/// Tool failure (not a model-visible error string; the session loop decides what to surface).
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Spawn failure.
    #[error("spawn: {0}")]
    Spawn(#[from] std::io::Error),
    /// Policy denied the call.
    #[error("policy: {0}")]
    Policy(String),
    /// Timeout hit.
    #[error("timeout after {0}s")]
    Timeout(u64),
    /// Workspace confinement violation.
    #[error("confinement: {0}")]
    Confinement(String),
}

/// What happened in a tool run — checkpointed into the WAL by the session loop.
#[derive(Debug, Clone)]
pub struct ToolExecution {
    /// stdout+stderr merged, capped inline.
    pub output: ToolOutput,
    /// True when output was truncated and the remainder went to the blob store.
    pub truncated_to_blob: bool,
}
