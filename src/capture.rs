//! Where audio comes from, and what it looks like once it arrives.
//!
//! Everything above this module sees one thing: a stream of fixed-size chunks
//! of mono sixteen-bit samples at a known rate. Underneath, that stream is an
//! `ffmpeg` child reading the system audio server in production and a file
//! under test. Which one is running is not a fact the wake word or the
//! endpointing rule is allowed to depend on, and
//! `docs/adr/0001-the-capture-source-is-a-seam.md` records why.
//!
//! The chunk size is not arbitrary. The wake-word pipeline advances in hops of
//! this many samples, so a source that emitted a different size would push the
//! resampling problem into every consumer.

use std::fmt;
use std::io;
use std::time::Duration;

pub mod ring;
pub mod wav;

pub use ring::Ring;
pub use wav::WavSource;

/// Samples per second. Every source in this crate produces this rate, and one
/// that cannot is refused rather than resampled.
pub const SAMPLE_RATE: u32 = 16_000;

/// Samples in one chunk: eighty milliseconds at `SAMPLE_RATE`.
pub const CHUNK_SAMPLES: usize = 1280;

/// Bytes in one chunk, as they arrive on a pipe.
pub const CHUNK_BYTES: usize = CHUNK_SAMPLES * 2;

/// How many samples `duration` holds at `SAMPLE_RATE`.
///
/// Durations are how the rest of the crate talks about audio, because a window
/// of three hundred milliseconds means something to a reader and a window of
/// four thousand eight hundred samples does not. This is the one place the two
/// are converted.
#[must_use]
pub fn samples_in(duration: Duration) -> usize {
    let exact = duration.as_nanos() * u128::from(SAMPLE_RATE) / 1_000_000_000;
    usize::try_from(exact).unwrap_or(usize::MAX)
}

/// A stream of audio chunks.
///
/// A chunk holds `CHUNK_SAMPLES` samples except possibly the last, which is
/// reported at its true length rather than padded. Padding would hand a
/// consumer a hop of audio that was never spoken, and it would be
/// indistinguishable from the real thing.
pub trait Source {
    /// The next chunk, or `None` once the source is exhausted.
    ///
    /// # Errors
    ///
    /// When the underlying file, pipe or process fails. A source that has
    /// ended cleanly returns `Ok(None)`, which is not an error; a source that
    /// stopped when it should not have returns `Err`.
    fn next_chunk(&mut self) -> Result<Option<Vec<i16>>, Error>;
}

/// What can go wrong between the device and a chunk.
#[derive(Debug)]
pub enum Error {
    /// The bytes are not the format they claim, or they stop mid-structure.
    Malformed(String),
    /// Readable, but not a shape this crate can use.
    Unsupported(String),
    /// A helper program is not installed.
    Missing(String),
    /// A helper program stopped when it was expected to keep running.
    Stopped(String),
    /// The operating system refused something.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(detail) => write!(f, "malformed audio: {detail}"),
            Self::Unsupported(detail) => write!(f, "unsupported audio: {detail}"),
            Self::Missing(detail) => write!(f, "{detail}"),
            Self::Stopped(detail) => write!(f, "capture stopped: {detail}"),
            Self::Io(error) => write!(f, "audio input or output failed: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::{CHUNK_BYTES, CHUNK_SAMPLES, Error, SAMPLE_RATE};
    use std::io;

    #[test]
    fn a_chunk_is_eighty_milliseconds_of_audio() {
        // The wake-word pipeline advances in eighty-millisecond hops. If this
        // ever stops being true, every consumer's framing is wrong.
        let samples = u32::try_from(CHUNK_SAMPLES).expect("a chunk fits in a word");
        assert_eq!(
            samples * 1000 / SAMPLE_RATE,
            80,
            "one chunk must be one hop"
        );
        assert_eq!(CHUNK_BYTES, CHUNK_SAMPLES * 2, "sixteen bits per sample");
    }

    #[test]
    fn a_duration_converts_to_the_sample_count_it_holds() {
        use super::samples_in;
        use std::time::Duration;

        assert_eq!(samples_in(Duration::from_secs(1)), 16_000);
        assert_eq!(samples_in(Duration::from_millis(300)), 4_800);
        assert_eq!(samples_in(Duration::ZERO), 0);
        // Sub-sample durations round down: a partial sample cannot be held.
        assert_eq!(samples_in(Duration::from_nanos(1)), 0);
    }

    #[test]
    fn a_missing_program_reads_as_its_own_message_with_nothing_prepended() {
        // The message names what to install, so a prefix like "error:" would
        // only get in the way of the one sentence that helps.
        let error = Error::Missing("'ffmpeg' was not found".to_owned());
        assert_eq!(error.to_string(), "'ffmpeg' was not found");
    }

    #[test]
    fn an_operating_system_failure_keeps_its_cause_reachable() {
        use std::error::Error as _;
        let error = Error::from(io::Error::other("pipe closed"));
        assert!(
            error.source().is_some(),
            "the underlying cause must survive conversion"
        );
    }
}
