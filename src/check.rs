//! Saying what is not ready, and what would fix it.
//!
//! Three rules shape this, and they are the reason it is worth its own module.
//!
//! **It never starts a model.** Readiness is a question about the system as it
//! stands; a check that loaded four gigabytes onto the card to answer it would
//! be changing what it measures. So the router is asked for its catalog at
//! `GET /v1/models`, which is answered from the catalog itself, and never at
//! `GET /models/<id>/health`, which relays to the child and so starts it.
//!
//! **It reads the right listing.** The two speech entries are deliberately
//! absent from `GET /models`, the router-mode surface, because that surface
//! says what a client may hold a conversation with and neither speech service
//! does. They appear on `GET /v1/models`. Asking the wrong one would report
//! both missing on a perfectly healthy system.
//!
//! **It says when it cannot tell.** Some things are not knowable from here, and
//! the honest report for those is neither a tick nor a cross.

mod ask;
mod probe;

use std::fmt;

use crate::capture::pulse;
use crate::config::Config;
use crate::speech::json;

/// How one check came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Checked, and it holds.
    Ready,
    /// Checked, and it does not.
    Problem,
    /// Not knowable from here, which is a third answer and not a pass.
    Unverifiable,
}

impl fmt::Display for Standing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Ready => "ok",
            Self::Problem => "PROBLEM",
            Self::Unverifiable => "unknown",
        };
        write!(f, "{word}")
    }
}

/// One thing checked.
#[derive(Debug, Clone)]
pub struct Finding {
    /// What was checked.
    pub what: String,
    /// How it came out.
    pub standing: Standing,
    /// What was observed.
    pub detail: String,
    /// What would fix it, when there is something to do.
    pub remedy: Option<String>,
}

impl Finding {
    fn ready(what: &str, detail: String) -> Self {
        Self {
            what: what.to_owned(),
            standing: Standing::Ready,
            detail,
            remedy: None,
        }
    }

    fn problem(what: &str, detail: String, remedy: &str) -> Self {
        Self {
            what: what.to_owned(),
            standing: Standing::Problem,
            detail,
            remedy: Some(remedy.to_owned()),
        }
    }

    fn unverifiable(what: &str, detail: String) -> Self {
        Self {
            what: what.to_owned(),
            standing: Standing::Unverifiable,
            detail,
            remedy: None,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<12} {:<22} {}", self.standing, self.what, self.detail)?;
        if let Some(remedy) = &self.remedy {
            write!(f, "\n{:<12} {:<22} fix: {remedy}", "", "")?;
        }
        Ok(())
    }
}

/// Which entries a listing names.
///
/// Pure, so the one thing that would silently misreport a healthy system --
/// reading the wrong listing, or only its first entry -- is tested rather than
/// trusted.
#[must_use]
pub fn entries_in(listing: &str) -> Vec<String> {
    json::fields(listing, "id")
}

/// Check everything, in the order a reader would want it.
#[must_use]
pub fn report(config: &Config) -> Vec<Finding> {
    let mut findings = vec![
        probe::program(pulse::PROGRAM, "audio transport"),
        probe::capture(config.capture_device.as_deref()),
        probe::playback_device(config.playback_device.as_deref()),
        probe::wake_models(&config.wake_models),
    ];
    findings.extend(ask::router(config));
    findings.push(probe::program(crate::agent::PROGRAM, "herdr"));
    findings.push(ask::agent(&config.agent));
    findings.push(probe::intake(config.intake_port));
    findings
}

/// Whether anything in a report is a problem.
#[must_use]
pub fn any_problem(findings: &[Finding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.standing == Standing::Problem)
}

#[cfg(test)]
mod tests {
    use super::{Finding, Standing, any_problem, entries_in};

    #[test]
    fn the_listing_the_speech_entries_appear_on_is_the_one_that_is_read() {
        // GET /v1/models. The router-mode listing at GET /models deliberately
        // omits both speech entries, so checking that one would report them
        // missing on a healthy system.
        let listing = r#"{"data":[{"id":"gemma3","object":"model","owned_by":"maestro-llamacpp"},
            {"id":"whisper","object":"model","owned_by":"maestro-llamacpp"},
            {"id":"tts","object":"model","owned_by":"maestro-llamacpp"}],"object":"list"}"#;

        let entries = entries_in(listing);
        assert!(entries.iter().any(|id| id == "whisper"));
        assert!(entries.iter().any(|id| id == "tts"));
        assert_eq!(entries.len(), 3, "every entry, not just the first");
    }

    #[test]
    fn a_router_mode_listing_without_the_speech_entries_reads_as_not_having_them() {
        // What GET /models answers now. Proof the two listings really differ,
        // so the note in this module's documentation stays true.
        let router_mode = r#"{"data":[{"id":"gemma3","status":{"value":"unloaded"}}]}"#;
        let entries = entries_in(router_mode);

        assert!(!entries.iter().any(|id| id == "whisper"));
        assert!(!entries.iter().any(|id| id == "tts"));
    }

    #[test]
    fn an_empty_listing_names_nothing_rather_than_failing() {
        assert!(entries_in(r#"{"data":[],"object":"list"}"#).is_empty());
        assert!(entries_in("").is_empty());
    }

    #[test]
    fn only_a_problem_counts_as_a_problem() {
        // The distinction that matters: something unverifiable must not fail
        // the check, or the exit status stops meaning anything on a machine
        // where a sink name simply cannot be confirmed.
        let ready = Finding::ready("a", "fine".to_owned());
        let unknown = Finding::unverifiable("b", "cannot tell".to_owned());
        let broken = Finding::problem("c", "missing".to_owned(), "install it");

        assert!(!any_problem(&[ready.clone(), unknown.clone()]));
        assert!(any_problem(&[ready, unknown, broken]));
    }

    #[test]
    fn a_finding_prints_its_remedy_when_it_has_one() {
        let broken = Finding::problem("herdr", "not on the path".to_owned(), "install herdr");
        let shown = broken.to_string();

        assert!(shown.contains("PROBLEM"));
        assert!(shown.contains("herdr"));
        assert!(shown.contains("fix: install herdr"), "{shown}");

        let fine = Finding::ready("herdr", "found".to_owned()).to_string();
        assert!(!fine.contains("fix:"), "{fine}");
    }

    #[test]
    fn the_three_standings_read_differently() {
        assert_eq!(Standing::Ready.to_string(), "ok");
        assert_eq!(Standing::Problem.to_string(), "PROBLEM");
        assert_eq!(Standing::Unverifiable.to_string(), "unknown");
    }
}
