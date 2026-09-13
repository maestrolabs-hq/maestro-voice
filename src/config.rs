//! What the daemon was told, and where it was told it.
//!
//! One file of `key = value` lines, every one of which an environment variable
//! can override. The format is read by hand rather than by a crate: it is
//! fifteen scalar settings with no nesting, and a configuration language would
//! be a dependency to audit for structure nothing here has. If this ever grows
//! a table, that is the moment to take the dependency, not before.
//!
//! **Nothing here names a machine.** The configuration file, the model
//! directory and the audio devices are all resolved from the environment at run
//! time, because a path compiled in is a fact about the computer it was
//! compiled on.

mod file;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::endpoint::Rule;
use crate::wake::Detector;

/// The prefix every override carries: `wake_threshold` is
/// `MAESTRO_VOICE_WAKE_THRESHOLD`.
pub const PREFIX: &str = "MAESTRO_VOICE_";

/// Everything the daemon needs to run one turn after another.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// The score a chunk must pass for the wake phrase to count as spoken.
    pub wake_threshold: f32,
    /// How far back to rewind when the wake fires, so the first word survives.
    pub pre_roll: Duration,
    /// Trailing quiet that ends a finished sentence.
    pub silence: Duration,
    /// How long to wait for speech before abandoning a wake that led nowhere.
    pub lead_in: Duration,
    /// The longest one utterance may run.
    pub cap: Duration,
    /// Where the router answers, as host and port.
    pub router: String,
    /// The catalog entry that transcribes.
    pub transcriber: String,
    /// The catalog entry that speaks.
    pub synthesizer: String,
    /// The Herdr agent that receives transcripts.
    pub agent: String,
    /// The loopback port the agent's extension posts finished turns to.
    pub intake_port: u16,
    /// The capture device, or the audio server's default when unset.
    pub capture_device: Option<String>,
    /// The playback device, or the audio server's default when unset.
    pub playback_device: Option<String>,
    /// Where the wake-word weights were fetched to.
    pub wake_models: PathBuf,
    /// The language to answer in when transcription declines to report one.
    pub fallback_language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Measured, not conventional: against the committed corpus the wake
            // phrase peaks at 0.998402 and "Hey Travis" at 0.348132, so 0.5
            // sits 1.44x above the loudest thing that must not wake the daemon.
            // See `crate::wake::Detector::DEFAULT_THRESHOLD`.
            wake_threshold: Detector::DEFAULT_THRESHOLD,
            // The detector fires at the end of the phrase, by which time the
            // sentence after it has usually started.
            pre_roll: Duration::from_millis(300),
            silence: Duration::from_millis(800),
            lead_in: Duration::from_secs(3),
            cap: Duration::from_secs(30),
            router: "127.0.0.1:8080".to_owned(),
            transcriber: "whisper".to_owned(),
            synthesizer: "tts".to_owned(),
            agent: "voice".to_owned(),
            intake_port: 8722,
            capture_device: None,
            playback_device: None,
            wake_models: PathBuf::from("models/wake"),
            fallback_language: "en".to_owned(),
        }
    }
}

impl Config {
    /// The configuration file, from the environment.
    ///
    /// `MAESTRO_VOICE_CONFIG` names it outright. Otherwise it sits under the
    /// usual configuration directory, which is itself read from the
    /// environment rather than assumed.
    #[must_use]
    pub fn path(environment: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
        if let Some(named) = environment(&format!("{PREFIX}CONFIG")) {
            return Some(PathBuf::from(named));
        }
        if let Some(base) = environment("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(base).join("maestro-voice/config"));
        }
        environment("HOME").map(|home| PathBuf::from(home).join(".config/maestro-voice/config"))
    }

    /// Settings from a file's text, then from the environment.
    ///
    /// Unknown keys are reported rather than ignored: a misspelled setting that
    /// silently does nothing is how someone spends an evening wondering why
    /// their threshold had no effect.
    #[must_use]
    pub fn read(text: &str, environment: &dyn Fn(&str) -> Option<String>) -> Loaded {
        let mut settings = file::parse(text);
        let mut problems = settings.problems;

        for key in Self::keys() {
            if let Some(value) = environment(&file::variable(PREFIX, key)) {
                drop(settings.values.insert((*key).to_owned(), value));
            }
        }

        let config = Self::from_values(&settings.values, &mut problems);
        for unknown in settings.values.keys().filter(|k| !Self::known(k)) {
            problems.push(format!("'{unknown}' is not a setting this daemon has"));
        }
        Loaded { config, problems }
    }

    /// Every setting name, which is also what the override check iterates.
    #[must_use]
    pub fn keys() -> &'static [&'static str] {
        &[
            "wake_threshold",
            "pre_roll_ms",
            "silence_ms",
            "lead_in_ms",
            "cap_ms",
            "router",
            "transcriber",
            "synthesizer",
            "agent",
            "intake_port",
            "capture_device",
            "playback_device",
            "wake_models",
            "fallback_language",
        ]
    }

    fn known(key: &str) -> bool {
        Self::keys().contains(&key)
    }

    fn from_values(values: &BTreeMap<String, String>, problems: &mut Vec<String>) -> Self {
        let mut config = Self::default();
        let get = |key: &str| values.get(key).map(String::as_str);

        if let Some(raw) = get("wake_threshold") {
            match raw.parse::<f32>() {
                Ok(value) if (0.0..=1.0).contains(&value) => config.wake_threshold = value,
                _ => problems.push(format!(
                    "wake_threshold must be a score between 0 and 1, not '{raw}'"
                )),
            }
        }
        file::duration(
            get("pre_roll_ms"),
            "pre_roll_ms",
            &mut config.pre_roll,
            problems,
        );
        file::duration(
            get("silence_ms"),
            "silence_ms",
            &mut config.silence,
            problems,
        );
        file::duration(
            get("lead_in_ms"),
            "lead_in_ms",
            &mut config.lead_in,
            problems,
        );
        file::duration(get("cap_ms"), "cap_ms", &mut config.cap, problems);

        if let Some(raw) = get("intake_port") {
            match raw.parse::<u16>() {
                Ok(port) => config.intake_port = port,
                Err(_) => problems.push(format!("intake_port must be a port number, not '{raw}'")),
            }
        }

        file::text(get("router"), &mut config.router);
        file::text(get("transcriber"), &mut config.transcriber);
        file::text(get("synthesizer"), &mut config.synthesizer);
        file::text(get("agent"), &mut config.agent);
        file::text(get("fallback_language"), &mut config.fallback_language);
        config.capture_device = file::optional(get("capture_device"));
        config.playback_device = file::optional(get("playback_device"));
        if let Some(path) = file::optional(get("wake_models")) {
            config.wake_models = PathBuf::from(path);
        }
        config
    }

    /// The endpointing rule these settings describe.
    #[must_use]
    pub const fn rule(&self) -> Rule {
        Rule::new(self.silence, self.lead_in, self.cap)
    }
}

/// A configuration, and everything wrong with the way it was written.
///
/// Both together, because a daemon that refused to start over one misspelled
/// key would be worse than one that says so and runs on its defaults, and a
/// daemon that said nothing would be worse than either.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The settings, with defaults wherever a value was missing or unusable.
    pub config: Config,
    /// What could not be understood, in the order it was found.
    pub problems: Vec<String>,
}
