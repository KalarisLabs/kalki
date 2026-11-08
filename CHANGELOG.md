# Changelog

All notable changes to Kalki Code. Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Versions follow SemVer. v0.x = pre-stable; breaking changes may land in minors.

## [Unreleased]

### Added
- v0.2.0 (in development): event variants `ToolCall`, `ToolResult`, `Usage`, `Anchor` in `kalki-protocol`
- WAL segment layer with CRC32C framing, 64 MiB rotation, torn-write truncation (impl/wal)
- Provider wires: OpenAI Responses, Anthropic Messages, OpenAI-compatible Chat Completions (impl/providers)
- Tool layer: deny-first TOML policy engine (duplicate-rule detection), argv-spawned `bash` runner with workspace confinement heuristics + 4 KiB inline cap + blob overflow, `apply_patch` v1, secret scrubbing (impl/tools)
- `.changes/` fragments; `cargo xtask ci` gate green on all crates
- Skills installed project-locally (rust-best-practices, rust-async-patterns, rust-testing, context-engineering) + Kalaris-authored `kalki-code-builder/harness-loop/testing`

### Changed
- Distribution for v0.x is **GitHub Releases only** (cargo dist + minisign); Homebrew/npm/crates.io deferred to 1.0
- LICENSE remains dual MIT OR Apache-2.0; DCO required

### Removed
- Trademark-clearance milestone dropped (GitHub Releases is the release channel for v0.x)

## [0.1.0] - 2026-11-08

### Added
- Initial workspace: `kalki-protocol`, `kalki-core`, `kalki` (doctor), `xtask`; WAL record codec (pre-CRC); CI; dual license; NOTICE; AGENTS.md; DEPENDENCIES.md
