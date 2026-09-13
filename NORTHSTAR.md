<!-- State the north star this repository steers by: four axes, one measured KPI each. -->
# North star

> Automate the guardrails to deliver faster, with higher quality, and more securely.

Speed, quality and security are not a trade-off triangle. Automation is what
lets one repository have all three, and scalability is what proves they still
hold as the work grows. This file is how maestro-voice steers by that.

## The point

One person can direct a coding agent without being at the keyboard. What changes
when it works is the distance between having a thought and the agent having it:
a sentence spoken across the room instead of a context switch back to a
terminal.

What would make it worthless is not slowness. It is unreliability. A wake word
that misses, a turn that vanishes without a sound, or a transcript delivered to
the wrong session all teach you to stop trusting it, and a voice interface you
do not trust is one you stop using entirely. Everything measured below is
chosen against that failure rather than against raw speed.

## Four axes

One North Star KPI per axis, not a list. Each axis names what it protects here;
the KPI is chosen for this repository, not copied from another.

| Axis | What it protects | Where a KPI usually comes from |
| --- | --- | --- |
| Speed | The edit-run loop and the path from commit to release | Fast tier duration, lead time, cost per test |
| Security | Nothing reaches a machine without passing the gates | Open findings, secrets in history, time to patch, verified release assets |
| Maintainability | The next reader can change it safely | Coverage on behaviour that matters, warning-free gates, module and complexity ceilings |
| Scalability | The guarantees still hold as the work grows | Latency under the documented load, cost per unit of work, heavy tier duration |

## KPIs

A KPI has a current value, a target and a measurement that produces it,
automated wherever possible. A KPI nobody measures is decoration.

| Axis | KPI | Current | Target | Measured by |
| --- | --- | --- | --- | --- |
| Speed | Fast tier wall-clock duration, p95 | not measured | under 5 minutes | `fast / common` and `fast-rust / check` job durations |
| Security | Open advisories against dependencies | not measured | zero | `just lang-audit` in the weekly heavy tier |
| Maintainability | Line coverage on stable | not measured | at least 90 percent | `just lang-coverage`, the profile floor |
| Scalability | End of speech to transcript delivered, warm | not measured | under 1 second | no measurement exists yet; see below |

Rules for filling the table:

- Unmeasured is written as `not measured`, never estimated. The first completed
  run establishes the baseline. All four are unmeasured today because no CI run
  and no turn have happened yet; that is the honest state of a repository on its
  first commit, not a gap to be filled with guesses.
- The scalability KPI has no measurement behind it and will not acquire one by
  being written down. It needs a benchmark that replays a recorded utterance
  through a warm speech model and records the interval. Until that exists the
  row is a target with no instrument, which is the weakest kind of entry here.
- A value a gate verifies on every run is written plain: it cannot drift without
  turning something red. A value read by hand carries the date it was read,
  because nothing keeps it current afterwards.
- Targets come from the first baseline and the language profile, not from this
  file. A target that is always green is too easy; one that is always red is
  fantasy. Tighten the first, fix or drop the second.

## How we hold ourselves to it

- **Shift left, automated.** If a rule matters, it is a gate in the hooks and
  CI. If it cannot be automated, it is written down as a check someone runs.
- **Guardrails over gatekeepers.** A linter rule, a schema or a policy check
  beats a review comment that will be forgotten.
- **A gate that cannot fail is not a gate.** Prove each one by injecting the
  fault it exists to catch and watching it fail.
- **A decision is recorded where it is enforced.** An ADR beside the thing it
  governs.
- **Plans name the axis they move.** Work that degrades an axis without a
  stated trade-off is flagged, not merged quietly.

## What this is not

Not a claim that the targets are met. Direction lives in
[ROADMAP.md](ROADMAP.md); this file records what is measured and how.
