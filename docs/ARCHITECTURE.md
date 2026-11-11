# Kalki architecture

**Status:** accepted design, partly built · **Owner:** Sayan Chowdhury, Kalaris Labs · **Updated:** 2026-11-11

Kalki is an open-source coding harness designed to improve itself under
supervision. It runs a coding agent in your terminal and records what the
agent does in an append-only log. The log is checksummed today and will be
hash-chained from v0.0.2. The design lets the agent propose changes to its own
prompts, skills, and context policy. A separate component, the gate, decides
which of those changes are kept. The agent cannot write to the gate.
Self-improvement is planned and not built.

This document is the single source of truth for the design. Every section says
what is **built** and what is **planned**. Section 12 lists what exists today.
The evidence behind each decision is in [research/LITERATURE.md](research/LITERATURE.md).

## 1. The problem the design answers

A harness is the software around a model: prompts, tools, context management,
memory, control flow. In 2026 a harness that edits itself is an established
research result. Recursive self-improvement (RSI) of a harness with frozen
model weights raises benchmark scores. The same literature shows four ways it
goes wrong, and each one shapes Kalki.

| Finding | Evidence | What Kalki does about it |
|---|---|---|
| A change that fixes one failure often breaks a task that worked. | In 16 paired runs, 55% of the 383 rejected self-modifications improved their triggering failure and degraded a previously passing case (arXiv:2609.24130). All 16 evolved agents in a second study gained tasks and lost tasks (arXiv:2611.07624). | The gate replays a protected set of previously passing cases for every proposal. It counts lost tasks, not only the mean score. |
| Evolution overfits the tasks it trains on, and gets more expensive. | Unregularized evolution scored highest on its own task set, transferred worst, and used 57% more tokens per trial than the regularized method (arXiv:2609.24972). | Proposals have an edit budget. A change must beat measured noise on the failures that motivated it, must not fall below the noise floor on held-out tasks, and must pass a cost rule that limits added tokens per point gained. |
| Summarizing history destroys who said what. | A forged user turn survived compaction 87–100% of the time in five harnesses that flatten messages to text, and 0% in four that keep messages role-typed (arXiv:2610.29869). Agents that kept a rule through compaction still broke it more than twice as often (arXiv:2611.03223). | The log keeps every span with its role and source. Summaries are views over the log. Rules are enforced by the policy engine in code and are re-read from source for every model call. |
| Errors compound, and a forward-only agent cannot undo them. | Rewinding both the context and the workspace to a checkpoint raised task success from 62.2% to 87.8% on an 82-task long-horizon benchmark (arXiv:2608.14380). | Every model turn is an aligned checkpoint of log position and workspace state. Rewind is a first-class action. |

One more result sets the cost target. On open-weight models, serving a 16K
context instead of 256K raised decode throughput 3.6 to 7 times, and agents
that kept a compact task state instead of full history stayed within a few
points of baseline accuracy on six benchmarks (arXiv:2611.05306). One model
collapsed under the tightest budget. Kalki therefore treats the context budget
as a per-model setting that can be tuned, not a constant.

## 2. The design in one picture

```
                 ┌────────────────────────── surfaces ──────────────────────────┐
                 │  terminal UI     headless CLI     JSONL RPC     CI mode      │
                 └──────────────────────────────┬───────────────────────────────┘
                                                │ kalki-protocol events only
┌───────────────────────────────────────────────▼───────────────────────────────┐
│ KERNEL  (Rust, deterministic, runs no model-written code in-process)          │
│                                                                               │
│  event log ── blob store ── checkpoints        policy engine ── sandbox       │
│  context compiler                               provider gateway ── budgets   │
└───────┬───────────────────────────────┬───────────────────────────┬───────────┘
        │ drives                        │ reads                     │ submits evidence
┌───────▼────────────┐        ┌─────────▼──────────┐        ┌───────▼────────────┐
│ EXECUTORS          │        │ GENOME             │        │ GATE               │
│ native (Rust loop) │        │ prompts, skills,   │        │ replay, held-out   │
│ pi (JSONL RPC)     │        │ tool guidance,     │        │ eval, noise floor, │
│ others via gateway │        │ context policy,    │        │ cost rule,         │
│                    │        │ memory             │        │ tripwires          │
└────────────────────┘        └─────────▲──────────┘        └───────┬────────────┘
                                        │ only the gate writes       │
                                        └────────────────────────────┘
```

