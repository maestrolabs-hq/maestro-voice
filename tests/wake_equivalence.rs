//! The Rust pipeline must agree with openWakeWord, and this file defines what
//! agreement means and why.
//!
//! This is the test the port exists to satisfy. Every way of getting the
//! framing wrong produces plausible numbers rather than an error, so without
//! something holding the port to the reference it degrades in silence.
//!
//! # Why this is a tolerance and not exact equality
//!
//! A spike got bit-for-bit agreement, but only because the Rust binary loaded
//! Python's own `libonnxruntime.so` out of a virtual environment. That is the
//! arrangement `docs/adr/0003` rejected as unshippable. The shipped binary
//! statically links its own ONNX Runtime, so the two sides now run different
//! builds, and asking two builds to agree bit for bit is asking two compilers
//! to agree on float rounding across different SIMD paths. Neither promises
//! that.
//!
//! | side | ONNX Runtime |
//! | --- | --- |
//! | this crate | 1.28.0, pyke's prebuilt, statically linked by `ort` |
//! | `reference_scores.txt` | 1.28.0, Microsoft's published wheel, via `openwakeword` |
//!
//! Same version, different builds. Matching the versions removed most of the
//! divergence and did not remove it.
//!
//! # The evidence behind the number
//!
//! The floor was measured across the whole corpus, 137 chunks, 60 of them
//! differing. The faults below were then injected into a working port and run
//! against this file as it ships -- not inferred from magnitudes measured
//! under the earlier exact-equality gate, and not taken from the spike, whose
//! 2.861e-06 was measured on a different corpus.
//!
//! | quantity | value | against the tolerance | caught by |
//! | --- | --- | --- | --- |
//! | runtime-build floor | 3.278e-07 | 3.05x below | -- |
//! | warm-up zeroing removed | 2.4437904e-06 | 2.44x above | exact-zero **and** numeric |
//! | head reads one feature row too far back | 3.272642e-01 | 327264x above | peak-chunk, decisions **and** numeric |
//! | mel offset 2.02 instead of 2.0 | 1.6325712e-03 | 1633x above | numeric only |
//! | mel buffer seeded with zeros | 4.4582963e-02 | 44583x above | numeric only |
//!
//! [`TOLERANCE`] is 1e-6 because the geometric mean of the floor and the
//! tightest fault is 8.9503e-07: the point that maximises the ratio of margin
//! on both sides at once. It is derived, not rounded to something comfortable.
//!
//! # The tolerance is not carrying this alone
//!
//! Its margin against the tightest fault is only 2.44x, so three further tests
//! below assert properties the tolerance cannot express: the warm-up scores are
//! *exactly* zero, the peak lands on the same chunk, and the detector reaches
//! the same decisions. Those are orthogonal to the tolerance rather than weaker
//! versions of it. The warm-up fault, which the tolerance has least room
//! against, is caught by the exact-zero test at any magnitude whatsoever --
//! that fault turns exact zeros into small non-zeros, which no tolerance
//! question can hide. If the numeric test below is ever loosened, that
//! reasoning must not leave with it.
//!
//! **Where this gate is thin, stated plainly.** Two of the four faults are
//! caught by the numeric test alone. That is tolerable only because their
//! margins are 1633x and 44583x, so no plausible re-measurement of the floor
//! reaches them; redundancy sits where the margin is narrowest, which is the
//! right way round. All four tests have been observed failing: the framing
//! shift exists in that table because peak-chunk and decisions caught none of
//! the other three, and a test never seen to fail is not yet a gate.
//!
//! # If the floor moves
//!
//! ONNX Runtime dispatches on CPU features at run time, so a machine with a
//! different instruction set may have a different floor. 3.278e-07 is one
//! machine's measurement, not a constant of nature.
//!
//! The response is fixed in advance so nobody improvises it: re-measure the
//! floor and widen with the recorded evidence, never nudge the constant until
//! the suite goes green. And the line at which widening stops being the answer
//! is a measurable event rather than a feeling -- **when a re-measured floor
//! reaches 1e-6 it has met the tolerance**, the numeric comparison no longer
//! separates runtime noise from a real fault, and the correct response is to
//! pin the runtime so both sides load one binary, which `docs/adr/0003` keeps
//! as its documented escape hatch. Today the floor sits 3.05x under that line
//! and the tightest fault 2.44x over it.
//!
//! The weights this needs are not in the repository, by licence. See
//! `docs/adr/0003-the-wake-word-runtime-and-its-weights.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use maestro_voice::wake::{CHUNK, Detector, ModelSet, SAMPLE_RATE, Scorer};

