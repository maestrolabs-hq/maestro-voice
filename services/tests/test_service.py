"""The readiness contract the router polls, exercised over a real socket.

src/launch/probe.rs sends `GET /health` and reads only the status line: 200
means ready, anything else means still loading. The router polls that until the
entry's startup budget runs out. A service that answered 200 before its model
was loaded would have the router route a transcription at a process that cannot
do one, so the transition is what these tests are about.

No graphics card is involved: the loader is a callable the tests supply.
"""

import http.client
import threading
import unittest

from maestro_speech.arguments import Settings
from maestro_speech.service import Service


def settings_on(port: int = 0) -> Settings:
    return Settings(model="/somewhere/m.bin", alias="test", host="127.0.0.1", port=port)


class Harness:
    """A service on an ephemeral port, with the load under the test's control."""

    def __init__(self, load, routes=None):
        self.service = Service(settings_on(), load=load, routes=routes or {})
        self.port = self.service.bind()
        self.thread = threading.Thread(target=self.service.serve, daemon=True)
        self.thread.start()

    def request(self, method: str, path: str, body: bytes | None = None, headers=None):
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=5)
        connection.request(method, path, body=body, headers=headers or {})
        response = connection.getresponse()
        payload = response.read()
        connection.close()
        return response.status, payload

    def close(self):
        self.service.close()


class TestHealth(unittest.TestCase):
    def test_it_answers_not_ready_until_the_model_has_loaded(self):
        loaded = threading.Event()
        harness = Harness(load=lambda: loaded.wait(timeout=5) or "backend")
        self.addCleanup(harness.close)

        status, _ = harness.request("GET", "/health")
        self.assertEqual(
            status,
            503,
            "a service that says 200 before its model is loaded gets traffic "
            "routed at a process that cannot answer it",
        )

        loaded.set()
        harness.service.wait_until_ready(timeout=5)

        status, _ = harness.request("GET", "/health")
        self.assertEqual(status, 200)

    def test_a_load_that_fails_stops_the_service_rather_than_lingering(self):
        """The router tells 'exited while loading' apart from 'never became
        ready', and the first is the honest report for a model that cannot
        load at all. Lingering would burn the whole startup budget instead."""

        def explode():
            raise RuntimeError("no such model")

        harness = Harness(load=explode)
        self.addCleanup(harness.close)

        self.assertTrue(
            harness.service.wait_until_stopped(timeout=5),
            "a failed load must stop the service",
        )

    def test_an_unknown_path_is_four_oh_four_even_once_ready(self):
        harness = Harness(load=lambda: "backend")
        self.addCleanup(harness.close)
        harness.service.wait_until_ready(timeout=5)

        status, _ = harness.request("GET", "/v1/nothing-here")

        self.assertEqual(status, 404)


class TestRoutes(unittest.TestCase):
    def test_a_route_receives_the_backend_and_the_body(self):
        seen = {}

        def handler(backend, body, _headers):
            seen["backend"] = backend
            seen["body"] = body
            return 200, "application/json", b'{"ok":true}'

        harness = Harness(load=lambda: "the-backend", routes={"/v1/echo": handler})
        self.addCleanup(harness.close)
        harness.service.wait_until_ready(timeout=5)

        status, payload = harness.request(
            "POST", "/v1/echo", body=b"hello", headers={"Content-Length": "5"}
        )

        self.assertEqual(status, 200)
        self.assertEqual(payload, b'{"ok":true}')
        self.assertEqual(seen["backend"], "the-backend")
        self.assertEqual(seen["body"], b"hello")

    def test_a_route_is_refused_until_the_model_is_ready(self):
        """Same reason as /health: the model is what answers, so there is
        nothing to answer with yet."""
        blocked = threading.Event()
        harness = Harness(
            load=lambda: blocked.wait(timeout=5) or "backend",
            routes={"/v1/echo": lambda *_: (200, "text/plain", b"never")},
        )
        self.addCleanup(harness.close)

        status, _ = harness.request("POST", "/v1/echo", body=b"x", headers={"Content-Length": "1"})

        self.assertEqual(status, 503)
        blocked.set()

    def test_a_handler_that_raises_becomes_a_five_hundred_not_a_dropped_socket(self):
        def explode(*_):
            raise ValueError("bad input")

        harness = Harness(load=lambda: "backend", routes={"/v1/echo": explode})
        self.addCleanup(harness.close)
        harness.service.wait_until_ready(timeout=5)

        status, payload = harness.request(
            "POST", "/v1/echo", body=b"x", headers={"Content-Length": "1"}
        )

        self.assertEqual(status, 500)
        self.assertIn(b"bad input", payload)


if __name__ == "__main__":
    unittest.main()