Five parts. The kernel records and enforces. An executor is the agent loop
that talks to the model. The genome is the set of files that self-improvement
may change. The gate decides which genome changes persist. Surfaces show the
user what is happening.

The rule that holds the design together: **the agent can read the genome and
propose changes to it, and only the gate can write it.** That will be enforced
by the sandbox write set (planned, v0.0.3) and by where the genome is stored
(section 5), not by a prompt.

## 3. Kernel

The kernel is the trusted part. It is small, deterministic, and has no
dependency on any agent framework.

### 3.1 Event log (built, hash chain planned)

Every meaningful occurrence is an event appended to a per-session log:
messages, tool calls, tool results, token usage, compaction boundaries,
lifecycle markers, snapshot anchors. Events are typed in `kalki-protocol`.
Messages carry their role (`user`, `assistant`, `tool`, `system`).

- **Built:** records are length-prefixed JSONL with a CRC32C checksum, written
  to segments that rotate at 64 MiB. A torn final record is truncated at open.
  Replay rebuilds a session. Crash drills kill the process mid-write and assert
  recovery.
- **Planned (v0.0.2):** each record carries the SHA-256 of the previous record.
  `kalki audit verify` checks the chain. The chain makes edits detectable. It
  does not stop an attacker with root access, and the docs say so.

The log is the only durable record. Everything else can be rebuilt from it.

### 3.2 Blob store (built)

A tool result larger than 4 KiB is written to a content-addressed store. The
event holds the SHA-256 handle, the size, and a short preview. The model sees
the preview and can fetch slices by handle. Large output never enters the
context unasked.

### 3.3 Policy engine and sandbox (policy built, sandbox planned)

- **Built:** a deny-first policy in TOML. A `bash` call with no matching rule
  is denied. Rules are checked in order and the first match wins. Rules have
  stable IDs, and a denial names the rule. The `bash` tool takes an argv array
  and never a shell string. Paths in the argv are confined to the workspace by
  path check. With no policy file, a built-in default allows a short list of
  read and build commands.
- **Gap in v0.0.1:** `apply_patch` is confined to the workspace by path check
  but is not checked against policy rules. An `ask` rule is treated as a
  denial, because there is no prompt yet. The first is fixed in v0.0.2. The
  second is fixed with the terminal UI in v0.0.3.
- **Planned (v0.0.3):** kernel-enforced confinement per platform. See
  [COMPATIBILITY.md](COMPATIBILITY.md) for the tiers. Until that lands, Kalki
  is not a security boundary, and the README says so.

Operating rules live here, in code. Section 1 shows why: a rule that exists
only in the prompt is lost or ignored after long runs.

### 3.4 Checkpoints and rewind (planned, v0.0.3)

At each model turn the kernel records a pair: the log sequence number and a
private git ref for the workspace state (`refs/kalki/cp/<session>/<seq>`).
Rewind restores both and adds a short note written by the agent about what the
abandoned path learned. Effects outside the workspace, such as network calls,
cannot be undone. The kernel marks those turns as not rewindable past.

Checkpoint refs live in the workspace repository, so the agent can delete
them. They are a recovery aid, not a security control. Rewind restores the
workspace. It never changes which genome is active (section 5).

### 3.5 Context compiler (planned, v0.0.4)

**Today (v0.0.1)** there is no compiler. The native loop sends the system
prompt, the objective, and the full message history on every turn. It has no
summarizer and no budget.

**Planned:** the context sent to the model is compiled for each call from the
log. It is a cache. It can be thrown away and rebuilt.

