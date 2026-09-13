//! What the machine is told, and what it asks for.
//!
//! Separated from the rules beside it because they change for different
//! reasons: this file is the vocabulary the runner and the machine agree on,
//! and `turn.rs` is the policy written in it. Keeping them apart also keeps the
//! policy readable as policy, which is the part worth arguing about.

use std::time::Duration;

use crate::tone::Cue;

/// Why a transcription did not happen.
///
/// Both end the turn the same way, and they are kept apart because the log
/// should say which: one is a device with no room, the other is a router that
/// was not there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The router would not serve the model, or it never became ready.
    Refused,
    /// The router could not be reached at all.
    Unreachable,
}

/// What became of a transcript handed to the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// The agent took it.
    Sent,
    /// The agent is sitting on a question, so the words would have answered the
    /// wrong one.
    Blocked,
    /// The agent did not react, so nothing can be assumed about whether it read
    /// them.
    Stalled,
}

/// Something the daemon noticed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// One chunk of audio, already classified by the caller.
    ///
    /// Classified rather than raw on purpose: the machine never sees a sample,
    /// which is what lets every rule below be exercised from a table of
    /// booleans instead of a recording.
    Chunk {
        /// The wake detector crossed its threshold on this chunk.
        woke: bool,
        /// This chunk holds speech.
        speech: bool,
        /// How much audio this chunk is.
        frame: Duration,
    },
    /// The transcriber answered.
    Transcribed {
        /// Whether it found nothing to say.
        empty: bool,
    },
    /// The transcriber would not, or could not, answer.
    NotTranscribed(Fault),
    /// The transcript reached the agent, or did not.
    Delivered(Delivery),
    /// A spoken block arrived from the agent's own process.
    ReplyReady,
    /// Playback finished on its own.
    Spoke,
    /// Synthesis or playback failed.
    NotSpoken,
    /// Capture failed past its retry budget and will not return.
    CaptureStopped,
}

/// What the runner must do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Ask the router for the transcriber now, so its load overlaps the speech
    /// still to come.
    ///
    /// Best effort by design: the transcription itself would load the model
    /// anyway, just later, so a warm that fails costs latency and never
    /// correctness. It is therefore never announced.
    Warm,
    /// Begin an utterance, starting from the pre-roll already captured.
    Record,
    /// Play a cue.
    Cue(Cue),
    /// Send the recorded utterance to the transcriber.
    Transcribe,
    /// Send the transcript to the agent.
    Deliver,
    /// Synthesize and play the reply that arrived.
    Play,
    /// Stop playback now.
    Hush,
    /// Give up on the work in flight; its result must not be acted on.
    Abandon,
    /// Stop the daemon.
    Halt,
}
