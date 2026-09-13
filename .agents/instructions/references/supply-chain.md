<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/security-boundaries/references/supply-chain.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Supply chain

Apply the [security entry](../security-boundaries.md), [quality profiles](../../../../profiles/README.md), and [shared baseline](../../../../profiles/base/quality-baseline.yaml). Profiles own artifact formats, producer choices, tiers, and required bindings; no signing, publishing, or execution authority is implied. The [matrix](security-enforcement-matrix.md) maps controls; [frameworks](frameworks.md) records SLSA alignment without claiming a level.

## SCH-001 — Build provenance
- **Requirement:** Released artifacts MUST carry verified SLSA specification 1.2 Build Track provenance under the profile reporting contract. The target level, build requirements, trusted builder, source revision, subject, signer, issuer, production commands, and verification policy MUST be project bindings; provenance alone MUST NOT be represented as achievement of a SLSA level.
- **Applicability:** Released binaries, packages, images, and other software artifacts.
- **Evidence:** Artifact-bound provenance, builder/control inventory, target-level requirement mapping, and verification results against the expected source revision and subject.

## SCH-002 — Signing and verification
- **Requirement:** Release artifacts and their provenance MUST be cryptographically bound and verified through the profile choice of Sigstore or GitHub artifact attestations. Verification MUST enforce the bound signer, issuer, source revision, subject, and trust policy, not merely successful command execution.
- **Applicability:** Released artifacts and associated provenance.
- **Evidence:** Signatures or artifact attestations, authorized identity/trust bindings, verification results, and rejected mismatched or untrusted subjects and identities.

## SCH-003 — Complete software bills of materials
- **Requirement:** Releases MUST include SBOMs in both SPDX and CycloneDX using the profiles' producer choice. Completeness, dependency versions, license metadata, supported inputs, and artifact association MUST be verified under project-bound policy.
- **Applicability:** Released artifacts with software dependencies.
- **Evidence:** Both SBOMs tied to released subjects and source, supported-input and completeness checks, and version/license metadata verification.

## SCH-004 — Release checksums
- **Requirement:** Released artifacts MUST have published checksums and verified digest-to-artifact mappings under a project-bound algorithm and production policy. Checksums MUST NOT substitute for provenance or authenticated verification.
- **Applicability:** Downloadable release artifacts.
- **Evidence:** Published checksum manifest, exact release subjects, and successful checks plus rejected altered-artifact checks.

## SCH-005 — Immutable dependency inputs
- **Requirement:** Dependencies MUST use pinned resolutions and committed ecosystem lockfiles where supported. External CI actions and reusable workflows MUST be pinned to full immutable commit SHAs, not mutable tags. Model and tool artifacts MUST have immutable identifiers and integrity verification; unsupported lockfile ecosystems MUST bind an equivalent resolved inventory.
- **Applicability:** Direct, transitive, build, runtime, model, tool, and CI dependencies.
- **Evidence:** Lockfiles or equivalent inventory, source/artifact integrity checks, and automated rejection of mutable external action references or unresolved dependency pins.

## SCH-006 — Least-privilege build identities
- **Requirement:** CI tokens MUST have only the permissions and lifetime needed under P-014. Untrusted pull-request code MUST NOT receive secrets or privileged publishing tokens under SEC-006; release permissions MUST be separately scoped to authorized work.
- **Applicability:** Build, test, dependency-update, provenance, and release workflows.
- **Evidence:** Enforced token/identity configuration, granted-versus-needed scope review, and denial evidence at untrusted-code and publishing boundaries.

## SCH-007 — Reviewed dependency updates
- **Requirement:** Projects MUST use automated dependency-update proposals with human or authorized review through the normal gated pull-request path. Updates MUST NOT be auto-merged; required compatibility, security, and license checks MUST pass before merge.
- **Applicability:** Supported automated dependency sources; unsupported sources require a bound reviewable update process.
- **Evidence:** Update configuration disabling auto-merge, reviewed pull requests, gate results, and documented handling of unsupported sources.

## SCH-008 — Protected source and release identity
- **Requirement:** Version tags MUST be immutable and protected against replacement or deletion. Changes to protected `main` MUST arrive only through pull requests, with direct pushes refused under ENF-007. Projects MUST bind and review applicable SLSA Source Track requirements without claiming an unverified level.
- **Applicability:** Release tags and repositories with a `main` branch; applicable source-integrity controls for every release.
- **Evidence:** Protected tag/branch configuration, rejected tag mutation and direct pushes, pull-request history, and Source Track requirement/control mapping.

## SCH-009 — Embedded dependency metadata
- **Requirement:** Shipped binaries MUST retain embedded dependency metadata where the language/toolchain supports it, using applicable profile controls. Embedding MUST NOT replace SBOMs, audits, licensing, checksums, or provenance.
- **Applicability:** Shipped binaries with supported embedding, including the Rust profile's auditable-binary requirement.
- **Evidence:** Inspection of final shipped binaries proving dependency metadata retention, profile-to-build mapping, or a documented unsupported-toolchain determination.

## SCH-010 — Reproducibility feasibility
- **Requirement:** Projects SHOULD make release builds reproducible where feasible and MUST record a feasibility assessment. A claim of reproducibility MUST be supported by independent rebuild comparison under bound inputs and comparison criteria, not inferred from a successful build.
- **Applicability:** Release build design and any reproducibility claim.
- **Evidence:** Feasibility decision, controlled input/environment inventory, rebuild comparison results where claimed, or recorded constraints and governed deviation where required by the matrix.

## SCH-011 — Third-party and vendored policy
- **Requirement:** Projects MUST bind policy for third-party and vendored code, models, and tools covering origin, integrity, licensing, review, vulnerability monitoring, local patches, updates, and removal. Vendoring MUST NOT exempt components from security or license checks.
- **Applicability:** Imported, copied, generated-from-third-party, vendored, model, and tool components.
- **Evidence:** Component inventory and policy, origin/integrity and license records, patch history, review/audit results, and update or removal decisions.

## SCH-012 — Informational Scorecard
- **Requirement:** OpenSSF Scorecard MUST remain optional, disabled until explicitly selected, authorized, and configured, and informational/report-only as the profiles define. Reports MUST NOT imply a score floor or merge safety; any blocking promotion requires separately defined criteria, not an inferred gate here.
- **Applicability:** Projects considering or using Scorecard.
- **Evidence:** Disabled state or selection/authorization/configuration and supported-check reports labeled informational, with no inferred blocking threshold or safety claim.

## SCH-013 — Published artifact verification
- **Requirement:** Projects MUST publish usable artifact verification instructions in `SECURITY.md`, following the [repository scaffold](../../../../bootstrap/base/repository/files/SECURITY.md#verifying-release-assets). Instructions MUST cover checksums, provenance/signature identity and subject checks, both SBOMs, required tools, and trust/network prerequisites without claiming unpublished evidence exists.
- **Applicability:** Repositories publishing release artifacts; pre-release repositories must state the absence of release evidence honestly.
- **Evidence:** Published instructions exercised against actual release assets, verification output tied to the release, or explicit pre-release status with unverified steps identified.
