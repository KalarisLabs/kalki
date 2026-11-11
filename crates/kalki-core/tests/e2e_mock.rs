//! Mock-model end-to-end tests: real HTTP provider wire against a loopback
//! axum mock — no real network, no real API keys.
//!
//! # Process model (why the children exist)
//! Edition 2024 makes `std::env::set_var` `unsafe`, and the workspace sets
//! `unsafe_code = "forbid"` (it reaches test targets) — the tests cannot mutate
//! process env in-process. Every scenario therefore spawns this test binary as
//! a child (`--ignored child_<name> --exact`) with the full `KALKI_*` /
//! `OPENAI_*` env populated for the child only (`Command::envs`, which is safe).
//! The child runs the real pipeline (`run_from_env` → `AnyProvider::from_env`
//! → provider → SSE decode → session loop → WAL) against the parent's mock.
//! Parents assert the child's exit status and independently audit the WAL; the
//! children assert the outcome and WAL semantics. `child_*` tests no-op in
//! normal runs (`#[ignore]`d + a `SCENARIO_ENV` guard). This file is the only
//! env-touching suite in the repo (as `Command` setup, never `set_var`).
//!
//! # Wire order
//! The decoder flushes a frame on its terminating blank line, buffering `event:` and `data:`
//! lines per frame — it handles both orders. These fixtures pin the
//! order the real Responses API sends (event: first), so a regression to
//! wire-strict single-order decoding fails here loudly.
//!
//! # Entry point
//! Children use `run_from_env`. It exercises `AnyProvider::from_env` (the real
//! `KALKI_PROVIDER` wiring); the model comes from `KALKI_MODEL` read by the
//! child, mirroring what the CLI passes in. `APPDATA` is redirected to a fresh
//! empty dir so the default policy is the built-in preamble (allows `cargo`),
//! not whatever `policy.toml` the real user profile happens to carry.
//!
//! Tests 1–3/5 run `cargo --version` as the gated bash tool: cargo exists by
//! definition wherever `cargo test` runs, and the preamble allows it. (The
//! `cmd /C` / `sh -c` fallback was considered and rejected: neither name is in
//! the preamble allow-list, so the tool would be policy-denied.)

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::post;
use kalki_core::Event;
use kalki_core::session::RunOutcome;
use kalki_core::wal::{self, WalWriter};
use serde_json::json;

const SCENARIO_ENV: &str = "KALKI_E2E_CHILD";
const OBJECTIVE_TOOL_CALL: &str = "kalki e2e mock: check the toolchain version, then stop";
const OBJECTIVE_TURN_CAP: &str = "kalki e2e mock: keep calling the bash tool forever";
const OBJECTIVE_UNICODE: &str = "Fix the parser in src/héllo.rs so it keeps “quotes” and 🦀 emoji — café users report é is dropped silently.";
const FINAL_TEXT_UNICODE: &str = "héllo wörld ✨ done";

// --- SSE transcripts --------------------------------------------------------
// Frame = `event:` first (the order the real Responses API sends), then `data:`,
// then a blank line to flush. The decoder flushes on the blank line and tolerates
// both orders; these fixtures pin the production wire order.

fn frame(event: &str, data: &serde_json::Value) -> String {
    format!("event: {event}\ndata: {data}\n\n")
}

fn text_delta_frame(text: &str) -> String {
    frame(
        "response.output_text.delta",
        &json!({"type": "response.output_text.delta", "delta": text}),
    )
}

fn tool_start_frame(id: &str, name: &str) -> String {
    frame(
        "response.output_item.added",
        &json!({"type": "response.output_item.added", "item": {"id": id, "name": name}}),
    )
}

fn args_delta_frame(id: &str, delta: &str) -> String {
    frame(
        "response.function_call_arguments.delta",
        &json!({"type": "response.function_call_arguments.delta", "item_id": id, "delta": delta}),
    )
}

fn completed_frame(input_tokens: u64, output_tokens: u64) -> String {
    frame(
        "response.completed",
        &json!({"type": "response.completed", "response": {"usage": {"input_tokens": input_tokens, "output_tokens": output_tokens}}}),
    )
}

