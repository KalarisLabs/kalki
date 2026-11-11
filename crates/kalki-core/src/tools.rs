//! Built-in tools (bash, apply_patch). Untrusted execution behind the policy gate.
//! v0.2: bash with argv-array spawn (no shell interpolation), 4 KiB inline cap, overflow to the blob store.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::blob::put_blob;
use crate::policy::{Decision, PolicyEngine, ToolRequest};

/// Maximum bytes of merged stdout+stderr kept in memory (the rest is discarded,
/// but counted in [`ToolOutput::bytes`]).
pub const MAX_CAPTURE_BYTES: usize = 64 * 1024;

/// Inline cap marker inserted when output overflows into the blob store.
const TRUNCATED_MARKER: &str = "truncated, blob handle";

/// The execution context a tool needs (workspace root, blob dir, policy).
#[derive(Clone)]
pub struct ToolContext {
    /// Absolute path of the session workspace (tools may not escape it).
    pub workspace: PathBuf,
    /// Blob store directory.
    pub blob_dir: PathBuf,
    /// Inline cap for tool output; overflow goes to the blob store with a `payload_ref`.
    pub inline_cap: usize,
}

/// Arguments to the bash tool (argv array; no shell string by design).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashArgs {
    /// Program + args; `argv[0]` is the program. Never a shell string — models must not get implicit `sh -c`.
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
    Spawn(#[source] std::io::Error),
    /// Policy denied the call.
    #[error("policy: {0}")]
    Policy(String),
    /// Timeout hit.
    #[error("timeout after {0}s")]
    Timeout(u64),
    /// Workspace confinement violation.
    #[error("confinement: {0}")]
    Confinement(String),
    /// The arguments to a tool were empty or otherwise unusable.
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    /// An I/O failure while reading, writing, or removing a file.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// A unified diff was malformed or did not apply.
    #[error("patch: {0}")]
    Patch(String),
    /// The blob store rejected the payload.
    #[error("blob: {0}")]
    Blob(#[from] crate::CoreError),
}

/// What happened in a tool run — checkpointed into the WAL by the session loop.
#[derive(Debug, Clone)]
pub struct ToolExecution {
    /// stdout+stderr merged, capped inline.
    pub output: ToolOutput,
    /// True when output was truncated and the remainder went to the blob store.
    pub truncated_to_blob: bool,
}

/// Run an argv command under the workspace confinement + policy gate.
///
/// Honest level of v0.2: argv confinement is a *path heuristic*, not a sandbox —
/// it rejects argv elements that escape the workspace but cannot stop a
/// PATH-resolved binary from doing anything the OS user may do. Real enforcement
/// lands with landlock in v0.3 (see docs/PROPOSAL.md §sandbox tiers).
///
/// Note on policy: `ask` rules are treated as deny here because the headless
/// runner has no approval channel yet; interactive approval lands with the TUI
/// in v0.2.x. The error names the rule id so the operator knows exactly which
/// rule to adjust.
///
/// On timeout the process is killed (`kill_on_drop`) and a [`ToolError::Timeout`]
/// is returned without partial output (simpler v0.2 semantics; documented).
///
/// Secrets are scrubbed from the inline view only; the blob stays raw — the blob
/// store is the ground truth the audit log signs, scrubbing it would falsify it.
///
/// # Errors
/// Returns [`ToolError`] on policy denial, confinement violation, spawn/IO
/// failure, or timeout.
pub async fn run_bash(
    ctx: &ToolContext,
    policy: &PolicyEngine,
    args: BashArgs,
) -> Result<ToolExecution, ToolError> {
    let start = Instant::now();
    if args.argv.is_empty() {
        return Err(ToolError::InvalidArgs("argv must not be empty".to_string()));
    }
    let BashArgs {
        argv: command_argv,
        timeout_secs,
    } = args;
    // Policy gate first: deny-first, before any filesystem or process side effects.
    let req = ToolRequest {
        tool: "bash",
        command: Some(&command_argv),
        path: None,
        network: false,
    };
    match policy.decide(&req) {
        Decision::Allowed => {}
        Decision::Denied { rule } => {
            let id = rule.as_deref().unwrap_or("default-deny").to_string();
            return Err(ToolError::Policy(format!("denied by rule `{id}`")));
        }
        Decision::NeedsApproval { rule } => {
            return Err(ToolError::Policy(format!(
                "rule `{rule}` requires interactive approval (headless mode: treated as denied)"
            )));
        }
    }
    let workspace = canonical_workspace(&ctx.workspace)?;
    for element in &command_argv {
        check_argv_element(element, &workspace)?;
    }
    let timeout_secs = timeout_secs.min(120);
    let mut cmd = tokio::process::Command::new(&command_argv[0]);
    cmd.args(&command_argv[1..]);
    cmd.current_dir(&workspace);
    cmd.kill_on_drop(true);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    // Minimal environment: PATH only, never argv/env of the session. Ambient
    // secrets (ANTHROPIC_API_KEY & co.) must not leak into tool processes.
    cmd.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        cmd.env("PATH", path);
    }
    let mut child = cmd.spawn().map_err(ToolError::Spawn)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ToolError::Spawn(std::io::Error::other("stdout not piped")))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ToolError::Spawn(std::io::Error::other("stderr not piped")))?;
    let kept = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(8192)));
    let total = Arc::new(AtomicU64::new(0));
    let pump_out = tokio::spawn(pump(stdout, kept.clone(), total.clone()));
    let pump_err = tokio::spawn(pump(stderr, kept.clone(), total.clone()));
    let waited =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), child.wait()).await;
    let status = match waited {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => return Err(ToolError::Io(e)),
        // kill_on_drop: dropping the handle kills the child; partial output is
        // deliberately discarded in v0.2 (documented on the function docs).
        Err(_) => return Err(ToolError::Timeout(timeout_secs)),
    };
    // Drain both pipes (EOF follows child death) so no bytes are lost.
    if let Err(e) = pump_out.await {
        tracing::warn!(error = %e, "bash stdout pump join failed");
    }
    if let Err(e) = pump_err.await {
        tracing::warn!(error = %e, "bash stderr pump join failed");
    }
    let kept = {
        let mut guard = kept.lock().await;
        std::mem::take(&mut *guard)
    };
    let exit_code = status.code().map(i64::from);
    let bytes = total.load(Ordering::Relaxed);
    let cap = ctx.inline_cap;
    let (inline, blob_handle, truncated_to_blob) = if kept.len() <= cap {
        (scrub_secrets(&String::from_utf8_lossy(&kept)), None, false)
    } else {
        // Blob stays raw (ground truth); only the inline view is scrubbed.
        let handle = put_blob(&ctx.blob_dir, &kept)?;
        let head = scrub_secrets(&String::from_utf8_lossy(&kept[..cap]));
        (
            format!("{head}\n[... {TRUNCATED_MARKER}: {handle}]"),
            Some(handle),
            true,
        )
    };
    Ok(ToolExecution {
        output: ToolOutput {
            inline,
            blob_handle,
            exit_code,
            bytes,
            elapsed_ms: elapsed_ms(start),
        },
        truncated_to_blob,
    })
}

