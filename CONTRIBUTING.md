<!-- Explain how to prepare, verify and submit a change. -->
# Contributing

## Setup

`just setup` installs hooks for `pre-commit`, `pre-merge-commit` and `pre-push`.
Install the documented prerequisites for the selected language first; setup
is explicit, never something reading this file does.

## The shape of a change

`main` takes changes through a PR with the checks green, for the maintainer too.
Use a branch name that describes the change; `change-topic` below is an example.

```text
git switch -c change-topic
just setup
just check
git push -u origin change-topic
gh pr create --fill
gh pr merge --squash
```

## Before writing code

Write the failing test first and watch it fail for the reason you expect.
A test that passes immediately may be testing the implementation rather than
the behaviour wanted. Documentation-only changes need review, not invented tests.

## Gates

`just check` runs every CI gate except one: the full-history secrets scan runs
in CI only, because it needs the whole history and the gitleaks binary;
locally the hook scans staged changes on every commit.
Heavy CI reports are separate evidence, never required merge checks. Local
success does not prove another platform; CI covers every claimed platform.

Hooks run common checks on commit and merge-commit, and language gates on push.
Fixers may modify files: inspect those changes and re-run the commit.
If a gate blocks correct code, say so in the PR; never bypass it quietly or
weaken it to pass. Allowlists need a reason and a check that entries remain true.

## Commit messages and releases

Use `type: description`, with an optional scope: `type(scope): description`.
Accepted types are `feat`, `fix`, `docs`, `refactor`, `test`, `ci`, `build`,
`chore`, `perf` and `revert`.

- From 1.0 onward, `feat` releases a minor version; `fix` and `perf` release a patch.
- A `!` or `BREAKING CHANGE:` footer declares a breaking change, releasing a
  major version from 1.0 onward. Explain the break in the footer, not only
  in the PR description.
- Before 1.0, the base policy releases features as patches and breaking changes
  as minor versions; fixes and performance changes remain patches.
- Other types do not release on their own unless marked breaking.

release-please opens the release PR from Conventional Commits. The `release-please` workflow does that on every push to main; with the default token its PR does not trigger the fast checks, so bind an app or token if they must run there. Merging it
publishes the tag, generated changelog and release assets through the selected
language's release workflow. Bootstrap alone does not build or publish assets;
verify that workflow before claiming a release is complete.

## Rules

English only in prose and identifiers. No path that names a machine in code,
configuration, workflows or documentation; derive paths at runtime and use
`/somewhere` in tests. These are separate gates.

See [AGENTS.md](AGENTS.md) for the working rules and the instructions it
points at, and [GOVERNANCE.md](GOVERNANCE.md) for the repository ruleset.
