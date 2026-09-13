<!-- Capture a decision, its reasons, costs and unresolved questions. -->
# 0002. The profile bindings this repository states

## Status

accepted

## Context

The Rust quality profile this repository adopts leaves a set of keys marked
`required_project_binding`. They are deliberately unset upstream, because each
one is a decision that depends on what the project is: which platforms it
claims, which test runner it uses, whether it publishes a library, whether it
has anything worth fuzzing. The profile is explicit that an unbound key is
unresolved and never a pass, so leaving them unstated would mean adopting the
profile without satisfying it.

They are recorded here, together, because they were decided together at
bootstrap, and each entry names the file that enforces it. An ADR is the
estate's mechanism for a decision that would otherwise be re-derived; a reader
who wants to know why the platform matrix has one leg should not have to
reconstruct it from a workflow file.

## Decision

| Profile key | Binding | Enforced in |
| --- | --- | --- |
| `testing.unit_and_integration_runner` | `cargo`, the profile default | `runner` in `just/lang.just` |
| `compatibility.msrv` | 1.85, the minimum for edition 2024 | `rust-version` in `Cargo.toml` |
| `compatibility.feature_matrix` | no optional features are declared | `Cargo.toml` has no `[features]` |
| `compatibility.platforms` | Linux only, for now | the matrix in `.github/workflows/fast-rust.yml` |
| `architecture.enforcement` | the decision layer performs no input or output | `tests/standards.rs` |
| `limits.module_physical_lines` | 250, counting whole files including tests | `tests/standards.rs` |
| `fuzzing.targets_and_budget` | unbound; no parser exists yet | the `fuzz` job is removed, not disabled |
| `auditable_build.dependency_metadata_retention_verification` | `cargo audit bin` against the released binary | recorded here; wired with the release workflow |
| `packaging.select_one` | `native_cargo_packaging`, the default | `publish = false` in `Cargo.toml` |
| `reports.services.coverage` | `artifact_only`, the default | the upload step in `.github/workflows/heavy-rust.yml` |
| `reports.tests` | unbound under the `cargo` runner | see Unresolved |
| `structural_rules.rules` | unbound; no custom rules are written | see Unresolved |

Three of these are judgements rather than defaults, and they are the ones worth
explaining.

**Platforms: Linux only.** The profile says to trim the matrix to the platforms
the repository claims. This daemon captures audio through PulseAudio and
delivers transcripts by running the `herdr` command, and neither has been
implemented or tested anywhere else. Claiming Windows and macOS in the matrix
would assert a portability nobody has demonstrated, and a green check for a
platform the code cannot serve is worse than an absent one. This diverges from
the sibling router, which genuinely runs on all three and says so.

**Fuzzing: removed rather than left failing.** The profile makes fuzzing
applicable to projects handling parser, protocol or untrusted input. This
repository will hand-roll HTTP parsing and read audio container headers, so it
will qualify. It does not qualify yet, because neither exists. A heavy job that
fails every week until then is noise that trains its reader to ignore heavy
results, so the job is removed and its return condition is written down instead.

**Application compatibility: not applicable.** `cargo-semver-checks` applies to
published libraries with a previous release. This crate sets `publish = false`
and ships a binary, so the `semver` job is removed per the profile's own
not-applicable route, which requires the determination to be documented rather
than silent. This is that documentation.

## Consequences

The profile's unbound keys become checkable claims, and the ones that are still
open are open in writing rather than by omission.

Two gates the profile requires are implemented here rather than inherited,
because the manifest defines them but ships no implementation:
`architecture_boundaries` and `module_size` both live in `tests/standards.rs`.
That means this repository owns their maintenance, and a future manifest that
starts shipping them will need reconciling with this copy.

The Linux-only matrix means a contributor on macOS cannot get a green fast tier
for their own platform. That is the honest state and not a defect to paper
over.

## Unresolved

`reports.tests` has no binding. The profile requires JUnit XML, and only the
nextest runner emits it natively; the `cargo` runner needs a verified adapter
that has not been selected. Until one is, test reporting is unresolved, which
under the profile is not a pass. The alternative is switching the runner to
nextest for its native JUnit, which is a live option rather than a decided one.

`structural_rules.rules` has no binding. The baseline lists `ast-grep` for
custom structural checks, and this repository has written none, so there is
nothing for it to run. The profile's applicability contract covers a project
with no custom structural checks; if that reading is wrong, the binding is
missing rather than satisfied.

Whether `similarity-rs` catches a duplicate in this codebase is unproven: the
profile lists it under `qualification_required`, and it is not installed on the
machine where this repository was bootstrapped. Duplication is therefore an
unqualified gate, not a passing one.

What reopens the platform decision: a capture implementation and a delivery
path that work on a second platform, with evidence from that platform. What
reopens the fuzzing decision: the first hand-rolled parser landing in `src`.
