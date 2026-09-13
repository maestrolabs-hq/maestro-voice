//! Keeping the recent past, so a wake word can look backwards.
//!
//! A wake-word detector fires at the *end* of the phrase, and by then the
//! sentence that follows has usually started. Recording from the moment of
//! detection therefore clips the first word: "hey jarvis run the tests"
//! becomes "un the tests". The fix is to have kept the audio all along and
//! rewind a little when the wake fires.
//!
//! How far to rewind is configuration rather than a constant, because the
//! right answer depends on the detector, the phrase and the speaker, and a
//! number buried in code is a number nobody tunes.

use std::collections::VecDeque;
use std::time::Duration;

use super::samples_in;

/// A fixed-length window of the most recent audio.
///
/// Writing past the end discards the oldest samples: the buffer is a window on
/// the present, and audio old enough to fall out of it is audio no wake word
/// will ever ask for.
#[derive(Debug)]
pub struct Ring {
    samples: VecDeque<i16>,
    capacity: usize,
}

impl Ring {
    /// A ring holding at most `window` of audio.
    #[must_use]
    pub fn holding(window: Duration) -> Self {
        let capacity = samples_in(window);
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Add a chunk, discarding whatever no longer fits.
    ///
    /// A chunk larger than the whole ring keeps only its tail, which is the
    /// same answer many small writes would have reached.
    pub fn write(&mut self, chunk: &[i16]) {
        if self.capacity == 0 {
            return;
        }
        let keep = chunk.len().min(self.capacity);
        self.samples.extend(&chunk[chunk.len() - keep..]);

        let excess = self.samples.len().saturating_sub(self.capacity);
        drop(self.samples.drain(..excess));
    }

    /// The most recent `window` of audio, oldest sample first.
    ///
    /// Returns what exists when asked for more than has been recorded. Padding
    /// the difference with silence would hand the transcriber audio that was
    /// never captured, and it would be indistinguishable from a quiet room.
    #[must_use]
    pub fn pre_roll(&self, window: Duration) -> Vec<i16> {
        let want = samples_in(window).min(self.samples.len());
        self.samples
            .iter()
            .skip(self.samples.len() - want)
            .copied()
            .collect()
    }

    /// How many samples are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether nothing has been recorded yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Ring;
    use std::time::Duration;

    /// A ring of no length is not a useful configuration, but it must not
    /// panic: it arrives from a configuration file, not from a programmer.
    #[test]
    fn a_ring_of_no_duration_accepts_writes_and_stays_empty() {
        let mut ring = Ring::holding(Duration::ZERO);
        ring.write(&[1, 2, 3]);

        assert!(ring.is_empty());
        assert!(ring.pre_roll(Duration::from_millis(300)).is_empty());
    }

    #[test]
    fn an_empty_write_changes_nothing() {
        let mut ring = Ring::holding(Duration::from_millis(100));
        ring.write(&[1, 2, 3]);
        ring.write(&[]);

        assert_eq!(ring.len(), 3, "writing nothing must not disturb the window");
    }
}
