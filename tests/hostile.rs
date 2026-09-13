//! Every parser in this crate, fed input it was not expecting.
//!
//! This exists because `docs/adr/0002` said the fuzzing decision reopens when
//! the first hand-rolled parser lands in `src`, and several now have. The
//! resolution is recorded in `docs/adr/0004`: libFuzzer needs a nightly
//! toolchain and `cargo-fuzz`, neither of which is installed on any machine
//! that has run these gates, and a gate nobody can run is not a gate. What
//! fuzzing actually buys -- "no input makes a parser panic, hang, or allocate
//! without bound" -- is bought here instead, deterministically, on every pull
//! request.
//!
//! Deterministic on purpose. A seeded generator means a failure is reproducible
//! from the seed printed beside it, rather than a red run nobody can repeat.
//! The generator is eight lines of arithmetic rather than a dependency.
//!
//! Every assertion is the same one: it returned. A parser that refuses input is
//! behaving correctly; a parser that panics takes the daemon down, and a parser
//! that reads for ever takes the microphone with it.

use maestro_voice::capture::WavSource;
use maestro_voice::config::Config;
use maestro_voice::intake::{Intake, Received};
use maestro_voice::speech::{audio, json};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::thread;

/// A small linear congruential generator, so a failing case is reproducible.
///
/// The constants are Numerical Recipes'. Nothing here is cryptography: what is
/// wanted is a repeatable spray of bytes, not an unpredictable one.
struct Spray(u32);

impl Spray {
    fn next(&mut self) -> u8 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        // The high bits of an LCG are the ones worth having.
        (self.0 >> 24) as u8
    }

    fn bytes(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.next()).collect()
    }
}

/// Inputs designed to break a parser, plus a repeatable spray of noise.
fn hostile_bytes(seed: u32) -> Vec<(String, Vec<u8>)> {
    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("empty".to_owned(), Vec::new()),
        ("one byte".to_owned(), vec![0]),
        ("all zeroes".to_owned(), vec![0; 1024]),
        ("all ones".to_owned(), vec![0xff; 1024]),
        // A header that promises far more than it delivers: the classic way to
        // make a reader allocate on a number it was told.
        (
            "RIFF claiming four gigabytes".to_owned(),
            b"RIFF\xff\xff\xff\xffWAVEfmt \x10\x00\x00\x00".to_vec(),
        ),
        (
            "WAV truncated mid-header".to_owned(),
            b"RIFF\x24\x00\x00\x00WAVEfmt ".to_vec(),
        ),
        (
            "WAV with a chunk size past the end".to_owned(),
            b"RIFF\x24\x00\x00\x00WAVEfmt \xff\xff\xff\x7f\x01\x00\x01\x00".to_vec(),
        ),
        ("nul bytes in text".to_owned(), b"key\0 = value\0".to_vec()),
        (
            "invalid utf8".to_owned(),
            vec![0xf0, 0x9f, 0x92, 0xa9, 0xff, 0xfe],
        ),
    ];

    let mut spray = Spray(seed);
    for n in 0..24 {
        cases.push((format!("spray {seed}/{n}"), spray.bytes(n * 37 % 900 + 1)));
    }
    cases
}

/// The same inputs as text, for the parsers that take text.
fn hostile_text(seed: u32) -> Vec<(String, String)> {
    let mut cases: Vec<(String, String)> = vec![
        ("empty".to_owned(), String::new()),
        ("only a quote".to_owned(), "\"".to_owned()),
        (
            "unterminated string".to_owned(),
            r#"{"text":"never closed"#.to_owned(),
        ),
        (
            "escape at the very end".to_owned(),
            r#"{"text":"trailing\"#.to_owned(),
        ),
        (
            "truncated unicode escape".to_owned(),
            r#"{"text":"\u12"#.to_owned(),
        ),
        (
            "lone surrogate".to_owned(),
            r#"{"text":"\ud800 alone"}"#.to_owned(),
        ),
        (
            "deeply nested".to_owned(),
            format!("{}{}", "[".repeat(2000), "]".repeat(2000)),
        ),
        (
            "key with no value".to_owned(),
            "wake_threshold =".to_owned(),
        ),
        ("value with no key".to_owned(), "= 0.5".to_owned()),
        ("only separators".to_owned(), "= = = =".to_owned()),
        (
            "a thousand equals signs".to_owned(),
            format!("router {}", "=".repeat(1000)),
        ),
    ];

    for (name, bytes) in hostile_bytes(seed) {
        cases.push((
            format!("{name} as lossy text"),
            String::from_utf8_lossy(&bytes).into_owned(),
        ));
    }
    cases
}

