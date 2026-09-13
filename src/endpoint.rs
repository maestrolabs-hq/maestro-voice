//! Deciding when a spoken utterance has ended.
//!
//! The rule is separated from the microphone on purpose. Endpointing is the
//! behaviour most likely to be wrong in a way nobody notices -- a window that
//! is slightly too short truncates the last word, and one slightly too long
//! makes every turn feel sluggish -- and it is the behaviour hardest to test
//! if it can only be reached through a live capture device. See
//! `docs/adr/0001-the-capture-source-is-a-seam.md`.
//!
//! Nothing here reads audio. Callers classify each frame as speech or silence
//! and feed that judgement in; this module owns only the arithmetic.

use std::time::Duration;

/// What the caller should do after feeding one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Keep recording: the utterance is still going, or has not begun.
    Continue,
    /// Stop recording and transcribe what was captured.
    Ended(Ending),
}

/// Why an utterance stopped, which is not the same question as whether it did.
///
/// A turn that ends in silence carries a sentence someone finished saying. One
/// that hits the cap may be a stuck microphone or a monologue, and the two
/// deserve different handling upstream even though both stop the recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// Enough uninterrupted silence followed speech.
    Silence,
    /// The hard limit was reached, whether or not speech ever started.
    Cap,
    /// The window to start speaking expired without any speech at all.
    NeverSpoke,
}

/// The endpointing rule for one utterance.
///
/// Three windows, each answering a different failure:
///
/// - `silence` ends a finished sentence.
/// - `lead_in` abandons a wake word that nothing followed, so a false trigger
///   costs nothing and says nothing.
/// - `cap` bounds the whole utterance, so a microphone stuck open cannot
///   record without end.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    silence: Duration,
    lead_in: Duration,
    cap: Duration,
}

impl Rule {
    /// A rule from its three windows.
    #[must_use]
    pub const fn new(silence: Duration, lead_in: Duration, cap: Duration) -> Self {
        Self {
            silence,
            lead_in,
            cap,
        }
    }

    /// Start tracking one utterance under this rule.
    #[must_use]
    pub const fn start(self) -> Utterance {
        Utterance {
            rule: self,
            elapsed: Duration::ZERO,
            quiet: Duration::ZERO,
            spoke: false,
        }
    }
}

/// One utterance in progress.
#[derive(Debug, Clone, Copy)]
pub struct Utterance {
    rule: Rule,
    elapsed: Duration,
    quiet: Duration,
    spoke: bool,
}

impl Utterance {
    /// Feed one frame and learn whether the utterance is over.
    ///
    /// `frame` is how much audio this call represents, so the caller's frame
    /// size stays the caller's business. Silence before any speech does not
    /// count toward the trailing-silence window: a pause while someone decides
    /// what to say is not the end of a sentence they never started.
    pub fn observe(&mut self, frame: Duration, speech: bool) -> Decision {
        self.elapsed = self.elapsed.saturating_add(frame);

        if speech {
            self.spoke = true;
            self.quiet = Duration::ZERO;
        } else if self.spoke {
            self.quiet = self.quiet.saturating_add(frame);
        }

        if self.spoke && self.quiet >= self.rule.silence {
            return Decision::Ended(Ending::Silence);
        }
        if !self.spoke && self.elapsed >= self.rule.lead_in {
            return Decision::Ended(Ending::NeverSpoke);
        }
        if self.elapsed >= self.rule.cap {
            return Decision::Ended(Ending::Cap);
        }
        Decision::Continue
    }

    /// Whether any frame so far was classified as speech.
    #[must_use]
    pub const fn heard_speech(self) -> bool {
        self.spoke
    }
}

#[cfg(test)]
mod tests {
    use super::{Decision, Ending, Rule};
    use std::time::Duration;

    const FRAME: Duration = Duration::from_millis(100);

    fn rule() -> Rule {
        Rule::new(
            Duration::from_millis(800),
            Duration::from_secs(3),
            Duration::from_secs(30),
        )
    }

    /// Feed a pattern of frames, where `true` is speech, and report the first
    /// decision that ended the utterance.
    fn run(pattern: &[bool]) -> Option<Ending> {
        let mut utterance = rule().start();
        for &speech in pattern {
            if let Decision::Ended(ending) = utterance.observe(FRAME, speech) {
                return Some(ending);
            }
        }
        None
    }

    #[test]
    fn silence_before_speech_does_not_end_the_utterance() {
        // Two seconds of quiet is far longer than the 800ms trailing window,
        // but nothing has been said yet, so that window must not apply.
        let pattern = [false; 20];
        assert_eq!(
            run(&pattern),
            None,
            "quiet before speech must not be read as a finished sentence"
        );
    }

    #[test]
    fn a_wake_word_nothing_follows_expires_as_a_false_wake() {
        // Past the three-second lead-in, and still nothing said.
        let pattern = [false; 31];
        assert_eq!(
            run(&pattern),
            Some(Ending::NeverSpoke),
            "a false trigger must give up on its own, and say which ending it was"
        );
    }

    #[test]
    fn eight_quiet_frames_after_speech_end_the_utterance() {
        let mut pattern = vec![true; 5];
        pattern.extend([false; 8]);
        assert_eq!(run(&pattern), Some(Ending::Silence));
    }

    #[test]
    fn a_pause_shorter_than_the_window_does_not_end_the_utterance() {
        // Seven quiet frames is 700ms, under the 800ms window: someone drawing
        // breath mid-sentence must not be treated as having finished.
        let mut pattern = vec![true; 5];
        pattern.extend([false; 7]);
        pattern.extend([true; 5]);
        assert_eq!(
            run(&pattern),
            None,
            "a 700ms pause is under the 800ms window and must not end the turn"
        );
    }

    #[test]
    fn the_quiet_window_restarts_after_more_speech() {
        let mut pattern = vec![true; 3];
        pattern.extend([false; 7]);
        pattern.push(true);
        pattern.extend([false; 8]);
        assert_eq!(
            run(&pattern),
            Some(Ending::Silence),
            "the window is measured from the last speech, not from the first pause"
        );
    }

    #[test]
    fn continuous_speech_stops_at_the_cap() {
        let pattern = [true; 400];
        assert_eq!(
            run(&pattern),
            Some(Ending::Cap),
            "a microphone that never goes quiet must still be bounded"
        );
    }

    #[test]
    fn heard_speech_reports_whether_anything_was_said() {
        let mut utterance = rule().start();
        assert!(!utterance.heard_speech());
        let _ = utterance.observe(FRAME, true);
        assert!(utterance.heard_speech());
    }
}
