"""The command line the router sends, read the only way a shim safely can.

Deliberately not argparse. The router sends every child the same arguments
whatever the child is -- the model, a context size, reasoning settings, the
whole of the catalog's flags table, then the alias, host and port -- and that
surface moves between llama-server releases. argparse exits with status 2 on an
argument it does not recognise, which would turn an upstream flag addition into
a model that never starts, reported as a start that failed for no stated reason.

So this scans for the four arguments a shim actually needs and skips everything
else without an opinion about it. The one thing it will not do is guess: a
missing port or model is refused, because a shim that picked its own port would
bind one the router is not polling and would be reported as never becoming
ready.
"""

import os
import sys
from dataclasses import dataclass

# Every child the router starts binds loopback. It is not a remote server, and
# a shim that defaulted to anything else would be publishing a model.
LOOPBACK = "127.0.0.1"

# The four this shim reads. Everything else on the line belongs to a stock
# model server and is none of a shim's business.
WANTED = ("--model", "--alias", "--host", "--port")


@dataclass(frozen=True)
class Settings:
    """What a shim needs from a command line it mostly ignores."""

    model: str
    alias: str
    host: str
    port: int

    @property
    def model_directory(self) -> str:
        """The directory holding the model file.

        The router insists on an existing regular file before it will spawn
        anything, whatever the runtime, while both backends here load from the
        directory that file sits in. Bridging the two is the entire reason a
        shim exists rather than a bare service.
        """
        return os.path.dirname(self.model)


def parse(argv: list[str]) -> Settings:
    """The four arguments a shim needs, from a line written for something else.

    Unknown arguments are skipped rather than rejected, including ones that do
    not exist yet. A known flag consumes the token after it; anything else is
    passed over, so a stray value left behind by a flag this does not know is
    simply skipped too.

    Raises SystemExit when a required argument is absent or unreadable, which
    the shim reports as a refusal to start rather than a service that never
    becomes ready.
    """
    found: dict[str, str] = {}
    index = 0
    while index < len(argv):
        token = argv[index]
        name, joined, value = token.partition("=")
        if name in WANTED:
            if joined:
                found[name] = value
                index += 1
                continue
            if index + 1 < len(argv):
                found[name] = argv[index + 1]
                index += 2
                continue
        index += 1

    return _checked(found)


def _checked(found: dict[str, str]) -> Settings:
    """Turns what was found into settings, or refuses and says which is missing."""
    for required in ("--model", "--port"):
        if required not in found:
            raise SystemExit(
                f"{required} is required: the router always sends it, so its "
                f"absence means this was not started by the router"
            )

    port = found["--port"]
    if not port.isdigit():
        raise SystemExit(f"'--port {port}' is not a port number")

    return Settings(
        model=found["--model"],
        # The alias is what a reply calls itself. Absent only when something
        # other than the router started this, so the model name stands in.
        alias=found.get("--alias", "unknown"),
        host=found.get("--host", LOOPBACK),
        port=int(port),
    )


def from_command_line() -> Settings:
    """The settings for this process, from the line it was started with."""
    return parse(sys.argv[1:])
