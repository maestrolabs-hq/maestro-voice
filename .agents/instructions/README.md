<!-- Explain what the bootstrap places here and how each copy records its source. -->
# Instructions

Verbatim copies of the organisation's instruction entries, placed here at
bootstrap from `manifests/instructions/base/` in the `maestrolabs-hq/maestro-manifests`
repository:

- `engineering-baseline/engineering-baseline.md`, with its `references/` directory
- `security-boundaries/security-boundaries.md`, with its `references/` directory

Each entry keeps its own directory rather than being flattened. That is not a
style choice: the entries link to each other with paths such as
`../../engineering-baseline/references/enforcement-matrix.md`, which resolve
only when the source layout is preserved. Flattening the two entries into one
`references/` directory breaks those links and merges two reference sets that
happen not to collide today.

Each copy starts with a two-line provenance comment:

```text
<!-- Copied from maestrolabs-hq/maestro-manifests manifests/instructions/base/<entry>/<file> -->
<!-- Revision: <commit sha> -->
```

Edit the source in maestro-manifests, then refresh the copy. Do not edit a
copy here. Until the Bootstrapper agent exists, the copy is a manual step.
A missing copy means the contract is unresolved, not waived.