fn tool_call_turn(input_tokens: u64, output_tokens: u64) -> String {
    let full_args = r#"{"argv":["cargo","--version"],"timeout_secs":5}"#;
    let split = 11; // mid-token on purpose: the accumulator concatenates raw deltas
    let (first, second) = full_args.split_at(split);
    assert_eq!(format!("{first}{second}"), full_args);
    [
        tool_start_frame("fc_1", "bash"),
        args_delta_frame("fc_1", first),
        args_delta_frame("fc_1", second),
        completed_frame(input_tokens, output_tokens),
    ]
    .concat()
}

fn text_turn(text: &str, input_tokens: u64, output_tokens: u64) -> String {
    [
        text_delta_frame(text),
        completed_frame(input_tokens, output_tokens),
    ]
    .concat()
}

/// Torn stream: the final `data:` frame lands but the connection dies before the
/// `event:` terminator — the decoder must surface an error, not a hang.
fn torn_stream_transcript() -> String {
    let mut out = text_delta_frame("partial");
    let dangling = json!({"type": "response.completed", "response": {"usage": {"input_tokens": 11, "output_tokens": 7}}});
    out.push_str("data: ");
    out.push_str(&dangling.to_string());
    out.push('\n');
    out
}

// --- Mock server --------------------------------------------------------------

/// Request transcripts served in order; extra requests repeat the last one.
struct Script(Vec<String>);

struct MockState {
    script: Vec<String>,
    hits: AtomicUsize,
    recorded: Arc<Mutex<Vec<String>>>,
}

async fn responses_handler(State(state): State<Arc<MockState>>, body: String) -> impl IntoResponse {
    state
        .recorded
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(body);
    let hits = state.hits.fetch_add(1, Ordering::Relaxed);
    let index = hits.min(state.script.len().saturating_sub(1));
    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        state.script[index].clone(),
    )
}

/// Bind the mock on `127.0.0.1:0` and serve it on a dedicated thread (own
/// current-thread runtime; `axum::serve` needs a runtime around `from_std`).
fn spawn_mock(script: Script) -> (String, Arc<Mutex<Vec<String>>>) {
    assert!(
        !script.0.is_empty(),
        "mock script must have at least one transcript"
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock listener");
    let addr = listener.local_addr().expect("mock listener local_addr");
    listener
        .set_nonblocking(true)
        .expect("mock listener must be non-blocking for tokio");
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let state = Arc::new(MockState {
        script: script.0,
        hits: AtomicUsize::new(0),
        recorded: Arc::clone(&recorded),
    });
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("mock runtime");
        runtime.block_on(async move {
            let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
                return; // mock dies silently -> child sees connection refused, test fails loudly
            };
            let app = Router::new()
                .route("/v1/responses", post(responses_handler))
                .with_state(state);
            let _ = axum::serve(listener, app).await;
        });
    });
    (format!("http://{addr}"), recorded)
}

// --- Child process plumbing ----------------------------------------------------

struct ChildOutcome {
    success: bool,
    report: String,
}

/// Re-exec this test binary as `child_<scenario>` with the env that cannot be
/// set in-process (edition 2024: `set_var` is unsafe; workspace forbids unsafe).
fn spawn_scenario(child_test: &str, envs: &[(String, String)], timeout: Duration) -> ChildOutcome {
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = std::process::Command::new(exe);
    cmd.arg(child_test)
        .arg("--exact")
        .arg("--ignored")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .envs(envs.iter().cloned())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn scenario child");
    let stdout = child.stdout.take().expect("stdout pipe");
    let stderr = child.stderr.take().expect("stderr pipe");
    let out_reader = std::thread::spawn(move || read_pipe(stdout));
    let err_reader = std::thread::spawn(move || read_pipe(stderr));

    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    timed_out = true;
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => panic!("try_wait failed for {child_test}: {e}"),
        }
    };
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    let status_text = match (timed_out, status) {
        (true, _) => format!("timeout after {timeout:?} (child killed)"),
        (false, Some(status)) => status.to_string(),
        (false, None) => "unknown (child waited but no status)".to_string(),
    };
    ChildOutcome {
        success: status.is_some_and(|s| s.success()) && !timed_out,
        report: format!(
            "status: {status_text}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr)
        ),
    }
}

fn read_pipe(mut pipe: impl std::io::Read) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = pipe.read_to_end(&mut buf);
    buf
}

