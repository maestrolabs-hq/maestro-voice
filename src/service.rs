//! The four things outside this process that a turn depends on.
//!
//! Each is a trait so that a whole turn can be driven from a file with no
//! router, no `herdr`, no graphics card and no speaker. That is the same
//! argument `docs/adr/0001-the-capture-source-is-a-seam.md` makes for the
//! microphone, applied to the other three boundaries: the behaviour worth
//! testing is what the daemon *does* about a refusal, and reproducing a real
//! refusal from a real router on demand is not something a test can do.
//!
//! The implementations live beside this file. Nothing here performs input or
//! output; it only says what input and output must look like.

use crate::speak::Language;
use crate::turn::{Delivery, Fault};

/// What the transcriber said about one utterance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    /// The words, as transcribed.
    pub text: String,
    /// The language it was detected in, when the service reported one.
    ///
    /// Optional because a detector can decline, and the design would rather
    /// answer in a default voice than refuse to answer at all.
    pub language: Option<Language>,
}

impl Transcript {
    /// Whether there is nothing here worth sending to an agent.
    ///
    /// Whitespace counts as nothing: a transcriber handed a cough returns a
    /// space or a bare punctuation mark often enough that treating it as words
    /// would wake the agent for a throat-clear.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.text.chars().any(char::is_alphanumeric)
    }
}

/// Speech recognition, as the router serves it.
pub trait Transcriber: Send + Sync {
    /// Ask for the model to be loaded now, without waiting for it.
    ///
    /// Best effort and deliberately unable to fail: the transcription that
    /// follows would load the model anyway, so a warm that does not land costs
    /// latency and never correctness.
    fn warm(&self);

    /// Transcribe one utterance, supplied as a complete WAV file.
    ///
    /// # Errors
    ///
    /// When the router refuses, the model never becomes ready, or nothing
    /// answers at all.
    fn transcribe(&self, wav: &[u8]) -> Result<Transcript, Fault>;
}

/// The way a transcript reaches the agent.
pub trait Courier: Send + Sync {
    /// Hand `text` to the agent, and say what became of it.
    ///
    /// Returns [`Delivery`] rather than a `Result` because "the agent is
    /// sitting on a question" is an answer about the agent, not a failure of
    /// this call.
    fn deliver(&self, text: &str) -> Delivery;
}

/// Speech synthesis, as the router serves it.
pub trait Synthesizer: Send + Sync {
    /// Say `text` in `language`, as samples at the crate's rate.
    ///
    /// `None` is a failure to synthesize. The reply is on screen either way,
    /// so the caller announces it quietly rather than treating it as an error
    /// worth a message.
    fn synthesize(&self, text: &str, language: &Language) -> Option<Vec<i16>>;
}

/// Where synthesized audio goes.
pub trait Player: Send + Sync {
    /// Play `samples`, returning when they have finished or been stopped.
    ///
    /// `false` reports that playback did not happen.
    fn play(&self, samples: &[i16]) -> bool;

    /// Stop whatever is playing, now.
    ///
    /// Called from the thread reading the microphone while [`Self::play`] is
    /// still running on another, because the whole point is to interrupt. An
    /// implementation must therefore tolerate being called when nothing is
    /// playing, and must not block.
    fn hush(&self);
}

/// Deciding whether a chunk of audio holds speech.
///
/// Its own trait because the daemon ships the simplest thing that works and
/// expects to replace it: see `docs/adr/0004`.
pub trait Ear: Send {
    /// Whether this chunk holds speech.
    fn speech(&mut self, chunk: &[i16]) -> bool;
}

#[cfg(test)]
mod tests {
    use super::Transcript;

    fn transcript(text: &str) -> Transcript {
        Transcript {
            text: text.to_owned(),
            language: None,
        }
    }

    #[test]
    fn a_transcript_with_words_is_not_empty() {
        assert!(!transcript("run the tests").is_empty());
        assert!(!transcript("42").is_empty(), "digits are words enough");
    }

    #[test]
    fn a_transcript_of_nothing_but_noise_counts_as_empty() {
        // What a transcriber hands back for a cough, a door, or a throat
        // clear. Delivering any of these would wake the agent for nothing.
        for noise in ["", " ", "\n", ".", "...", " . ", "-", "?"] {
            let t = transcript(noise);
            let empty = t.is_empty();
            assert_eq!(
                empty,
                noise.chars().all(|c| !c.is_alphanumeric()),
                "{noise:?} was judged {empty}"
            );
        }
    }

    #[test]
    fn punctuation_around_real_words_is_still_words() {
        assert!(!transcript("...run the tests.").is_empty());
    }
}
