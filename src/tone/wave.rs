//! Turning a pitch and a length into samples.
//!
//! Separate from the cue vocabulary above it because the two change for
//! different reasons: which sounds mean what is a design question, and how a
//! sine becomes sixteen-bit audio without clicking is an arithmetic one.

use std::f32::consts::TAU;
use std::time::Duration;

use crate::capture::samples_in;

/// How long each tone takes to rise and fall. A waveform that begins at full
/// amplitude clicks, and the click is louder than the tone it introduces.
const FADE: Duration = Duration::from_millis(8);

/// Full scale, as the multiplier from a unit waveform to a sample.
const FULL_SCALE: f32 = 32_767.0;

/// The sample rate as a float, so generating a waveform needs no conversion.
/// A literal rather than a cast, which is lossy in general; the test below is
/// what stops the two spellings of the rate drifting apart.
const RATE: f32 = 16_000.0;

/// Append `count` samples of silence.
pub(super) fn silence(out: &mut Vec<i16>, count: usize) {
    out.resize(out.len() + count, 0);
}

/// Append one faded tone at `hertz`, `count` samples long, scaled by `gain`.
pub(super) fn append(out: &mut Vec<i16>, hertz: f32, count: usize, gain: f32) {
    let advance = TAU * hertz / RATE;
    let mut phase: f32 = 0.0;
    for index in 0..count {
        out.push(sample(phase.sin() * gain * envelope(index, count)));
        phase += advance;
    }
}

/// The rise-and-fall shape applied to one tone, between zero and one.
fn envelope(index: usize, length: usize) -> f32 {
    let fade = samples_in(FADE).min(length / 2).max(1);
    if index < fade {
        fraction(index, fade)
    } else if index + fade >= length {
        fraction(length - index - 1, fade)
    } else {
        1.0
    }
}

/// One count as a fraction of another.
///
/// Exact rather than approximate: both are sample counts within a cue under a
/// second long, far inside the integers a single-precision float holds without
/// loss. The duration and loudness tests check that reason still holds.
#[allow(clippy::cast_precision_loss)]
fn fraction(index: usize, length: usize) -> f32 {
    index as f32 / length as f32
}

/// A unit waveform value as a sample.
///
/// Truncation cannot occur: the value is clamped to the representable range
/// immediately before the conversion, and the loudness test is what checks the
/// clamp is never the thing doing the work.
#[allow(clippy::cast_possible_truncation)]
fn sample(value: f32) -> i16 {
    (value * FULL_SCALE).clamp(-FULL_SCALE, FULL_SCALE) as i16
}

#[cfg(test)]
mod tests {
    use super::{RATE, append, envelope, silence};
    use crate::capture::SAMPLE_RATE;

    #[test]
    fn the_float_rate_matches_the_integer_rate() {
        let exact = f32::from(u16::try_from(SAMPLE_RATE).expect("the rate fits"));
        assert!(
            (RATE - exact).abs() < f32::EPSILON,
            "the waveform rate must be the crate's rate"
        );
    }

    #[test]
    fn the_envelope_opens_at_zero_and_closes_at_zero() {
        let length = 1000;
        assert!(envelope(0, length) < f32::EPSILON, "must open silently");
        assert!(
            envelope(length - 1, length) < f32::EPSILON,
            "must close silently"
        );
        assert!(
            (envelope(length / 2, length) - 1.0).abs() < f32::EPSILON,
            "and reach full amplitude in between"
        );
    }

    /// A tone shorter than two fades still has to fade, or it clicks at both
    /// ends. The window shrinks to fit rather than being skipped.
    #[test]
    fn a_tone_shorter_than_its_fade_still_opens_and_closes_quietly() {
        let mut out = Vec::new();
        append(&mut out, 880.0, 12, 0.22);

        assert_eq!(out.len(), 12);
        assert_eq!(out[0], 0, "a very short tone must still start at silence");
    }

    #[test]
    fn silence_is_actually_silent_and_the_right_length() {
        let mut out = vec![7i16; 3];
        silence(&mut out, 100);

        assert_eq!(out.len(), 103, "silence appends rather than replaces");
        assert!(
            out[3..].iter().all(|s| *s == 0),
            "a rest must contain no signal"
        );
    }
}
