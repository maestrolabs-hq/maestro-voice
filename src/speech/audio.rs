//! Turning the synthesizer's reply into audio this crate can play.
//!
//! The speech service returns whatever rate its model runs at -- Chatterbox
//! generates at 24 kHz -- and everything above the capture seam is mono at
//! [`crate::capture::SAMPLE_RATE`]. Converting here is what keeps that a
//! crate-wide fact rather than a thing each caller has to remember.
//!
//! This reads a WAV header, which `crate::capture::wav` also does, and the two
//! are deliberately not shared. That one is a *gate*: it refuses anything
//! outside the one shape capture may produce, by name, because a fixture at the
//! wrong rate is a bug to report. This one is an *adapter*: it accepts what the
//! service sends and converts it. Merging them would mean the gate had to stop
//! refusing, which is the whole value of it.

/// Mono, sixteen-bit samples at `rate`, read from a WAV file.
///
/// # Errors
///
/// When the bytes are not a WAV this crate can read: the wrong container, more
/// than one channel, or a sample width other than sixteen bits.
pub fn decode(wav: &[u8]) -> Result<(Vec<i16>, u32), String> {
    if wav.len() < 12 || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err("the reply is not a WAV file".to_owned());
    }

    let mut at = 12;
    let mut format = None;
    while at + 8 <= wav.len() {
        let id = &wav[at..at + 4];
        let size =
            u32::from_le_bytes([wav[at + 4], wav[at + 5], wav[at + 6], wav[at + 7]]) as usize;
        let body = at + 8;
        let end = body.saturating_add(size).min(wav.len());

        if id == b"fmt " && size >= 16 {
            let channels = u16::from_le_bytes([wav[body + 2], wav[body + 3]]);
            let rate =
                u32::from_le_bytes([wav[body + 4], wav[body + 5], wav[body + 6], wav[body + 7]]);
            let bits = u16::from_le_bytes([wav[body + 14], wav[body + 15]]);
            if channels != 1 {
                return Err(format!("{channels} channels; this crate plays mono"));
            }
            if bits != 16 {
                return Err(format!("{bits} bits per sample; sixteen is required"));
            }
            format = Some(rate);
        } else if id == b"data" {
            let rate = format.ok_or_else(|| "the data arrived before the format".to_owned())?;
            let samples = wav[body..end]
                .chunks_exact(2)
                .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            return Ok((samples, rate));
        }

        // Chunks are padded to an even length.
        at = body + size + (size % 2);
    }
    Err("the reply carries no audio".to_owned())
}

/// `samples` at `from` hertz, resampled to `to` hertz.
///
/// Linear interpolation, which is the honest choice for speech being played
/// once through a desk speaker: it is a few lines, it introduces no delay, and
/// what it costs is high-frequency detail nobody listening to a spoken summary
/// will miss. A better resampler is a change to make when something measures
/// this as the problem.
#[must_use]
pub fn resample(samples: &[i16], from: u32, to: u32) -> Vec<i16> {
    if from == to || from == 0 || samples.is_empty() {
        return samples.to_vec();
    }

    let ratio = f64::from(to) / f64::from(from);
    // Lossy only for lengths no speech reaches: an hour of audio is 5.8e7.
    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    let wanted = (samples.len() as f64 * ratio) as usize;

    (0..wanted)
        .map(|n| {
            #[allow(clippy::cast_precision_loss)]
            let source = n as f64 / ratio;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let left = source as usize;
            let right = (left + 1).min(samples.len() - 1);
            let fraction = source - source.floor();

            let blended = f64::from(samples[left])
                .mul_add(1.0 - fraction, f64::from(samples[right]) * fraction);
            #[allow(clippy::cast_possible_truncation)]
            let rounded = blended.round() as i32;
            // Clamped to exactly the range of the target, so the cast below
            // cannot truncate; the lint cannot see that.
            #[allow(clippy::cast_possible_truncation)]
            let bounded = rounded.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            bounded
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{decode, resample};
    use crate::capture::SAMPLE_RATE;
    use crate::runner::wav::encode;

    #[test]
    fn a_wav_this_crate_wrote_reads_back_at_its_own_rate() {
        let samples = vec![0, 1000, -1000, 32767, -32768];
        let (read, rate) = decode(&encode(&samples)).expect("our own encoder must decode");

        assert_eq!(read, samples);
        assert_eq!(rate, SAMPLE_RATE);
    }

    /// The case that made this module exist: the speech service returns the
    /// rate its model runs at, and Chatterbox runs at 24 kHz.
    #[test]
    fn a_reply_at_another_rate_is_read_and_converted() {
        let mut wav = encode(&[500; 2400]);
        // Rewrite the declared rate to 24 kHz, as the service would send it.
        wav[24..28].copy_from_slice(&24_000u32.to_le_bytes());

        let (samples, rate) = decode(&wav).expect("another rate is still readable");
        assert_eq!(rate, 24_000);

        let played = resample(&samples, rate, SAMPLE_RATE);
        assert_eq!(
            played.len(),
            1600,
            "a tenth of a second stays a tenth of a second"
        );
    }

    #[test]
    fn resampling_preserves_how_long_the_audio_lasts() {
        // The failure this prevents: speech played at the wrong rate is not an
        // error, it is a voice that sounds slowed down or sped up.
        for (from, to) in [(24_000, 16_000), (16_000, 24_000), (22_050, 16_000)] {
            let one_second = vec![100; from as usize];
            let converted = resample(&one_second, from, to);
            let drift = converted.len().abs_diff(to as usize);
            assert!(
                drift <= 1,
                "{from} to {to} must stay one second, got {} samples",
                converted.len()
            );
        }
    }

    #[test]
    fn resampling_to_the_same_rate_changes_nothing_at_all() {
        let samples = vec![1, -2, 3, -4];
        assert_eq!(resample(&samples, SAMPLE_RATE, SAMPLE_RATE), samples);
    }

    #[test]
    fn a_steady_tone_keeps_its_level_through_conversion() {
        let flat = vec![8000; 2400];
        let converted = resample(&flat, 24_000, SAMPLE_RATE);

        assert!(
            converted.iter().all(|s| (*s - 8000).abs() <= 1),
            "interpolating a constant must return the constant"
        );
    }

    #[test]
    fn what_is_not_a_wav_is_refused_by_name_rather_than_played_as_noise() {
        for (bytes, expected) in [
            (b"not a wav at all".to_vec(), "not a WAV"),
            (
                encode(&[]).into_iter().take(10).collect::<Vec<_>>(),
                "not a WAV",
            ),
        ] {
            let refusal = decode(&bytes).expect_err("this must be refused");
            assert!(
                refusal.contains(expected),
                "the refusal must say what was wrong, said {refusal:?}"
            );
        }
    }

    #[test]
    fn stereo_is_refused_rather_than_played_at_double_speed() {
        let mut wav = encode(&[0; 100]);
        wav[22..24].copy_from_slice(&2u16.to_le_bytes());

        let refusal = decode(&wav).expect_err("stereo must be refused");
        assert!(refusal.contains("mono"), "said {refusal:?}");
    }
}
