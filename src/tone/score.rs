//! What each cue is made of.
//!
//! Separated from the vocabulary beside it because they are different
//! questions: this file is the arithmetic of pitch, length and loudness, and
//! `tone.rs` is the far more interesting question of which cues exist and what
//! they mean. The module-size gate exposed that seam rather than inventing it.

/// Peak amplitude as a fraction of full scale. Quiet on purpose: these fire on
/// every false wake, and a cue at full scale is a cue the user turns off.
pub(super) const GAIN: f32 = 0.22;

/// The gentler level for the cues that mean "nothing is coming".
pub(super) const SOFT_GAIN: f32 = 0.10;

/// One part of a cue: a tone at a pitch, or a silence.
pub(super) struct Step {
    pub(super) hertz: f32,
    pub(super) millis: u64,
}

impl Step {
    /// Whether this step is a silence rather than a pitch.
    ///
    /// Named once so that neither the synthesizer nor a test compares a float
    /// to zero by hand, and so "a rest" is one idea with one spelling.
    pub(super) fn is_rest(&self) -> bool {
        self.hertz <= 0.0
    }
}

const fn tone(hertz: f32, millis: u64) -> Step {
    Step { hertz, millis }
}

const fn rest(millis: u64) -> Step {
    Step { hertz: 0.0, millis }
}

pub(super) static LISTENING: [Step; 1] = [tone(880.0, 90)];
pub(super) static DISMISSED: [Step; 1] = [tone(440.0, 70)];
pub(super) static MUTED: [Step; 1] = [tone(294.0, 220)];
pub(super) static REFUSED: [Step; 3] = [tone(330.0, 90), rest(70), tone(330.0, 90)];
pub(super) static BLOCKED: [Step; 5] = [
    tone(660.0, 70),
    rest(60),
    tone(660.0, 70),
    rest(60),
    tone(660.0, 70),
];
pub(super) static STOPPED: [Step; 1] = [tone(220.0, 600)];

#[cfg(test)]
mod tests {
    use super::{BLOCKED, REFUSED};

    #[test]
    fn a_rest_is_silent_and_a_tone_is_not() {
        assert!(REFUSED[1].is_rest(), "the gap in a refusal is a rest");
        assert!(!REFUSED[0].is_rest(), "and the halves around it are tones");
    }

    #[test]
    fn the_multi_tone_cues_alternate_tone_and_rest() {
        // A cue whose rests went missing would sound like one long beep, which
        // is how "two tones" quietly becomes indistinguishable from one.
        for (name, steps) in [("refused", &REFUSED[..]), ("blocked", &BLOCKED[..])] {
            for (at, step) in steps.iter().enumerate() {
                assert_eq!(
                    step.is_rest(),
                    at % 2 == 1,
                    "{name} step {at} breaks the tone-rest alternation"
                );
            }
        }
    }
}
