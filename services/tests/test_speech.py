"""Reading a speech request, and turning samples into bytes on the wire.

The model is not involved in any of this, so none of it needs a graphics card.
What does need saying: the measured French output peaked at 1.005, which clips.
Normalisation is therefore a correctness requirement here rather than polish,
and it is tested as one.
"""

import io
import unittest
import wave

from maestro_speech.speech import DEFAULT_LANGUAGE, PEAK, normalise, read_request, to_wav


class TestReadRequest(unittest.TestCase):
    def test_it_reads_the_openai_shape(self):
        request = read_request(b'{"model":"tts","input":"The tests pass.","voice":"default"}')

        self.assertEqual(request.text, "The tests pass.")
        self.assertEqual(request.voice, "default")
        self.assertEqual(request.response_format, "wav")

    def test_the_language_field_selects_the_voice_language(self):
        """Not part of OpenAI's shape. It is here because the approved design
        says the reply comes back in whichever language was spoken, and
        Chatterbox selects that per call rather than per loaded model."""
        request = read_request(b'{"input":"Les tests passent.","language":"fr"}')

        self.assertEqual(request.language, "fr")

    def test_the_language_defaults_rather_than_failing(self):
        self.assertEqual(read_request(b'{"input":"hi"}').language, DEFAULT_LANGUAGE)

    def test_an_empty_input_is_refused(self):
        """Nothing to say is a caller error, not silence to synthesise."""
        with self.assertRaises(ValueError) as refused:
            read_request(b'{"input":"   "}')

        self.assertIn("input", str(refused.exception))

    def test_a_body_that_is_not_json_is_refused(self):
        with self.assertRaises(ValueError) as refused:
            read_request(b"not json at all")

        self.assertIn("JSON", str(refused.exception))

    def test_an_unsupported_language_is_refused_listing_what_there_is(self):
        with self.assertRaises(ValueError) as refused:
            read_request(b'{"input":"hi","language":"klingon"}')

        self.assertIn("klingon", str(refused.exception))
        self.assertIn("fr", str(refused.exception))

    def test_an_unsupported_response_format_is_refused_rather_than_mislabelled(self):
        """Returning WAV bytes under an mp3 label would be a lie the caller
        cannot detect until something fails to play."""
        with self.assertRaises(ValueError) as refused:
            read_request(b'{"input":"hi","response_format":"mp3"}')

        self.assertIn("mp3", str(refused.exception))


class TestNormalise(unittest.TestCase):
    def test_it_pulls_a_clipping_signal_under_the_ceiling(self):
        """The measured defect: French output peaked at 1.005."""
        clipping = [0.0, 1.005, -1.002, 0.5]

        normalised = normalise(clipping)

        self.assertLessEqual(max(abs(s) for s in normalised), PEAK + 1e-6)

    def test_it_keeps_the_shape_of_the_signal(self):
        normalised = normalise([0.0, 1.005, -0.5025, 0.2010])

        # Every sample scaled by the same factor: the second is still twice
        # the third in magnitude, and the sign is unchanged.
        self.assertAlmostEqual(normalised[1] / -normalised[2], 2.0, places=4)
        self.assertLess(normalised[2], 0.0)

    def test_it_leaves_a_quiet_signal_alone(self):
        """Normalising up would amplify the noise floor of a short reply."""
        quiet = [0.0, 0.2, -0.1]

        self.assertEqual(list(normalise(quiet)), quiet)

    def test_silence_does_not_divide_by_zero(self):
        self.assertEqual(list(normalise([0.0, 0.0])), [0.0, 0.0])


class TestToWav(unittest.TestCase):
    def test_it_writes_a_readable_sixteen_bit_mono_file(self):
        payload = to_wav([0.0, 0.5, -0.5], sample_rate=24000)

        with wave.open(io.BytesIO(payload)) as handle:
            self.assertEqual(handle.getnchannels(), 1)
            self.assertEqual(handle.getsampwidth(), 2)
            self.assertEqual(handle.getframerate(), 24000)
            self.assertEqual(handle.getnframes(), 3)

    def test_full_scale_does_not_wrap_around(self):
        """The bug this catches is a positive peak becoming a loud negative
        click, which is what naive scaling by 32768 does at exactly 1.0."""
        payload = to_wav([1.0, -1.0], sample_rate=24000)

        with wave.open(io.BytesIO(payload)) as handle:
            frames = handle.readframes(2)
        first = int.from_bytes(frames[0:2], "little", signed=True)
        second = int.from_bytes(frames[2:4], "little", signed=True)

        self.assertGreater(first, 32000)
        self.assertLess(second, -32000)


if __name__ == "__main__":
    unittest.main()
