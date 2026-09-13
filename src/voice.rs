//! Telling speech from a quiet room, by loudness alone.
//!
//! This is the simplest thing that answers the question the endpointing rule
//! asks, and it is deliberately not a neural voice activity detector. The one
//! this project would otherwise use, Silero, is a separate model under its own
//! licence, and fetching it is a decision with consequences of its own;
//! shipping a loudness gate first keeps the turn loop working and honest about
//! what it is. `docs/adr/0004` records that decision and what replaces it.
//!
//! What it does do is adapt, because a fixed loudness threshold is a fact about
//! one room with one microphone at one gain. The floor is the quietest thing
//! heard recently, and speech is whatever stands well above it.
//!
//! **The limit, stated plainly.** Loudness cannot separate a loud stationary
//! room from someone talking without pausing: both are sustained energy. This
//! gate resolves that by trusting the quietest recent moment, which means
//! several seconds of speech with no gap at all will eventually be read as the
//! new floor. Real speech has gaps, and the utterance cap bounds the damage
//! when it does not. A neural detector is the fix, not a cleverer threshold.
//!
//! Nothing here performs input or output.

use std::collections::VecDeque;

use crate::service::Ear;

/// How far above the floor a chunk must be to count as speech.
///
/// A doubling in amplitude. Speech against a quiet room is many times this; the
/// margin is for a room that is not quiet.
const OVER_FLOOR: f32 = 2.0;

/// The quietest floor worth believing, as a root-mean-square amplitude.
///
/// Digital silence has a floor of zero, and everything is infinitely above
/// zero. This is the level below which a room is treated as silent rather than
/// as a very quiet reference to measure against.
const FLOOR_MINIMUM: f32 = 30.0;

/// How much recent audio the floor is drawn from, in chunks of 80 milliseconds.
///
/// Long enough to span the pauses between words and short enough to follow a
/// room that changes. Four seconds.
const WINDOW: usize = 50;

/// A loudness gate whose floor is the quietest thing it heard recently.
#[derive(Debug, Clone)]
pub struct Gate {
    recent: VecDeque<f32>,
    over: f32,
}

impl Default for Gate {
    fn default() -> Self {
        Self::new(OVER_FLOOR)
    }
}

impl Gate {
    /// A gate whose speech threshold is `over` times the floor.
    #[must_use]
    pub fn new(over: f32) -> Self {
        Self {
            recent: VecDeque::with_capacity(WINDOW),
            over,
        }
    }

    /// The floor as it currently stands, which is what makes the adaptation
    /// observable in a test rather than only in a room. Also what `check`
    /// reports, so a misbehaving microphone can be seen rather than guessed at.
    ///
    /// A gate that has heard nothing reports the minimum rather than infinity,
    /// so the first words spoken into a fresh daemon are heard. Folding an
    /// empty window would otherwise put the threshold out of reach of any
    /// audio at all.
    #[must_use]
    pub fn floor(&self) -> f32 {
        // Folded from infinity, not from the minimum: seeding the fold with the
        // minimum would cap the answer at it, and a noisy room would report a
        // silent floor forever.
        let quietest = self.recent.iter().copied().fold(f32::INFINITY, f32::min);
        if quietest.is_finite() {
            quietest.max(FLOOR_MINIMUM)
        } else {
            FLOOR_MINIMUM
        }
    }

    /// The loudness a chunk must exceed to count as speech.
    #[must_use]
    pub fn threshold(&self) -> f32 {
        self.floor() * self.over
    }
}

/// The root-mean-square amplitude of a chunk.
#[must_use]
pub fn loudness(chunk: &[i16]) -> f32 {
    if chunk.is_empty() {
        return 0.0;
    }
    let total: f64 = chunk.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    // A chunk is 1280 samples, so the count is exact in a double many times
    // over; the lint is about lengths this code cannot produce.
    #[allow(clippy::cast_precision_loss)]
    let mean = total / chunk.len() as f64;
    // Lossy by construction: an amplitude needs six significant figures and
    // single precision carries seven.
    #[allow(clippy::cast_possible_truncation)]
    let rms = mean.sqrt() as f32;
    rms
}

impl Ear for Gate {
    fn speech(&mut self, chunk: &[i16]) -> bool {
        let level = loudness(chunk);
        // Judged against the floor as it stood before this chunk, so a chunk
        // never raises the bar it is itself being measured against.
        let speaking = level > self.threshold();

        if self.recent.len() == WINDOW {
            self.recent.pop_front();
        }
        self.recent.push_back(level);
        speaking
    }
}

