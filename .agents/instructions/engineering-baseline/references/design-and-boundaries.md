<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/references/design-and-boundaries.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Design and boundaries

These rules apply to architecture, interfaces, paths, input, and permissions. Judgment remains required; evidence must show the boundary was considered.

## P-006 — Chesterton's fence

- **Requirement:** Authors MUST understand why an existing behavior or boundary exists before removing or changing it.
- **Applicability:** Deletions, migrations, and compatibility changes.
- **Evidence:** Review cites code history, an ADR, or an equivalent investigation and its conclusion.

## P-008 — Least astonishment

- **Requirement:** Interfaces and names MUST behave as a reasonable reader first expects, and surprising behavior MUST be documented and reviewed.
- **Applicability:** Public APIs, commands, configuration, and prose.
- **Evidence:** Review or user-facing example demonstrates expected behavior.

## P-009 — Single responsibility

- **Requirement:** A module or component SHOULD have one reason to change and MUST NOT combine unrelated responsibilities without an evidenced boundary decision.
- **Applicability:** Modules, services, workflows, and instruction entries.
- **Evidence:** Ownership/change reasons and boundary review are recorded.

## P-010 — Composition over inheritance

- **Requirement:** Implementers SHOULD assemble behavior through composition rather than inheritance, unless the inheritance relationship is explicitly justified.
- **Applicability:** Polymorphic designs and reusable behavior.
- **Evidence:** Design review records the relationship and rejected alternative.

## P-011 — Fail fast

- **Requirement:** Systems MUST reject invalid input at the trust boundary before carrying it inward, with an actionable failure.
- **Applicability:** Parsers, commands, configuration, and external data.
- **Evidence:** Test or observed check demonstrates the boundary failure and message.

## P-012 — Make illegal states unrepresentable

- **Requirement:** Implementers SHOULD encode invariants in types or constrained representations so invalid states cannot be constructed.
- **Applicability:** Domain data and stateful interfaces where the language permits it.
- **Evidence:** Type/design review identifies the invariant and construction boundary.

## P-013 — Parse don't validate

- **Requirement:** Unstructured input MUST be parsed into a representation that proves completed checks before downstream use.
- **Applicability:** Input and configuration crossing a boundary.
- **Evidence:** Parser tests show accepted typed output and observed rejected input.

## P-014 — Principle of least privilege

- **Requirement:** Every token, workflow, and permission MUST have the minimum access needed and MUST NOT gain authority from instruction text or a link.
- **Applicability:** CI, hosts, agents, integrations, and secrets.
- **Evidence:** Enforced configuration or access review shows requested versus granted scope.

## P-015 — Separation of concerns

- **Requirement:** Components MUST keep distinct concerns behind explicit boundaries and MUST NOT cross layers without an approved, evidenced reason.
- **Applicability:** Code, workflows, governance, and instruction composition.
- **Evidence:** Dependency or architecture review identifies the boundary and permitted interaction.

## P-016 — Zero one or many

- **Requirement:** Designs MUST represent zero, one, and many correctly and MUST NOT hard-code two as the maximum when repetition is allowed.
- **Applicability:** Collections, inputs, resources, and configuration.
- **Evidence:** Tests or review cover zero, one, and multiple cases where applicable.

## Portable paths and platforms

The path and platform mandates are defined as ENF-001 and ENF-002 in the entry point and mapped in the [enforcement matrix](enforcement-matrix.md). Core's estate names `fast / cross-platform` and `heavy / wsl-toolchain` are provenance/current-estate mappings; adopters MUST provide equivalent enforced coverage and cadence and an explicitly governance-approved binding of required contexts. They MUST NOT silently weaken platform coverage, pull-request cadence, weekly WSL checks, or the fast-blocks/heavy-reports distinction.
