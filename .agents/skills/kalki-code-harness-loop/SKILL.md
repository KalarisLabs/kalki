---
name: kalki-code-harness-loop
description: End-to-end map of the Kalki Code harness — session FSM, WAL replay, context compiler, tool dispatch, self-recursion gates. Load for any task touching the core loop.
---

# Kalki Code Harness Loop (Kalaris Labs)

## The loop, precisely
1. **Attach**: client → daemon socket → worker process per session; TUI reads protocol frames.
2. **Turn**: model call → streamed deltas → tool-call intents queued.
3. **Gate**: every tool call = (schema check) → (policy lookup, deny-first TOML rule IDs) → (grant check) → (sandbox tier: WASI | landlock+seccomp+rlimits+netns | bollard) → execute → scrub output (regex secrets + 4 KiB cap) → handle returned.
4. **Record**: every event appended to WAL before the UI sees it; CRC per record; SHA-256 prev-hash chain; periodic signed anchor into redb snapshot.
5. **Context**: before each model call, the compiler emits JIT context from the subtask tree (invariants, open decisions, ledger tail, active task spec) — stable prefix for cache hits; eviction, never summarization-delete.
6. **Verify**: state flips only on independent eval (mocked tests oracles in CI; real suites pre-release). Agent never sets `passes: true`.
7. **Compaction**: cache eviction + `Compaction` event; original bytes remain fetchable by `payload_ref`.
8. **Crash**: worker dies → daemon reattaches → boot verifies anchor+tail → pending tool calls replayed idempotently or land in the human-review queue. Nothing is auto-acknowledged silently.

## Self-recursion (v0.4)
- **Genome (task-level)**: smallest CRUD edit to prompts/skills/memory/subagents; rollback by refinement ID; `.agents/`-tier skills signed (minisign) — unsigned skills never activate.
- **Improvement library (harnessing-level)**: EXPERIENCE.md-style entries; adopted harnesses indexed; transfer measured across task sets.
- **Admission control (both)**: held-out eval slice must pass; zero tolerance for hack-tripwire hits; regression ceiling per task; full rollback path exists before the edit is proposed.

## Failure invariants (never violate)
- Agent cannot write to: policy files, grant state, audit log, genome adoption state.
- No adopted edit without a named eval delta + archived rollback point.
- Crash recovery never replays an unverifiable side effect as done — it re-enters the loop or queues a human.
