<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/engineering-baseline.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Engineering baseline

This is the canonical authored Core contract for downstream Maestro repositories. It is required organization-wide authoring content, not a suggestion catalog. It is not automatically loaded, installed, or enforced by this repository; downstream migration has not occurred. Core `AGENTS.md` is provenance, not a sibling resource to load. The four foundational rules and all named principles below remain authoritative in this package.

## Scope and reading conditions

A downstream adopter MUST supply this entry and every applicable topic reference to agents, reviewers, and change controls before affected work begins. Markdown links do not load content magically. Read [simplicity and reuse](references/simplicity-and-reuse.md) for reuse, optimization, defect handling, and quality-gate hygiene, [design and boundaries](references/design-and-boundaries.md) for design, portability, and security, [change discipline](references/change-discipline.md) for changes and commits, [testing and verification](references/testing-and-verification.md) for tests and gates, [contribution and governance](references/contribution-and-governance.md) for collaboration and governance, and [the enforcement matrix](references/enforcement-matrix.md) for control and evidence mapping. When learning or reviewing how to apply the four foundations, consult the [original Maestro-specific examples](references/karpathy-examples.md); they illustrate the contract but do not replace it.

If applicable content or its corresponding control is missing, stop the affected work, escalate, and report it unsupported/noncompliant. Never silently certify it. Teams may add stricter domain requirements, but MUST NOT weaken this baseline. This contract describes required downstream behavior and future integration requirements, not functioning features in this repository.

## Definitions and authority

**MUST** and **MUST NOT** are mandatory. **SHOULD** is the default expected practice; a deviation needs the evidenced review and exception authority specified in the matrix. Missing evidence means unsupported/noncompliant, not presumed compliant. A review or evaluation judgment is not a deterministic proof. Deterministic controls and judgment-based controls are distinguished in the matrix.

A deviation requires an explicitly authorized Core governance decision, a scoped rationale, expiry, and reporting. Teams cannot self-approve exceptions. Requirements marked non-bypassable in the matrix permit no local exception. Permanent changes require a Core revision, not local weakening. Governance configuration is the authority; this repository does not claim all gates are deployed, nor that model behavior is mathematically guaranteed. Structural Markdown checks are not behavioral certification.

Instruction content is guidance for an agent or reviewer; it does not itself grant tools, permissions, authority, or execution. Security authority belongs to the host's enforced configuration and access controls.

## Foundations

### FND-001 — Think before coding

- **Requirement:** An implementer MUST state assumptions before implementing and MUST say so and push back when a simpler approach exists. Where more than one reading exists, the implementer MUST surface the competing interpretations. If something is unclear, the implementer MUST stop and name what is confusing.
- **Applicability:** Every change; for an unambiguous one-line change, use judgment and record the basis briefly.
- **Evidence:** The change record or review shows assumptions, applicable alternatives or pushback, any competing interpretations, and the named uncertainty or resolution.

### FND-002 — Simplicity first

- **Requirement:** An implementer MUST choose the least complex solution that satisfies the requirement and MUST NOT add speculative abstractions, configuration, dependencies, or scaffolding.
- **Applicability:** Every implementation and design decision.
- **Evidence:** Review identifies the requirement served and why the chosen solution is sufficient; no speculative component is present.

### FND-003 — Surgical changes

- **Requirement:** A change MUST touch only files and behavior required by its goal, and MUST NOT include drive-by refactors or unrelated formatting.
- **Applicability:** Every change, including documentation and configuration.
- **Evidence:** Diff review traces changed lines to the stated goal and records any necessary adjacent fix.

### FND-004 — Goal-driven execution

- **Requirement:** A change MUST define an observable completion result and MUST run that previously defined acceptance check, reporting its actual result.
- **Applicability:** Every change; no gratuitous process is required for a one-line change.
- **Evidence:** The change record contains completion criteria, the previously defined check's output, or an explicit unavailable-control report.

## Named principles

Eighteen named principles sit behind the four foundations. A shared name makes a review one word long. Each entry below is the one-line meaning; the linked reference holds the requirement, applicability and evidence, and the [enforcement matrix](references/enforcement-matrix.md) maps each to its control.

