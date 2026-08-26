"""ONNX toxicity predictor.

Runs `onnxruntime` directly rather than through `optimum`, which depends on
torch unconditionally - several hundred megabytes for an inference path that
does not need it.
"""

from __future__ import annotations

import threading
from typing import TYPE_CHECKING, Any

from ._paths import get_model_dir

if TYPE_CHECKING:
    from collections.abc import Sequence
    from pathlib import Path

#: Longest input in tokens. Anything past this is truncated.
DEFAULT_MAX_LENGTH = 128

#: Probability at or above which text counts as toxic.
DEFAULT_THRESHOLD = 0.5

#: Index of the toxic class in the model's output.
TOXIC_LABEL = "toxic"


class ToxicityPredictor:
    """Toxicity probability from the ONNX model.

    The model is loaded on first use, so constructing a predictor is cheap and
    never reaches for the network:

        predictor = ToxicityPredictor()
        predictor.predict("some text")            # 0.0 - 1.0
        predictor.predict_batch(["a", "b"])       # one pass over both
        predictor.is_toxic("some text")           # bool
    """

    __slots__ = (
        "_lock",
        "_max_length",
        "_model_dir",
        "_session",
        "_threshold",
        "_tokenizer",
        "_toxic_index",
    )

    def __init__(
        self,
        model_dir: Path | str | None = None,
        *,
        max_length: int = DEFAULT_MAX_LENGTH,
        threshold: float = DEFAULT_THRESHOLD,
    ) -> None:
        """Prepare a predictor without loading anything yet.

        :param model_dir: Model directory. Resolved on first use when omitted.
        :param max_length: Longest input in tokens.
        :param threshold: Probability at or above which :meth:`is_toxic` is true.
        """
        self._model_dir = model_dir
        self._max_length = max_length
        self._threshold = threshold
        self._session: Any = None
        self._tokenizer: Any = None
        self._toxic_index = 1
        self._lock = threading.Lock()

    @property
    def threshold(self) -> float:
        """Probability at or above which :meth:`is_toxic` is true."""
        return self._threshold

    def load(self) -> None:
        """Load the model, downloading it if necessary.

        Called automatically on first prediction; call it directly to control
        when the download happens.
        """
        if self._session is not None:
            return
        with self._lock:
            if self._session is not None:
                return

            import numpy as np  # noqa: PLC0415 - keep `import badwords` light
            import onnxruntime as ort  # noqa: PLC0415
            from transformers import AutoTokenizer  # noqa: PLC0415

            directory = self._model_dir if self._model_dir is not None else get_model_dir()
            path = str(directory)

            session = ort.InferenceSession(
                str(directory) + "/model.onnx",
                providers=["CPUExecutionProvider"],
            )
            self._tokenizer = AutoTokenizer.from_pretrained(path)
            self._toxic_index = _toxic_index(path, np)
            self._session = session

    def predict(self, text: str) -> float:
        """Toxicity probability of one text, between 0.0 and 1.0."""
        return self.predict_batch([text])[0]

    def predict_batch(self, texts: Sequence[str]) -> list[float]:
        """Toxicity probability of several texts, in one pass.

        Texts are padded to the longest in the batch. The shipped model is
        INT8-quantized and not perfectly invariant to that padding, so a score
        here can differ from :meth:`predict` on the same text by a few
        hundredths. Use :meth:`predict` when a text sits near the threshold and
        the exact value matters.
        """
        if not texts:
            return []
        self.load()

        import numpy as np  # noqa: PLC0415

        encoded = self._tokenizer(
            list(texts),
            return_tensors="np",
            truncation=True,
            padding=True,
            max_length=self._max_length,
        )
        expected = {node.name for node in self._session.get_inputs()}
        inputs = {
            name: value.astype(np.int64) for name, value in encoded.items() if name in expected
        }

        logits = self._session.run(None, inputs)[0]
        probabilities = _softmax(logits, np)
        return [float(row[self._toxic_index]) for row in probabilities]

    def is_toxic(self, text: str, threshold: float | None = None) -> bool:
        """Whether the text scores at or above the threshold."""
        return self.predict(text) >= (self._threshold if threshold is None else threshold)


def _softmax(logits: Any, np: Any) -> Any:  # noqa: ANN401
    """Row-wise softmax."""
    shifted = logits - logits.max(axis=-1, keepdims=True)
    exponentiated = np.exp(shifted)
    return exponentiated / exponentiated.sum(axis=-1, keepdims=True)


def _toxic_index(model_dir: str, np: Any) -> int:  # noqa: ANN401, ARG001
    """Index of the toxic class, from the model config when it records one.

    Models exported before 3.0.0 carry no id2label, in which case index 1 is
    the toxic class - that is how the training pipeline has always labelled it.
    """
    import json  # noqa: PLC0415
    from pathlib import Path  # noqa: PLC0415

    config_path = Path(model_dir) / "config.json"
    try:
        config = json.loads(config_path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return 1

    id2label = config.get("id2label") or {}
    for index, label in id2label.items():
        if str(label).strip().lower() in {TOXIC_LABEL, "label_1", "1"}:
            return int(index)
    return 1