/// The largest score difference attributable to the runtime build rather than
/// to a fault. Derived in this file's header; do not widen without re-measuring.
const TOLERANCE: f32 = 1e-6;

/// `model.py`: "zero predictions for first 5 frames during model
/// initialization". Exact, not approximate.
const WARMUP: usize = 5;

const REMEDY: &str = "just fetch-wake-models";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures() -> PathBuf {
    repo_root().join("tests/fixtures/wake")
}

/// The recorded scores, as the exact `f32` values openWakeWord produced.
///
/// Stored as bit patterns rather than decimal text. The comparison is now a
/// tolerance, which makes this matter more rather than less: decimal
/// round-tripping between two languages would quietly consume part of a margin
/// that is only 3x wide, and nobody would ever see it happen.
fn reference() -> BTreeMap<String, Vec<f32>> {
    let path = fixtures().join("reference_scores.txt");
    let text = fs::read_to_string(&path).expect("the reference scores are committed");

    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next().expect("a file name").to_owned();
            let scores = fields
                .map(|hex| {
                    let bits = u32::from_str_radix(hex, 16).expect("a hexadecimal bit pattern");
                    f32::from_bits(bits.swap_bytes())
                })
                .collect();
            (name, scores)
        })
        .collect()
}

/// The samples of one 16 kHz mono fixture.
fn samples(path: &Path) -> Vec<i16> {
    let mut reader = hound::WavReader::open(path).expect("a committed fixture opens");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "{}: mono only", path.display());
    assert_eq!(
        spec.sample_rate as usize,
        SAMPLE_RATE,
        "{}: 16 kHz only",
        path.display()
    );
    reader
        .samples::<i16>()
        .map(|sample| sample.expect("a sample"))
        .collect()
}

fn models() -> ModelSet {
    ModelSet::at(&repo_root().join("models/wake")).unwrap_or_else(|missing| {
        panic!("{missing}\n\nThis test cannot run without them. Run `{REMEDY}`.")
    })
}

/// Score one fixture the way a live microphone would deliver it.
fn scores_for(models: &ModelSet, name: &str) -> Vec<f32> {
    let audio = samples(&fixtures().join("corpus").join(name));
    let mut scorer = Scorer::open(models).expect("the models load");

    audio
        .chunks_exact(CHUNK)
        .map(|chunk| scorer.push(chunk).expect("a score"))
        .collect()
}

/// The index of the highest score, which is where the phrase was heard.
fn peak_chunk(scores: &[f32]) -> usize {
    scores
        .iter()
        .enumerate()
        .fold((0, f32::MIN), |(best, high), (index, &score)| {
            if score > high {
                (index, score)
            } else {
                (best, high)
            }
        })
        .0
}

/// Which chunks the detector would have woken on.
fn decisions(scores: &[f32]) -> Vec<bool> {
    let mut detector = Detector::new(Detector::DEFAULT_THRESHOLD);
    scores
        .iter()
        .map(|&score| detector.observe(score))
        .collect()
}

