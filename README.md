# Kalki

[![ci](https://github.com/KalarisLabs/kalki/actions/workflows/ci.yml/badge.svg)](https://github.com/KalarisLabs/kalki/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

**A coding harness that improves itself, and has to prove each improvement.**

Kalki runs a coding agent in your terminal and records what it does in an
append-only log. The design lets the agent propose changes to its own prompts,
skills, and context policy. A gate that the agent cannot write to re-runs the
failures behind a change, a protected set of cases that pass today, and
held-out tasks, and measures cost. A change is kept only if it helps and breaks
nothing that worked.

One Rust binary. Any model. No telemetry. By [Kalaris Labs](https://kalarislabs.com).

> **Status: v0.0.1, pre-release.** Kalki today is a headless agent loop with a
> durable log, three provider wires, and a deny-first policy. It has no
> terminal UI and no sandbox, and it does not yet improve itself. See
> [what works today](#what-works-today). Do not run it on a workspace or with a
> model you do not trust, except inside a container or virtual machine.

## Why

Agents that rewrite their own harness score higher on benchmarks. Left to
judge their own changes, they also break things that worked, overfit their
test tasks, and get more expensive. In one 2026 study, 55% of the
self-modifications a reviewer rejected had fixed the failure they targeted and
broken something else ([arXiv:2609.24130](https://arxiv.org/abs/2609.24130)).

Kalki is built around the reviewer. The design is in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and the reasoning is in
[docs/PROPOSAL.md](docs/PROPOSAL.md).

## Install

Download the archive for your platform from the
[releases page](https://github.com/KalarisLabs/kalki/releases), unpack it, and
put `kalki` on your `PATH`.

| Platform | Archive |
|---|---|
| Linux x86_64 | `kalki-v0.0.1-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 | `kalki-v0.0.1-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple Silicon | `kalki-v0.0.1-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `kalki-v0.0.1-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `kalki-v0.0.1-x86_64-pc-windows-msvc.zip` |

Or build from source with Rust 1.88 or newer:

```bash
git clone https://github.com/KalarisLabs/kalki
cd kalki
cargo build --release --package kalki   # binary: target/release/kalki
```

## Quick start

**1. Decide what the agent may run.** Kalki denies any command that no rule
allows. With no policy file it uses a built-in default that allows `ls`, `rg`,
`cat`, `head`, `tail`, `pwd`, `git`, `cargo`, and `rustc`. That default allows
all of `git` and `cargo`, including `git push` and build scripts.

To set your own rules, write a policy file. It replaces the default. On Linux
and macOS it lives at `~/.config/kalki/policy.toml` (or under
`$XDG_CONFIG_HOME`). On Windows it lives at `%APPDATA%\kalki\policy.toml`.

```toml
# Rules are checked in order and the first match wins, so denials go first.
[[rules]]
id = "deny-publish-and-push"
action = "deny"
tools = ["bash"]
commands = ["cargo publish", "git push"]

[[rules]]
id = "allow-build-and-read"
action = "allow"
tools = ["bash"]
commands = ["cargo", "git", "ls", "cat", "rg"]
```

Two limits in v0.0.1: an `ask` rule is treated as a denial, because there is
no prompt yet, and `apply_patch` is confined to the workspace but is not
checked against policy rules.

**2. Point Kalki at a model.** For a local open-weight model served by Ollama:

```bash
export KALKI_PROVIDER=openai_compat
export OPENAI_COMPAT_BASE_URL=http://localhost:11434/v1
export OPENAI_COMPAT_API_KEY=local        # any non-empty value for local servers
export KALKI_MODEL=qwen3-coder
```

**3. Check the setup, then run a task** from inside a git repository:

```bash
kalki doctor
kalki run -p "run the tests and tell me which ones fail" --max-turns 20
```

`--max-turns` defaults to 200. Without `KALKI_PROVIDER`, Kalki uses the OpenAI
Responses wire and reads `OPENAI_API_KEY`.

Other providers and model servers are listed in
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).

## What works today

| Works in v0.0.1 | Not yet |
|---|---|
| `kalki run`: a headless session with a turn cap | Terminal UI |
| Append-only session log with checksums, rotation, and crash recovery | Hash chain and `kalki audit verify` |
| Large tool output stored by hash, kept out of the context | Sandbox |
| Deny-first policy with named rules, applied to `bash` | Policy on `apply_patch`; rewind |
| Tools: `bash` (argv only, no shell string) and `apply_patch` | Self-improvement: genome and gate |
| OpenAI Responses, Anthropic Messages, OpenAI-compatible wires | Pi as an executor |

The plan for each missing piece is in [ROADMAP.md](ROADMAP.md).

## How it is put together

```
surfaces     terminal UI · headless CLI · JSONL protocol
kernel       event log · policy · checkpoints · context compiler · provider gateway
executors    native Rust loop · Pi over RPC
genome       prompts, skills, context policy: what self-improvement may change
gate         eight checks a change must pass before it is kept
```

| Path | Contents |
|---|---|
| `crates/kalki-protocol` | Event and wire types |
| `crates/kalki-core` | The kernel: log, blobs, policy, providers, session loop, tools |
| `crates/kalki` | The `kalki` binary |
| `xtask` | `cargo xtask ci`, the local gate |
| `docs/` | Architecture, proposal, whitepaper, compatibility, ADRs |
| `.agents/skills/` | Skills for coding agents that work on this repository |

## Documentation

| Read | To learn |
|---|---|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | The design, and what is built against what is planned |
| [docs/PROPOSAL.md](docs/PROPOSAL.md) | What Kalki is for and how success is measured |
| [docs/WHITEPAPER.md](docs/WHITEPAPER.md) | The admission rule, hypotheses, and evaluation plan |
| [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) | Operating systems, terminals, models, providers |
| [docs/research/LITERATURE.md](docs/research/LITERATURE.md) | The papers behind the design |
| [ROADMAP.md](ROADMAP.md) | What comes next, with exit criteria |
| [AGENTS.md](AGENTS.md) | Rules for coding agents and humans working in this repository |

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) first. In
short: run `cargo xtask ci`, sign off your commits with `git commit -s`, and
open an ADR before changing the protocol, the log format, the policy engine,
or the gate.

Questions go to [Discussions](https://github.com/KalarisLabs/kalki/discussions).
Vulnerabilities go to [SECURITY.md](SECURITY.md). Conduct is covered by the
[Code of Conduct](CODE_OF_CONDUCT.md), and decision-making by
[GOVERNANCE.md](GOVERNANCE.md).

## Acknowledgements

Kalki's design draws on [Pi](https://github.com/earendil-works/pi) by Earendil
Works, Prime Agent by Prime Intellect, Codex CLI by OpenAI, and the research
listed in the literature map. Kalki is not affiliated with or endorsed by any
of them. See [NOTICE](NOTICE).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. Author: Sayan Chowdhury, Kalaris Labs.
