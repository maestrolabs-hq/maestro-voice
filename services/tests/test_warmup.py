"""Readiness must mean the model can answer, not that its constructor returned.

Found the hard way. Against the real faster-whisper, `WhisperModel(...)` built
happily and the service reported 200, and only the first genuine transcription
failed -- on a cuBLAS library the process could not load. The router had by
then been told the model was ready and would have routed a real utterance at a
process that could not transcribe one.

So loading ends with a real inference. If that inference cannot run, the load
has failed, the service stops, and the router reports a child that exited while
loading -- which is the truth, and is distinguishable from a slow one.
"""

import pathlib
import sys
import unittest

STUBS = pathlib.Path(__file__).resolve().parent / "stubs"
if str(STUBS) not in sys.path:
    sys.path.insert(0, str(STUBS))

from maestro_speech import speech, whisper  # noqa: E402
from maestro_speech.arguments import Settings  # noqa: E402


def settings_for(tmp: pathlib.Path) -> Settings:
    model = tmp / "model.bin"
    model.write_bytes(b"stub")
    return Settings(model=str(model), alias="whisper", host="127.0.0.1", port=1)


class TestWhisperWarmup(unittest.TestCase):
    def setUp(self):
        import faster_whisper

        faster_whisper.WhisperModel.instances.clear()

    def test_loading_ends_with_a_real_transcription(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            model = whisper.load(settings_for(pathlib.Path(tmp)))

        self.assertGreaterEqual(
            model.calls,
            1,
            "a load that never ran an inference cannot know the model works",
        )

    def test_a_warmup_that_fails_makes_the_load_fail(self):
        import faster_whisper
        import tempfile

        faster_whisper.WhisperModel.fail_transcribe = True
        self.addCleanup(setattr, faster_whisper.WhisperModel, "fail_transcribe", False)

        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(RuntimeError):
                whisper.load(settings_for(pathlib.Path(tmp)))


class TestSpeechWarmup(unittest.TestCase):
    def test_loading_ends_with_a_real_generation(self):
        model = speech.load(Settings(model="/somewhere/m", alias="tts", host="h", port=1))

        self.assertGreaterEqual(model.calls, 1)


if __name__ == "__main__":
    unittest.main()
