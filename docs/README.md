# Kalki documentation

Each document says at the top whether it describes what is built or what is
planned. Where two documents disagree, the one higher in this table wins.

| Document | What it is | Read it to |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | The design. Section 12 lists what exists today. | Understand the parts and their boundaries |
| [adr/](adr/) | Decision records, numbered | Learn why a choice was made and what was rejected |
| [PROPOSAL.md](PROPOSAL.md) | The product proposal | Understand the goal, the users, and the measurements |
| [WHITEPAPER.md](WHITEPAPER.md) | The whitepaper proposal, draft 1 | Read the admission rule, the hypotheses, and the evaluation plan |
| [COMPATIBILITY.md](COMPATIBILITY.md) | Operating systems, terminals, models, providers | Check whether a setup is supported or only expected to work |
| [TUI.md](TUI.md) | The terminal UI specification (not built) | Build or review terminal UI work |
| [RELEASING.md](RELEASING.md) | The release process | Cut or verify a release |
| [research/LITERATURE.md](research/LITERATURE.md) | The papers behind the design, with how deeply each was read | Trace a design choice to its evidence |

Outside this directory: [../README.md](../README.md) for users,
[../AGENTS.md](../AGENTS.md) for anyone changing the code,
[../ROADMAP.md](../ROADMAP.md) for what comes next, and
[../DEPENDENCIES.md](../DEPENDENCIES.md) for what the build depends on.

## Decision records

| ADR | Decision |
|---|---|
| [0001](adr/0001-workspace-and-license.md) | Cargo workspace, one-way crate graph, MIT or Apache-2.0, DCO |
| [0002](adr/0002-provider-internal-trait.md) | One internal `Provider` trait. Superseded in part: all three wires are hand-written on `reqwest`. |
| [0003](adr/0003-cedar-v1.5.md) | TOML policy first; Cedar later behind the same decision API |
| [0004](adr/0004-executors-and-pi.md) | Pluggable executors. Pi runs under the kernel over RPC; no fork. |
| [0005](adr/0005-versioning-and-release-channel.md) | First release is v0.0.1. GitHub Releases only. |
| [0006](adr/0006-admission-gate.md) | The eight checks a self-modification must pass |
