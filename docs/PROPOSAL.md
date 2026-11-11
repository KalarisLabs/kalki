# Kalki product proposal

**Organization:** Kalaris Labs · **Author:** Sayan Chowdhury · **Date:** 2026-11-11
**License:** MIT OR Apache-2.0 · **Repository:** github.com/KalarisLabs/kalki

> This is a proposal for a product that is partly built. v0.0.1 is a headless
> agent loop with a durable log, three provider wires, and a deny-first
> policy. It does not yet improve itself. Section 8 separates what exists from
> what is planned. No number in this document was measured on Kalki unless it
> says so.

## 1. Summary

Kalki is an open-source coding harness that improves itself under supervision.

A harness is the software around a model: prompts, tools, context management,
memory. In 2026 the fastest way to make a coding agent better is to let it
rewrite its own harness from experience. The research shows this works. It
also shows that an unsupervised loop breaks old abilities while gaining new
ones, overfits its test tasks, and gets steadily more expensive.

Kalki's proposition is the supervision. An agent running on Kalki may propose
any change to its prompts, skills, and context policy. A gate that the agent
cannot write to re-runs the failures behind the change and a protected set of
working cases, checks held-out tasks, measures cost, and admits the change
only if it helps without breaking anything. Every decision is recorded.

Kalki is a single Rust binary. It works with any model reachable over an
OpenAI-compatible, OpenAI Responses, or Anthropic Messages endpoint, which
covers the open-weight ecosystem. It sends no telemetry.

## 2. Who it is for

| User | What they need | What Kalki gives them |
|---|---|---|
| A developer running open-weight models locally | An agent that works well on a 32K-context model without hand-tuning prompts | A harness that tunes its own context budget and prompts for that model, with evidence |
| A team adopting coding agents | To know what the agent did, and that last week's behaviour still holds | A verifiable log of every action, and a protected set of cases that every harness change must still pass |
| A researcher in agent self-improvement | A base where harness changes are cheap to evaluate and hard to fool | Matched starting points from the log, deterministic tests of harness code against a mock model, and an admission record for every change |
| An institution with compliance duties | Local execution, no data egress, an audit trail | One binary, no telemetry, a hash-chained log, policy as code |

## 3. Why now

Three things became true in the second half of 2026.

1. **Harness self-improvement works, and its failure modes are documented.**
   About twenty papers between September and November 2026 evolve agent
   harnesses with frozen weights. The careful ones report the same problems:
   collateral regressions, overfitting, rising cost, and a best-scoring variant
   that is not the most useful one. See [research/LITERATURE.md](research/LITERATURE.md).
2. **Open-weight models are good enough, and short context makes them cheap.**
   Serving a 16K context instead of 256K raises decode throughput 3.6 to 7
   times on current open-weight models, and a harness that keeps a compact
   task state loses little accuracy (arXiv:2611.05306). The harness decides
   whether that saving is available.
3. **Nobody ships the supervised version.** Research code evolves harnesses
   offline against a benchmark. Production agents do not self-modify, or do so
   without a gate. No open-source harness combines self-modification, an
   admission gate, and a verifiable log.

## 4. What Kalki is

Five parts. The full design is in [ARCHITECTURE.md](ARCHITECTURE.md).

- **Kernel.** Records every event in an append-only log, enforces a deny-first
  policy on tool calls, checkpoints the workspace, compiles the model's
  context, and talks to model providers. Runs no model-written code in its own
  process.
- **Executors.** The agent loop. A native Rust loop is built in. Pi, an
  existing open-source harness, runs under the kernel as a second executor.
- **Genome.** The files self-improvement may change: prompt notes, skills,
  tool guidance, context policy, memory.
- **Gate.** Eight checks that a genome change must pass before it persists.
- **Surfaces.** A terminal UI, a headless CLI, and a JSONL protocol.

## 5. How it differs

| | Typical coding agent | Research RSI framework | Kalki |
|---|---|---|---|
| Changes its own harness | No, or prompt memory only | Yes, offline against a benchmark | Yes, during use and offline |
| Who decides a change is kept | A person | The search procedure | A gate outside the agent's write set |
| Checks for broken old abilities | No | Sometimes, by mean score | Always, by counting lost cases on a protected set |
| Counts cost in acceptance | No | Sometimes | Always |
| Record of what happened | Session transcript | Benchmark logs | Hash-chained event log with verification |
| Context after long sessions | Summary replaces history | Varies | Summary is a view; history remains; roles are preserved |
| Undo | Limited | No | Rewind of workspace and context together |
| Models | Often tied to one vendor | Whatever the paper used | Any endpoint on three standard wires |
| Runtime | Node.js or Python | Python | One static binary |

Kalki does not try to out-tool anyone. It ships two tools. The bet is that a
small, trusted kernel with a good gate improves faster than a large harness
that cannot tell whether a change helped.

