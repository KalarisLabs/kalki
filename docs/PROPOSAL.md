# Kalki Code — Build Proposal

**Name:** Kalki Code · **Organization:** Kalaris Labs · **Author:** Sayan Chowdhury (hello@kalarislabs.com)
**Status:** proposal v1.0 (pre-release) · **Date:** 2026-11-08 · **Audience:** founders, institutional adopters, contributors
**License:** MIT OR Apache-2.0 · **Repo:** github.com/Kalaris-Labs/kalki-code (pre-release) · **Site:** kalarislabs.com

> This is a proposal for a build that has started (v0.1.0 scaffold: workspace compiles, `kalki doctor` runs) and has not shipped a session loop. claims marked [unverified] are engineering estimates awaiting measurement. Nothing here is a shipping capability yet.

---

## 1. BLUF

Kalki Code is a **self-recursive continual coding harness written in Rust**, terminal-first, that treats long-horizon autonomous software work (design target: **1000 hours**) as an engineering discipline rather than a model-scale gamble. It is the control plane — durability, memory, verification, self-improvement, governance — that models worth institutional money deserve.

Five claims carry the design:

1. **1000-hour autonomy is an architecture problem, not a model-size problem** — METR's time-horizon study (doubling ~every 7 months, NeurIPS 2025, arXiv:2503.14499) plus Long-Horizon-Terminal-Bench's ~4.3% frontier pass@1 (arXiv:2607.08964) show error compounding, not skill, is binding; the only antidote is per-step verification and externalized state.
2. **Self-recursion is safe only behind oracle-gated admission control** — Prime Intellect's own Factorio run proved unguarded `/refine` builds cheating skills; DGM proved archived-variant admission works (SWE-bench 20→50.0%, arXiv:2505.22954). Kalki adopts nothing without a held-out eval delta and an instant rollback.
3. **Memory is a re-queryable log; context is a disposable, JIT-compiled cache** — compaction as cache eviction, never deletion. Subtask-tree compilation (Spine, arXiv:2610.37224) showed +11.8% milestone score at ~−32% cost.
4. **A crash-only Rust single binary is an unclaimed wedge** — Prime Agent's Rust rewrite (Oct 2026) reached 14.18× faster first paint, 4.76× smaller process RSS, 2.89× smaller install; the field has no Rust-native, self-recursive, institution-grade continual harness today.
5. **Institutions standardize on governed infrastructure, not clever agents** — hash-chained audit, RBAC, eval-as-CI, reproducible trajectories, signed releases. This is the sale OpenAI's Harness Engineering case study (~1M LOC, ~1,500 PRs, 3→7 engineers) proves the budget exists for.

---

## 2. The problem we are solving

Production harnesses in 2026 share four defects:

