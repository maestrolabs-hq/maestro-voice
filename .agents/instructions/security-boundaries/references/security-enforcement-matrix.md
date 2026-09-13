<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/security-boundaries/references/security-enforcement-matrix.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Security enforcement matrix

This is the single extended mapping from stable security IDs to downstream mechanisms and evidence. It supersedes nothing and carries forward every row of the [entry's SEC map](../security-boundaries.md#enforcement-and-evidence-map). SEC requirements and scope remain verbatim there; applicability for the extended families is in [secure development](secure-development.md), [security testing](security-testing.md), [supply chain](supply-chain.md), and [vulnerability response](vulnerability-response.md).

`Deterministic` means a repeatable machine control; `Judgment` means review/evaluation, not machine-proof; `mixed` requires both. `not supplied` means this package implements or installs no control. Missing applicable mechanisms, bindings, or evidence is unsupported/noncompliant and stops affected work. A documented not-applicable determination needs evidence, not silent omission.

Core governance means the engineering baseline's protected exception authority, scoped rationale, expiry, and reporting; no approver is named here. **None** permits no local exception. A row with Core governance MUST NOT weaken another rule marked **None**, including engineering rules. No gate may be weakened, bypassed, or removed to make a change pass.

Referenced engineering IDs retain their own requirements and rows in the [engineering matrix](../../engineering-baseline/references/enforcement-matrix.md), not duplicate security definitions: FND-001; P-011, P-013, P-014; ENF-005, ENF-006, ENF-007, ENF-008, ENF-009, ENF-010, ENF-011; C-001, C-005, C-006. C-005 supplies the engineering evidence-retention obligation; ENF-011 preserves instruction authority boundaries throughout this contract.

| ID | Name | Mechanism (type) | Required evidence | Exception policy |
| --- | --- | --- | --- | --- |
| SEC-001 | Minimize sensitive data | Minimization, storage, redaction, retention review (Judgment; not supplied) | Needed-data rationale and redacted output/storage or retention evidence | Core governance |
| SEC-002 | Treat input as data | Provenance and instruction/data separation review (Judgment; not supplied) | Source provenance, conflict handling, and ignored authority claims | **None** |
| SEC-003 | Validate boundaries | Path, URL, revision, and argument boundary validation (Deterministic; not supplied) | Validation result showing authorized-root/scheme checks and rejected escapes | **None** |
| SEC-004 | Use real authority | Enforced authority and least-privilege configuration (Deterministic; not supplied) | Host policy/access-control evidence naming the authorized decision source | **None** |
| SEC-005 | Scope sensitive approvals | Scoped approval record and sensitivity review (mixed; not supplied) | Separate approval with scope, target, expiry, and evidence; no self-waiver | **None** |
| SEC-006 | Inspect code safely | Isolated least-privilege execution conditions (Deterministic; not supplied) | Isolation, privilege, network, secret, and resource-bound evidence | **None** |
| SEC-007 | Stop and escalate incidents | Stop boundary and redacted escalation review (mixed; not supplied) | Stop/containment authorization and redacted incident record | **None** |
| SEC-008 | Keep truthful evidence | Truthful provenance and gate status review (Judgment; not supplied) | Record of observed, supplied, executed, blocked, and unchecked evidence | **None** |
| SEC-009 | Preserve safe progress | Bounded continuation review (Judgment; not supplied) | Authorized read-only scope and explicit partial-result report | Core governance |
| SDL-001 | Threat models at boundaries | STRIDE/ADR review and mitigation tests (mixed; not supplied) | Revision-linked model/ADRs, changed-boundary review, mitigation results, residual risks | Core governance |
| SDL-002 | Secure defaults | Configuration review and negative tests (mixed; not supplied) | Denied-default access, safe startup, and disabled production-debug evidence | Core governance |
| SDL-003 | Parse at the trust boundary | Parser and scope/resource rejection tests (Deterministic; not supplied) | Accepted checked values and rejected malformed, unauthorized, and over-bound inputs before effects | **None** |
| SDL-004 | Injection and output contexts | Sink review and adversarial tests (mixed; not supplied) | Injection-class inventory, structured interfaces/encoding, and context-specific rejection results | Core governance |
| SDL-005 | Enforced authentication and authorization | Host/server access enforcement and tests (Deterministic; not supplied) | Access configuration, needed/granted scopes, rejected unauthenticated and cross-resource actions | **None** |
| SDL-006 | Secret lifecycle | Scans, store/rotation review, redaction tests (mixed; not supplied) | Staged/history scans, approved-store integration, rotation policy/records, log/diagnostic tests without secrets | **None** |
| SDL-007 | Vetted cryptography | Inventory/policy review and configuration tests (mixed; not supplied) | Accepted libraries/protocols/algorithms, key-management evidence, rejected deprecated options | Core governance |
| SDL-008 | Safe errors and logging | Failure/redaction tests and destination review (mixed; not supplied) | Sanitized actionable failures, log-injection results, emitted-field and access review | Core governance |
| SDL-009 | Classified minimal data | Inventory/policy review and lifecycle checks (mixed; not supplied) | Classification/purpose, minimized data, access/retention/deletion rules and observed results | Core governance |
| SDL-010 | Dependency hygiene | Pins, audits, and license review (mixed; not supplied) | Resolved inventory, lockfiles, license decisions, vulnerability reports, reviewed updates | Core governance |
| SDL-011 | Agent and model boundaries | Threat/evaluation review and host denial tests (mixed; not supplied) | LLM category applicability, provenance, adversarial/disclosure tests, unauthorized-action denials | **None** |
| SST-001 | Static application security testing | Fast SAST gate and optional-analysis binding review (mixed; not supplied) | Rules/inputs, fast blocking reports and negative checks; enabled CodeQL support, authority, queries, results | **None** |
| SST-002 | Software composition analysis | Profile SCA gates and finding review (mixed; not supplied) | Resolved inputs, reports, vulnerability decisions, failed/violating scan rejection | **None** |
| SST-003 | Secret scanning | Staged and full-history gates (Deterministic; not supplied) | Scope/history configuration, redacted reports, synthetic blocking tests in both scopes | **None** |
| SST-004 | Infrastructure and configuration scanning | Misconfiguration gates and coverage review (mixed; not supplied) | Input/rule inventory, scanner reports, tier/gate results, applicability record | Core governance |
| SST-005 | Container image scanning | Image gates and disposition review (mixed; not supplied) | Image digest, vulnerability/configuration reports, dispositions and enforced gate results | Core governance |
| SST-006 | Workflow security | Workflow lint/audit and SHA checks (mixed; not supplied) | Reports, immutable-pin checks, trust/secret/token review, rejected unsafe examples | **None** |
| SST-007 | Fuzzing and property testing | Profile test execution and scope review (mixed; not supplied) | Targets/properties, command/tier mapping, budgets/platforms, results/regressions or applicability evidence | Core governance |
| SST-008 | Dynamic application security testing | Authorized DAST and findings review (mixed; not supplied) | Target authority, bound conditions/tier/cadence, sanitized revision-linked results and gate disposition | Core governance |
| SST-009 | Pull-request dependency review | Dependency review and required gates (mixed; not supplied) | Dependency diff, vulnerability/license/provenance/privilege decisions and gate results | Core governance |
| SST-010 | Findings reporting | SARIF production/adapter validation (Deterministic; not supplied) | Valid reports, revision mapping, producer/adapter verification and original gate results | Core governance |
| SST-011 | Expiring suppressions | In-place review and expiry/validity checks (mixed; not supplied) | Scoped reasons/expiry, authorized exceptions as needed, rejected stale/unjustified entries | Core governance |
| SST-012 | Fail closed on gate failure | Protected gates and negative checks (Deterministic; not supplied) | Local/CI parity, violations/errors/missing-report failures, blocked-change reports without bypass | **None** |
| SCH-001 | Build provenance | Provenance verification and target/control review (mixed; not supplied) | Released-subject provenance, builder/binding inventory, target-level mapping, source/subject verification | Core governance |
| SCH-002 | Signing and verification | Identity/subject/trust verification (Deterministic; not supplied) | Signatures or artifact attestations, trust bindings, successful and rejected identity/subject checks | Core governance |
| SCH-003 | Complete software bills of materials | Dual-format production and completeness review (mixed; not supplied) | SPDX and CycloneDX tied to artifacts/source, supported inputs, verified dependency/license metadata | Core governance |
| SCH-004 | Release checksums | Digest production and comparison (Deterministic; not supplied) | Published manifest, exact subjects, successful comparison and altered-artifact rejection | Core governance |
| SCH-005 | Immutable dependency inputs | Resolution/integrity and full-SHA checks (Deterministic; not supplied) | Lockfiles or bound equivalent, immutable model/tool IDs, rejected mutable actions and unresolved pins | **None** |
| SCH-006 | Least-privilege build identities | Enforced CI identity scope and denial tests (Deterministic; not supplied) | Needed/granted permission/lifetime bindings, untrusted-code and publishing denials | **None** |
| SCH-007 | Reviewed dependency updates | Update configuration and pull-request review (mixed; not supplied) | Auto-merge disabled, reviewed proposals, compatibility/security/license gates, unsupported-source process | Core governance |
| SCH-008 | Protected source and release identity | Protected source/tags and Source Track review (mixed; not supplied) | Rejected tag mutation/direct pushes, pull-request history, Source Track control mapping | **None** |
| SCH-009 | Embedded dependency metadata | Final-binary inspection and support review (mixed; not supplied) | Retained metadata in shipped binaries and profile/build mapping or unsupported-toolchain evidence | Core governance |
| SCH-010 | Reproducibility feasibility | Feasibility review and rebuild comparison (mixed; not supplied) | Feasibility/constraint decision, inputs, comparison results for claims, governed deviations where needed | Core governance |
| SCH-011 | Third-party and vendored policy | Component/policy and patch review (Judgment; not supplied) | Origin/integrity, licenses, patch history, audits, monitoring, update/removal decisions | Core governance |
| SCH-012 | Informational Scorecard | Activation and report-use review (Judgment; not supplied) | Disabled state or authorized selection, supported reports marked informational, no inferred safety floor | Core governance |
| SCH-013 | Published artifact verification | Instruction review and exercised verification (mixed; not supplied) | Published checksum/provenance/identity/SBOM instructions, trust prerequisites, release results or honest pre-release status | Core governance |
| VR-001 | Private reporting | Published channel and routing/access check (mixed; not supplied) | Usable private channel, verified routing/access, sanitized intake check | **None** |
| VR-002 | Honest acknowledgement | Commitment and timestamp review (Judgment; not supplied) | Published bound window/coverage, receipt/acknowledgement records, misses and escalation | Core governance |
| VR-003 | Severity and weakness triage | Contextual severity/CWE review (Judgment; not supplied) | Scale binding, impact/exploitability rationale, affected versions, uncertainty, disposition | Core governance |
| VR-004 | Gated security fixes | Protected change/release path (mixed; not supplied) | Reviewed pull requests, unchanged gates, test/release results, protected-branch evidence | **None** |
| VR-005 | Regression test first | Observed test history and result review (mixed; not supplied) | Restricted failure before implementation and later pass tied to vulnerable/fixed revisions | **None** |
| VR-006 | Security advisories | Advisory/version/credit review (Judgment; not supplied) | Published impact, affected/fixed versions, remediation and restricted consent/withholding evidence | Core governance |
| VR-007 | Coordinated disclosure | Restricted coordination and authorization review (Judgment; not supplied) | Bound timing/participants/authority, embargo decisions, scoped transmission, actual notifications | Core governance |
| VR-008 | Root cause and weakness sweep | Root-cause/sweep review (Judgment; not supplied) | Sweep scope/results, corrective actions, regressions, threat-model update or no-change rationale | Core governance |
| VR-009 | Dependency vulnerability response | Response policy and disposition review (mixed; not supplied) | Policy, affected inventory, response decisions/timing, gate results, permitted authorized exceptions | Core governance |
| VR-010 | Retained truthful response evidence | Protected records, retention checks, and truthfulness review (mixed; not supplied) | Revision-linked redacted records, access/retention results, incident history, blocked/unchecked disclosures | **None** |

Framework references and their limits are in [frameworks](frameworks.md). Artifact signing evidence is not a claim of framework assessment or certification. No mechanism, approver, target level, numeric threshold, or response window is supplied by this matrix.
