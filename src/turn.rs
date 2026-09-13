//! The turn lifecycle: what the daemon does, and when.
//!
//! This is the decision layer for a whole turn, and it performs no input or
//! output at all. Events arrive already classified -- this chunk woke the
//! detector, that chunk was speech, the transcriber refused -- and actions go
//! out as a list for someone else to carry out. `tests/standards.rs` enforces
//! that, because the alternative is a state machine reachable only by speaking
//! into a microphone with a router running, which is a state machine nobody
//! tests.
//!
//! Two rules here are worth finding in one place rather than deducing from the
//! transitions.
//!
//! **Waiting for the agent is not a state.** A delivered transcript returns the
//! daemon to [`Phase::Idle`], still listening for the wake word. The agent may
//! work for minutes, and a daemon deaf for the duration is one that cannot be
//! corrected halfway through.
//!
//! **Newest wins.** Saying the phrase again cancels whatever is in flight
//! rather than queueing behind it. Queueing would have the agent act on the
//! first thought after hearing the second.

use std::time::Duration;

use crate::endpoint::{Decision, Ending, Rule, Utterance};
use crate::tone::Cue;

mod signal;

pub use signal::{Action, Delivery, Event, Fault};

/// What the daemon is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Listening for the wake word, holding no model and recording nothing.
    Idle,
    /// Recording an utterance.
    Listening,
    /// The utterance is with the transcriber, or its transcript with the agent.
    Working,
    /// A reply is playing.
    Speaking,
}

/// The turn lifecycle.
#[derive(Debug)]
pub struct Machine {
    phase: Phase,
    rule: Rule,
    utterance: Option<Utterance>,
    /// A reply that arrived when it could not be played, kept for the next
    /// quiet moment. Held rather than dropped: the answer is the point of the
    /// turn. Only the newest is kept, for the same reason utterances are.
    held_reply: bool,
}

impl Machine {
    /// A machine that endpoints under `rule`, idle and listening.
    #[must_use]
    pub const fn new(rule: Rule) -> Self {
        Self {
            phase: Phase::Idle,
            rule,
            utterance: None,
            held_reply: false,
        }
    }

    /// What the daemon is doing.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Feed one event and learn what to do about it.
    #[must_use]
    pub fn observe(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::CaptureStopped => vec![Action::Cue(Cue::Stopped), Action::Halt],
            Event::Chunk {
                woke: true,
                frame: _,
                speech: _,
            } => self.woken(),
            Event::Chunk { speech, frame, .. } => self.heard(speech, frame),
            Event::Transcribed { empty } => self.transcribed(empty),
            Event::NotTranscribed(_) => self.finish(Some(Cue::Refused)),
            Event::Delivered(outcome) => self.delivered(outcome),
            Event::ReplyReady => self.reply_ready(),
            Event::Spoke => self.spoken(None),
            Event::NotSpoken => self.spoken(Some(Cue::Mute)),
        }
    }

    /// The wake phrase was just spoken, whatever was happening before.
    fn woken(&mut self) -> Vec<Action> {
        let mut actions = match self.phase {
            Phase::Speaking => vec![Action::Hush],
            Phase::Working => vec![Action::Abandon],
            Phase::Idle | Phase::Listening => Vec::new(),
        };

        actions.push(Action::Cue(Cue::Listening));
        actions.push(Action::Warm);
        actions.push(Action::Record);

        self.phase = Phase::Listening;
        self.utterance = Some(self.rule.start());
        actions
    }

    /// One chunk that did not wake anything.
    fn heard(&mut self, speech: bool, frame: Duration) -> Vec<Action> {
        if self.phase != Phase::Listening {
            return Vec::new();
        }
        let Some(utterance) = self.utterance.as_mut() else {
            return Vec::new();
        };

        match utterance.observe(frame, speech) {
            Decision::Continue => Vec::new(),
            // A capped utterance is still an utterance: a monologue or a stuck
            // microphone both produced audio, and discarding it would lose a
            // sentence someone did say.
            Decision::Ended(Ending::Silence | Ending::Cap) => {
                self.utterance = None;
                self.phase = Phase::Working;
                vec![Action::Transcribe]
            }
            // A wake nothing followed. Silent on purpose: a chime on every
            // false trigger is how a person learns to switch this off.
            Decision::Ended(Ending::NeverSpoke) => self.finish(None),
        }
    }

    /// The transcriber answered.
    fn transcribed(&mut self, empty: bool) -> Vec<Action> {
        if self.phase != Phase::Working {
            // Abandoned: a newer utterance has taken over, and delivering this
            // one would act on a thought already replaced.
            return Vec::new();
        }
        if empty {
            // An empty prompt must never reach the agent.
            return self.finish(Some(Cue::Dismissed));
        }
        vec![Action::Deliver]
    }

    /// The agent took the transcript, or did not.
    fn delivered(&mut self, outcome: Delivery) -> Vec<Action> {
        if self.phase != Phase::Working {
            return Vec::new();
        }
        match outcome {
            Delivery::Sent => self.finish(None),
            Delivery::Blocked | Delivery::Stalled => self.finish(Some(Cue::Blocked)),
        }
    }

    /// A reply is available to speak.
    fn reply_ready(&mut self) -> Vec<Action> {
        if self.phase == Phase::Idle {
            self.phase = Phase::Speaking;
            return vec![Action::Play];
        }
        self.held_reply = true;
        Vec::new()
    }

    /// Playback ended, well or badly.
    fn spoken(&mut self, cue: Option<Cue>) -> Vec<Action> {
        if self.phase != Phase::Speaking {
            return Vec::new();
        }
        self.finish(cue)
    }

    /// Return to idle, announcing `cue` if there is anything to announce, and
    /// releasing a reply that was waiting for the microphone to close.
    fn finish(&mut self, cue: Option<Cue>) -> Vec<Action> {
        self.utterance = None;
        self.phase = Phase::Idle;

        let mut actions: Vec<Action> = cue.map(Action::Cue).into_iter().collect();
        if self.held_reply {
            self.held_reply = false;
            self.phase = Phase::Speaking;
            actions.push(Action::Play);
        }
        actions
    }
}
