//! Measuring how loud a chunk is.
//!
//! One function, separated because the module beside it is about a decision and
//! this is about arithmetic. It is also the number `check` reports, so keeping
//! it nameable on its own is worth a file.

/// The root-mean-square amplitude of a chunk.
///
/// Root-mean-square rather than peak: a single click reaches full scale and says
/// nothing about whether anyone is talking, while the mean square is what a
/// listener would call loudness.
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

#[cfg(test)]
mod tests {
    use super::loudness;
    use crate::capture::CHUNK_SAMPLES;

    /// A square wave at a constant amplitude.
    fn at(amplitude: i16) -> Vec<i16> {
        (0..CHUNK_SAMPLES)
            .map(|n| if n % 2 == 0 { amplitude } else { -amplitude })
            .collect()
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

    #[test]
    fn a_single_click_does_not_read_as_loud_as_a_voice() {
        // Why root-mean-square and not peak: both of these touch full scale.
        let mut click = vec![0i16; CHUNK_SAMPLES];
        click[0] = i16::MAX;

        assert!(
            loudness(&click) < loudness(&at(i16::MAX / 4)),
            "one sample at full scale must not outweigh a sustained voice"
        );
    }
}