/// Read a pipe to end, retaining at most [`MAX_CAPTURE_BYTES`]; `total` keeps the true count.
async fn pump<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
    kept: Arc<tokio::sync::Mutex<Vec<u8>>>,
    total: Arc<AtomicU64>,
) {
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                total.fetch_add(n as u64, Ordering::Relaxed);
                let mut guard = kept.lock().await;
                let room = MAX_CAPTURE_BYTES.saturating_sub(guard.len());
                if room > 0 {
                    guard.extend_from_slice(&chunk[..n.min(room)]);
                }
            }
        }
    }
}

/// Canonicalize the workspace root; every confinement check builds on it.
fn canonical_workspace(workspace: &Path) -> Result<PathBuf, ToolError> {
    std::fs::canonicalize(workspace).map_err(|e| {
        ToolError::Confinement(format!(
            "cannot canonicalize workspace {}: {e}",
            workspace.display()
        ))
    })
}

/// Reject argv elements that escape the workspace (`..` segments, or absolute/
/// relative paths resolving outside it). Bare program names pass through and are
/// resolved against PATH — see the limitation note on [`run_bash`].
fn check_argv_element(element: &str, workspace: &Path) -> Result<(), ToolError> {
    let path = Path::new(element);
    let path_like = element.contains(['/', '\\']) || path.is_absolute();
    for segment in element.split(['/', '\\']) {
        if segment == ".." {
            return Err(ToolError::Confinement(format!(
                "`..` segment in argv element {element:?} escapes the workspace"
            )));
        }
    }
    if !path_like {
        return Ok(());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        // Flag-style arguments like `/C` are rooted-but-not-drive-qualified on
        // Windows (Path::is_absolute == false); pushing only the plain components
        // keeps the check honest instead of `join` replacing the workspace prefix.
        let mut resolved = workspace.to_path_buf();
        for component in path.components() {
            if let std::path::Component::Normal(part) = component {
                resolved.push(part);
            }
        }
        resolved
    };
    let resolved = if absolute.exists() {
        std::fs::canonicalize(&absolute)
            .map_err(|e| ToolError::Confinement(format!("cannot canonicalize {element:?}: {e}")))?
    } else {
        lexical_normalize(&absolute)
    };
    if !resolved.starts_with(workspace) {
        return Err(ToolError::Confinement(format!(
            "path {element:?} resolves outside the workspace"
        )));
    }
    Ok(())
}

