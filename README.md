<!-- Introduce this repository, its use and the rules for changing it. -->
# maestro-voice

An always-on voice front end for the pi coding agent: say a wake phrase, speak,
and your words reach a running agent as a prompt; its reply is read back aloud.

[![CI](https://github.com/maestrolabs-hq/maestro-voice/actions/workflows/ci.yml/badge.svg)](https://github.com/maestrolabs-hq/maestro-voice/actions/workflows/ci.yml)
[![Heavy](https://github.com/maestrolabs-hq/maestro-voice/actions/workflows/heavy.yml/badge.svg)](https://github.com/maestrolabs-hq/maestro-voice/actions/workflows/heavy.yml)
[![Scorecard](https://api.securityscorecards.dev/projects/github.com/maestrolabs-hq/maestro-voice/badge)](https://securityscorecards.dev/viewer/?uri=github.com/maestrolabs-hq/maestro-voice)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Why it exists

Talking to a coding agent means typing at it, which means being at the keyboard
with a terminal focused. This is the other route: a wake phrase, a sentence, and
a spoken answer, while the agent keeps a single long-running session rather than
starting cold each time.

It owns the parts that belong to a device and a turn: the microphone, the wake
word, voice activity detection, the state machine for one exchange, and
playback. It owns no models. Speech recognition and speech synthesis run as
children of the [maestro-llamacpp](https://github.com/maestrolabs-hq/maestro-llamacpp)
router, which already arbitrates local model memory, so voice models are
admitted, evicted and idle-unloaded against the same budget as everything else
on the device. Two allocators with no shared ledger is how a graphics card gets
over-committed.

See [NORTHSTAR.md](NORTHSTAR.md) for commitments and honest measurement.

## Use

Nothing is wired yet. The binary compiles and says so:

```text
$ cargo run
maestro-voice 0.1.0
The turn loop is not wired yet: no microphone is opened and no model is called.
```

The endpointing rule in `src/endpoint.rs` is real and tested; everything around
it is not built.

## Development

```text
just setup
just check
```

`just setup` installs local hooks. `just check` runs every CI gate except the
CI-only full-history secrets scan; the local hook scans staged changes.
Heavy reports are separate evidence, not merge gates. Hooks run on commit
and merge-commit; language gates run on push. A local run cannot prove another
platform works: CI tests every claimed platform on each pull request.
See [CONTRIBUTING.md](CONTRIBUTING.md).

## Decisions

| ADR | Decision | Status |
| --- | --- | --- |
| [0001](docs/adr/0001-the-capture-source-is-a-seam.md) | Capture is an injectable source, so wake and endpoint behaviour is testable without a microphone | accepted |
| [0002](docs/adr/0002-the-profile-bindings-this-repository-states.md) | The quality-profile keys this repository binds, and the ones still open | accepted |

## Governance

- [GOVERNANCE.md](GOVERNANCE.md): who decides and what constrains them.
- [AGENTS.md](AGENTS.md): working rules for people and agents.
- [CONTEXT.md](CONTEXT.md): terms, including those designed but not built.
- [ROADMAP.md](ROADMAP.md) and [TODO.md](TODO.md): direction and short reminders.
- [SECURITY.md](SECURITY.md), [SUPPORT.md](SUPPORT.md) and
  [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md): reporting and conduct.
- [CHANGELOG.md](CHANGELOG.md) and [LICENSE](LICENSE): releases and reuse.

## Limitations

Stated because they are properties of the design, not defects awaiting a fix.

**It holds the microphone continuously.** Hearing a wake phrase means listening
all the time. Audio is examined in memory and nothing is transcribed until the
phrase fires, but the device is open whenever the daemon runs, and any
indicator your system shows for a live microphone will stay on.

**Linux only, for now.** Capture goes through PulseAudio and delivery runs the
`herdr` command. Neither has been implemented or tested on macOS or Windows, so
the platform matrix claims one platform rather than asserting portability
nobody has demonstrated. See
[ADR 0002](docs/adr/0002-the-profile-bindings-this-repository-states.md).

**It depends on things it does not own.** A running router to load the models,
a running Herdr session to deliver into, and `ffmpeg` on the path. Each is a
process that can be absent or die, and the daemon reports that rather than
pretending otherwise.

**Duplication is an unqualified gate.** The profile requires `similarity-rs` to
be proved against a real duplicate before its result means anything, and that
proof has not been done here.

## What this is not

Not a dictation tool: the transcript becomes a prompt for an agent, not text in
an editor. Not a voice changer, which is a different problem with a much tighter
latency budget. Not a model runner: if you want a model loaded, that is the
router's job.

A green badge is not proof of the commitments above. Unmeasured work stays
unmeasured until evidence is recorded.