1. **Fixed prefix.** System prompt and genome content, in a stable order so
   that provider prompt caches hit.
2. **Rules.** Read from the policy source on every call. Never taken from a
   summary.
3. **Task state.** A structured note the agent maintains: goal, verified
   findings, files touched, next steps. Each line points to the log events it
   came from. It is part of the session overlay (section 5): tagged as
   agent-written and never placed in the fixed prefix.
4. **Open work.** The subtask the agent is in, kept verbatim. Finished
   subtasks collapse to their result (the Spine approach, arXiv:2610.37224).
5. **Handles.** References to large outputs in the blob store.

The budget for parts 3 to 5 comes from the model's profile. A model that
degrades with long context gets a small budget and frequent state updates. A
model that degrades when history is cut gets a larger one. The budget is part
of the genome, so the gate can tune it per model with evidence. Admission is
per model and executor (section 5), so a budget admitted for one model is not
applied to another.

Every span in the compiled context keeps its role. The compiler never flattens
a conversation into one string.

### 3.6 Provider gateway (wires built, gateway planned)

- **Built:** three hand-written wire implementations on `reqwest`: OpenAI
  Responses, Anthropic Messages, and OpenAI-compatible Chat Completions. The
  third covers nearly every open-weight model server and hosted inference
  provider. Token usage, including cached tokens, is recorded per call.
- **Planned (v0.0.2):** a model catalog with one profile per model: tool-call
  mode, context budget, reasoning-field handling. A text tool-call mode for
  models with no reliable native function calling.
- **Planned (v0.0.3):** the kernel serves the same wires on a loopback
  endpoint. An executor that is pointed at this endpoint has every model call
  recorded, metered, and capped by the kernel. This is how Kalki supervises an
  agent it did not write.

Model and provider support is listed in [COMPATIBILITY.md](COMPATIBILITY.md).

## 4. Executors

An executor is an agent loop. The kernel does not care which one runs, as long
as its model traffic and its file changes pass through the kernel.

