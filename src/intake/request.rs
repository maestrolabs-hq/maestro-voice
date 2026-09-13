//! Reading one request, far enough to answer it and no further.
//!
//! Hand-rolled over the standard library, as the sibling router is: this speaks
//! to exactly one client, a local extension posting one small body, and a
//! framework would be a dependency and an attack surface bought for a single
//! endpoint.
//!
//! Every read here is bounded. A peer that opens a connection and then dribbles
//! header bytes for an hour is the cheapest way to stop a daemon that is
//! waiting on it, so the head has a byte budget, the body has a declared length
//! that is checked before anything is allocated for it, and the caller sets a
//! read timeout over the whole exchange.

use std::io::{self, BufRead, BufReader, Read};
use std::net::TcpStream;

/// The most this will read before the body: enough for ordinary headers and
/// far less than a request that is trying to be expensive.
const MAX_HEAD: u64 = 8 * 1024;

/// The most of an over-large body that is read and thrown away so that the peer
/// can finish writing and hear why it was refused.
///
/// Without draining, a refusal closes the socket while the peer is still
/// sending, so it sees a connection reset instead of the status, and "too
/// large" reaches the extension as "the daemon is broken". Reading *all* of an
/// arbitrarily large body to be polite about rejecting it would make a refusal
/// the cheapest way to occupy the daemon, so this is bounded and a peer that
/// runs past it is reset, which is the right answer to it.
const DRAIN_CEILING: u64 = 64 * 1024;

/// The one path this serves. Anything else is a caller that has the wrong
/// daemon.
const TURN_PATH: &str = "/turn";

/// What one request turned out to be.
pub(super) enum Parsed {
    /// A posted turn, carrying the final assistant message when there was one.
    Turn(Option<String>),
    /// Not a post of the one path served.
    UnknownPath,
    /// A declared body larger than the caller's bound.
    TooLarge,
    /// Unreadable: a broken request line, a bad length, or bytes that are not
    /// text.
    Malformed,
}

/// Read one request from `stream`, allowing a body of at most `max_body` bytes.
pub(super) fn read(stream: &TcpStream, max_body: usize) -> Parsed {
    let mut reader = BufReader::new(stream);
    let Some(length) = head(&mut reader, max_body) else {
        return Parsed::Malformed;
    };

    match length {
        Length::Unknown => Parsed::Malformed,
        Length::Unwanted => Parsed::UnknownPath,
        Length::TooLarge(declared) => {
            // Exactly what was promised, capped: the reader knows how much of
            // the body it has already buffered, so draining here consumes the
            // right number of bytes and stops, rather than reading on until a
            // timeout expires.
            let want = u64::try_from(declared).unwrap_or(DRAIN_CEILING);
            let _ = io::copy(&mut reader.take(want.min(DRAIN_CEILING)), &mut io::sink());
            Parsed::TooLarge
        }
        Length::Body(0) => Parsed::Turn(None),
        Length::Body(count) => body(&mut reader, count),
    }
}

/// What the head said about the body that follows it.
enum Length {
    Body(usize),
    Unwanted,
    TooLarge(usize),
    Unknown,
}

/// Consume the request line and headers, and report what they promise.
///
/// Returns `None` only when the head itself could not be read.
fn head(reader: &mut BufReader<&TcpStream>, max_body: usize) -> Option<Length> {
    let mut head = reader.take(MAX_HEAD);
    let mut line = String::new();
    if head.read_line(&mut line).ok()? == 0 {
        return Some(Length::Unknown);
    }
    if !serves(&line) {
        return Some(Length::Unwanted);
    }

    let mut declared = None;
    loop {
        line.clear();
        if head.read_line(&mut line).ok()? == 0 {
            return Some(Length::Unknown);
        }
        if line.trim_end_matches(['\r', '\n']).is_empty() {
            break;
        }
        if let Some(value) = value_of(&line, "content-length") {
            declared = Some(value.trim().parse::<usize>().ok()?);
        }
    }

    Some(match declared {
        None => Length::Unknown,
        Some(count) if count > max_body => Length::TooLarge(count),
        Some(count) => Length::Body(count),
    })
}

/// Whether the request line is the one post this serves.
fn serves(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    parts.next() == Some("POST") && parts.next() == Some(TURN_PATH)
}

/// A header's value, when the line carries that header.
fn value_of<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let (field, value) = line.split_once(':')?;
    field.trim().eq_ignore_ascii_case(name).then_some(value)
}

/// Read exactly the declared number of bytes and read them as text.
fn body(reader: &mut BufReader<&TcpStream>, count: usize) -> Parsed {
    let mut bytes = vec![0u8; count];
    if reader.read_exact(&mut bytes).is_err() {
        return Parsed::Malformed;
    }
    String::from_utf8(bytes).map_or(Parsed::Malformed, |text| Parsed::Turn(Some(text)))
}