/// Resolve `.`/`..` textually without touching the filesystem.
fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

/// Redact token-like substrings: anything after a known secret prefix that is at
/// least 8 token chars (`[A-Za-z0-9_-]`) becomes `[REDACTED]`. Hand loop, no regex.
///
/// Known prefixes: `sk-`, `ghp_`, `gho_`, `github_pat_`, `AKIA`, `xoxb-`, `xoxp-`, `AIza`.
/// Deliberately conservative: non-matching text passes through untouched.
#[must_use]
pub fn scrub_secrets(text: &str) -> String {
    const PREFIXES: [&[u8]; 8] = [
        b"sk-",
        b"ghp_",
        b"gho_",
        b"github_pat_",
        b"AKIA",
        b"xoxb-",
        b"xoxp-",
        b"AIza",
    ];
    const MIN_BODY: usize = 8;
    let is_body = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if let Some(prefix) = PREFIXES.iter().find(|p| bytes[i..].starts_with(p)) {
            let body_start = i + prefix.len();
            let mut end = body_start;
            while end < bytes.len() && is_body(bytes[end]) {
                end += 1;
            }
            if end - body_start >= MIN_BODY {
                out.extend_from_slice(b"[REDACTED]");
                i = end;
                continue;
            }
            out.extend_from_slice(&bytes[i..body_start]);
            i = body_start;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    // We only ever delete whole ASCII ranges, so the bytes stay valid UTF-8;
    // keep a lossless fallback instead of panicking regardless.
    match String::from_utf8(out) {
        Ok(s) => s,
        Err(_) => text.to_string(),
    }
}

/// Arguments to the apply-patch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchArgs {
    /// The unified diff to apply.
    pub patch: String,
    /// Compute the hunks that would apply without writing anything.
    #[serde(default)]
    pub dry_run: bool,
}

enum HunkLine {
    Context(String),
    Removal(String),
    Addition(String),
}

struct Hunk {
    old_start: usize,
    old_count: usize,
    new_count: usize,
    lines: Vec<HunkLine>,
}

struct FileDiff {
    old_path: Option<String>,
    new_path: Option<String>,
    saw_plus_header: bool,
    hunks: Vec<Hunk>,
}

impl FileDiff {
    /// `true` when the diff creates the file (`--- /dev/null`).
    fn is_create(&self) -> bool {
        self.old_path.is_none()
    }
    /// `true` when the diff deletes the file (`+++ /dev/null`).
    fn is_delete(&self) -> bool {
        self.saw_plus_header && self.new_path.is_none()
    }
}