#[test]
fn no_input_makes_the_wav_reader_panic() {
    for (name, bytes) in hostile_bytes(1) {
        // Refusing is correct. Panicking would take the daemon down; hanging
        // would take the microphone with it.
        let _outcome = WavSource::from_bytes(&bytes);
        assert!(!name.is_empty());
    }
}

#[test]
fn no_input_makes_the_synthesis_decoder_panic() {
    for (name, bytes) in hostile_bytes(2) {
        let outcome = audio::decode(&bytes);
        assert!(
            outcome.is_err() || outcome.is_ok(),
            "{name} must return either way"
        );
    }
}

#[test]
fn no_input_makes_the_resampler_panic_or_run_away() {
    let mut spray = Spray(3);
    for n in 0..12 {
        let samples: Vec<i16> = spray
            .bytes(n * 13 + 1)
            .chunks(2)
            .map(|pair| i16::from_le_bytes([pair[0], *pair.get(1).unwrap_or(&0)]))
            .collect();

        // Including the degenerate rates a damaged header hands over. A zero
        // rate would divide by zero; a rate of 1 would turn a one-second reply
        // into a four-hour one. Both must be refused rather than attempted.
        for (from, to) in [(0, 16_000), (16_000, 1), (1, 16_000), (48_000, 16_000)] {
            let out = audio::resample(&samples, from, to);
            assert!(
                out.len() <= samples.len().saturating_mul(13) + 16,
                "resampling {from} to {to} must not run away: {} from {}",
                out.len(),
                samples.len()
            );
        }
    }
}

#[test]
fn no_input_makes_the_json_reader_panic() {
    for (name, text) in hostile_text(4) {
        for field in ["text", "language", "code", "id"] {
            let _one = json::field(&text, field);
            let all = json::fields(&text, field);
            assert!(
                all.len() < 100_000,
                "{name}: reading '{field}' produced {} values",
                all.len()
            );
        }
    }
}

#[test]
fn no_input_makes_the_configuration_reader_panic() {
    for (name, text) in hostile_text(5) {
        let loaded = Config::read(&text, &|_| None);
        // Whatever it made of it, the defaults must still be usable: a
        // configuration that parsed badly must not produce a daemon that
        // cannot endpoint.
        assert!(
            loaded.config.cap > loaded.config.silence,
            "{name} left an unusable rule"
        );
    }
}

#[test]
fn no_request_makes_the_intake_panic_or_hang() {
    // The real trust boundary: anything that reaches this socket can put words
    // in the speaker, so it is the parser that most needs to survive nonsense.
    for (name, bytes) in hostile_bytes(6) {
        let intake = Intake::bind(0).expect("bind");
        let port = intake.address().expect("address").port();

        let client = thread::spawn(move || {
            let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
                return;
            };
            let _written = stream.write_all(&bytes);
            let _flushed = stream.flush();
            // Close the write side so the parser sees the end of the request
            // rather than waiting out its timeout. A client that sends nonsense
            // and hangs up is the realistic case, and it keeps this a fast-tier
            // test rather than a minute of sleeping.
            let _closed = stream.shutdown(Shutdown::Write);
            // Read the answer so a refusal that never replies shows up as a
            // hang here rather than passing silently.
            let mut reply = Vec::new();
            let _read = stream.take(8192).read_to_end(&mut reply);
        });

        let received = intake.accept().expect("accepting must not fail");
        assert!(
            matches!(received, Received::Turn(_) | Received::Refused(_)),
            "{name} must be answered one way or the other"
        );
        client.join().expect("the client thread must not panic");
    }
}

#[test]
fn a_declared_length_far_larger_than_the_body_is_refused_rather_than_waited_on() {
    // The cheapest denial of service there is: promise a gigabyte, send four
    // bytes, and leave the socket open.
    let intake = Intake::bind(0).expect("bind");
    let port = intake.address().expect("address").port();

    let client = thread::spawn(move || {
        let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
            return;
        };
        let head = "POST /turn HTTP/1.1\r\nHost: localhost\r\n\
                    Content-Length: 1073741824\r\n\r\nfour";
        let _written = stream.write_all(head.as_bytes());
        let _closed = stream.shutdown(Shutdown::Write);
        let mut reply = Vec::new();
        let _read = stream.take(8192).read_to_end(&mut reply);
    });

    let received = intake.accept().expect("accept");
    assert!(
        matches!(received, Received::Refused(_)),
        "a body larger than the cap must be refused, got {received:?}"
    );
    client.join().expect("client");
}
