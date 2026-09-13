"""The shim as the router actually runs it: a subprocess, a real socket.

Everything between the router's command line and the JSON reply is the real
code here. Only the model is a stub, because the property under test is the
integration and not the transcription.

The argument line is the one src/launch/invocation.rs builds, plus a flag that
does not exist. That flag is the whole point: AGENTS.md in the router warns the
llama-server flag surface moves between releases, so a shim that died on an
unknown argument would turn an upstream addition into a broken microphone.
"""

import http.client
import os
import pathlib
import socket
import subprocess
import time
import unittest

SERVICES = pathlib.Path(__file__).resolve().parent.parent
SHIM = SERVICES / "llama-server-whisper"
STUBS = pathlib.Path(__file__).resolve().parent / "stubs"

BOUNDARY = "----maestrovoiceshim"
AUDIO = b"RIFF$\x00\x00\x00WAVEfmt \x00\x01\x02\x03"


def free_port() -> int:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def router_line(model: str, port: int) -> list[str]:
    """What the router sends, including a flag that does not exist yet."""
    return [
        "--model", model,
        "--ctx-size", "32768",
        "--jinja",
        "--n-gpu-layers", "999",
        "--fit", "on",
        "--no-mmap",
        # The invented one. Nothing may treat this as an error.
        "--quantum-entangle", "42",
        "--alias", "whisper",
        "--host", "127.0.0.1",
        "--port", str(port),
    ]


class TestShim(unittest.TestCase):
    def setUp(self):
        self.port = free_port()
        model = STUBS / "pretend-model.bin"
        model.write_bytes(b"not a real model, and never opened by the stub")
        self.addCleanup(model.unlink, missing_ok=True)

        environment = dict(os.environ)
        # The system interpreter, with the stub package ahead of anything real.
        environment["MAESTRO_WHISPER_PYTHON"] = "/usr/bin/python3"
        environment["PYTHONPATH"] = str(STUBS)
        environment["MAESTRO_WHISPER_DEVICE"] = "cpu"

        self.process = subprocess.Popen(
            [str(SHIM), *router_line(str(model), self.port)],
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.addCleanup(self._stop)

    def _stop(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=10)
        # Closed explicitly: an unclosed pipe is a ResourceWarning in the test
        # output, and warnings nobody reads are how a real one gets missed.
        for pipe in (self.process.stdout, self.process.stderr):
            if pipe is not None:
                pipe.close()

    def _wait_for_health(self, timeout=30.0):
        """The status of GET /health, once the socket accepts at all."""
        deadline = time.time() + timeout
        last = None
        while time.time() < deadline:
            if self.process.poll() is not None:
                _, err = self.process.communicate()
                self.fail(f"the shim exited {self.process.returncode}: {err.decode()[:600]}")
            try:
                connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=2)
                connection.request("GET", "/health")
                last = connection.getresponse().status
                connection.close()
                if last == 200:
                    return last
            except (ConnectionRefusedError, OSError):
                pass
            time.sleep(0.1)
        return last

    def test_it_starts_and_becomes_ready_despite_an_invented_flag(self):
        self.assertEqual(
            self._wait_for_health(),
            200,
            "the shim must ignore arguments it does not know, including ones "
            "that do not exist yet",
        )

    def test_it_transcribes_a_multipart_request(self):
        self.assertEqual(self._wait_for_health(), 200)

        body = (
            f"--{BOUNDARY}\r\n"
            'Content-Disposition: form-data; name="file"; filename="utterance.wav"\r\n'
            "Content-Type: audio/wav\r\n\r\n"
        ).encode() + AUDIO + f"\r\n--{BOUNDARY}--\r\n".encode()

        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=30)
        connection.request(
            "POST",
            "/v1/audio/transcriptions",
            body=body,
            headers={
                "Content-Type": f"multipart/form-data; boundary={BOUNDARY}",
                "Content-Length": str(len(body)),
            },
        )
        response = connection.getresponse()
        status, payload = response.status, response.read()
        connection.close()

        self.assertEqual(status, 200, payload[:400])
        self.assertIn(b'"language"', payload)
        # The stub reports the byte count it was handed, which proves the audio
        # reached a real file with nothing added or lost on the way.
        self.assertIn(str(len(AUDIO)).encode(), payload)


class TestShimConfiguration(unittest.TestCase):
    def test_a_missing_interpreter_is_refused_with_a_usable_message(self):
        environment = dict(os.environ)
        environment["MAESTRO_WHISPER_PYTHON"] = "/somewhere/that/does/not/exist"

        finished = subprocess.run(
            [str(SHIM), "--model", "/somewhere/m.bin", "--port", "1"],
            env=environment,
            capture_output=True,
            timeout=30,
            check=False,
        )

        self.assertEqual(finished.returncode, 78)
        self.assertIn(b"MAESTRO_WHISPER_PYTHON", finished.stderr)


if __name__ == "__main__":
    unittest.main()
