"""A stand-in for faster-whisper, so the shim can be exercised without CUDA.

Only the surface `maestro_speech.whisper` actually uses: a model built from a
directory, and a `transcribe` returning lazy segments plus an info object. The
point of the stub is that everything between the router's command line and the
JSON reply is the real code.
"""

from dataclasses import dataclass


@dataclass
class Segment:
    text: str


@dataclass
class Info:
    language: str
    duration: float


class WhisperModel:
    # Every instance built during a test, so a test can inspect the one the
    # code under test made rather than one it was handed.
    instances: list = []

    # Set by the test that proves a warmup failure fails the load.
    fail_transcribe = False

    def __init__(self, directory, device="cpu", compute_type="int8"):
        self.directory = directory
        self.device = device
        self.compute_type = compute_type
        self.calls = 0
        WhisperModel.instances.append(self)

    def transcribe(self, path, language=None, beam_size=5, vad_filter=False):
        if WhisperModel.fail_transcribe:
            raise RuntimeError("Library libcublas.so.12 is not found or cannot be loaded")
        self.calls += 1
        # The audio really was written to a file the caller can open, which is
        # what the real decoder needs and therefore worth asserting here.
        with open(path, "rb") as handle:
            size = len(handle.read())
        detected = language or "fr"
        segments = iter([Segment(text=" transcribed "), Segment(text=f"{size} bytes")])
        return segments, Info(language=detected, duration=1.25)
