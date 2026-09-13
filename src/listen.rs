//! Hearing the wake phrase in a live stream.
//!
//! Joins the two halves of `crate::wake`: the scorer, which is the only part
//! that knows a neural network exists, and the detector, which turns the score
//! series into the one event the daemon cares about.
//!
//! A scorer that fails mid-stream does not stop the daemon. The honest answer
//! to "was the phrase spoken" is no, and a microphone that goes deaf while
//! still running is exactly the failure this repository says is its worst
//! outcome -- so it is said out loud, once, rather than swallowed silently or
//! turned into a crash.

use crate::service::Waker;
use crate::wake::{Detector, Error, ModelSet, Scorer};

/// A scorer and a detector, listening together.
///
/// Not `Debug`: the scorer holds neural network sessions with nothing useful to
/// print, and a derived implementation would only report that they exist.
pub struct Listener {
    scorer: Scorer,
    detector: Detector,
    /// Set once the scorer has failed, so the complaint is made once rather
    /// than twelve times a second for as long as the daemon runs.
    complained: bool,
}

impl Listener {
    /// A listener over `models`, firing above `threshold`.
    ///
    /// # Errors
    ///
    /// When the models cannot be loaded.
    pub fn open(models: &ModelSet, threshold: f32) -> Result<Self, Error> {
        Ok(Self {
            scorer: Scorer::open(models)?,
            detector: Detector::new(threshold),
            complained: false,
        })
    }
}

impl Waker for Listener {
    fn woke(&mut self, chunk: &[i16]) -> bool {
        match self.scorer.push(chunk) {
            Ok(score) => {
                self.complained = false;
                self.detector.observe(score)
            }
            Err(why) => {
                if !self.complained {
                    eprintln!("maestro-voice: the wake detector stopped scoring: {why}");
                    eprintln!("maestro-voice: the wake word will not be heard until this clears");
                    self.complained = true;
                }
                false
            }
        }
    }
}
