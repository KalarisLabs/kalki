# Security Policy — Kalki Code

Kalki Code executes model-written commands. We treat this as a hostile-environment problem by default.

## Reporting a vulnerability

- Email **hello@kalarislabs.com** with `[SECURITY] kalki-code` in the subject, or use GitHub Security Advisories on the repository.
- Please include: affected version, platform, reproduction steps, and impact.
- We acknowledge within **72 hours** and triage within **14 days**. We will credit you in the advisory unless you decline.
- No public disclosure until a fixed release exists (embargo up to 90 days, negotiable for critical issues).

## Threat model (short)

1. **Prompt injection** via repo files, issue bodies, or web content into tool calls — mitigated by data/instruction separation, deny-first policy, output scrubbing, and explicit egress grants.
2. **Tool-call misuse** (overbroad grants, `rm -rf`-class mistakes) — mitigated by layered sandboxing (WASI/landlock/seccomp/JOB objects) and grant auditing.
3. **Reward hacking** (agent editing policy, audit log, or its own genome without admission control) — mitigated by hash-chained audit log, server-side gates, and benchmark-gated self-adoption only.
4. **Supply chain** — `cargo deny` (licenses+advisories), daily `cargo audit`, SHA-pinned Actions, signed releases, SBOM per release.
5. **Secrets** — `kalki` never uploads code, prompts, or logs by default. API keys stay in env/keyring; nothing is sent to Kalaris Labs.

## Institution posture

Local-first, zero telemetry by default, zero code egress. A `THREAT_MODEL.md` and data-flow diagram for procurement review will ship with v0.5.
