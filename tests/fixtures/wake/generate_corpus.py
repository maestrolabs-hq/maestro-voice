"""Regenerate the wake-word audio corpus.

Run with an interpreter that has Chatterbox installed, passing the output
directory:

    <chatterbox-venv>/bin/python generate_corpus.py tests/fixtures/wake/corpus

This file is the authoritative record of what each fixture says. The prompts
live here rather than in the Markdown note beside them because one of them is
French, and the repository's English-only gate scans Markdown. The gate is a
heuristic about prose, not about fixture data; keeping the data in the script
that produces it respects both.

Determinism is best-effort. Seeds are fixed, but Chatterbox samples on a GPU
and small numerical differences between driver or library versions can change
the waveform. The checksums recorded for this corpus therefore pin what was
generated, not what any future run would generate; regenerating means
re-recording the reference scores too.
"""

import sys
from pathlib import Path

import torch
import torchaudio

from chatterbox import ChatterboxMultilingualTTS

# 16 kHz mono is the only rate openWakeWord's melspectrogram model was traced
# for, so the corpus is stored at the rate it will be consumed at.
TARGET_RATE = 16_000
SEED = 20260912

# What each fixture is for, and exactly what is said in it.
UTTERANCES = [
    (
        "wake_hey_jarvis_en",
        "en",
        "Hey Jarvis.",
        "the phrase itself: the detector must fire here",
    ),
    (
        "near_miss_en",
        "en",
        "Hey Travis, did you see what happened?",
        "close enough to be interesting, different enough that it must not fire",
    ),
    (
        "negative_speech_en",
        "en",
        "Run the tests again and tell me which one broke.",
        "ordinary English speech: must not fire",
    ),
    (
        "negative_speech_fr",
        "fr",
        "Lance les tests encore une fois et dis-moi lequel a casse.",
        "ordinary French speech: the daemon is bilingual, so French must not fire either",
    ),
]


def room_tone(seconds: float) -> torch.Tensor:
    """Low-level deterministic noise, which is what a quiet room sounds like.

    Digital silence is not a realistic negative: it exercises none of the
    melspectrogram's dynamic range.
    """
    generator = torch.Generator().manual_seed(SEED)
    samples = int(TARGET_RATE * seconds)
    return torch.randn(1, samples, generator=generator) * 0.0015


def to_target_rate(wav: torch.Tensor, source_rate: int) -> torch.Tensor:
    mono = wav if wav.dim() == 2 else wav.unsqueeze(0)
    if mono.shape[0] > 1:
        mono = mono.mean(dim=0, keepdim=True)
    if source_rate != TARGET_RATE:
        mono = torchaudio.functional.resample(mono, source_rate, TARGET_RATE)
    return mono.clamp(-1.0, 1.0)


def write(path: Path, wav: torch.Tensor) -> None:
    torchaudio.save(
        str(path), wav, TARGET_RATE, encoding="PCM_S", bits_per_sample=16
    )
    print(f"{path.name}: {wav.shape[1] / TARGET_RATE:.2f}s")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    destination = Path(sys.argv[1])
    destination.mkdir(parents=True, exist_ok=True)

    device = "cuda" if torch.cuda.is_available() else "cpu"
    model = ChatterboxMultilingualTTS.from_pretrained(device=device)

    for name, language, text, _why in UTTERANCES:
        torch.manual_seed(SEED)
        wav = model.generate(text, language_id=language)
        write(destination / f"{name}.wav", to_target_rate(wav, model.sr))

    write(destination / "room_tone.wav", room_tone(3.0))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
