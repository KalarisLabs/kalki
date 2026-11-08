# Contributing to Kalki Code

Thanks for helping build the harness better agents deserve. Kalaris Labs welcomes all contributions under DCO.

## Before you write code

1. Read `AGENTS.md` (rules), `docs/ARCHITECTURE.md` (structure), and `DEPENDENCIES.md` (what you may add).
2. Open an issue or discussion first for anything touching: wire protocol, WAL format, permissions/sandbox, self-recursion gates, or licensing.
3. Breaking changes require an accepted RFC in `docs/rfc/`.

## Developer Certificate of Origin

Every commit must carry a DCO sign-off:

```bash
git commit -s -m "feat(core): WAL segment rotation"
```

By signing off you certify the Developer Certificate of Origin 1.1 (https://developercertificate.org). We do not use a CLA.

## The gate

```bash
cargo xtask ci   # formats, lints, tests, doc build — must pass locally
```

Rules agents may rely on:

- TUI crate sees only `kalki-protocol` types. No business logic in the TUI.
- Persistence goes through the WAL only. No hidden files, no side channels.
- Library crates never use `unwrap`/`expect`/`panic` outside tests.
- Every dependency change updates `DEPENDENCIES.md` and needs maintainer sign-off.
- No `unsafe_code` (workspace `forbid`), except in crates explicitly granted it in `docs/adr/`.

## Commit and PR style

- Conventional Commits (`feat|fix|docs|refactor|test|perf|ci|chore`).
- Small PRs; one logical change; add/extend tests; update `CHANGELOG.md` (Keep a Changelog).
- PR template checklist: tests, docs, DEPENDENCIES.md if deps changed, security notes if sandbox/policy touched.

## Security

Please report vulnerabilities privately via `SECURITY.md`. Do not open public issues for them.
