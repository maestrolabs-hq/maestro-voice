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

Fetch the wake-word weights once, then ask what is not ready:

```text
just fetch-wake-models
maestro-voice check
```

`check` starts nothing. It reads the router's catalog at `GET /v1/models`,
never `GET /models/<id>/health`, because that relays to the child and would
load a model to answer a question about readiness. Its output has three
standings rather than two:

```text
ok           audio transport        'ffmpeg' at /usr/bin/ffmpeg
ok           microphone             'default' delivered 1280 samples
unknown      speaker                'default' cannot be confirmed: ffmpeg exits 0 ...
PROBLEM      voice agent            herdr does not know an agent named 'voice'
                                    fix: start it with 'herdr agent start', ...
```

`unknown` is not a pass and not a failure: it is something this check cannot
determine, and the line says why. Only `PROBLEM` sets the exit status.

Then:

```text
maestro-voice run
```

Settings come from the file named by `MAESTRO_VOICE_CONFIG`, or
`maestro-voice/config` under the usual configuration directory. Every setting
can be overridden for one run by `MAESTRO_VOICE_<SETTING>` -- so
`MAESTRO_VOICE_WAKE_THRESHOLD=0.6 maestro-voice run` tries a threshold without
editing anything. `maestro-voice help` lists the commands; `Config::keys` is
the full list of settings.

The two speech entries are served by the maestro-llamacpp router, which admits
them against the same memory budget as every other local model. This daemon
loads nothing itself.

## The spoken reply

The agent's reply comes back through the agent's own process, not off its
terminal. Herdr's documentation is explicit that pi draws on the alternate
screen, so rows that scroll away never enter its scrollback and reading a whole
reply back out of the pane is not something that can be relied on.

Instead, `extensions/voice-speak.ts` is a pi extension loaded in the voice pane.
When a turn settles it posts the tail of the final assistant message to the
daemon on loopback, and the daemon decides what, if anything, to say:

```text
agent_end      carries the messages; keep the latest assistant text
agent_settled  carries only its type, but means the turn is final
                 -> POST /turn  (text/plain, at most 16 KiB)
                      -> src/intake.rs   accepts it at the trust boundary
                      -> src/speak.rs    finds <speak>...</speak>, or does not
```

The extension is inert unless `MAESTRO_VOICE_PORT` names the daemon's intake
port, which is what marks a pane as the voice pane; an ordinary pi session
elsewhere loads the file and does nothing. It makes no judgement about the
reply and never delays, retries into, or fails a turn because the speaker was
unavailable.

The agent is expected to end a spoken turn with a short block:

```text
<speak>I added the shutdown handler and the test passes.</speak>
```

Only that block is spoken. Code, tables and paths in the written reply never
reach the speaker, and the few things that read badly aloud even inside the
block are normalised: inline code loses its backticks, `src/proxy/relay.rs:109`
becomes "relay.rs line 109", and a link is said as its host.

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

**A spoken reply depends on the agent writing one.** Only the marked block is
read aloud, so a turn where the agent forgets it is a turn that says nothing.
That was accepted deliberately, in exchange for never reading a diff to you, on
the condition that it be countable: a turn that settles without a block is
posted to the daemon exactly like one that has a block, so "how often does this
happen" is a number rather than an impression.

**It depends on things it does not own.** A running router to load the models,
a running Herdr session to deliver into, and `ffmpeg` on the path. Each is a
process that can be absent or die, and the daemon reports that rather than
pretending otherwise.

**A misconfigured speaker cannot be detected.** `ffmpeg` exits 0 when the sink
does not exist, because the audio server plays to its default instead. So a
wrong `playback_device` is audible on the wrong device rather than reported,
and `check` says so rather than showing a tick it has not earned.

**Speech is told from the room by loudness, not by a neural detector.** It
adapts to the room, and it cannot separate a loud stationary room from someone
talking without pausing. The limit and what replaces it are in
[ADR 0005](docs/adr/0005-a-loudness-gate-until-a-neural-one-earns-its-place.md).

## What this is not

Not a dictation tool: the transcript becomes a prompt for an agent, not text in
an editor. Not a voice changer, which is a different problem with a much tighter
latency budget. Not a model runner: if you want a model loaded, that is the
router's job.

A green badge is not proof of the commitments above. Unmeasured work stays
unmeasured until evidence is recorded.