struct ParentRun {
    _tmp: tempfile::TempDir, // keeps the scenario tree alive for the duration of the test
    home: PathBuf,
    recorded: Arc<Mutex<Vec<String>>>,
}

/// Spin up a fresh temp tree + mock + child env, and assert the child passed.
fn run_e2e(scenario: &str, script: Script, timeout: Duration) -> ParentRun {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("kalki-home");
    let workspace = tmp.path().join("workspace");
    let appdata = tmp.path().join("appdata"); // empty: built-in policy preamble applies
    for dir in [&home, &workspace, &appdata] {
        std::fs::create_dir_all(dir).expect("create scenario dirs");
    }
    let (base_url, recorded) = spawn_mock(script);
    let child_test = format!("child_{scenario}");
    let envs: Vec<(String, String)> = [
        ("KALKI_HOME", home.to_string_lossy().into_owned()),
        (
            "KALKI_E2E_WORKSPACE",
            workspace.to_string_lossy().into_owned(),
        ),
        ("APPDATA", appdata.to_string_lossy().into_owned()),
        ("OPENAI_API_KEY", "sk-test-e2e-mock".to_string()),
        ("OPENAI_BASE_URL", base_url),
        ("KALKI_PROVIDER", "openai_responses".to_string()),
        ("KALKI_MODEL", "gpt-mock".to_string()),
        (SCENARIO_ENV, scenario.to_string()),
    ]
    .map(|(key, value)| (key.to_string(), value))
    .to_vec();
    let child = spawn_scenario(&child_test, &envs, timeout);
    assert!(
        child.success,
        "child scenario {scenario} failed\n{}",
        child.report
    );
    ParentRun {
        _tmp: tmp,
        home,
        recorded,
    }
}

fn wal_segment_files(dir: &Path) -> Vec<PathBuf> {
    let mut segments: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "wal")
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("seg-"))
        })
        .collect();
    segments.sort();
    segments
}

fn replay_paths(segments: &[PathBuf]) -> Vec<Event> {
    let mut events = Vec::new();
    for segment in segments {
        events.extend(
            wal::replay_segment(segment).expect("wal::replay_segment must iterate without error"),
        );
    }
    events
}

fn replay_session_events(home: &Path, session_id: &str) -> Vec<Event> {
    let segments = wal_segment_files(&home.join("sessions").join(session_id));
    assert!(
        !segments.is_empty(),
        "expected at least one seg-*.wal for session {session_id}"
    );
    replay_paths(&segments)
}

/// All events committed under `home/sessions/*` (one session per fresh tree).
fn collect_wal_events(home: &Path) -> (usize, Vec<Event>) {
    let sessions = home.join("sessions");
    let segments: Vec<PathBuf> = std::fs::read_dir(&sessions)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|session| wal_segment_files(&session.path()))
        .collect();
    (segments.len(), replay_paths(&segments))
}

fn lifecycle_states(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Lifecycle { state, .. } => Some(state.clone()),
            _ => None,
        })
        .collect()
}

fn count_usage(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::Usage { .. }))
        .count()
}

fn child_guard(scenario: &str) -> bool {
    std::env::var(SCENARIO_ENV).as_deref() == Ok(scenario)
}

fn run_child(max_turns: usize, objective: &str) -> RunOutcome {
    let model = std::env::var("KALKI_MODEL").expect("parent sets KALKI_MODEL");
    let workspace =
        PathBuf::from(std::env::var("KALKI_E2E_WORKSPACE").expect("parent sets workspace"));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .expect("child runtime");
    runtime
        .block_on(kalki_core::session::run_from_env(
            objective.to_string(),
            model,
            max_turns,
            workspace,
        ))
        .expect("run_from_env must succeed against the mock")
}

/// Guard + run + WAL replay shared by the child scenarios. `None` when this is
/// not the child the parent spawned (normal `cargo test` runs).
fn guarded_child(
    scenario: &str,
    max_turns: usize,
    objective: &str,
) -> Option<(RunOutcome, Vec<Event>)> {
    if !child_guard(scenario) {
        return None;
    }
    let outcome = run_child(max_turns, objective);
    let home = std::env::var("KALKI_HOME").expect("parent sets KALKI_HOME");
    let events = replay_session_events(Path::new(&home), &outcome.session_id);
    Some((outcome, events))
}

