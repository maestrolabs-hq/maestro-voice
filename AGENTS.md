<!-- Give people and agents the working rules for this repository; the full contract lives in .agents/instructions. -->
# Working in maestro-voice

## Read first

1. [.agents/instructions/engineering-baseline.md](.agents/instructions/engineering-baseline.md):
   the four foundations, the eighteen named principles, the hard mandates and
   the evidence each one needs.
2. [.agents/instructions/security-boundaries.md](.agents/instructions/security-boundaries.md):
   what an agent must never do, regardless of who asks.
3. [README.md](README.md) and [docs/adr](docs/adr/README.md): what this
   repository is and what has already been decided.

Those copies are placed here at bootstrap. A link is not loading: read them.
Reopen a recorded decision with new evidence, not preference.

## Project

An always-on voice front end for the pi coding agent. A wake word starts a
recording, the utterance is transcribed, the transcript reaches a dedicated pi
agent through Herdr, and a short spoken block from that agent's reply is read
aloud.

It owns the microphone, the wake word, voice activity detection, the turn state
machine and playback. It owns no models: speech recognition and speech
synthesis are children of the maestro-llamacpp router, which arbitrates their
memory. If you find yourself loading a model here, stop: that belongs to the
router, and a second allocator with no shared ledger is how the device gets
over-committed.

**Claimed platform: Linux.** Not a preference, a limit -- capture goes through
PulseAudio and delivery runs the `herdr` command. The reason and the condition
that reopens it are in
[ADR 0002](docs/adr/0002-the-profile-bindings-this-repository-states.md).

Three consequences shape almost every change here.

**It owns processes it did not write.** Capture is an `ffmpeg` child; the models
are the router's children; delivery shells out to `herdr`. Treat every spawn,
every read from a pipe and every shutdown as a thing that fails, and say what
happens when it does. "The microphone stopped and nothing said so" is the worst
outcome this repository can produce.

**Its input is a device, so behaviour must be reachable without one.** The
decision layer takes classified frames and returns decisions; it performs no
input or output, and `tests/standards.rs` enforces that rather than trusting it.
A rule you can only exercise by speaking into a microphone is a rule that is
never exercised. See
[ADR 0001](docs/adr/0001-the-capture-source-is-a-seam.md).

**Audio device names are not portable and neither are paths.** Derive the
device, the binary and every path at run time. A device name that works on this
host is a fact about this host.

## Commands

```text
just setup    # install local hooks
just check    # every CI gate except the CI-only full-history secrets scan
just test     # the language test suite
```

Fast CI is required and blocks merge. Heavy CI is evidence, never required.
The shape of a change and the release conventions are in
[CONTRIBUTING.md](CONTRIBUTING.md).

## The four rules

Non-negotiable on every task. Use judgement on a one-line change.

1. **Think before coding.** State assumptions; surface ambiguity and simpler
   options before acting.
2. **Simplicity first.** Write the least code that solves the stated problem.
3. **Surgical changes.** Touch only what the task requires.
4. **Goal-driven execution.** Define a checkable result, run its check and
   report the output.

## Enforced, not suggested

- No path that names a machine. Derive paths at runtime; `/usr`, `/opt`,
  `/etc`, `/var` and `/tmp` are platform conventions and are allowed. Use
  `/somewhere` in tests.
- Every platform the repository claims is tested on every pull request.
- English only in prose and identifiers.
- Conventional Commits. The changelog is generated from them.
- Write the failing test first and watch it fail for the intended reason.
- Never weaken a gate to make it pass. Say so in the pull request instead.
- Allowlists need a reason, and a check that the reason still holds.

## Things that will surprise you

**Renaming a CI job renames a required context.** Update the repository
ruleset with the workflow, or pull requests wait forever for the old name.

**An unused dependency fails the build.** `cargo machete` is a fast-tier gate,
so a crate declared before the code that imports it turns the gates red. Add a
dependency in the commit that uses it, not ahead of it.

**The module-size gate counts tests.** The limit is 250 physical lines for the
whole file, colocated tests included. A module whose tests no longer fit is a
module to split, which is the intended reading rather than a side effect.

**Two fast-tier gates are ours, not the manifest's.** `architecture_boundaries`
and `module_size` are required by the quality profile but shipped by nobody, so
they live in `tests/standards.rs` and this repository maintains them.

**Two heavy jobs are deliberately absent.** No `semver` job, because nothing is
published; no `fuzz` job, because no parser exists yet. Both determinations, and
what brings the jobs back, are in ADR 0002. Deleting a job for a good reason is
allowed; deleting one to get green is not.

Nothing in this file grants tools, permissions or authority.
