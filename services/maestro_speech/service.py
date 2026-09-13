"""The HTTP service both shims are, minus the model that answers.

The readiness contract is the part worth stating. src/launch/probe.rs sends
`GET /health` and reads only the status line: 200 means ready, anything else
means still loading, and the router polls that until the entry's startup budget
runs out. So the model loads on a background thread and nothing answers 200
until it has, because a service that said otherwise would have a transcription
routed at a process that cannot do one.

A load that fails stops the service instead of lingering. The router tells
"the server exited while loading" apart from "not ready within its startup
budget", and the first is the honest report for a model that cannot load at
all -- lingering would spend the whole budget before saying anything.

Threaded on purpose: transcription takes seconds and a single-threaded server
would leave a health probe queued behind it.
"""

import os
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# Where to write what the router cannot show you. Both of a child's streams are
# sent to nowhere by src/launch/server.rs -- a pipe nobody drains would block
# the child once it filled -- so without this a failure is invisible.
LOG_VARIABLE = "MAESTRO_SPEECH_LOG"

# Bodies larger than this are refused rather than allocated. Thirty seconds of
# 16 kHz mono audio is under a megabyte; this is generous by two orders of
# magnitude and still bounded.
MAX_BODY_BYTES = 64 * 1024 * 1024


def log(message: str) -> None:
    """Records one line, if somewhere to record it was configured."""
    destination = os.environ.get(LOG_VARIABLE)
    if not destination:
        return
    try:
        with open(destination, "a", encoding="utf-8") as handle:
            handle.write(f"{message}\n")
    except OSError:
        # A log that cannot be written must not take the service down with it.
        pass


class Service:
    """One speech service: a model loading behind a bound socket."""

    def __init__(self, settings, load, routes):
        self.settings = settings
        self._load = load
        self._routes = routes
        self._backend = None
        self._ready = threading.Event()
        self._stopped = threading.Event()
        self._serving = threading.Event()
        self._server = None

    def bind(self) -> int:
        """Takes the port before anything is loaded, and reports which one.

        Bound first so the socket exists while the model loads: the router
        starts polling immediately, and a refused connection during a load
        looks the same to it as a child that died.
        """
        service = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def do_GET(self):  # noqa: N802 - the name http.server dispatches on
                if self.path.split("?")[0] == "/health":
                    service._health(self)
                else:
                    service._say(self, 404, "text/plain", b"not found\n")

            def do_POST(self):  # noqa: N802 - the name http.server dispatches on
                service._route(self)

            def log_message(self, fmt, *args):
                log(f"{self.address_string()} {fmt % args}")

        self._server = ThreadingHTTPServer((self.settings.host, self.settings.port), Handler)
        return self._server.server_address[1]

    def serve(self) -> None:
        """Loads the model on a background thread and answers until stopped.

        The serving thread owns the socket for its whole life: closing it here
        rather than in `close` is what stops a stopped service from leaving a
        listening port behind, which on an always-on daemon accumulates.
        """
        self._serving.set()
        threading.Thread(target=self._load_backend, daemon=True).start()
        try:
            self._server.serve_forever(poll_interval=0.1)
        finally:
            self._server.server_close()
            self._stopped.set()

    def _load_backend(self) -> None:
        try:
            self._backend = self._load()
        except Exception as failure:  # noqa: BLE001 - any failure is fatal here
            log(f"load failed: {type(failure).__name__}: {failure}")
            print(f"load failed: {failure}", file=sys.stderr)
            self.close()
            return
        self._ready.set()
        log(f"ready: {self.settings.alias}")

    def wait_until_ready(self, timeout: float) -> bool:
        return self._ready.wait(timeout=timeout)

    def wait_until_stopped(self, timeout: float) -> bool:
        return self._stopped.wait(timeout=timeout)

    def close(self) -> None:
        """Asks the service to stop, and waits for it to have stopped.

        `shutdown` must not be called from the serving thread, which is why it
        runs on its own; and it only returns once `serve_forever` has, so a
        service that was never serving is not asked to stop.
        """
        if self._server is None or not self._serving.is_set():
            self._stopped.set()
            return
        threading.Thread(target=self._server.shutdown, daemon=True).start()
        self._stopped.wait(timeout=10)

    def _health(self, handler) -> None:
        if self._ready.is_set():
            self._say(handler, 200, "application/json", b'{"status":"ok"}')
        else:
            self._say(handler, 503, "application/json", b'{"status":"loading"}')

    def _route(self, handler) -> None:
        route = self._routes.get(handler.path.split("?")[0])
        if route is None:
            self._say(handler, 404, "text/plain", b"not found\n")
            return
        if not self._ready.is_set():
            self._say(handler, 503, "application/json", b'{"status":"loading"}')
            return

        body = self._body(handler)
        if body is None:
            return
        try:
            status, content_type, payload = route(self._backend, body, handler.headers)
        except Exception as failure:  # noqa: BLE001 - reported, never dropped
            log(f"{handler.path} failed: {type(failure).__name__}: {failure}")
            self._say(handler, 500, "text/plain", f"{failure}\n".encode())
            return
        self._say(handler, status, content_type, payload)

    def _body(self, handler) -> bytes | None:
        """The request body, or None when it was refused and already answered."""
        declared = handler.headers.get("Content-Length")
        if declared is None:
            self._say(handler, 411, "text/plain", b"a Content-Length is required\n")
            return None
        if not declared.isdigit():
            self._say(handler, 400, "text/plain", b"a Content-Length must be a byte count\n")
            return None
        length = int(declared)
        if length > MAX_BODY_BYTES:
            self._say(handler, 413, "text/plain", b"that body is larger than this service reads\n")
            return None
        return handler.rfile.read(length)

    @staticmethod
    def _say(handler, status: int, content_type: str, payload: bytes) -> None:
        handler.send_response(status)
        handler.send_header("Content-Type", content_type)
        handler.send_header("Content-Length", str(len(payload)))
        handler.end_headers()
        handler.wfile.write(payload)


def run(settings, load, routes) -> None:
    """Binds, reports the port, and serves until the model fails or the process ends."""
    service = Service(settings, load=load, routes=routes)
    port = service.bind()
    log(f"bound {settings.host}:{port} for '{settings.alias}' from {settings.model}")
    service.serve()