/// Apply (or, with `dry_run`, simulate) a unified-diff patch inside the workspace.
///
/// Supports the exact unified-diff subset git produces: `--- a/path` /
/// `+++ b/path` file headers, `@@ -a[,b] +c[,d] @@` hunks, additions/deletions/
/// context with **exact line match — no fuzz** (documented limitation), and
/// file creation (`--- /dev/null`) and deletion (`+++ /dev/null`). Paths must be
/// workspace-relative; absolute paths and `..` segments are rejected.
///
/// # Errors
/// Returns [`ToolError::Patch`] for malformed diffs and context mismatches,
/// [`ToolError::Confinement`] for out-of-workspace paths, [`ToolError::Io`] for
/// filesystem failures.
// Async by spec (the session loop awaits all tools uniformly); v0.2 does no I/O on a runtime yet.
#[allow(clippy::unused_async)]
pub async fn apply_patch(
    ctx: &ToolContext,
    args: ApplyPatchArgs,
) -> Result<ToolExecution, ToolError> {
    let start = Instant::now();
    let workspace = canonical_workspace(&ctx.workspace)?;
    let files = parse_patch(&args.patch)?;
    if files.is_empty() {
        return Err(ToolError::Patch("no file diffs found in patch".to_string()));
    }
    let mut summary = String::new();
    let mut total_hunks = 0usize;
    for file in &files {
        if !file.saw_plus_header {
            return Err(ToolError::Patch(format!(
                "missing +++ header for {}",
                file.old_path.as_deref().unwrap_or("<new file>")
            )));
        }
        let rel = if file.is_delete() {
            file.old_path.clone()
        } else {
            file.new_path.clone().or_else(|| file.old_path.clone())
        }
        .ok_or_else(|| ToolError::Patch("file diff has no path".to_string()))?;
        let target = resolve_in_workspace(&workspace, &rel)?;
        let original = match std::fs::read_to_string(&target) {
            Ok(content) => Some(content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(ToolError::Io(e)),
        };
        if file.is_create() && original.is_some() {
            return Err(ToolError::Patch(format!(
                "cannot create {rel}: file already exists"
            )));
        }
        if !file.is_create() && original.is_none() {
            return Err(ToolError::Patch(format!("cannot patch missing file {rel}")));
        }
        let content = original.unwrap_or_default();
        let had_trailing_newline = content.ends_with('\n') || file.is_create();
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        let (new_lines, applied) = apply_hunks(lines, &file.hunks)?;
        total_hunks += applied;
        if !args.dry_run {
            if file.is_delete() {
                std::fs::remove_file(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut text = new_lines.join("\n");
                if had_trailing_newline && !new_lines.is_empty() {
                    text.push('\n');
                }
                std::fs::write(&target, text)?;
            }
        }
        let marker = if file.is_create() {
            'A'
        } else if file.is_delete() {
            'D'
        } else {
            'M'
        };
        // write! into a String is infallible; the result is intentionally discarded.
        let _ = write!(summary, "\n  {marker} {rel} ({} hunk(s))", file.hunks.len());
    }
    let verb = if args.dry_run {
        "would change"
    } else {
        "changed"
    };
    let inline = format!(
        "apply_patch ({}): {} file(s) {verb}, {} hunk(s){summary}",
        if args.dry_run { "dry run" } else { "applied" },
        files.len(),
        total_hunks,
    );
    let bytes = inline.len() as u64;
    Ok(ToolExecution {
        output: ToolOutput {
            inline,
            blob_handle: None,
            exit_code: None,
            bytes,
            elapsed_ms: elapsed_ms(start),
        },
        truncated_to_blob: false,
    })
}

/// Parse a unified diff into per-file hunk lists.
fn parse_patch(patch: &str) -> Result<Vec<FileDiff>, ToolError> {
    let err = |msg: &str| ToolError::Patch(msg.to_string());
    let mut files: Vec<FileDiff> = Vec::new();
    let mut current: Option<FileDiff> = None;
    let mut hunk: Option<Hunk> = None;
    let close_hunk = |file: &mut FileDiff, hunk: &mut Option<Hunk>| -> Result<(), ToolError> {
        let Some(h) = hunk.take() else { return Ok(()) };
        let old_block = h
            .lines
            .iter()
            .filter(|l| !matches!(l, HunkLine::Addition(_)))
            .count();
        let new_block = h
            .lines
            .iter()
            .filter(|l| !matches!(l, HunkLine::Removal(_)))
            .count();
        if old_block != h.old_count || new_block != h.new_count {
            return Err(ToolError::Patch(format!(
                "hunk @@ -{},{} +?,{} @@ declares counts that do not match its body",
                h.old_start, h.old_count, h.new_count
            )));
        }
        file.hunks.push(h);
        Ok(())
    };
    for raw in patch.lines() {
        if let Some(rest) = raw.strip_prefix("--- ") {
            if let Some(mut file) = current.take() {
                close_hunk(&mut file, &mut hunk)?;
                files.push(file);
            }
            let raw_path = rest.trim();
            let old_path = if raw_path == "/dev/null" {
                None
            } else {
                Some(strip_diff_prefix(raw_path).to_string())
            };
            current = Some(FileDiff {
                old_path,
                new_path: None,
                saw_plus_header: false,
                hunks: Vec::new(),
            });
            continue;
        }
        if let Some(rest) = raw.strip_prefix("+++ ") {
            let file = current.as_mut().ok_or_else(|| err("+++ before ---"))?;
            close_hunk(file, &mut hunk)?;
            let raw_path = rest.trim();
            file.new_path = if raw_path == "/dev/null" {
                None
            } else {
                Some(strip_diff_prefix(raw_path).to_string())
            };
            file.saw_plus_header = true;
            continue;
        }
        if raw.starts_with("@@") {
            let file = current
                .as_mut()
                .ok_or_else(|| err("hunk before file header"))?;
            close_hunk(file, &mut hunk)?;
            hunk = Some(parse_hunk_header(raw)?);
            continue;
        }
        if let Some(open) = hunk.as_mut() {
            match raw.as_bytes().first() {
                Some(b' ') => open.lines.push(HunkLine::Context(raw[1..].to_string())),
                Some(b'-') => open.lines.push(HunkLine::Removal(raw[1..].to_string())),
                Some(b'+') => open.lines.push(HunkLine::Addition(raw[1..].to_string())),
                Some(b'\\') => {} // "\ No newline at end of file" — offsets are line-based, so safe to ignore
                None => open.lines.push(HunkLine::Context(String::new())), // lenient: bare empty line = empty context
                _ => return Err(err("unexpected line inside hunk")),
            }
        }
        // Anything else (diff --git, index, mode lines…) is ignored header noise.
    }
    if let Some(mut file) = current.take() {
        close_hunk(&mut file, &mut hunk)?;
        files.push(file);
    } else if hunk.is_some() {
        return Err(err("hunk without file header"));
    }
    Ok(files)
}

/// Strip the `a/` / `b/` prefix git adds; returns the raw path otherwise.
fn strip_diff_prefix(path: &str) -> &str {
    path.get(2..)
        .filter(|_| path.starts_with("a/") || path.starts_with("b/"))
        .unwrap_or(path)
}

/// Parse `@@ -a[,b] +c[,d] @@ …` into a [`Hunk`] with an empty body.
fn parse_hunk_header(line: &str) -> Result<Hunk, ToolError> {
    let body = line
        .strip_prefix("@@")
        .ok_or_else(|| ToolError::Patch(format!("bad hunk header {line:?}")))?;
    let end = body
        .find("@@")
        .ok_or_else(|| ToolError::Patch(format!("bad hunk header {line:?}")))?;
    let mut parts = body[..end].split_whitespace();
    let (old_start, old_count) = parse_range(
        parts
            .next()
            .ok_or_else(|| ToolError::Patch(format!("bad hunk header {line:?}")))?,
        '-',
    )?;
    let (_new_start, new_count) = parse_range(
        parts
            .next()
            .ok_or_else(|| ToolError::Patch(format!("bad hunk header {line:?}")))?,
        '+',
    )?;
    Ok(Hunk {
        old_start,
        old_count,
        new_count,
        lines: Vec::new(),
    })
}

fn parse_range(token: &str, sign: char) -> Result<(usize, usize), ToolError> {
    let body = token
        .strip_prefix(sign)
        .ok_or_else(|| ToolError::Patch(format!("bad hunk range {token:?}")))?;
    let (start, count) = match body.split_once(',') {
        Some((s, c)) => (s, Some(c)),
        None => (body, None),
    };
    let start: usize = start
        .trim()
        .parse()
        .map_err(|_| ToolError::Patch(format!("bad hunk range {token:?}")))?;
    let count: usize = match count {
        Some(c) => c
            .trim()
            .parse()
            .map_err(|_| ToolError::Patch(format!("bad hunk range {token:?}")))?,
        None => 1,
    };
    Ok((start, count))
}

/// Join a patch path onto the workspace after rejecting escapes.
fn resolve_in_workspace(workspace: &Path, rel: &str) -> Result<PathBuf, ToolError> {
    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(ToolError::Confinement(format!(
            "absolute patch path {rel:?}"
        )));
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => {
                return Err(ToolError::Confinement(format!(
                    "patch path {rel:?} escapes the workspace"
                )));
            }
        }
    }
    let joined = lexical_normalize(&workspace.join(path));
    if !joined.starts_with(workspace) {
        return Err(ToolError::Confinement(format!(
            "patch path {rel:?} escapes the workspace"
        )));
    }
    Ok(joined)
}

