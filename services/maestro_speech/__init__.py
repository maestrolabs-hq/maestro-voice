"""Speech services the maestro-llamacpp router supervises as ordinary children.

Two of them, each behind a shim on the search path named `llama-server-<name>`,
because that is how the router lets a catalog point at something other than the
stock model server: the catalog says which runtime, the machine says where it
is. Nothing in the router changes to serve these.

The package is deliberately thin. Argument parsing, readiness and HTTP framing
are shared and testable without a graphics card; the two model backends are not,
and are kept behind a loader the tests replace.
"""
