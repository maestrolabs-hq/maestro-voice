//! Just enough HTTP to talk to the router.
//!
//! Hand-rolled, and the reason is the same one the sibling router gives for its
//! own: the requests are three shapes against one loopback peer, and a client
//! library would arrive with a runtime, a TLS stack and a dependency tree to
//! audit for no behaviour this needs. What it must get right is small and
//! stated here: bound every read, delimit the body the three legal ways, and
//! never let a peer that stops talking hold the daemon open.
//!
//! It is a client only. The server side of the trust boundary is
//! `crate::intake`, which is where untrusted input actually arrives.

mod reply;
mod wire;

use std::io::{self, Write};
use std::net::TcpStream;
use std::time::Duration;

pub use reply::Reply;

/// How long a request may take in total before the peer is treated as gone.
///
/// A cold model load is the long case and the router holds the connection while
/// it happens, so this is minutes rather than seconds. The entry's own startup
/// budget is what actually bounds a load; this only stops a socket being held
/// for ever by a peer that has stopped writing.
const TIMEOUT: Duration = Duration::from_secs(300);

/// A GET, used to ask for a model without wanting anything back.
///
/// # Errors
///
/// When the peer cannot be reached, or stops answering mid-reply.
pub fn get(address: &str, path: &str) -> io::Result<Reply> {
    let mut stream = connect(address)?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    reply::read(stream)
}

/// A POST carrying JSON.
///
/// # Errors
///
/// When the peer cannot be reached, or stops answering mid-reply.
pub fn post_json(address: &str, path: &str, body: &str) -> io::Result<Reply> {
    send(address, path, "application/json", body.as_bytes())
}

/// A POST carrying one file as `multipart/form-data`, with extra text fields.
///
/// The audio must survive byte for byte, including bytes that happen to look
/// like the boundary, which is why the boundary is checked against the payload
/// rather than assumed unique.
///
/// # Errors
///
/// When the peer cannot be reached, or stops answering mid-reply.
pub fn post_file(
    address: &str,
    path: &str,
    file: (&str, &str, &[u8]),
    fields: &[(&str, &str)],
) -> io::Result<Reply> {
    let boundary = wire::boundary(file.2);
    let body = wire::multipart(&boundary, file, fields);
    let kind = format!("multipart/form-data; boundary={boundary}");
    send(address, path, &kind, &body)
}

fn send(address: &str, path: &str, kind: &str, body: &[u8]) -> io::Result<Reply> {
    let mut stream = connect(address)?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: {kind}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    reply::read(stream)
}

fn connect(address: &str) -> io::Result<TcpStream> {
    let stream = TcpStream::connect(address)?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    Ok(stream)
}
