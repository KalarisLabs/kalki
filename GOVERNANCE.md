# Governance

Kalki is an open-source project stewarded by Kalaris Labs. This file says who
decides what, and how that changes.

## Roles

| Role | Can do | How you get it |
|---|---|---|
| Contributor | Open issues, discussions, and pull requests | Sign off your commits (DCO) |
| Reviewer | Approve pull requests in an area | Sustained good reviews; nominated by a maintainer |
| Maintainer | Merge, cut releases, change this file | Invited by the existing maintainers |

Current maintainer: **Sayan Chowdhury** ([@saynchowdhury](https://github.com/saynchowdhury)).

The project has one maintainer today. That is a risk. The roadmap treats adding
a second maintainer from outside Kalaris Labs as a release goal.

## How decisions are made

1. **Ordinary changes** need one maintainer approval and green CI.
2. **Architecture changes** need an accepted ADR in `docs/adr/` before code is
   merged. That covers the wire protocol, the log format, the policy engine, the
   admission gate, and licensing. Anyone can propose an ADR. Discussion happens
   in the pull request that adds it.
3. **Disagreements** are settled by maintainer consensus. If there is none, the
   lead maintainer decides and records the reason in the ADR.

## What a pull request alone cannot decide

- Weakening the admission gate, the deny-first policy default, or the audit log.
- Adding telemetry that is on by default.
- Changing the license.

Each of these needs an ADR, a two-week public comment period, and the agreement
of all maintainers.

## Releases

Maintainers cut releases from signed tags. The process is in
[docs/RELEASING.md](docs/RELEASING.md). Pushes to this repository are made from
the `@saynchowdhury` GitHub account.

## Licensing and contributions

Kalki is dual-licensed `MIT OR Apache-2.0`. Contributions are accepted under the
[Developer Certificate of Origin](https://developercertificate.org). There is no
contributor license agreement.
