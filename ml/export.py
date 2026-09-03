#!/usr/bin/env python3
"""Export a trained checkpoint to ONNX.

Separate from training on purpose: a run that trains for hours and then fails
at the export step should not have to train again. `train.py` calls this, and
so can you.

Run: python export.py --checkpoint models/checkpoints/final
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

MODELS_DIR = Path(__file__).parent / "models"
DEFAULT_CHECKPOINT = MODELS_DIR / "checkpoints" / "final"


def export(checkpoint: Path, destination: Path) -> Path:
    """Write an ONNX model plus its tokenizer and config into `destination`."""
    from optimum.onnxruntime import ORTModelForSequenceClassification
    from transformers import AutoTokenizer

    if not checkpoint.exists():
        raise FileNotFoundError(f"no checkpoint at {checkpoint}")

    destination.mkdir(parents=True, exist_ok=True)
    model = ORTModelForSequenceClassification.from_pretrained(str(checkpoint), export=True)
    model.save_pretrained(str(destination))
    AutoTokenizer.from_pretrained(str(checkpoint)).save_pretrained(str(destination))

    verify_config(destination)
    return destination


def verify_config(destination: Path) -> None:
    """Fail loudly if the export lost what inference needs to read.

    Inference names each output from `id2label` and picks sigmoid or softmax
    from `problem_type`. The model published before 3.1 carried neither, so
    every caller had to assume index 1 was "toxic" - not a mistake worth
    repeating silently.
    """
    config_path = destination / "config.json"
    config = json.loads(config_path.read_text(encoding="utf-8"))

    missing = [key for key in ("id2label", "problem_type") if not config.get(key)]
    if missing:
        raise RuntimeError(f"{config_path} is missing {', '.join(missing)}")

    labels = [label for _, label in sorted((int(k), v) for k, v in config["id2label"].items())]
    print(f"  problem_type: {config['problem_type']}")
    print(f"  axes:         {', '.join(labels)}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Export a checkpoint to ONNX")
    parser.add_argument("--checkpoint", type=Path, default=DEFAULT_CHECKPOINT)
    parser.add_argument("--onnx-dir", type=Path, default=MODELS_DIR)
    args = parser.parse_args()

    print(f"Exporting {args.checkpoint} -> {args.onnx_dir}")
    export(args.checkpoint, args.onnx_dir)
    print("\nNext: python evaluate.py, then make ml-quantize")


if __name__ == "__main__":
    main()
