---
name: kalki-code-testing
description: How Kalki Code is tested — mock-model differential harness, insta frame snapshots, WAL kill -9 recovery, benchmark protocol. Load before adding or changing any test.
---

# Kalki Code Testing (Kalaris Labs)

## Layers
1. **Unit** (pure): `kalki-protocol` serde roundtrips, WAL frame codec, policy eval (table-driven), context compiler determinism.
2. **Mock-model differential** (the backbone): `tests/mock/` — scripted SSE server (`kalki-testmock` pattern), seeded, zero latency, fault injection; run the real loop against it; assert session transcripts + provider requests + TUI frames.
   - TUI frames: render under a vt100-parser-like emulator; `insta` snapshot compare.
   - Protocol: message-type set diff vs expectation (protocol parity).
   - Transcripts: normalized JSONL diff (harness parity).
3. **Crash drills** (`tests/crash/`): kill -9 at seeded points (mid-turn, mid-tool-call, mid-snapshot, mid-anchor); assert: boot < 10 s, anchor+tail verify OK, no replayed side effect, human-review queue contains the torn call.
4. **Soak**: nightly 6 h synthetic loop; RSS slope regression gate (alert > 2 MB/h); handle leaks via worker generation handshake.
5. **Benchmarks** (perf hillclimb): process-tree RSS, time-to-first-render, warm start, event append/replay rate; A/B on same sandbox, alternating order, two reviewer-model sign-off; publish methodology, never cherry-pick.

## Commands
```bash
cargo xtask ci        # full local gate
cargo nextest run     # tests
cargo xtask crash     # kill -9 drills
cargo xtask bench     # criterion + process probes
INSTA_UPDATE=auto cargo nextest run  # snapshot refresh, then review diffs
```

## Style
- Names: `does_x_when_y`; no sleeps — use barriers/notify.
- One assertion per concept, three max per test.
- Mock model responses live as files in `tests/fixtures/sse/` — diffable in review.
- Never assert wall-clock time; assert budgets with generous ceilings + a metric instead.
