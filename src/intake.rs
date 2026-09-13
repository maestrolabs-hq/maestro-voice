//! The socket a finished turn arrives on.
//!
//! The voice agent runs in its own terminal pane, and the daemon needs the text
//! it just wrote. Reading that text off the terminal is not an option: Herdr's
//! own documentation warns that pi draws on the alternate screen, so rows that
//! scroll away never reach its scrollback and a read cannot be relied on to
//! recover a whole reply. So the agent's own process tells us, through a pi
//! extension that posts here when a turn settles.
//!
//! This is the trust boundary. Anything that can reach this socket can put
//! words in the speaker, so it is bound to loopback, the peer is checked a
//! second time after accepting, the body is bounded before it is allocated, and
//! a peer that goes quiet mid-request cannot hold the daemon open.

mod request;

use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use request::Parsed;

/// The most text one posted turn may carry.
///
/// The spoken block is written last, so the extension sends the tail of the
/// final assistant message rather than all of it, and this is the size of that
/// tail. A block anywhere near this size would already have failed the far
/// smaller cap in [`crate::speak::MAX_SPOKEN_CHARS`]; the headroom is for the
/// prose that precedes it.
pub const MAX_BODY: usize = 16 * 1024;

/// How long one exchange may take before the peer is treated as gone.
///
/// A connection that is opened and then left silent is the cheapest way to stop
/// a daemon that is waiting on it. The extension posts a small body from the
/// same host, so seconds are generous.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(5);

/// The listener the agent's extension posts to.
#[derive(Debug)]
pub struct Intake {
    listener: TcpListener,
}

/// One finished turn, as the agent left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    message: Option<String>,
}

impl Turn {
    /// The final assistant message, when the turn produced one.
    ///
    /// `None` is a turn that ended without any assistant text at all. It is
    /// posted, and arrives here, for the same reason a reply with no spoken
    /// block does: a silent turn is a thing to count, not a thing to miss.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

/// What one accepted connection turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Received {
    /// A turn to decide what to say about.
    Turn(Turn),
    /// A caller that was answered and ignored.
    Refused(Refusal),
}

/// Why a caller was ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The peer was not on this host.
    NotLoopback,
    /// The declared body was over [`MAX_BODY`].
    TooLarge,
    /// Not a post of the one path this serves.
    UnknownPath,
    /// A request that could not be read as one.
    Malformed,
}

/// Whether a peer may put words in the speaker.
///
/// Binding loopback is what actually keeps the network out; this is the second
/// lock on the same door, and it is a rule about an address so that it can be
/// tested as one.
#[must_use]
pub fn admits(peer: &SocketAddr) -> bool {
    peer.ip().is_loopback()
}

impl Intake {
    /// Listen on loopback. Port `0` takes whatever the system offers.
    ///
    /// # Errors
    ///
    /// When the port cannot be bound.
    pub fn bind(port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        Ok(Self { listener })
    }

    /// Where this is listening, which is how a caller learns the port when it
    /// asked for any.
    ///
    /// # Errors
    ///
    /// When the socket cannot report its own address.
    pub fn address(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Wait for one caller and answer it.
    ///
    /// Only a failure of the listener itself is an error. A caller that is
    /// refused, unreadable or gone is an ordinary outcome: the daemon serves
    /// one agent all day, and a bad request must not end the loop that is
    /// waiting for the next good one.
    ///
    /// # Errors
    ///
    /// When accepting a connection fails.
    pub fn accept(&self) -> std::io::Result<Received> {
        let (stream, peer) = self.listener.accept()?;
        Ok(serve(&stream, &peer))
    }
}

/// Read one request and answer it, whatever it turns out to be.
fn serve(stream: &TcpStream, peer: &SocketAddr) -> Received {
    if !admits(peer) {
        answer(stream, "403 Forbidden");
        return Received::Refused(Refusal::NotLoopback);
    }

    if stream.set_read_timeout(Some(EXCHANGE_TIMEOUT)).is_err() {
        answer(stream, "400 Bad Request");
        return Received::Refused(Refusal::Malformed);
    }

    match request::read(stream, MAX_BODY) {
        Parsed::Turn(message) => {
            answer(stream, "204 No Content");
            Received::Turn(Turn { message })
        }
        Parsed::UnknownPath => {
            answer(stream, "404 Not Found");
            Received::Refused(Refusal::UnknownPath)
        }
        Parsed::TooLarge => {
            answer(stream, "413 Content Too Large");
            Received::Refused(Refusal::TooLarge)
        }
        Parsed::Malformed => {
            answer(stream, "400 Bad Request");
            Received::Refused(Refusal::Malformed)
        }
    }
}

/// Answer with a status and nothing else.
///
/// Best effort on purpose. The caller is an extension inside the agent's own
/// process, and the design's first rule for it is that it never disturbs the
/// agent; a daemon that failed loudly because that extension hung up early
/// would be breaking the same rule from the other end. Every reply here has an
/// empty body, so `Connection: close` delimits it and no `Content-Length` is
/// sent -- which also keeps the 204 legal.
fn answer(stream: &TcpStream, status: &str) {
    let mut stream = stream;
    let _ = stream.write_all(format!("HTTP/1.1 {status}\r\nConnection: close\r\n\r\n").as_bytes());
    let _ = stream.flush();
}