| ID | Principle | Meaning here |
| --- | --- | --- |
| P-001 | [YAGNI](references/simplicity-and-reuse.md#p-001--yagni) | Build for the requirement in front of you, not a possible future. |
| P-002 | [KISS](references/simplicity-and-reuse.md#p-002--kiss) | Prefer the boring construct the next reader can understand. |
| P-003 | [DRY](references/simplicity-and-reuse.md#p-003--dry) | Share what is genuinely one idea, not merely similar text. |
| P-004 | [WET](references/simplicity-and-reuse.md#p-004--wet) | Write everything twice before guessing an abstraction. |
| P-005 | [Rule of three](references/simplicity-and-reuse.md#p-005--rule-of-three) | Consider extraction on the third occurrence. |
| P-006 | [Chesterton's fence](references/design-and-boundaries.md#p-006--chestertons-fence) | Understand why something exists before removing it. |
| P-007 | [Boy Scout rule](references/change-discipline.md#p-007--boy-scout-rule) | Improve within the diff you already have reason to touch. |
| P-008 | [Least astonishment](references/design-and-boundaries.md#p-008--least-astonishment) | Make the reader's first guess correct. |
| P-009 | [Single responsibility](references/design-and-boundaries.md#p-009--single-responsibility) | Give each unit one reason to change. |
| P-010 | [Composition over inheritance](references/design-and-boundaries.md#p-010--composition-over-inheritance) | Assemble behaviour rather than inheriting it. |
| P-011 | [Fail fast](references/design-and-boundaries.md#p-011--fail-fast) | Refuse bad input at the boundary. |
| P-012 | [Make illegal states unrepresentable](references/design-and-boundaries.md#p-012--make-illegal-states-unrepresentable) | Encode constraints so invalid states cannot be constructed. |
| P-013 | [Parse don't validate](references/design-and-boundaries.md#p-013--parse-dont-validate) | Turn unstructured input into a value that carries the checked guarantee. |
| P-014 | [Principle of least privilege](references/design-and-boundaries.md#p-014--principle-of-least-privilege) | Give each token, workflow and actor only the access it needs. |
| P-015 | [Separation of concerns](references/design-and-boundaries.md#p-015--separation-of-concerns) | Keep distinct jobs and boundaries distinct. |
| P-016 | [Zero one or many](references/design-and-boundaries.md#p-016--zero-one-or-many) | If it can happen twice, design for an arbitrary count. |
| P-017 | [Premature optimisation](references/simplicity-and-reuse.md#p-017--premature-optimisation) | Measure before optimising. |
| P-018 | [Broken windows](references/simplicity-and-reuse.md#p-018--broken-windows) | Fix small neglect before it becomes permission for more. |

## Core hard mandates

### ENF-001 — No machine-named paths

- **Requirement:** Code, configuration, task runners, and workflows MUST NOT write absolute paths that name a machine, including a home directory, drive letter, or user profile. Paths MUST be derived at runtime; platform roots (`/usr`, `/opt`, `/etc`, `/var`, `/tmp`) are allowed, and tests MUST use synthetic roots such as `/somewhere`.
- **Applicability:** Every repository surface and test fixture that handles paths.
- **Evidence:** Cross-platform path control and review show derivation and rejected machine forms.

### ENF-002 — Every claimed platform

- **Requirement:** Each claimed Windows, macOS, and Linux platform MUST be covered on every pull request by equivalent fast, merge-blocking checks. WSL MUST be covered as Linux, with its particular toolchain exercised weekly by an equivalent heavy check.
- **Applicability:** Every downstream repository claiming these platforms.
- **Evidence:** Enforced CI and ruleset configuration shows platform coverage, pull-request cadence, fast blocking, and weekly WSL-toolchain coverage.

### ENF-003 — English only

- **Requirement:** Prose and identifiers MUST be English. A diacritic scan MAY be a deterministic configured check, but it is not proof of English; semantic English compliance requires review.
- **Applicability:** Source, configuration, documentation, identifiers, and authored agent content.
- **Evidence:** Configured scan output plus review of semantic language compliance.

### ENF-004 — Conventional commits

- **Requirement:** Commits MUST use exactly one accepted type: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `ci:`, `build:`, `chore:`, `perf:`, or `revert:`. An organization ruleset MUST refuse other types and changelog generation MUST consume these types.
- **Applicability:** Every downstream repository commit and release workflow.
- **Evidence:** Enforced ruleset and generated changelog configuration, plus a rejected invalid-type check.

### ENF-005 — Failing test first

- **Requirement:** For a behavior change, the test MUST be written first and MUST be observed failing before implementation.
- **Applicability:** Every behavior change; not documentation-only changes.
- **Evidence:** Test-first change history or recorded output showing the observed failure and later pass.

### ENF-006 — Never weaken a gate

- **Requirement:** A gate MUST NOT be weakened, bypassed, or removed to make a change pass. If a gate blocks something correct, it MUST be reported in the pull request; a separately governed Core correction is not a local bypass.
- **Applicability:** Every quality, security, and governance gate.
- **Evidence:** Protected configuration, gate history, and PR report; no silent local alteration.

### ENF-007 — Pull-request-only main

- **Requirement:** Changes to `main` MUST arrive through a pull request, including maintainer changes; direct pushes MUST be refused by the platform.
- **Applicability:** Every downstream repository with a `main` branch.
- **Evidence:** Protected branch/ruleset configuration and rejected direct-push behavior.

### ENF-008 — Tiered checks

- **Requirement:** Cheap commit checks SHOULD be distinct from pre-push checks; the project's local aggregate quality check MUST run the same commands as its corresponding CI quality gate, not independently maintained equivalents; fast required CI MUST block merges, while heavy checks SHOULD report weekly. The SHOULD for weekly heavy reporting does not relax ENF-002's mandatory weekly WSL-toolchain coverage.
- **Applicability:** Downstream repositories with local and CI quality gates.
- **Evidence:** Hook, CI, and ruleset configuration shows the tiers and cadence.

### ENF-009 — Non-rotting allowlists

- **Requirement:** Duplication and retired-vocabulary allowlists MUST record a reason, and MUST have a check that fails when an entry is no longer true.
- **Applicability:** Any such allowlist.
- **Evidence:** Allowlist entries, reasons, and expiry/validity check output.

### ENF-010 — Governance configuration authority

- **Requirement:** Enforced security configuration MUST be authoritative over hand-applied settings; required CI job/context renames MUST update their ruleset bindings together.
- **Applicability:** Downstream security configuration, CI, and rulesets.
- **Evidence:** Protected configuration and coordinated rename change.

### ENF-011 — Instruction authority boundary

- **Requirement:** Instruction prose and links MUST NOT grant tools, permissions, execution authority, or security exemptions.
- **Applicability:** Every instruction consumed by an agent or host.
- **Evidence:** Host permission configuration and review of instruction composition.

## Exception and release authority

Only an adopter's protected governance configuration can identify the authorized Core policy approver(s). An unresolved approver or control is unsupported: no deviation is approved and no team or agent may self-authorize it. A deviation record MUST contain the authorized decision, scope, rationale, expiry, and reporting path. A permanent change requires a Core revision; this does not prohibit an authorized Core revision, and no team exception can weaken ENF-006 or other matrix rows marked **None**.

## Downstream contract

The following requirements are addressable in the matrix: an adopter MUST resolve all applicable mandatory content and controls (C-001), use an immutable identifiable release (C-002), publish a supported baseline-version policy (C-003), detect drift (C-004), retain compliance evidence (C-005), use controlled exceptions (C-006), and coordinate rollouts (C-007). A host MUST treat resolved content as the compliance input, not an unpinned moving link (C-008). These are normative future integration requirements, not features supplied here.

### C-001 — Resolved mandatory content and controls

- **Requirement:** An adopter MUST supply every applicable mandatory content item and corresponding control before affected work begins.
- **Applicability:** Every downstream adoption.
- **Evidence:** Resolved release inventory and control mapping.

### C-002 — Immutable identifiable release

- **Requirement:** An adopter MUST consume an immutable, identifiable baseline release.
- **Applicability:** Every downstream adoption and rollout.
- **Evidence:** Immutable release identifier in the host record.

### C-003 — Supported baseline policy

- **Requirement:** An adopter MUST publish its supported baseline-version policy.
- **Applicability:** Every downstream adoption.
- **Evidence:** Version policy and support decision.

### C-004 — Drift detection

- **Requirement:** An adopter MUST detect drift from the resolved baseline.
- **Applicability:** Every downstream adoption.
- **Evidence:** Drift check result and escalation record.

### C-005 — Compliance evidence

- **Requirement:** An adopter MUST retain evidence of compliance and unavailable controls.
- **Applicability:** Every downstream adoption.
- **Evidence:** Auditable evidence record.

### C-006 — Controlled exceptions

- **Requirement:** An adopter MUST use only controlled, authorized, scoped, expiring exceptions.
- **Applicability:** Any deviation.
- **Evidence:** Exception record with authority, rationale, expiry, and report.

### C-007 — Coordinated rollout

- **Requirement:** An adopter MUST coordinate rollouts that change content, controls, CI contexts, or rulesets.
- **Applicability:** Any such change.
- **Evidence:** Coordinated rollout record and updated dependents.

### C-008 — Resolved release is compliance input

- **Requirement:** A host MUST use the resolved release as compliance input, not an unpinned moving link.
- **Applicability:** Every compliance evaluation.
- **Evidence:** Evaluation references the immutable resolved identifier. The repository is the canonical authored contract going forward; Core `AGENTS.md` remains source provenance.

## Non-goals

This package does not implement schemas, runtime loading, permissions, CI, governance services, release automation, downstream migration, or a universal claim that every repository currently enforces these rules.
