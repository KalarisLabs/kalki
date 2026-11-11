# Roadmap

Targets, not promises. Each milestone has an exit test. A milestone is done
when its exit test passes in CI, not when its code is merged.

While the version is `0.0.x`, any release may break anything. The policy is in
[docs/adr/0005-versioning-and-release-channel.md](docs/adr/0005-versioning-and-release-channel.md).

## v0.0.1 — Foundation (current)

- [x] Workspace: `kalki-protocol`, `kalki-core`, `kalki`, `xtask`
- [x] Event log: CRC framing, segment rotation, torn-write repair, replay, kill drills
- [x] Blob store with a 4 KiB inline cap
- [x] Deny-first policy engine
- [x] Tools: `bash` (argv), `apply_patch`
- [x] Provider wires: OpenAI Responses, Anthropic Messages, OpenAI-compatible
- [x] Headless `kalki run` and `kalki doctor`
- [x] CI on five platforms, MSRV 1.88 check, dependency policy, release pipeline
- [x] Architecture, proposal, whitepaper, compatibility, governance
- [ ] First green CI run on GitHub
- [ ] Release dry run builds all five targets
- [ ] Tag `v0.0.1`

**Exit test:** a published v0.0.1 release whose five archives pass `kalki doctor`.

## v0.0.2 — A log you can trust

- [ ] Hash chain on every record; `kalki audit verify`
- [ ] `kalki resume <session>` after a crash or a quit
- [ ] Model catalog: one profile per model (tool-call mode, context budget, reasoning fields)
- [ ] Text tool-call mode for models without native function calling
- [ ] Read the workspace `AGENTS.md` into the fixed prefix
- [ ] Live conformance run of the three wires against real endpoints
- [ ] Split `tools.rs` and `session.rs` under the 500-line cap
- [ ] Check `apply_patch` against policy, one decision per target path

**Exit test:** flipping one byte in any log segment makes `kalki audit verify` fail and name the record. A tool-using task completes on at least three open-weight models through Ollama.

## v0.0.3 — Terminal, sandbox, executors

- [ ] Terminal UI per [docs/TUI.md](docs/TUI.md), in a `kalki-tui` crate that depends on `kalki-protocol` only
- [ ] Sandbox: Landlock and seccomp on Linux, Seatbelt on macOS, Job Object on Windows
- [ ] Loopback provider gateway
- [ ] Pi executor over JSONL RPC ([ADR-0004](docs/adr/0004-executors-and-pi.md))
- [ ] Aligned checkpoints and `/rewind`

**Exit test:** the same scripted session through the native and Pi executors produces the same model calls in the kernel log. The compaction-laundering attack of arXiv:2610.29869 succeeds 0% of the time against both. A sandboxed `bash` cannot write outside the workspace on Linux.

## v0.0.4 — Context compiler

- [ ] Subtask tree and structured task state with pointers to source events
- [ ] Per-model context budgets from the catalog
- [ ] Large outputs replaced by handles after first use
- [ ] Rules re-read from the policy source on every call

**Exit test:** rule violations on ARCBench do not rise with session length. A long task completes inside a 32K budget on one open-weight model.

## v0.0.5 — Genome and gate

- [ ] Genome store outside the workspace: versions by content hash, loaded only with an admission record or a trusted signature, three scopes, rollback by hash
- [ ] Evaluation runner: `k` live runs per arm from matched starting points
- [ ] Scripted mock model and log replay, for testing the gate's own code
- [ ] Gate log separate from session logs; held-out results withheld from the proposer
- [ ] CI mode: the gate runs on a protected branch with held-out tasks as secrets
- [ ] Noise floor and per-case flake margins measured and published for our own task set
- [ ] Cost per gate decision measured
- [ ] The eight admission checks ([ADR-0006](docs/adr/0006-admission-gate.md)), in a `kalki-gate` crate
- [ ] Repair loop: provisional notes in the session overlay

**Exit test:** five seeded bad candidates (hard-coded answer, broken protected case, cost only, edited checker, no-op on a flaky trigger) are each rejected by the intended check. A genome file changed by `git checkout` does not load. One real candidate is admitted with a full record.

## v0.0.6 — Evolution and the scorecard

- [ ] Offline evolve loop with an edit budget and edit history
- [ ] Archive of non-dominated variants on score and cost
- [ ] First public scorecard: transfer gain, preservation, cost change, integrity, recovery

**Exit test:** the scorecard is published with model, task set, run count, and cost, and a second person reproduces one row from the instructions.

## v0.1.0 — First compatibility promise

- [ ] Log format and `kalki-protocol` types change only with a migration path
- [ ] 72-hour soak test with seeded kills and a memory-growth limit
- [ ] Threat model document
- [ ] A second maintainer from outside Kalaris Labs
- [ ] Install script and self-update

## Not planned before v0.1.0

Model Context Protocol, a native Gemini wire, package-manager distribution,
editor plugins, a hosted service, model training.

## Stop conditions

Kalki should be stopped or redirected if any of these holds:

- By v0.0.6 the gate admits no change that improves transfer gain on any model we test.
- No number of runs per arm makes the gate both sound and affordable.
- A class of log corruption is found that the design cannot detect.
- Self-modification changes policy-relevant behaviour that the gate did not flag, and no added check fixes it.
