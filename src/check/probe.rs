//! The individual checks, each of which touches the world.
//!
//! Kept apart from the reporting beside it so that what a finding *is* stays
//! readable without the process spawning, socket binding and path searching
//! that answering one requires.

use std::env;
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};

use super::Finding;
use crate::capture::{Source, pulse};
use crate::wake::ModelSet;

/// Whether a program resolves on the search path.
pub fn program(name: &str, what: &str) -> Finding {
    match on_path(name) {
        Some(found) => Finding::ready(what, format!("'{name}' at {}", found.display())),
        None => Finding::problem(
            what,
            format!("'{name}' is not on the search path"),
            &format!("install {name}, or put it on PATH"),
        ),
    }
}

/// Whether audio actually arrives from the capture device.
///
/// Opens the device and reads one chunk. A device that is named, exists, and
/// delivers nothing is the failure worth catching here, and it is not visible
/// from configuration alone.
pub fn capture(device: Option<&str>) -> Finding {
    let named = pulse::device_or_default(device);
    let mut source = match pulse::PulseSource::open(pulse::PROGRAM, named) {
        Ok(source) => source,
        Err(why) => {
            return Finding::problem(
                "microphone",
                format!("'{named}' would not open: {why}"),
                "check the audio server is running and the source name is right",
            );
        }
    };

    match source.next_chunk() {
        Ok(Some(chunk)) if !chunk.is_empty() => Finding::ready(
            "microphone",
            format!("'{named}' delivered {} samples", chunk.len()),
        ),
        Ok(_) => Finding::problem(
            "microphone",
            format!("'{named}' opened but delivered no audio"),
            "check the source is not muted and is the one receiving your voice",
        ),
        Err(why) => Finding::problem(
            "microphone",
            format!("'{named}' stopped: {why}"),
            "check the audio server is running",
        ),
    }
}

/// Whether the playback device is the one configured, which cannot be known.
///
/// `ffmpeg` exits 0 when the sink does not exist, because the audio server
/// falls back to its default. So a misconfigured sink is not detectable here:
/// the reply would be audible on the wrong device, and every check would pass.
/// The honest report is that it is unverifiable.
pub fn playback_device(device: Option<&str>) -> Finding {
    let named = pulse::device_or_default(device);
    Finding::unverifiable(
        "speaker",
        format!(
            "'{named}' cannot be confirmed: ffmpeg exits 0 for a sink that does \
             not exist and the audio server plays to its default instead, so a \
             wrong name here is audible on the wrong device rather than reported"
        ),
    )
}

/// Whether the wake-word weights are where the daemon expects them.
pub fn wake_models(directory: &Path) -> Finding {
    match ModelSet::at(directory) {
        Ok(_) => Finding::ready(
            "wake weights",
            format!(
                "all three present in {}; their checksums are verified by the \
                 fetch step, not here",
                directory.display()
            ),
        ),
        Err(missing) => Finding::problem(
            "wake weights",
            missing.to_string(),
            "run 'just fetch-wake-models', which downloads them and checks their sums",
        ),
    }
}

/// Whether the port the agent's extension posts to is free to bind.
pub fn intake(port: u16) -> Finding {
    match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
        Ok(listener) => {
            drop(listener);
            Finding::ready("intake port", format!("{port} is free"))
        }
        Err(why) => Finding::problem(
            "intake port",
            format!("{port} cannot be bound: {why}"),
            "stop whatever holds it, or set 'intake_port' to another",
        ),
    }
}

/// The first match for `name` on the search path.
///
/// Read from the environment rather than looked for in a fixed place, because
/// a compiled-in directory is a fact about one machine.
fn on_path(name: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|directory| directory.join(name))
            .find(|candidate| candidate.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::{intake, on_path, wake_models};
    use crate::check::Standing;
    use std::net::{Ipv4Addr, TcpListener};
    use std::path::Path;

    #[test]
    fn a_program_that_is_not_installed_is_reported_with_a_remedy() {
        let finding = super::program("a-program-nobody-installed", "example");
        assert_eq!(finding.standing, Standing::Problem);
        assert!(finding.remedy.is_some(), "a problem must say what to do");
    }

    #[test]
    fn the_search_path_is_read_from_the_environment() {
        // Something every machine has, found without naming a directory.
        assert!(
            on_path("sh").is_some() || on_path("cmd.exe").is_some(),
            "a shell must be findable on the search path"
        );
        assert_eq!(on_path("a-program-nobody-installed"), None);
    }

    #[test]
    fn a_port_already_held_is_reported_rather_than_assumed_free() {
        let held = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
        let port = held.local_addr().expect("address").port();

        let finding = intake(port);
        assert_eq!(
            finding.standing,
            Standing::Problem,
            "a bound port must not read as free: {finding:?}"
        );
    }

    #[test]
    fn a_free_port_reads_as_free_and_is_left_free() {
        let scratch = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
        let port = scratch.local_addr().expect("address").port();
        drop(scratch);

        assert_eq!(intake(port).standing, Standing::Ready);
        // The check must release it again, or checking twice would fail.
        assert_eq!(intake(port).standing, Standing::Ready);
    }

    #[test]
    fn missing_wake_weights_name_the_fetch_step() {
        let finding = wake_models(Path::new("/somewhere/not-here"));
        assert_eq!(finding.standing, Standing::Problem);
        assert!(
            finding
                .remedy
                .as_deref()
                .is_some_and(|r| r.contains("fetch-wake-models")),
            "the remedy must name the command: {finding:?}"
        );
    }

    #[test]
    fn a_speaker_is_reported_as_unverifiable_rather_than_ready() {
        // The finding this project must not fake: ffmpeg exits 0 for a sink
        // that does not exist, so a green tick here would be a lie.
        let finding = super::playback_device(Some("NoSuchSink"));
        assert_eq!(finding.standing, Standing::Unverifiable);
        assert!(
            finding.detail.contains("exits 0"),
            "it must say why it cannot tell: {finding:?}"
        );
    }
}