#[cfg(test)]
mod tests {
    use super::{FLOOR_MINIMUM, Gate, WINDOW, loudness};
    use crate::capture::CHUNK_SAMPLES;
    use crate::service::Ear;

    /// A chunk at a constant amplitude.
    fn at(amplitude: i16) -> Vec<i16> {
        (0..CHUNK_SAMPLES)
            .map(|n| if n % 2 == 0 { amplitude } else { -amplitude })
            .collect()
    }

    /// Feed `chunks` of one level and report how many read as speech.
    fn feed(gate: &mut Gate, amplitude: i16, chunks: usize) -> usize {
        let audio = at(amplitude);
        (0..chunks).filter(|_| gate.speech(&audio)).count()
    }

    /// Feed audio where the count is not what the test is about.
    fn soak(gate: &mut Gate, amplitude: i16, chunks: usize) {
        let heard = feed(gate, amplitude, chunks);
        debug_assert!(heard <= chunks);
    }

    #[test]
    fn digital_silence_is_not_speech() {
        let mut gate = Gate::default();
        assert!(!gate.speech(&at(0)), "a muted microphone is not a voice");
        assert!(!gate.speech(&[]), "and neither is no audio at all");
    }

    #[test]
    fn ordinary_speech_levels_register_from_the_first_chunk() {
        let mut gate = Gate::default();
        assert!(
            gate.speech(&at(3000)),
            "a normal speaking level must not need a warm-up to be heard"
        );
    }

    #[test]
    fn a_quiet_room_does_not_become_a_reference_that_hears_everything() {
        // The failure this prevents: a floor that falls to zero in a silent
        // room, after which any faint sound is infinitely above it.
        let mut gate = Gate::default();
        assert_eq!(feed(&mut gate, 0, 200), 0, "silence is never speech");

        assert!(
            gate.floor() >= FLOOR_MINIMUM,
            "the floor must not fall below what silence is worth: {}",
            gate.floor()
        );
        assert!(
            !gate.speech(&at(25)),
            "a faint hiss must not read as speech just because the room is quiet"
        );
    }

    #[test]
    fn a_steady_noisy_room_stops_counting_as_speech() {
        // A fan. The first chunks read as speech, which is correct -- nothing
        // yet knows the room -- but a gate that never learns reports speech all
        // day and the daemon records until the cap on every wake.
        let mut gate = Gate::default();
        soak(&mut gate, 600, WINDOW);

        assert!(
            gate.floor() > FLOOR_MINIMUM * 2.0,
            "the floor must follow a genuinely noisy room: {}",
            gate.floor()
        );
        assert!(
            !gate.speech(&at(700)),
            "the fan must stop counting as speech once it is the floor"
        );
        assert!(
            gate.speech(&at(6000)),
            "and a voice over the fan must still register"
        );
    }

    #[test]
    fn a_sentence_with_ordinary_pauses_reads_as_speech_throughout() {
        // Real speech is not continuous: there are gaps between words, and the
        // floor is drawn from them. This is the case the gate must get right,
        // because it is the case that happens.
        let mut gate = Gate::default();
        soak(&mut gate, 0, WINDOW);

        let mut heard = 0;
        let mut spoken = 0;
        for word in 0..20 {
            // Roughly a third of a second of word, then a short gap.
            heard += feed(&mut gate, 4000, 4);
            spoken += 4;
            soak(&mut gate, 0, if word % 3 == 0 { 3 } else { 1 });
        }

        assert_eq!(
            heard, spoken,
            "every chunk of every word must read as speech, heard {heard} of {spoken}"
        );
    }

    #[test]
    fn the_floor_is_judged_before_the_chunk_that_is_being_measured() {
        // Otherwise the first loud chunk raises the floor and then fails to
        // clear it, and speech that starts abruptly is missed.
        let mut gate = Gate::default();
        assert!(
            gate.speech(&at(5000)),
            "a shout into a fresh gate is speech, not its own new floor"
        );
    }

    #[test]
    fn loudness_is_the_amplitude_rather_than_the_peak_or_the_sum() {
        // A square wave's root-mean-square is its amplitude, which is the one
        // case with an exact answer to check against.
        let level = loudness(&at(1000));
        assert!(
            (level - 1000.0).abs() < 1.0,
            "a square wave at 1000 must measure 1000, measured {level}"
        );
        assert!(
            loudness(&[]) < f32::EPSILON,
            "no audio has no loudness rather than an undefined one"
        );
    }
}
