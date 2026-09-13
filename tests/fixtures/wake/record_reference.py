"""Record openWakeWord's own per-chunk scores, so the Rust port can be held to them.

Run with an interpreter that has openwakeword 0.4.0 and onnxruntime 1.28.0:

    <venv>/bin/python record_reference.py corpus reference_scores.txt

The onnxruntime version is not incidental. The Rust port links ONNX Runtime
1.28.0 through the ort crate, and recording this file against a different
version widens the gap the port is measured against for no reason. It will
still not be bit-identical, because pyke's prebuilt build and Microsoft's PyPI
wheel are different builds of that version, but the residual difference is then
the floor that tests/wake_equivalence.rs measured its bound against.

Each WAV is fed exactly as a live microphone would feed it: 1280-sample chunks,
in order, one predict() per chunk, with a fresh model per file so that one
file's tail cannot leak into the next one's warm-up.

Scores are written as the hexadecimal bit pattern of the float32 the model
produced, not as decimal text. The comparison this feeds is a tolerance, which
makes the storage format matter more rather than less: decimal round-tripping
between two languages would quietly consume part of a margin that is only about
threefold, and nobody would ever see it happen.
"""

import os
import struct
import sys
import wave

import numpy as np
import openwakeword
from openwakeword.model import Model

CHUNK = 1280
MODELS = os.path.join(os.path.dirname(openwakeword.__file__), "resources", "models")
WAKE_MODEL = os.path.join(MODELS, "hey_jarvis_v0.1.onnx")


def read_wav(path):
    with wave.open(path, "rb") as handle:
        if (
            handle.getnchannels() != 1
            or handle.getsampwidth() != 2
            or handle.getframerate() != 16000
        ):
            raise SystemExit(f"{path}: need 16 kHz mono 16-bit")
        return np.frombuffer(handle.readframes(handle.getnframes()), dtype=np.int16)


def score_series(path):
    model = Model(wakeword_model_paths=[WAKE_MODEL], vad_threshold=0)
    label = os.path.basename(WAKE_MODEL)[:-5]
    samples = read_wav(path)

    scores = []
    for start in range(0, len(samples) - CHUNK + 1, CHUNK):
        prediction = model.predict(samples[start : start + CHUNK])
        scores.append(np.float32(prediction[label]))
    return scores


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    corpus, destination = sys.argv[1], sys.argv[2]

    names = sorted(n for n in os.listdir(corpus) if n.endswith(".wav"))
    lines = [
        "# openWakeWord's own scores for the corpus beside this file, one line",
        "# per fixture: the file name, then one float32 per 1280-sample chunk as",
        "# a hexadecimal bit pattern, in order.",
        "#",
        "# Produced by record_reference.py against openwakeword 0.4.0 and the",
        "# hey_jarvis_v0.1 head. Regenerate whenever the corpus changes; the two",
        "# are a matched pair and neither means anything without the other.",
        "#",
        "# The Rust port is required to reproduce these bit for bit. A tolerance",
        "# was tried and rejected: injected faults moved scores by as little as",
        "# 2.9e-06, which a 0.02 tolerance accepted.",
        "",
    ]

    for name in names:
        scores = score_series(os.path.join(corpus, name))
        packed = " ".join(struct.pack("<f", s).hex() for s in scores)
        peak = max(float(s) for s in scores)
        print(f"{name:26} chunks={len(scores):4} peak={peak:.6f}")
        lines.append(f"{name} {packed}")

    with open(destination, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
