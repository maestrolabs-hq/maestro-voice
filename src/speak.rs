//! Turning the agent's reply into something worth hearing.
//!
//! The agent ends a turn with a short block marked for speech, and this module
//! is what decides whether there is one, whether it is usable, and what the
//! synthesizer is finally handed.
//!
//! **The parse happens here, once.** The pi extension that watches the agent
//! posts the final assistant message as it stands and makes no judgement about
//! it. A second implementation of these rules in TypeScript would be a second
//! thing to keep correct, tested by whichever of the two was easier to reach,
//! and the one on the far side of a socket is the harder one. Input is parsed
//! into a checked representation at the boundary it arrives on.
//!
//! Nothing here performs input or output: it takes the text of a reply and
//! returns a decision, which is what lets every rule below be exercised from an
//! ordinary test.

mod normalise;

/// The longest text handed to the synthesizer, in bytes.
///
/// At an ordinary speaking rate this is around half a minute of audio. The
/// block is meant to be a sentence or three; a reply long enough to pass this
/// is a reply that stopped being a spoken summary, and reading all of it would
/// hold the speaker for longer than the owner will wait.
pub const MAX_SPOKEN_CHARS: usize = 600;

const OPEN: &str = "<speak>";
const CLOSE: &str = "</speak>";

/// Sentence endings, used to cut an overlong block where a listener expects a
/// pause rather than mid-word.
const SENTENCE_END: [char; 3] = ['.', '!', '?'];

/// The language a thing is spoken in, as the transcription reports it.
///
/// This is the interface between transcription and speech: whisper detects the
/// language of the utterance, and the same tag decides which voice answers, so
/// a question asked in French is answered in French. Accepted as two or three
/// lowercase ASCII letters, which is what an ISO 639 code is; anything else is
/// refused here rather than sent to a synthesizer that would guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language(String);

impl Language {
    /// A language tag, or `None` when it is not one.
    #[must_use]
    pub fn new(tag: &str) -> Option<Self> {
        let usable = matches!(tag.len(), 2 | 3)
            && tag
                .chars()
                .all(|c| c.is_ascii_lowercase() && c.is_ascii_alphabetic());
        usable.then(|| Self(tag.to_owned()))
    }

    /// The tag as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Text ready for the synthesizer, and the language to say it in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spoken {
    text: String,
    language: Language,
    truncated: bool,
}

impl Spoken {
    /// What to say.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Which voice should say it.
    #[must_use]
    pub const fn language(&self) -> &Language {
        &self.language
    }

    /// Whether the block was longer than [`MAX_SPOKEN_CHARS`] and was cut.
    ///
    /// Worth logging rather than announcing: the whole reply is on screen, and
    /// telling the owner their agent is too wordy on every turn is its own
    /// kind of noise.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

/// What a finished turn left for the speaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A usable block: say this.
    Spoken(Spoken),
    /// The turn carried no block at all.
    ///
    /// Not a failure. The design accepted that the block depends on the agent
    /// remembering to write one, on the condition that a turn without one is
    /// counted rather than mysterious. This is the countable outcome.
    Silent,
}

/// Why a block that exists cannot be spoken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// An opening marker with no closing one.
    ///
    /// Refused rather than read to the end of the message: a reply cut off
    /// mid-block is exactly the case where guessing reads half a sentence with
    /// total confidence.
    Unterminated,
    /// A well-formed block holding nothing to say.
    Empty,
}

/// What to speak for one finished turn, if anything.
///
/// `message` is the final assistant message exactly as it was written; an
/// empty one is simply a turn with no block. `language` comes from the
/// transcription of the utterance being answered.
///
/// # Errors
///
/// [`Refusal::Unterminated`] when a block is opened and never closed, and
/// [`Refusal::Empty`] when a well-formed block holds no words.
pub fn extract(message: &str, language: &Language) -> Result<Outcome, Refusal> {
    let Some(open) = message.find(OPEN) else {
        return Ok(Outcome::Silent);
    };
    let after = &message[open + OPEN.len()..];
    let Some(close) = after.find(CLOSE) else {
        return Err(Refusal::Unterminated);
    };

    let said = normalise::for_speech(&after[..close]);
    if said.is_empty() {
        return Err(Refusal::Empty);
    }

    let (text, truncated) = shorten(&said);
    Ok(Outcome::Spoken(Spoken {
        text,
        language: language.clone(),
        truncated,
    }))
}

/// Cut to the cap, preferring a finished sentence and then a word boundary.
fn shorten(said: &str) -> (String, bool) {
    if said.len() <= MAX_SPOKEN_CHARS {
        return (said.to_owned(), false);
    }

    let mut cut = MAX_SPOKEN_CHARS;
    while !said.is_char_boundary(cut) {
        cut -= 1;
    }
    let head = &said[..cut];

    let end = head
        .rfind(SENTENCE_END)
        .map_or_else(|| head.rfind(' ').unwrap_or(cut), |at| at + 1);
    (head[..end].trim_end().to_owned(), true)
}
