# ADR-0005: The first release is v0.0.1, and releases ship from GitHub only

- Status: accepted
- Date: 2026-11-11
- Deciders: Sayan Chowdhury
- Supersedes: the version numbers in ADR-0001 and in the earlier roadmap

## Context

The workspace was numbered 0.1.0 and the roadmap ran 0.1 → 0.2 → 0.5 → 1.0.
Nothing had been tagged or published. A `0.1.0` label suggests more maturity
than a headless loop with no sandbox has.

## Decision

1. The first public release is **v0.0.1**. All crates share the workspace
   version.
2. While the version is `0.0.x`, every release may break anything. The patch
   number is a milestone counter.
3. **v0.1.0** is the first release with a compatibility promise. From v0.1.0
   the event log format and the `kalki-protocol` wire types change only with a
   migration path.
4. **v1.0.0** adds the CLI and config to that promise.
5. Releases are GitHub Releases built by `.github/workflows/release.yml` from a
   signed tag. There is no crates.io, npm, or Homebrew package before v0.1.0.
6. Every release is created as a draft and marked pre-release. A maintainer
   publishes it after checking the artifacts.

## Consequences

- Version strings do not appear in source comments or user-facing text. The
  binary reports `CARGO_PKG_VERSION`.
- The release workflow refuses a tag that does not match the Cargo version or
  that has no changelog section.
- Users get binaries with a checksum file and a GitHub build provenance
  attestation. Detached minisign signatures are deferred until there is a key
  ceremony to document.

## Alternatives considered

| Option | Verdict | Why |
|---|---|---|
| Keep 0.1.0 | Rejected | Overstates maturity. |
| CalVer | Rejected | Says nothing about compatibility, which is what users of a log format need. |
| `cargo-dist` generated workflow | Deferred | A hand-written matrix is about 100 lines and has no generator to keep in sync. Revisit when installers are needed. |

## Verification

`cargo xtask ci` passes at version 0.0.1. The release workflow has a manual
dry-run trigger that builds all five targets without creating a release. It
has not yet run on GitHub.
