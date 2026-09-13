//! Capture and playback through the system audio server, via `ffmpeg`.
//!
//! A subprocess rather than a bound audio library, for the reason
//! `docs/adr/0001-the-capture-source-is-a-seam.md` records: the seam this sits
//! behind is what makes wake-word and endpointing behaviour testable without a
//! device, and it keeps the least portable code in the crate in one file.
//!
//! No device name appears here. This host has exactly one capture source and
//! it happens to be the audio server's default, but that is a fact about this
//! host. Asking for the default is portable; naming `RDPSource` would be a
//! machine's answer written into a program.

use std::io::{Read, Write};
use std::process::{Child, ChildStdout, Command, Stdio};

use super::{CHUNK_BYTES, Error, SAMPLE_RATE, Source};

/// What the audio server calls the device it would pick itself.
const DEFAULT_DEVICE: &str = "default";

/// The program that moves audio between this process and the audio server.
pub const PROGRAM: &str = "ffmpeg";

/// The configured device, or the audio server's own default.
///
/// An empty override is treated as absent: a configuration file with a blank
/// value means the operator did not choose, not that the device has no name.
#[must_use]
pub fn device_or_default(configured: Option<&str>) -> &str {
    match configured {
        Some(name) if !name.trim().is_empty() => name,
        _ => DEFAULT_DEVICE,
    }
}

/// Which way audio is moving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// From the audio server into this process.
    Capture,
    /// From this process out to the audio server.
    Playback,
}

/// The `ffmpeg` arguments for moving audio in `direction` against `device`.
///
/// One function rather than two, because the part that must never differ
/// between them is the middle: capture that reads one channel at one rate and
/// playback that writes another would be a mismatch nothing reports. Written
/// twice, that agreement is a thing to remember; written once, it is a fact.
///
/// The format is demanded rather than accepted in both directions. Raw samples
/// carry no header, so audio in another shape is misread rather than refused.
#[must_use]
pub fn args(direction: Direction, device: &str) -> Vec<String> {
    let rate = SAMPLE_RATE.to_string();
    let mut out = vec!["-hide_banner", "-loglevel", "error"];

    match direction {
        Direction::Capture => out.extend(["-f", "pulse", "-i", device]),
        Direction::Playback => out.extend(["-f", "s16le"]),
    }
    out.extend(["-ac", "1", "-ar", &rate]);
    match direction {
        Direction::Capture => out.extend(["-f", "s16le", "-"]),
        Direction::Playback => out.extend(["-i", "-", "-f", "pulse", device]),
    }

    out.iter().map(|argument| (*argument).to_owned()).collect()
}

/// Start `program`, mapping a missing binary to a message that names it.
fn start(program: &str, args: &[String], stdin: Stdio, stdout: Stdio) -> Result<Child, Error> {
    Command::new(program)
        .args(args)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::Missing(format!(
                    "'{program}' was not found on the search path, and it is how this \
                     program reaches the microphone and the speaker; install it, or \
                     name another one in configuration"
                ))
            } else {
                Error::Io(error)
            }
        })
}

/// Audio arriving from the system audio server.
pub struct PulseSource {
    child: Child,
    output: ChildStdout,
}

/// Reports which child is producing the audio, not the audio itself.
impl std::fmt::Debug for PulseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PulseSource {{ process: {} }}", self.child.id())
    }
}

impl PulseSource {
    /// Start capturing from `device`.
    ///
    /// # Errors
    ///
    /// When `program` is not installed, or the operating system refuses to
    /// start it. A device that does not exist is not detected here: `ffmpeg`
    /// exits once running, which surfaces as a stopped capture on first read.
    pub fn open(program: &str, device: &str) -> Result<Self, Error> {
        let arguments = args(Direction::Capture, device);
        let mut child = start(program, &arguments, Stdio::null(), Stdio::piped())?;
        let Some(output) = child.stdout.take() else {
            return Err(Error::Stopped(format!("'{program}' gave no output stream")));
        };
        Ok(Self { child, output })
    }

    /// Why the child is no longer producing audio.
    fn stopped(&mut self) -> Error {
        let status = match self.child.try_wait() {
            Ok(Some(status)) => status.to_string(),
            Ok(None) => "it is still running but sent no audio".to_owned(),
            Err(error) => format!("its state could not be read: {error}"),
        };
        Error::Stopped(format!("the capture process ended: {status}"))
    }
}

impl Source for PulseSource {
    /// The next chunk, or an error.
    ///
    /// Never `Ok(None)`: a microphone does not reach an end, so a stream that
    /// stops is a failure rather than a completion. Reporting it as the end of
    /// the audio would leave the daemon holding a device it cannot hear.
    fn next_chunk(&mut self) -> Result<Option<Vec<i16>>, Error> {
        let mut bytes = [0u8; CHUNK_BYTES];
        match self.output.read_exact(&mut bytes) {
            Ok(()) => Ok(Some(
                bytes
                    .chunks_exact(2)
                    .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                    .collect(),
            )),
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Err(self.stopped()),
            Err(error) => Err(Error::Io(error)),
        }
    }
}

/// Stop the child when the source goes away, rather than leaving it holding
/// the microphone until the daemon exits.
impl Drop for PulseSource {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Play `samples` through `device` and wait for them to finish.
///
/// # Errors
///
/// When `program` is not installed, the operating system refuses to start it,
/// or it exits reporting failure.
pub fn play(program: &str, device: &str, samples: &[i16]) -> Result<(), Error> {
    let arguments = args(Direction::Playback, device);
    let mut child = start(program, &arguments, Stdio::piped(), Stdio::null())?;

    if let Some(mut input) = child.stdin.take() {
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        // A closed pipe means playback stopped early, which the status below
        // reports; it is not separately interesting.
        let _ = input.write_all(&bytes);
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Stopped(format!("playback exited with {status}")))
    }
}
