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
//! | worst observed runtime floor | 1.013279e-06 | 39.5x below | -- |
//! | warm-up zeroing removed | 2.4437904e-06 | 16.4x **below** | exact-zero only |
//! | head reads one feature row too far back | 3.272642e-01 | 8182x above | peak-chunk, decisions **and** numeric |
//! | mel offset 2.02 instead of 2.0 | 1.6325712e-03 | 40.8x above | numeric only |
//! | mel buffer seeded with zeros | 4.4582963e-02 | 1115x above | numeric only |
//!
//! # How [`TOLERANCE`] is derived, and the assumption it rests on
//!
//! Only two faults constrain this constant: the mel offset and the mel buffer,
//! because they are the only ones the numeric test catches *alone*. The
//! warm-up fault does not constrain it, and calibrating as though it did is
//! what made the previous constant needlessly tight.
//!
//! Geometric mean of the worst floor observed on any machine, 1.013279e-06,
//! and the tightest fault this test alone must catch, 1.6325712e-03, is
//! 4.067247e-05. [`TOLERANCE`] is 4e-05: that value rounded *down*, because
//! rounding down tightens the gate. It leaves 39.5x above the worst floor and
//! 40.8x below the mel offset -- deliberately symmetric, so the constant is
//! defensible on a machine nobody has measured yet.
//!
//! **This is safe only because the exact-zero test catches the warm-up fault
//! independently.** That fault is 16.4x *below* this tolerance and the numeric
//! test can no longer see it at all. If
//! [`the_warm_up_scores_are_exactly_zero`] is removed, weakened, or stops
//! being run, this constant is invalid and must be re-derived from scratch
//! against whatever faults the numeric test is then alone in catching. That
//! sentence is the load-bearing assumption of the whole gate.
//!
//! # The tolerance is not carrying this alone
//!
//! Three further tests assert properties the tolerance cannot express: the
//! warm-up scores are *exactly* zero, the peak lands on the same chunk, and
//! the detector reaches the same decisions. Those are orthogonal to the
//! tolerance rather than weaker versions of it. The warm-up fault turns exact
//! zeros into small non-zeros, which no tolerance question can hide.
//!
//! **Where this gate is thin, stated plainly.** Two of the four faults are
//! caught by the numeric test alone, and one -- the warm-up fault -- is now
//! caught by the exact-zero test alone. Every fault has exactly one or more
//! gate and none has none, which is the property that matters; but two of the
//! three gates are now single points of failure rather than one. All four
//! tests have been observed failing: the framing shift exists in that table
//! because peak-chunk and decisions caught none of the other three, and a test
//! never seen to fail is not yet a gate.
//!
//! # Observed floors, one row per machine
//!
//! ONNX Runtime dispatches on CPU features at run time, so the floor is a
//! property of the machine, not a constant of nature. The first version of
//! this file calibrated against a single machine and the gate broke the first
//! time it met a second one. Adding a row here when a new machine appears is
//! what "re-measure and widen with recorded evidence" means in practice.
//!
//! Both sides run ONNX Runtime 1.28.0 throughout; only the build differs, and
//! the reference column is fixed because `reference_scores.txt` was recorded
//! once, on the development machine, with Microsoft's published wheel.
//!
//! | machine | CPU | this crate's build | observed maximum divergence |
//! | --- | --- | --- | --- |
//! | development | AMD Ryzen 7 9800X3D | pyke prebuilt, static | 3.278e-07 |
//! | GitHub hosted `ubuntu-latest` | see the canary note in a run log | pyke prebuilt, static | 1.013279e-06 |
//!
//! Two machines already differ by 3.1x. That spread, not either number, is
//! why the bound is set with symmetric margin rather than just above the
//! largest floor seen so far.
//!
//! # If the floor moves
//!
//! Re-measure, add a row above, and widen from the recorded evidence. Never
//! nudge the constant until the suite goes green.
//!
//! The line at which widening stops being the answer is a measurable event
//! rather than a feeling: **when an observed floor reaches 1.6e-04** it is
//! within an order of magnitude of the mel offset at 1.6325712e-03, the
//! tightest fault this test alone catches, and the numeric comparison has
//! stopped separating runtime noise from a real fault. Reaching it would mean
//! runtime noise had grown roughly 160x from today's worst observation, and
//! the answer then is not another widening.
//!
//! It is also not pinning the runtime. That was this file's previous answer
//! and it was written for the wrong cause: it removes the *build* variable,
//! while what moved here was the *CPU*. Both sides already ran 1.28.0 when the
//! floor tripled. Pinning would cost an ONNX Runtime built from source on
//! every clone and leave the test free to break on a third machine.
//!
//! The real answer, unproven and needing its own spike, is to make ONNX
//! Runtime deterministic across CPUs by constraining graph optimisation or
//! kernel selection. If that works it would restore exact equality, which is
//! strictly better than any tolerance. That is the condition that reopens this
//! decision.
//!
//! The weights this needs are not in the repository, by licence. See
//! `docs/adr/0003-the-wake-word-runtime-and-its-weights.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use maestro_voice::wake::{CHUNK, Detector, ModelSet, SAMPLE_RATE, Scorer};

