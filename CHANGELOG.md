# Changelog

All notable changes to Kalki. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
While the version is `0.0.x`, any release may break anything.

## [Unreleased]

## [0.0.1] - unreleased

The first public release. A headless agent loop on a durable log. No terminal
UI, no sandbox, no self-improvement yet.

### Added

- `kalki run -p "<objective>" [--model] [--max-turns]`: one headless session against the configured provider.
- `kalki doctor`: environment checks.
- Event log: length-prefixed JSONL records with CRC32C, segments that rotate at 64 MiB, torn-tail repair at open, replay, and kill drills.
- Event types: `Message`, `ToolCall`, `ToolResult`, `Usage`, `Compaction`, `Lifecycle`, `Anchor`.
- Content-addressed blob store. Tool output over 4 KiB is stored by SHA-256 and referenced by handle.
- Deny-first policy engine in TOML with stable rule IDs, duplicate-rule detection, and multi-token command prefixes.
- Tools: `bash` (argv array, workspace path confinement, policy-checked) and `apply_patch` (workspace path confinement; not yet policy-checked).
- Provider wires, hand-written on `reqwest`: OpenAI Responses, Anthropic Messages, OpenAI-compatible Chat Completions. Selected with `KALKI_PROVIDER`. Cached-token accounting.
- Session loop: objective, model turns, policy-checked tool calls, turn cap, abort markers.
- End-to-end tests against a loopback mock model server.
- CI: format, clippy, tests on Linux x86_64 and aarch64, macOS Apple Silicon and Intel, and Windows; MSRV 1.88; rustdoc; `cargo-deny`; DCO check.
- Release workflow: five targets, `SHA256SUMS`, build provenance attestation, draft pre-release.
- `cargo xtask lint`: a file-size ratchet with remediation text.
- Documentation: architecture, product proposal, whitepaper proposal, compatibility matrix, terminal UI specification, literature map, release guide, ADRs 0001 to 0006.
- Community files: code of conduct, governance, support, issue and pull request templates, code owners, dependabot.
- Agent skills in `.agents/skills/`, including `caveman` and `ponytail`.

### Changed

- The version is `0.0.1`. The workspace was numbered `0.1.0` and never released (ADR-0005).
- The repository is `github.com/KalarisLabs/kalki`.
- `cargo xtask ci` runs rustdoc with `-D warnings`, matching CI.

### Fixed

- The SSE decoder flushes on a blank line for both field orders. Streams that send `event:` before `data:` previously lost every frame.
- The CI workflow pinned an `actions/checkout` commit that does not exist. All actions are now pinned to verified commits.
- Four broken intra-doc links that failed the rustdoc job.
- The workspace did not build on its declared MSRV. An unused dependency required Rust 1.89, and one test relied on newer temporary-lifetime rules.
- Double-encoded UTF-8 in the end-to-end test file, including the fixture for the Unicode round-trip test.

### Removed

- `rusqlite`, `async-openai`, and `tracing-appender` were declared and never used. `axum` is now a dev-dependency.