// --- Scenario 1: tool call then text finish -----------------------------------

#[test]
#[ignore = "child re-entry: spawned by the parent test with a mock-provided env"]
fn child_one_shot_session_runs_a_tool_call_and_finishes() {
    let Some((outcome, events)) = guarded_child(
        "one_shot_session_runs_a_tool_call_and_finishes",
        10,
        OBJECTIVE_TOOL_CALL,
    ) else {
        return;
    };
    assert_eq!(outcome.final_text, "done");
    assert_eq!(outcome.turns, 2);
    assert!(!outcome.aborted);
    assert_eq!(lifecycle_states(&events), ["created", "closed"]);

    let tool_calls: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::ToolCall { .. }))
        .collect();
    let [
        Event::ToolCall {
            name,
            arguments_json,
            ..
        },
    ] = tool_calls.as_slice()
    else {
        panic!("expected exactly one ToolCall, got {tool_calls:?}");
    };
    assert_eq!(name, "bash");
    assert!(
        arguments_json.contains("cargo"),
        "arguments: {arguments_json}"
    );

    let tool_results: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::ToolResult { .. }))
        .collect();
    let [Event::ToolResult { exit_code, .. }] = tool_results.as_slice() else {
        panic!("expected exactly one ToolResult, got {tool_results:?}");
    };
    assert_eq!(*exit_code, Some(0));
    assert_eq!(count_usage(&events), 2, "one usage record per model turn");
}

#[test]
fn one_shot_session_runs_a_tool_call_and_finishes() {
    let scenario = "one_shot_session_runs_a_tool_call_and_finishes";
    let script = Script(vec![tool_call_turn(11, 7), text_turn("done", 20, 2)]);
    let run = run_e2e(scenario, script, Duration::from_secs(30));

    let bodies = run.recorded.lock().expect("recorded bodies");
    assert!(
        bodies.len() >= 2,
        "expected at least two requests, got {}",
        bodies.len()
    );
    assert!(bodies[0].contains(OBJECTIVE_TOOL_CALL));
    assert!(
        bodies[1].contains("\"role\":\"tool\""),
        "tool message missing in second request"
    );
    assert!(
        bodies[1].contains("exit=Some(0)"),
        "tool result missing in second request"
    );
    drop(bodies);

    let (segments, events) = collect_wal_events(&run.home);
    assert!(
        segments >= 1,
        "expected at least one seg-*.wal under {}",
        run.home.display()
    );
    assert!(
        !events.is_empty(),
        "wal replay must yield the committed events"
    );
}

// --- Scenario 2: turn cap -----------------------------------------------------

#[test]
#[ignore = "child re-entry: spawned by the parent test with a mock-provided env"]
fn child_one_shot_aborts_at_turn_cap_when_model_keeps_calling_tools() {
    let Some((outcome, events)) = guarded_child(
        "one_shot_aborts_at_turn_cap_when_model_keeps_calling_tools",
        2,
        OBJECTIVE_TURN_CAP,
    ) else {
        return;
    };
    assert!(outcome.aborted);
    assert_eq!(outcome.turns, 2);
    let states = lifecycle_states(&events);
    assert!(
        states.iter().any(|state| state == "aborted:turn_cap"),
        "expected aborted:turn_cap in {states:?}"
    );
    assert!(!states.iter().any(|state| state == "closed"));
    assert!(
        count_usage(&events) >= 2,
        "expected at least two Usage events"
    );
}

#[test]
fn one_shot_aborts_at_turn_cap_when_model_keeps_calling_tools() {
    let scenario = "one_shot_aborts_at_turn_cap_when_model_keeps_calling_tools";
    let script = Script(vec![tool_call_turn(11, 7)]); // repeated: the model never stops calling bash
    let run = run_e2e(scenario, script, Duration::from_secs(30));
    let (_segments, events) = collect_wal_events(&run.home);
    assert!(
        lifecycle_states(&events)
            .iter()
            .any(|state| state == "aborted:turn_cap")
    );
}

// --- Scenario 3: torn stream --------------------------------------------------

