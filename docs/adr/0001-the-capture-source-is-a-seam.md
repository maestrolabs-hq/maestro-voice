<!-- Capture a decision, its reasons, costs and unresolved questions. -->
# 0001. The capture source is a seam

## Status

accepted

## Context

This repository's input is a microphone. Its two most delicate behaviours are
wake-word detection and endpointing, and both are delicate in the same way:
they fail quietly. A wake threshold slightly too high means the daemon ignores
you now and then. A trailing-silence window slightly too short truncates the
last word of every sentence. Neither raises an error, and neither is visible in
a log that only records what was captured.

Those behaviours are also the ones a test cannot reach if the only way into the
code is a live capture device. A test that needs a microphone needs a human to
speak into it, which means it does not run in CI, which means in practice it
does not run. The estate's own bar is that a gate which cannot fail is not a
gate; a test that never executes is the same problem wearing a different hat.

There is a second force. Audio capture is where this repository meets the
operating system, and that meeting is the least portable thing here: device
names, servers and permissions differ between Linux, WSL, macOS and Windows,
and on this host capture arrives through WSLg's PulseAudio bridge rather than a
physical device.

## Decision

Audio capture is an injectable source, not a call the rest of the code makes
directly. In production the source is an `ffmpeg` child process reading
PulseAudio. Under test it is a WAV file read from a committed corpus. Both
present the same thing to everything above them: a stream of fixed-size frames
of pulse-code-modulated samples at a known rate.

Two consequences follow, and they are the reason for the decision rather than
side effects of it:

The wake-word and endpointing rules take classified frames and return
decisions. They perform no input or output at all, which
`tests/standards.rs` enforces as an architecture boundary rather than leaving
to good intentions.

The behaviour that matters is therefore testable from a file. A recording of
the wake phrase, a phrase that nearly triggers it, a silence and a full
utterance are ordinary test fixtures, and the whole path from frames to
endpoint decision runs in CI on every pull request with no device present.

The alternative considered was binding a Rust audio library directly and
accepting that wake and endpoint behaviour is only exercised by hand. It was
rejected because it makes the project's least visible failures its least
tested ones.

## Consequences

Testing the part most likely to be subtly wrong becomes ordinary. Frames can
be replayed deterministically, so a threshold change is a diff with evidence
rather than an opinion about how it felt in the room.

Porting the capture path to another platform becomes a change in one place,
and it cannot silently change decision behaviour, because the decision layer
never knew what platform it was on.

The costs are real. A subprocess boundary is a process that can die, so
capture must treat `ffmpeg` exiting as an expected event with a restart policy,
not an impossible one. Frames cross that boundary as bytes on a pipe, which is
slower than an in-process callback and adds a small buffering delay; for a
wake-word loop running at eighty-millisecond hops this is affordable, and that
affordability is an assumption this decision rests on. The committed WAV corpus
is binary content in the repository, kept small deliberately, and the
`check-added-large-files` hook bounds it at one megabyte.

## Unresolved

Whether the `ffmpeg` subprocess is fast enough for barge-in, where playback
must stop within a few hundred milliseconds of speech starting, is not yet
measured. It is measured before barge-in ships, not assumed.

What reopens this decision: evidence that the subprocess boundary costs enough
latency to be heard, or a second capture platform whose frames cannot be made
to look like the first's. Either would mean the seam is in the wrong place, not
that there should be no seam. A preference for fewer moving parts does not
reopen it.
