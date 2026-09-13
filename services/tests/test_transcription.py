"""Reading an OpenAI-shaped transcription request.

This is the request the whole design turns on. The router routes its generic
endpoint by reading a model name out of a JSON body, but a transcription is
multipart/form-data and is not JSON -- which is why the daemon addresses the
dedicated per-model endpoint instead, where the model comes from the path and
the body is never parsed. What arrives here is therefore the caller's multipart
body, byte for byte.

No graphics card: parsing a request is separate from answering one.
"""

import unittest

from maestro_speech.transcription import Request, read_request

BOUNDARY = "----maestrovoice7d8f"


def multipart(parts: list[tuple[str, str | None, bytes]]) -> bytes:
    """A multipart body, assembled by hand so the bytes are exactly stated.

    Each part is a name, an optional filename, and its content.
    """
    chunks = []
    for name, filename, content in parts:
        disposition = f'form-data; name="{name}"'
        if filename is not None:
            disposition += f'; filename="{filename}"'
        head = f"--{BOUNDARY}\r\nContent-Disposition: {disposition}\r\n"
        if filename is not None:
            head += "Content-Type: audio/wav\r\n"
        chunks.append(head.encode() + b"\r\n" + content + b"\r\n")
    chunks.append(f"--{BOUNDARY}--\r\n".encode())
    return b"".join(chunks)


CONTENT_TYPE = f"multipart/form-data; boundary={BOUNDARY}"

# Not a real encoder: a RIFF header and a few bytes, enough to prove the bytes
# survive the parse unchanged. What decodes it is the backend's problem.
AUDIO = b"RIFF$\x00\x00\x00WAVEfmt \x00\x01\x02\x03\xff\xfe\r\n--not-a-boundary"


class TestReadRequest(unittest.TestCase):
    def test_it_reads_the_file_part_byte_for_byte(self):
        body = multipart([("file", "utterance.wav", AUDIO)])

        request = read_request(body, CONTENT_TYPE)

        self.assertEqual(
            request.audio,
            AUDIO,
            "audio must survive the parse unchanged, including bytes that "
            "look like a boundary and a trailing CRLF",
        )
        self.assertEqual(request.filename, "utterance.wav")

    def test_the_optional_parts_are_read_when_present(self):
        body = multipart(
            [
                ("file", "utterance.wav", AUDIO),
                ("model", None, b"whisper"),
                ("language", None, b"fr"),
                ("response_format", None, b"verbose_json"),
            ]
        )

        request = read_request(body, CONTENT_TYPE)

        self.assertEqual(request.language, "fr")
        self.assertEqual(request.response_format, "verbose_json")

    def test_language_is_absent_rather_than_guessed_when_not_given(self):
        """The owner speaks French and English and expects a reply in whichever
        was used, so an unstated language must reach the backend as unstated
        and be detected, never defaulted to one of the two."""
        body = multipart([("file", "utterance.wav", AUDIO)])

        request = read_request(body, CONTENT_TYPE)

        self.assertIsNone(request.language)

    def test_an_empty_language_is_the_same_as_none(self):
        """Some clients send the field with nothing in it rather than omitting
        it; that is a request to detect, not a request for a language called
        the empty string."""
        body = multipart([("file", "a.wav", AUDIO), ("language", None, b"")])

        self.assertIsNone(read_request(body, CONTENT_TYPE).language)

    def test_the_default_response_format_is_json(self):
        body = multipart([("file", "a.wav", AUDIO)])

        self.assertEqual(read_request(body, CONTENT_TYPE).response_format, "json")

    def test_a_body_with_no_file_part_is_refused(self):
        body = multipart([("model", None, b"whisper")])

        with self.assertRaises(ValueError) as refused:
            read_request(body, CONTENT_TYPE)

        self.assertIn("file", str(refused.exception))

    def test_a_body_that_is_not_multipart_is_refused_saying_so(self):
        with self.assertRaises(ValueError) as refused:
            read_request(b'{"model":"whisper"}', "application/json")

        self.assertIn("multipart/form-data", str(refused.exception))


class TestReply(unittest.TestCase):
    def test_json_carries_the_text_and_the_detected_language(self):
        """The detected language is what lets the agent answer in the language
        it was spoken to, so it is returned even though OpenAI's plain json
        shape does not require it."""
        payload = Request.reply("json", "Run the tests", "en", 1.75)

        self.assertEqual(payload["text"], "Run the tests")
        self.assertEqual(payload["language"], "en")

    def test_verbose_json_adds_the_duration(self):
        payload = Request.reply("verbose_json", "Roule les tests", "fr", 1.75)

        self.assertEqual(payload["language"], "fr")
        self.assertEqual(payload["duration"], 1.75)
        self.assertEqual(payload["task"], "transcribe")


if __name__ == "__main__":
    unittest.main()
