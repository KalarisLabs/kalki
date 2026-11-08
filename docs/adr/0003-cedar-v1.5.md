# ADR-0003: Cedar policy engine in v1.5 (sketch)

- Status: proposed
- Date: 2026-11-08
- Deciders: Sayan Chowdhury

## Context

v1 ships a hand-rolled deny-first TOML policy engine (`kalki_core::policy::PolicyEngine`) with stable rule IDs — the Claude Code/Codex Rules pattern, deliberately minimal. Institutions will eventually want attribute-based conditions (time, principal, target sensitivity, grant provenance) that outgrow TOML.

## Decision (proposed)

Adopt AWS Cedar (`cedar-policy`, Apache-2.0) behind the existing `Decision` API in v1.5: same `decide(&ToolRequest) -> Decision` shape, policy files translate via a `kalki policy convert` tool, rule IDs survive as annotation strings, and the audit log records both the TOML-era rule IDs and Cedar entity references. `cedar-policy` is not a 2026-recommended dependency for v1 — it would displace the 11-column dependency budget before the Cedar-side review exists.

## Consequences

v1 keeps a legible, auditable policy layer; v1.5 adds expressiveness without a model-facing change. If institutions stay TOML-comfortable, Cedar may never ship — that is an acceptable outcome and this ADR is superseded rather than forced.

## Verification

Institutional pilot (≥ 2 orgs) must demand ABAC before v1.5 work starts; otherwise this ADR dies on the vine, intentionally.
