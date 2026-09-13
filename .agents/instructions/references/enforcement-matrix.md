<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/references/enforcement-matrix.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Enforcement matrix

This matrix is the single mapping from stable IDs to required downstream mechanisms and evidence. `Deterministic` means a machine control can produce a repeatable result; `Judgment` means review or evaluation is required and is not machine-proof. `Not supplied` means these files do not implement or install the control. Exception authority is Core governance only unless marked **None**.

| ID | Name | Mechanism (type) | Required evidence | Exception policy |
| --- | --- | --- | --- | --- |
| FND-001 | Think before coding | Review (Judgment; not supplied) | Assumptions, alternatives, done criteria | Core governance |
| FND-002 | Simplicity first | Review/evaluation (Judgment; not supplied) | Current requirement and simpler alternative | Core governance |
| FND-003 | Surgical changes | Review (Judgment; not supplied) | Diff-to-goal trace | Core governance |
| FND-004 | Goal-driven execution | Check/report (Deterministic where automated; not supplied) | Previously defined acceptance check and actual output | Core governance |
| ENF-001 | No machine-named paths | Path scan/review (mixed; not supplied) | Derived paths and rejected machine forms | **None** |
| ENF-002 | Every claimed platform | CI/ruleset (Deterministic; not supplied) | Equivalent enforced platform coverage, cadence, and approved context binding | **None** |
| ENF-003 | English only | Scan/review (mixed; not supplied) | Diacritic scan plus semantic English review | **None** |
| ENF-004 | Conventional commits | Ruleset/changelog (Deterministic; not supplied) | Accepted-type enforcement and changelog configuration | **None** |
| ENF-005 | Failing test first | Test/review (mixed; not supplied) | Observed failure before implementation and later result | **None** |
| ENF-006 | Never weaken a gate | Protected configuration/review (mixed; not supplied) | No gate weakening, bypass, or removal; PR report if blocked | **None** |
| ENF-007 | Pull-request-only main | Branch/ruleset (Deterministic; not supplied) | Protected main and rejected direct push | **None** |
| ENF-008 | Tiered checks | Hooks/CI (mixed; not supplied) | Cheap, pre-push, same-command local/CI parity, fast-blocking, and heavy-reporting configuration; weekly WSL-toolchain coverage remains ENF-002 | Core governance |
| ENF-009 | Non-rotting allowlists | Check/configuration (mixed; not supplied) | Reasons and validity check output | Core governance |
| ENF-010 | Governance configuration authority | Protected config/ruleset (Deterministic; not supplied) | Authoritative config and coordinated context rename | **None** |
| ENF-011 | Instruction authority boundary | Host permissions/review (mixed; not supplied) | Permissions independent of instruction prose | **None** |
| C-001 | Resolved mandatory content and controls | Release/control inventory (Deterministic; not supplied) | Resolved content and control mapping | Core governance |
| C-002 | Immutable identifiable release | Release configuration (Deterministic; not supplied) | Immutable release identifier | Core governance |
| C-003 | Supported baseline policy | Governance record (Judgment; not supplied) | Version support policy | Core governance |
| C-004 | Drift detection | Check/report (Deterministic; not supplied) | Drift result and escalation | Core governance |
| C-005 | Compliance evidence | Audit record (Deterministic; not supplied) | Compliance and unavailable-control evidence | Core governance |
| C-006 | Controlled exceptions | Protected governance (mixed; not supplied) | Authorized scoped rationale, expiry, and report | **None** |
| C-007 | Coordinated rollout | Change governance (Judgment; not supplied) | Rollout coordination and updated dependents | Core governance |
| C-008 | Resolved release is compliance input | Evaluation configuration (Deterministic; not supplied) | Immutable identifier used by evaluation | Core governance |
| P-001 | YAGNI | Review (Judgment; not supplied) | Current requirement for each addition | Core governance |
| P-002 | KISS | Review (Judgment; not supplied) | Simpler alternative considered | Core governance |
| P-003 | DRY | Review (Judgment; not supplied) | Single knowledge source or rationale | Core governance |
| P-004 | WET | Review (Judgment; not supplied) | Duplication boundary and rationale | Core governance |
| P-005 | Rule of three | Review (Judgment; not supplied) | Occurrence count and extraction rationale | Core governance |
| P-006 | Chesterton's fence | Review/ADR (Judgment; not supplied) | History, ADR, or investigation | Core governance |
| P-007 | Boy Scout rule | Review (Judgment; not supplied) | Bounded improvement or deferral | Core governance |
| P-008 | Least astonishment | Review/evaluation (Judgment; not supplied) | Expected behavior example | Core governance |
| P-009 | Single responsibility | Review (Judgment; not supplied) | Reasons-to-change boundary | Core governance |
| P-010 | Composition over inheritance | Design review (Judgment; not supplied) | Relationship and alternative | Core governance |
| P-011 | Fail fast | Test/runtime (Deterministic; not supplied) | Observed boundary rejection | **None** |
| P-012 | Make illegal states unrepresentable | Type/design review (Judgment; not supplied) | Invariant and construction boundary | Core governance |
| P-013 | Parse don't validate | Test/runtime (Deterministic; not supplied) | Typed output and rejected input | **None** |
| P-014 | Principle of least privilege | Enforced host configuration (Deterministic; not supplied) | Granted versus needed scope | **None** |
| P-015 | Separation of concerns | Architecture check/review (mixed; not supplied) | Boundary and permitted interaction | Core governance |
| P-016 | Zero one or many | Test/review (mixed; not supplied) | Zero, one, and many cases | Core governance |
| P-017 | Premature optimisation (measure first) | Benchmark/profile review (Judgment; not supplied) | Observed bottleneck and result | Core governance |
| P-018 | Broken windows | Review/check (mixed; not supplied) | Bounded fix or explicit report | **None** |

All mandatory entries are applicable whenever their applicability statement matches; teams cannot opt out. A missing mechanism or evidence is unsupported/noncompliant and stops affected work. Exception records MUST name the authorized Core governance decision, scope, rationale, expiry, and report; this matrix does not invent approvers or release numbers.
