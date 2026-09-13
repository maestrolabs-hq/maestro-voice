//! Hearing the wake phrase.
//!
//! Two things live here and they are deliberately separable. [`Scorer`] turns
//! audio into a number per 80-millisecond chunk and is the only part that
//! knows a neural network exists. [`Detector`] turns that number series into
//! the one event the rest of the daemon cares about: the phrase was just
//! spoken.
//!
//! The split is not tidiness. The scoring engine's licence constrains what
//! this project may be, so the engine is the component most likely to be
//! replaced; and the firing rule is the part that must be testable without a
//! microphone, a model, or a licence. Keeping the boundary honest is what
//! makes both true at once. See
//! `docs/adr/0001-the-capture-source-is-a-seam.md` and `docs/adr/0003`.

mod error;
mod models;
mod scorer;

pub use error::Error;
pub use models::{Missing, ModelSet};
pub use scorer::{CHUNK, SAMPLE_RATE, Scorer};

/// Turns a score series into wake events.
///
/// Saying the phrase holds the score above the threshold for several chunks
/// running. Reporting each of them would wake the daemon once per eighty
/// milliseconds for one utterance, so this reports the crossing rather than
/// the state: once on the way up, then nothing until the score has fallen back
/// under the line.
///
/// Nothing here performs input or output, which is what lets the behaviour
/// most likely to be tuned by feel be tested by replay instead.
#[derive(Debug, Clone, Copy)]
pub struct Detector {
    threshold: f32,
    /// False while the score sits above the threshold, so that a held score
    /// cannot fire twice.
    armed: bool,
}

impl Detector {
    /// A detector that fires above `threshold`.
    ///
    /// The threshold is a tuning decision about a particular room, voice and
    /// microphone, so it is supplied rather than chosen here.
    #[must_use]
    pub const fn new(threshold: f32) -> Self {
        Self {
            threshold,
            armed: true,
        }
    }

    /// Feed one score; true exactly once per upward crossing.
    ///
    /// A score equal to the threshold counts as below it. That is an arbitrary
    /// choice made once so it is not made differently twice: it keeps a
    /// threshold of zero from firing on digital silence.
    pub fn observe(&mut self, score: f32) -> bool {
        let above = score > self.threshold;
        let fires = above && self.armed;
        self.armed = !above;
        fires
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A detector starts ready, so a phrase spoken immediately is not lost.
    #[test]
    fn a_fresh_detector_is_armed() {
        let mut detector = Detector::new(0.5);
        assert!(detector.observe(0.9), "the first phrase must wake it");
    }

    /// Silence neither fires nor disarms.
    #[test]
    fn scores_under_the_threshold_leave_it_armed() {
        let mut detector = Detector::new(0.5);
        for quiet in [0.0, 0.1, 0.49, 0.2] {
            assert!(!detector.observe(quiet), "{quiet} is under the threshold");
        }
        assert!(detector.observe(0.51), "still ready after the quiet");
    }

    /// The interesting failure is the one that costs nothing to write down: a
    /// detector that reports the state rather than the crossing floods the
    /// daemon. This is that test.
    #[test]
    fn a_held_score_fires_exactly_once() {
        let mut detector = Detector::new(0.5);
        let held = [0.8_f32; 12];

        let fired = held.iter().filter(|&&s| detector.observe(s)).count();

        assert_eq!(fired, 1, "one utterance is one wake, not twelve");
    }
}
