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

use maestro_voice::capture::pulse::{self, Direction, PulseSource};
use maestro_voice::capture::{
    Attempt, CHUNK_SAMPLES, Error as CaptureError, Reopen, Restart, Ring, SAMPLE_RATE, Source,
    Supervised, Wait, WavSource, samples_in,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

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
fn a_wav_outside_the_readable_shape_is_refused_and_names_the_reason() {
    // Each refusal has to say which property was wrong. "unsupported audio" on
    // its own sends someone reading a hex dump.
    for (bytes, named, what) in [
        (
            wav(2, SAMPLE_RATE, 16, &ramp(64)),
            "channel",
            "two channels",
        ),
        (
            wav(1, SAMPLE_RATE, 8, &ramp(64)),
            "bit",
            "eight-bit samples",
        ),
    ] {
        let refusal = WavSource::from_bytes(&bytes)
            .map(|_| ())
            .expect_err(&format!("{what} must be refused"));

        assert!(
            refusal.to_string().contains(named),
            "a file with {what} must be refused for that reason, got: {refusal}"
        );
    }
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
fn a_ring_keeps_only_the_most_recent_audio_it_has_room_for() {
    let mut ring = Ring::holding(Duration::from_millis(500));
    let capacity = samples_in(Duration::from_millis(500));

    // Three times its capacity goes in; only the last capacity-worth stays.
    let written = ramp(capacity * 3);
    for chunk in written.chunks(CHUNK_SAMPLES) {
        ring.write(chunk);
    }

    assert_eq!(
        ring.len(),
        capacity,
        "a ring must not grow past its capacity"
    );
    assert_eq!(
        ring.pre_roll(Duration::from_millis(500)),
        written[written.len() - capacity..],
        "what remains must be the newest audio, not the oldest"
    );
}

#[test]
fn the_pre_roll_is_the_tail_of_the_audio_and_keeps_its_order() {
    // A wake word fires at the END of the phrase, so the sentence that follows
    // has already begun. The pre-roll is how the first word is not clipped.
    let mut ring = Ring::holding(Duration::from_secs(2));
    let written = ramp(samples_in(Duration::from_secs(1)));
    ring.write(&written);

    let roll = ring.pre_roll(Duration::from_millis(300));

    assert_eq!(roll.len(), samples_in(Duration::from_millis(300)));
    assert_eq!(roll.len(), 4800, "300ms at 16 kHz is 4800 samples");
    assert_eq!(
        roll,
        written[written.len() - 4800..],
        "the pre-roll must be the most recent audio, in the order it arrived"
    );
}

#[test]
fn a_pre_roll_longer_than_the_audio_returns_what_exists_rather_than_silence() {
    // Waking two hundred milliseconds after the daemon starts must not
    // manufacture the second of audio that never happened.
    let mut ring = Ring::holding(Duration::from_secs(2));
    let written = ramp(1000);
    ring.write(&written);

    let roll = ring.pre_roll(Duration::from_secs(1));

    assert_eq!(roll, written, "short is honest; padded silence is not");
}

#[test]
fn a_ring_with_no_audio_yields_an_empty_pre_roll() {
    let ring = Ring::holding(Duration::from_secs(2));

    assert!(ring.is_empty());
    assert!(ring.pre_roll(Duration::from_millis(300)).is_empty());
}

#[test]
fn a_pre_roll_of_no_duration_yields_nothing() {
    let mut ring = Ring::holding(Duration::from_secs(2));
    ring.write(&ramp(1000));

    assert!(
        ring.pre_roll(Duration::ZERO).is_empty(),
        "asking for no audio must return no audio, not everything"
    );
}

#[test]
fn a_single_write_larger_than_the_ring_keeps_its_tail() {
    let mut ring = Ring::holding(Duration::from_millis(100));
    let capacity = samples_in(Duration::from_millis(100));
    let written = ramp(capacity * 2 + 13);

    ring.write(&written);

    assert_eq!(ring.len(), capacity);
    assert_eq!(
        ring.pre_roll(Duration::from_millis(100)),
        written[written.len() - capacity..],
        "one oversized write must behave like many small ones"
    );
}

/// The policy this repository runs with: three tries, starting at 200ms.
fn policy() -> Restart {
    Restart::allowing(3, Duration::from_millis(200))
}

/// A device that is simply gone.
struct NeverOpens;

impl Reopen for NeverOpens {
    fn reopen(&mut self) -> Result<Box<dyn Source>, CaptureError> {
        Err(CaptureError::Stopped("the device is gone".to_owned()))
    }
}

/// A source that dies partway through, then cannot be reopened.
struct DiesAfter(Option<Vec<i16>>);

impl Reopen for DiesAfter {
    fn reopen(&mut self) -> Result<Box<dyn Source>, CaptureError> {
        match self.0.take() {
            Some(samples) => Ok(Box::new(WavSource::from_bytes(&usable(&samples))?)),
            None => Err(CaptureError::Stopped("and now it is gone".to_owned())),
        }
    }
}

/// Records what it was asked to wait for instead of waiting, so the backoff
/// schedule is observable and the test takes microseconds.
#[derive(Clone, Default)]
struct Recorded(Rc<RefCell<Vec<Duration>>>);

impl Wait for Recorded {
    fn wait(&mut self, how_long: Duration) {
        self.0.borrow_mut().push(how_long);
    }
}

#[test]
fn a_capture_that_never_reopens_gives_up_and_says_why_once() {
    let waits = Recorded::default();
    let mut capture = Supervised::new(NeverOpens, policy(), waits.clone());

    let refusal = capture
        .next_chunk()
        .map(|_| ())
        .expect_err("a device that never opens cannot produce audio");

    let message = refusal.to_string();
    assert!(
        message.contains('3') && message.contains("gone"),
        "the message must say how many tries and what the last failure was, got: {message}"
    );
    assert_eq!(
        waits.0.borrow().as_slice(),
        [Duration::from_millis(200), Duration::from_millis(400)],
        "it must wait between tries, and longer each time, before giving up"
    );
}

#[test]
fn a_capture_that_dies_partway_is_reopened_rather_than_ending_the_stream() {
    // A source running out is not the end of the audio: microphones do not
    // end. It has to be treated as a failure and reopened.
    let waits = Recorded::default();
    let mut capture = Supervised::new(DiesAfter(Some(ramp(64))), policy(), waits.clone());

    let first = capture.next_chunk().expect("the first open works");
    assert_eq!(first.map(|c| c.len()), Some(64), "the audio it did have");

    let refusal = capture
        .next_chunk()
        .map(|_| ())
        .expect_err("the exhausted source cannot be reopened");

    assert!(refusal.to_string().contains("gone"), "got: {refusal}");
}

#[test]
fn audio_arriving_forgives_the_failures_before_it() {
    // Two hiccups then success must not leave the daemon one hiccup away from
    // stopping for the rest of the day.
    let mut restart = policy();
    let _ = restart.failed();
    let waits = Recorded::default();
    let mut capture = Supervised::new(DiesAfter(Some(ramp(64))), restart, waits.clone());

    let _ = capture.next_chunk().expect("the first open works");

    assert_eq!(
        capture.failures(),
        0,
        "a chunk of real audio must clear the count"
    );
}

#[test]
fn capture_gives_up_after_three_consecutive_failures() {
    // Driven by counting failures rather than by breaking ffmpeg: a policy
    // that can only be exercised by killing a real process is a policy that
    // never gets exercised.
    let mut restart = policy();

    assert!(
        matches!(restart.failed(), Attempt::Retry(_)),
        "first retries"
    );
    assert!(
        matches!(restart.failed(), Attempt::Retry(_)),
        "second retries"
    );
    assert_eq!(
        restart.failed(),
        Attempt::GiveUp,
        "the third consecutive failure must stop, not retry forever"
    );
}

#[test]
fn a_capture_that_recovers_gets_its_full_allowance_again() {
    // A microphone that hiccups twice an hour must not accumulate its way to
    // a permanent stop over a long day.
    let mut restart = policy();
    let _ = restart.failed();
    let _ = restart.failed();

    restart.succeeded();

    assert!(
        matches!(restart.failed(), Attempt::Retry(_)),
        "a success in between must clear the count"
    );
}

#[test]
fn the_wait_grows_between_attempts() {
    // A device that is gone stays gone for a while; retrying at full speed
    // burns a core and fills the log.
    let mut restart = policy();

    let Attempt::Retry(first) = restart.failed() else {
        panic!("expected a retry");
    };
    let Attempt::Retry(second) = restart.failed() else {
        panic!("expected a retry");
    };

    assert_eq!(first, Duration::from_millis(200));
    assert!(second > first, "the second wait must exceed the first");
}

#[test]
fn a_policy_that_allows_nothing_gives_up_immediately() {
    let mut restart = Restart::allowing(0, Duration::from_millis(200));

    assert_eq!(
        restart.failed(),
        Attempt::GiveUp,
        "allowing no attempts must not be read as allowing endless ones"
    );
}

#[test]
fn a_wav_on_disk_opens_and_yields_the_samples_playback_takes() {
    // Playing a file is these two halves composed: read it, hand the samples
    // to the sink. Both halves are exercised here; only the sink is not.
    let samples = ramp(2048);
    let path = std::env::temp_dir().join("maestro-voice-capture-open.wav");
    std::fs::write(&path, usable(&samples)).expect("the temp directory is writable");

    let source = WavSource::open(&path).expect("a usable file opens from disk");

    assert_eq!(
        source.samples(),
        samples,
        "what came off disk must be what playback would be handed"
    );
    std::fs::remove_file(&path).expect("clean up");
}

#[test]
fn a_file_that_is_not_there_is_refused_rather_than_fatal() {
    let missing = std::env::temp_dir().join("maestro-voice-capture-absent.wav");
    let _ = std::fs::remove_file(&missing);

    assert!(
        WavSource::open(&missing).is_err(),
        "a missing file must be an error, not a panic"
    );
}

#[test]
fn no_device_name_for_this_machine_is_written_into_the_code() {
    // The survey found exactly one capture source on this host, called
    // RDPSource, and it is the default. Naming it here would encode one
    // machine's answer; asking the audio server for its default does not.
    assert_eq!(pulse::device_or_default(None), "default");
    assert_eq!(pulse::device_or_default(Some("RDPSource")), "RDPSource");
    assert_eq!(
        pulse::device_or_default(Some("")),
        "default",
        "an empty override is not a device name"
    );
}

#[test]
fn the_capture_command_asks_for_exactly_the_format_the_crate_reads() {
    // Every consumer above assumes mono sixteen-bit little-endian audio at the
    // crate's rate. If ffmpeg is asked for anything else, the samples are
    // silently misinterpreted rather than refused.
    let line = pulse::args(Direction::Capture, "RDPSource").join(" ");

    assert!(line.contains("-f pulse"), "reads the audio server: {line}");
    assert!(line.contains("-i RDPSource"), "from the device: {line}");
    assert!(line.contains("-ac 1"), "one channel: {line}");
    assert!(
        line.contains(&format!("-ar {SAMPLE_RATE}")),
        "at the crate's rate: {line}"
    );
    assert!(line.contains("-f s16le"), "raw little-endian out: {line}");
    assert!(line.ends_with(" -"), "to standard output: {line}");
}

#[test]
fn the_playback_command_declares_the_format_it_is_being_given() {
    // Playback reads raw samples with no header, so the format has to be
    // stated on the way in or ffmpeg guesses and the tone comes out wrong.
    let line = pulse::args(Direction::Playback, "RDPSink").join(" ");

    assert!(line.contains("-f s16le"), "raw little-endian in: {line}");
    assert!(line.contains("-ac 1"), "one channel: {line}");
    assert!(
        line.contains(&format!("-ar {SAMPLE_RATE}")),
        "at the crate's rate: {line}"
    );
    assert!(line.contains("-i -"), "from standard input: {line}");
    assert!(line.contains("-f pulse"), "to the audio server: {line}");
    assert!(line.ends_with("RDPSink"), "to the sink: {line}");
}

#[test]
fn a_missing_capture_program_says_what_it_looked_for_and_does_not_panic() {
    // A device problem must never take the daemon down, and the message has to
    // name the thing to install rather than report an error number.
    let refusal = PulseSource::open("maestro-voice-no-such-program", "default")
        .expect_err("a program that does not exist cannot open");

    let message = refusal.to_string();
    assert!(
        message.contains("maestro-voice-no-such-program"),
        "the message must name the program it looked for, got: {message}"
    );
    assert!(
        message.contains("search path"),
        "and say where it looked, got: {message}"
    );
}

#[test]
fn playing_through_a_missing_program_is_refused_rather_than_fatal() {
    let refusal = pulse::play("maestro-voice-no-such-program", "default", &[0, 1, 2])
        .expect_err("a program that does not exist cannot play");

    assert!(
        refusal
            .to_string()
            .contains("maestro-voice-no-such-program"),
        "got: {refusal}"
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
