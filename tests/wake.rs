//! The wake-word stage, tested where it can be tested without a device.
//!
//! Two behaviours live here. The firing rule is pure arithmetic over a score
//! series and needs neither a microphone nor a model, so it is exercised
//! directly. Locating the model set touches the filesystem but not the models,
//! so its failure message is exercised against a directory that does not
//! exist.
//!
//! The exact-equality comparison against openWakeWord's own scores needs the
//! model weights, which this repository does not redistribute. It lives beside
//! these tests and fetches its fixtures; see `docs/adr/0003`.

use std::path::Path;

use maestro_voice::wake::{Detector, ModelSet};

/// Saying the phrase holds the score above the threshold for several chunks in
/// a row. A detector that reported each of them would deliver the same
/// utterance to the agent once per eighty milliseconds, so the rule is one
/// report per crossing, not one per chunk above the line.
#[test]
fn the_detector_fires_once_per_crossing_rather_than_once_per_chunk() {
    let mut detector = Detector::new(0.5);
    let saying_the_phrase = [0.01, 0.04, 0.62, 0.91, 0.88, 0.71, 0.53];

    let fired: Vec<bool> = saying_the_phrase
        .iter()
        .map(|&score| detector.observe(score))
        .collect();

    assert_eq!(
        fired,
        vec![false, false, true, false, false, false, false],
        "one report on the way up, and silence while the score stays high"
    );
}

/// Once the score falls back under the threshold the detector rearms, because
/// the phrase said twice is two wakes.
#[test]
fn the_detector_rearms_after_the_score_falls_back() {
    let mut detector = Detector::new(0.5);
    let twice = [0.7, 0.2, 0.9];

    let fired: Vec<bool> = twice.iter().map(|&s| detector.observe(s)).collect();

    assert_eq!(
        fired,
        vec![true, false, true],
        "falling below the threshold rearms; the second phrase wakes again"
    );
}

/// Exactly at the threshold counts as below it. An arbitrary choice, made once
/// and written down, so that a threshold of 0.0 does not fire on silence.
#[test]
fn the_threshold_itself_does_not_fire() {
    let mut detector = Detector::new(0.5);

    assert!(
        !detector.observe(0.5),
        "equal to the threshold is not above it"
    );
    assert!(detector.observe(0.500_01), "just above it is above it");
}

/// A missing model set is the most likely first-run failure, and the daemon
/// must say what to do about it rather than panic or report a linker error.
/// The message has to carry three things: which file, where it looked, and the
/// command that fixes it.
#[test]
fn a_missing_model_set_names_the_file_the_place_and_the_remedy() {
    let nowhere = Path::new("/somewhere/wake-models");

    let error = ModelSet::at(nowhere).expect_err("no model set exists at a synthetic path");
    let message = error.to_string();

    assert!(
        message.contains("melspectrogram.onnx"),
        "the message must name the missing file: {message}"
    );
    assert!(
        message.contains("/somewhere/wake-models"),
        "the message must say where it looked: {message}"
    );
    assert!(
        message.contains("just fetch-wake-models"),
        "the message must name the command that fixes it: {message}"
    );
}
