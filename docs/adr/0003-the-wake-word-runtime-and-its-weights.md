<!-- Capture a decision, its reasons, costs and unresolved questions. -->
# 0003. The wake-word runtime is linked in; its weights are not

## Status

accepted

## Context

The wake word is three small neural networks run in series on every
eighty-millisecond chunk of audio, for as long as the daemon is running. Two
questions had to be answered before any of that could ship, and they turned
out to be independent: where the inference engine comes from, and where the
weights come from.

**The engine.** Running ONNX models from Rust means the `ort` crate, which can
obtain ONNX Runtime three ways: download a prebuilt one at build time, locate
one at run time, or link one the operator built. A spike proved the port
correct but linked against a `libonnxruntime.so` taken out of a Python virtual
environment, which is not a thing a shipped binary may depend on. The
build-time download had appeared to be unavailable, because `ort-sys`'s build
script uses `ureq` and failed to compile without a transport enabled. That
turned out to be a missing feature rather than a broken build script.

**The weights.** The four models the spike used came from the `openwakeword`
0.4.0 wheel. The package's own metadata says:

> All of the code in openWakeWord is licensed under the Apache 2.0 license.
> All of the included pre-trained models are licensed under the Creative
> Commons Attribution-NonCommercial-ShareAlike 4.0 International license due to
> the inclusion of datasets with unknown or restrictive licensing as part of
> the training data.

So the wheel's Apache-2.0 classifier covers its code only. Every `.onnx` in
`resources/models` is CC BY-NC-SA 4.0: NonCommercial, and ShareAlike. The
package separately describes the speech-embedding stage as originating in a
Google TFHub module under Apache-2.0, but it also says that model was
re-implemented, and the blanket statement above governs the files as
distributed. The cautious reading is the only defensible one.

This repository declares itself MIT, in `REUSE.toml`, for every path. Three
facts therefore could not all hold at once: the models are committed, the
licence metadata is true, and the repository stays MIT.

## Decision

**The runtime is linked in.** `ort` is taken with `download-binaries` and
`tls-native`. `ort-sys` fetches a prebuilt ONNX Runtime 1.28.0 from
`cdn.pyke.io` during the build and links it **statically**. The resulting
binary has no `libonnxruntime` dependency, needs no system package, and runs
with `ORT_DYLIB_PATH` and `LD_LIBRARY_PATH` unset. There is no Debian package
for ONNX Runtime and nothing in `ldconfig` on the target host, so a run-time
lookup would have meant every operator building the library by hand.

The download is verified against a SHA256 pinned in `build/download/dist.tsv`
inside the crate source, so pinning `ort` in `Cargo.lock` pins the artifact.

**The weights are fetched, not committed.** `just fetch-wake-models` obtains
them and verifies each against a checksum manifest committed here. The
manifest is what preserves the guarantee that committing the files would have
given: any run can say exactly which weights it executed against.

Absence is an ordinary first-run state, so it gets an ordinary message.
`ModelSet::at` names every missing file, the directory searched, and the
command that fixes it, before any session is opened.

Silero's voice-activity model ships inside the same `openwakeword` wheel and
must not be taken from there. The identical file is published by the
`silero-vad` package under MIT, by its own metadata: "MIT License, Copyright
(c) 2020-present Silero Team". The fetch step takes it from upstream. This is
recorded because the two copies are interchangeable as bytes and not as
obligations, which is exactly the sort of thing a later simplification
destroys.

**No Git LFS.** The fetched weights total about 5 MB, they are immutable
upstream artifacts rather than files that churn, and LFS would add a clone-time
dependency whose absence yields broken fixtures instead of a clear error. This
is reconsidered if committed fixture weight approaches roughly 50 MB, or if
fixtures start changing on a cadence rather than being pinned.

## Consequences

The binary is self-contained with respect to inference, and deployment stops
being a question. Nobody has to install ONNX Runtime, and the failure mode
where a Python environment is deleted and the daemon stops working is gone.

**The build now requires network access.** This is a real constraint and
someone will hit it: an air-gapped or offline build fails, and it fails in
`ort-sys`'s build script rather than anywhere obviously ours. The escape hatch
is to build ONNX Runtime from source and point `ort` at it, which is supported
and which also removes both build-only licence exceptions in `deny.toml`.

Two build-only crates fall outside the licence allowlist and are excepted by
name rather than by widening it: `webpki-root-certs` under CDLA-Permissive-2.0,
which is Mozilla's root certificate data and is pulled in by `ureq` for every
TLS backend, and `hmac-sha256` under ISC, which is what verifies the downloaded
archive. Neither is linked into the shipped binary.

**The wake stage's licence constrains the whole project.** While the engine is
openWakeWord, `maestro-voice` cannot be distributed commercially with a working
wake word out of the box, because the weights that make it work are
NonCommercial and ShareAlike. Nothing in this repository is encumbered -- that
is the point of not committing them -- but a user who runs the fetch step
accepts those terms. Stating this once, here, is cheaper than rediscovering it
the day somebody asks.

That raises the value of the boundary in `src/wake.rs`. Inference is behind an
interface that names no engine, and exactly one module imports `ort`, so
replacing the engine is a change in one place rather than a rewrite. No
permissively licensed pretrained alternative was found within openWakeWord: its
own README invites requests for "pre-trained models with more permissive
licensing", which is good evidence that none exists there today.

A test that needs fetched fixtures is a test that needs a setup step, and a
machine without one fails loudly rather than skipping. That is deliberate: a
test that quietly does not run is the failure this repository's first decision
already refused.

## Unresolved

Whether the phrase fires reliably in the owner's own voice and accent is not
answered by anything automated here. Every fixture is synthetic speech, and no
amount of it settles a question about a particular person in a particular room.
That is a manual check and it stays recorded as one.

What reopens the runtime decision: a `cdn.pyke.io` brownout making builds
unreliable, an ONNX Runtime appearing in the distribution's package manager, or
a target this does not publish a prebuilt binary for.

What reopens the weights decision: openWakeWord publishing permissively
licensed pretrained models, or a wake engine of comparable quality under terms
that permit redistribution. Either would let the fixtures be committed and the
setup step disappear.