#[test]
#[ignore = "child re-entry: spawned by the parent test with a mock-provided env"]
fn child_one_shot_aborts_when_stream_ends_mid_frame() {
    let Some((outcome, events)) = guarded_child(
        "one_shot_aborts_when_stream_ends_mid_frame",
        10,
        OBJECTIVE_TOOL_CALL,
    ) else {
        return;
    };
    assert!(outcome.aborted);
    assert_eq!(outcome.turns, 1);
    let states = lifecycle_states(&events);
    assert!(
        states.iter().any(|state| state.starts_with("aborted:")),
        "expected an aborted lifecycle, got {states:?}"
    );
}

#[test]
fn one_shot_aborts_when_stream_ends_mid_frame() {
    let scenario = "one_shot_aborts_when_stream_ends_mid_frame";
    let run = run_e2e(
        scenario,
        Script(vec![torn_stream_transcript()]),
        Duration::from_secs(10),
    );
    let (_segments, events) = collect_wal_events(&run.home);
    assert!(
        lifecycle_states(&events)
            .iter()
            .any(|state| state.starts_with("aborted:"))
    );
}

// --- Scenario 4: kill -9 drill ------------------------------------------------

fn message_event(seq: u64) -> Event {
    Event::Message {
        ts_ms: 1_700_000_000_000 + seq,
        seq,
        role: "user".to_string(),
        content: format!("kill-9 probe {seq}"),
        payload_ref: None,
    }
}

fn single_wal_segment(dir: &Path) -> Result<PathBuf, String> {
    let segments = wal_segment_files(dir);
    if segments.len() == 1 {
        Ok(segments.into_iter().next().expect("len checked"))
    } else {
        Err(format!(
            "expected exactly one seg-*.wal, found {}",
            segments.len()
        ))
    }
}

#[test]
fn kill_minus9_drill_recovers_committed_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let events: Vec<Event> = (1..=5).map(message_event).collect();
    {
        let mut writer = WalWriter::open(dir.path())?;
        for event in &events {
            writer.append(event)?;
        }
    } // simulated kill -9: no clean shutdown, no ceremony

    let segment = single_wal_segment(dir.path())?;
    // Simulate the garbage tail a crash can leave behind.
    std::fs::OpenOptions::new()
        .append(true)
        .open(&segment)?
        .write_all(b"\xde\xad\xbe\xefpartial")?;

    // Recovery open must truncate the garbage tail; the committed prefix survives.
    {
        let _repaired = WalWriter::open(dir.path())?;
    }
    let recovered: Vec<Event> = wal::replay_segment(&segment)?.collect();
    assert_eq!(
        recovered, events,
        "committed prefix must survive the garbage tail"
    );

    // Appending after repair extends the committed prefix (the brief's "6th event").
    let sixth = message_event(6);
    {
        let mut writer = WalWriter::open(dir.path())?;
        writer.append(&sixth)?;
    }
    let mut expected = events.clone();
    expected.push(sixth);
    let after: Vec<Event> = wal::replay_segment(&segment)?.collect();
    assert_eq!(
        after, expected,
        "append after repair must extend, not corrupt"
    );
    Ok(())
}

// --- Scenario 5: unicode objective roundtrip -----------------------------------

#[test]
#[ignore = "child re-entry: spawned by the parent test with a mock-provided env"]
fn child_unicode_objective_roundtrips() {
    let Some((outcome, events)) =
        guarded_child("unicode_objective_roundtrips", 10, OBJECTIVE_UNICODE)
    else {
        return;
    };
    assert_eq!(outcome.final_text, FINAL_TEXT_UNICODE);
    assert!(!outcome.aborted);
    assert_eq!(outcome.turns, 1);
    assert_eq!(lifecycle_states(&events), ["created", "closed"]);
}

#[test]
fn unicode_objective_roundtrips() {
    let scenario = "unicode_objective_roundtrips";
    let run = run_e2e(
        scenario,
        Script(vec![text_turn(FINAL_TEXT_UNICODE, 11, 7)]),
        Duration::from_secs(30),
    );
    let bodies = run.recorded.lock().expect("recorded bodies");
    assert!(!bodies.is_empty(), "the mock must have seen the request");
    assert!(
        bodies[0].contains(OBJECTIVE_UNICODE),
        "the exact unicode objective must reach the wire verbatim"
    );
}
