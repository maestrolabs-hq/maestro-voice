//! Reading `key = value` lines, and the small conversions that follow.
//!
//! Pure text in, values out. Nothing here opens a file or reads the
//! environment, which is what lets every rule below be exercised from a string
//! literal.

use std::collections::BTreeMap;
use std::time::Duration;

/// Settings as written, and anything unreadable about the way they were.
pub struct Settings {
    /// Keys to values, last occurrence winning.
    pub values: BTreeMap<String, String>,
    /// Lines that are not a setting at all.
    pub problems: Vec<String>,
}

/// The environment variable that overrides `key`.
#[must_use]
pub fn variable(prefix: &str, key: &str) -> String {
    format!("{prefix}{}", key.to_ascii_uppercase())
}

/// Read `key = value` lines, ignoring blanks and comments.
#[must_use]
pub fn parse(text: &str) -> Settings {
    let mut values = BTreeMap::new();
    let mut problems = Vec::new();

    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            problems.push(format!(
                "line {}: '{line}' is not 'key = value'",
                number + 1
            ));
            continue;
        };
        // A value may carry anything after the first equals sign, including
        // another one: a device name is not this parser's business.
        values.insert(
            key.trim().to_ascii_lowercase(),
            unquote(value.trim()).to_owned(),
        );
    }
    Settings { values, problems }
}

/// A value with one matching pair of surrounding quotes removed.
///
/// Quotes are optional, and they exist so a device name with trailing spaces
/// can be written down exactly.
fn unquote(value: &str) -> &str {
    let quoted = (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
        || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2);
    if quoted {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Apply a milliseconds setting, or say why it was not applied.
pub fn duration(raw: Option<&str>, key: &str, into: &mut Duration, problems: &mut Vec<String>) {
    let Some(raw) = raw else { return };
    match raw.parse::<u64>() {
        Ok(milliseconds) => *into = Duration::from_millis(milliseconds),
        Err(_) => problems.push(format!(
            "{key} must be a whole number of milliseconds, not '{raw}'"
        )),
    }
}

/// Apply a text setting, ignoring one that was left blank.
pub fn text(raw: Option<&str>, into: &mut String) {
    if let Some(value) = optional(raw) {
        *into = value;
    }
}

/// A text setting, where blank means "not set" rather than "set to nothing".
///
/// The same reading `crate::capture::pulse` gives a device name: an operator
/// who wrote nothing did not choose, and did not choose a device with no name.
#[must_use]
pub fn optional(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{optional, parse, unquote, variable};

    #[test]
    fn ordinary_lines_read_as_settings() {
        let settings = parse("router = 127.0.0.1:8080\nagent = voice\n");
        assert_eq!(settings.values["router"], "127.0.0.1:8080");
        assert_eq!(settings.values["agent"], "voice");
        assert!(settings.problems.is_empty());
    }

    #[test]
    fn blanks_and_comments_are_not_settings_and_not_problems() {
        let settings = parse("\n  \n# a comment\n\nagent = voice\n");
        assert_eq!(settings.values.len(), 1);
        assert!(
            settings.problems.is_empty(),
            "a comment is not a mistake: {:?}",
            settings.problems
        );
    }

    #[test]
    fn a_line_that_is_not_a_setting_is_reported_with_its_number() {
        let settings = parse("agent = voice\nthis is not a setting\n");
        assert_eq!(settings.problems.len(), 1);
        assert!(
            settings.problems[0].contains("line 2"),
            "the report must say where: {:?}",
            settings.problems
        );
    }

    #[test]
    fn a_value_may_contain_the_separator() {
        // A router address carries a colon; a future setting may carry an
        // equals sign, and splitting on the last one would mangle it.
        let settings = parse("router = 127.0.0.1:8080\ncapture_device = alsa_input=1\n");
        assert_eq!(settings.values["capture_device"], "alsa_input=1");
    }

    #[test]
    fn quotes_let_a_device_name_keep_its_spaces() {
        assert_eq!(unquote("\"RDP Source \""), "RDP Source ");
        assert_eq!(unquote("'RDPSink'"), "RDPSink");
        assert_eq!(unquote("RDPSink"), "RDPSink", "quotes are optional");
        assert_eq!(unquote("\"unbalanced"), "\"unbalanced");
    }

    #[test]
    fn keys_are_matched_regardless_of_how_they_were_capitalised() {
        let settings = parse("AGENT = voice\n  Router  =  here  \n");
        assert_eq!(settings.values["agent"], "voice");
        assert_eq!(settings.values["router"], "here");
    }

    #[test]
    fn a_later_line_replaces_an_earlier_one() {
        let settings = parse("agent = first\nagent = second\n");
        assert_eq!(settings.values["agent"], "second");
    }

    #[test]
    fn a_blank_value_means_unset_rather_than_set_to_nothing() {
        assert_eq!(optional(Some("  ")), None);
        assert_eq!(optional(Some("")), None);
        assert_eq!(optional(None), None);
        assert_eq!(optional(Some(" RDPSink ")).as_deref(), Some("RDPSink"));
    }

    #[test]
    fn an_override_is_the_key_shouted_with_the_prefix() {
        assert_eq!(
            variable("MAESTRO_VOICE_", "wake_threshold"),
            "MAESTRO_VOICE_WAKE_THRESHOLD"
        );
    }
}