/// The largest score difference attributable to the machine rather than to a
/// fault. Derived in this file's header; do not widen without re-measuring.
///
/// Valid only while [`the_warm_up_scores_are_exactly_zero`] runs: that test,
/// not this bound, is what catches the warm-up fault.
const TOLERANCE: f32 = 4e-05;

/// The fraction of [`TOLERANCE`] above which an observed divergence is
/// reported without failing.
///
/// Without this the floor is only ever learned when the gate breaks, which is
/// how the first calibration reached a pull request as a red check instead of
/// as a warning. A machine drifting toward the bound now says so while the
/// suite is still green.
const CANARY_FRACTION: f32 = 0.2;

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

    let mut observed = 0.0_f32;

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
        observed = observed.max(worst);

        assert!(
            worst <= TOLERANCE,
            "{name}: diverged from openWakeWord by {worst:e}, over the {TOLERANCE:e} \
             bound.\nThat is larger than any floor this bound was measured \
             against, so it is a fault rather than noise. See this file's \
             header, and add a row to its table of observed floors before \
             considering any change to the bound."
        );
    }

    report_floor(observed);
}

/// Say what the floor was on this machine when it climbs toward the bound.
///
/// Printed rather than asserted: a machine whose floor is merely higher than
/// the recorded ones has not failed, it has produced the evidence the header's
/// table wants. `cargo test` shows this only for a failing test, so the note
/// is written to stderr, which `--nocapture` and every CI log surface.
fn report_floor(observed: f32) {
    if observed < TOLERANCE * CANARY_FRACTION {
        return;
    }

    eprintln!(
        "wake_equivalence canary: this machine's floor is {observed:e}, above \
         {CANARY_FRACTION} of the {TOLERANCE:e} bound.\n\
         The gate still passes. Add a row to the table of observed floors in \
         this file's header, naming the CPU below, so the next calibration has \
         the evidence.\n\
         CPU: {}",
        cpu_name().unwrap_or_else(|| "unknown".to_owned())
    );
}

/// The CPU this ran on, for the header's table.
///
/// The floor depends on which kernels ONNX Runtime dispatches to, so a row in
/// that table means nothing without naming the processor it was measured on.
/// Linux only, and absent elsewhere rather than guessed: this crate claims one
/// platform, and a wrong name in an evidence table is worse than no name.
fn cpu_name() -> Option<String> {
    let info = fs::read_to_string("/proc/cpuinfo").ok()?;
    info.lines()
        .find(|line| line.starts_with("model name"))
        .and_then(|line| line.split_once(':'))
        .map(|(_, name)| name.trim().to_owned())
}

/// Orthogonal to the tolerance, and now the *only* thing standing between the
/// warm-up fault and a green suite.
///
/// That fault moves the scores by 2.4437904e-06, which is 16.4x **below**
/// [`TOLERANCE`]: the numeric test cannot see it any more. This one catches it
/// at any magnitude at all, because the fault turns exact zeros into small
/// non-zeros and no tolerance question can hide that.
///
/// Deleting or weakening this test therefore invalidates [`TOLERANCE`], which
/// is derived on the assumption that this test exists. The header says so too.
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
