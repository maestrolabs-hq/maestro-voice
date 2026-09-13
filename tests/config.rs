//! Settings: what they default to, what overrides what, and what is refused.
//!
//! Driven through a fake environment rather than the real one, so the tests say
//! what they mean and do not depend on the machine they run on.

use maestro_voice::config::{Config, PREFIX};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

/// An environment built from pairs, and nothing else.
fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: BTreeMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name: &str| map.get(name).cloned()
}

fn read(text: &str, pairs: &[(&str, &str)]) -> maestro_voice::config::Loaded {
    Config::read(text, &environment(pairs))
}

#[test]
fn an_empty_configuration_is_the_documented_defaults() {
    let loaded = read("", &[]);

    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
    let config = loaded.config;
    assert!(
        (config.wake_threshold - 0.5).abs() < f32::EPSILON,
        "the measured default, not a round one"
    );
    assert_eq!(config.silence, Duration::from_millis(800));
    assert_eq!(config.lead_in, Duration::from_secs(3));
    assert_eq!(config.cap, Duration::from_secs(30));
    assert_eq!(config.pre_roll, Duration::from_millis(300));
    assert_eq!(config.transcriber, "whisper");
    assert_eq!(config.synthesizer, "tts");
}

#[test]
fn a_file_setting_replaces_the_default() {
    let loaded = read(
        "wake_threshold = 0.65\nagent = jarvis\nsilence_ms = 1200\n",
        &[],
    );

    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
    assert!((loaded.config.wake_threshold - 0.65).abs() < 1e-6);
    assert_eq!(loaded.config.agent, "jarvis");
    assert_eq!(loaded.config.silence, Duration::from_millis(1200));
}

#[test]
fn the_environment_wins_over_the_file() {
    // The point of an override: changing one setting for one run without
    // editing, and remembering to un-edit, a file.
    let loaded = read(
        "agent = fromfile\nrouter = 127.0.0.1:1\n",
        &[
            (&format!("{PREFIX}AGENT"), "fromenvironment"),
            (&format!("{PREFIX}ROUTER"), "127.0.0.1:9999"),
        ],
    );

    assert_eq!(loaded.config.agent, "fromenvironment");
    assert_eq!(loaded.config.router, "127.0.0.1:9999");
}

#[test]
fn a_misspelled_setting_is_reported_rather_than_ignored() {
    // Ignoring it is how someone spends an evening wondering why their
    // threshold had no effect.
    let loaded = read("wake_threshhold = 0.7\n", &[]);

    assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
    assert!(
        loaded.problems[0].contains("wake_threshhold"),
        "the report must name the key: {:?}",
        loaded.problems
    );
    assert!(
        (loaded.config.wake_threshold - 0.5).abs() < f32::EPSILON,
        "and the default must stand"
    );
}

#[test]
fn an_unusable_value_is_reported_and_the_default_stands() {
    // Refusing to start over one bad line would be worse than saying so and
    // running; saying nothing would be worse than either.
    let loaded = read(
        "wake_threshold = loud\nsilence_ms = ages\nintake_port = 99999\n",
        &[],
    );

    assert_eq!(loaded.problems.len(), 3, "{:?}", loaded.problems);
    assert!((loaded.config.wake_threshold - 0.5).abs() < f32::EPSILON);
    assert_eq!(loaded.config.silence, Duration::from_millis(800));
    assert_eq!(loaded.config.intake_port, 8722);
}

#[test]
fn a_threshold_outside_the_score_range_is_refused() {
    // A score is between zero and one. A threshold of 50 never fires and a
    // threshold of -1 fires on silence, and neither says so on its own.
    for impossible in ["-0.1", "1.5", "50"] {
        let loaded = read(&format!("wake_threshold = {impossible}\n"), &[]);
        assert_eq!(
            loaded.problems.len(),
            1,
            "{impossible} must be refused: {:?}",
            loaded.problems
        );
    }
}

#[test]
fn a_blank_device_means_the_audio_server_default() {
    let loaded = read("capture_device =\nplayback_device = RDPSink\n", &[]);

    assert_eq!(loaded.config.capture_device, None, "blank is not a choice");
    assert_eq!(loaded.config.playback_device.as_deref(), Some("RDPSink"));
}

#[test]
fn the_endpointing_rule_is_built_from_the_three_windows() {
    let loaded = read("silence_ms = 500\nlead_in_ms = 2000\ncap_ms = 10000\n", &[]);
    let mut utterance = loaded.config.rule().start();

    // Five frames of speech, then five of quiet, is 500ms of trailing silence.
    for _ in 0..5 {
        let _speaking = utterance.observe(Duration::from_millis(100), true);
    }
    let mut ended = false;
    for _ in 0..5 {
        if matches!(
            utterance.observe(Duration::from_millis(100), false),
            maestro_voice::endpoint::Decision::Ended(_)
        ) {
            ended = true;
        }
    }
    assert!(ended, "the configured silence window must be what applies");
}

#[test]
fn the_configuration_file_is_found_from_the_environment_and_never_compiled_in() {
    let named = Config::path(&environment(&[(
        &format!("{PREFIX}CONFIG"),
        "/somewhere/voice.conf",
    )]));
    assert_eq!(named, Some(PathBuf::from("/somewhere/voice.conf")));

    let xdg = Config::path(&environment(&[("XDG_CONFIG_HOME", "/somewhere/config")]));
    assert_eq!(
        xdg,
        Some(PathBuf::from("/somewhere/config/maestro-voice/config"))
    );

    let home = Config::path(&environment(&[("HOME", "/somewhere/person")]));
    assert_eq!(
        home,
        Some(PathBuf::from(
            "/somewhere/person/.config/maestro-voice/config"
        ))
    );

    assert_eq!(
        Config::path(&environment(&[])),
        None,
        "an environment that says nothing must not produce a guess"
    );
}

#[test]
fn every_setting_can_be_overridden_from_the_environment() {
    // The failure this prevents: a setting added to the struct and the file
    // parser but not to the list the override loop walks, which then works in
    // the file and silently ignores its variable.
    for key in Config::keys() {
        let value = match *key {
            "wake_threshold" => "0.75",
            "intake_port" => "9001",
            key if key.ends_with("_ms") => "1234",
            _ => "overridden",
        };
        let loaded = read("", &[(&format!("{PREFIX}{}", key.to_uppercase()), value)]);
        assert!(
            loaded.problems.is_empty(),
            "{key} must be settable from the environment: {:?}",
            loaded.problems
        );
    }
}
