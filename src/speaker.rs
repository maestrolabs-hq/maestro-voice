//! Playing audio, and being able to stop.
//!
//! [`crate::capture::pulse::play`] plays a buffer and returns when it has
//! finished, which is right for a cue and wrong for a reply: saying the wake
//! phrase during an answer must interrupt it, and an interruption has to reach
//! a process that is already running.
//!
//! So playback keeps the child where another thread can find it. `hush` is
//! called from the thread reading the microphone while `play` is still running
//! on a worker, which is the whole point, and it must therefore never block and
//! must tolerate being called when nothing is playing.
//!
//! **A wrong sink name cannot be detected here.** `ffmpeg` exits 0 when the
//! device does not exist, because the audio server falls back to its default.
//! That is reported by `check` and documented rather than papered over: the
//! honest statement is that the configured sink is unverifiable, not that it
//! was verified.

use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use crate::capture::pulse::{self, Direction};
use crate::service::Player;

/// A speaker backed by an `ffmpeg` child, interruptible while it plays.
#[derive(Debug)]
pub struct Speaker {
    program: String,
    device: String,
    /// The child currently playing, if any. Held so another thread can end it.
    playing: Mutex<Option<Child>>,
}

impl Speaker {
    /// A speaker writing to `device`, or the audio server's default.
    #[must_use]
    pub fn new(device: Option<&str>) -> Self {
        Self {
            program: pulse::PROGRAM.to_owned(),
            device: pulse::device_or_default(device).to_owned(),
            playing: Mutex::new(None),
        }
    }

    /// Start a child that will play whatever is written to its input.
    fn start(&self) -> std::io::Result<Child> {
        Command::new(&self.program)
            .args(pulse::args(Direction::Playback, &self.device))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }
}

impl Player for Speaker {
    fn play(&self, samples: &[i16]) -> bool {
        let mut child = match self.start() {
            Ok(child) => child,
            Err(why) => {
                eprintln!("maestro-voice: could not start '{}': {why}", self.program);
                return false;
            }
        };

        let mut input = child.stdin.take();
        // Published before the write, so a hush arriving mid-sentence finds
        // something to stop. A hush between starting and here is the one gap,
        // and it costs a cue rather than an interruption nobody can make.
        if let Ok(mut slot) = self.playing.lock() {
            *slot = Some(child);
        }

        if let Some(pipe) = input.as_mut() {
            let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            // A closed pipe is what being hushed looks like from here, and it
            // is not worth reporting as a failure of playback.
            let _written = pipe.write_all(&bytes);
        }
        // Closing the input is what tells the child the audio is complete.
        drop(input);

        let finished = self
            .playing
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .map(|mut child| child.wait());

        match finished {
            Some(Ok(status)) => status.success(),
            // Taken by `hush`, which already ended it: the audio did not finish,
            // and saying so is correct.
            None => false,
            Some(Err(why)) => {
                eprintln!("maestro-voice: playback failed: {why}");
                false
            }
        }
    }

    fn hush(&self) {
        let Ok(mut slot) = self.playing.lock() else {
            return;
        };
        if let Some(mut child) = slot.take() {
            // Killed rather than asked politely: the point is that the next
            // word out of the speaker is not this one.
            let _killed = child.kill();
            let _reaped = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Speaker;
    use crate::capture::pulse;
    use crate::service::Player;

    #[test]
    fn a_configured_device_is_used_and_a_blank_one_falls_back() {
        assert_eq!(Speaker::new(Some("RDPSink")).device, "RDPSink");
        assert_eq!(
            Speaker::new(Some("   ")).device,
            pulse::device_or_default(None),
            "a blank setting means the operator did not choose"
        );
        assert_eq!(Speaker::new(None).device, pulse::device_or_default(None));
    }

    #[test]
    fn hushing_a_speaker_that_is_not_playing_does_nothing_and_does_not_panic() {
        // It is called from the audio thread on every wake word, whether or not
        // anything happens to be playing.
        let speaker = Speaker::new(None);
        speaker.hush();
        speaker.hush();
    }

    #[test]
    fn the_program_is_a_name_so_it_resolves_at_run_time() {
        assert!(
            !Speaker::new(None).program.contains('/'),
            "a path here would name one machine"
        );
    }
}
