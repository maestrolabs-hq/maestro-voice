<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/references/testing-and-verification.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Testing and verification

These rules apply to behavior changes and quality gates. Evidence may be automated or review-based, but structural Markdown tests are not behavioral certification.

## Required behavior evidence

Behavior changes MUST write the failing test first and MUST observe it fail before implementing the fix. A test that passes immediately proves nothing about the requested behavior. The evidence record MUST include the observed failure and subsequent result. Deterministic controls may reject a change automatically; review and evaluation judgments MUST remain identified as judgments in evidence.

Cheap commit checks and pre-push checks SHOULD be separated from fast required CI and weekly heavy checks as described in [change discipline](change-discipline.md). Missing or unavailable controls MUST be reported unsupported/noncompliant; they MUST NOT be silently treated as passing.

## Evidence quality

A check MUST exercise the claimed behavior rather than merely parse a document. Evaluation of judgment-based principles MUST identify the reviewer, decision, rationale, and scope in the host's authorized record; this package does not invent approver identities. No arbitrary LOC, duplication, or threshold substitutes for review judgment.
