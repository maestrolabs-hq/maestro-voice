<!-- Capture a decision, its reasons, costs and unresolved questions. -->
# 0004. Hostile input instead of a fuzzer nobody can run

## Status

accepted

## Context

[ADR 0002](0002-the-profile-bindings-this-repository-states.md) removed the
heavy tier's `fuzz` job and wrote down the condition that brings it back: *the
first hand-rolled parser landing in `src`*. That condition is now met several
times over.

| Parser | Reads | From |
| --- | --- | --- |
| `src/intake/request.rs` | HTTP requests | a socket, from an untrusted peer |
| `src/http/reply.rs`, `src/http/body.rs` | HTTP replies | the router |
| `src/capture/wav.rs` | WAV containers | committed fixtures |
| `src/speech/audio.rs` | WAV containers | the synthesis service |
| `src/speech/json.rs` | JSON string fields | the router and Herdr |
| `src/config/file.rs` | settings | a file the operator wrote |

The first of those is a genuine trust boundary: anything that can reach that
socket can put words in the speaker.

So the decision was reopened, as ADR 0002 said it should be. The profile asks
for `cargo-fuzz`, which needs a nightly toolchain. Neither `cargo-fuzz` nor a
nightly toolchain is installed on any machine that has run these gates:
`cargo fuzz --version` reports `no such command`, and `rustup toolchain list`
shows stable, 1.85 and 1.88 only.

That is the whole difficulty. Adding a `fuzz/` directory and restoring the
heavy job would produce a gate nobody here has executed, which is the failure
this estate names most often and which ADR 0002 already records against
`similarity-rs`: *a gate whose installation is untested is a gate that can fail
for a reason unrelated to the code*. Writing unrunnable harnesses to satisfy a
profile key would be worse than the absence it replaced, because the absence at
least admitted what it was.

## Decision

Buy what fuzzing buys, by a means this repository can actually run, and say so
rather than claiming the profile key is satisfied.

`tests/hostile.rs` feeds every parser above input it was not expecting:
malformed, truncated, oversized, degenerate and randomly sprayed. The
generator is a seeded linear congruential sequence, eight lines of arithmetic
rather than a dependency, so a failure is reproducible from the seed printed
beside it instead of being a red run nobody can repeat. Every assertion is the
same one: **it returned**. Refusing input is correct behaviour; panicking takes
the daemon down, and reading without bound takes the microphone with it.

This is not a substitute for coverage-guided fuzzing and is not recorded as
one. What it is: the same property, checked against inputs chosen by hand plus
a repeatable spray, on every pull request, on the toolchain that is installed.

The profile key `fuzzing.targets_and_budget` therefore remains **unbound**, and
the heavy `fuzz` job stays absent. That is an honest unresolved, not a pass.

## Consequences

It found a real crash on its first run, which is the argument for it. A `fmt `
chunk that *declares* sixteen bytes and delivers four made `decode` index past
the end of the buffer and panic: reachable from any damaged or truncated reply,
and not reachable from any test written by someone imagining what a WAV looks
like. The fix bounds-checks the declared size against the bytes actually
present, and refuses a sample rate outside four kilohertz to a hundred and
ninety-two, because a declared rate of one turns a one-second reply into a
four-hour allocation. Both faults were proved by reverting the fix and watching
`tests/hostile.rs` fail with the original panic.

The intake tests close the client's write side after sending. Without that each
hostile request waited out the five-second exchange timeout and the file took
eighty-two seconds, which is not a fast-tier test. A client that sends nonsense
and hangs up is also the more realistic case. With it, the file runs in ten
milliseconds.

The cost is that this catches what someone thought to write down, plus noise.
Coverage-guided fuzzing explores paths nobody thought of, and that difference is
real. A parser fault that these tests miss is the outcome this decision accepts.

## Unresolved

Whether the deterministic spray reaches the branches that matter is not
measured. `cargo-llvm-cov` is in the heavy tier and could answer it against
these tests specifically; that has not been run.

What reopens this decision, replacing the condition in ADR 0002, which this
supersedes:

- `cargo-fuzz` and a nightly toolchain becoming available to the machines that
  run these gates, in which case the harnesses are written and the heavy job
  restored, with `tests/hostile.rs` kept as the fast-tier check.
- A parser fault reaching a branch or a release that `tests/hostile.rs` did not
  catch. That is evidence the cheaper answer was the wrong one.

A preference for having the profile key show green does not reopen it.