| Executor | What it is | Kernel control | Status |
|---|---|---|---|
| `native` | A minimal loop in Rust: model turn, policy-checked tool calls, repeat. Tools: `bash`, `apply_patch`. | Full. The kernel runs every tool call itself. Today `bash` is checked against policy and `apply_patch` is path-confined only. | Built, headless |
| `pi` | [Pi](https://github.com/earendil-works/pi) run as a child process over its JSONL RPC mode. | Model calls via the gateway. The whole process runs in the sandbox. Workspace changes are checkpointed. Pi's own tool calls are observed, not individually approved. | Planned, v0.0.3 |
| others | Any agent that accepts a custom model base URL. | Same as `pi`. | Later |

Pi is the first external executor because it is small, MIT-licensed, has a
documented RPC protocol, and is the base used by recent harness-evolution
research (arXiv:2609.20519). Running Pi under Kalki gives a full-featured
agent on day one while the native loop grows. Kalki does not fork Pi and does
not link Node.js. The decision record is [ADR-0004](adr/0004-executors-and-pi.md).

One caution applies to every external executor. Pi was among the harnesses
whose compaction step accepted forged turns (section 1). When Kalki drives Pi,
Kalki compiles the context and Pi's own compaction is turned off.

## 5. Genome

The genome is what self-improvement is allowed to change. It is a set of plain
files, versioned by content hash, so every change is a diff with an author, a
parent, and a way back.

| In the genome | Never in the genome |
|---|---|
| Prompt notes added to the fixed base prompt | The base system prompt |
| Skills (`SKILL.md` files) | The kernel, the policy engine, the sandbox |
| Tool guidance and tool descriptions | Policy rules |
| Context policy: budgets, when to update task state | The gate and its thresholds |
| Persistent memory entries, each with a pointer to its source events | The evaluation tasks and their checkers |
| Sub-agent definitions | The audit log |

The right-hand column is outside the agent's write set. A proposal that
touches it is rejected before evaluation.

### 5.1 Where the genome lives

The active genome is not a directory the agent can edit. Each admitted version
is stored by content hash under `~/.kalki/genome/store/`, outside every
workspace. The kernel loads a version only if its hash is named in an
admission record in the local gate log, or it carries a signature from a key
the user has trusted.

Files under `<repo>/.kalki/genome/` are proposals and a shareable copy. They
are inert until admitted or trusted. A genome that arrives by `git pull`, or
that changes because of `git checkout`, `git stash`, or a rewind, therefore
cannot take effect on its own. Rollback is a switch to an earlier hash in the
store, which the agent cannot delete.

There are three scopes: project, user, and organization. An organization
genome is a signed bundle. A narrower scope overrides a wider one, except for
entries that the organization scope pins.

Admission is per profile. An admission record names the model, the executor,
and the settings the change was evaluated on. The change is active only for
those. Using it with another model or executor needs another evaluation.

### 5.2 The session overlay

Not everything the agent writes is genome. Within a session the agent writes
task-state notes, rewind notes, and provisional notes from the repair loop
(section 7). Together these are the session overlay. Overlay text is tagged as
agent-written, sits outside the fixed prefix, ends with the session, and is
never a policy rule. The gate does not review it, which is why it cannot
persist.

Memory follows the same split. What the agent remembers within a session is
overlay. A memory entry that should outlive the session is a genome change and
goes through the gate.

## 6. Gate

The gate is admission control for self-modification. It answers one question:
should this proposed genome change persist?

A candidate `h'` replaces the current genome `h` only if all of these hold:

1. **Scope.** The diff touches only genome files and stays inside the edit
   budget for this round.
2. **No leakage.** The diff contains no task names, answers, or values from
   the evaluation tasks. The check reads the diff, not the stated rationale.
3. **It fixes what it claims.** On the failures that motivated it, `h` and
   `h'` are each run `k` times from the same starting state, model, and
   settings. `h'` must pass more often than `h` by more than the measured
   noise. Running `h` again matters: a task chosen because it failed will
   often pass on a rerun with no change at all.
4. **It breaks nothing protected.** A protected case counts as lost when its
   pass rate under `h'` falls below its pass rate under `h` by more than that
   case's measured flake margin. `h'` may lose at most `m` cases. The default
   for `m` is zero. A case too flaky to measure is removed from the protected
   set by a person, with a record.
5. **It does no general harm.** On held-out tasks, `score(h') ≥ score(h) − τ`.
   Both scores are means over `k` runs. `τ` is the noise tolerance measured by
   running the unchanged genome repeatedly. This is a no-harm test. The
   requirement to gain something is check 3, and check 6 covers a change with
   no held-out gain.
6. **The cost is justified.** If the gain exceeds `τ`, added token cost stays
   under a limit that grows with the gain. If the gain is within `τ`, the
   change must lower cost or remove something.
7. **No tripwire fired.** Canary files sit in paths the sandbox denies, and an
   attempt to read one is reported. Test files and checkers were not
   modified. Git history holding the reference solution was not consulted.
8. **It can be undone.** The parent version is in the genome store, outside
   the workspace.

### 6.1 What evidence the gate uses

The genome changes what the model is told. A recorded model response
therefore says nothing about a candidate: it was produced under the old
genome. Every check that compares `h` with `h'` uses live model runs, `k` per
arm. That is the main cost of the gate, and it is why `k` and the size of the
protected set are open questions (section 13).

The log supplies matched starting points: the task, the workspace snapshot,
the model, and the settings of the original run. Log replay and the scripted
mock model test kernel and gate code deterministically, at no model cost.
They do not evaluate a genome.

The protected set is re-run on a schedule against the whole active genome, to
catch changes that are harmless alone and harmful together.

### 6.2 What the proposer sees

Each decision produces a record: the diff, the evidence for each check, the
verdict, and the reason. Records go to a gate log that is separate from the
session logs and outside the proposer's read set.

For checks 1 to 4, 7, and 8 the proposer is told which check failed and why.
For the held-out checks, 5 and 6, it is told only pass or fail. Per-task
held-out results never reach it, because a proposer that sees them will fit
them. Check 2 matches text, so it catches a copied answer and not a
paraphrased one. Withholding held-out results is the main defence against
leakage, and check 2 is the backstop.

Rejected candidates are kept in an archive with the conditions under which
they did help, because the best-scoring candidate is often not the most
useful one (arXiv:2611.07624).

### 6.3 Where the gate runs

The gate's thresholds, tasks, checkers, protected set, archive, and log live
under `~/.kalki/gate/` (section 10). There are two deployments.

- **Local.** The agent and the gate run as the same operating-system user.
  The separation is exactly as strong as the sandbox that denies the agent
  process access to those paths. It is weakest on Windows.
- **CI.** The genome change is a pull request. The gate runs on a protected
  branch, and the held-out tasks are CI secrets the agent never holds. The
  agent can only open the pull request. This is the stronger deployment, and
  project genomes should be admitted this way.

The gate is planned for v0.0.5. Nothing in Kalki self-modifies before it lands.

## 7. Four loops

| Loop | Timescale | What happens | Persists? |
|---|---|---|---|
| Task | Seconds to hours | The executor works. Planned: each increment is verified by a command the agent does not control, and a stuck agent rewinds. | Workspace commits and the log |
| Repair | Within a session | The agent notices a repeated failure and writes a provisional note in the session overlay (section 5.2). The note applies to this session only. | No, unless the gate admits it as a genome change |
| Evolve | Offline, hours to days | A proposer reads recorded failures, proposes genome changes, and submits them to the gate. Accepted changes form a set of non-dominated variants on score and cost. | Yes, through the gate |
| Train | Offline, later | Sessions are exported as training data so an open-weight model can be tuned for the harness (arXiv:2610.22791). | Out of scope before v0.1 |

Built today, headless: model turns, tool calls, and a turn cap. Verification,
rewind, and the other three loops are planned.

Parallel sub-agents are used sparingly. Measured across 2,124 runs, parallel
sub-agents cost 1.41 to 3.31 times the tokens and helped mainly on long tasks
with separable work (arXiv:2610.10263). Kalki fans out only for independent
work, alternative attempts, or independent verification, and only inside a
token budget.

## 8. Surfaces

All surfaces consume `kalki-protocol` events and send commands. None contains
agent logic.

- **Headless CLI (built):** `kalki run -p "<objective>"`, `kalki doctor`.
- **Terminal UI (planned, v0.0.3):** specified in [TUI.md](TUI.md).
- **JSONL RPC (planned):** the same event stream on stdin and stdout, for
  editors and scripts.
- **CI mode (planned):** runs the gate against a pull request that changes the
  genome.

## 9. Crates

The dependency graph is one-way and enforced by Cargo.

```
kalki-protocol  ←  kalki-core  ←  kalki (binary)
       ↑                              ↑
   kalki-tui (planned)            kalki-gate (planned)
```

| Crate | Holds | Status |
|---|---|---|
| `kalki-protocol` | Event and wire types. Depends on `serde` and `thiserror` only. | Built |
| `kalki-core` | Kernel: log, blobs, policy, provider wires, session loop, tools. Prints nothing. | Built |
| `kalki` | The `kalki` binary. Owns the terminal and exit codes. | Built |
| `kalki-tui` | Terminal UI. Depends on `kalki-protocol` only. | Planned, v0.0.3 |
| `kalki-gate` | Admission checks and the evaluation runner. | Planned, v0.0.5 |
| `xtask` | Repo automation: `cargo xtask ci`. No dependencies. | Built |

A crate is created when its first code lands. There are no empty crates.

## 10. State on disk

```
<config dir>/kalki/policy.toml     deny-first policy
~/.kalki/sessions/<id>/            log segments for one session
~/.kalki/blobs/<sha256>            large tool outputs
~/.kalki/genome/store/<hash>/      admitted genome versions (planned)
~/.kalki/gate/                     gate log, thresholds, tasks, checkers,
                                   protected set, archive (planned)
<repo>/.kalki/genome/              project genome: proposals and a shared
                                   copy, inert until admitted (planned)
<repo>/.git/refs/kalki/cp/         workspace checkpoints (planned)
```

`KALKI_HOME` overrides `~/.kalki`. Nothing is sent to Kalaris Labs. There is
no telemetry.

## 11. Rules that follow from the design

These are binding. [AGENTS.md](../AGENTS.md) turns them into checks.

1. The log is the only durable truth. No side files, no hidden state.
2. A summary is a view. It never replaces the record it summarizes.
3. Messages keep their role from the provider wire to the compiled context.
4. Tool output is data. It is scrubbed, truncated, and never treated as an
   instruction.
5. Rules are enforced in the policy engine, not requested in a prompt.
6. No genome version loads without an admission record or a trusted
   signature.
7. The gate, the policy, the evaluation tasks, and the log are outside the
   agent's write set. Held-out tasks and the gate log are also outside its
   read set. On one machine this is as strong as the sandbox.
8. Every number Kalki publishes names its model, task set, run count, and cost.

## 12. What exists today (v0.0.1)

| Capability | State |
|---|---|
| Headless session loop: objective, model turns, tool calls, turn cap | Built, tested against a mock model |
| Event log: CRC framing, segment rotation, torn-write repair, replay | Built, with kill drills |
| Blob store with 4 KiB inline cap | Built |
| Deny-first policy engine | Built. Applied to `bash`; not yet to `apply_patch`. |
| Tools: `bash` (argv), `apply_patch` | Built |
| Provider wires: OpenAI Responses, Anthropic Messages, OpenAI-compatible | Built, tested against recorded streams. Not yet run against every live provider. |
| CI on Linux, macOS, Windows; MSRV 1.88; release pipeline for five targets | Built |
| Hash chain, audit verify, resume, model catalog | Planned, v0.0.2 |
| Sandbox, terminal UI, gateway, Pi executor, rewind | Planned, v0.0.3 |
| Context compiler | Planned, v0.0.4 |
| Genome and gate | Planned, v0.0.5 |
| Offline evolution and public scorecard | Planned, v0.0.6 |

## 13. Open questions

1. **Gate thresholds.** The noise tolerance `τ` and cost limit are unmeasured
   for our setup. They will be set from our own repeated runs, not copied from
   papers.
2. **Is a stricter gate better?** Strict admission discards useful
   alternatives (arXiv:2611.07624). The archive is our answer. It is untested.
3. **Sandbox on Windows.** No equivalent of Landlock exists for stock Windows.
   The plan is Job Objects plus an AppContainer, with WSL2 as the stronger
   option.
4. **External executors and tool approval.** Kalki confines Pi as a process
   and cannot approve its tool calls one by one. Whether process confinement
   is enough is an open security question.
5. **Long runs.** No published system has shown a 1000-hour unattended run.
   Kalki makes no such claim. Soak tests are a v0.1 goal.
6. **Gate cost.** Every comparison needs `k` live runs per arm. The values of
   `k` and the protected-set size that keep the gate both sound and
   affordable are unmeasured. If no such values exist, the gate admits
   nothing, which is a stop condition in the roadmap.
7. **Anchoring the hash chain.** A process running as the same user can
   rewrite a log and re-chain it. The chain head needs an anchor outside the
   agent's write set, such as the gate log or a signed git tag. This is not
   designed yet.
8. **Memory admission.** Persistent memory goes through the same gate as
   prompts and skills. That may be too slow for memory to be useful. A
   lighter path is undecided.

Questions 6 to 8, and the changes to sections 5 and 6 that go with them, came
from a zero-context design review on 2026-11-11.
