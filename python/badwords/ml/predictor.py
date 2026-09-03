"""ONNX toxicity predictor.

Runs `onnxruntime` directly rather than through `optimum`, which depends on
torch unconditionally - several hundred megabytes for an inference path that
does not need it.

The model scores several axes at once (toxicity, insult, threat and so on),
named by its own config rather than by anything hardcoded here, so a model
trained on a different label set stays usable.
"""

from __future__ import annotations

import json
import threading
import warnings
from pathlib import Path
from typing import TYPE_CHECKING, Any

from ._paths import get_model_dir
from .scores import TOXICITY, Scores

if TYPE_CHECKING:
    from collections.abc import Sequence

#: Longest input in tokens. Anything past this is truncated.
DEFAULT_MAX_LENGTH = 128

#: Probability at or above which text counts as toxic.
#:
#: On 10,000 held-out rows the overall-toxicity axis peaks at 0.25 and is flat
#: between 0.25 and 0.35, so 0.3 sits in the middle of that plateau. The 0.5
#: this used to default to traded about seven points of recall for one point of
#: precision. Each axis has its own best threshold - `make ml-evaluate` prints
#: them - and a moderation policy should set them per axis.
DEFAULT_THRESHOLD = 0.3

#: What the pre-3.1 binary model called its positive class.
TOXIC_LABEL = "toxic"

#: Output width of that model: two mutually exclusive classes.
_BINARY_WIDTH = 2


class ToxicityPredictor:
    """Toxicity scores from the ONNX model.

    The model is loaded on first use, so constructing a predictor is cheap and
    never reaches for the network:

        predictor = ToxicityPredictor()
        predictor.predict("some text")                  # overall, 0.0 - 1.0
        predictor.predict_scores("some text").as_dict() # every axis
        predictor.is_toxic("some text")                 # bool
    """

    __slots__ = (
        "_labels",
        "_lock",
        "_max_length",
        "_model_dir",
        "_multi_label",
        "_session",
        "_threshold",
        "_tokenizer",
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
        self._labels: tuple[str, ...] = ()
        self._multi_label = True
        self._lock = threading.Lock()

    @property
    def threshold(self) -> float:
        """Probability at or above which :meth:`is_toxic` is true."""
        return self._threshold

    @property
    def labels(self) -> tuple[str, ...]:
        """The axes this model scores. Empty until it is loaded."""
        return self._labels

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

            import onnxruntime as ort  # noqa: PLC0415 - keep `import badwords` light
            from transformers import AutoTokenizer  # noqa: PLC0415

            directory = Path(self._model_dir) if self._model_dir is not None else get_model_dir()
            session = ort.InferenceSession(
                str(directory / "model.onnx"),
                providers=["CPUExecutionProvider"],
            )
            # The tokenizer is loaded exactly as training loaded it. transformers
            # suggests fix_mistral_regex=True for this file; training did not
            # pass it, and matching training is what keeps inference honest.
            with warnings.catch_warnings():
                warnings.filterwarnings("ignore", message=".*fix_mistral_regex.*")
                self._tokenizer = AutoTokenizer.from_pretrained(str(directory))
            self._labels, self._multi_label = _label_space(directory, session)
            self._session = session

    def predict(self, text: str) -> float:
        """Overall toxicity of one text, between 0.0 and 1.0."""
        return self.predict_scores(text).toxicity

    def predict_batch(self, texts: Sequence[str]) -> list[float]:
        """Overall toxicity of several texts, in one pass."""
        return [scores.toxicity for scores in self.predict_scores_batch(texts)]

    def predict_scores(self, text: str) -> Scores:
        """Every axis for one text."""
        return self.predict_scores_batch([text])[0]

    def predict_scores_batch(self, texts: Sequence[str]) -> list[Scores]:
        """Every axis for several texts, in one pass.

        Texts are padded to the longest in the batch. The shipped model is
        INT8-quantized and not perfectly invariant to that padding, so a score
        here can differ from :meth:`predict_scores` on the same text by a few
        hundredths. Score a text alone when it sits near the threshold and the
        exact value matters.
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
        probabilities = _sigmoid(logits, np) if self._multi_label else _softmax(logits, np)
        return [Scores(self._labels, tuple(float(value) for value in row)) for row in probabilities]

    def is_toxic(self, text: str, threshold: float | None = None) -> bool:
        """Whether overall toxicity reaches the threshold."""
        return self.predict(text) >= (self._threshold if threshold is None else threshold)


def _sigmoid(logits: Any, np: Any) -> Any:  # noqa: ANN401
    """Independent probability per axis."""
    return 1.0 / (1.0 + np.exp(-logits))


def _softmax(logits: Any, np: Any) -> Any:  # noqa: ANN401
    """Row-wise softmax, for a head whose classes are mutually exclusive."""
    shifted = logits - logits.max(axis=-1, keepdims=True)
    exponentiated = np.exp(shifted)
    return exponentiated / exponentiated.sum(axis=-1, keepdims=True)


def _label_space(model_dir: Path, session: Any) -> tuple[tuple[str, ...], bool]:  # noqa: ANN401
    """Return the axis names and whether they are independent of each other.

    Names come from the model's config. Models exported before 3.1 record no
    id2label and have two mutually exclusive outputs, which is the old
    clean/toxic head.
    """
    config: dict[str, Any] = {}
    try:
        config = json.loads((model_dir / "config.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        config = {}

    id2label = config.get("id2label") or {}
    labels = tuple(
        label for _, label in sorted(((int(k), v) for k, v in id2label.items()), key=lambda p: p[0])
    )
    problem_type = config.get("problem_type")

    if not labels:
        width = session.get_outputs()[0].shape[-1]
        width = width if isinstance(width, int) else _BINARY_WIDTH
        labels = (
            ("clean", TOXIC_LABEL)
            if width == _BINARY_WIDTH
            else tuple(f"label_{index}" for index in range(width))
        )

    if problem_type == "multi_label_classification":
        return labels, True
    if problem_type == "single_label_classification":
        return labels, False
    return labels, TOXICITY in labels or len(labels) != _BINARY_WIDTH
