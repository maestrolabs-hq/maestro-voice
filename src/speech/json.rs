//! Reading one string out of a JSON reply, and writing one into a request.
//!
//! Not a JSON parser, and it should not become one. The daemon reads three
//! string fields from replies it asked for -- `text`, `language` and the `code`
//! on a refusal -- and writes three into one request body. A dependency for
//! that would be a dependency to audit for behaviour nothing here needs.
//!
//! What it must get right is escaping, in both directions. A transcript is
//! arbitrary speech: it will eventually contain a quotation mark, and a reader
//! that stops at the first one truncates the sentence without complaining.

/// The value of the first string field named `name`, unescaped.
///
/// Returns `None` when the field is absent or is not a string, which the caller
/// treats as "not reported" rather than as an error: a transcriber that
/// declines to name a language is not a failure.
#[must_use]
pub fn field(body: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\"");
    let mut from = 0;
    while let Some(at) = body[from..].find(&key) {
        let after = from + at + key.len();
        let rest = body[after..].trim_start();
        if let Some(value) = rest.strip_prefix(':') {
            let value = value.trim_start();
            if let Some(text) = value.strip_prefix('"') {
                return Some(unescape(text));
            }
            // A field of this name that is not a string: keep looking, since a
            // later one may be the one asked for.
        }
        from = after;
    }
    None
}

/// The contents of a JSON string, up to its unescaped closing quote.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();

    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    // A lone surrogate is left as the replacement character
                    // rather than dropped: something was said there.
                    let point = u32::from_str_radix(&hex, 16).ok();
                    out.push(point.and_then(char::from_u32).unwrap_or('\u{fffd}'));
                }
                Some(other) => out.push(other),
                None => break,
            },
            other => out.push(other),
        }
    }
    out
}

/// `value` as a JSON string, quoted and escaped.
#[must_use]
pub fn quote(value: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Everything below a space must be escaped for the string to be
            // legal JSON at all.
            c if (c as u32) < 0x20 => {
                // Writing into a String cannot fail.
                let _escaped = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::{field, quote};

    #[test]
    fn a_plain_field_reads_back() {
        let body = r#"{"text":"run the tests","language":"en"}"#;
        assert_eq!(field(body, "text").as_deref(), Some("run the tests"));
        assert_eq!(field(body, "language").as_deref(), Some("en"));
        assert_eq!(field(body, "duration"), None);
    }

    #[test]
    fn a_transcript_containing_a_quotation_mark_is_not_truncated() {
        // Someone will eventually say the word "quote". A reader that stops at
        // the first quotation mark loses the rest of the sentence in silence.
        let body = r#"{"text":"he said \"run it\" and left"}"#;
        assert_eq!(
            field(body, "text").as_deref(),
            Some(r#"he said "run it" and left"#)
        );
    }

    #[test]
    fn escapes_come_back_as_the_characters_they_stand_for() {
        let body = r#"{"text":"one\ntwo\ttabbed\\slash \u00e9"}"#;
        assert_eq!(
            field(body, "text").as_deref(),
            Some("one\ntwo\ttabbed\\slash \u{e9}")
        );
    }

    #[test]
    fn a_refusal_code_is_found_inside_its_error_object() {
        let body = r#"{"error":{"message":"no room","code":"insufficient_room"}}"#;
        assert_eq!(
            field(body, "code").as_deref(),
            Some("insufficient_room"),
            "the code is what causes are told apart by"
        );
    }

    #[test]
    fn a_field_that_is_not_a_string_is_not_reported_as_one() {
        let body = r#"{"duration":1.5,"text":"words"}"#;
        assert_eq!(field(body, "duration"), None);
        assert_eq!(field(body, "text").as_deref(), Some("words"));
    }

    #[test]
    fn a_name_appearing_as_a_value_does_not_stand_in_for_the_field() {
        // "text" occurs first as someone's spoken word, not as a key.
        let body = r#"{"language":"text","text":"the real one"}"#;
        assert_eq!(field(body, "text").as_deref(), Some("the real one"));
    }

    #[test]
    fn an_empty_transcript_is_reported_as_empty_rather_than_missing() {
        assert_eq!(field(r#"{"text":""}"#, "text").as_deref(), Some(""));
    }

    #[test]
    fn quoting_survives_a_round_trip_through_reading() {
        for awkward in [
            r#"say "hello" twice"#,
            "a\\backslash",
            "line\nbreak",
            "tab\there",
            "accented \u{e9}\u{e8}",
            "",
        ] {
            let body = format!("{{\"input\":{}}}", quote(awkward));
            assert_eq!(
                field(&body, "input").as_deref(),
                Some(awkward),
                "quoting {awkward:?} must survive being read back"
            );
        }
    }

    #[test]
    fn a_control_character_is_escaped_into_legal_json() {
        assert_eq!(quote("a\u{1}b"), r#""a\u0001b""#);
    }
}
