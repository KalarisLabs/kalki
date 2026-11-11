---
name: harness-engineering
description: "Engineering autonomous coding harnesses, agent execution runtimes, durability (WAL), context compilation, sandboxed tool dispatch, eval oracles, and self-recursive continual learning."
---

# Harness Engineering

The engineering discipline of building, operating, and evaluating agent harnesses, execution runtimes, and self-improving agent loops.

## Core Axiom

> **The binding constraint on autonomous software engineering is compounding error across steps, not raw model capability.** Harness-layer architecture (externalized state, verifier-gated transitions, JIT context compilation, tool sandboxing) dictates long-horizon agent performance more than model-layer shifts.

---

## 1. The 10-Layer Harness Architecture

```
Layer 1: Surface        [CLI / TUI / Socket API / Headless JSON]
Layer 2: Supervisor     [Crash-only Daemon / Process-per-session / Continue-As-New]
Layer 3: State Plane    [Append-only WAL / CRC32C / SHA-256 Chain / Content-Addressed Blobs]
Layer 4: Context Plane  [JIT Subtask-Tree Compiler / Prefix Cache Preservation / Masking]
Layer 5: Model Plane    [Provider Trait / Multi-Provider SSE / Token Accounting / Rate Limits]
Layer 6: Tool Plane     [Results-as-Handles / Sandboxing: WASI, Landlock, Bollard / Deny-First Policy]
Layer 7: Recursion      [Genome CRUD / Cross-Task Experience Library / Oracle Admission Gate]
Layer 8: Governance     [Hash-Chained Audit / Zero Ambient Authority / Signed Releases]
Layer 9: Observability  [OTel Tracing / Prometheus Metrics / RSS Slope / Crash Drills]
Layer 10: Distribution  [Single Static Binary / Zero Runtime Dependencies]
```

### Layer 1: Surface
- **Attach/Detach Session Model**: Long-running sessions execute independently of the presentation layer. Detaching the TUI/CLI must not kill the session.
- **Protocol Enums Only**: Cross-process communication uses strictly typed protocol enums (e.g., `kalki-protocol`). No unstructured ad-hoc JSON over IPC sockets.

### Layer 2: Supervisor
- **Crash-Only Design**: Workers are killable at any `await` point (`kill -9`). Reattachment via the daemon must resume execution with zero data loss.
- **Worker Generation Cycling**: Fresh worker generation every $N$ hours with state reconstructed from WAL replay (the *Continue-As-New* pattern), preventing memory leaks and runtime state corruption.

### Layer 3: State Plane (Durable Execution)
- **Append-Only WAL**: All state transitions, turns, and tool calls are recorded to an append-only Write-Ahead Log before being dispatched or shown to UI.
- **Framing & Hash Chaining**: Every log record has an 8-byte length prefix, CRC32C framing, and a `prev_hash` SHA-256 chain for tamper detection.
- **Content-Addressed Blobs**: Payloads exceeding 4 KiB are stored in a content-addressed blob store; WAL events carry a `payload_ref`.
- **Idempotent Replay**: Boot sequence verifies snapshot anchor + log tail. Tool calls carry idempotency keys to prevent duplicate execution during recovery.

### Layer 4: Context Plane (Compiled Disposable Cache)
- **Context is Cache, Not Memory**: Model context is a JIT-compiled projection of persistent state, never the source of truth.
- **Subtask Trees**: Goal $\to$ Task $\to$ Subtask tree is compiled just-in-time for each prompt.
- **Eviction & Masking Over Summarization**: Never summarize and delete history. Evict old detail or mask resolved branches while keeping full logs queryable from durable storage.
- **Stable Prefix Preservation**: Structure prompts with static system rules and persistent schemas at the front to maximize KV cache hit rates (Anthropic prompt caching, Gemini context caching).

