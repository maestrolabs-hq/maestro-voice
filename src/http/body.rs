//! Delimiting the body of a reply.
//!
//! The three legal ways, all of them, because the router uses two and a child
//! behind it may use the third. Getting this wrong does not raise an error: it
//! returns a truncated transcript, which is the failure this crate keeps
//! finding in different disguises.

use std::io::{self, BufRead, BufReader, Read};
use std::net::TcpStream;

/// The most a reply may carry before it is refused rather than allocated.
///
/// Synthesized speech is the large case: thirty seconds of 16 kHz mono is under
/// a megabyte, so this is generous by a wide margin and still bounded.
pub const MAX_BODY: usize = 64 * 1024 * 1024;

/// Read the body, however the headers said it would be delimited.
pub fn read(
    reader: &mut BufReader<TcpStream>,
    chunked: bool,
    length: Option<usize>,
) -> io::Result<Vec<u8>> {
    if chunked {
        return chunks(reader);
    }
    if let Some(declared) = length {
        return exactly(reader, declared);
    }
    // No length and no chunking: the body runs to the close, which is legal and
    // is what `Connection: close` asks for.
    let mut rest = Vec::new();
    reader.take(MAX_BODY as u64).read_to_end(&mut rest)?;
    Ok(rest)
}

fn exactly(reader: &mut BufReader<TcpStream>, declared: usize) -> io::Result<Vec<u8>> {
    if declared > MAX_BODY {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the reply declared {declared} bytes, over the {MAX_BODY} limit"),
        ));
    }
    let mut body = vec![0; declared];
    reader.read_exact(&mut body)?;
    Ok(body)
}

fn chunks(reader: &mut BufReader<TcpStream>) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the chunked body stopped before its final zero-length chunk",
            ));
        }
        // A chunk size may carry extensions after a semicolon.
        let size_text = header.trim_end().split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("not a chunk size: {size_text:?}"),
            )
        })?;

        if size == 0 {
            // The trailer, then the blank line that ends it.
            loop {
                let mut trailer = String::new();
                if reader.read_line(&mut trailer)? == 0 || trailer.trim_end().is_empty() {
                    break;
                }
            }
            return Ok(body);
        }

        if body.len() + size > MAX_BODY {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("the chunked body passed the {MAX_BODY} limit"),
            ));
        }
        let mut chunk = vec![0; size];
        reader.read_exact(&mut chunk)?;
        body.extend_from_slice(&chunk);

        // The CRLF that follows every chunk.
        let mut ending = String::new();
        let _crlf = reader.read_line(&mut ending)?;
    }
}
