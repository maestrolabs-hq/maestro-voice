<!-- Record long-term direction without promising dates. -->
# Roadmap

Themes describe direction, not delivery dates. [TODO.md](TODO.md) holds
short-term reminders.

## Theme: hear the phrase reliably

- **Why:** everything downstream is unreachable until the wake word works, and a
  detector that misses one time in five is one nobody leaves running.
- **Done when:** a committed audio corpus of the phrase, near-miss phrases and
  silence runs in CI, and the measured false-accept and false-reject rates are
  recorded in [NORTHSTAR.md](NORTHSTAR.md) rather than described.
- **Issue / ADR:** [ADR 0001](docs/adr/0001-the-capture-source-is-a-seam.md)
  makes this testable without a device.

## Theme: one turn, end to end

- **Why:** the value is a complete exchange, not a transcript. Wake, record,
  transcribe, deliver, hear an answer.
- **Done when:** speaking a sentence puts it in the agent's session and the
  spoken block comes back through the speaker, with every failure along the way
  producing an audible signal rather than silence.

## Theme: never be the reason a model will not load

- **Why:** the router arbitrates device memory for every local model. A voice
  stack that pins memory outside that ledger breaks the guarantee for everything
  else.
- **Done when:** the speech models are ordinary router children, evicted and
  reloaded like any other, and that is demonstrated rather than assumed.

## Theme: say what is wrong, out loud

- **Why:** a daemon you interact with by voice cannot report failures on a
  screen you are not looking at.
- **Done when:** every failure in the turn path has a distinct audible signal
  that does not depend on the speech synthesis model being available, since that
  model is one of the things that can fail.

## Not planned

- **Dictation into an editor.** The transcript is a prompt for an agent. Turning
  it into a general text-input method is a different product.
- **Realtime voice conversion.** A much tighter latency budget and a different
  problem; it belongs elsewhere.
- **Speaker identification.** The wake word is not authentication and will not
  be made into it.
- **Cloud speech services.** Local models under the router, or nothing.
