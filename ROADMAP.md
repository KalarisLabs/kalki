# Roadmap — Kalki Code

Targets, not promises. Semver starts at v0.1.0; **1.0 = stability contract, not feature list**.

## v0.1.0 — Scaffold (now, 2026-11)
- [x] Workspace: `kalki-protocol`, `kalki-core`, `kalki` (CI green: fmt, clippy, nextest)
- [x] WAL record framing (encode/decode) + roundtrip tests
- [ ] `kalki doctor` v2 (keyring, sandbox probe, WAL integrity)
- [ ] docs/PROPOSAL.md + docs/ARCHITECTURE.md + ADRs 0001–0003
- [ ] License dual MIT/Apache-2.0, DCO, NOTICE

## v0.2.0 — The session loop (M1–M2, Nov–Dec 2026) — **landed on main, tag pending final P9**
- [x] WAL segments (CRC-framed JSONL) + replay + torn-write repair (`impl/wal` merged; snapshot anchor API in `kalki-core::snapshot`; redb index lands with hash-chain milestone)
- [x] Provider layer v1 (OpenAI Responses + Anthropic Messages + OpenAI-compatible: Kimi/GLM/DeepSeek/xAI/Ollama — `AnyProvider` via `KALKI_PROVIDER`)
- [x] Minimal tool loop (bash + apply_patch behind deny-first policy grants)
- [ ] Mock-model e2e + kill -9 recovery suite on main (in flight)
- [ ] TUI v1 (ratatui, inline mode) — deferred to v0.3; v0.2 ships headless `kalki run`
- [ ] 50-task SWE-bench Pro slice + harness-CI (mock-model parity) — M2 gate
- [ ] Attach/detach daemon + worker reuse across invocations — v0.2.x then v0.3


## v0.3.0 — Memory & context (M2, Jan 2027)
- [ ] Subtask-tree context compiler (Spine-style, JIT, masking > summarization)
- [ ] Memory tiers (core/recall/archival) with provenance + quarantine/contradiction checks
- [ ] Artifact ledger (invariants/decisions/progress/feature-list.json)
- [ ] gix-first git integration (status/diff/worktree/checkpoints)
- [ ] `kalki audit verify` (hash-chain CLI)

## v0.4.0 — Self-recursion with admission control (M3, Feb–Mar 2027)
- [ ] Genome (H=(ρ,G,K,M)) editing via smallest CRUD ops; rollback by ID
- [ ] Oracle gate: held-out SWE-bench Pro + hvtb-10 hack tripwire; no adoption without pass
- [ ] Harnessing-level experience library (Harness²-inspired, cross-task)
- [ ] `kalki-eval` reporter + release notes deltas

## v0.5.0 — Institutions (M4, Apr 2027)
- [ ] RBAC + managed config (`/etc/kalki/managed.toml`), airgapped mode, MDM hooks
- [ ] MSI/DEB/RPM/pkg via cargo-dist + nfpm; docs site on kalarislabs.com/docs
- [ ] OSSF Best Practices badge; third-party maintainer; THREAT_MODEL.md
- [ ] 72 h soak: RSS-slope assertion, seeded kill -9 recovery ≥ 95 %

## 1.0.0 — Stability contract (H2 2027)
- semver on CLI/config/protocol; documented MSRV policy; deprecation windows
- 24-month security backports; ≥ 2 non-author maintainers
- published methodology: n runs, seeds, variance vs Codex/Prime Agent on fixed tasks

KILL CRITERIA (explicit): no path to a trustworthy 100 h unattended run by v0.4; self-recursion causing policy drift in shadow runs; RSS slope > X MB/h sustained; or unfixable evaluator collapse. See docs/PROPOSAL.md §Risks.
