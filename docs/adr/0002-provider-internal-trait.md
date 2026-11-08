# ADR-0002: Provider quar: async-openai quarantined behind an internal trait

- Status: accepted
- Date: 2026-11-08
- Deciders: Sayan Chowdhury

## Context

We need OpenAI (Responses + Chat Completions), Anthropic Messages, Gemini, and OpenAI-compatible endpoints (Kimi/GLM/DeepSeek/xAI/Ollama). `async-openai` 0.43.0 covers Responses well but is OpenAI-scoped and 0.x; `genai` 0.7.3 covers the long tail but is young; rig-core is too opinionated (0.46.0, ~60 breaking releases).

## Decision

Internal `kalki_core::provider::Provider` trait is the only call site. v0.1 ships it implemented by `async-openai` (Responses) and hand-rolled SSE paths (Messages/Chat Completions/streaming). If the provider conformance suite (tracked in `research_notes/Kalki Code/provider_layer.md`) fails on any target wire, we swap the impl to a hand-rolled reqwest client without touching call sites.

## Consequences

We inherit OpenAI-first velocity without marrying their crate; the long tail is ours to own. Costs: two code paths to maintain; mitigated by a single conformance fixture suite (wire-identical outputs asserted across impls).

## Alternatives considered

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| genai 0.7.3 everywhere | one impl, all wires | young; unknown breakage surface | fallback impl |
| rig-core | battery included | opinionated runtime; 0.x churn; heavy | rejected as framework |
| hand-rolled only | total control | slowest to first demo | rejected for v0.1 |

## Verification

Conformance matrix in CI: same prompt → identical streamed event sequence from async-openai and hand-rolled impl (semantic equality of tool calls, usage, and finish reasons).
