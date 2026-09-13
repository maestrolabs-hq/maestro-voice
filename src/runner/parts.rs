//! What the daemon is given, and how it reports stopping.
//!
//! Two small types, kept beside the loop rather than inside it because the
//! module-size gate counts whole files and the loop is the part worth reading in
//! one sitting.

use std::sync::Arc;

use crate::service::{Courier, Player, Synthesizer, Transcriber};

/// The four services a turn needs.
///
/// Reference-counted because each is handed to a worker thread: the loop cannot
/// wait for a transcription, so the transcriber must outlive the call that
/// started it.
#[derive(Clone)]
pub struct Services {
    /// Speech recognition.
    pub transcriber: Arc<dyn Transcriber>,
    /// The way a transcript reaches the agent.
    pub courier: Arc<dyn Courier>,
    /// Speech synthesis.
    pub synthesizer: Arc<dyn Synthesizer>,
    /// Where synthesized audio goes.
    pub player: Arc<dyn Player>,
}

/// Why the daemon stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// Capture failed past its retry budget, and the machine called a halt.
    CaptureGaveUp,
    /// The audio ran out, which only a file does. A microphone does not end, so
    /// under a real capture this is unreachable; under test it is the ordinary
    /// ending.
    AudioEnded,
}
