//! Short synthesized cues, because speech is not available when it is needed.
//!
//! Failures announce themselves with tones rather than words, and the reason
//! is structural: text-to-speech is a model the router owns, so it can be
//! refused for want of memory, fail to load, or be evicted. Those are exactly
//! the moments something needs saying. A cue that depends on the thing that
//! broke is not a cue.
//!
//! The vocabulary is small and countable: one tone acknowledges, two mean
//! refused, three mean blocked, and a long one means capture has given up.

use std::time::Duration;

use crate::capture::samples_in;

mod wave;

/// Peak amplitude as a fraction of full scale. Quiet on purpose: these fire on
/// every false wake, and a cue at full scale is a cue the user turns off.
const GAIN: f32 = 0.22;

/// The gentler level for a cue that means "never mind".
const SOFT_GAIN: f32 = 0.10;

/// One part of a cue: a tone at a pitch, or a silence.
struct Step {
    hertz: f32,
    millis: u64,
}

const fn tone(hertz: f32, millis: u64) -> Step {
    Step { hertz, millis }
}

const fn rest(millis: u64) -> Step {
    Step { hertz: 0.0, millis }
}

static LISTENING: [Step; 1] = [tone(880.0, 90)];
static DISMISSED: [Step; 1] = [tone(440.0, 70)];
static REFUSED: [Step; 3] = [tone(330.0, 90), rest(70), tone(330.0, 90)];
static BLOCKED: [Step; 5] = [
    tone(660.0, 70),
    rest(60),
    tone(660.0, 70),
    rest(60),
    tone(660.0, 70),
];
static STOPPED: [Step; 1] = [tone(220.0, 600)];

/// What the daemon has to say without speaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// The wake word was heard and recording has started.
    Listening,
    /// A wake nothing followed, abandoned quietly.
    Dismissed,
    /// The router would not serve a model, or it never became ready.
    Refused,
    /// The agent is sitting on a question, so the transcript was not sent.
    Blocked,
    /// Capture failed repeatedly and has stopped.
    Stopped,
}

impl Cue {
    /// The tones and silences this cue is made of.
    fn score(self) -> &'static [Step] {
        match self {
            Self::Listening => &LISTENING,
            Self::Dismissed => &DISMISSED,
            Self::Refused => &REFUSED,
            Self::Blocked => &BLOCKED,
            Self::Stopped => &STOPPED,
        }
    }

    /// How loud this cue is, relative to full scale.
    const fn gain(self) -> f32 {
        match self {
            Self::Dismissed => SOFT_GAIN,
            _ => GAIN,
        }
    }

    /// How long the whole cue lasts.
    #[must_use]
    pub fn length(self) -> Duration {
        self.score()
            .iter()
            .map(|step| Duration::from_millis(step.millis))
            .sum()
    }

    /// The cue as samples at the crate's rate, ready to play.
    #[must_use]
    pub fn samples(self) -> Vec<i16> {
        let gain = self.gain();
        let mut out = Vec::with_capacity(samples_in(self.length()));

        for step in self.score() {
            let count = samples_in(Duration::from_millis(step.millis));
            if step.hertz == 0.0 {
                wave::silence(&mut out, count);
            } else {
                wave::append(&mut out, step.hertz, count, gain);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Cue;
    use crate::capture::samples_in;
    use std::time::Duration;

    const EVERY: [Cue; 5] = [
        Cue::Listening,
        Cue::Dismissed,
        Cue::Refused,
        Cue::Blocked,
        Cue::Stopped,
    ];

    /// How many separate beeps a cue contains, measured over short windows: a
    /// sine crosses zero twice a cycle, so a per-sample test would count every
    /// cycle as its own beep.
    fn bursts(samples: &[i16]) -> usize {
        let window = samples_in(Duration::from_millis(5)).max(1);
        let mut count = 0;
        let mut inside = false;
        for slice in samples.chunks(window) {
            let loud = peak(slice) > 1000;
            if loud && !inside {
                count += 1;
            }
            inside = loud;
        }
        count
    }

    fn peak(samples: &[i16]) -> i32 {
        samples
            .iter()
            .map(|s| i32::from(*s).abs())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn every_cue_lasts_exactly_as_long_as_it_says() {
        for cue in EVERY {
            assert_eq!(
                cue.samples().len(),
                samples_in(cue.length()),
                "{cue:?} must produce the audio its own duration promises"
            );
        }
    }

    #[test]
    fn every_cue_is_actually_audible() {
        for cue in EVERY {
            assert!(peak(&cue.samples()) > 1000, "{cue:?} announces nothing");
        }
    }

    #[test]
    fn no_cue_is_loud_enough_to_be_unpleasant_all_day() {
        for cue in EVERY {
            assert!(
                peak(&cue.samples()) < i32::from(i16::MAX) / 3,
                "{cue:?} is too loud to live with"
            );
        }
    }

    #[test]
    fn every_cue_begins_and_ends_at_silence() {
        for cue in EVERY {
            let samples = cue.samples();
            assert!(
                peak(&samples[..1]) < 64 && peak(&samples[samples.len() - 1..]) < 64,
                "{cue:?} must fade in and out rather than snap"
            );
        }
    }

    #[test]
    fn the_failure_cues_carry_their_meaning_in_their_shape() {
        assert_eq!(bursts(&Cue::Listening.samples()), 1);
        assert_eq!(bursts(&Cue::Dismissed.samples()), 1);
        assert_eq!(bursts(&Cue::Refused.samples()), 2, "refusal is two tones");
        assert_eq!(bursts(&Cue::Blocked.samples()), 3, "blocked is three tones");
        assert_eq!(bursts(&Cue::Stopped.samples()), 1);
    }

    #[test]
    fn the_stop_cue_is_the_long_one() {
        for cue in [Cue::Listening, Cue::Dismissed, Cue::Refused, Cue::Blocked] {
            assert!(
                Cue::Stopped.length() > cue.length(),
                "the stop cue must outlast {cue:?}"
            );
        }
    }

    #[test]
    fn no_two_cues_sound_the_same() {
        for (i, first) in EVERY.iter().enumerate() {
            for second in &EVERY[i + 1..] {
                assert_ne!(
                    first.samples(),
                    second.samples(),
                    "{first:?} and {second:?} would be indistinguishable"
                );
            }
        }
    }
}
