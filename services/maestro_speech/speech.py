"""The speech service, behind the `llama-server-tts` shim.

Serves `POST /v1/audio/speech` in the OpenAI shape and `GET /health`. Started
by the maestro-llamacpp router like any other model child.

**Why Chatterbox and not Qwen3-TTS.** Qwen3-TTS-12Hz-1.7B-Base, which is the
one on this estate, supports only `generate_voice_clone`: it refuses
`generate_voice_design` and `generate_custom_voice` outright and returns an
empty supported-speaker list, so it cannot say a single word without a
reference recording and its transcript, and no reference exists. Chatterbox
ships `conds.pt`, a built-in voice, and selects the language per call. Measured
2026-09-12 on an RTX 5090: 3075 MiB loaded, English and French both spoken with
no reference at all.

**Normalisation is a fix, not polish.** That same measurement had the French
output peaking at 1.005, which clips. Every reply is scaled under a ceiling
before it becomes bytes, and a quiet reply is left alone rather than amplified.
"""

import array
import io
import json
import math
import os
import wave
from dataclasses import dataclass

from . import arguments, service

DEVICE = os.environ.get("MAESTRO_TTS_DEVICE", "cuda")

# Chatterbox's own multilingual checkpoint. Named because the default differs
# between releases and a voice that silently changed would be a surprise.
T3_MODEL = os.environ.get("MAESTRO_TTS_T3_MODEL", "v3")

DEFAULT_LANGUAGE = "en"

# The two the owner speaks. Chatterbox supports twenty-three; refusing the rest
# keeps a typo from being synthesised as confident nonsense in Malay.
LANGUAGES = ("en", "fr")

# Full scale is 1.0 and the measured peak was 1.005. This leaves a little under
# it rather than exactly at it, because the conversion to integers rounds.
PEAK = 0.95

PATH = "/v1/audio/speech"


@dataclass(frozen=True)
class Request:
    """One thing to say."""

    text: str
    voice: str
    language: str
    response_format: str


def read_request(body: bytes) -> Request:
    """Reads an OpenAI-shaped speech request.

    Raises ValueError for anything unusable, which the service reports as a
    refusal naming what was wrong.
    """
    try:
        asked = json.loads(body)
    except (json.JSONDecodeError, UnicodeDecodeError) as broken:
        raise ValueError(f"this request body is not JSON: {broken}") from broken

    text = str(asked.get("input", "")).strip()
    if not text:
        raise ValueError("'input' is what there is to say, and it was empty")

    language = str(asked.get("language", DEFAULT_LANGUAGE)).lower()
    if language not in LANGUAGES:
        raise ValueError(
            f"'{language}' is not a language this service speaks; it speaks "
            f"{', '.join(LANGUAGES)}"
        )

    response_format = str(asked.get("response_format", "wav")).lower()
    if response_format != "wav":
        raise ValueError(
            f"'{response_format}' is not a format this service returns; it "
            "returns wav. Returning WAV bytes under another name would be a "
            "lie the caller cannot detect until playback fails"
        )

    return Request(
        text=text,
        voice=str(asked.get("voice", "default")),
        language=language,
        response_format=response_format,
    )


def normalise(samples):
    """Scales a signal under the ceiling, leaving a quiet one alone.

    Only ever attenuates. Amplifying a quiet reply would raise its noise floor
    with it, and a short spoken block has proportionally more of one.
    """
    peak = max((abs(float(s)) for s in samples), default=0.0)
    if peak <= PEAK or peak == 0.0:
        return samples
    factor = PEAK / peak
    return [float(s) * factor for s in samples]


def to_wav(samples, sample_rate: int) -> bytes:
    """Sixteen-bit mono PCM in a WAV container.

    Scaled by 32767 rather than 32768 so that a sample at exactly full scale
    stays positive: at 32768 it wraps to the largest negative value, which is
    heard as a click precisely at the loudest moment.
    """
    encoded = array.array(
        "h",
        (int(max(-1.0, min(1.0, float(s))) * 32767) for s in samples),
    )
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(sample_rate)
        handle.writeframes(encoded.tobytes())
    return buffer.getvalue()


def load(_settings):
    """Loads Chatterbox and proves it can speak, which is what ready means.

    Imported here so this module stays importable, and testable, on a machine
    with no CUDA stack. The weights come from the model root the router already
    validated; Chatterbox resolves them itself through the hub cache that root
    contains.

    The warmup exists for the reason the transcription service documents: a
    model that constructs is not necessarily a model that can run, and the
    router must not be told 200 by a process that cannot answer.
    """
    from chatterbox.mtl_tts import ChatterboxMultilingualTTS

    service.log(f"loading chatterbox {T3_MODEL} on {DEVICE}")
    model = ChatterboxMultilingualTTS.from_pretrained(device=DEVICE, t3_model=T3_MODEL)
    model.generate("Ready.", language_id=DEFAULT_LANGUAGE, audio_prompt_path=None)
    service.log("warmup generation succeeded")
    return model


def speak(model, body: bytes, _headers) -> tuple[int, str, bytes]:
    """Answers one speech request."""
    try:
        request = read_request(body)
    except ValueError as refused:
        return 400, "application/json", _error(str(refused))

    audio = model.generate(request.text, language_id=request.language, audio_prompt_path=None)
    samples = _samples(audio)
    payload = to_wav(normalise(samples), sample_rate=int(model.sr))
    service.log(f"spoke {len(samples) / float(model.sr):.2f}s of {request.language}")
    return 200, "audio/wav", payload


def _samples(audio):
    """A flat sequence of floats, whatever shape the model returned."""
    flattened = audio.squeeze().float().cpu().numpy()
    return [float(s) for s in flattened.tolist() if not math.isnan(s)]


def _error(reason: str) -> bytes:
    return json.dumps({"error": {"message": reason, "type": "invalid_request_error"}}).encode()


def main() -> None:
    settings = arguments.from_command_line()
    service.run(
        settings,
        load=lambda: load(settings),
        routes={PATH: speak},
    )


if __name__ == "__main__":
    main()
