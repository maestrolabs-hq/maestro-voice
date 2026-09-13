//! The `maestro-voice` binary: `check` says what is not ready, `run` listens.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use maestro_voice::agent::Herdr;
use maestro_voice::capture::{Restart, Sleep, Supervised, pulse};
use maestro_voice::check;
use maestro_voice::config::{Config, Loaded};
use maestro_voice::listen::Listener;
use maestro_voice::runner::{Daemon, Services, Stopped};
use maestro_voice::service::Waker;
use maestro_voice::speak::Language;
use maestro_voice::speaker::Speaker;
use maestro_voice::speech::Router;
use maestro_voice::turn::Machine;
use maestro_voice::voice::Gate;
use maestro_voice::wake::ModelSet;

mod serve;

fn main() -> ExitCode {
    // The argument selects a subcommand and nothing else: it is matched below
    // against a closed set, and anything outside it is refused. It grants no
    // privilege, names no path, and authenticates nobody, so the property
    // `rust.lang.security.args.args` exists to protect -- that argv is
    // attacker-controlled and must not be trusted for security -- is not one
    // this line relies on. The rule reads argv[0], the executable path; this
    // reads argv[1].
    // nosemgrep: rust.lang.security.args.args
    let command = env::args().nth(1).unwrap_or_else(|| "check".to_owned());
    let loaded = settings();

    for problem in &loaded.problems {
        eprintln!("maestro-voice: {problem}");
    }

    match command.as_str() {
        "check" => check_command(&loaded.config),
        "run" => serve::run(&loaded.config),
        "help" | "--help" | "-h" => {
            usage();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("maestro-voice: '{other}' is not a command");
            usage();
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    println!("maestro-voice {}", env!("CARGO_PKG_VERSION"));
    println!("  check   report what is ready and what is not, starting nothing");
    println!("  run     listen for the wake word and serve turns");
    println!();
    println!("Settings are read from the file named by MAESTRO_VOICE_CONFIG, or");
    println!("from maestro-voice/config under the usual configuration directory.");
    println!("Every setting can be overridden by MAESTRO_VOICE_<SETTING>.");
}

/// The configuration, from wherever the environment says it lives.
fn settings() -> Loaded {
    let path = Config::path(&|name| env::var(name).ok());
    let text = path
        .as_deref()
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default();
    Config::read(&text, &|name| env::var(name).ok())
}

fn check_command(config: &Config) -> ExitCode {
    let findings = check::report(config);
    for finding in &findings {
        println!("{finding}");
    }

    if check::any_problem(&findings) {
        println!();
        println!("Not ready. Each line marked PROBLEM says what would fix it.");
        return ExitCode::FAILURE;
    }
    println!();
    println!("Ready. Lines marked unknown are not failures: they are things this");
    println!("check cannot determine, and they say why.");
    ExitCode::SUCCESS
}

/// Build everything the daemon needs, or say what stopped it.
///
/// Kept here rather than in `serve` so that the wiring -- which service is
/// which -- reads in one place.
fn assemble(config: &Config) -> Result<(Daemon, Box<dyn Waker>), String> {
    let language = Language::new(&config.fallback_language)
        .ok_or_else(|| format!("'{}' is not a language tag", config.fallback_language))?;

    let models = ModelSet::at(&PathBuf::from(&config.wake_models))
        .map_err(|missing| format!("{missing}\nRun 'just fetch-wake-models' to download them."))?;
    let listener = Listener::open(&models, config.wake_threshold)
        .map_err(|why| format!("the wake detector would not start: {why}"))?;

    let router = Arc::new(Router::new(
        &config.router,
        &config.transcriber,
        &config.synthesizer,
    ));
    let services = Services {
        transcriber: router.clone(),
        synthesizer: router,
        courier: Arc::new(Herdr::new(&config.agent)),
        player: Arc::new(Speaker::new(config.playback_device.as_deref())),
    };

    let daemon = Daemon::new(
        Machine::new(config.rule()),
        services,
        config.pre_roll,
        language,
    );
    Ok((daemon, Box::new(listener)))
}

/// How many times a dead capture is reopened before the daemon says so.
///
/// Five, the first retry after a quarter second and each one waiting twice as
/// long. Enough to ride out an audio server restarting; few enough that a
/// microphone which is genuinely gone is announced rather than retried in
/// silence for ever.
const CAPTURE_ATTEMPTS: u32 = 5;

/// A microphone that reopens itself when `ffmpeg` dies, and gives up eventually.
fn microphone(config: &Config) -> Supervised<Reopener, Sleep> {
    let device = pulse::device_or_default(config.capture_device.as_deref()).to_owned();
    let policy = Restart::allowing(CAPTURE_ATTEMPTS, Duration::from_millis(250));
    Supervised::new(Reopener { device }, policy, Sleep)
}

/// Opens the capture device again after it has failed.
struct Reopener {
    device: String,
}

impl maestro_voice::capture::Reopen for Reopener {
    fn reopen(
        &mut self,
    ) -> Result<Box<dyn maestro_voice::capture::Source>, maestro_voice::capture::Error> {
        let source = pulse::PulseSource::open(pulse::PROGRAM, &self.device)?;
        Ok(Box::new(source))
    }
}

/// What the daemon printed on its way out.
fn farewell(stopped: Stopped) -> ExitCode {
    match stopped {
        Stopped::CaptureGaveUp => {
            eprintln!("maestro-voice: the microphone stopped and did not come back");
            ExitCode::FAILURE
        }
        Stopped::AudioEnded => {
            eprintln!("maestro-voice: the audio ended");
            ExitCode::SUCCESS
        }
    }
}

/// The speech gate, whose floor adapts to whatever room this is.
fn ears() -> Gate {
    Gate::default()
}
