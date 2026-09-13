//! The handful of things that read badly aloud, even inside a block an agent
//! wrote for speech.
//!
//! This is a safety net, not a markdown-to-speech engine. The block is prose by
//! construction, so the only cases handled are the ones that keep slipping into
//! prose written by something that spends its day in a repository: inline code,
//! file paths, line references and links.
//!
//! Every rule is a whole-token rule. Splitting on whitespace and judging each
//! token in isolation is what keeps this predictable -- a rule that could match
//! across a space would eventually eat a sentence.

/// Punctuation that ends a sentence rather than belonging to the token before
/// it. Note the absence of `-` and `/`, which appear inside real tokens.
const TRAILING: [char; 11] = ['.', ',', ';', ':', '!', '?', ')', ']', '}', '"', '\''];

/// One block's text, ready to be spoken.
///
/// Backticks go first and wholesale: an inline code span is said as its
/// contents, and a stray backtick is not worth a parser. Whitespace collapses
/// as a consequence of splitting on it.
pub(super) fn for_speech(block: &str) -> String {
    let plain: String = block.chars().filter(|c| *c != '`').collect();
    plain
        .split_whitespace()
        .map(token)
        .collect::<Vec<_>>()
        .join(" ")
}

/// One whitespace-delimited token, with its sentence punctuation put back.
fn token(word: &str) -> String {
    let core = word.trim_end_matches(TRAILING);
    let tail = &word[core.len()..];

    let said = host_of(core)
        .or_else(|| path_and_line(core))
        .or_else(|| file_name(core))
        .unwrap_or_else(|| core.to_owned());

    format!("{said}{tail}")
}

/// A link, said as the host it points at.
///
/// The path within a site is unreadable aloud and the whole link is on screen,
/// so the useful half is where it goes.
fn host_of(core: &str) -> Option<String> {
    let rest = core
        .strip_prefix("https://")
        .or_else(|| core.strip_prefix("http://"))?;
    let host = rest.split('/').next().unwrap_or(rest);
    (!host.is_empty()).then(|| host.to_owned())
}

/// A path with a line reference, said as the file and the line.
///
/// `src/proxy/relay.rs:109` becomes `relay.rs line 109`, because the line
/// number is the part a listener is being asked to act on.
fn path_and_line(core: &str) -> Option<String> {
    let (path, line) = core.rsplit_once(':')?;
    if line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!("{} line {line}", file_name(path)?))
}

/// A path, said as its file name.
///
/// The directory chain is noise aloud and identifies nothing the file name
/// does not. A token only counts as a path when it has a separator *and* its
/// last segment carries an extension, which is what keeps `and/or` a word and
/// `docs/adr/` a directory nobody needs read out.
fn file_name(core: &str) -> Option<String> {
    if !core.contains('/') {
        return None;
    }
    let last = core.rsplit('/').next()?;
    last.contains('.').then(|| last.to_owned())
}
