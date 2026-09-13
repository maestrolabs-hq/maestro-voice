"""The OpenAI-shaped transcription request, and the reply to it.

Reading the request is separate from answering it so the parsing is testable
without a graphics card, which is most of what can go wrong here: the audio
arrives as multipart/form-data and must survive byte for byte, including bytes
that happen to look like a boundary.

The parser is the standard library's, not a hand-written one. A multipart
parser is exactly the sort of thing whose edge cases are discovered in
production, and `email` has had those found for it already.

Language is the field to be careful with. The owner speaks French and English
and expects a reply in whichever was used, so an unstated language must reach
the backend as unstated -- detected, never defaulted to one of the two.
"""

from dataclasses import dataclass
from email import policy
from email.parser import BytesParser

# What OpenAI's transcription endpoint calls the audio part.
FILE_PART = "file"


@dataclass(frozen=True)
class Request:
    """One transcription asked for."""

    audio: bytes
    filename: str
    language: str | None
    response_format: str

    @staticmethod
    def reply(response_format: str, text: str, language: str, duration: float) -> dict:
        """The JSON body for a finished transcription.

        The detected language is carried even in the plain `json` shape, which
        OpenAI does not require. It is what lets the agent answer in the
        language it was spoken to, and a client that does not want it can
        ignore a field it did not ask for.
        """
        payload = {"text": text, "language": language}
        if response_format == "verbose_json":
            payload["task"] = "transcribe"
            payload["duration"] = duration
        return payload


def read_request(body: bytes, content_type: str | None) -> Request:
    """Reads a multipart transcription request.

    Raises ValueError when the body is not multipart or carries no audio, which
    the service reports as a refusal rather than attempting a transcription of
    nothing.
    """
    if not content_type or "multipart/form-data" not in content_type:
        raise ValueError(
            "a transcription must be sent as multipart/form-data with a "
            f"'{FILE_PART}' part; this request declared "
            f"'{content_type or 'nothing'}'"
        )

    parts = _parts(body, content_type)
    if FILE_PART not in parts:
        raise ValueError(
            f"no '{FILE_PART}' part in this request: the audio is what there "
            "is to transcribe"
        )

    filename, audio = parts[FILE_PART]
    return Request(
        audio=audio,
        filename=filename or "audio.wav",
        # An empty field is a request to detect, not a language named "".
        language=_text(parts, "language") or None,
        response_format=_text(parts, "response_format") or "json",
    )


def _parts(body: bytes, content_type: str) -> dict[str, tuple[str | None, bytes]]:
    """Every named part, as its filename and its bytes."""
    # The parser wants a message; a request body is a message without its
    # headers, so the declared content type is put back in front of it.
    head = f"Content-Type: {content_type}\r\nMIME-Version: 1.0\r\n\r\n".encode()
    message = BytesParser(policy=policy.default).parsebytes(head + body)

    found: dict[str, tuple[str | None, bytes]] = {}
    for part in message.iter_parts() if message.is_multipart() else []:
        name = part.get_param("name", header="content-disposition")
        if name is None:
            continue
        payload = part.get_payload(decode=True)
        found[str(name)] = (part.get_filename(), payload or b"")
    return found


def _text(parts: dict[str, tuple[str | None, bytes]], name: str) -> str:
    """One part as text, or the empty string when it was not sent."""
    if name not in parts:
        return ""
    return parts[name][1].decode("utf-8", errors="replace").strip()