## 6. The goal, stated so it can be checked

The goal is to be the best harness in the world at recursive self-improvement.
"Best" is defined here by five measurements. Kalki will publish all five for
every release that claims progress, with model, task set, run count, and cost.

| Measurement | Meaning | Why it is on the list |
|---|---|---|
| **Transfer gain** | Score change on tasks never used to select changes | The only gain that is not overfitting |
| **Preservation** | Number of previously passing cases lost | Mean scores hide regressions |
| **Cost change** | Tokens and price per task, after against before | Improvement that doubles cost is a different product |
| **Integrity** | Tripwire hits, and whether the log verifies | A gain obtained by cheating is a loss |
| **Recovery** | Fraction of stuck or crashed sessions that finish | Long tasks fail by compounding error |

A harness is ahead when it improves transfer gain without losing on the other
four. Kalki claims no lead on any of them today. It has measured none of them.

The plan to get there is to make evaluation trustworthy first and cheap
second. The ceiling on reliable self-improvement is set by the cost of
verifying a change (arXiv:2609.08175). A change to the genome can only be
judged by live model runs, several per arm, so the gate is expensive. Kalki
lowers that cost where it honestly can: the log supplies matched starting
points, cheap checks reject before expensive ones, and changes to harness
code are tested against a mock model with no model spend. Whether that is
cheap enough is unmeasured, and it is a stop condition.

## 7. What we will not do

- Claim a result we have not measured, or publish a score without its cost.
- Claim a 1000-hour autonomous run. No published system has shown one.
- Turn on telemetry by default, ever.
- Let any component modify its own gate.
- Add a tool, a dependency, or a crate before the code that needs it.

## 8. Status

| Built in v0.0.1 | Planned |
|---|---|
| Headless session loop with a turn cap | Hash-chained log and `kalki audit verify` (v0.0.2) |
| CRC-framed, rotating, crash-tested event log | Model catalog and text tool-call mode (v0.0.2) |
| Content-addressed blob store | Sandbox, terminal UI, gateway, Pi executor, rewind (v0.0.3) |
| Deny-first policy engine | Context compiler with per-model budgets (v0.0.4) |
| `bash` and `apply_patch` tools | Genome and admission gate (v0.0.5) |
| OpenAI Responses, Anthropic Messages, and OpenAI-compatible wires | Offline evolution and the public scorecard (v0.0.6) |
| CI on five platforms, MSRV check, release pipeline | Stable log format, soak tests, second maintainer (v0.1.0) |

The roadmap with exit criteria is in [ROADMAP.md](../ROADMAP.md).

## 9. Open source and community

- Dual-licensed MIT or Apache-2.0. Contributions under the DCO. No CLA.
- Decisions that change the protocol, the log, the policy engine, or the gate
  are made by ADR in public. See [GOVERNANCE.md](../GOVERNANCE.md).
- The project has one maintainer. Adding a second from outside Kalaris Labs is
  a condition of v0.1.0.
- Attribution: Kalki's design draws on Pi (Earendil Works), Prime Agent (Prime
  Intellect), Codex CLI (OpenAI), and the papers in the literature map. Kalki
  is not affiliated with any of them. See [NOTICE](../NOTICE).

## 10. Risks

| Risk | Why it is real | Response |
|---|---|---|
| The gate is too strict to admit anything useful | Strict admission discards helpful variants (arXiv:2611.07624) | Archive rejected candidates with the conditions where they helped. Measure admission rate. |
| The gate is gamed | Agents have been observed retrieving reference solutions from git history | Tripwires, checkers outside the write set, a judge that is never the actor |
| The gate costs too much to run | Every comparison needs several live runs per arm, because a recorded response says nothing about a changed genome | Cheap checks first. Measure cost per decision. Run the gate in CI in batches. |
| Evaluation tasks are too few or too easy | A small set makes noise look like progress | Measure the noise floor first. Report run counts. |
| Process-level confinement of external executors is insufficient | Kalki cannot approve Pi's tool calls individually | Native executor is the reference. State the difference plainly. |
| Windows sandboxing lags | No Landlock equivalent | Recommend WSL2. Do not call Windows confinement strong. |
| One maintainer | Bus factor of one | Governance goal with a deadline at v0.1.0 |
| The field moves faster than the build | About twenty papers in two months | Small kernel, replaceable executors, genome as plain files |

## 11. Stop conditions

Kalki should be stopped or redirected if any of these holds:

- By v0.0.6, the gate admits no change that improves transfer gain on any
  model we test.
- A class of log corruption is found that the design cannot detect.
- Self-modification changes policy-relevant behaviour in a way the gate did
  not flag, and no added check fixes it.
- No number of runs per arm makes the gate both sound and affordable.
