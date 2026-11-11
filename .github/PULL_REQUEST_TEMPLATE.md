## What changed

<!-- One paragraph. Link the issue or ADR. -->

## Evidence (AGENTS.md rule 40)

- **Measured:** <!-- commands you ran and their result, e.g. `cargo xtask ci` green -->
- **Assumed:** <!-- what you believe but did not check -->
- **Not verified:** <!-- say "unmeasured" out loud -->

## Checklist

- [ ] `cargo xtask ci` passes locally
- [ ] Every commit is signed off (`git commit -s`)
- [ ] `CHANGELOG.md` has a line under `[Unreleased]` if behaviour changed
- [ ] `DEPENDENCIES.md` updated if a dependency changed
- [ ] Crash drill added or updated if this touches the log, snapshots, or grants
- [ ] ADR added if this changes protocol, log format, policy, or the gate
