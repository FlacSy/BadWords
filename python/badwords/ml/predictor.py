"""ONNX toxicity predictor."""

from __future__ import annotations

from pathlib import Path

from ._paths import get_model_dir


class ToxicityPredictor:
    """ML-based toxicity probability (0.0–1.0)."""

    def __init__(self, model_dir: Path | str | None = None) -> None:
        """Load model. Uses get_model_dir() if model_dir is None."""
        if model_dir is None:
            model_dir = get_model_dir()
        self._model_dir = Path(model_dir)
        self._model = None
        self._tokenizer = None

    def _load(self) -> None:
        if self._model is not None:
            return
        from optimum.onnxruntime import ORTModelForSequenceClassification
        from transformers import AutoTokenizer

        path = str(self._model_dir)
        self._model = ORTModelForSequenceClassification.from_pretrained(path)
        self._tokenizer = AutoTokenizer.from_pretrained(path, fix_mistral_regex=True)

    def predict(self, text: str) -> float:
        """Return toxic probability 0.0–1.0."""
        self._load()
        inputs = self._tokenizer(
            text,
            return_tensors="pt",
            truncation=True,
            max_length=128,
        )
        outputs = self._model(**inputs)
        probs = outputs.logits.softmax(dim=-1)
        return probs[0, 1].item()
