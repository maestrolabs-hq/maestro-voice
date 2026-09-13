"""The transcription service, behind the `llama-server-whisper` shim.

Serves `POST /v1/audio/transcriptions` in the OpenAI shape and `GET /health`.
Started by the maestro-llamacpp router like any other model child, which is
what puts it under the same memory budget as everything else on the device.

Language is detected rather than assumed. The owner speaks French and English
and expects an answer in whichever was used, so the detected language is
returned with the text and the caller passes it on.

The model is faster-whisper large-v3, loaded from the directory holding the
model file the router insisted existed. CTranslate2 wants a directory; the
router wants a file; the shim's `model_directory` is the bridge.
"""

import array
import io
import json
import os
import tempfile
import wave

from . import arguments, service, transcription

# Overridable because a machine without a usable card should be able to run
# this slowly rather than not at all.
DEVICE = os.environ.get("MAESTRO_WHISPER_DEVICE", "cuda")
COMPUTE_TYPE = os.environ.get("MAESTRO_WHISPER_COMPUTE_TYPE", "float16")

# Whisper's own decoding defaults are conservative for dictation. A wake-word
# utterance is one or two sentences, so beam search costs little and helps.
BEAM_SIZE = int(os.environ.get("MAESTRO_WHISPER_BEAM_SIZE", "5"))

PATH = "/v1/audio/transcriptions"


def load(settings):
    """Loads the model and proves it can transcribe, which is what ready means.

    Imported here rather than at module scope so the service module stays
    importable, and testable, on a machine with no CUDA stack installed.

    The warmup is not an optimisation. Constructing a `WhisperModel` succeeds
    on a machine whose loader cannot find cuBLAS; only the first real inference
    fails. Without this the service would report itself ready and the router
    would route a genuine utterance at a process that cannot answer one.
    """
    from faster_whisper import WhisperModel

    directory = settings.model_directory
    service.log(f"loading faster-whisper from {directory} on {DEVICE}/{COMPUTE_TYPE}")
    model = WhisperModel(directory, device=DEVICE, compute_type=COMPUTE_TYPE)
    _warm(model)
    return model


def _warm(model) -> None:
    """Runs one real inference, so a broken backend fails the load.

    The segment list is lazy, so it is consumed here: an unconsumed generator
    would do no work at all and prove nothing.
    """
    handle, path = tempfile.mkstemp(suffix=".wav", prefix="maestro-voice-warm-")
    try:
        with os.fdopen(handle, "wb") as audio:
            audio.write(_silence())
        segments, _ = model.transcribe(path, language="en", beam_size=1, vad_filter=False)
        list(segments)
        service.log("warmup inference succeeded")
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass


def _silence() -> bytes:
    """A tenth of a second of 16 kHz mono silence, as a WAV."""
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(16000)
        handle.writeframes(array.array("h", [0] * 1600).tobytes())
    return buffer.getvalue()


def transcribe(model, body: bytes, headers) -> tuple[int, str, bytes]:
    """Answers one transcription request."""
    try:
        request = transcription.read_request(body, headers.get("Content-Type"))
    except ValueError as refused:
        return 400, "application/json", _error(str(refused))

    text, language, duration = _run(model, request)
    payload = transcription.Request.reply(
        request.response_format, text, language, duration
    )
    service.log(f"transcribed {duration:.2f}s of {language}: {text[:80]}")

    if request.response_format == "text":
        return 200, "text/plain; charset=utf-8", text.encode("utf-8")
    return 200, "application/json", json.dumps(payload, ensure_ascii=False).encode("utf-8")


def _run(model, request) -> tuple[str, str, float]:
    """Transcribes the audio, returning the text, its language and its length.

    The audio goes to a real file because CTranslate2 decodes it through
    ffmpeg, which wants something seekable. Deleted whatever happens: an
    always-on daemon that left one file per utterance behind would fill a disk
    at the speed of conversation.
    """
    handle, path = tempfile.mkstemp(suffix=_suffix(request.filename), prefix="maestro-voice-")
    try:
        with os.fdopen(handle, "wb") as audio:
            audio.write(request.audio)
        segments, info = model.transcribe(
            path,
            # None is what makes it detect. Never defaulted to one of the two
            # languages the owner speaks.
            language=request.language,
            beam_size=BEAM_SIZE,
            vad_filter=False,
        )
        # The segment list is lazy: consuming it is what does the work.
        text = "".join(segment.text for segment in segments).strip()
        return text, info.language, info.duration
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass


def _suffix(filename: str) -> str:
    """The extension of the uploaded name, so ffmpeg has a hint to work from."""
    _, extension = os.path.splitext(filename)
    return extension if extension else ".wav"


def _error(reason: str) -> bytes:
    return json.dumps({"error": {"message": reason, "type": "invalid_request_error"}}).encode()


def main() -> None:
    settings = arguments.from_command_line()
    service.run(
        settings,
        load=lambda: load(settings),
        routes={PATH: transcribe},
    )


if __name__ == "__main__":
    main()
