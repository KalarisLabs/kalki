# Kalki Code — Dependency Decision Table

**Owner:** Kalaris Labs (Sayan Chowdhury) · **Last review:** 2026-11-08 · **Next review:** 2027-02-08 (quarterly)
Rules: every dependency change must update this file and `docs/adr/`. CI enforces `cargo deny`.
Exact versions = `Cargo.lock`-resolved as of 2026-11-08. All Rust deps are MSRV-compatible with rustc 1.88.

## Core (v0.1 — first build, required)

| Dependency | Exact version | Purpose | Required / Optional | License | OS support | Security risk | Fallback | Benchmark needed | Owner | Removal / Review |
|---|---|---|---|---|---|---|---|---|---|---|
| tokio | 1.54.1 | async runtime (process-per-session, I/O, timers) | Required | MIT | Linux / macOS / Windows | Low (ubiquitous, audited) | none viable | No (soak RSS test in CI instead) | Sayan Chowdhury | 2027-02-08 |
| reqwest | 0.13.5 | HTTP client for provider APIs + SSE | Required | MIT / Apache-2.0 | Linux / macOS / Windows | Medium (TLS stack; keep updated; rustls avoids OpenSSL) | hyper direct | No (connection-pool soak test) | Sayan Chowdhury | 2027-02-08 |
| rustls | 0.23.46 | TLS (via reqwest rustls feature; no native TLS) | Required (transitive, pinned) | MIT / Apache-2.0 / ISC | Linux / macOS / Windows | Medium (TLS-critical; bump with reqwest) | none | No | Sayan Chowdhury | 2027-02-08 |
| async-openai | 0.43.0 | Reference impl for OpenAI/Responses wire; isolated behind internal `Provider` trait | Required v0.1 (quarantine pending) | MIT | Linux / macOS / Windows | Medium (0.x churn; API-surface mismatch risk) | Internal thin client on reqwest (documented in ADR-0002) | Yes — provider conformance suite (all live endpoints) | Sayan Chowdhury | 2027-02-08 or on switch to internal client |
| rusqlite (bundled) | 0.41.1 | WAL-mode SQLite: session index + control tables | Required | MIT | Linux / macOS / Windows | Low | fjall (append-heavy path) | Yes — append/replay bench vs fjall (in progress) | Sayan Chowdhury | 2027-02-08 |
| libsqlite3-sys (bundled) | 0.39.1 | SQLite engine (bundled, no system dep) | Required (transitive) | MIT | Linux / macOS / Windows | Low | same as rusqlite | same as rusqlite | Sayan Chowdhury | 2027-02-08 |
| JSONL/WAL segments (ours) | n/a — our code | Append-only event log; CRC-framed, hash-chained | Required | N/A (ours) | Linux / macOS / Windows | Design-controlled (tamper-evident, not tamper-proof) | bincode 3 / rmp-serde encoding switch | Yes — replay <5 s @ 3.6M events; kill -9 drills | Sayan Chowdhury | every milestone |
| serde | 1.0.229 | serialization | Required | MIT / Apache-2.0 | Linux / macOS / Windows | Low | none | No | Sayan Chowdhury | 2027-05-08 |
| serde_json | 1.0.154 | JSON for config/events/log | Required | MIT / Apache-2.0 | Linux / macOS / Windows | Low | rmp-serde for hot paths | No | Sayan Chowdhury | 2027-05-08 |
| tracing | 0.1.44 | structured spans/logs (daemon, core) | Required | MIT | Linux / macOS / Windows | Low | log crate | No | Sayan Chowdhury | 2027-05-08 |
| tracing-subscriber | 0.3.23 | fmt/JSONL log sinks | Required | MIT | Linux / macOS / Windows | Low | none | No | Sayan Chowdhury | 2027-05-08 |
| axum | 0.8.9 | loopback/unix-socket health + control surface (`/healthz`, `/metrics`) | Required (small, replaceable) | MIT | Linux / macOS / Windows | Medium-low | tiny hyper service | No | Sayan Chowdhury | 2027-02-08 |
| clap | 4.6.7 | CLI parsing (`kalki` binary) | Required | MIT / Apache-2.0 | Linux / macOS / Windows | Low | pico-args | No | Sayan Chowdhury | 2027-05-08 |
| thiserror | 2.0.21 | typed errors (libraries) | Required | MIT / Apache-2.0 | Linux / macOS / Windows | Low | manual impls | No | Sayan Chowdhury | 2027-05-08 |
| anyhow | 1.0.104 | error glue (binary only, never in libs) | Required (bin) | MIT / Apache-2.0 | Linux / macOS / Windows | Low | eyre | No | Sayan Chowdhury | 2027-05-08 |

| TOML config + policy engine files | toml 1.2.1 | Parse workspace `.kalki.toml`, XDG config, and the deny-first policy TOML (v1; Cedar comes in v1.5+) | Required | MIT / Apache-2.0 | Linux / macOS / Windows | Low | hand-rolled parser (rejected — why? toml is tiny and spec-pinned) | No | Sayan Chowdhury | 2027-02-08 |
| crc32c | 0.6.9 | CRC-32C checksums for WAL record torn-write detection | Required | MIT / Apache-2.0 | Linux / macOS / Windows | Low | `crc` crate (not chosen — crc32c matches RFC 3720/SCTP framing lineage) | No | Sayan Chowdhury | 2027-05-08 |

