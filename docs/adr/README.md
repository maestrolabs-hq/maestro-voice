<!-- Explain when and how to record an architectural decision. -->
# Architecture decision records

An ADR keeps a choice and its reasoning beside the thing it governs. Write
one for anything that would otherwise be re-derived: a boundary, a trade-off,
a gate or a constraint whose reason is not obvious from the code.

Copy [0000-template.md](0000-template.md). Number records from `0001` upward,
using four digits and a kebab-case title, for example
`0001-record-decisions.md`. The zero template is not a decision.

Statuses: `proposed`, `accepted`, `superseded by NNNN`, `rejected`.
Accepted ADRs are superseded, not rewritten. Update the old status to point
to the replacement; preserve its reasoning.

[CODEOWNERS](../../.github/CODEOWNERS) routes ADR review to the reasoning,
not just the diff. State the cost and unresolved questions, not only the choice.
