# Changelog

All notable changes to Kalki Code. Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Versions follow SemVer. v0.x = pre-stable; breaking changes may land in minors.

## [Unreleased] — v0.2.0 (session loop landed on main, tag pending P9)

### Added
- v0.2 event variants `ToolCall`, `ToolResult`, `Usage`, `Anchor` in `kalki-protocol` (hash-chain wiring deferred to v0.3)
- Mock-model e2e + kill -9 drill + turn-cap/torn-stream tests on main (`cargo test --test e2e_mock`, 5 tests, `ef8879b`)
- **Fixed: SSE decoder now flushes on blank line (both wire orders — previously only `data:`-first transcripts decoded; the real Responses/Anthropic `event:`-first order lost every frame) — caught by the e2e suite, fixed in `provider/mod.rs`**
- WAL segment layer: CRC32C framing, 64 MiB rotation, torn-tail repair at open, `replay_segment`, kill -9 drills (11 tests) - branch merged (`impl/wal`)
- Provider wires with `AnyProvider` env selection (`KALKI_PROVIDER`): OpenAI Responses, Anthropic Messages, OpenAI-compatible Chat Completions — hand-rolled SSE decoding, cached-token accounting, `idx:N` tool-arg correlation — 45 unit tests (`impl/providers`)
- Tool layer: deny-first TOML policy engine (duplicate-rule detection, multi-token argv prefixes, scrubbing), argv-spawned `bash` runner with workspace confinement + 4 KiB inline cap + blob overflow, `apply_patch` v1 — 28 tests (`impl/tools`)
- Session FSM (`kalki_core::session`): user objective → model turns interleaved with gated tool calls → WAL; turn cap; abort markers in WAL; `run_from_env`
- `kalki run -p "..." --model --max-turns` headless one-shot; `kalki doctor` env checks
- `.changes/` fragments; `cargo xtask ci` gate green (fmt, clippy `-D warnings`, tests, docs)
- ADR-0003 (Cedar policy, v1.5 sketch)

### Changed
- Distribution for v0.x is **GitHub Releases only** (cargo dist + minisign); Homebrew/npm/crates.io deferred to 1.0
- `cargo xtask ci` now also gates on `clippy --all-targets -D warnings` being clean workspace-wide

### Removed
- Trademark-clearance milestone dropped (GitHub Releases is the release channel for v0.x)

## [0.1.0] - 2026-11-08

### Added
- Initial workspace: `kalki-protocol`, `kalki-core`, `kalki` (doctor), `xtask`; WAL record codec (pre-CRC); CI; dual license; NOTICE; AGENTS.md; DEPENDENCIES.md
