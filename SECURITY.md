# Security policy

Kalki runs commands that a language model writes. Read the next section before
you rely on it.

## What v0.0.1 does and does not protect

**Kalki v0.0.1 is not a security boundary.**

| Protection | State |
|---|---|
| Deny-first policy on `bash` commands | Built. It matches the program and argument prefix. It cannot see what the program then does. |
| `bash` takes an argv array, never a shell string | Built |
| Paths in `bash` arguments and in patches confined to the workspace | Built, as a path check. Not enforced by the operating system. |
| Policy on `apply_patch` | Not built. A patch may write any file in the workspace. |
| Secret scrubbing of tool output | Built, by pattern. Patterns miss things. |
| Operating-system sandbox | Not built. Planned for v0.0.3. |
| Network egress control | Not built. Planned for v0.0.3. |
| Tamper-evident log (hash chain) | Not built. Planned for v0.0.2. Records carry checksums, which detect corruption and not tampering. |

The built-in default policy allows `git` and `cargo`. Both can run arbitrary
code, through hooks and build scripts. If the workspace or the model is
untrusted, run Kalki inside a container or a virtual machine.

## Reporting a vulnerability

- Preferred: [open a private advisory](https://github.com/KalarisLabs/kalki/security/advisories/new) on GitHub.
- Or email **hello@kalarislabs.com** with `[SECURITY] kalki` in the subject.

Include the version (`kalki --version`), the platform, the steps to reproduce,
and the impact. Do not open a public issue.

We acknowledge within 72 hours and triage within 14 days. We ask that you not
disclose publicly until a fixed release exists, for up to 90 days. We credit
reporters in the advisory unless they decline.

Only the latest release is supported while the version is `0.0.x`.

## In scope

- A command or patch that takes effect although the policy denies it.
- A path-confinement bypass.
- A secret reaching the log, a blob, or a model request when it should have
  been scrubbed.
- Log corruption that replay does not detect.
- A flaw in the release pipeline or in the published artifacts.

Not in scope until the sandbox ships: an allowed program doing something
harmful. That is the gap stated above.

## Threats the design addresses

The design and its limits are in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and section 5 of
[docs/WHITEPAPER.md](docs/WHITEPAPER.md).

1. **Prompt injection** through repository files, tool output, or web content.
   Tool output is data and keeps its role. Rules live in the policy engine,
   not in the prompt.
2. **Compaction laundering**, where injected text is summarized into a trusted
   position. Summaries are views over the log and never change a message's role.
3. **Reward hacking**, where an agent edits its own gate, policy, tests, or
   log. Those are outside the agent's write set. *(Enforced by the sandbox;
   planned.)*
4. **Supply chain.** `cargo-deny` checks licenses, bans, and sources on every
   pull request and advisories daily. GitHub Actions are pinned to commits.
   Release archives ship with SHA-256 sums and build provenance.
5. **Data egress.** Kalki has no telemetry. It sends data only to the model
   endpoint you configure.

## Verifying a release

See [docs/RELEASING.md](docs/RELEASING.md) for the checksum and provenance
commands.
