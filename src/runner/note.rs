//! Results coming back from work that happens off the audio thread.
//!
//! Transcribing, delivering and speaking all take seconds, and the thread
//! reading the microphone cannot afford to wait for any of them: it would stop
//! hearing the wake word, which is the one thing that must always be heard.
//! So each is done on its own thread and reports back here.
//!
//! Every note carries the generation it belongs to. Saying the wake phrase
//! again bumps the generation, which is how a result that is no longer wanted
//! is recognised and dropped rather than acted on. Cancelling the work itself
//! is not attempted: a transcription already in flight will finish and its note
//! will be ignored, which costs a little compute and keeps the daemon from
//! having to reason about half-cancelled network calls.

use std::sync::mpsc::Sender;
use std::thread;

use crate::service::Transcript;
use crate::turn::{Delivery, Fault};

/// Something that finished away from the audio thread.
#[derive(Debug)]
pub enum Note {
    /// A transcription came back.
    Transcribed(u64, Result<Transcript, Fault>),
    /// The agent took the transcript, or did not.
    Delivered(u64, Delivery),
    /// Playback finished; `false` means it did not happen.
    Spoke(u64, bool),
    /// The agent's own process posted a finished turn.
    ///
    /// Carries no generation: a reply belongs to whichever utterance the agent
    /// was answering, and the daemon may well have moved on. Discarding it for
    /// being late would throw away the answer.
    Reply(String),
}

impl Note {
    /// The generation this note belongs to, if it belongs to one.
    #[must_use]
    pub const fn generation(&self) -> Option<u64> {
        match self {
            Self::Transcribed(at, _) | Self::Delivered(at, _) | Self::Spoke(at, _) => Some(*at),
            Self::Reply(_) => None,
        }
    }
}

/// Run `work` on its own thread and post what it returns.
///
/// The send is allowed to fail silently: a closed channel means the daemon has
/// stopped, and a worker shouting about it on the way out helps nobody.
pub fn spawn<F>(post: &Sender<Note>, work: F)
where
    F: FnOnce() -> Note + Send + 'static,
{
    let post = post.clone();
    drop(thread::spawn(move || {
        let note = work();
        drop(post.send(note));
    }));
}

#[cfg(test)]
mod tests {
    use super::Note;
    use crate::turn::Delivery;
    use std::sync::mpsc;

    #[test]
    fn work_notes_carry_a_generation_and_a_reply_does_not() {
        assert_eq!(Note::Delivered(7, Delivery::Sent).generation(), Some(7));
        assert_eq!(Note::Spoke(3, true).generation(), Some(3));
        assert_eq!(
            Note::Reply("done".to_owned()).generation(),
            None,
            "a reply answers an utterance the daemon may have moved past"
        );
    }

    #[test]
    fn a_worker_posts_its_result() {
        let (post, notes) = mpsc::channel();
        super::spawn(&post, || Note::Spoke(1, true));

        let note = notes
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the worker must post");
        assert_eq!(note.generation(), Some(1));
    }

    #[test]
    fn a_worker_whose_daemon_has_gone_does_not_panic() {
        // The daemon stopping while work is in flight is ordinary shutdown, not
        // an error the worker should turn into a crash.
        let (post, notes) = mpsc::channel::<Note>();
        drop(notes);
        super::spawn(&post, || Note::Spoke(1, true));
        // Nothing to assert but the absence of a panic; give the thread a
        // moment to run and fail to send.
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
