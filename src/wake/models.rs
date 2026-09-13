//! Locating the weights the wake word runs on, and saying something useful
//! when they are absent.
//!
//! The weights are not in this repository and will not be: openWakeWord
//! licenses its pre-trained models under CC BY-NC-SA 4.0, which this
//! repository cannot redistribute while declaring itself MIT. They are fetched
//! instead, which makes "the models are not here yet" the ordinary first-run
//! state rather than an exceptional one. See
//! `docs/adr/0003-the-wake-word-runtime-and-its-weights.md`.

use std::fmt;
use std::path::{Path, PathBuf};

/// The three files the wake-word pipeline runs, in the order it runs them.
///
/// Silero's voice-activity model ships in the same upstream wheel and is
/// deliberately not here: this stage does not use it, and the copy that
/// belongs in this project comes from a different package under a different
/// licence. The fetch step explains that trap; `docs/adr/0003` records it.
const REQUIRED: [&str; 3] = [
    "melspectrogram.onnx",
    "embedding_model.onnx",
    "hey_jarvis_v0.1.onnx",
];

/// The command that puts the weights where this type looks for them.
const REMEDY: &str = "just fetch-wake-models";

/// A directory holding every file the wake word needs.
///
/// Existence is checked once, here, so that opening a session later fails only
/// for reasons worth reporting differently. The directory is supplied by the
/// caller rather than discovered, because a path that names this machine has
/// no business in this crate.
#[derive(Debug, Clone)]
pub struct ModelSet {
    directory: PathBuf,
}

impl ModelSet {
    /// The model set in `directory`, or what is missing from it.
    ///
    /// # Errors
    ///
    /// Returns [`Missing`] naming every absent file, not merely the first, so
    /// one run tells you everything you have to fetch.
    pub fn at(directory: &Path) -> Result<Self, Missing> {
        let absent: Vec<&'static str> = REQUIRED
            .iter()
            .copied()
            .filter(|name| !directory.join(name).is_file())
            .collect();

        if absent.is_empty() {
            Ok(Self {
                directory: directory.to_path_buf(),
            })
        } else {
            Err(Missing {
                directory: directory.to_path_buf(),
                absent,
            })
        }
    }

    /// The three model files, in the order the pipeline runs them:
    /// melspectrogram, then the shared speech-embedding backbone, then the
    /// wake head -- the only stage that knows which phrase it listens for.
    ///
    /// Returned together rather than one accessor per stage, because three
    /// accessors differing only in an index are three chances for one of them
    /// to point at the wrong file.
    pub(super) fn paths(&self) -> [PathBuf; 3] {
        REQUIRED.map(|name| self.directory.join(name))
    }
}

/// Model files that were looked for and not found.
///
/// Carries all three facts a first run needs: which files, where they were
/// expected, and the command that puts them there. A message with only the
/// first is a message that sends someone reading source code.
#[derive(Debug, Clone)]
pub struct Missing {
    directory: PathBuf,
    absent: Vec<&'static str>,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the wake word needs model weights this repository does not ship: \
             {} {} missing from {}. Run `{REMEDY}` to fetch them.",
            self.absent.join(", "),
            if self.absent.len() == 1 { "is" } else { "are" },
            self.directory.display(),
        )
    }
}

impl std::error::Error for Missing {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every required name must be a bare file name. A name carrying a
    /// separator would reach outside the directory the caller nominated.
    #[test]
    fn every_required_name_stays_inside_the_directory() {
        for name in REQUIRED {
            assert!(
                !name.contains('/') && !name.contains('\\') && !name.contains(".."),
                "{name} must be a bare file name"
            );
        }
    }

    /// The paths must stay in pipeline order and inside the nominated
    /// directory, since the scorer binds them positionally.
    #[test]
    fn the_paths_are_the_required_files_in_pipeline_order() {
        let set = ModelSet {
            directory: PathBuf::from("/somewhere/models"),
        };

        let paths = set.paths();

        for (path, name) in paths.iter().zip(REQUIRED) {
            assert_eq!(path, &PathBuf::from("/somewhere/models").join(name));
        }
    }

    #[test]
    fn one_absent_file_reads_as_singular() {
        let one = Missing {
            directory: PathBuf::from("/somewhere/models"),
            absent: vec!["head.onnx"],
        };
        assert!(one.to_string().contains("head.onnx is missing"));
    }
}
