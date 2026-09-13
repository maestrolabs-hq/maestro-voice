//! The checksum manifest, and the guard that keeps it from rotting.
//!
//! This repository does not redistribute the wake-word weights: openWakeWord
//! licenses its pretrained models under CC BY-NC-SA 4.0, which cannot sit
//! inside a tree that declares itself MIT. They are fetched instead, and the
//! committed manifest is what replaces the guarantee that committing them
//! would have given -- any run can still say exactly which bytes it used.
//!
//! A manifest nobody checks is a list of hopes, so three things are asserted
//! here: the manifest names exactly the models the code requires, every line
//! is a well-formed entry, and every file on disk matches the sum recorded for
//! it. A fifth file appearing, a name changing, or a weight being swapped all
//! fail. See `docs/adr/0003-the-wake-word-runtime-and-its-weights.md`.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// The command that fetches and verifies the weights.
const REMEDY: &str = "just fetch-wake-models";

/// Exactly the models the wake stage loads.
///
/// Silero's voice-activity model is deliberately absent: this stage does not
/// use it, and the copy that belongs in this project comes from a different
/// package. See the manifest's own note.
const EXPECTED: [&str; 3] = [
    "melspectrogram.onnx",
    "embedding_model.onnx",
    "hey_jarvis_v0.1.onnx",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest_path() -> PathBuf {
    repo_root().join("tests/fixtures/wake/models.sha256")
}

fn models_dir() -> PathBuf {
    repo_root().join("models/wake")
}

/// The manifest as `(sha256, file name)` pairs, comments and blanks dropped.
fn manifest() -> Vec<(String, String)> {
    let text = fs::read_to_string(manifest_path()).expect("the manifest is committed");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (sum, name) = line
                .split_once(char::is_whitespace)
                .unwrap_or_else(|| panic!("not a checksum line: {line}"));
            (sum.trim().to_owned(), name.trim().to_owned())
        })
        .collect()
}

/// The allowlist cannot quietly grow. A model added to the manifest without
/// being added to the code, or the reverse, fails here.
#[test]
fn the_manifest_names_exactly_the_models_the_code_requires() {
    let mut named: Vec<String> = manifest().into_iter().map(|(_, name)| name).collect();
    named.sort();

    let mut expected: Vec<String> = EXPECTED.iter().map(|&n| n.to_owned()).collect();
    expected.sort();

    assert_eq!(
        named, expected,
        "the manifest and the code must name the same models"
    );
}

#[test]
fn every_manifest_entry_is_a_well_formed_sha256() {
    for (sum, name) in manifest() {
        assert_eq!(
            sum.len(),
            64,
            "{name}: a sha256 is 64 hex characters: {sum}"
        );
        assert!(
            sum.chars().all(|c| c.is_ascii_hexdigit()),
            "{name}: not hexadecimal: {sum}"
        );
    }
}

/// The test that makes the manifest mean something.
///
/// It fails rather than skips when the weights are absent. A test that quietly
/// does not run is the failure `docs/adr/0001` already refused, so absence is
/// reported with the command that fixes it.
#[test]
fn every_model_matches_its_recorded_checksum() {
    let directory = models_dir();

    for (expected_sum, name) in manifest() {
        let path = directory.join(&name);
        let bytes = fs::read(&path).unwrap_or_else(|error| {
            let where_it_looked = path.display();
            panic!(
                "cannot read the wake-word weights: {where_it_looked} ({error}).\n\
                 This repository does not ship them, by licence. Run \
                 `{REMEDY}` to fetch and verify them."
            )
        });

        assert_eq!(
            sha256(&bytes),
            expected_sum,
            "{name} does not match the manifest; re-run `{REMEDY}`"
        );
    }
}

/// A SHA-256 as lowercase hexadecimal.
fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

/// The digest must be right, or every comparison above passes vacuously.
/// These are the FIPS 180-4 examples.
#[test]
fn the_digest_agrees_with_the_published_vectors() {
    assert_eq!(
        sha256(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}
