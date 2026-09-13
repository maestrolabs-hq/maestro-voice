"""The one class `maestro_speech.speech` uses, with none of the model."""


class _Audio:
    """Enough of a tensor for the service to flatten into samples."""

    def __init__(self, samples):
        self._samples = samples

    def squeeze(self):
        return self

    def float(self):
        return self

    def cpu(self):
        return self

    def numpy(self):
        return self

    def tolist(self):
        return list(self._samples)


class ChatterboxMultilingualTTS:
    sr = 24000

    def __init__(self):
        self.calls = 0

    @classmethod
    def from_pretrained(cls, device="cpu", t3_model="v3"):
        model = cls()
        model.device = device
        model.t3_model = t3_model
        return model

    def generate(self, text, language_id="en", audio_prompt_path=None):
        self.calls += 1
        # A short ramp that peaks above full scale, so a service that forgot to
        # normalise would be caught by whatever asserts on the output.
        return _Audio([0.0, 0.5, 1.005, -0.75])
