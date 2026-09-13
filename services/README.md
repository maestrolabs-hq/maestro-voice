<!-- Explain the two speech services, how the router starts them, and how to install them. -->
# Speech services

Two services this daemon talks to but does not run: transcription and speech
synthesis. Both are children of the
[maestro-llamacpp](https://github.com/maestrolabs-hq/maestro-llamacpp) router,
which admits them against the same memory budget as every other model on the
device, evicts them under pressure and unloads them when they go idle.

That is the whole reason they are here rather than started by the daemon. Two
allocators with no shared ledger is how a graphics card gets over-committed.

## How the router starts something it has never heard of

The router resolves a catalog entry's `runtime = "name"` to a binary called
`llama-server-<name>` on the search path. The catalog says *which* runtime; the
machine says *where* it is. A catalog cannot name a path, and that restriction
is deliberate and enforced: configuration that can choose an arbitrary
executable is configuration that can do anything.

So these two are ordinary programs with the expected names:

| Catalog entry | `runtime` | Binary the router looks for |
| --- | --- | --- |
| `whisper` | `whisper` | `llama-server-whisper` |
| `tts` | `tts` | `llama-server-tts` |

**No part of the router changed to serve them.** Three of its rules had to be
met instead, and each is met here rather than relaxed:

1. It refuses to spawn anything whose `path` is not an existing regular file,
   whatever the runtime. Each entry therefore points at a real weight file, and
   the shim derives what its backend actually wants from that file's location.
2. It sends every child the same argument line regardless of what the child is.
   The shims read `--model`, `--alias`, `--host` and `--port`, and ignore
   everything else, including flags that do not exist yet.
3. It polls `GET /health` and reads only the status line: 200 means ready,
   anything else means still loading. Both services answer accordingly.

## Ignoring arguments is a correctness requirement

The router builds one argument line for all children, and the `llama-server`
flag surface moves between releases. A shim that exited on an unrecognised
argument would turn an upstream flag addition into a broken microphone, which
is why the parser is hand-written rather than `argparse`: `argparse` exits with
status 2 on an argument it does not know.

`tests/test_shim.py` runs the real shim with the real router argument line plus
`--quantum-entangle 42`, a flag no `llama-server` has ever had, and requires it
to start, become ready and transcribe.

## Readiness means the model can answer

Not that its constructor returned. Against the real faster-whisper, building a
`WhisperModel` succeeded on a machine whose dynamic loader could not find
cuBLAS, the service reported 200, and only the first genuine transcription
failed with `libcublas.so.12 is not found or cannot be loaded`. The router had
by then been told the model was ready, and would have routed a real utterance
at a process that could not answer one.

Loading therefore ends with a real inference. If that inference cannot run, the
load has failed, the service stops, and the router reports a child that exited
while loading, which is the truth and is distinguishable from a slow one.

The same finding is why `llama-server-whisper` derives `LD_LIBRARY_PATH` from
the interpreter it is about to run: the CUDA libraries CTranslate2 needs are
installed inside the virtual environment, where the loader does not look.

## Install

The shims must be on the search path under their exact names. Symlink them,
which keeps the repository the single copy:

```bash
mkdir -p "${HOME}/.local/bin"
ln -sf "${PWD}/llama-server-whisper" "${HOME}/.local/bin/llama-server-whisper"
ln -sf "${PWD}/llama-server-tts" "${HOME}/.local/bin/llama-server-tts"
```

Run that from this directory, and make sure the target is on `PATH`. Each shim
resolves its own location through the symlink, so the Python package beside it
is found without any further configuration.

Confirm the router agrees, from a checkout of maestro-llamacpp:

```bash
model-router check catalog.toml
```

## Interpreters

Neither service creates or modifies a virtual environment. Each resolves its
interpreter at run time from an environment variable, with a default under the
home directory, so no path to one machine is written into a file.

| Variable | Used by | Default | Needs |
| --- | --- | --- | --- |
| `MAESTRO_WHISPER_PYTHON` | `llama-server-whisper` | a `whisper-local` virtual environment | `faster-whisper`, `ctranslate2` |
| `MAESTRO_TTS_PYTHON` | `llama-server-tts` | a `chatterbox-local` virtual environment | `chatterbox` |

A shim whose interpreter is missing exits 78, which is `EX_CONFIG`: the machine
is not set up, as distinct from a model that failed to load. The router reports
the exit status, so the two are worth telling apart.

Other settings, all optional: `MAESTRO_WHISPER_DEVICE`,
`MAESTRO_WHISPER_COMPUTE_TYPE`, `MAESTRO_WHISPER_BEAM_SIZE`,
`MAESTRO_TTS_DEVICE`, `MAESTRO_TTS_T3_MODEL`, and `MAESTRO_SPEECH_LOG`.

`MAESTRO_SPEECH_LOG` is worth knowing about: the router sends both of a child's
streams to nowhere, because a pipe nobody drains would block the child once it
filled. Without a log file, a failure inside one of these services is invisible.

## What they serve

Both answer `GET /health`. Beyond that:

```text
POST /v1/audio/transcriptions    multipart/form-data, OpenAI shape
     file, and optionally model, language, response_format
     -> {"text": ..., "language": ...}

POST /v1/audio/speech            JSON, OpenAI shape
     model, input, voice, response_format, and language
     -> WAV bytes
```

Language on the transcription side is detected when not given, and returned
with the text, because the reply is expected in whichever language was spoken.
`language` on the synthesis side is not part of the OpenAI shape; it is here
because Chatterbox selects its language per call, which is what lets one loaded
model answer in either.

## Testing

```bash
./run-tests
```

Runs on the system interpreter with no third-party packages and no graphics
card. Both model backends are imported inside their load functions precisely so
that everything around them stays testable, and the two stubs under
`tests/stubs` stand in for them.

What that does **not** cover, and is therefore verified by hand:

- loading either real model, which needs CUDA;
- the quality of a transcription or of synthesised speech;
- behaviour under the router itself, as opposed to the argument line it sends.
