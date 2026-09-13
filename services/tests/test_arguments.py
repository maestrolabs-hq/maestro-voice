"""The router's flag surface, as a shim must survive it.

Every child the router starts is sent the same arguments regardless of what it
is, and that surface moves between llama-server releases. A shim that died on
an argument it had not heard of would turn an upstream flag addition into a
broken microphone, so tolerance is the property under test here.
"""

import unittest

from maestro_speech.arguments import Settings, parse


# What src/launch/invocation.rs actually sends, in its order: the model, the
# context size, the reasoning settings, then the catalog's flags table, then
# the alias, host and port. Copied from a real catalog entry rather than
# imagined, so this stays a test about the router and not about a guess.
ROUTER_LINE = [
    "--model", "/somewhere/models/whisper/model.bin",
    "--ctx-size", "32768",
    "--reasoning-format", "deepseek",
    "--jinja",
    "--n-gpu-layers", "999",
    "--fit", "on",
    "--fit-target", "1536",
    "--no-mmap",
    "--alias", "whisper",
    "--host", "127.0.0.1",
    "--port", "41337",
]


class TestParse(unittest.TestCase):
    def test_it_reads_the_four_arguments_a_shim_needs(self):
        settings = parse(ROUTER_LINE)

        self.assertEqual(settings.model, "/somewhere/models/whisper/model.bin")
        self.assertEqual(settings.alias, "whisper")
        self.assertEqual(settings.host, "127.0.0.1")
        self.assertEqual(settings.port, 41337)

    def test_it_ignores_an_argument_that_does_not_exist_yet(self):
        """The whole reason this parser is not argparse.

        argparse exits with status 2 on an unrecognised argument. An invented
        flag stands in for whatever llama-server adds next release.
        """
        invented = ROUTER_LINE + ["--quantum-entangle", "42", "--warp-core"]

        settings = parse(invented)

        self.assertEqual(settings.port, 41337)
        self.assertEqual(settings.model, "/somewhere/models/whisper/model.bin")

    def test_it_ignores_a_negated_flag_spelled_from_its_key(self):
        settings = parse(["--no-fa", "--model", "/somewhere/m.bin", "--port", "1"])

        self.assertEqual(settings.model, "/somewhere/m.bin")
        self.assertEqual(settings.port, 1)

    def test_it_accepts_a_flag_joined_to_its_value(self):
        settings = parse(["--model=/somewhere/m.bin", "--port=8080"])

        self.assertEqual(settings.model, "/somewhere/m.bin")
        self.assertEqual(settings.port, 8080)

    def test_the_host_defaults_to_loopback(self):
        """Every child the router starts binds loopback; it is not remote."""
        settings = parse(["--model", "/somewhere/m.bin", "--port", "1"])

        self.assertEqual(settings.host, "127.0.0.1")

    def test_a_missing_port_is_refused_rather_than_defaulted(self):
        """A shim that guessed a port would bind one the router is not polling,
        and the router would report a model that never became ready."""
        with self.assertRaises(SystemExit) as refused:
            parse(["--model", "/somewhere/m.bin"])

        self.assertIn("--port", str(refused.exception))

    def test_a_missing_model_is_refused_rather_than_defaulted(self):
        with self.assertRaises(SystemExit) as refused:
            parse(["--port", "8080"])

        self.assertIn("--model", str(refused.exception))

    def test_a_port_that_is_not_a_number_is_refused(self):
        with self.assertRaises(SystemExit) as refused:
            parse(["--model", "/somewhere/m.bin", "--port", "eleven"])

        self.assertIn("eleven", str(refused.exception))

    def test_the_model_directory_is_the_parent_of_the_model_file(self):
        """The router insists on a file; CTranslate2 wants the directory that
        holds it. The shim is what bridges the two."""
        settings = Settings(
            model="/somewhere/models/snapshots/abc/model.bin",
            alias="whisper",
            host="127.0.0.1",
            port=1,
        )

        self.assertEqual(settings.model_directory, "/somewhere/models/snapshots/abc")


if __name__ == "__main__":
    unittest.main()