/// The numeric comparison. Read this file's header before changing the bound.
#[test]
fn every_fixture_scores_within_the_measured_runtime_floor() {
    let models = models();
    let reference = reference();
    assert!(
        !reference.is_empty(),
        "the reference file named no fixtures"
    );

    for (name, expected) in reference {
        let actual = scores_for(&models, &name);

        assert_eq!(
            actual.len(),
            expected.len(),
            "{name}: chunk count differs, so the framing differs"
        );

        let worst = actual
            .iter()
            .zip(&expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);

        assert!(
            worst <= TOLERANCE,
            "{name}: diverged from openWakeWord by {worst:e}, over the {TOLERANCE:e} \
             bound.\nThat is larger than the runtime-build floor this bound was \
             measured against, so it is a fault rather than noise. See this \
             file's header."
        );
    }
}

/// Orthogonal to the tolerance, and the only thing standing between the
/// warm-up fault and a green suite: the bound has just 2.9x of margin against
/// that fault, while this test catches it at any magnitude at all.
#[test]
fn the_warm_up_scores_are_exactly_zero() {
    let models = models();

    for (name, expected) in reference() {
        let actual = scores_for(&models, &name);

        // Exactly zero is the assertion: a near-zero here is the fault, which
        // is the whole reason this test exists beside a tolerance.
        let zeroed = |scores: &[f32]| scores[..WARMUP].iter().all(|&score| score == 0.0);

        assert!(
            zeroed(&expected),
            "{name}: the reference itself lost its warm-up zeros, so it is not a \
             reference any more"
        );
        assert!(
            zeroed(&actual),
            "{name}: the first {WARMUP} scores must be exactly 0.0, not merely \
             small. Got {:?}",
            &actual[..WARMUP]
        );
    }
}

/// Hold one property of the score series equal across every fixture.
///
/// The two tests below stay separate rather than becoming one parameterised
/// test: they assert different things, and each has to be able to fail, and be
/// seen to fail, on its own. Only the loop is shared.
fn every_fixture<T>(project: impl Fn(&[f32]) -> T, complaint: &str)
where
    T: PartialEq + std::fmt::Debug,
{
    let models = models();

    for (name, expected) in reference() {
        let actual = scores_for(&models, &name);

        assert_eq!(project(&actual), project(&expected), "{name}: {complaint}");
    }
}

/// Where the phrase was heard, which no tolerance expresses. A port that got
/// the buffering right but the ordering wrong scores close numbers at the
/// wrong moments.
#[test]
fn the_peak_lands_on_the_same_chunk_as_the_reference() {
    every_fixture(
        peak_chunk,
        "the loudest moment moved, so the framing shifted",
    );
}

/// What the scores are actually for. Numbers may differ in their last bits;
/// the decision taken from them may not differ at all.
#[test]
fn the_detector_reaches_the_same_decisions_as_the_reference() {
    every_fixture(
        decisions,
        "the port and the reference would wake on different chunks",
    );
}

/// The behaviour stated as behaviour rather than as numbers.
///
/// The near miss is the valuable fixture: "Hey Travis" peaks at 0.348132, so
/// the default threshold of 0.5 has real margin while 0.3 would fire on it.
#[test]
fn only_the_wake_phrase_wakes_the_detector() {
    let models = models();

    let woke = |name: &str| {
        decisions(&scores_for(&models, name))
            .into_iter()
            .filter(|&f| f)
            .count()
    };

    assert_eq!(woke("wake_hey_jarvis_en.wav"), 1, "the phrase must wake it");
    for quiet in [
        "near_miss_en.wav",
        "negative_speech_en.wav",
        "negative_speech_fr.wav",
        "room_tone.wav",
    ] {
        assert_eq!(woke(quiet), 0, "{quiet} must not wake it");
    }
}

/// French speech must be as silent here as English speech. The daemon is
/// bilingual and the wake head is not: an English-trained model firing on
/// ordinary French would make the wake word unusable half the time.
#[test]
fn french_speech_stays_far_below_the_threshold() {
    let peak = scores_for(&models(), "negative_speech_fr.wav")
        .into_iter()
        .fold(f32::MIN, f32::max);

    assert!(
        peak < 0.01,
        "French speech peaked at {peak}, which is close enough to matter"
    );
}
