//! Session FSM: attach → turn loop (model → tools → WAL events) → close.
//! v0.2: one-shot headless run used by `kalki run -p`. Daemon/reattach: v0.2.x work.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::CoreError;
use crate::policy::PolicyEngine;
use crate::provider::{AnyProvider, Provider, ProviderStream};
use crate::wal;

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
    /// Session identifier.
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
    /// Session id (use it to locate the WAL on disk).
    pub session_id: String,
    /// Final assistant text (if the model finished).
    pub final_text: String,
    /// Number of model turns executed.
    pub turns: usize,
    /// True if the loop hit the turn cap or an error aborted it.
    pub aborted: bool,
}

const SYSTEM_PROMPT: &str = "You are Kalki Code, a terminal coding harness running headless. \
You operate through tools only: bash (argv array, no shell strings) and apply_patch. \
When done, answer plainly and stop calling tools. Say what you changed, what you ran, what you did NOT verify.";

const BLOB_INLINE_CAP: usize = 4 * 1024;

#[derive(Default)]
struct ToolAccumulator {
    /// Accumulating tool calls keyed by the delta id as delivered by the wire.
    calls: HashMap<String, AccumCall>,
    /// Arrival order so `idx:N` deltas can be matched to the N-th pending call.
    order: Vec<String>,
}

#[derive(Default)]
struct AccumCall {
    name: String,
    args: String,
}

impl ToolAccumulator {
    fn new() -> Self {
        Self::default()
    }

    fn start(&mut self, id: String, name: String) {
        self.calls.insert(
            id.clone(),
            AccumCall {
                name,
                args: String::new(),
            },
        );
        self.order.push(id);
    }

    fn args_delta(&mut self, id: &str, delta: &str) {
        let key = resolve_key(id, &self.order);
        if let Some(call) = self.calls.get_mut(&key) {
            call.args.push_str(delta);
        }
    }

    fn drain_completed(&mut self) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for key in std::mem::take(&mut self.order) {
            if let Some(call) = self.calls.remove(&key)
                && !call.name.is_empty()
            {
                out.push((key, call.name, call.args));
            }
        }
        out
    }
}

fn resolve_key(id: &str, order: &[String]) -> String {
    if let Some(index) = id.strip_prefix("idx:")
        && let Ok(n) = index.parse::<usize>()
        && let Some(key) = order.get(n)
    {
        return key.clone();
    }
    id.to_string()
}

/// Parsed result of one consumed provider stream.
struct TurnArtifacts {
    text: String,
    usage_in: u64,
    usage_out: u64,
    usage_cached: Option<u64>,
    calls: Vec<(String, String, String)>,
}

/// Consume one provider stream to completion, collecting text, usage and tool calls.
///
/// # Errors
/// Returns the stream error (caller writes the WAL abort marker).
async fn consume_turn(stream: &mut ProviderStream) -> Result<TurnArtifacts, String> {
    let mut acc = ToolAccumulator::new();
    let mut text = String::new();
    let mut usage_in = 0u64;
    let mut usage_out = 0u64;
    let mut usage_cached: Option<u64> = None;

    loop {
        match stream.next_event().await {
            None => break,
            Some(Err(e)) => return Err(e.to_string()),
            Some(Ok(ev)) => match ev {
                crate::provider::ProviderEvent::TextDelta(t) => text.push_str(&t),
                crate::provider::ProviderEvent::ToolCallStart { id: call_id, name } => {
                    acc.start(call_id, name);
                }
                crate::provider::ProviderEvent::ToolCallArgsDelta { id: call_id, delta } => {
                    acc.args_delta(&call_id, &delta);
                }
                crate::provider::ProviderEvent::Usage {
                    input_tokens,
                    output_tokens,
                    cached_tokens,
                } => {
                    usage_in += input_tokens;
                    usage_out += output_tokens;
                    usage_cached = cached_tokens.or(usage_cached);
                }
                crate::provider::ProviderEvent::Finish { .. } => break,
                crate::provider::ProviderEvent::ReasoningDelta(_) => {}
            },
        }
    }

    Ok(TurnArtifacts {
        text,
        usage_in,
        usage_out,
        usage_cached,
        calls: acc.drain_completed(),
    })
}

