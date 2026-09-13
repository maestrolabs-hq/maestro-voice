//! What the daemon will say out loud, taken from what the agent wrote.
//!
//! These run against the public surface rather than the internals, because the
//! rule that matters is "given this reply, say this" -- not which pass of the
//! normaliser happened to do it.

use maestro_voice::speak::{Language, Outcome, Refusal, extract};

/// The language tag is the transcription lane's half of the interface; these
/// tests only need a valid one.
fn english() -> Language {
    Language::new("en").expect("en is a valid tag")
}

/// The spoken text for a reply, or a panic naming what came back instead.
fn spoken(message: &str) -> String {
    match extract(message, &english()) {
        Ok(Outcome::Spoken(spoken)) => spoken.text().to_owned(),
        Ok(Outcome::Silent) => panic!("expected a spoken block, got silence"),
        Err(refusal) => panic!("expected a spoken block, got {refusal:?}"),
    }
}

/// A reply of the shape this daemon actually meets: prose, a fenced code
/// block, a table, a file path, and the spoken block last.
const REPLY: &str = "\
I added the handler to `src/main.rs` and the test passes.

```rust
fn main() {
    println!(\"not for the speaker\");
}
```

| lane | status |
| --- | --- |
| shutdown | merged |

See src/proxy/relay.rs:109 for the timeout.

<speak>
I added the shutdown handler and the test passes.
</speak>";

#[test]
fn only_the_block_is_spoken_and_the_code_and_table_are_not() {
    let said = spoken(REPLY);

    assert_eq!(said, "I added the shutdown handler and the test passes.");
    for absent in ["println", "lane", "---", "```"] {
        assert!(
            !said.contains(absent),
            "the reply's {absent:?} must not reach the speaker: {said:?}"
        );
    }
}

#[test]
fn a_turn_with_no_block_is_silence_rather_than_an_error() {
    // The design accepted that the block depends on the agent remembering it.
    // A forgotten block is a thing to count, not a failure to handle.
    let outcome = extract("Done. Tests pass.", &english()).expect("no block is not an error");

    assert_eq!(
        outcome,
        Outcome::Silent,
        "a reply without a block must be countable silence"
    );
}

#[test]
fn an_unterminated_block_is_refused_rather_than_guessed_at() {
    // Truncation mid-block is exactly when guessing would speak half a
    // sentence with confidence.
    let refusal =
        extract("<speak>I added the handler and", &english()).expect_err("must not be accepted");

    assert_eq!(refusal, Refusal::Unterminated);
}

#[test]
fn a_block_holding_nothing_worth_saying_is_refused() {
    let refusal = extract("<speak>   </speak>", &english()).expect_err("must not be accepted");

    assert_eq!(refusal, Refusal::Empty);
}

#[test]
fn the_first_well_formed_block_wins() {
    let said = spoken("<speak>the first</speak> and <speak>the second</speak>");

    assert_eq!(said, "the first");
}

#[test]
fn the_language_the_utterance_was_in_travels_with_the_text() {
    let french = Language::new("fr").expect("fr is a valid tag");
    let Ok(Outcome::Spoken(said)) = extract("<speak>Les tests passent.</speak>", &french) else {
        panic!("expected a spoken block");
    };

    assert_eq!(
        said.language().as_str(),
        "fr",
        "the synthesizer needs the language the owner spoke, not a guess"
    );
}

#[test]
fn a_tag_that_is_not_a_language_is_rejected_at_the_boundary() {
    for bad in ["", "e", "english", "EN", "e1"] {
        assert!(
            Language::new(bad).is_none(),
            "{bad:?} must not be accepted as a language tag"
        );
    }
    assert!(Language::new("en").is_some());
    assert!(Language::new("fra").is_some());
}

/// Each normalisation rule gets its own case, named for what it protects the
/// ear from.
#[test]
fn inline_code_is_spoken_without_its_backticks() {
    assert_eq!(
        spoken("<speak>I set `reasoning_effort` high.</speak>"),
        "I set reasoning_effort high."
    );
}

#[test]
fn a_path_is_spoken_as_its_file_name() {
    // The directory chain is noise aloud and is on screen anyway.
    assert_eq!(
        spoken("<speak>The fix is in src/proxy/relay.rs today.</speak>"),
        "The fix is in relay.rs today."
    );
}

#[test]
fn a_path_with_a_line_number_says_the_line() {
    assert_eq!(
        spoken("<speak>See src/proxy/relay.rs:109 for it.</speak>"),
        "See relay.rs line 109 for it."
    );
}

#[test]
fn a_url_is_spoken_as_its_host() {
    assert_eq!(
        spoken("<speak>Filed at https://github.com/owner/repo/issues/9016 now.</speak>"),
        "Filed at github.com now."
    );
}

#[test]
fn sentence_punctuation_survives_the_path_rules() {
    assert_eq!(
        spoken("<speak>Look at src/main.rs, then src/proxy/relay.rs:12.</speak>"),
        "Look at main.rs, then relay.rs line 12."
    );
}

#[test]
fn a_word_with_a_slash_is_not_mistaken_for_a_path() {
    // "and/or" has no extension on its last segment, so it is left alone.
    assert_eq!(
        spoken("<speak>Use one and/or the other.</speak>"),
        "Use one and/or the other."
    );
}

#[test]
fn whitespace_inside_the_block_is_collapsed() {
    assert_eq!(
        spoken("<speak>  Tests   pass.\n\n  Nothing   broke.  </speak>"),
        "Tests pass. Nothing broke."
    );
}

#[test]
fn an_overlong_block_is_cut_at_a_sentence_and_says_it_was_cut() {
    let long = "All of the tests pass. ".repeat(60);
    let Ok(Outcome::Spoken(said)) = extract(&format!("<speak>{long}</speak>"), &english()) else {
        panic!("expected a spoken block");
    };

    assert!(said.truncated(), "an overlong block must report being cut");
    assert!(
        said.text().len() <= maestro_voice::speak::MAX_SPOKEN_CHARS,
        "cut to the cap, was {}",
        said.text().len()
    );
    assert!(
        said.text().ends_with('.'),
        "the cut lands on a finished sentence: {:?}",
        said.text()
    );
}

#[test]
fn a_block_within_the_cap_is_not_reported_as_cut() {
    let Ok(Outcome::Spoken(said)) = extract("<speak>Short enough.</speak>", &english()) else {
        panic!("expected a spoken block");
    };

    assert!(!said.truncated());
}
