//! The two project checks the Rust profile names in its fast tier:
//! `module_size` and `architecture_boundaries`. Tests, not promises.
//!
//! The profile mandates both but ships neither, because neither can be written
//! without knowing what this repository's layers are. That binding is here.

use std::fs;
use std::path::Path;

mod common;
use common::{has_extension, repo_root, sources};

/// `limits.module_physical_lines` from `profiles/base/rust.yaml`.
///
/// Counted as the profile states it: physical lines, the whole file. Rust
/// colocates unit tests with the code they cover, so this charges a module for
/// its own tests. That is a real cost and it is the intended one -- a module
/// whose tests no longer fit is a module doing too much to test in one place.
///
/// It is a dumping-ground tripwire, not a design rule. The design rules are
/// per-function and live in `Cargo.toml`: `too_many_lines`,
/// `cognitive_complexity`, `too_many_arguments`.
const MAX_MODULE_LINES: usize = 250;

/// The decision layer: modules that turn observations into choices.
///
/// These exist so the behaviour that is hardest to test can be reached without
/// a microphone, which is the whole point of
/// `docs/adr/0001-the-capture-source-is-a-seam.md`. A rule module that opens a
/// file, a socket or a process has quietly undone that, and the failure would
/// not show up as a broken test -- it would show up as a test nobody could
/// write.
const DECISION_MODULES: &[&str] = &["endpoint.rs"];

/// What the decision layer may not reach for.
///
/// Spelled as path prefixes rather than whole lines so that `use std::io::Read`
/// and `std::io::stdin()` are both caught.
const FORBIDDEN_IN_DECISIONS: &[&str] =
    &["std::process", "std::net", "std::fs", "std::io", "std::env"];

fn module_files() -> Vec<std::path::PathBuf> {
    let src = repo_root().join("src");
    sources()
        .into_iter()
        .filter(|p| p.starts_with(&src) && has_extension(p, &["rs"]))
        .collect()
}

#[test]
fn no_module_becomes_a_dumping_ground() {
    let files = module_files();
    assert!(!files.is_empty(), "no module found under src");

    let over: Vec<String> = files
        .iter()
        .filter_map(|path| {
            let count = fs::read_to_string(path).expect("read").lines().count();
            (count > MAX_MODULE_LINES).then(|| {
                format!(
                    "  {}: {count} lines (max {MAX_MODULE_LINES})",
                    path.display()
                )
            })
        })
        .collect();
    assert!(over.is_empty(), "Module too large:\n{}\n", over.join("\n"));
}

/// The list above cannot rot into a lie: a module that is renamed or deleted
/// makes this fail rather than silently stopping being checked.
#[test]
fn every_named_decision_module_exists() {
    let src = repo_root().join("src");
    let missing: Vec<&str> = DECISION_MODULES
        .iter()
        .copied()
        .filter(|name| !src.join(name).is_file())
        .collect();
    assert!(
        missing.is_empty(),
        "DECISION_MODULES names modules that do not exist: {missing:?}"
    );
}

#[test]
fn the_decision_layer_performs_no_input_or_output() {
    let src = repo_root().join("src");
    let mut found = Vec::new();

    for name in DECISION_MODULES {
        let path = src.join(name);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            for forbidden in FORBIDDEN_IN_DECISIONS {
                if line.contains(forbidden) {
                    found.push(format!("  {name}:{}: {}", n + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        found.is_empty(),
        "The decision layer must stay reachable without a device, so it may not \
         use {FORBIDDEN_IN_DECISIONS:?}:\n\n{}\n",
        found.join("\n")
    );
}

/// Prose may name a forbidden module while explaining why it is forbidden;
/// only code counts.
fn is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("*")
}

/// A gate that cannot fail is not a gate. This proves the boundary check reads
/// what it claims to read, without needing a real violation committed to `src`.
#[test]
fn the_boundary_check_would_catch_a_violation() {
    let offending = "    let file = std::fs::read_to_string(path);";
    assert!(
        !is_comment(offending) && FORBIDDEN_IN_DECISIONS.iter().any(|f| offending.contains(f)),
        "the boundary check must flag a real filesystem call"
    );

    let explained = "//! never reaches for std::fs, by design";
    assert!(
        is_comment(explained),
        "the boundary check must not flag prose explaining the rule"
    );
}

#[test]
fn the_size_gate_reads_whole_files() {
    let path = repo_root().join("src").join("lib.rs");
    let counted = fs::read_to_string(&path).expect("read").lines().count();
    assert!(
        counted > 0 && counted <= MAX_MODULE_LINES,
        "lib.rs should count as a small module, counted {counted}"
    );
}

/// The walk must reach the crate, or every gate above passes vacuously.
#[test]
fn the_walk_finds_the_crate() {
    let files = module_files();
    let names: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    for expected in ["lib.rs", "main.rs", "endpoint.rs"] {
        assert!(
            names.iter().any(|n| n == expected),
            "the walk missed {expected}: {names:?}"
        );
    }
}

/// Keeps `Path` used regardless of how the helpers evolve, and documents that
/// the gates operate on real paths rather than strings.
#[test]
fn repo_root_is_a_directory() {
    let root: &Path = &repo_root();
    assert!(root.is_dir(), "repo root must be a directory");
}