fn tool_specs() -> Vec<crate::provider::ToolSpec> {
    vec![
        crate::provider::ToolSpec {
            name: "bash".to_string(),
            description: "Run a program with argv array (no shell). Output capped at 4KiB inline.".to_string(),
            parameters_json_schema: r#"{"type":"object","properties":{"argv":{"type":"array","items":{"type":"string"}},"timeout_secs":{"type":"integer"}},"required":["argv"],"additionalProperties":false}"#
                .to_string(),
        },
        crate::provider::ToolSpec {
            name: "apply_patch".to_string(),
            description: "Apply a unified diff to files in the workspace.".to_string(),
            parameters_json_schema: r#"{"type":"object","properties":{"patch":{"type":"string"},"dry_run":{"type":"boolean"}},"required":["patch"],"additionalProperties":false}"#
                .to_string(),
        },
    ]
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn new_session_id() -> String {
    use std::fmt::Write;
    let mut id = String::with_capacity(26);
    for b in now_ms().to_le_bytes() {
        write!(id, "{b:02x}").expect("write to String cannot fail");
    }
    id.push_str("-kalki");
    id
}

fn sessions_dir() -> PathBuf {
    let base = std::env::var("KALKI_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".kalki")))
        .or_else(|_| std::env::var("USERPROFILE").map(|h| PathBuf::from(h).join(".kalki")))
        .unwrap_or_else(|_| PathBuf::from(".kalki"));
    base.join("sessions")
}

/// Write the abort marker to the WAL and build the aborted outcome.
fn abort_run(
    writer: &mut wal::WalWriter,
    id: &str,
    turns: usize,
    reason: &str,
    final_text: String,
) -> RunOutcome {
    let _ = writer.append(&crate::Event::Lifecycle {
        ts_ms: now_ms(),
        state: format!("aborted:{reason}"),
    });
    RunOutcome {
        session_id: id.to_string(),
        final_text,
        turns,
        aborted: true,
    }
}

/// The session loop: user objective → model turns interleaved with gated tool calls → WAL.
///
/// # Errors
/// Provider stream or WAL failures abort the run; the WAL keeps what happened.
pub async fn run_one_shot<P: Provider>(opts: &RunOptions<P>) -> Result<RunOutcome, CoreError> {
    let id = new_session_id();
    let dir = sessions_dir().join(&id);
    std::fs::create_dir_all(dir.join("blobs"))?;
    let mut writer = wal::WalWriter::open(&dir)?;
    let mut seq: u64 = 0;

    writer.append(&crate::Event::Lifecycle {
        ts_ms: now_ms(),
        state: "created".to_string(),
    })?;

    let mut messages = vec![crate::provider::ChatMessage {
        role: "user".to_string(),
        content: opts.objective.clone(),
        tool_call_id: None,
    }];

    let mut turns = 0usize;
    let mut final_text = String::new();
    let mut aborted = false;

    while turns < opts.max_turns {
        turns += 1;
        let request = crate::provider::ChatRequest {
            model: opts.model.clone(),
            system: Some(SYSTEM_PROMPT.to_string()),
            messages: messages.clone(),
            tools: tool_specs(),
            max_output_tokens: Some(1024),
            reasoning_effort: None,
        };

        let mut stream = match opts.provider.open(request).await {
            Ok(s) => s,
            Err(e) => {
                return Ok(abort_run(
                    &mut writer,
                    &id,
                    turns,
                    format!("open:{e}").as_str(),
                    final_text,
                ));
            }
        };

        let artifacts = match consume_turn(&mut stream).await {
            Ok(a) => a,
            Err(reason) => {
                return Ok(abort_run(
                    &mut writer,
                    &id,
                    turns,
                    format!("stream:{reason}").as_str(),
                    final_text,
                ));
            }
        };
        let TurnArtifacts {
            text: text_this_turn,
            usage_in,
            usage_out,
            usage_cached,
            calls,
        } = artifacts;

        seq += 1;
        writer.append(&crate::Event::Usage {
            ts_ms: now_ms(),
            seq,
            provider: opts.provider.name().to_string(),
            input_tokens: usage_in,
            output_tokens: usage_out,
            cached_tokens: usage_cached,
        })?;

        final_text = text_this_turn.clone();
        messages.push(crate::provider::ChatMessage {
            role: "assistant".to_string(),
            content: text_this_turn,
            tool_call_id: None,
        });

        if calls.is_empty() {
            break;
        }
        let tool_msgs = run_tool_calls(opts, &mut writer, &mut seq, calls).await?;
        messages.extend(tool_msgs);
    }

    if turns >= opts.max_turns {
        aborted = true;
        let _ = writer.append(&crate::Event::Lifecycle {
            ts_ms: now_ms(),
            state: "aborted:turn_cap".to_string(),
        });
    } else {
        let _ = writer.append(&crate::Event::Lifecycle {
            ts_ms: now_ms(),
            state: "closed".to_string(),
        });
    }
    Ok(RunOutcome {
        session_id: id,
        final_text,
        turns,
        aborted,
    })
}

/// Execute gated tool calls, appending `ToolCall`/`ToolResult` WAL events.
///
/// # Errors
/// WAL append failures abort the loop (the WAL holds the truth of what ran).
async fn run_tool_calls<P: Provider>(
    opts: &RunOptions<P>,
    writer: &mut wal::WalWriter,
    seq: &mut u64,
    calls: Vec<(String, String, String)>,
) -> Result<Vec<crate::provider::ChatMessage>, CoreError> {
    let mut out = Vec::with_capacity(calls.len());
    for (call_id, name, args_json) in calls {
        *seq += 1;
        writer.append(&crate::Event::ToolCall {
            ts_ms: now_ms(),
            seq: *seq,
            id: call_id.clone(),
            name: name.clone(),
            arguments_json: args_json.clone(),
        })?;
        let (exit_code, blob_handle, bytes) = execute_tool(opts, &name, &args_json).await;
        let result_text = format!("exit={exit_code:?} bytes={bytes} handle={blob_handle}");
        *seq += 1;
        writer.append(&crate::Event::ToolResult {
            ts_ms: now_ms(),
            seq: *seq,
            id: call_id.clone(),
            handle: blob_handle,
            exit_code,
            bytes,
        })?;
        out.push(crate::provider::ChatMessage {
            role: "tool".to_string(),
            content: result_text,
            tool_call_id: Some(call_id),
        });
    }
    Ok(out)
}

/// Any-provider entry for the CLI: picks the wire from `KALKI_PROVIDER`.
///
/// # Errors
/// Provider env/config errors.
pub async fn run_from_env(
    objective: String,
    model: String,
    max_turns: usize,
    workspace: PathBuf,
) -> Result<RunOutcome, CoreError> {
    let policy = default_policy()?;
    let provider = AnyProvider::from_env().map_err(|e| CoreError::WalCorruption {
        offset: 0,
        reason: format!("provider init: {e}"),
    })?;
    let opts = RunOptions {
        workspace,
        provider,
        policy,
        model,
        objective,
        max_turns,
    };
    run_one_shot(&opts).await
}

fn default_policy() -> Result<PolicyEngine, CoreError> {
    let preamble = r#"
[[rules]]
id = "allow-listing"
action = "allow"
tools = ["bash"]
commands = ["ls", "rg", "cat", "head", "tail", "pwd", "git", "cargo", "rustc"]

[[rules]]
id = "deny-destructive"
action = "deny"
tools = ["bash"]
commands = ["rm", "rmdir", "del", "mkfs", "dd", "shutdown"]
"#;
    let global = if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("kalki").join("policy.toml")
    } else if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("kalki").join("policy.toml")
    } else {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| String::from("."));
        PathBuf::from(home)
            .join(".config")
            .join("kalki")
            .join("policy.toml")
    };
    let toml = std::fs::read_to_string(&global).unwrap_or_else(|_| preamble.to_string());
    PolicyEngine::from_toml_str(&toml).map_err(|e| CoreError::WalCorruption {
        offset: 0,
        reason: format!("policy: {e}"),
    })
}