/// Apply hunks with exact line matching (no fuzz), returning the new lines and hunk count.
fn apply_hunks(lines: Vec<String>, hunks: &[Hunk]) -> Result<(Vec<String>, usize), ToolError> {
    let mut lines = lines;
    let mut offset: i64 = 0;
    for (index, hunk) in hunks.iter().enumerate() {
        // Unified diff: when the old block is empty the position is "insert after old_start".
        let anchor = if hunk.old_count == 0 {
            hunk.old_start
        } else {
            hunk.old_start.saturating_sub(1)
        };
        let start_i64 = i64::try_from(anchor)
            .map_err(|_| ToolError::Patch(format!("hunk {} position out of range", index + 1)))?
            + offset;
        let start = usize::try_from(start_i64)
            .map_err(|_| ToolError::Patch(format!("hunk {} position out of range", index + 1)))?;
        let old_block = hunk
            .lines
            .iter()
            .filter(|l| !matches!(l, HunkLine::Addition(_)))
            .count();
        let mut cursor = start;
        for line in &hunk.lines {
            match line {
                HunkLine::Addition(_) => {}
                HunkLine::Context(expected) | HunkLine::Removal(expected) => {
                    let found = lines.get(cursor).ok_or_else(|| {
                        ToolError::Patch(format!("hunk {} runs past end of file", index + 1))
                    })?;
                    if found != expected {
                        return Err(ToolError::Patch(format!(
                            "hunk {} context mismatch at line {}: expected {expected:?}, found {found:?}",
                            index + 1,
                            cursor + 1
                        )));
                    }
                    cursor += 1;
                }
            }
        }
        let mut new_seq: Vec<String> = Vec::with_capacity(hunk.lines.len());
        for line in &hunk.lines {
            match line {
                HunkLine::Context(text) | HunkLine::Addition(text) => new_seq.push(text.clone()),
                HunkLine::Removal(_) => {}
            }
        }
        let new_len = new_seq.len();
        drop(lines.splice(start..start + old_block, new_seq));
        offset += i64::try_from(new_len).unwrap_or(i64::MAX)
            - i64::try_from(old_block).unwrap_or(i64::MAX);
    }
    Ok((lines, hunks.len()))
}

