<!-- Record who decides and the repository controls that constrain them. -->
# Governance

## Who decides

One maintainer: @FrancoisLDaigneault. The maintainer is constrained by this
repository's ruleset, not exempt from it.

## What the ruleset and platform enforce

These are deployment requirements, not settings installed by these files.
Verify them on this repository before claiming they are active; unresolved
controls are not passes.

| Control | Per-repository binding |
| --- | --- |
| No force-push or branch deletion | Protected branches, including main; no bypass actors. |
| Version tags immutable | Refuse updates and deletion of version tags. |
| PR required, squash only | Main, including maintainer changes; zero approving reviews. |
| Required status checks | `fast / common` plus every fast context from the selected language workflow, including each claimed platform. |
| Conventional Commits | Refuse commit messages outside the types in CONTRIBUTING.md. |
| Actions pinned to full SHA | Every external action reference, checked by workflow audit. |
| GITHUB_TOKEN read-only by default | Repository workflow permissions; grant writes only in the job needing them. |
| Two-factor authentication | Maintainer account uses a secure second factor. |

Language context names come from the deployed language workflow, not an
invented generic check. The language folders have not been authored in base;
record the exact contexts when selecting one. Heavy contexts are never required.

## How a change lands

1. Create a branch and a pull request; direct pushes to main are refused.
2. All required fast checks pass against the current base.
3. Squash merge.

Zero approving reviews: one maintainer cannot approve their own PR. The gate
is the checks, not a rubber stamp. See [CONTRIBUTING.md](CONTRIBUTING.md).

## How a decision is recorded

Write an [ADR](docs/adr/README.md) beside what it governs for anything that
would otherwise be re-derived. State the choice, its cost and what remains
unresolved. Accepted decisions are superseded, not silently rewritten.
Security reports follow [SECURITY.md](SECURITY.md).
