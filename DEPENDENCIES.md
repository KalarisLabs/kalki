# Dependencies

**Owner:** Sayan Chowdhury, Kalaris Labs · **Last review:** 2026-11-11 · **Next review:** 2027-02-11

Rules:

- Every dependency change updates this file and says why in the pull request.
- A dependency is added when the code that needs it lands, never before.
- `Cargo.lock` is committed and CI builds with `--locked`.
- `deny.toml` is the enforced policy. CI runs `cargo deny check bans licenses sources`
  on every pull request and `cargo deny check advisories` daily.
- Every dependency must build on Rust 1.88. CI checks that.

Versions below are the ones resolved in `Cargo.lock` on 2026-11-11.

## In the build

| Crate | Version | Used by | Purpose | If it had to go |
|---|---|---|---|---|
| tokio | 1.54.1 | core, cli | Async runtime: processes, files, timers | No practical replacement |
| reqwest | 0.13.5 | core | HTTP client and byte streams for the provider wires | `hyper` directly |
| rustls | 0.23.46 | via reqwest | TLS. No OpenSSL, no native TLS. | None |
| aws-lc-rs | 1.19.0 | via rustls | Cryptography for TLS. **Compiles C code, so builds need a C compiler.** | `ring` backend |
| serde | 1.0.229 | all | Serialization | None |
| serde_json | 1.0.154 | core, cli | JSON for events and provider wires | None |
| toml | 1.2.1 | core | Policy file parsing | None worth the cost |
| crc32c | 0.6.9 | core | Checksums on log records | `crc` crate |
| sha2 | 0.11.1 | core | SHA-256 for blob addressing, and for the planned hash chain | None |
| futures-util | 0.3.34 | core | `StreamExt` over the response byte stream | Hand-written poll loop |
| thiserror | 2.0.21 | protocol, core | Typed errors in libraries | Manual impls |
| tracing | 0.1.44 | core, cli | Structured logs | `log` |
| tracing-subscriber | 0.3.23 | cli | Log output, plain and JSONL | None |
| clap | 4.6.7 | cli | Command-line parsing | `pico-args` |
| anyhow | 1.0.104 | cli only | Error glue in the binary. Never in libraries. | `eyre` |

`xtask` has no dependencies.

## Test only

| Crate | Version | Purpose |
|---|---|---|
| axum | 0.8.9 | The loopback mock model server in end-to-end tests |
| tempfile | 3.27.0 | Temporary directories for log and workspace tests |

## Build and CI tools

| Tool | Where | Purpose |
|---|---|---|
| cargo-nextest | CI | Test runner. Doctests run separately with `cargo test --doc`. |
| cargo-deny | CI and local (0.20.2 used locally) | Licenses, bans, sources, advisories |
| GitHub Actions | CI | Each action is pinned to a commit SHA. Dependabot proposes updates weekly. |

## License policy

Allowed, as listed in `deny.toml`: MIT, MIT-0, Apache-2.0, Apache-2.0 with the
LLVM exception, BSD-3-Clause, ISC, Unicode-3.0. Anything else fails CI. On
2026-11-11 the full dependency tree passed.

## Planned, not in the build

These were researched on 2026-11-08. Recheck the version, the MSRV, and the
license before adopting any of them.

| Crate | For | Milestone |
|---|---|---|
| ratatui, crossterm | Terminal UI | v0.0.3 |
| insta, vt100 | Terminal UI frame tests (dev only) | v0.0.3 |
| landlock, seccompiler | Linux sandbox | v0.0.3 |
| gix | In-process git status, diff, and checkpoint refs. Until then Kalki shells out to `git`. | v0.0.3, behind a benchmark |

## Removed

| Crate | Removed | Why |
|---|---|---|
| async-openai | 2026-11-11 | Declared and never called. All three provider wires are hand-written on `reqwest`. |
| rusqlite | 2026-11-11 | Declared and never called. It also required Rust 1.89, which broke the 1.88 MSRV. The log is the store. |
| tracing-appender | 2026-11-11 | Declared and never called. |

## Banned

Enforced by `deny.toml`. Do not re-add without an ADR.

| Crate | Why |
|---|---|
| openssl-sys, native-tls | One TLS stack: rustls. Keeps release builds static. |
| git2 | C dependency. Use the `git` CLI now and `gix` later. |
| sled | Unmaintained |

Also rejected without a ban entry: agent frameworks as a base (`rig-core` and
similar), and `cedar-policy` until the TOML policy is outgrown (ADR-0003).