fn elapsed_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_all_known_secret_prefixes() {
        let text = "keys: sk-abcdefghijklmnop1 ghp_abcdefghijklmnop2 gho_abcdefghijklmnop3 \
github_pat_abcdefghijklmnop4 AKIAIOSFODNN7EXAMPLE xoxb-1234567890-abcdefghij \
xoxp-1234567890-abcdefghij AIzaSyA1234567890abcdefghijklmnopqrstuvwx";
        let scrubbed = scrub_secrets(text);
        assert_eq!(scrubbed.matches("[REDACTED]").count(), 8);
        assert!(!scrubbed.contains("sk-abcdefghijklmnop1"));
        assert!(!scrubbed.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!scrubbed.contains("AIzaSyA1234567890"));
    }

    #[test]
    fn leaves_short_tokens_and_plain_text_untouched() {
        let text = "hello world, sk-abc and AKIA short are not secrets";
        assert_eq!(scrub_secrets(text), text);
    }

    #[test]
    fn redacts_tokens_embedded_in_larger_text() {
        let text = "export KEY=sk-abcdefghijklmnop123456 tail";
        let scrubbed = scrub_secrets(text);
        assert_eq!(scrubbed, "export KEY=[REDACTED] tail");
    }

    #[test]
    fn keeps_text_without_secrets_byte_identical() {
        let text = "unicode stays intact: héllo → wörld 🦀";
        assert_eq!(scrub_secrets(text), text);
    }
}
