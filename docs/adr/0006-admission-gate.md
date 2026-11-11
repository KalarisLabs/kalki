# ADR-0006: Self-modification is admitted by an external gate with eight checks

- Status: accepted (design); implementation planned for v0.0.5
- Date: 2026-11-11
- Deciders: Sayan Chowdhury

## Context

Earlier documents said self-improvement would sit behind an "oracle gate":
adopt a change only on a positive held-out evaluation delta, with rollback.
The 2026 literature shows that rule is necessary and not sufficient.

- A change can raise the mean score and still break tasks that passed. In one
  study 55% of rejected proposals did exactly that (arXiv:2609.24130). In
  another, all sixteen evolved agents lost previously solved tasks
  (arXiv:2611.07624).
- A positive delta on a small task set is often noise, and selecting on it
  repeatedly overfits (arXiv:2609.24972).
- Evolution adds cost unless cost is part of acceptance (arXiv:2609.24972,
  arXiv:2610.26770).
- The single best-scoring candidate is often not the most useful
  (arXiv:2611.07624).

## Decision

A candidate genome `h'` replaces the current genome `h` only if all eight
checks pass. They run in this order so that cheap checks reject first.

| # | Check | Passes when |
|---|---|---|
| 1 | Scope | The diff touches only genome files and is within this round's edit budget. |
| 2 | Leakage | The diff contains no task identifiers, answers, or constants from the evaluation tasks. |
| 3 | Trigger | On the failures that motivated the change, `h` and `h'` are each run `k` times from the same starting state, model, and settings. `h'` passes more often than `h` by more than measured noise. |
| 4 | Preservation | A protected case is lost when its pass rate under `h'` falls below its rate under `h` by more than its flake margin. `h'` loses at most `m` cases. Default `m = 0`. |
| 5 | Noise floor | On held-out tasks, `score(h') ≥ score(h) − τ`, both as means over `k` runs. `τ` is measured from repeated runs of the unchanged genome. This is a no-harm test. |
| 6 | Cost | If the gain exceeds `τ`: added tokens per task ≤ `κ ×` gain. Otherwise the change must reduce cost or delete something. |
| 7 | Tripwires | No attempt to read a canary (canaries sit in sandbox-denied paths), no test or checker modified, no reference solution retrieved from history. |
| 8 | Rollback | The parent version exists in the genome store, outside the workspace. |

Further rules:

- The gate runs in a process the agent cannot write to. Its thresholds, the
  evaluation tasks, and the protected set are outside the sandbox write set.
  Locally that is as strong as the sandbox. In CI, with held-out tasks as
  secrets and the gate on a protected branch, it does not depend on the
  sandbox. CI is the preferred deployment for project genomes.
- Comparisons use live model runs. A recorded response was produced under the
  old genome and is not evidence about a candidate. Log replay and the mock
  model test gate code, not genomes.
- The judge of a run is never the model instance that produced it.
- Every decision is a record in a gate log, separate from session logs and
  outside the proposer's read set. The proposer learns which check failed.
  For the held-out checks 5 and 6 it learns only pass or fail.
- The kernel loads a genome version only if its hash is in an admission
  record or it carries a trusted signature. Files in a repository are
  proposals until then.
- An admission record names the model, executor, and settings. The change is
  active only for those.
- Rejected candidates go to an archive with a note on where they helped.
- The protected set is re-run on a schedule against the whole active genome.
- Thresholds `τ`, `κ`, and `m` are configuration, set from our own measurements.

## Consequences

- Self-modification is slower than an ungated loop. That is the intent.
- Each decision costs `2k` live runs per case compared. The values of `k` and
  the protected-set size that keep this affordable are unmeasured.
- The gate needs evaluation tasks with checkers that cannot be gamed. Building
  that set is most of the work.
- Until the gate exists, no part of Kalki changes its own genome.

## Alternatives considered

| Option | Verdict | Why |
|---|---|---|
| Positive mean delta only | Rejected | Hides lost tasks and admits noise. |
| Human approval for every change | Kept as an option | Does not scale to an offline loop, and a human cannot see regressions without the same evidence. |
| No ground truth, behavioural contrast only | Rejected as the admission rule | Useful for proposing. Not evidence of correctness. |

## Review

A zero-context design review on 2026-11-11 found four flaws in the first
draft of this record, all corrected above: replay cannot evaluate a
prompt-side change; checks 3 and 4 ignored run-to-run noise; a genome stored
in the workspace could be rewritten by `git checkout`; and gate evidence in
the session log leaked held-out results to the proposer.

## Verification

Planned. The gate gets its own test suite with seeded bad candidates: one that
hard-codes an answer, one that breaks a protected case, one that only adds
cost, one that edits a checker, and one that changes nothing while its trigger
case is flaky. Each must be rejected by the intended check. A genome file
changed by `git checkout` must not load.
