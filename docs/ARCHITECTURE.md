# Kalki Code — Architecture

**Status:** proposed v1 · Owner: Sayan Chowdhury, Kalaris Labs · Updated: 2026-11-08
Numbers cited from `research_notes/Kalki Code/*.md` (2026-11-08). Unverified items marked [unverified].

## Crates (one-way graph, Cargo-enforced)

```
┌──────────────┐
│ kalki-protocol │  types only: Event, WalHeader, snapshots (serde, thiserror)
└──────┬───────┘
       │ (only dependency of the TUI)
┌──────▼───────┐     ┌──────────────┐     ┌──────────────┐
│  kalki-core   │◄────│ kalki (CLI)  │     │  kalki-tui   │  M2: renders protocol frames
│ WAL · sessions│     │  bin singleton│     │              │  (M2, ratatui)
│ providers ·   │     └──────────────┘     └──────────────┘
│ supervisor    │
└──────────────┘
```

**Why (Prime Agent Rust architecture, verified):** nine/ten crates, `pa-types` shared crate, TUI depends on
nothing internal but types; largest file ~2,500 lines; one-way graph enforced by Cargo (Prime Agent Rust, Oct 2026).

## Process model

```
kalki (supervisor, systemd/launchd/Windows Service)
 └─ daemon (owns socket; multiplexes clients; never runs model code)
     ├─ worker session A (process) ── kernel optional ── tools (sandboxed)
     ├─ worker session B (process)
     └─ ...
clients: kalki TUI · --headless pipes · ACP-style socket (M2+)
```

- Crash-only workers: killable at any await; boot = verify anchor + 500-event tail → replay; RPO=0, RTO<10 s (v1 SLOs).
- Fresh generation every N hours/tasks (Temporal Continue-As-New pattern, DIY).
- Attach/detach sessions; Running/Idle/Inactive lifecycle (inspired by Prime Agent; independent implementation).

## Three planes (the memory thesis)

| Plane | Store | Semantics |
|---|---|---|
| Workspace | git worktrees, branches `kalki/<session>/<idx>` | checkpoints every verified increment (private refs `refs/kalki/cp/*`) |
| Trajectory | CRC-framed JSONL segments + redb snapshot index; SHA-256 hash chain | append-only, hash-chained, re-queryable; boot verifies tail anchor |
| Context | subtask-tree JIT compiler (Spine-style) → disposable prompt cache | eviction ≠ deletion; masking > summarization; stable prefix for provider caches |

## Tool plane (see PROPOSAL §5)

Handles-not-payloads · programmatic/batched calls (RLM-compat) · deny-first policy with stable rule IDs · sandbox tiers (WASI / landlock+seccomp+rlimits+netns / bollard) · idempotency keys (journal replay) · scrubbing (secret regex + 4 KiB cap) · per-call audit record.

## State layout (`~/.kalki/`)

```
sessions/<id>.wal        CRC-framed JSONL segments (rotated ~64–128 MB)
snapshots/<id>.redb      index + anchor hashes
blobs/<sha256>           out-of-band payloads (> 4 KiB)
genome/                  prompts/skills/memory/subagents (CRUD registry only)
config.toml              global; workspace .kalki.toml overrides (committed-ok)
audit/ grants.jsonl      hash-chained grant log; kalki audit verify
```

## Durability details (from WAL research)

- JSONL segments + per-record CRC32C + length-prefix; torn tail truncated at boot.
- Boot: verify anchor (from snapshot) + last 500 events; full verify async.
- rusqlite-WAL ≥ 3.51.3 (2026 WAL-reset bug, primary source); SQLite for index, JSONL for log, redb for snapshots.
- Boot budget: tail-verify < 5 s; full 3.6 M-event verify background ~5–8 s [unverified].
- Tool output → blob store; ~3–5 GB per 1000 h session projected [unverified].
- Temporal: wrong fit for single-machine v1; optional adapter at M4 for fleet deployments.

## Providers (from provider research)

`Provider` trait in core. Wires: OpenAI Responses (gpt-5/6-class), Chat Completions (long tail: Kimi/GLM/DeepSeek/xAI/Groq/Ollama), Anthropic Messages (`cache_control`, thinking replay), Gemini (implicit prefix caching ≥ 2k–4k tokens). Catalog: bundled `models.json` + runtime refresh (ETag, schema-versioned, atomic swap). Token accounting: provider-reported usage; tiktoken-rs only for pre-flight estimates [±15 %].

## Self-recursion (M3+, admission-gated)

Task-level smallest-CRUD genome edits + harnessing-level experience library. Admission: held-out SWE-bench Pro slice + hvtb-10 tripwire + zero-regression ceiling; rollback by refinement ID always available; unsigned skills never activate.

## Performance reference points (Prime Agent Rust, Oct 2026 — their numbers, not ours)

First paint 23.6 ms · time-to-type cold 55.8 ms · RSS 106 MB · install 59.6 MB. Kalki Code targets: parity at M2, then hillclimb with the same alternating-A/B + reviewer-model methodology.

## Known-weak spots (tracked in ROADMAP honesty clause)

Time-to-first-token on Windows; sandbox on stock Windows; evaluator collapse (mitigation: hack-verifiable evals); memory poisoning via adopted skills (mitigation: minisign + quarantine); 1000-hour claim requires soak evidence (target: v0.5 gate).
