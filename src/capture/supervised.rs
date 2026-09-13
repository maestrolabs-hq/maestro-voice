//! Keeping capture alive across a child that dies, and stopping when it will
//! not come back.
//!
//! This is where the restart policy meets a real source. It exists as its own
//! type so the loop that decides "reopen, wait, or give up" can be exercised
//! against a source that is simply gone, rather than by killing a process and
//! hoping the code under test was what noticed.
//!
//! Waiting is injected for the same reason. A test that proves the backoff
//! schedule by sleeping through it is a test nobody keeps.

use std::thread;
use std::time::Duration;

use super::{Attempt, Error, Restart, Source};

/// Something that can open a source, so a dead one can be replaced.
pub trait Reopen {
    /// Open the source again.
    ///
    /// # Errors
    ///
    /// When the device or the program behind it is unavailable.
    fn reopen(&mut self) -> Result<Box<dyn Source>, Error>;
}

/// How to pass the time between attempts.
pub trait Wait {
    /// Pause for `how_long`.
    fn wait(&mut self, how_long: Duration);
}

/// The real one: actually sleeps.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sleep;

impl Wait for Sleep {
    fn wait(&mut self, how_long: Duration) {
        thread::sleep(how_long);
    }
}

/// A source that reopens itself when it fails, until it has failed too often.
pub struct Supervised<R, W> {
    opener: R,
    restart: Restart,
    waiter: W,
    inner: Option<Box<dyn Source>>,
}

/// Reports the policy and whether a source is open, not the source itself,
/// which is a trait object with nothing useful to print.
impl<R, W> std::fmt::Debug for Supervised<R, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Supervised {{ open: {}, failures: {} }}",
            self.inner.is_some(),
            self.restart.failures()
        )
    }
}

impl<R: Reopen, W: Wait> Supervised<R, W> {
    /// Supervise whatever `opener` opens, under `restart`.
    pub const fn new(opener: R, restart: Restart, waiter: W) -> Self {
        Self {
            opener,
            restart,
            waiter,
            inner: None,
        }
    }

    /// How many consecutive failures have not been forgiven by fresh audio.
    #[must_use]
    pub const fn failures(&self) -> u32 {
        self.restart.failures()
    }

    /// Record a failure and either wait for another try or stop.
    fn absorb(&mut self, why: &Error) -> Option<Error> {
        self.inner = None;
        match self.restart.failed() {
            Attempt::Retry(pause) => {
                self.waiter.wait(pause);
                None
            }
            Attempt::GiveUp => Some(Error::Stopped(format!(
                "capture failed {} times in a row and has stopped; the last failure was: {why}",
                self.restart.failures()
            ))),
        }
    }
}

impl<R: Reopen, W: Wait> Source for Supervised<R, W> {
    /// The next chunk, reopening the source as often as the policy allows.
    ///
    /// A source that runs out is treated as a failure rather than an ending.
    /// Microphones do not reach an end, so `Ok(None)` from below means the
    /// audio stopped arriving, which is the thing this type exists to notice.
    fn next_chunk(&mut self) -> Result<Option<Vec<i16>>, Error> {
        loop {
            let Some(source) = self.inner.as_mut() else {
                match self.opener.reopen() {
                    Ok(source) => self.inner = Some(source),
                    Err(why) => {
                        if let Some(stopped) = self.absorb(&why) {
                            return Err(stopped);
                        }
                    }
                }
                continue;
            };

            let outcome = source.next_chunk();
            match outcome {
                Ok(Some(chunk)) => {
                    self.restart.succeeded();
                    return Ok(Some(chunk));
                }
                Ok(None) => {
                    let ended = Error::Stopped("the source produced no more audio".to_owned());
                    if let Some(stopped) = self.absorb(&ended) {
                        return Err(stopped);
                    }
                }
                Err(why) => {
                    if let Some(stopped) = self.absorb(&why) {
                        return Err(stopped);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Sleep, Wait};
    use std::time::{Duration, Instant};

    #[test]
    fn the_real_waiter_actually_waits() {
        // Otherwise the backoff is a number in a log and nothing more.
        let started = Instant::now();
        Sleep.wait(Duration::from_millis(20));

        assert!(
            started.elapsed() >= Duration::from_millis(20),
            "Sleep must pass the time it was given"
        );
    }
}
