#!/usr/bin/env python3
"""Quantize ONNX model to INT8 for smaller size and faster inference."""

import argparse
import logging

# Suppress tokenizer regex warning from optimum's maybe_save_preprocessors
logging.getLogger("transformers.tokenization_utils_base").setLevel(logging.ERROR)
import platform
import shutil
import tempfile
from pathlib import Path

from optimum.onnxruntime import ORTQuantizer
from optimum.onnxruntime.configuration import AutoQuantizationConfig

MODELS_DIR = Path(__file__).parent / "models"


def get_qconfig() -> AutoQuantizationConfig:
    """Select quantization config for current CPU."""
    machine = platform.machine().lower()
    if "aarch64" in machine or "arm" in machine:
        return AutoQuantizationConfig.arm64(is_static=False, per_channel=False)
    if "x86" in machine or "amd64" in machine:
        return AutoQuantizationConfig.avx512(is_static=False, per_channel=False)
    return AutoQuantizationConfig.avx2(is_static=False, per_channel=False)


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantize ONNX model to INT8")
    parser.add_argument(
        "--model-dir",
        type=Path,
        default=MODELS_DIR,
        help="Path to model directory",
    )
    args = parser.parse_args()

    model_dir = args.model_dir.resolve()
    if not (model_dir / "model.onnx").exists():
        raise FileNotFoundError(f"model.onnx not found in {model_dir}")

    orig_size = (model_dir / "model.onnx").stat().st_size
    print(f"Loading model from {model_dir} ({orig_size / 1e6:.0f} MB)...")

    quantizer = ORTQuantizer.from_pretrained(str(model_dir))
    qconfig = get_qconfig()
    print(f"Quantizing (dynamic INT8)...")

    with tempfile.TemporaryDirectory() as tmp:
        quantizer.quantize(save_dir=tmp, quantization_config=qconfig)
        quant_path = Path(tmp) / "model_quantized.onnx"
        if not quant_path.exists():
            quant_path = Path(tmp) / "model.onnx"
        if quant_path.exists():
            target = model_dir / "model.onnx"
            target.unlink()
            shutil.copy(quant_path, target)
            new_size = target.stat().st_size
            print(f"Done: {orig_size / 1e6:.1f} MB -> {new_size / 1e6:.1f} MB ({100 * new_size / orig_size:.0f}%)")


if __name__ == "__main__":
    main()
