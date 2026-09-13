//! Handing a transcript to the voice agent, through Herdr.
//!
//! The agent runs in a terminal pane, and `herdr agent prompt` is the supported
//! way to put text into one: it honours the pane's bracketed-paste mode and
//! sends the text followed by an encoded return.
//!
//! Two of its answers matter and neither is a failure of this daemon.
//! `agent_blocked` means the agent is sitting on an approval or question, so
//! the words would have answered the wrong question; Herdr rejects the prompt
//! before sending any input, which is exactly the behaviour wanted. And
//! `agent_prompt_stalled` means the prompt went in but the agent did not react
//! within Herdr's own window, so nothing can be assumed about whether it was
//! read. Both are announced with tones and the transcript is written to the log
//! so that nothing the owner said is lost.
//!
//! Told apart by the machine-readable error code Herdr puts on stderr, never by
//! its prose.

use std::process::{Command, Stdio};

use crate::service::Courier;
use crate::speech::json;
use crate::turn::Delivery;

/// The command that talks to the current Herdr session.
pub const PROGRAM: &str = "herdr";

/// Herdr's code for an agent sitting on a question.
const BLOCKED: &str = "agent_blocked";

/// Herdr's code for a prompt that produced no reaction.
const STALLED: &str = "agent_prompt_stalled";

/// Delivery to one named agent in a Herdr pane.
#[derive(Debug, Clone)]
pub struct Herdr {
    program: String,
    target: String,
}

impl Herdr {
    /// Deliver to the agent named `target`.
    ///
    /// The program is a name rather than a path so it resolves on the search
    /// path at run time, which is what keeps this working on a machine that is
    /// not the one it was written on.
    #[must_use]
    pub fn new(target: &str) -> Self {
        Self {
            program: PROGRAM.to_owned(),
            target: target.to_owned(),
        }
    }

    /// The arguments for one prompt.
    ///
    /// Separated so the shape can be asserted without running anything. The
    /// transcript is a single argument and never interpolated into a shell
    /// line: it is arbitrary speech, and speech contains quotation marks,
    /// semicolons and backticks.
    #[must_use]
    pub fn arguments(target: &str, text: &str) -> Vec<String> {
        vec![
            "agent".to_owned(),
            "prompt".to_owned(),
            target.to_owned(),
            text.to_owned(),
        ]
    }

    /// Which delivery outcome an error report describes.
    #[must_use]
    pub fn outcome_of(report: &str) -> Delivery {
        match json::field(report, "code").as_deref() {
            Some(BLOCKED) => Delivery::Blocked,
            Some(STALLED) => Delivery::Stalled,
            other => {
                // An unrecognised failure is still a prompt whose fate is
                // unknown, which is what Stalled means: not delivered, and not
                // provably lost either. Naming the code keeps the log useful
                // when Herdr grows one this does not know.
                eprintln!(
                    "maestro-voice: herdr reported '{}'",
                    other.unwrap_or("no code")
                );
                Delivery::Stalled
            }
        }
    }
}

impl Courier for Herdr {
    fn deliver(&self, text: &str) -> Delivery {
        let outcome = Command::new(&self.program)
            .args(Self::arguments(&self.target, text))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let finished = match outcome {
            Ok(finished) => finished,
            Err(why) => {
                eprintln!("maestro-voice: could not run '{}': {why}", self.program);
                eprintln!("maestro-voice: the transcript was: {text}");
                return Delivery::Stalled;
            }
        };

        if finished.status.success() {
            return Delivery::Sent;
        }

        let report = String::from_utf8_lossy(&finished.stderr);
        let outcome = Self::outcome_of(&report);
        // Logged in full so that nothing said is lost, whatever the tone said.
        eprintln!("maestro-voice: the agent did not take the transcript ({outcome:?})");
        eprintln!("maestro-voice: the transcript was: {text}");
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::{Herdr, STALLED};
    use crate::turn::Delivery;

    #[test]
    fn a_prompt_passes_the_transcript_as_one_argument() {
        // Speech contains quotation marks, semicolons and backticks. Passed as
        // an argument they are text; interpolated into a shell line they are
        // not.
        let awkward = r#"run "the tests"; rm -rf / `whoami`"#;
        let arguments = Herdr::arguments("voice", awkward);

        assert_eq!(arguments[..3], ["agent", "prompt", "voice"]);
        assert_eq!(
            arguments[3], awkward,
            "the transcript must arrive whole and uninterpreted"
        );
        assert_eq!(arguments.len(), 4, "and nothing else may be appended");
    }

    #[test]
    fn an_agent_on_a_question_is_told_apart_from_one_that_did_not_react() {
        let blocked = r#"{"error":{"code":"agent_blocked","message":"approval UI"}}"#;
        let stalled = format!(r#"{{"error":{{"code":"{STALLED}"}}}}"#);

        assert_eq!(Herdr::outcome_of(blocked), Delivery::Blocked);
        assert_eq!(Herdr::outcome_of(&stalled), Delivery::Stalled);
    }

    #[test]
    fn a_failure_with_no_code_is_unknown_rather_than_assumed_delivered() {
        // The dangerous default would be Sent: it would announce nothing and
        // leave the owner believing the agent had their words.
        for report in ["", "herdr: command not found", "{}"] {
            assert_eq!(
                Herdr::outcome_of(report),
                Delivery::Stalled,
                "{report:?} must not be read as a successful delivery"
            );
        }
    }

    #[test]
    fn the_program_is_a_name_so_it_resolves_at_run_time() {
        let herdr = Herdr::new("voice");
        assert_eq!(herdr.program, super::PROGRAM);
        assert!(
            !herdr.program.contains('/'),
            "a path here would name one machine"
        );
    }
}
