# Kalki: Admission-Controlled Recursive Self-Improvement on a Verifiable Harness Kernel

**Sayan Chowdhury** · Kalaris Labs · hello@kalarislabs.com
**Whitepaper proposal, draft 1** · 2026-11-11

> **What this document is.** A proposal: a design, the hypotheses it makes, and
> the experiments that would confirm or refute them. It reports no results.
> The system is partly implemented (section 8). Numbers attributed to other
> papers are theirs, are from preprints, and have not been reproduced by us.

## Abstract

Agent harnesses that rewrite themselves from experience now raise benchmark
scores without changing model weights. The same body of work shows that
self-modification, left to select its own changes, breaks previously working
behaviour, overfits the tasks that guide it, and raises cost. We propose
Kalki, an open-source coding harness built around a separation of duties. An
agent may propose any change to a defined set of harness files, the genome. A
gate outside the agent's write set admits a change only if it passes eight
checks: scope, leakage, improvement on the motivating failures, preservation
of a protected set, a measured noise floor, a cost rule, integrity tripwires,
and reversibility. The gate rests on the kernel underneath it: an
append-only, hash-chained event log that keeps every message role-typed,
aligned checkpoints of log and workspace that supply matched starting points
for comparison, and a genome store that the agent cannot write. We state five hypotheses, define
the measurements that test them, and commit to reporting transfer gain,
preservation, cost, integrity, and recovery together. Kalki is a single Rust
binary and is model-agnostic across three standard wire protocols.

## 1. Introduction

The performance of a coding agent depends on its model and on its harness: the
prompts, tools, control flow, memory, and context management around the model.
Recursive self-improvement (RSI) of the harness holds the weights fixed and
lets the system revise the rest from its own execution traces. Between
September and November 2026 this became a crowded research area, and its
results are consistent: harness evolution raises scores on the tasks that
drive it.

The more careful studies ask a second question: is the evolved harness better,
or only higher scoring? Their answers define our problem.

- **Collateral regression.** Tayebati et al. place an external gate around an
  unmodified agent and replay proposals against matched evidence. Of 383
  rejected proposals, 211 improved the failure that motivated them and
  degraded a case that had worked [1].
- **Coverage churn.** Wang et al. evaluate sixteen evolved agents from four
  methods. All sixteen solve new tasks and lose old ones. The top-scoring
  agent is never the one with the largest mean improvement [2].
- **Overfitting and cost.** Xia et al. show that unregularized evolution wins
  on its own task set, transfers worst, and uses 57% more tokens per trial
  than a regularized procedure [3].
- **A cost ceiling.** Cai et al. argue that the attainable performance of
  reliable self-evolution is bounded by the cost of verification and
  evaluation [4].

Two results on context management add constraints. Dabirsiaghi shows that
harnesses which flatten a conversation into labelled text before summarizing
allow a forged user turn to survive compaction 87–100% of the time, while
role-typed harnesses allow it 0% [5]. Zha et al. show that an agent which
still holds a rule after compaction violates it more than twice as often as
with full history [6]. A self-improving system that edits its own context
policy must not be able to launder instructions or erode rules this way.

Our thesis is that these are systems problems with systems answers.
Self-improvement needs the same structure as a change-management process in
any high-assurance software: a proposer, an independent reviewer with
evidence, a record, and a way back. The contribution of this proposal is a
harness in which that structure is enforced by construction. Whether it is
cheap enough to run on every change is one of the questions we test.

### Contributions

1. An **admission rule** of eight ordered checks for harness self-modification
   (section 4), combining matched-evidence preservation [1], regularized
   acceptance [3], and tripwires.
2. A **kernel design** that the rule depends on: a role-typed, hash-chained
   log, aligned checkpoints that supply matched starting points, and a genome
   store outside the agent's write set (section 3).
3. A **reporting standard** of five measurements to be published together
   (section 6).
4. An **open-source implementation** in Rust, partly complete, model-agnostic
   across three wire protocols (section 8).

## 2. Background and related work

**Harness evolution.** Darwin Gödel Machine admits agent variants into an
archive after benchmark validation [7]. Harness² evolves eight file-level
harness components by proposing, probing, and composing edits against
behavioural contrast, with an experience library across tasks and no ground
truth [8]. HarnessPro evolves harnesses against LLM-written tests and returns
variants at three cost levels [9]. SoL-Pi runs an automated search over 152
ideas on the Pi harness and keeps four efficiency mechanisms that transfer
across two models [10]. Self-HPO alternates harness search with reinforcement
learning of an open-weight policy [11]. RuHarness evolves the diagnostic
rubric together with the harness [12].

**Admission and reliability.** The works cited in section 1 [1–4] are the
closest to ours. Kalki differs in scope and in substrate. It applies admission
to an interactive coding harness in daily use and not only to an offline
benchmark loop, and it grounds the evidence in a verifiable log.

