//! The two checks that ask something else whether it is ready.
//!
//! Split from the probes beside them because both reach a service rather than
//! the operating system, and because the module-size gate said so.

use std::env;
use std::process::{Command, Stdio};

use super::{Finding, entries_in};
use crate::config::Config;
use crate::http;

/// Whether the router answers, and whether it serves both speech entries.
pub fn router(config: &Config) -> Vec<Finding> {
    let reply = match http::get(&config.router, "/v1/models") {
        Ok(reply) => reply,
        Err(why) => {
            return vec![Finding::problem(
                "router",
                format!("{} did not answer: {why}", config.router),
                "start the model router, or set 'router' to where it listens",
            )];
        }
    };

    if !reply.ok() {
        return vec![Finding::problem(
            "router",
            format!("{} answered {}", config.router, reply.status),
            "check the router is healthy",
        )];
    }

    let entries = entries_in(&reply.text());
    let mut findings = vec![Finding::ready(
        "router",
        format!("{} serves {} entries", config.router, entries.len()),
    )];

    for (what, wanted) in [
        ("transcriber entry", &config.transcriber),
        ("synthesizer entry", &config.synthesizer),
    ] {
        findings.push(if entries.iter().any(|id| id == wanted) {
            Finding::ready(what, format!("'{wanted}' is in the catalog"))
        } else {
            Finding::problem(
                what,
                format!("'{wanted}' is not among {entries:?}"),
                "add the entry to the router's catalog, or correct the name here",
            )
        });
    }
    findings
}

/// Whether the agent that receives transcripts exists.
pub fn agent(target: &str) -> Finding {
    if env::var("HERDR_ENV").as_deref() != Ok("1") {
        return Finding::unverifiable(
            "voice agent",
            format!("not running inside Herdr, so '{target}' cannot be looked up"),
        );
    }

    let outcome = Command::new(crate::agent::PROGRAM)
        .args(["agent", "get", target])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match outcome {
        Ok(status) if status.success() => {
            Finding::ready("voice agent", format!("'{target}' is live"))
        }
        Ok(_) => Finding::problem(
            "voice agent",
            format!("herdr does not know an agent named '{target}'"),
            "start it with 'herdr agent start', or correct 'agent' in the configuration",
        ),
        Err(why) => Finding::problem(
            "voice agent",
            format!("could not ask herdr: {why}"),
            "check herdr is installed and this session is inside it",
        ),
    }
}
