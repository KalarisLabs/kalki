# Literature map

**Last reviewed:** 2026-11-11

The papers that shaped [ARCHITECTURE.md](../ARCHITECTURE.md), with the design
consequence of each. Nearly all are 2026 arXiv preprints. None of the numbers
below has been reproduced by us.

**How deeply each paper was read.** Honesty about this matters more than a long list.

- **R** — read as a structured report of the whole paper.
- **A** — abstract and introduction only.
- **N** — taken from earlier project notes and not re-read in this review.

## 1. Admission control for self-modification

| Paper | Read | Result | Consequence for Kalki |
|---|---|---|---|
| Self-Healing Harness for Runtime Oversight of Agent Self-Modification, [2609.24130](https://arxiv.org/abs/2609.24130) | A | An external gate around an unmodified agent. Across 16 paired runs on AppWorld, Terminal-Bench, and τ²-bench, the gate rejected 383 proposals by replay. 211 of them (55%) improved the triggering failure and degraded a case that had worked. | The gate replays a protected set. A proposal gets provisional authority during evaluation and persistent authority only after it passes. |
| RRSI: Regularized Recursive Self-Improvement of Agent Harnesses, [2609.24972](https://arxiv.org/abs/2609.24972) | R | Harness evolution overfits its task set. Controls: a shrinking edit budget, an edit history given to the proposer, a leakage screen on the diff, noise-aware acceptance, a cost-performance rule, pruning. Unregularized evolution scored highest on the evolve set and worst on transfer (40.3 against 43.6), at 3.80M against 2.42M tokens per trial. | Gate checks 1, 2, 5, and 6 are taken from here. |
| A Theory of Reliable Self-Evolution for Agent Harnesses, [2609.08175](https://arxiv.org/abs/2609.08175) | A | States a condition for reliable adoption and a validation rule. Argues the performance ceiling is set by the cost of verification and evaluation. | Cheap verification is the lever. Log replay and a mock model exist to lower that cost. |
| Beneath the Surface: What We Overlook When We Talk About Agent Evolution, [2611.07624](https://arxiv.org/abs/2611.07624) | A | Four methods, sixteen evolved agents. Every one gained tasks and lost tasks. Most reliability gains raised cost. The highest-scoring agent did not give the largest mean improvement. Stricter admission can discard useful alternatives. | Count lost tasks. Keep an archive of rejected candidates with the conditions under which they helped. |
| When the Critic Falls Behind (RuHarness), [2610.36826](https://arxiv.org/abs/2610.36826) | A | Fixed rubrics go stale as the harness changes. Rubrics that are updated from new failures generalize better. | Two kinds of criteria. Diagnostic rubrics may evolve and guide proposals. Admission criteria are fixed and outside the agent's reach. |
| Darwin Gödel Machine, [2505.22954](https://arxiv.org/abs/2505.22954) | N | An archive of agent variants admitted after benchmark validation. | The archive. |
| HGM-Dual, [2610.30042](https://arxiv.org/abs/2610.30042) | N | Each candidate carries compressed evidence of its ancestors. | Lineage in the genome history. |

## 2. Harness evolution methods

| Paper | Read | Result | Consequence for Kalki |
|---|---|---|---|
| Harness²: Recursive Agent Harnessing for an Open World, [2610.14224](https://arxiv.org/abs/2610.14224) | N | Propose, probe, compose at task level, and an experience library across tasks. No ground truth. | The shape of the evolve loop. Kalki adds a gate on top. |
| HarnessPro, [2610.26770](https://arxiv.org/abs/2610.26770) | A | Label-free evolution scored on LLM-written tests. Returns cost-focused, balanced, and performance-focused harnesses. Reports +2.0 points pass rate and −26.1% cost per task against a prior method. | The evolve loop keeps a set of non-dominated variants on score and cost, not one winner. |
| SoL-Pi, [2609.20519](https://arxiv.org/abs/2609.20519) | R | Automated search over 152 ideas on the Pi harness kept four mechanisms. Tokens fell 49.0% on EdgeBench with a score drop from 44.8 to 42.0. Mechanisms found on one model transferred to another. | Evidence that Pi is a workable base. Three mechanisms are adopted: fuse an edit with its test command, replace large outputs with handles after first use, and reduce logs through a checker that verifies quoted evidence. |
| Self-Evolving Coding Agents via Harness-Policy Optimization, [2610.22791](https://arxiv.org/abs/2610.22791) | A | Alternates harness search with reinforcement learning of the policy model. Raised an open-weight model from 57.8% to 64.8% average resolve rate. | The train loop. Sessions must be exportable as training data. |
| Recursive Agent Harnesses, [2606.13643](https://arxiv.org/abs/2606.13643) | N | A parent spawns full child harnesses in parallel. | Sub-agents are full sessions with their own log. |

Seen in search and not read: Harmony Harness (2610.15150), Self-Evolving Agent
Harness (2610.18813), MaD-RSI (2610.28201), HarnessMM (2610.32813), SelfSearch
(2609.37968), RSI-Env (2611.01736), Harness-S (2611.04277).

## 3. Context, memory, and recovery

| Paper | Read | Result | Consequence for Kalki |
|---|---|---|---|
| You don't need long context, [2611.05306](https://arxiv.org/abs/2611.05306) | R | Cutting the served context from 256K to 16K raised decode throughput 3.6× to 7× on three open-weight models. Compacting into a task state kept accuracy within −3.7 to +5.6 points on six benchmarks. One model fell from 53.0% to 3.6% at the tightest budget. Cost savings for closed models are modeled, not measured. | Short context with an explicit task state is the default. The budget is per model. |
| Compaction Laundering, [2610.29869](https://arxiv.org/abs/2610.29869) | A | Harnesses that flatten a conversation to labelled text before summarizing let a forged user turn survive 87–100% of the time (pi, opencode, Cline, Aider, OpenHands). Role-typed harnesses: 0% (Codex, Gemini CLI, Qwen Code, Continue). | Messages stay role-typed end to end. When Kalki drives Pi, Kalki compiles the context. |
| Do Agents Still Follow the Rules? (ARCBench), [2611.03223](https://arxiv.org/abs/2611.03223) | A | Compaction rarely deletes a rule. Agents that still have the rule violate it more than twice as often as with full history. | Rules are enforced in code and re-read from source for each call. |
| AgentRewind, [2608.14380](https://arxiv.org/abs/2608.14380) | R | Aligned checkpoints of context and workspace, plus a note carried back from the failed path. Success 62.2% → 87.8% on 82 tasks. Removing workspace rewind cost the most (43.9%). | Checkpoints pair a log position with a workspace ref. |
| Spine, [2610.37224](https://arxiv.org/abs/2610.37224) | N | Compiles a persistent subtask tree into context just in time. | Part 4 of the context compiler. |
| When Sub-Agents Work in Parallel, [2610.10263](https://arxiv.org/abs/2610.10263) | A | 354 tasks, 2,124 runs. Parallel sub-agents used 1.41× to 3.31× the tokens and helped mostly on long tasks. | Fan out only for independent work, inside a budget. |
| The Complexity Trap, [2508.21433](https://arxiv.org/abs/2508.21433) | N | Masking old observations matches summarization at lower cost. | Mask first. |

Seen and not read: AutoCompact (2610.02163), Compact-Bench (2610.31710),
Benchmarks Are Surprisingly Compaction-Friendly (2610.31778), StateHarness
(2610.17616), Memory as Infrastructure (2609.05510).

## 4. Benchmarks we intend to report on

| Benchmark | Measures | Used for |
|---|---|---|
| Terminal-Bench | Terminal tasks end to end | Headline task score |
| SWE-bench Pro | Repository issue resolution | Held-out transfer |
| MettleBench (2608.14380) | Long tasks with ordered acceptance criteria | Rewind |
| ARCBench (2611.03223) | Rule compliance after compaction | Policy and context compiler |
| Compact-Bench (2610.31710) | What compaction costs | Context compiler |

Which version of each benchmark we run, and on which models, is decided when
the evaluation runner exists. Until then this table is intent.

## 5. What this review did not cover

Sandboxing, provider APIs, terminal UI libraries, and release tooling were
researched earlier in the project and are not repeated here. Papers marked
**A** may contain limitations we have not seen. Read the paper before relying
on a number from this page.