**Context management.** Belcak et al. show that short contexts with periodic
task-state compaction preserve accuracy on six long-horizon benchmarks and
raise open-weight decode throughput 3.6 to 7 times, with one model collapsing
under the tightest budget [13]. Spine compiles a persistent subtask tree into
context for each call [14]. AgentRewind restores context and workspace
together and reports a rise from 62.2% to 87.8% task success on 82
long-horizon tasks [15].

**Harnesses.** Pi is a minimal extensible harness with a JSONL RPC mode; it
has no permission system [16]. Codex CLI demonstrates a production harness in
Rust with kernel-level sandboxing. Prime Agent implements a CRUD-editable
harness state with rollback and reports reward hacking by its refinement loop
in one environment. None of these combines self-modification with an external
admission gate and a verifiable log.

## 3. System design

The system has five parts. The kernel is trusted. The others are not.

### 3.1 Kernel

**Event log.** A session is a sequence of typed events: messages with their
role, tool calls, tool results, token usage, checkpoints, and gate decisions.
Records are length-prefixed and CRC-checked. Each record will carry the
SHA-256 of its predecessor, so that altering history requires rewriting every
later hash. The chain detects tampering. It does not prevent it by an attacker
with write access to the whole store, and we do not claim otherwise.

**Role preservation.** A message keeps the role assigned at the provider wire
through storage, compilation, and any summary. The compiler never serializes a
conversation into a single string. This removes the laundering channel of [5]
by construction.

**Policy in code.** Tool calls are checked by a deny-first policy engine.
Operating rules are evaluated by that engine for each call and re-read from
their source for each context. No rule depends on surviving a summary [6].

**Aligned checkpoints.** At each model turn the kernel records a pair: a log
sequence number and a private git ref for the workspace. Rewind restores both
and carries forward a note from the abandoned path [15].

**Context compilation.** The context for a call is compiled from the log: a
stable prefix, the rules, a structured task state with pointers to source
events, the open subtask verbatim, and handles to large outputs. The size
budget is a per-model parameter [13].

**Gateway.** The kernel implements three provider wires and can serve them on
a loopback endpoint. Any executor pointed at the endpoint has its model
traffic recorded and capped without its cooperation.

### 3.2 Executors

An executor is an agent loop. The native executor is a minimal loop with two
tools, which the kernel runs itself under its policy. Pi runs as a second executor over its RPC
protocol, confined as a process, with its own compaction disabled. Running two
executors on identical tasks gives an independent baseline for every claim.

### 3.3 Genome

The genome is a set of files versioned by content hash: prompt notes, skills,
tool guidance, context policy, persistent memory entries with source
pointers, and sub-agent definitions. The base system prompt, the kernel, the
policy, the gate, the evaluation tasks, and the log are excluded and lie
outside the agent's write set.

Admitted versions are kept in a store outside every workspace. The kernel
loads a version only if its hash appears in an admission record or it carries
a trusted signature. Genome files inside a repository are proposals, so a
version delivered by `git pull` or changed by `git checkout` is inert. An
admission record names the model, executor, and settings it was evaluated on,
and the change is active only for those.

Text the agent writes for itself during a session (task state, rewind notes,
provisional notes) is a session overlay. It is tagged as agent-written, kept
out of the stable prefix, and discarded with the session.

### 3.4 Loops

A **task** loop does the work and verifies each increment by a command the
agent does not control. A **repair** loop lets the agent write a provisional
note after a repeated failure; it applies to the current session only. An
**evolve** loop runs offline over recorded failures and submits candidates to
the gate. A **train** loop, out of scope here, would export sessions to tune
an open-weight model for the harness [11].

## 4. The admission rule

Let `h` be the active genome and `h'` a candidate. Let `F` be the failures
that motivated `h'`, `P` a protected set of cases that pass under `h`, and `T`
a held-out task set never shown to the proposer. Every comparison runs both
genomes `k` times from the same starting state, model, and settings. Let
`p(g, x)` be the pass rate of genome `g` on case `x` over those runs, `s(·)`
the mean score on `T`, and `c(·)` the mean token cost per task. Let `τ` be the
noise tolerance, the spread of `s(h)` over repeated runs of the unchanged
genome, and `δ(x)` the flake margin of case `x`, measured the same way.

`h'` is admitted if and only if all of the following hold.

1. **Scope.** `diff(h, h')` touches only genome files and contains at most
   `b_t` edits, where the budget `b_t` decreases over rounds [3].
2. **Leakage.** `diff(h, h')` contains no identifier, answer, or constant from
   `F`, `P`, or `T`. The check reads the diff, not the rationale [3].
