<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/references/change-discipline.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Change discipline

These rules apply to authoring, review, and source control changes.

## P-007 — Boy Scout rule

- **Requirement:** Authors SHOULD leave code better within the diff they already have reason to touch, but MUST NOT use this rule to justify a drive-by refactor.
- **Applicability:** Any change touching existing code or documentation.
- **Evidence:** Review shows the bounded improvement or records why it was deferred.

## Contribution contract

Changes to `main` MUST arrive through a pull request, including maintainer changes; direct pushes MUST be refused by the platform. Commits MUST use exactly one accepted conventional type: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `ci:`, `build:`, `chore:`, `perf:`, or `revert:`. A gate that blocks something correct MUST be reported in the pull request; it MUST NOT be weakened, bypassed, or removed to make a change pass. One-line changes need no gratuitous process, but still require applicable safety checks.

The tier mapping, including local/CI command parity, is defined by ENF-008. The context-binding and non-bypass requirements are defined as ENF-002 and ENF-006 in the entry point. These controls are required downstream contracts; this repository does not claim they are universally deployed.
