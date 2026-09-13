//! What can go wrong between audio and a score.
//!
//! This is the part of the wake stage that outlives its engine. The pipeline
//! in `scorer.rs` is openWakeWord-shaped and is expected to be replaced --
//! its licence is a constraint on the whole project, see `docs/adr/0003` --
//! so callers are given an error type that names no engine. The conversion
//! from the ONNX runtime's own error lives in `scorer.rs`, which keeps this
//! module, and everything above it, free of that dependency.

use std::fmt;

use super::models::Missing;

/// Anything that stops a score being produced.
#[derive(Debug)]
pub enum Error {
    /// The weights are not where they were expected. Actionable: the message
    /// names the files, the directory and the command that fetches them.
    Models(Missing),
    /// The runtime refused to load a model or to run one.
    ///
    /// Flattened to a message on purpose. No caller above the wake stage
    /// should have to name the inference engine in order to handle a failure.
    Runtime(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Models(missing) => missing.fmt(f),
            Self::Runtime(message) => write!(f, "the wake-word runtime failed: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<Missing> for Error {
    fn from(missing: Missing) -> Self {
        Self::Models(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// A missing model set must keep its actionable message when it is carried
    /// as the general error, rather than being flattened to a category.
    #[test]
    fn a_missing_model_set_keeps_its_remedy_through_the_conversion() {
        let missing = super::super::ModelSet::at(Path::new("/somewhere/models"))
            .expect_err("a synthetic path holds no models");
        let carried = Error::from(missing);

        let message = carried.to_string();
        assert!(message.contains("just fetch-wake-models"), "{message}");
        assert!(message.contains("/somewhere/models"), "{message}");
    }

    #[test]
    fn a_runtime_failure_says_which_layer_failed() {
        let error = Error::Runtime("no such file".to_owned());
        assert_eq!(
            error.to_string(),
            "the wake-word runtime failed: no such file"
        );
    }
}