3. **Trigger.** The mean of `p(h', x) − p(h, x)` over `x` in `F` exceeds the
   mean of `δ(x)` [1]. Running `h` again is required: a case chosen because
   it failed regresses toward passing with no change.
4. **Preservation.** The number of cases `x` in `P` with
   `p(h, x) − p(h', x) > δ(x)` is at most `m`. By default `m = 0` [1, 2].
5. **Noise floor.** `s(h') ≥ s(h) − τ` [3]. This is a no-harm test. The
   requirement to gain is check 3.
6. **Cost.** If `s(h') − s(h) > τ`, then `c(h') − c(h) ≤ κ · (s(h') − s(h))`.
   Otherwise `c(h') < c(h)` or `h'` is a strict deletion from `h` [3, 9].
7. **Integrity.** During evaluation no canary file was read, no test or
   checker was modified, and no reference solution was retrieved.
8. **Reversibility.** `h` is retained and restorable.

Checks run in this order, cheapest first. Preservation is counted in cases
and not as a mean, because a mean conceals churn [2]. Candidates that fail
only checks 4 to 6 are archived with the subset of tasks on which they helped,
since a rejected candidate may be the right one under a stated condition [2].
The protected set is re-run periodically against the whole active genome to
detect changes that are safe alone and harmful together [1].

**Evidence.** The genome changes what the model is told, so a recorded
response is not evidence about a candidate: it was produced under `h`. Every
comparison therefore uses live runs, `2k` per case. The log contributes the
matched starting state, not the responses. Log replay and a scripted mock
model are used to test the kernel and the gate's own code, which is
deterministic.

**Information flow.** Decisions are written to a gate log that the proposer
cannot read. The proposer learns which check failed. For checks 5 and 6,
which use `T`, it learns only pass or fail. Check 2 matches text and cannot
catch a paraphrase, so withholding per-task results on `T` is the primary
defence against fitting the held-out set.

The rule is our synthesis. It is not derived from the theory in [4], and we
make no guarantee claim. Section 6 states how we will test it.

## 5. Threat model

| Adversary | Capability | Defence | Residual risk |
|---|---|---|---|
| The agent optimizing its own score | Proposes genome changes, runs tools in the workspace | Gate outside the write set; tripwires; judge is never the actor | A reward hack the tripwires do not anticipate |
| Injected content in files, tool output, or web pages | Text that reaches the context | Tool output handled as data; roles preserved; rules enforced in code | A model that obeys injected text within what policy allows |
| A compromised external executor | Arbitrary behaviour inside its process | Process sandbox; gateway caps; workspace checkpoints | Damage inside the workspace before the next checkpoint |
| Someone with write access to the log store | Rewrites history | Hash chain detects it | Not prevented. Needs external anchoring. |
| A proposer fitting the held-out set | Reads gate evidence; submits many candidates | Gate log outside its read set; pass or fail only for held-out checks | One bit per round still leaks. The held-out set must be rotated. |
| A genome delivered by a repository | Files changed by `git pull` or `git checkout` | Loaded only with a local admission record or a trusted signature | A user who trusts a bad key |
| A malicious skill or genome bundle | Instructions that run with agent permissions | Admission through the gate; signed bundles for the organization scope | Social engineering of a maintainer |

Sandbox strength differs by platform and is weakest on Windows. Until the
sandbox ships, Kalki is not a security boundary. On one machine the agent and
the gate run as the same operating-system user, so their separation is as
strong as the sandbox. Running the gate in CI, on a protected branch with
`T` held as a secret, removes that dependency.

## 6. Hypotheses and evaluation plan

Each hypothesis has a measurement and a result that would refute it.

| | Hypothesis | Measurement | Refuted if |
|---|---|---|---|
| H1 | Gated evolution loses fewer protected cases than ungated evolution at similar transfer gain. | Cases lost on `P`; score on `T`. Same proposer, same budget, gate on and off. | Lost cases are not lower, or transfer gain falls by more than `τ`. |
| H2 | Ordering the checks cheapest-first, with matched starting points from the log, lowers the cost of a gate decision against evaluating every candidate on the full task set. | Dollars per admitted or rejected decision under the ordered rule and under full evaluation. | The ratio is under 2. |
| H3 | Role-typed compilation removes compaction laundering. | The attack of [5] against Kalki's native executor, and against Pi driven by Kalki. | Success rate above 0% in either. |
| H4 | Policy enforced in code holds rule compliance flat as sessions lengthen. | ARCBench [6] at increasing session length. | Violations rise with length. |
| H5 | Per-model context budgets chosen through the gate beat one fixed budget on cost at equal accuracy. | Cost and score on three open-weight models, tuned against fixed. | No cost reduction at equal score on at least two models. |

**Protocol.** At least three runs per configuration. Task identifiers and
harness version pinned by hash. The judge is a different model instance from
the actor. `τ` is measured before any gated experiment and reported. Each
result names the model, the task set, the run count, and the cost.

