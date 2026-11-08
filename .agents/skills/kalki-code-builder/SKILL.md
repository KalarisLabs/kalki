---
name: kalki-code-builder
description: Kalki Code architecture and conventions — load before writing or reviewing any kalki-code code. Enforces crate boundaries, error handling, persistence rules, and the definition of done.
---

# Kalki Code Builder (Kalaris Labs)

Author: Sayan Chowdhury. Load this before touching Rust in this repo.

## Read order for a fresh task (< 10 min)
1. `AGENTS.md` (numbered rules)
2. `docs/ARCHITECTURE.md` (crates graph)
3. `docs/PROPOSAL.md` §4 (design of the thing you're building)
4. `DEPENDENCIES.md` (what you may add) — never add one without updating it

## Crate boundaries (Cargo enforces most)
- `kalki-protocol`: types only; serde/thiserror only; no I/O, no async.
- `kalki-core`: WAL, sessions, providers, supervisor; never prints to stdout; no UI types.
- `kalki` (CLI/TUI): owns terminal; business logic forbidden; calls `kalki-core`.
- `kalki-tui` (when added): depends on `kalki-protocol` only.
- New crate? Needs an ADR first.

## House rules
- No `unsafe_code` (`forbid` at workspace).
- Libraries: no `unwrap`/`expect`/`panic!`; errors via `thiserror`. Binaries: `anyhow` allowed.
- WAL is the only persistence. Hash-chained, CRC-framed. Boot = verify anchor + 500-event tail, then background verify.
- Tools return handles, never payloads; > 4 KiB goes to the blob store.
- No telemetry. No network except provisioned provider endpoints + explicit grants.
- `tracing` spans everywhere; no `println!` in libs; `kalki-core` never writes to the terminal.
- Config precedence: file < env (`KALKI_*`) < CLI flags.

## Definition of done
- `cargo xtask ci` green (fmt, clippy -D warnings, nextest, doc build).
- New crate/public API → docs + DEPENDENCIES.md/ADR touched; changelog fragment added.
- Unsafe-adjacent or WAL/hash code → kill -9 test added to `tests/crash/`.
- Evidence in PR: what was measured, not "works on my machine".

## If blocked
- Architecture question → `docs/adr/` then ask maintainers.
- Benchmark needed → see `kalki-code-testing` skill.