async fn execute_tool<P: Provider>(
    opts: &RunOptions<P>,
    name: &str,
    args_json: &str,
) -> (Option<i64>, String, u64) {
    let parsed: serde_json::Value = match serde_json::from_str(args_json) {
        Ok(v) => v,
        Err(_) => {
            return (
                Some(2),
                "<invalid arguments json>".to_string(),
                u64::try_from(args_json.len()).unwrap_or(u64::MAX),
            );
        }
    };
    let ctx = crate::tools::ToolContext {
        workspace: opts.workspace.clone(),
        blob_dir: sessions_dir().join("blobs"),
        inline_cap: BLOB_INLINE_CAP,
    };
    match name {
        "bash" => {
            let Some(argv) = parsed
                .get("argv")
                .and_then(|a| serde_json::from_value::<Vec<String>>(a.clone()).ok())
            else {
                return (Some(2), "<missing argv>".to_string(), 0);
            };
            let timeout_secs = parsed
                .get("timeout_secs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(30);
            match crate::tools::run_bash(
                &ctx,
                &opts.policy,
                crate::tools::BashArgs { argv, timeout_secs },
            )
            .await
            {
                Ok(exec) => (
                    exec.output.exit_code,
                    exec.output
                        .blob_handle
                        .unwrap_or_else(|| "inline".to_string()),
                    exec.output.bytes,
                ),
                Err(e) => (Some(1), format!("<tool error: {e}>"), 0),
            }
        }
        "apply_patch" => {
            let patch = parsed
                .get("patch")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let dry_run = parsed
                .get("dry_run")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            match crate::tools::apply_patch(&ctx, crate::tools::ApplyPatchArgs { patch, dry_run })
                .await
            {
                Ok(exec) => (
                    None,
                    exec.output
                        .blob_handle
                        .unwrap_or_else(|| "inline".to_string()),
                    exec.output.bytes,
                ),
                Err(e) => (Some(1), format!("<tool error: {e}>"), 0),
            }
        }
        _ => (Some(2), "<unknown tool>".to_string(), 0),
    }
}
