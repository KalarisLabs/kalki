//! Session FSM: attach → turn loop (model → tools → WAL events) → close.
//! v0.2: one-shot headless run used by `kalki run -p`. Daemon/reattach: v0.2.x work.

use serde::{Deserialize, Serialize};

use crate::policy::PolicyEngine;
use crate::provider::Provider;

/// Lifecycle states of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Turns are being processed.
    Running,
    /// Nothing in flight; awaiting user or schedule.
    Idle,
    /// Terminated cleanly.
    Closed,
    /// Reattached after a worker crash (boot verified anchor + tail).
    CrashRecovered,
}

/// A persisted session handle (kept small; heavy state lives in the WAL/snapshots).
#[derive(Debug, Clone)]
pub struct Session {
    /// Session identifier (ULID-class, 26 chars).
    pub id: String,
    /// Current sequence number (WAL seq).
    pub seq: u64,
    /// Lifecycle state.
    pub state: SessionState,
}

/// Inputs for one-shot harness runs.
pub struct RunOptions<P: Provider> {
    /// Workspace root.
    pub workspace: std::path::PathBuf,
    /// Provider implementation to use.
    pub provider: P,
    /// Policy engine for tool gating.
    pub policy: PolicyEngine,
    /// Model identifier.
    pub model: String,
    /// User objective text.
    pub objective: String,
    /// Hard cap on turns (defense against runaway loops; default 200).
    pub max_turns: usize,
}

/// Outcome of a one-shot run.
#[derive(Debug)]
pub struct RunOutcome {
    /// Final assistant text (if the model finished).
    pub final_text: String,
    /// Number of model turns executed.
    pub turns: usize,
    /// True if the loop hit the turn cap or an error aborted it.
    pub aborted: bool,
}

/// The session loop. v0.2 implementation task P7 — see docs/ARCHITECTURE.md.
///
/// # Errors
/// Always in v0.2 (the loop lands with task P7); typed so the stub stays honest.
pub fn run_one_shot<P: Provider>(_opts: &RunOptions<P>) -> Result<RunOutcome, crate::CoreError> {
    Err(crate::CoreError::WalCorruption {
        offset: 0,
        reason: "session loop not implemented yet (task P7)".to_string(),
    })
}
