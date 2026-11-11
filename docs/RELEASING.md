# Releasing

How a Kalki release is cut. The policy behind it is
[ADR-0005](adr/0005-versioning-and-release-channel.md).

## Rules

- The first release is `v0.0.1`. While the version is `0.0.x`, any release may
  break anything.
- A release is a signed tag on `main`. The tag is `v` plus the workspace
  version in `Cargo.toml`.
- Pushes and tags are made from the `@saynchowdhury` GitHub account.
- The workflow creates a **draft pre-release**. A maintainer publishes it.

## Before you tag

1. `main` is green in CI.
2. `cargo xtask ci` passes locally.
3. `Cargo.toml` `[workspace.package] version` is the version you are releasing.
4. `CHANGELOG.md` has a `## [x.y.z] - YYYY-MM-DD` section for it.
5. Run the release workflow by hand as a dry run and confirm all five targets
   build: **Actions → release → Run workflow**. A dry run builds and uploads
   workflow artifacts. It creates no release.

## Cut the release

```bash
git switch main && git pull --ff-only
git tag -s v0.0.1 -m "Kalki v0.0.1"
git push origin v0.0.1
```

The `release` workflow then:

1. Checks that the tag matches the Cargo version and that the changelog has a
   section for it. It stops if either is wrong.
2. Builds `kalki` for five targets and runs `kalki --version` and
   `kalki doctor` on each native runner.
3. Packages each binary with the licenses, notice, readme, and changelog.
4. Writes `SHA256SUMS` and a GitHub build provenance attestation.
5. Creates a draft GitHub Release with the changelog section as notes.

## Verify and publish

```bash
gh release view v0.0.1
gh release download v0.0.1 --dir dist
cd dist && sha256sum -c SHA256SUMS
gh attestation verify kalki-v0.0.1-x86_64-unknown-linux-musl.tar.gz --repo KalarisLabs/kalki
```

Unpack one archive, run `kalki doctor`, then publish:

```bash
gh release edit v0.0.1 --draft=false
```

## Artifacts

| File | Platform |
|---|---|
| `kalki-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` | Linux x86_64, static |
| `kalki-vX.Y.Z-aarch64-unknown-linux-musl.tar.gz` | Linux aarch64, static |
| `kalki-vX.Y.Z-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon |
| `kalki-vX.Y.Z-x86_64-apple-darwin.tar.gz` | macOS Intel |
| `kalki-vX.Y.Z-x86_64-pc-windows-msvc.zip` | Windows x86_64 |
| `SHA256SUMS` | Checksums for all of the above |

## If a release is bad

Do not delete or move a published tag. Mark the release as broken in its
notes, fix forward, and release the next patch version.

## Known gaps

- The workflow has not yet run on GitHub. The first dry run is its real test.
  The static Linux builds compile C code for TLS with the musl toolchain, and
  that is the step most likely to need adjustment.
- macOS binaries are not notarized and Windows binaries are not signed.
  Gatekeeper and SmartScreen will warn.
- There is no install script, package-manager formula, or self-update.
