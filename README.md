# kalki-code

**Kalki Code** — a self-recursive continual coding harness. Rust, terminal-first, built to outlast a session and learn from itself. By [Kalaris Labs](https://kalarislabs.com) (Sayan Chowdhury).

> ⚠️ **Pre-alpha (v0.1.0).** The scaffold compiles and `kalki doctor` runs. No session loop yet — see [ROADMAP.md](ROADMAP.md) and [docs/PROPOSAL.md](docs/PROPOSAL.md). Nothing here is production-ready.

## What it is

- **Terminal-native harness** in Rust — single static binary, daemon + worker-process-per-session, attach/detach long sessions.
- **Continual memory** — append-only hash-chained WAL; context is a compiled, disposable cache; nothing is "summarized and forgotten".
- **Self-recursive** — the harness edits its own genome (prompts, skills, memory) behind **oracle-gated admission control** with instant rollback.
- **Long-horizon by design** — targets 1000-hour sessions: crash-only workers, replay-on-boot, SILO'd sandboxes per tool call.
- **Your keys, your machine** — local-first, zero telemetry by default, audit hash-chain baked in.

Inspired by [Prime Intellect's Prime Agent](https://arxiv.org/abs/2608.23552) and [Pi](https://github.com/earendil-works/pi). Not affiliated with or endorsed by them. See [NOTICE](NOTICE).

## Install (preview)

```bash
# Linux / macOS
curl -fsSL https://kalarislabs.com/kalki/install.sh | sh
# Windows (PowerShell, beta)
irm https://kalarislabs.com/kalki/install.ps1 | iex
```

## First run

```bash
kalki            # detects git repo + provider keys (OPENAI_API_KEY, ANTHROPIC_API_KEY, ...)
kalki doctor     # environment diagnostics
```

## Repository layout

```
crates/kalki-protocol   shared wire/event types (deps: serde only)
crates/kalki-core       WAL, sessions, provider client, supervisor (no UI)
crates/kalki            the `kalki` CLI/TUI binary
docs/                   proposal, architecture, ADRs
```

## Authorship & license

Author: **Sayan Chowdhury** · Kalaris Labs · hello@kalarislabs.com. Dual-licensed `MIT OR Apache-2.0` — see [LICENSE-APACHE](LICENSE-APACHE) / [LICENSE-MIT](LICENSE-MIT) / [NOTICE](NOTICE).

Contributions: `Signed-off-by` (DCO) required — see [CONTRIBUTING.md](CONTRIBUTING.md).
