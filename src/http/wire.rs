//! Building a request body.
//!
//! Pure: bytes in, bytes out, no socket. That is what lets the one thing here
//! worth getting wrong -- a boundary that appears inside the audio -- be tested
//! rather than hoped about.

/// A boundary that does not occur in `payload`.
///
/// A multipart boundary is only a delimiter if it appears nowhere in the
/// content. Audio is arbitrary bytes, so a fixed boundary is a bet; it is a
/// bet that would be won almost always and lost silently, producing a truncated
/// transcription of a sentence that was recorded correctly. Cheaper to check.
#[must_use]
pub fn boundary(payload: &[u8]) -> String {
    let mut candidate = "maestro-voice-boundary-8f3a1c".to_owned();
    while contains(payload, candidate.as_bytes()) {
        candidate.push('x');
    }
    candidate
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.len() <= haystack.len() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// One file part and any number of text parts, in the order given.
#[must_use]
pub fn multipart(boundary: &str, file: (&str, &str, &[u8]), fields: &[(&str, &str)]) -> Vec<u8> {
    let (name, filename, content) = file;
    let mut body = Vec::with_capacity(content.len() + 512);

    for (field, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{field}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

#[cfg(test)]
mod tests {
    use super::{boundary, multipart};

    #[test]
    fn the_boundary_avoids_bytes_that_appear_in_the_payload() {
        let plain = boundary(b"ordinary audio");
        // Audio that happens to contain the boundary: the case a fixed string
        // loses silently.
        let awkward = boundary(plain.as_bytes());

        assert_ne!(
            plain, awkward,
            "a boundary occurring in the payload must be extended"
        );
        assert!(
            !plain
                .as_bytes()
                .windows(awkward.len())
                .any(|w| w == awkward.as_bytes()),
            "and the replacement must not occur in it either"
        );
    }

    #[test]
    fn the_file_part_carries_the_payload_unchanged() {
        // Bytes that look like a header, a boundary and a terminator at once.
        let audio: Vec<u8> = (0..=255u8).chain(b"\r\n--".iter().copied()).collect();
        let mark = boundary(&audio);
        let body = multipart(&mark, ("file", "utterance.wav", &audio), &[]);

        let start = body
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("a blank line ends the part headers")
            + 4;
        let end = body.len() - format!("\r\n--{mark}--\r\n").len();

        assert_eq!(
            &body[start..end],
            audio.as_slice(),
            "the audio must survive byte for byte"
        );
    }

    #[test]
    fn text_fields_come_before_the_file_and_name_themselves() {
        let body = multipart(
            "edge",
            ("file", "utterance.wav", b"audio"),
            &[("model", "whisper"), ("response_format", "json")],
        );
        let text = String::from_utf8_lossy(&body);

        let model = text.find("name=\"model\"").expect("the model field");
        let file = text.find("name=\"file\"").expect("the file part");
        assert!(model < file, "fields must precede the file part");
        assert!(text.contains("name=\"response_format\""));
        assert!(
            text.ends_with("--edge--\r\n"),
            "the body must be terminated: {text:?}"
        );
    }

    #[test]
    fn a_body_with_no_fields_is_still_well_formed() {
        let body = multipart("edge", ("file", "u.wav", b"a"), &[]);
        let text = String::from_utf8_lossy(&body);
        assert!(text.starts_with("--edge\r\n"));
        assert!(text.ends_with("--edge--\r\n"));
    }
}
