//! Capture behaviour exercised without a capture device.
//!
//! Every test here builds its audio from bytes written to the RIFF/WAVE
//! specification rather than recorded, which is the point of
//! `docs/adr/0001-the-capture-source-is-a-seam.md`: the path from samples to
//! frames must be reachable in continuous integration, where no microphone
//! exists.
//!
//! The header is assembled field by field rather than by a writer in this
//! crate on purpose. A writer and a reader that share an author share their
//! misreadings, and a test built on the reader's own idea of the format would
//! agree with it however wrong it was.

use maestro_voice::capture::{CHUNK_SAMPLES, SAMPLE_RATE, Source, WavSource};

/// A RIFF/WAVE file carrying `samples`, written from the specification.
///
/// Sizes are little-endian, `fmt ` describes uncompressed pulse-code
/// modulation, and `data` is the samples themselves.
fn wav(channels: u16, rate: u32, bits: u16, samples: &[i16]) -> Vec<u8> {
    let block_align = channels * bits / 8;
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend(
        u32::try_from(36 + data.len())
            .expect("fixture fits")
            .to_le_bytes(),
    );
    out.extend(b"WAVE");

    out.extend(b"fmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * u32::from(block_align)).to_le_bytes());
    out.extend(block_align.to_le_bytes());
    out.extend(bits.to_le_bytes());

    out.extend(b"data");
    out.extend(
        u32::try_from(data.len())
            .expect("fixture fits")
            .to_le_bytes(),
    );
    out.extend(data);
    out
}

/// A mono 16 kHz 16-bit file, which is the only shape this crate accepts.
fn usable(samples: &[i16]) -> Vec<u8> {
    wav(1, SAMPLE_RATE, 16, samples)
}

/// A ramp, so a dropped or duplicated sample changes the values and not only
/// the count.
fn ramp(count: usize) -> Vec<i16> {
    (0..count)
        .map(|i| i16::try_from(i % 1000).expect("under i16"))
        .collect()
}

fn drain(source: &mut impl Source) -> Vec<Vec<i16>> {
    let mut chunks = Vec::new();
    while let Some(chunk) = source.next_chunk().expect("a well-formed fixture reads") {
        chunks.push(chunk);
    }
    chunks
}

#[test]
fn whole_chunks_carry_exactly_the_promised_sample_count() {
    let samples = ramp(CHUNK_SAMPLES * 3);
    let mut source = WavSource::from_bytes(&usable(&samples)).expect("a usable fixture opens");

    let chunks = drain(&mut source);

    assert_eq!(chunks.len(), 3, "three chunks of audio, three chunks out");
    for (n, chunk) in chunks.iter().enumerate() {
        assert_eq!(
            chunk.len(),
            CHUNK_SAMPLES,
            "chunk {n} must be exactly one hop of audio"
        );
    }
    assert_eq!(
        chunks.concat(),
        samples,
        "the samples must survive chunking unchanged and in order"
    );
}

#[test]
fn a_final_partial_chunk_is_reported_short_rather_than_padded() {
    // Padding a short tail with silence would hand the wake word a hop that
    // never happened, and it would look exactly like real audio.
    let samples = ramp(CHUNK_SAMPLES + 7);
    let mut source = WavSource::from_bytes(&usable(&samples)).expect("a usable fixture opens");

    let chunks = drain(&mut source);

    assert_eq!(chunks.len(), 2, "one whole chunk and one remainder");
    assert_eq!(
        chunks[1].len(),
        7,
        "the remainder must keep its true length"
    );
    assert_eq!(chunks.concat(), samples, "and must not invent samples");
}

#[test]
fn an_exhausted_source_keeps_saying_it_is_exhausted() {
    let mut source = WavSource::from_bytes(&usable(&ramp(4))).expect("a usable fixture opens");
    let _ = drain(&mut source);

    assert!(
        source
            .next_chunk()
            .expect("reading past the end is not an error")
            .is_none(),
        "a drained source must stay drained rather than restarting"
    );
}

#[test]
fn a_wav_at_the_wrong_sample_rate_is_refused_and_names_its_rate() {
    let bytes = wav(1, 44_100, 16, &ramp(64));

    let refusal = WavSource::from_bytes(&bytes).expect_err("44.1 kHz must be refused");

    let message = refusal.to_string();
    assert!(
        message.contains("44100") && message.contains(&SAMPLE_RATE.to_string()),
        "the refusal must say what it got and what it needs, got: {message}"
    );
}

#[test]
fn a_stereo_wav_is_refused_and_says_so() {
    let bytes = wav(2, SAMPLE_RATE, 16, &ramp(64));

    let refusal = WavSource::from_bytes(&bytes).expect_err("two channels must be refused");

    assert!(
        refusal.to_string().contains("channel"),
        "the refusal must name the channel count as the problem, got: {refusal}"
    );
}

#[test]
fn an_eight_bit_wav_is_refused_and_says_so() {
    let bytes = wav(1, SAMPLE_RATE, 8, &ramp(64));

    let refusal = WavSource::from_bytes(&bytes).expect_err("eight-bit audio must be refused");

    assert!(
        refusal.to_string().contains("bit"),
        "the refusal must name the sample width, got: {refusal}"
    );
}

#[test]
fn bytes_that_are_not_a_wav_at_all_are_refused() {
    let refusal = WavSource::from_bytes(b"this is not audio").expect_err("garbage must be refused");

    assert!(
        refusal.to_string().contains("RIFF"),
        "the refusal must say what it looked for, got: {refusal}"
    );
}

#[test]
fn a_wav_whose_data_chunk_is_truncated_is_refused() {
    // The header promises more samples than the file carries. Reading it as
    // though it were complete would silently shorten the recording.
    let mut bytes = usable(&ramp(64));
    bytes.truncate(bytes.len() - 40);

    let refusal = WavSource::from_bytes(&bytes).expect_err("a short file must be refused");

    assert!(
        refusal.to_string().contains("truncated"),
        "the refusal must name truncation, got: {refusal}"
    );
}

#[test]
fn chunks_that_are_not_understood_are_skipped_rather_than_refused() {
    // Recorders routinely add LIST or fact chunks. Refusing a file for
    // carrying metadata would reject ordinary recordings.
    let samples = ramp(32);
    let mut bytes = usable(&samples);
    let mut with_extra: Vec<u8> = bytes.drain(..12).collect();
    with_extra.extend(b"LIST");
    with_extra.extend(4u32.to_le_bytes());
    with_extra.extend(b"INFO");
    with_extra.extend(bytes);
    // The RIFF size field is now understated, which readers tolerate.

    let mut source = WavSource::from_bytes(&with_extra).expect("metadata must not refuse a file");

    assert_eq!(
        drain(&mut source).concat(),
        samples,
        "the audio must survive an unknown chunk sitting in front of it"
    );
}
