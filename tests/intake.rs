//! The socket the voice agent's extension posts a finished turn to.
//!
//! Everything here speaks real HTTP over a real socket, because the thing being
//! checked is the boundary itself: what it accepts, what it refuses, and what
//! it tells a caller that must never be left hanging.

use maestro_voice::intake::{Intake, MAX_BODY, Received, Refusal, admits};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;

/// Post a body to the intake and return the whole reply.
///
/// The client half runs on this thread and the listener on the caller's, so a
/// refusal that never answered would hang the test rather than pass it.
fn post(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain; \
         charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write");
    let mut reply = String::new();
    stream.read_to_string(&mut reply).expect("read");
    reply
}

/// Serve exactly one request while a client posts `body`, and report both
/// halves.
fn exchange(path: &'static str, body: String) -> (Received, String) {
    let intake = Intake::bind(0).expect("bind");
    let port = intake.address().expect("address").port();
    let client = thread::spawn(move || post(port, path, &body));
    let received = intake.accept().expect("accept");
    (received, client.join().expect("client thread"))
}

#[test]
fn a_finished_turn_arrives_with_the_text_the_agent_wrote() {
    let (received, reply) = exchange("/turn", "<speak>Tests pass.</speak>".to_owned());

    let Received::Turn(turn) = received else {
        panic!("expected a turn, got {received:?}");
    };
    assert_eq!(turn.message(), Some("<speak>Tests pass.</speak>"));
    assert!(
        reply.starts_with("HTTP/1.1 204"),
        "the extension must never be left waiting: {reply:?}"
    );
}

#[test]
fn a_turn_that_produced_no_assistant_message_still_arrives() {
    // The silent turn is the one worth counting, so it is posted too.
    let (received, reply) = exchange("/turn", String::new());

    let Received::Turn(turn) = received else {
        panic!("expected a turn, got {received:?}");
    };
    assert_eq!(turn.message(), None);
    assert!(reply.starts_with("HTTP/1.1 204"));
}

#[test]
fn a_body_over_the_bound_is_refused_without_being_read_into_memory() {
    let (received, reply) = exchange("/turn", "x".repeat(MAX_BODY + 1));

    assert_eq!(received, Received::Refused(Refusal::TooLarge));
    assert!(
        reply.starts_with("HTTP/1.1 413"),
        "an oversized post must be told so: {reply:?}"
    );
}

#[test]
fn a_body_at_the_bound_is_still_accepted() {
    let (received, _) = exchange("/turn", "x".repeat(MAX_BODY));

    let Received::Turn(turn) = received else {
        panic!("the bound is inclusive, got {received:?}");
    };
    assert_eq!(turn.message().map(str::len), Some(MAX_BODY));
}

#[test]
fn an_unknown_path_is_refused() {
    let (received, reply) = exchange("/speak-now", "anything".to_owned());

    assert_eq!(received, Received::Refused(Refusal::UnknownPath));
    assert!(reply.starts_with("HTTP/1.1 404"));
}

#[test]
fn a_request_that_is_not_a_post_is_refused() {
    let intake = Intake::bind(0).expect("bind");
    let port = intake.address().expect("address").port();
    let client = thread::spawn(move || {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .write_all(b"GET /turn HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("write");
        let mut reply = String::new();
        stream.read_to_string(&mut reply).expect("read");
        reply
    });
    let received = intake.accept().expect("accept");

    assert_eq!(received, Received::Refused(Refusal::UnknownPath));
    assert!(client.join().expect("client").starts_with("HTTP/1.1 404"));
}

/// Binding loopback is what actually stops a remote caller, and the peer check
/// is the second lock on the same door. It is a rule about an address, so it is
/// tested as one rather than by finding a second network interface.
#[test]
fn only_a_loopback_peer_may_put_words_in_the_speaker() {
    for allowed in ["127.0.0.1:51000", "[::1]:51000"] {
        let peer: SocketAddr = allowed.parse().expect("address");
        assert!(admits(&peer), "{allowed} is loopback and must be admitted");
    }
    for refused in [
        "203.0.113.5:51000",
        "192.168.1.20:51000",
        "[2001:db8::1]:80",
    ] {
        let peer: SocketAddr = refused.parse().expect("address");
        assert!(
            !admits(&peer),
            "{refused} is not loopback and must be refused"
        );
    }
}

#[test]
fn the_listener_is_reachable_only_on_loopback() {
    let intake = Intake::bind(0).expect("bind");
    let bound = intake.address().expect("address");

    assert!(
        bound.ip().is_loopback(),
        "the intake must not be reachable off this host, bound {bound}"
    );
}
