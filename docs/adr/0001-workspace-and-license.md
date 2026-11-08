# ADR-0001: Rust workspace, dual license, DCO

- Status: accepted
- Date: 2026-11-08
- Deciders: Sayan Chowdhury

## Context

Kalki Code must be adoptable by institutions (patent clarity, auditability, single static binary), and built ground-up by agents without hesitation. A monolithic TS codebase lost this in Prime Agent's own rewrite narrative; the fix was Cargo workspace + one-way graph.

## Decision

Rust 1.88 workspace: `crates/kalki-protocol`, `crates/kalki-core`, `crates/kalki` (+ `kalki-tui` M2, `xtask`). One-way graph; Cargo enforces it. Dual license `MIT OR Apache-2.0`. DCO sign-off, no CLA. `unsafe_code = forbid` workspace-wide. Libraries never `panic`; binaries may.

## Consequences

Institutional adoption possible without copyleft; Cargo catches boundary violations at compile time; agents get a hard, machine-checkable architecture. Costs: boilerplate (protocol types), compile times; mitigated by sccache + mold linker in CI.

## Alternatives considered

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| Go | fast builds, simple | no trait-level sandbox story as strong; weaker enum exhaustiveness | rejected |
| TypeScript monorepo | fastest iteration | exactly what Prime Agent rewrote away from | rejected |
| Python + same architecture | ML ecosystem | weeks-long daemon reliability + single static binary are penalties | rejected |
| GPL/AGPL | copyleft leverage | kills institutional adoption (Zed's GPL learnings) | rejected |

## Verification

Every CI run proves the graph compiles one-way; license allowlist in `cargo deny` proves the dual license holds; `kalki` builds to one static musl binary < 30 MB target.
