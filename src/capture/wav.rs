//! Reading a RIFF/WAVE file as a chunk source.
//!
//! Hand-written rather than taken from a crate, because the subset this
//! repository needs is small and fixed: uncompressed pulse-code modulation,
//! one channel, sixteen bits, one rate. Everything outside that subset is
//! refused by name instead of resampled, since a file at the wrong rate is
//! almost always a mistake upstream, and silently converting it would hide
//! the mistake rather than report it.
//!
//! This is also the source every test uses, so its refusals are part of the
//! contract rather than defensive noise.

use std::fmt;
use std::fs;
use std::path::Path;

use super::{CHUNK_SAMPLES, Error, SAMPLE_RATE, Source};

/// The size of the `fmt ` body this crate reads, in bytes.
const FMT_FIELDS: usize = 16;

/// A file's samples, handed out one chunk at a time.
pub struct WavSource {
    samples: Vec<i16>,
    next: usize,
}

/// Reports the shape rather than the samples: a failed assertion should print
/// where the source got to, not tens of thousands of numbers.
impl fmt::Debug for WavSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WavSource {{ samples: {}, consumed: {} }}",
            self.samples.len(),
            self.next
        )
    }
}

impl WavSource {
    /// Read a file from disk.
    ///
    /// # Errors
    ///
    /// When the file cannot be read, is not a RIFF/WAVE file, or is not mono
    /// sixteen-bit audio at `SAMPLE_RATE`.
    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::from_bytes(&fs::read(path)?)
    }

    /// Read a file already in memory.
    ///
    /// # Errors
    ///
    /// As [`WavSource::open`], minus the reading.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut rest = riff_body(bytes)?;
        let mut described = false;
        let mut samples = None;

        while rest.len() >= 8 {
            let (id, payload, consumed) = split_chunk(rest)?;
            match &id {
                b"fmt " => {
                    read_format(payload)?;
                    described = true;
                }
                b"data" => samples = Some(decode(payload)),
                _ => {}
            }
            rest = rest.get(consumed..).unwrap_or(&[]);
        }

        if !described {
            return Err(Error::Malformed("no 'fmt ' chunk".to_owned()));
        }
        let Some(samples) = samples else {
            return Err(Error::Malformed("no 'data' chunk".to_owned()));
        };
        Ok(Self { samples, next: 0 })
    }

    /// Every sample in the file, in order.
    #[must_use]
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }
}

impl Source for WavSource {
    fn next_chunk(&mut self) -> Result<Option<Vec<i16>>, Error> {
        if self.next >= self.samples.len() {
            return Ok(None);
        }
        let end = (self.next + CHUNK_SAMPLES).min(self.samples.len());
        let chunk = self.samples[self.next..end].to_vec();
        self.next = end;
        Ok(Some(chunk))
    }
}

/// The bytes between the `RIFF`/`WAVE` wrapper and the end of the file.
fn riff_body(bytes: &[u8]) -> Result<&[u8], Error> {
    if bytes.len() < 12 {
        return Err(Error::Malformed(format!(
            "only {} bytes, too short to hold a RIFF header",
            bytes.len()
        )));
    }
    if &bytes[0..4] != b"RIFF" {
        return Err(Error::Malformed(
            "does not begin with 'RIFF', so it is not a WAV file".to_owned(),
        ));
    }
    if &bytes[8..12] != b"WAVE" {
        return Err(Error::Malformed(
            "RIFF file is not of the 'WAVE' form".to_owned(),
        ));
    }
    Ok(&bytes[12..])
}

/// Check one `fmt ` body describes audio this crate can use.
fn read_format(payload: &[u8]) -> Result<(), Error> {
    if payload.len() < FMT_FIELDS {
        return Err(Error::Malformed("the 'fmt ' chunk is truncated".to_owned()));
    }
    let encoding = u16::from_le_bytes([payload[0], payload[1]]);
    let channels = u16::from_le_bytes([payload[2], payload[3]]);
    let rate = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let bits = u16::from_le_bytes([payload[14], payload[15]]);

    if encoding != 1 {
        return Err(Error::Unsupported(format!(
            "encoding {encoding} is compressed; only uncompressed audio is read"
        )));
    }
    if channels != 1 {
        return Err(Error::Unsupported(format!(
            "{channels} channels; only one channel is read"
        )));
    }
    if rate != SAMPLE_RATE {
        return Err(Error::Unsupported(format!(
            "{rate} samples per second; {SAMPLE_RATE} is required"
        )));
    }
    if bits != 16 {
        return Err(Error::Unsupported(format!(
            "{bits} bits per sample; 16 is required"
        )));
    }
    Ok(())
}

/// Little-endian pairs to samples, dropping a trailing odd byte.
fn decode(payload: &[u8]) -> Vec<i16> {
    payload
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}

/// One chunk: its four-character identifier, its payload, and how many bytes
/// of the input it occupied including its header and any pad byte.
///
/// A chunk claiming more bytes than remain is truncation, and it is an error
/// rather than a short read. A `data` chunk read short is a recording that
/// quietly lost its ending, which is exactly the class of failure this
/// repository refuses to produce.
fn split_chunk(rest: &[u8]) -> Result<([u8; 4], &[u8], usize), Error> {
    let id = [rest[0], rest[1], rest[2], rest[3]];
    let declared = u32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]);
    let size = usize::try_from(declared).map_err(|_| {
        Error::Malformed(format!("a chunk declares {declared} bytes, more than fit"))
    })?;

    let Some(payload) = rest.get(8..8 + size) else {
        return Err(Error::Malformed(format!(
            "the '{}' chunk declares {size} bytes but only {} follow, so the file is truncated",
            String::from_utf8_lossy(&id).trim_end(),
            rest.len() - 8
        )));
    };
    // Chunks are padded to an even length; the pad byte is not payload.
    Ok((id, payload, 8 + size + size % 2))
}