### Layer 5: Model Plane
- **Internal Trait Abstraction**: Abstract model providers (OpenAI, Anthropic, Gemini, local Ollama) behind a strict internal trait. Pin dependencies and isolate wire format differences.
- **Streaming & Token Accounting**: Stream SSE deltas, track prompt/completion tokens, and enforce exponential backoff with jitter on rate limits.

### Layer 6: Tool Plane & Sandboxing
- **Results-as-Handles**: Large tool outputs (stdout, file dumps) must be stored in the blob store and returned to context as `handle + schema + size`. Fetch slices on demand to prevent **Context Rot** and **Context Anxiety**.
- **Tiered Sandbox Isolation**:
  1. *WASI / Wasmtime*: Untrusted scripts, parsers, and plugins.
  2. *Landlock + Seccomp + Rlimits + Netns*: Fast (<30ms) local host execution with bounded CPU/RAM and no default network.
  3. *OCI Containers (Bollard)*: Networked, full-environment testing.
  4. *Windows*: Job Objects + Restricted Tokens (or WSL2 backend).
- **Deny-First Policy**: All tool calls require authorization against declared policies (e.g., Cedar policy-as-code) with stable rule IDs.

### Layer 7: Self-Recursion & Continual Learning
- **Two Recursion Loops**:
  - *Loop 1 (Task-Level Genome CRUD)*: Modifies prompt templates, skills, or subagent profiles $H=(\rho, G, K, M)$.
  - *Loop 2 (Cross-Task Experience Library)*: Synthesizes execution contrasts into structured experience records.
- **The Oracle Admission Gate**: No self-modification may be adopted without:
  1. Positive evaluation delta on a held-out, pinned benchmark slice.
  2. Regression ceiling (e.g., $\le 10\%$ regression on any single subtask).
  3. Hack-tripwire tests clean (detects reward hacking, e.g., modifying test runners or hardcoding outputs).
  4. Instant rollback snapshot archived before applying changes.
  5. Evaluator separation: the evaluating model is never the executing agent.

### Layer 8: Governance & Audit
- **Tamper-Evident Audit Trails**: Every permission grant, tool execution, and state change is signed and hash-chained.
- **Zero Ambient Authority**: Tools execute with minimal necessary capabilities. No tool inherits user ambient environment variables or unsanitized credentials.
- **Airgapped / Local-First Readiness**: Support fully local model endpoints, offline embedded documentation, and local-only state.

### Layer 9: Observability
- OpenTelemetry tracing, Prometheus gauges for WAL lag, snapshot age, memory RSS slope, and tool latency percentiles ($p50, p95, p99$).
- Automated crash drills (`kill -9` injected at mid-turn, mid-tool-call, and mid-snapshot) to guarantee 100% seeded recovery.

### Layer 10: Distribution
- Single static musl/target binary (`cargo-zigbuild`), reproducible builds, SLSA provenance, and signed releases.

---

## 2. Common Anti-Patterns & Diagnoses

| Anti-Pattern | Manifestation | Root Cause | Harness Fix |
|---|---|---|---|
| **Context Rot** | Model ignores earlier instructions, hallucinating non-existent tools | Stuffing unbounded tool stdout/diffs directly into context | Implement **Results-as-Handles**; store payloads in blob store |
| **Reward Hacking** | Agent passes tests by editing the test script or mocking the oracle | Agent has write access to test runner, evaluation harness, or reward channel | Isolate evaluation oracle into an unmodifiable sandbox tier |
| **Approval Fatigue** | User auto-approves 90%+ prompts, granting accidental dangerous access | Interactivity friction on every minor tool call | Declare **Deny-First Policy Rules** with deterministic capability grants |
| **State Drift** | Crash or restart causes agent to re-execute side effects or drop subtasks | Ephemeral memory held in RAM; no write-ahead logging | **Append-Only CRC-Framed WAL** with idempotent tool-call replay keys |
| **Context Anxiety** | Degraded reasoning as context window approaches 70%+ capacity | Accumulating resolved turn history without compaction | **JIT Subtask-Tree Compilation** + masking resolved branches |
