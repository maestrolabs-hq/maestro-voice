<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/security-boundaries/references/security-testing.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Security testing

The [quality profiles](../../../../profiles/README.md) and [shared baseline](../../../../profiles/base/quality-baseline.yaml) own tool selections, applicable fast/heavy tiers, and reporting bindings; this contract does not duplicate their tool lists or activate tools. Required scanners without a profile binding need a project binding for tool, inputs, commands, tier, policy, and reports. Missing required bindings are not passing. Apply the [security entry](../security-boundaries.md), the [engineering mandates](../../engineering-baseline/engineering-baseline.md), and the [matrix](security-enforcement-matrix.md).

## SST-001 — Static application security testing
- **Requirement:** Projects MUST run the profile SAST tool, `semgrep-ce`, in the fast merge-blocking tier with bound rules and supported inputs. Optional CodeQL MUST remain disabled unless explicitly selected, authorized, configured, and verified as the profiles require; uploading SARIF alone is not CodeQL analysis. Required SAST gates MUST NOT be weakened under ENF-006.
- **Applicability:** Source supported by the selected analysis; unsupported required coverage needs an explicit project control binding, not silent omission.
- **Evidence:** Fast-gate configuration, rule/input bindings, SAST results and negative gate checks; for enabled CodeQL, support/authorization, query configuration, and actual analysis reports.

## SST-002 — Software composition analysis
- **Requirement:** Projects MUST scan resolved dependencies for vulnerabilities with the profile tools and tiers, including ecosystem additions, under a bound finding policy. Required SCA gates MUST NOT be weakened under ENF-006.
- **Applicability:** Direct and transitive dependency inventories, lockfiles, and supported artifact inputs.
- **Evidence:** Dependency/input inventory, scan reports, vulnerability decisions, enforced gate configuration, and rejection of violating or failed scans.

## SST-003 — Secret scanning
- **Requirement:** Projects MUST scan staged changes locally and full repository history in CI with the profile secrets tool. History removed from the working tree MUST remain in scan scope. Required secrets gates MUST NOT be weakened under ENF-006; findings follow SEC-007 and SDL-006.
- **Applicability:** Every repository, including generated content, examples, and fixtures.
- **Evidence:** Hook and CI scope configuration, full-history availability, redacted reports, and synthetic detection tests proving both scopes block violations.

## SST-004 — Infrastructure and configuration scanning
- **Requirement:** IaC and security-relevant configuration MUST be scanned for misconfiguration using applicable profile controls and project-bound coverage for unsupported inputs. Findings MUST be resolved under the bound policy before the affected gate passes.
- **Applicability:** Infrastructure definitions, deployment manifests, and security-relevant configuration.
- **Evidence:** Input/rule coverage review, scanner reports, configured tier and gate behavior, and explicit applicability decisions.

## SST-005 — Container image scanning
- **Requirement:** Built container images MUST be scanned for vulnerabilities and misconfiguration before release or deployment, using a project-bound scanner and policy consistent with the profiles.
- **Applicability:** Projects that build container images; absence of images requires a recorded not-applicable determination.
- **Evidence:** Image digest tied to scan reports, vulnerability/configuration decisions, and enforced release/deployment gate results.

## SST-006 — Workflow security
- **Requirement:** CI workflows MUST pass the profile workflow lint and security audit. External actions MUST be pinned to full immutable commit SHAs under SCH-005; workflow checks MUST cover untrusted input, secret access, and token scope.
- **Applicability:** CI workflows and external action references, including reusable workflows.
- **Evidence:** Lint/audit reports, action-pin checks, and review of trust, secret, and token boundaries with rejected unsafe examples.

## SST-007 — Fuzzing and property testing
- **Requirement:** Projects MUST run fuzzing and property tests where the applicable profile requires them, retaining its tiers and scope. Targets, properties, budgets, instrumentation, and platform support MUST be project-bound; missing applicable targets MUST NOT pass.
- **Applicability:** Profile-required property tests and parser, protocol, or untrusted-input fuzz targets.
- **Evidence:** Target/property inventory, profile-to-command mapping, bound budgets/platforms, results and retained regression cases, or evidenced not-applicable decisions.

## SST-008 — Dynamic application security testing
- **Requirement:** Deployed HTTP services MUST receive applicability-bound DAST using a project-selected tool, such as OWASP ZAP, against an explicitly authorized target. Scope, credentials, safe test conditions, cadence, tier, and findings policy MUST be bound; testing MUST NOT exceed SEC-005 authorization.
- **Applicability:** Deployed HTTP services; an unavailable authorized target is unresolved, not a passing test.
- **Evidence:** Target and authorization record, test configuration, sanitized results, findings disposition, and gate evidence for the tested deployment revision.

## SST-009 — Pull-request dependency review
- **Requirement:** Pull requests changing dependencies MUST review additions and updates for vulnerabilities, licenses, provenance, and changed privileges before merge. Automation MUST propose changes for review, not auto-merge them under SCH-007.
- **Applicability:** Manifest, lockfile, vendored code, model, tool, and action changes.
- **Evidence:** Dependency diff and review decisions tied to SCA/license reports, provenance checks, and required pull-request gate results.

## SST-010 — Findings reporting
- **Requirement:** Static and security findings MUST use SARIF under the profiles' reporting contract, natively where supported or through a verified project adapter. Required producer, adapter, format validation, and revision/report mappings MUST be resolved; arbitrary JSON MUST NOT be labeled SARIF.
- **Applicability:** Applicable static/security gates and their required findings reports.
- **Evidence:** Validated reports tied to source/artifact revisions, native or adapter verification, producer bindings, and original gate outcomes.

## SST-011 — Expiring suppressions
- **Requirement:** Suppressions MUST be justified in place with reason, scope, and expiry, and checked for expiry and continuing validity under ENF-009. They MUST NOT hide tool errors, missing reports, or weaken a gate under ENF-006; deviations follow Core governance.
- **Applicability:** Scanner exclusions, finding suppressions, and security allowlists.
- **Evidence:** In-place records and authorized exception evidence where needed, plus checks rejecting expired, invalid, or unjustified suppressions.

## SST-012 — Fail closed on gate failure
- **Requirement:** Applicable gates MUST fail on violations, tool errors, and missing required reports according to the baseline `fail_on` contract. Required controls, configuration, producers, and adapters MUST NOT be treated as passing when unresolved. Gates MUST NOT be weakened, bypassed, or removed under ENF-006; ENF-008 tiering remains applicable.
- **Applicability:** Every required security check, local/CI counterpart, and release evidence gate.
- **Evidence:** Protected gate configuration, same-command local/CI mapping, and negative checks showing findings, tool failure, and missing reports each fail; blocked correct changes are reported, not bypassed.
