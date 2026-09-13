<!-- Define project terms without describing their implementation. -->
# Context

Terms only, no implementation. A term marked **designed, not built** has a
meaning agreed and no code behind it; a written definition is not evidence of
delivery.

## Wake word

The spoken phrase that takes the daemon from listening-for-a-phrase to
recording-an-utterance. Not a password and not authentication: it is a cheap,
local, always-running check for one sound pattern, and it is expected to be
wrong occasionally in both directions.

## Utterance

One continuous stretch of speech, from the moment the wake word fires to the
moment endpointing says it is over. An utterance is what gets transcribed. It is
not a turn: a turn also contains the agent's reply.

## Endpointing

Deciding that an utterance has ended. Distinct from voice activity detection,
which judges a single frame; endpointing is the rule built on top of those
judgements, with its own windows for trailing silence, for a wake word nothing
followed, and for an upper bound on the whole recording.

## Voice activity detection

Classifying one frame of audio as speech or not speech. It answers a question
about a moment. It never decides that a sentence is finished; that is
endpointing.

## Turn

One complete exchange: an utterance goes to the agent, the agent works, and a
spoken block comes back. A turn can last minutes, because the agent may be doing
real work, and the daemon stays able to hear the wake word throughout.

## Spoken block

The short passage the agent marks for speech, distinct from the full written
reply. The written reply may contain code, file paths and tables, none of which
belong in audio. Only the spoken block is read aloud.

## Pre-roll

The audio kept from just before the wake word finished firing. Detection
completes at the end of the phrase, so recording from that instant alone would
clip whatever was said immediately after it. Pre-roll is what prevents a missing
first word.

## Barge-in (designed, not built)

Interrupting playback by speaking. The intended meaning is that a wake word
during speech stops the audio rather than queueing behind it. Nothing implements
this yet, and the latency it requires has not been measured.

## Shim

A small program on the search path that lets the router start something that is
not a stock model server, by presenting the flag surface the router already
sends and translating it. It is how a runtime the router does not know about
becomes an ordinary supervised child.

## Router child

A model process the maestro-llamacpp router starts, supervises, admits against a
memory budget and eventually unloads. This repository calls such children over
the network; it never starts one, and it never decides whether one fits in
memory.
