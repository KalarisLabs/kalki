# Contributing to Kalki

Contributions are welcome, from people and from coding agents directed by
people. Kalki is pre-release (v0.0.x), so the most useful contributions right
now are bug reports, compatibility reports for models and platforms, and small
fixes.

## Before you write code

1. Read [AGENTS.md](AGENTS.md). Its rules are binding and CI checks most of them.
2. Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), especially section 12,
   which lists what exists today.
3. Check [ROADMAP.md](ROADMAP.md). Work that belongs to a later milestone is
   likely to be asked to wait.
4. Open an issue or a discussion before changing the event types, the log
   format, the policy engine, executors, the admission gate, or the license.
   Those changes need an ADR in `docs/adr/`. Copy `0000-template.md`.

## Set up

You need Rust 1.88 or newer, a C compiler, and `git`.

```bash
git clone https://github.com/KalarisLabs/kalki
cd kalki
cargo xtask ci
```

`cargo xtask ci` runs the format check, the file-size lint, clippy, the tests,
and rustdoc. It needs no network and no API key: tests run against a mock
model server on loopback.

Optional, to match CI exactly:

```bash
cargo install cargo-deny --locked && cargo deny check
rustup toolchain install 1.88.0 && cargo +1.88.0 check --workspace --all-targets --locked
```

## Make the change

- One logical change per pull request. Small is good.
- Add a test that fails without your change.
- Add a line to `CHANGELOG.md` under `[Unreleased]` if behaviour changed.
- If you add or update a dependency, update [DEPENDENCIES.md](DEPENDENCIES.md)
  and say why in the pull request.
- Use [Conventional Commits](https://www.conventionalcommits.org):
  `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `ci`, `chore`.

## Sign your commits

Every commit must carry a Developer Certificate of Origin sign-off:

```bash
git commit -s -m "fix(core): repair torn tail before rotation"
```

By signing off you certify the
[DCO 1.1](https://developercertificate.org). There is no CLA. CI rejects pull
requests with unsigned commits.

If a coding agent wrote the change, the person directing it signs off, and the
commit names the agent in a `Co-Authored-By` trailer.

## Open the pull request

Push to your fork and open a pull request against `main`. The template asks
for three lists: what you measured, what you assumed, and what you did not
verify. Fill in all three. "Unmeasured" is an acceptable answer. A missing
list is not.

Reviews argue against the change on purpose. That is the process, not a
judgment of the contributor.

## Reporting compatibility

A report that a model, a provider, a terminal, or a platform works or fails is
a contribution. Open an issue with the output of `kalki doctor`, the provider
settings without the key, and what happened.
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) marks a row "Verified" only
when someone has reported it.

## Security

Report vulnerabilities privately. See [SECURITY.md](SECURITY.md). Do not open a
public issue for them.
