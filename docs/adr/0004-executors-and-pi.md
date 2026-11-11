# ADR-0004: Executors are pluggable, and Pi is the first external one

- Status: accepted
- Date: 2026-11-11
- Deciders: Sayan Chowdhury

## Context

Kalki needs an agent loop. Two paths were open.

1. Build on [Pi](https://github.com/earendil-works/pi), a minimal TypeScript
   agent harness (MIT, v1.5.2 on 2026-11-10). Pi has a mature loop, a unified
   provider layer, a terminal UI, and a documented JSONL RPC mode.
2. Keep building the native Rust loop that already exists in `kalki-core`
   (session loop, three provider wires, policy-checked `bash`, path-confined `apply_patch`).

Facts that bear on the choice:

- Pi requires Node.js 22.19 or newer. Kalki ships as one static binary.
- Pi has no permission system. Its README says it runs with the permissions of
  the user who launched it.
- Pi was one of five harnesses whose compaction step let a forged user turn
  survive into memory 87–100% of the time (arXiv:2610.29869).
- Recent harness-evolution research uses Pi as its base and reports that
  mechanisms found on it transfer across models (arXiv:2609.20519).
- Kalki's value is the kernel and the gate. The agent loop is the part that
  the gate improves, so it should be replaceable.

## Decision

The kernel defines an executor boundary. Two executors are planned.

- **`native`**: the Rust loop in `kalki-core`. The reference executor. Every
  tool call is checked against policy before it runs.
- **`pi`**: Pi started as a child process with `pi --mode rpc`. Kalki sends
  commands and reads events over JSONL on stdin and stdout.

For any external executor, the kernel keeps control through three mechanisms
and none of them depends on the executor cooperating:

1. **Model traffic** goes through the kernel's loopback gateway. Calls are
   recorded, metered, and capped.
2. **The process** runs inside the sandbox.
3. **The workspace** is checkpointed by the kernel at each turn.

When Kalki drives Pi, Pi's own compaction is disabled and Kalki compiles the
context.

Kalki does not fork Pi, vendor it, or link Node.js. Pi is an optional program
found on `PATH`.

## Consequences

- A full-featured agent runs under Kalki's log and gate as soon as the gateway
  and the RPC adapter exist, without waiting for the native loop to catch up.
- Pi and the native loop can be compared on identical tasks. That comparison
  is the baseline every Kalki claim is measured against.
- Kalki cannot approve Pi's tool calls one at a time. Confinement for Pi is at
  process level. This is weaker than the native executor and the docs say so.
- Pi's RPC protocol can change. The adapter pins a tested Pi version range and
  fails with a clear message outside it.

## Alternatives considered

| Option | Verdict | Why |
|---|---|---|
| Fork Pi and build Kalki in TypeScript | Rejected | Loses the single binary, and puts the trusted kernel in the same runtime as untrusted extensions. |
| Embed Node.js in the Rust binary | Rejected | Large, and still two runtimes to secure. |
| Native loop only, no external executors | Rejected | Slower to a useful agent, and no independent baseline. |
| Agent Client Protocol adapter first | Deferred | Worth adding after Pi. One adapter is enough to prove the boundary. |

## Verification

Planned for v0.0.3. The same scripted mock-model session is run through both
executors, and the test asserts that the kernel log contains the same model
calls and the same workspace result.
