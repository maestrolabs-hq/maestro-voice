//! Reading one reply off a socket.
//!
//! The three legal ways to delimit a body all appear here, because the router
//! uses two of them and a child behind it may use the third: a declared length,
//! chunked transfer, and "until the connection closes". Getting this wrong does
//! not raise an error, it returns a truncated transcript, which is the failure
//! this crate keeps finding in different disguises.
//!
//! Deliberately not shared with `crate::intake::request`, which parses the
//! server side of a different protocol: that one reads requests from an
//! untrusted peer and must refuse them, this one reads replies from a peer the
//! daemon started talking to. They resemble each other at the level of "both
//! split on a blank line"; the rules they enforce have nothing in common.

use std::io::{self, BufRead, BufReader};
use std::net::TcpStream;

use super::body;

/// What came back.
#[derive(Debug, Clone)]
pub struct Reply {
    /// The status code from the status line.
    pub status: u16,
    /// The body, however it was delimited.
    pub body: Vec<u8>,
}

impl Reply {
    /// Whether the status is a 2xx.
    #[must_use]
    pub const fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// The body as text, for the JSON replies that carry an error.
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Read a whole reply, or say why it could not be read.
pub fn read(stream: TcpStream) -> io::Result<Reply> {
    let mut reader = BufReader::new(stream);

    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the peer closed without answering",
        ));
    }
    let status = status_of(&line)?;

    let mut length: Option<usize> = None;
    let mut chunked = false;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the headers stopped before the blank line that ends them",
            ));
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            continue;
        };
        let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
        match name.as_str() {
            "content-length" => length = value.parse().ok(),
            "transfer-encoding" if value.eq_ignore_ascii_case("chunked") => chunked = true,
            _ => {}
        }
    }

    let body = body::read(&mut reader, chunked, length)?;

    Ok(Reply { status, body })
}

/// `HTTP/1.1 200 OK`: the second word, and none of the rest.
fn status_of(line: &str) -> io::Result<u16> {
    line.split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("not a status line: {:?}", line.trim_end()),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{Reply, read, status_of};
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};

    /// Serve `raw` to one caller and read it back through the real parser.
    fn round_trip(raw: &'static [u8]) -> std::io::Result<Reply> {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let address = listener.local_addr().expect("an address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let _ = stream.write_all(raw);
            let _ = stream.flush();
        });

        let client = TcpStream::connect(address).expect("connect");
        let reply = read(client);
        server.join().expect("the server thread");
        reply
    }

    #[test]
    fn a_declared_length_is_read_exactly() {
        let reply = round_trip(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello")
            .expect("a well-formed reply");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"hello");
        assert!(reply.ok());
    }

    #[test]
    fn a_chunked_body_is_rejoined_in_order() {
        // The shape the router relays a streamed reply in. A parser that took
        // the first chunk for the whole body would return a quarter of a
        // sentence and call it a transcript.
        let reply = round_trip(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
              5\r\nhello\r\n1\r\n \r\n5\r\nworld\r\n0\r\n\r\n",
        )
        .expect("a well-formed chunked reply");

        assert_eq!(reply.body, b"hello world");
    }

    #[test]
    fn a_body_with_no_length_runs_to_the_close() {
        let reply = round_trip(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nuntil the end")
            .expect("a close-delimited reply");
        assert_eq!(reply.body, b"until the end");
    }

    #[test]
    fn a_refusal_keeps_its_status_and_its_body() {
        // The router answers a refusal as JSON carrying a machine-readable
        // code, and both halves have to survive for the caller to tell a
        // refusal from a timeout.
        let reply = round_trip(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 58\r\n\r\n\
              {\"error\":{\"code\":\"insufficient_room\",\"type\":\"server_error\"}}",
        )
        .expect("a refusal is still a reply");

        assert_eq!(reply.status, 503);
        assert!(!reply.ok());
        assert!(reply.text().contains("insufficient_room"));
    }

    #[test]
    fn binary_bodies_survive_byte_for_byte() {
        // Synthesized speech comes back as a WAV, and a parser that touched
        // the bytes would produce audio nobody could play.
        let reply = round_trip(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n\x00\xff\r\n")
            .expect("a binary reply");
        assert_eq!(reply.body, b"\x00\xff\r\n");
    }

    #[test]
    fn a_peer_that_closes_without_answering_is_an_error_rather_than_an_empty_reply() {
        let outcome = round_trip(b"");
        assert!(
            outcome.is_err(),
            "silence must not read as a successful empty reply"
        );
    }

    #[test]
    fn a_status_line_that_is_not_one_is_refused() {
        assert!(status_of("not http at all\r\n").is_err());
        assert_eq!(
            status_of("HTTP/1.1 204 No Content\r\n").expect("parsed"),
            204
        );
    }
}