## Git integration (decided, adopt at M2) — benchmark gate before merge

| Dependency | Exact version | Purpose | Required / Optional | License | OS support | Security risk | Fallback | Benchmark needed | Owner | Removal / Review |
|---|---|---|---|---|---|---|---|---|---|---|
| gix (gitoxide) | 0.90.0 | in-process status/diff/worktree/checkpoint refs | Required (M2) | MIT / Apache-2.0 | Linux / macOS / Windows | Low-medium | git CLI ≥ 2.31 shell-out | Yes — status/diff on 1M-file monorepo; worktree create time | Sayan Chowdhury | 2027-02-08 |
| gix-status | 0.37.0 | status incl. renames | Required (M2, transitive) | MIT / Apache-2.0 | same | same | same | same | same | 2027-02-08 |
| gix-diff | 0.70.0 | in-process diff for review panel | Required (M2, transitive) | MIT / Apache-2.0 | same | same | unidiff 0.4.1 (import/export) | same | same | 2027-02-08 |
| git CLI | host-provided | LFS, hooks, `git apply --3way`, CI oracle | Optional v1 (required for LFS workflows) | GPL-2.0 (subprocess, no linkage) | Linux / macOS / Windows (git required) | Low (subprocess sandbox for mutations) | pure-gix paths | No | Sayan Chowdhury | 2027-05-08 |

Decision source: `research_notes/Kalki Code/git_integration.md` (gix-first hybrid; git2 rejected).

## TUI (adopt at M2 — after ratatui recheck)

| Dependency | Exact version | Purpose | Required / Optional | License | OS support | Security risk | Fallback | Benchmark needed | Owner | Removal / Review |
|---|---|---|---|---|---|---|---|---|---|---|
| ratatui | 0.30.2 | TUI framework | Optional (M2) | MIT | Linux / macOS / Windows | Low | inline vt100 renderer (ours) | Yes — frame time under token streaming | Sayan Chowdhury | 2027-02-08 |
| crossterm | 0.29.0 | terminal backend | Optional (M2) | MIT | Linux / macOS / Windows | Low | ratatui backend abstraction | No | Sayan Chowdhury | 2027-02-08 |
| ratatui-crossterm | 0.1.2 | split backend (0.30 line) | Optional (M2) | MIT | Linux / macOS / Windows | Low | direct crossterm | No | Sayan Chowdhury | 2027-02-08 |
| tui-textarea-2 | 0.13.2 | multi-line editor | Optional (M2) | MIT | Linux / macOS / Windows | Low | manual line editor | No | Sayan Chowdhury | 2027-02-08 |
| insta | 1.49.0 | snapshot frame tests | Dev-required | MIT / Apache-2.0 | Linux / macOS / Windows | Low | none | No | Sayan Chowdhury | 2027-05-08 |
| vt100 | 0.18.1 | differential frame diffing (Codex VT100Backend pattern) | Dev-required | MIT | Linux / macOS / Windows | Low | insta only | No | Sayan Chowdhury | 2027-05-08 |

Decision source: `research_notes/Kalki Code/tui_stack.md`.

## Dev / CI only (never shipped)

| Dependency | Version | Purpose | License | Notes |
|---|---|---|---|---|
| cargo-nextest | ≥ 0.9.150 (pinned in CI) | test runner (3× faster; GHSA-vc25-qg58-w57c) | Apache-2.0 / MIT | doctests run separately |
| cargo-deny | latest | advisories + license allowlist (MIT/Apache/BSD/ISC) + bans | MIT / Apache-2.0 | CI weekly + per-PR |
| cargo-audit | latest | daily advisory scan | MIT / Apache-2.0 | scheduled workflow |
| cargo-llvm-cov | latest | coverage (non-blocking) | Apache-2.0 / MIT | informational |
| cargo-semver-checks | v2 action | breaking-change gate on kalki-protocol/core | MIT / Apache-2.0 | PRs only |
| git-cliff | latest | changelog from conventional commits | MIT | release flow |
| cargo-dist | v0.34.0 | release builds + installers + Homebrew formula | MIT | tag-triggered release.yml (M3) |

## Explicitly rejected (do not re-add without an ADR)

| Dependency | Why | Source |
|---|---|---|
| sled | alpha since 2024; unmaintained | rust_implementation.md |
| TensorZero | archived 2026-06 | rust_implementation.md |
| rustformers/llm | archived 2024-06 | rust_implementation.md |
| git2 / libgit2 | C dep, GPL-2.0+linking exception, slow upstream | git_integration.md |
| seccomp (old crate) | stale since 2020 | rust_implementation.md |
| event-sourcing / saga / durable crates | toy-grade (870 dl) | rust_implementation.md |
| rig-core (as framework) | 0.x churn; use behind trait if ever | rust_implementation.md |
| cedar-policy | great, but not for v1: hand-rolled TOML rule IDs first | sandbox_permissions.md |

## Notes

- `async-openai` is quarantined: all call sites go through `kalki_core::provider::Provider`. First milestone that passes the provider conformance matrix may keep it; otherwise we swap to a hand-rolled client on reqwest (ADR-0002).
- `openssl`/native TLS stacks are banned; rustls only. Bundled SQLite only; no system sqlite linkage.
- Any dependency with a known advisory fails CI via `cargo deny --deny warnings`.
