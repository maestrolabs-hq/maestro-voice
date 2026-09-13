<!-- Capture a decision, its reasons, costs and unresolved questions. -->
# 0005. A loudness gate until a neural one earns its place

## Status

accepted

## Context

The endpointing rule in `src/endpoint.rs` asks one question of every frame: was
that speech? It performs no input or output and does not classify anything
itself, which is what
[ADR 0001](0001-the-capture-source-is-a-seam.md) is about. Something has to
answer it.

The obvious answer is Silero, a small neural voice activity detector, and it is
what this project would use if the choice were only technical. Two things made
it not only technical.

It is a separate model under its own licence, fetched separately.
[ADR 0003](0003-the-wake-word-runtime-and-its-weights.md) records what was
learned while sorting out the wake-word weights: openWakeWord's bundled copy of
`silero_vad.onnx` is swept up by that package's blanket NonCommercial claim,
while the `silero-vad` package ships an MIT-licensed model of its own. They are
not the same file -- 1 807 522 bytes against 2 327 524 -- and there are four
variants in the second. Choosing among them is a decision with consequences,
and it was deferred rather than made in passing.

Meanwhile the turn loop needed a speech decision to exist at all.

## Decision

Ship a loudness gate, and be explicit that it is one.

`src/voice.rs` classifies a chunk as speech when its root-mean-square amplitude
stands a factor above a floor, and the floor is the quietest thing heard in the
last four seconds. Adaptive rather than fixed, because a fixed threshold is a
fact about one room with one microphone at one gain, and this daemon is meant
to be always on in a room whose noise changes.

The first attempt followed the floor with a single decay rate and excluded
speech from teaching it. A test caught what that does: a steady fan reads as
speech, speech never teaches the floor, so the fan stays speech for ever and
every wake records to the cap. A single rate cannot separate a stationary loud
room from someone talking without pausing -- both are sustained energy -- and
that is the honest reason real systems reach for a neural detector. The
windowed minimum can separate them, because a room has quiet moments between
words and a fan does not.

## Consequences

The turn loop works today, with no model to fetch, no licence to resolve and
nothing on the graphics card. The gate is pure, so every rule in it is tested
from a table of amplitudes rather than by speaking into a microphone.

Its limit is stated in the module and repeated here because it is the thing a
reader should know: several seconds of speech with no gap at all will eventually
become the new floor, and the speaker will be classified as silent while still
talking. Real speech has gaps, and the utterance cap bounds the damage when it
does not. A cleverer threshold does not fix this; a different kind of detector
does.

It is also the component most likely to be blamed for a behaviour it did not
cause. Endpointing that feels wrong -- a sentence cut short, a turn that will
not end -- is as likely to be this gate misjudging a frame as the windows in
`src/endpoint.rs` being wrong. Both are worth checking before either is tuned.

## Unresolved

Whether this is good enough in the owner's actual room is not measured. Every
fixture is synthetic, and the threshold that matters is the one that holds with
a real microphone at a real distance in a room with whatever is running in it.
That is a manual check and it has not been done.

What reopens this decision:

- Endpointing that misbehaves in the room in a way traced to frame
  classification rather than to the windows.
- The `silero-vad` model being fetched for some other reason, at which point the
  licence question is already answered and the cost of using it is a few lines
  behind `crate::service::Ear`, which exists to make exactly that swap cheap.

A preference for the more sophisticated component does not reopen it. If it is
swapped, the loudness gate stays as the fallback for a machine that has not
fetched the model, because a daemon that cannot classify a frame cannot end an
utterance, and a daemon that cannot end an utterance is not a daemon.
