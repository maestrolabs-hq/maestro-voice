//! Writing an utterance as a WAV file.
//!
//! The transcription service takes a complete audio file in a multipart form,
//! not a stream of samples, so the recording is wrapped in the smallest
//! container that says what it is: forty-four bytes of canonical header and the
//! samples after it.
//!
//! Hand-rolled rather than taken from a crate, which is the opposite of the
//! choice made for *reading* WAV in the test fixtures. Reading an arbitrary file
//! means coping with whatever a generator wrote, including chunks in an order
//! nobody expected, and that is where a library earns its place. Writing one
//! file whose every field this crate already knows is a header, not a parser.

use crate::capture::SAMPLE_RATE;

/// One channel: capture is mono and stays mono.
const CHANNELS: u16 = 1;

/// Sixteen bits per sample, matching the rest of the crate.
const BITS: u16 = 16;

/// Uncompressed pulse-code modulation.
const PCM: u16 = 1;

/// The bytes of a WAV file holding `samples`.
#[must_use]
pub fn encode(samples: &[i16]) -> Vec<u8> {
    let data_bytes = u32::try_from(samples.len() * 2).unwrap_or(u32::MAX);
    let block_align = CHANNELS * BITS / 8;
    let byte_rate = SAMPLE_RATE * u32::from(block_align);

    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    // Everything after this field: the 36-byte remainder of the header plus the
    // samples.
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&PCM.to_le_bytes());
    out.extend_from_slice(&CHANNELS.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&BITS.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::encode;
    use crate::capture::{SAMPLE_RATE, WavSource};

    fn field(wav: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(wav[at..at + 4].try_into().expect("four bytes"))
    }

    #[test]
    fn the_header_is_forty_four_bytes_and_then_the_samples() {
        let wav = encode(&[1, -1, 2, -2]);
        assert_eq!(wav.len(), 44 + 8, "four samples is eight bytes of audio");
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn the_declared_sizes_agree_with_the_bytes_that_follow_them() {
        // A size field that disagrees with reality is how a reader ends up
        // transcribing silence, or a quarter of a sentence, with no error.
        let wav = encode(&[7; 100]);
        assert_eq!(
            field(&wav, 4) as usize,
            wav.len() - 8,
            "the RIFF size counts everything after itself"
        );
        assert_eq!(
            field(&wav, 40) as usize,
            200,
            "one hundred samples, two bytes each"
        );
        assert_eq!(field(&wav, 24), SAMPLE_RATE, "the rate must be declared");
    }

    /// The round trip that matters: what this writes, this crate's own reader
    /// must read back unchanged. Both sides are ours, and a header nobody reads
    /// back is a header nobody has checked.
    #[test]
    fn what_is_written_reads_back_sample_for_sample() {
        let samples: Vec<i16> = (0..1000).map(|n| (n * 31 % 9000) as i16 - 4500).collect();

        let wav = encode(&samples);
        let source = WavSource::from_bytes(&wav).expect("our own header must parse");

        assert_eq!(
            source.samples(),
            samples.as_slice(),
            "the round trip must not alter a single sample"
        );
    }

    #[test]
    fn an_empty_recording_is_still_a_valid_file() {
        // A wake with nothing after it can reach this with nothing recorded, and
        // a truncated file would be a parse error instead of an empty transcript.
        let wav = encode(&[]);
        let source = WavSource::from_bytes(&wav).expect("an empty file is still a file");
        assert!(source.samples().is_empty());
    }
}
