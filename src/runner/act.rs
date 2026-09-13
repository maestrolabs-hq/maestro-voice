//! The side effects a turn asks for.
//!
//! Split from the loop beside it at the seam the module-size gate exposed, and
//! it is a real one: `runner.rs` reads audio and decides nothing, this file
//! performs effects and decides nothing. Every choice about *whether* to do any
//! of it was already made in `crate::turn`.
//!
//! Each slow effect is handed to its own thread. The loop must keep reading
//! chunks, because the wake word is the one thing that must always be heard,
//! and a transcription takes seconds.

use std::sync::Arc;

use crate::tone::Cue;
use crate::turn::Action;

use super::{Daemon, Note, note, wav};

impl Daemon {
    /// Do what the machine asked; true when it asked to stop.
    pub(super) fn carry_out(&mut self, actions: Vec<Action>) -> bool {
        for action in actions {
            match action {
                Action::Halt => return true,
                Action::Warm => self.warm(),
                Action::Record => self.start_recording(),
                Action::Cue(cue) => self.announce(cue),
                Action::Transcribe => self.transcribe(),
                Action::Deliver => self.deliver(),
                Action::Play => self.play(),
                Action::Hush => self.services.player.hush(),
                Action::Abandon => self.generation += 1,
            }
        }
        false
    }

    /// Ask for the transcription model now, and do not wait for it.
    fn warm(&mut self) {
        let transcriber = Arc::clone(&self.services.transcriber);
        drop(std::thread::spawn(move || transcriber.warm()));
    }

    /// Begin an utterance from the pre-roll already captured.
    fn start_recording(&mut self) {
        self.generation += 1;
        self.recording = self.ring.pre_roll(self.pre_roll);
        self.capturing = true;
    }

    /// Play a cue without holding up the audio loop.
    ///
    /// On its own thread because a cue lasts up to six hundred milliseconds and
    /// the loop cannot stop reading for that long -- the acknowledgement plays
    /// at the same moment recording starts, so blocking here would cut a hole
    /// in the utterance it is acknowledging.
    fn announce(&mut self, cue: Cue) {
        let player = Arc::clone(&self.services.player);
        self.start(move |_| {
            // Whether a cue reached the speaker changes nothing the daemon
            // does: it is already announcing that something went wrong.
            let _played = player.play(&cue.samples());
            Note::Announced
        });
    }

    fn transcribe(&mut self) {
        self.capturing = false;
        let audio = std::mem::take(&mut self.recording);
        let wav = wav::encode(&audio);
        let transcriber = Arc::clone(&self.services.transcriber);
        self.start(move |at| Note::Transcribed(at, transcriber.transcribe(&wav)));
    }

    fn deliver(&mut self) {
        let Some(transcript) = self.transcript.take() else {
            return;
        };
        let courier = Arc::clone(&self.services.courier);
        self.start(move |at| Note::Delivered(at, courier.deliver(&transcript.text)));
    }

    fn play(&mut self) {
        let Some(spoken) = self.reply.take() else {
            return;
        };
        let synthesizer = Arc::clone(&self.services.synthesizer);
        let player = Arc::clone(&self.services.player);
        self.start(move |at| {
            let said = synthesizer
                .synthesize(spoken.text(), spoken.language())
                .is_some_and(|samples| player.play(&samples));
            Note::Spoke(at, said)
        });
    }

    /// Begin one piece of work off the audio thread.
    ///
    /// The one place that records work as outstanding and stamps it with the
    /// generation it belongs to. Written once because the three callers above
    /// were otherwise the same eleven lines with a different middle, and a
    /// spawner that forgot either half would either hang a file-driven run or
    /// act on an utterance the speaker had already replaced.
    fn start<F>(&mut self, work: F)
    where
        F: FnOnce(u64) -> Note + Send + 'static,
    {
        let at = self.generation;
        self.outstanding += 1;
        note::spawn(&self.post, move || work(at));
    }
}
