<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/references/simplicity-and-reuse.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Simplicity and reuse

These rules apply when designing, extracting, optimizing behavior, handling defects, or maintaining quality gates. Read the entry point before applicable work; its definitions and exception process are authoritative.

## P-001 — YAGNI

- **Requirement:** Teams MUST build for the requirement in front of them and MUST NOT add behavior solely for an anticipated future.
- **Applicability:** New features, abstractions, configuration, and dependencies.
- **Evidence:** Review links each addition to a current requirement.

## P-002 — KISS

- **Requirement:** Implementers MUST prefer the boring construct that a future reader can understand and MUST NOT choose cleverness without evidenced benefit.
- **Applicability:** All implementation and documentation.
- **Evidence:** Review records the simpler alternative considered where complexity is material.

## P-003 — DRY

- **Requirement:** Authors SHOULD factor genuinely identical knowledge into one authoritative representation, while preserving meaningful differences.
- **Applicability:** Repeated logic, policy, and vocabulary.
- **Evidence:** Review identifies the shared idea and its single source, or explains why repetition is intentional.

## P-004 — WET

- **Requirement:** Authors SHOULD allow a small, clear duplication to reveal whether two occurrences are truly one idea; WET balances, but does not cancel, DRY.
- **Applicability:** Proposed extractions and early repeated code.
- **Evidence:** Review records the duplication's boundary and why extraction is not yet justified.

## P-005 — Rule of three

- **Requirement:** Authors SHOULD extract a shared abstraction at the third clear occurrence, not merely the second, unless an earlier extraction is required for correctness.
- **Applicability:** Repeated behavior or structure.
- **Evidence:** Review identifies occurrences and the reason for extraction timing.

## P-017 — Premature optimisation

- **Requirement:** Implementers MUST measure a performance problem first and MUST NOT trade clarity or correctness for an intuition about speed.
- **Applicability:** Performance-sensitive changes; not ordinary correctness work.
- **Evidence:** Benchmark, profile, or production measurement records the observed bottleneck and result.

## P-018 — Broken windows

- **Requirement:** Maintainers MUST address an observed defect in touched scope or record it explicitly, and MUST NOT tolerate a weakened or bypassed quality gate (ENF-006).
- **Applicability:** Changes and quality controls; Boy Scout scope remains bounded by P-007.
- **Evidence:** Diff or issue record shows the bounded fix, report, or reasoned non-action.

## Reuse guardrails

Accepted duplication and retired-vocabulary exceptions MUST use justified, non-rotting allowlists. An entry requires a reason and a check that it remains true; thresholds and arbitrary line counts are not substitutes for judgment.