**Benchmarks.** Terminal-Bench for end-to-end terminal tasks. SWE-bench Pro
for held-out transfer. MettleBench for rewind [15]. ARCBench for rule
compliance [6]. Versions are fixed when the evaluation runner exists.

**Reporting standard.** Every claim of improvement reports five measurements
together: transfer gain, preservation, cost change, integrity, and recovery.
A result that reports a subset is incomplete.

## 7. Limitations

- The rule adds evaluation cost to every change: `2k` live runs per case
  compared. If no `k` makes the rule both sound and affordable, the gate
  admits nothing. We treat that as a stop condition.
- A protected set fixes today's behaviour in place. Some regressions are
  intended. Removing a case from `P` is a human decision and is logged.
- A held-out set that is reused across many rounds stops being held out. It
  must be rotated.
- The gate judges the genome and not the model. A provider-side model update
  changes behaviour outside its view; the noise floor must then be re-measured.
- External executors are confined as processes. Their individual tool calls
  are not approved.
- We have read several cited papers only at abstract depth. The reading depth
  of each is recorded in the literature map.

## 8. Implementation status

As of v0.0.1 (November 2026), the implementation comprises about 5,600 lines
of Rust in three crates.

| Component | Status |
|---|---|
| Event log: CRC framing, rotation, torn-write repair, replay, kill drills | Implemented |
| Blob store, deny-first policy engine, `bash` and `apply_patch` tools | Implemented |
| Three provider wires with usage accounting | Implemented; tested against recorded streams and a mock server |
| Headless session loop | Implemented |
| Hash chain and verification | Designed |
| Checkpoints, rewind, sandbox, gateway, Pi executor, terminal UI | Designed |
| Context compiler | Designed |
| Genome, gate, evolve loop, evaluation runner | Designed |

No hypothesis in section 6 has been tested.

## 9. Roadmap to results

1. Hash chain, resume, model catalog, text tool-call mode.
2. Sandbox, gateway, Pi executor, rewind, terminal UI. H3 becomes testable.
3. Context compiler. H4 and H5 become testable.
4. Genome and gate. H1 and H2 become testable.
5. Evolve loop and the first public scorecard.

A full paper follows the first complete scorecard. Target venues are an arXiv
preprint in cs.SE and cs.AI, then a peer-reviewed venue.

## References

Entries marked † were read at abstract depth only.

1. S. Tayebati et al. *Self-Healing Harness for Runtime Oversight of Agent Self-Modification.* arXiv:2609.24130, 2026. †
2. X. Wang et al. *Beneath the Surface: What We Overlook When We Talk About Agent Evolution.* arXiv:2611.07624, 2026. †
3. P. Xia et al. *RRSI: Regularized Recursive Self-Improvement of Agent Harnesses.* arXiv:2609.24972, 2026.
4. Q. Cai et al. *A Theory of Reliable Self-Evolution for Agent Harnesses.* arXiv:2609.08175, 2026. †
5. A. Dabirsiaghi. *Compaction Laundering: Provenance Destruction in Context-Compacting Coding Agents.* arXiv:2610.29869, 2026. †
6. Y. Zha et al. *Do Agents Still Follow the Rules? Benchmarking Rule Compliance under Agent Context Compaction.* arXiv:2611.03223, 2026. †
7. J. Zhang et al. *Darwin Gödel Machine: Open-Ended Evolution of Self-Improving Agents.* arXiv:2505.22954, 2025.
8. R. Xu et al. *Harness²: Recursive Agent Harnessing for an Open World.* arXiv:2610.14224, 2026.
9. Z. Pan et al. *HarnessPro: Recursive Harness Self-Improvement with Pareto Optimization for Performance-Cost Trade-offs.* arXiv:2610.26770, 2026. †
10. H. Liu et al. *SoL-Pi: Recursively Scaling Auto-Research Loops for Efficient Agent Harness.* arXiv:2609.20519, 2026.
11. Z. Wei et al. *Self-Evolving Coding Agents via Harness-Policy Optimization.* arXiv:2610.22791, 2026. †
12. H. Chen et al. *When the Critic Falls Behind: Co-Evolving Rubrics and Harnesses for Continual Agent Self-Improvement.* arXiv:2610.36826, 2026. †
13. P. Belcak et al. *You don't need long context.* arXiv:2611.05306, 2026.
14. *Spine: Compiling Subtask Trees into Structured Context for Long-Horizon Coding Agents.* arXiv:2610.37224, 2026.
15. Y. Zhuang et al. *AgentRewind: Recoverable Execution for Long-Horizon LLM Agents.* arXiv:2608.14380, 2026.
16. Earendil Works. *Pi.* https://github.com/earendil-works/pi, 2026.
