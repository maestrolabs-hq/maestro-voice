<!-- Explain supported versions, private reporting and release verification. -->
# Security policy

## Supported versions

The latest release is supported. `main` is development, not a supported release.
At bootstrap there is no release yet.

## Private reporting

Report privately through [GitHub Security Advisories](https://github.com/maestrolabs-hq/maestro-voice/security/advisories/new),
not a public issue. Aim for acknowledgement within a week; one maintainer reads
reports and this is not a staffed response-time guarantee.

## What is in scope

Anything that reaches a machine without passing the gates: a supply-chain
compromise, a workflow running untrusted input with credentials, or captured
configuration that leaks a secret. Credentials never enter the repository;
keep tokens and keys in the approved secret store, not examples or fixtures.

## Automated controls

Full-history secret scanning catches secrets removed from the working tree.
Workflow lint and security audit check automation; dependency updates propose
reviewable changes. The selected language profile lists its own scanners.
These files are configuration, not evidence that a deployed control has run.

## Verifying release assets

Releases must ship `SHA256SUMS`, provenance and SBOMs in both SPDX and CycloneDX.
Download the release assets together, then verify their checksums:

```text
sha256sum --check SHA256SUMS
```

For each asset, replace `release-asset` with its downloaded filename:

```text
gh attestation verify release-asset --repo maestrolabs-hq/maestro-voice
```

Check the expected signer, source revision and subject, not merely command
success. `gh` still resolves the Sigstore trust root unless it is cached;
this is not an unconditional offline-verification promise. Bootstrap ships
no release assets; the language workflow must produce and verify this evidence
before publishing.