| Failure | Evidence |
|---|---|
| **Stateless across sessions** | Compaction-by-summarization destroys provenance; PI criticizes "a succession of agents connected by a prompt and some files"; Anthropic's two-agent pattern (Claude Code) is a workaround, not a fix |
| **Memory without provenance** | Summary-only memory → poisoning, drift, hallucinated recall; no re-query path to ground truth |
| **Self-improvement without admission control** | `/refine`-style loops amplify reward hacking (PI's Factorio RCON exploit); Harness² avoids ground truth but has no hack tripwires either |
| **No governance** | No hash-chained audit, no reproducible trajectories, no RBAC — disqualifying for institutions standardizing spend |

Kalki Code is the answer to all four, in one binary.

---

## 3. State of the art — what we adopt, adapt, avoid

### 3.1 Prime Agent (Prime Intellect) — verified Oct 2026

TypeScript launch (Aug 5, 2026) → **ground-up Rust rewrite** (Oct 2026, "Rewriting Prime Agent in Rust"): 2,000+ agents across 10,000+ Prime Sandboxes, ~200B tokens of GLM-5.3; 10 crates, one-way Cargo graph, `pa-types` shared crate, TUI only depends on protocol types; worker-per-session + small supervisor; platform abstraction via traits (Windows beta); catalog-driven models (runtime fetch, ships models without releases); sessions persist on disk; re-attach after restart; differential parity testing (TUI frames / transcripts / daemon protocol / feature audit); release hygiene (v0.9.x→0.10.2 weekly, nightly pre-releases, changelog fragments, one-line installers, self-update).

Measured (their runtime suite, 4-core/8GB, not cargo states): first paint 23.6 ms; time-to-type cold 55.8 ms / warm 42.4 ms; RSS 106 MB; install 59.6 MB; **>300k downloads, >8B tokens processed**. Installers: `install.sh`/`install.ps1` via `app.primeintellect.ai`, Homebrew, GitHub Releases (5 targets). **Not on crates.io or npm.** License MIT.

**Adopt (architecture):** workspace shape · `pa-types`-equivalent protocol crate · worker-per-session supervisor · catalog-driven models · differential parity CI · release mechanics (installers, nightly, changelog fragments, weekly cadence) · FSM orchestration (Planner/Implementer/Reviewer/Verifier with separate verifier).
**Adapt:** add WAL hash-chain + blob store + snapshot index (they persist JSONL + kernel snapshots only); add sandboxing tiers (PI runs with user permissions); add oracle-gated genome adoption (PI's `/refine` ungated, reward-hacked in Factorio); add governance plane (none today).
**Avoid:** kernel-as-only-tool minimalism (keep it optional, not mandatory); ARC-AGI-3-style unreplicated headline claims (we publish methodology first).

### 3.2 Harness² (Google Cloud AI Research, arXiv:2610.14224) — verified

Two-level recursion: **task-level** Propose→Probe→Compose contrastive self-edits (invariant + domain candidates; parallel/sequential; RSA-style aggregation) and **harnessing-level** improvement library (`EXPERIENCE.md`, cross-task). Eight file-editable components (AHE decomposition). No ground truth; behavioral contrast evidence; reported +11.5 WorkBuddy (GLM-5), +4.5 JobBench, +11.8 pp LAB Partial (**unreviewed preprint; 1–3 runs; not independently replicated**).

**Adopt:** the two-level shape · propose/probe/compose loop with observation-bound edits · experience library across task streams.
**Adapt:** add admission control (their selector ≠ verified optimality); add hack tripwires; add signed skills.
**Avoid:** verifier-free claims presented as quality guarantees; their benchmark judge asymmetries.

### 3.3 Industry (verified 2026)

Claude Code (leak-derived; 5-layer context pipeline; auto-approve ~93% — permission fatigue; arXiv 2604.14228) · Codex CLI (Rust; kernel sandbox; apply_patch; ~1M LOC harness-engineering case study) · Gemini CLI→Antigravity (enterprise-only since 2026-06) · mini-swe-agent (single bash tool matches/beats elaborate harnesses on DeepSWE — tool minimalism wins) · Devin (sandboxed VM agents) · Copilot coding agent · Aider/OpenHands/OpenCode (TS/Python; no self-recursion) · Gastown/Ralph (community loops, unordered). None ship: durable WAL replay mid-tool-call, hash-chained audit, oracle-gated self-adoption, cargo workspace discipline. **R1: Kalki differentiates on durability + trust + self-improvement safety, not on tool count.**

---

## 4. The design (system shape)

### 4.1 Workspace & distribution

Rust 1.88 workspace, Cargo-enforced one-way graph (ADR-0001):

```
kalki-protocol → kalki-core → { kalki CLI, kalki-tui (M2) }   xtask (automation)
```

Single static musl binary (Linux x86_64/aarch64), macOS universal, Windows x86_64 beta; cargo-dist releases; npm shim `@kalarislabs/kalki-code`; Homebrew tap; `curl | sh` at kalarislabs.com (§7 for exact launch commands). Size target < 30 MB stripped; cold start < 60 ms.

### 4.2 Process model — crash-only, reattachable

`kalki` (thin supervisor, launchd/systemd/SchTasks) → daemon (socket multiplex; no model code) → **worker process per session**. Killable at any await point. Boot: verify snapshot anchor + last 500 events → replay; pending tool calls idempotent-replay or enter human-review queue (never auto-"done"). SLO v1: **RPO=0, RTO<10 s**. Fresh generation handshake every N hours (Continue-As-New pattern) defeats slow RSS growth; RSS slope alerting (> 2 MB/h) is CI-enforced on 72 h soaks.

### 4.3 Three planes of state

| Plane | Implementation | Property |
|---|---|---|
| Workspace | git worktrees (`.kalki/worktrees/`), private refs `refs/kalki/cp/*` (gix 0.90; git CLI for LFS/hooks/apply-3way) | checkpoint = verified increment; merge-safe; GC-swept |
| Trajectory | append-only CRC32C-framed JSONL segments + SHA-256 prev-hash chain + redb snapshot index + 4 KiB inline threshold → content-addressed blob store | hash-chained audit; re-queryable; replayable |
| Context | subtask-tree JIT compiler (Spine-style): inline nesting for coupled work, parallel child harnesses for independent work, masking > summarization, stable prefix for provider prompt caching | cache eviction ≠ deletion; −32 % cost, +11.8 % milestone (literature reference) |

### 4.4 Memory & artifacts

Memory tiers (Letta-inspired, Kalki-native): **core** (pinned invariants, always loaded) · **recall** (SQLite FTS chunks of the WAL, ranked by relevance) · **archival** (blob store + artifact ledger: `invariants.md`, `decisions.md`, `progress.log`, `feature-list.json` with `passes` flips only on independent eval). Provenance on every entry (source event hash); quarantine on contradiction; adopted memories tagged by origin harness version. **The WAL is the only truth**; everything else is a view.

### 4.5 Tool plane — the industry gap

| Industry today | Kalki Code |
|---|---|
| Payload stdout/files stuffed into context (rot) | **Handles**: bytes → blob store; context sees `handle+schema+4 KiB preview` |
| One tool call per ReAct turn | **Programmatic calls**: batch/fan-out inside the kernel (RLM-compatible `llm_batch`); truncated REPL echo |
| Tools on ambient env (user perms) | **Tiered sandbox**: WASI (untrusted) → landlock+seccomp+rlimits+empty-netns (local shell) → bollard (networked) |
| Approval fatigue (93 % auto-approve, Claude Code) | **Deny-first policy** (stable rule IDs), session-scoped grants, classifier + N-block demotion; grants hash-chained in the audit log |
| Self-certified "done" | **Independent flip** on mocked oracle (CI) / suite eval (pre-release); agent never sets `passes` |
| No resume mid-tool-call | **Idempotency keys + WAL journal**; crash replay is exact or human-queued |
| Tool pool static | **Lazy discovery + per-task policy profiles** (Fix/Research/Refactor/Autonomous); MCP client+server with signed-skill admission |
| edits: str_replace/search-replace | **apply_patch + tree-sitter AST fallback** (Codex-class determinism) |
| Telemetry ad hoc | **None.** Opt-in redacted crash bundle later; institution-lockable |

### 4.6 Self-recursion — the two loops, admission-gated

```
LOOP 1 (task-level, minutes)            LOOP 2 (harnessing-level, days)
 trajectory ──► smallest CRUD edit      improvement record ──► lesson extract
 prompts/skills/memory/subagents        EXPERIENCE.md library (cross-task)
 rollback by refinement ID              adopted harnesses indexed
        │                                       │
        └────► ORACLE GATE ◄────────────────────┘
       held-out eval delta > 0 AND per-task regression ceiling
       AND hvtb-10 hack tripwire clean AND rollback point archived
                     ↓
              genome merge or hard revert
```

Gate details: eval = SWE-bench Pro slice (n=50, seeded stratification) + Long-Horizon-Terminal-Bench smoke (6 tasks) + hvtb-10 anti-cheat battery; judge never the executor (Anthropic's never-self-judge rule); adoption atomic with a pre-merge snapshot; PI's experience + Harness²'s library format are the schema. **No gate, no adoption — ever.** Kill switch: `genome.adopt` writes are outside the agent's sandbox write set (enforced at the WASI/landlock layer, not by prompt).

### 4.7 Providers & models (compatibility for "most usage")

`Provider` trait; v1 wires: OpenAI **Responses** (gpt-5.x/6-class), OpenAI-compatible **Chat Completions** (Kimi/GLM/DeepSeek/xAI/Groq/Ollama — responses API is not implemented on the long tail), **Anthropic Messages** (`cache_control` breakpoints; thinking-signature replay), **Gemini** (implicit prefix caching ≥ 2k tokens). Catalog-driven (bundled `models.json` + ETag refresh, schema-versioned, data-only, no secrets, atomic swap). Keyring-or-env auth; single redaction choke point. Token accounting: provider-reported `usage`; tiktoken-rs for pre-flight only (±15 %).

### 4.8 TUI

M2: ratatui 0.30.2 + crossterm 0.29.0 (inline scrollback default; alt-screen opt-in), `tui-textarea-2` editor, `@` file picker, slash-command palette, approval modals; differential frame tests (vt100+insta) — the Prime Agent parity-testing pattern, and the CI backbone (`kalki-tui` crate depends only on `kalki-protocol`).

### 4.9 Reliability & observability

tracing→JSONL rolling file (source of truth) + optional OTLP (feature-gated) + prometheus on loopback/unix; key series: event-log append/replay lag, snapshot age, RSS slope, model-call retry rate, tool p50/p95. systemd/k8s watchdogs; ractor supervision (budget + meltdown); doctor command with `--fix`; `.changes/` fragments; semver from tags.

---

## 5. The 1000-hour thesis, honestly

**No published system demonstrates a 1000-hour unattended run.** Closest: 30-min/6h-scale artifacts (Anthropic DAW 6 h / C compiler 2,000 sessions ≈ 100k LOC with human prompting; Prime Agent Factorio/ARC; METR suite at ~hour scale). Kalki's honest claim: *the union of the above techniques makes 100 h runs plausible and 1000 h measurable*; kilo-hour claims ship only behind the v0.5 soak gate (72 h synthetic, seeded kill -9 ≥ 95 % recovery, RSS slope < 2 MB/h) and only with published methodology.

Kill criteria (explicit, in ROADMAP): evaluator collapse in gate canaries; self-recursion policy drift in shadow; RSS slope uncontrolled; unfixable WAL corruption class; or M2 not shipping an end-to-end session by 2027-01-31.

---

## 6. Build plan & release engineering

**Milestones (ROADMAP.md is binding):** v0.1 scaffold → **v0.2 session loop (2026-12)** → v0.3 memory/compiler (2027-01) → v0.4 self-recursion gated (2027-03) → v0.5 institutions (2027-04) → 1.0 stability (H2 2027).

### 6.1 Dependency strategy

DEPENDENCIES.md (committed): exact locked versions, 11-column decision table, ban list (sled, TensorZero, seccomp 0.1, rustformers/llm, git2, rig as framework), TLS=rustls-only, bundled SQLite. New deps need ADR + sign-off. See §DEPENDENCIES.md for the table.

### 6.2 Release & version-launch commands (verified mechanics)

One-time: Homebrew tap repo `homebrew-kalarislabs`; `HOMEBREW_TAP_TOKEN`; crates.io trusted publishing (GitHub OIDC, `id-token: write`); npm scope `@kalarislabs`; minisign keypair (public key at kalarislabs.com); installers hosted at `releases.kalarislabs.com` + GitHub Releases (5 targets); first publish of each crate manual, then per-crate trusted publisher.

Per release (maintainer machine):

```bash
cargo release minor --execute          # bumps, tags v0.2.0, changelog (git-cliff), publishes crates.io in order
cargo dist build --tag v0.2.0          # binaries + installers + SBOM (CycloneDX) + SHA256SUMS + minisign
cargo dist publish                     # GitHub Release + Homebrew formula push
npm publish --provenance               # @kalarislabs/kalki-code shim (Trusted Publishing, auto-provenance)
kalki doctor --release-smoke           # scripted sanity: install → first-run → one turn → audit verify
gh attestation verify dist/kalki.tar.gz --owner Kalaris-Labs
minisign -Vm kalki.tar.gz -p kalarislabs.com/kalki.pub
```

Nightly: `vX.Y.Z-nightly` rolling pre-release (Prime Agent pattern). MSRV: 1.88 (bump via ADR only).

### 6.3 Distribution matrix

`curl|sh` (Linux/macOS, R2-backed + GitHub fallback) · `irm|iex` (Windows beta) · Homebrew tap · `@kalarislabs/kalki-code` npm shim (aliased optionalDependencies, no postinstall) · `cargo install kalki` · DEB/RPM/MSI via cargo-dist+nfpm (institutions) · distroless container · flake.nix.

---

## 7. Evaluation & research methodology

**Harness CI (beads every release):** mock-model differential parity (TUI frames via vt100+insta; transcripts; daemon protocol messages) · core50 SWE-bench Pro V2 stratified (seed published) · lh-smoke (6/46 LHTB) · TB2.0 via Harbor · hvtb-10 tripwire · keep-if-delta>0 with ≤ 10 % per-task regression ceiling **after measuring our noise floor** ([unverified] — measured at M2) · nightly 6 h synthetic soak with RSS-slope assertion · kill -9 drill suite (mid-turn, mid-tool-call, mid-anchor).

**Public reporting (release notes):** mean ± stddev, n≥3 runs, tasks hash-pinned, judge ≠ executor, methodology URL, cost per task. No cherry-picking; no ARC-AGI-style unreplicated claims without replication notes.

**Paper:** the research paper accompanies the proposal (`reports/Kalki Code research paper.md`, Sayan Chowdhury) — architecture, threat model, admission control, evaluation design. To target COLM 2027 / arXiv (cs.SE + cs.AI).

---

## 8. Security & compliance (institutions)

Deny-first policy TOML (v1) → Cedar decision engine (v1.5+, `cedar-policy` Apache-2.0). Layered sandbox (WASI/landlock/seccomp/netns/Job Objects; Windows = WSL2 default, MXC tracking). Output scrubbing + canary files. Hash-chained grant log (`kalki audit verify`). Supply chain: cargo-deny + daily audit + SHA-pinned Actions + SBOM + provenance attestations. Secrets: keyring v4 + env; nothing to Kalaris Labs. Local-first; zero egress without grant; airgapped mode: bundled catalog + allowed-endpoints list + embedded offline docs (single-page `docs.html`). SOC-2-friendly data-flow doc ships at v0.5; GDPR: transcripts are local-only by default; PII redaction opt-in.

---

## 9. Kalaris Labs positioning (honest, for kalarislabs.com)

**Announce as:** “Kalaris Labs is building Kalki Code — the first open, self-recursive continual coding harness: Rust-native, oracle-gated self-improvement, crash-only durability, zero telemetry. Research and roadmap published; v0.2 session loop in December 2026.”
**Never claim:** a shipped 1000-hour run; unverified parity vs Codex/Prime Agent; ARC-AGI-style numbers; “best” without published methodology.
**Launch kit:** this proposal + paper + repo (public, Apache/MIT, DCO) + v0.1 tags + 3 demo recordings + HN/r-LocalLLaMA/X posts + kalarislabs.com page with honest scope + attribution block (pi, Prime Intellect).

---

## 10. Risks & open problems

1. No public 1000-hour run exists (ours would be the first). 2. Prime Agent/Harness² numbers unreplicated; adopt mechanisms, not claims. 3. Evaluator collapse is the existential risk — hvtb-10 + independent judge + rollback are load-bearing, not optional. 4. Windows sandbox parity is genuinely hard (WSL2 crutch; MXC unreleased). 5. WAL hash-chain detects tampering, not root/WORM attackers — document honestly. 6. gix status perf inconsistencies reported on some trees — benchmark before merge; CLI fallback retained. 7. Dependency churn: async-openai quarantined (ADR-0002); Temporal Rust GA'd Sept 2026 but we DIY for v1. 8. Keyring behaviors differ across platforms — CI-tested matrix. 9. "Kalki" trademark clearance required (cultural overload) — run clearance search before printing.

---

## 11. What exists right now (as of 2026-11-08)

Workspace compiles; `kalki doctor` runs; WAL framing roundtrips; DEPENDENCIES.md (locked, 11-column); AGENTS.md (42 rules); 4 project skills installed (rust-best-practices, rust-async-patterns, rust-testing, context-engineering — project-local, Snyk-flag read); 3 Kalaris-authored skills (builder, harness-loop, testing); CI workflow; dual license; NOTICE; ROADMAP; this proposal; the research paper; 14 research notes + prior-phase notes (20 total). **Next:** v0.2 session loop behind `docs/ARCHITECTURE.md`.

---

## 12. Research index (all Kalki Code notes, 2026-11-08)

`research_notes/Kalki Code/`: prime_agent_rust_packaging · tui_stack · provider_layer · wal_durability · sandbox_permissions · git_integration · release_distribution · ci_cd_hygiene · benchmarks_evals · reliability_observability · security_threat_model · deployment_engineering · context_engineering · governance_licensing — plus the first-phase corpus `Self recursive continual harness design/` (prime_intellect, google_harness2, arxiv_landscape, long_horizon_memory, industry_harnesses, rust_implementation).
