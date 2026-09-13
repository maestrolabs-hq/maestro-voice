//! When to try the microphone again, and when to stop trying.
//!
//! Capture is a child process, so it can die: the audio server restarts, the
//! device disappears when a laptop undocks, the operating system reaps
//! something. Most of those heal on their own, which argues for retrying. Some
//! do not, and retrying those forever produces the worst outcome this
//! repository can have -- a daemon that looks alive, holds the microphone, and
//! hears nothing, saying nothing about it.
//!
//! So the rule is bounded, and bounded consecutively. A device that hiccups
//! twice an hour must not accumulate its way to a permanent stop across a long
//! day, so any success clears the count.
//!
//! The rule is separated from the waiting on purpose. Nothing here sleeps,
//! spawns or reads a clock, which is what lets the give-up behaviour be tested
//! in microseconds instead of by killing a real process and hoping.

use std::time::Duration;

/// The longest this will ever wait between attempts.
///
/// Doubling without a ceiling means a daemon left running overnight retries
/// once a day. A device that comes back should be noticed in seconds.
const LONGEST_WAIT: Duration = Duration::from_secs(5);

/// What to do after capture failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    /// Open the source again, after waiting this long.
    Retry(Duration),
    /// Stop. Too many failures in a row, and something needs to say so.
    GiveUp,
}

/// How many consecutive failures capture may absorb, and how long to wait.
#[derive(Debug, Clone, Copy)]
pub struct Restart {
    allowed: u32,
    failures: u32,
    first_wait: Duration,
}

impl Restart {
    /// A policy allowing `allowed` consecutive failures, the first retry
    /// waiting `first_wait` and each following one waiting twice as long, up
    /// to a ceiling.
    #[must_use]
    pub const fn allowing(allowed: u32, first_wait: Duration) -> Self {
        Self {
            allowed,
            failures: 0,
            first_wait,
        }
    }

    /// Record that audio arrived, which forgives every failure before it.
    pub const fn succeeded(&mut self) {
        self.failures = 0;
    }

    /// Record a failure and learn whether to try again.
    pub fn failed(&mut self) -> Attempt {
        self.failures += 1;
        if self.failures >= self.allowed {
            return Attempt::GiveUp;
        }
        Attempt::Retry(self.wait())
    }

    /// How many consecutive failures have not yet been forgiven.
    #[must_use]
    pub const fn failures(self) -> u32 {
        self.failures
    }

    /// The wait before the attempt that follows the failures so far.
    fn wait(self) -> Duration {
        let doublings = self.failures.saturating_sub(1).min(16);
        self.first_wait
            .saturating_mul(1u32 << doublings)
            .min(LONGEST_WAIT)
    }
}

#[cfg(test)]
mod tests {
    use super::{Attempt, LONGEST_WAIT, Restart};
    use std::time::Duration;

    #[test]
    fn the_wait_stops_doubling_at_the_ceiling() {
        // Without a ceiling, a daemon left running overnight would retry about
        // once a day, and a device that came back would go unnoticed.
        let mut restart = Restart::allowing(u32::MAX, Duration::from_millis(200));
        let mut longest = Duration::ZERO;
        for _ in 0..40 {
            if let Attempt::Retry(wait) = restart.failed() {
                longest = longest.max(wait);
            }
        }

        assert_eq!(longest, LONGEST_WAIT, "the wait must reach its ceiling");
    }

    #[test]
    fn the_failure_count_is_reportable_so_a_log_can_say_which_attempt_this_is() {
        let mut restart = Restart::allowing(3, Duration::from_millis(200));
        assert_eq!(restart.failures(), 0);
        let _ = restart.failed();
        assert_eq!(restart.failures(), 1);
        restart.succeeded();
        assert_eq!(restart.failures(), 0, "a success forgives the count");
    }
}
