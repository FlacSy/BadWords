"""Machine-learning toxicity detection.

Optional: install with ``pip install 'badwords-py[ml]'``.

    from badwords.ml import ToxicityPredictor

    predictor = ToxicityPredictor()
    predictor.predict("some text")                    # overall, 0.0 - 1.0
    predictor.predict_scores("some text").as_dict()   # every axis

:class:`~badwords.ml.hybrid.HybridFilter` combines this with the rule-based
filter, calling the model only for text the rules are unsure about.
"""

from __future__ import annotations

from ._paths import ModelDownloadError, ModelNotFoundError, download_model, get_model_dir
from .hybrid import DEFAULT_CERTAIN_AT, HybridFilter, HybridResult
from .predictor import DEFAULT_THRESHOLD, ToxicityPredictor
from .scores import TOXICITY, Scores

__all__ = [
    "DEFAULT_CERTAIN_AT",
    "DEFAULT_THRESHOLD",
    "TOXICITY",
    "HybridFilter",
    "HybridResult",
    "ModelDownloadError",
    "ModelNotFoundError",
    "Scores",
    "ToxicityPredictor",
    "download_model",
    "get_model_dir",
]
