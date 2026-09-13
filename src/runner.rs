//! Carrying out what the turn machine decides.
//!
//! This is the effectful half of the design: it reads chunks, asks the wake
//! detector and the speech gate about each one, hands the answers to
//! [`crate::turn::Machine`], and does whatever the machine asks for. Every rule
//! lives there and every side effect lives here, which is what lets whole turns
//! be driven from a WAV file with no router, no `herdr` and no speaker.
//!
//! **Nothing slow happens on this thread.** Transcription, delivery and
//! synthesis each take seconds, and the loop cannot stop reading audio for that
//! long without going deaf to the wake word. They run on their own threads and
//! report back through [`note::Note`]; results from a generation the daemon has
//! moved past are dropped. See `note.rs`.

mod act;
mod note;
mod parts;
pub mod wav;

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use crate::capture::{Ring, SAMPLE_RATE, Source};
use crate::service::{Ear, Transcript, Waker};
use crate::speak::{self, Language, Outcome, Spoken};
use crate::turn::{Event, Machine};

pub use note::Note;
pub use parts::{Services, Stopped};

/// How long to wait for work in flight once the audio has run out.
///
/// Only reached when the source is a file, which is to say under test: a
/// microphone does not end. It bounds a test rather than a daemon.
const DRAIN_LIMIT: Duration = Duration::from_secs(30);

/// The daemon: a turn machine, the services it drives, and the audio it holds.
pub struct Daemon {
    machine: Machine,
    services: Services,
    pre_roll: Duration,
    fallback_language: Language,
    ring: Ring,
    recording: Vec<i16>,
    capturing: bool,
    transcript: Option<Transcript>,
    reply: Option<Spoken>,
    language: Option<Language>,
    generation: u64,
    outstanding: usize,
    post: Sender<Note>,
    notes: Receiver<Note>,
}

impl Daemon {
    /// A daemon that endpoints under `machine`'s rule.
    ///
    /// `pre_roll` is how far back to rewind when the wake phrase fires, and
    /// `fallback_language` is the voice used when transcription declines to
    /// report one.
    #[must_use]
    pub fn new(
        machine: Machine,
        services: Services,
        pre_roll: Duration,
        fallback_language: Language,
    ) -> Self {
        let (post, notes) = mpsc::channel();
        Self {
            machine,
            services,
            pre_roll,
            fallback_language,
            // Twice the pre-roll, so a rewind is never asking for audio that
            // was discarded one chunk ago.
            ring: Ring::holding(pre_roll * 2),
            recording: Vec::new(),
            capturing: false,
            transcript: None,
            reply: None,
            language: None,
            generation: 0,
            outstanding: 0,
            post,
            notes,
        }
    }

    /// Where the agent's extension posts finished turns.
    #[must_use]
    pub fn notifier(&self) -> Sender<Note> {
        self.post.clone()
    }

    /// Read audio until the daemon halts or the audio ends.
    pub fn run(
        &mut self,
        source: &mut dyn Source,
        waker: &mut dyn Waker,
        ear: &mut dyn Ear,
    ) -> Stopped {
        let stopped = self.pump(source, waker, ear);
        // Always: a halt still has its own stop tone in flight, and a file that
        // ran out still has a turn finishing behind it.
        self.drain();
        stopped
    }

    /// Read until something stops the daemon, without waiting for the tail.
    fn pump(
        &mut self,
        source: &mut dyn Source,
        waker: &mut dyn Waker,
        ear: &mut dyn Ear,
    ) -> Stopped {
        loop {
            if self.collect() {
                return Stopped::CaptureGaveUp;
            }
            match source.next_chunk() {
                Ok(Some(chunk)) => {
                    if self.heard(&chunk, waker, ear) {
                        return Stopped::CaptureGaveUp;
                    }
                }
                Ok(None) => return Stopped::AudioEnded,
                Err(_) => {
                    let actions = self.machine.observe(Event::CaptureStopped);
                    self.carry_out(actions);
                    return Stopped::CaptureGaveUp;
                }
            }
        }
    }

    /// One chunk of audio; true when the daemon must stop.
    fn heard(&mut self, chunk: &[i16], waker: &mut dyn Waker, ear: &mut dyn Ear) -> bool {
        self.ring.write(chunk);
        if self.capturing {
            self.recording.extend_from_slice(chunk);
        }

        let woke = waker.woke(chunk);
        let speech = ear.speech(chunk);
        // A chunk is at most 1280 samples, exact in a double many times over.
        #[allow(clippy::cast_precision_loss)]
        let seconds = chunk.len() as f64 / f64::from(SAMPLE_RATE);
        let frame = Duration::from_secs_f64(seconds);

        let actions = self.machine.observe(Event::Chunk {
            woke,
            speech,
            frame,
        });
        self.carry_out(actions)
    }

    /// Take every note that has arrived; true when the daemon must stop.
    fn collect(&mut self) -> bool {
        loop {
            match self.notes.try_recv() {
                Ok(note) => {
                    if self.absorb(note) {
                        return true;
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return false,
            }
        }
    }

    /// Wait out the work still in flight, once the audio has ended.
    fn drain(&mut self) {
        let deadline = Instant::now() + DRAIN_LIMIT;
        while self.outstanding > 0 {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return;
            }
            match self.notes.recv_timeout(left) {
                Ok(note) => {
                    if self.absorb(note) {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    }

    /// Turn one note into an event, unless it belongs to a turn already past.
    fn absorb(&mut self, note: Note) -> bool {
        if note.counted() {
            self.outstanding = self.outstanding.saturating_sub(1);
        }
        if let Some(at) = note.generation() {
            if at != self.generation {
                // A result for an utterance that has been replaced. Dropping it
                // is the whole purpose of the generation.
                return false;
            }
        }

        let event = match note {
            Note::Transcribed(_, Ok(transcript)) => {
                let empty = transcript.is_empty();
                self.language.clone_from(&transcript.language);
                self.transcript = Some(transcript);
                Event::Transcribed { empty }
            }
            Note::Transcribed(_, Err(fault)) => Event::NotTranscribed(fault),
            Note::Delivered(_, outcome) => Event::Delivered(outcome),
            Note::Spoke(_, true) => Event::Spoke,
            Note::Spoke(_, false) => Event::NotSpoken,
            // Nothing depends on a cue having finished; it was counted only so
            // that a run driven from a file waits for its own tones.
            Note::Announced => return false,
            Note::Reply(message) => {
                let Some(spoken) = self.decide_reply(&message) else {
                    return false;
                };
                self.reply = Some(spoken);
                Event::ReplyReady
            }
        };

        let actions = self.machine.observe(event);
        self.carry_out(actions)
    }

    /// What to speak for a finished turn, if anything.
    fn decide_reply(&self, message: &str) -> Option<Spoken> {
        let language = self
            .language
            .clone()
            .unwrap_or_else(|| self.fallback_language.clone());
        match speak::extract(message, &language) {
            Ok(Outcome::Spoken(spoken)) => Some(spoken),
            // Counted rather than announced: the design accepted that the block
            // depends on the agent remembering, on condition that a turn
            // without one is visible instead of mysterious.
            Ok(Outcome::Silent) | Err(_) => None,
        }
    }
}
