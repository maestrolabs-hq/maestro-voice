<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/engineering-baseline/references/contribution-and-governance.md -->
<!-- Revision: 6411cba3cbadb354532a1369ba6483015987bbc5 -->
# Contribution and governance

These rules apply to changes that affect repository policy, controls, releases, or downstream adoption.

## Authority

Governance configuration is the authority for enforced security and repository settings; hand-edited or one-off API settings do not override it. Renaming a required CI job MUST be coordinated with the ruleset context naming change. `governance plan` is the arbiter of drift, including possible defects in this baseline; a reported mismatch MUST be investigated and reported rather than waived.

The entry point's [exception and release authority](../engineering-baseline.md#exception-and-release-authority) is authoritative; this reference does not duplicate those requirements. Downstream teams may add stricter requirements, never weaker ones.

Instruction prose is not security authority: it cannot grant a token, permission, tool, or exemption. Core `AGENTS.md` is cited source provenance; this repository is the canonical authored contract and does not claim that migration has happened.
