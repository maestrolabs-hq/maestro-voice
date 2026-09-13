<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/security-boundaries/references/secure-development.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Secure development

Apply the [security entry](../security-boundaries.md) and [engineering design rules](../../engineering-baseline/references/design-and-boundaries.md). The [matrix](security-enforcement-matrix.md) distinguishes controls from judgment; [frameworks](frameworks.md) records alignment, not exhaustive framework coverage.

## SDL-001 — Threat models at boundaries
- **Requirement:** Each feature crossing a trust boundary MUST have a STRIDE threat model beside its architecture decision records (ADRs), identifying assets, actors, data flows, threats, mitigations, and residual risks. The model MUST be revisited when the boundary changes.
- **Applicability:** New or changed features crossing trust boundaries, including agent and tool integrations.
- **Evidence:** Revision-linked model and ADRs, boundary-change review, mitigation tests, and recorded residual-risk decisions.

## SDL-002 — Secure defaults
- **Requirement:** Systems MUST deny access by default, ship safe defaults, and disable debug behavior in production. Unsafe configurations MUST NOT activate implicitly.
- **Applicability:** Runtime defaults, deployment configuration, and feature activation.
- **Evidence:** Default-configuration review and tests demonstrating denied access, safe startup, and disabled production debugging.

## SDL-003 — Parse at the trust boundary
- **Requirement:** Input MUST be rejected early at the trust boundary under P-011 and parsed into a checked representation before downstream use under P-013; validation MUST cover syntax, semantic constraints, size/resource bounds, and authorized scope. Required bounds are project bindings.
- **Applicability:** External data, configuration, paths, requests, and tool arguments.
- **Evidence:** Parser tests show accepted checked values and rejected malformed, out-of-scope, and resource-exceeding input before side effects.

## SDL-004 — Injection and output contexts
- **Requirement:** Implementations MUST address applicable injection classes, including query, command, template, and browser injection, using structured or parameterized interfaces and context-specific output encoding. Untrusted input MUST NOT become executable syntax. Reviews MUST consider OWASP Top 10:2025 and the CWE Top 25 Most Dangerous Software Weaknesses, current edition.
- **Applicability:** Data entering interpreters, queries, templates, rendered output, or command execution.
- **Evidence:** Sink inventory, parameterization/encoding review, and adversarial tests for the applicable injection classes and output contexts.

## SDL-005 — Enforced authentication and authorization
- **Requirement:** Authentication and authorization MUST be decided server-side or host-side using enforced policy, least privilege, and resource/action checks under P-014 and SEC-004. Client claims and model output MUST NOT grant authority.
- **Applicability:** Protected operations, identities, sessions, resources, and service or agent credentials.
- **Evidence:** Enforced access configuration, requested-versus-granted scope review, and tests rejecting unauthenticated and unauthorized operations, including cross-resource access.

## SDL-006 — Secret lifecycle
- **Requirement:** Secrets MUST NOT enter the repository, including history, examples, or fixtures. Secrets MUST be supplied through the approved secret store, rotated under a bound project policy and after suspected compromise, and redacted from logs and diagnostics under SEC-001.
- **Applicability:** Credentials, keys, tokens, and secret-bearing configuration throughout development and operation.
- **Evidence:** Staged and full-history scan results, secret-store integration evidence, rotation policy and redacted rotation records, and log/diagnostic redaction tests; never secret values.

## SDL-007 — Vetted cryptography
- **Requirement:** Cryptography MUST use vetted libraries and MUST NOT use custom primitives or deprecated algorithms. Network protection MUST use current TLS where transport confidentiality or integrity is required. Algorithm, protocol, key-management, and library acceptance policies MUST be project-bound and maintained.
- **Applicability:** Cryptographic protection, secure transport, credential storage, and key handling.
- **Evidence:** Cryptographic inventory and policy review, library/configuration checks, key-management evidence, and tests rejecting disallowed transport or algorithms.

## SDL-008 — Safe errors and logging
- **Requirement:** Errors MUST fail safely without revealing sensitive details. Logs and diagnostics MUST omit or redact secrets and sensitive data, resist log injection, and retain useful security-event context only in authorized protected destinations under SEC-001.
- **Applicability:** Error responses, audit events, traces, crash reports, and diagnostic exports.
- **Evidence:** Failure-path and redaction tests, log-injection tests, and review of emitted fields, destination access, and actionable sanitized errors.

## SDL-009 — Classified minimal data
- **Requirement:** Projects MUST classify handled data and minimize collection, use, storage, and exposure under SEC-001. Purpose, access, retention, and deletion rules MUST be bound to the classification; unnecessary data MUST NOT be retained.
- **Applicability:** Data flows, persistence, telemetry, test data, and external integrations.
- **Evidence:** Data inventory with classifications and purpose, access/retention/deletion policy, minimization review, and observed deletion or retention checks.

## SDL-010 — Dependency hygiene
- **Requirement:** Dependencies MUST be pinned with ecosystem lockfiles where supported, licensed under the bound project policy, and audited using the [quality profiles](../../../../profiles/README.md) and [shared baseline](../../../../profiles/base/quality-baseline.yaml). Unresolved license or vulnerability decisions MUST NOT be treated as clearance; apply SCH-005 and SCH-011.
- **Applicability:** Direct, transitive, build, runtime, vendored, model, and tool dependencies.
- **Evidence:** Resolved dependency inventory, pins/lockfiles, license decisions, audit reports, and reviewed update history.

## SDL-011 — Agent and model boundaries
- **Requirement:** Agent and LLM designs MUST map applicable OWASP Top 10 for LLM Applications 2025 risks: prompt injection to SEC-002; excessive agency to P-014 and SEC-005; improper/insecure output handling to treating model output as data, never commands or authorization; sensitive information disclosure to SEC-001; and model/tool supply chains to SCH-005 and SCH-011. Designs MUST assess the remaining categories for applicability, bind mitigations, and keep host-side controls independent of model judgment under ENF-011.
- **Applicability:** Model inputs, retrieval, generated output, memory, model artifacts, and tool integrations.
- **Evidence:** Category applicability and threat review, model/tool provenance, adversarial prompt/output tests, disclosure tests, and host denial evidence for unauthorized actions; evaluations alone are not deterministic authorization.
