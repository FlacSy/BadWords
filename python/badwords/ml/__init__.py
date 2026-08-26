"""Machine-learning toxicity detection.

Optional: install with ``pip install 'badwords-py[ml]'``.

    from badwords.ml import ToxicityPredictor

    predictor = ToxicityPredictor()
    predictor.predict("some text")   # 0.0 - 1.0

:class:`~badwords.ml.hybrid.HybridFilter` combines this with the rule-based
filter, calling the model only for text the rules are unsure about.
"""

from __future__ import annotations

from ._paths import ModelDownloadError, ModelNotFoundError, download_model, get_model_dir
from .hybrid import HybridFilter, HybridResult
from .predictor import ToxicityPredictor

__all__ = [
    "HybridFilter",
    "HybridResult",
    "ModelDownloadError",
    "ModelNotFoundError",
    "ToxicityPredictor",
    "download_model",
    "get_model_dir",
]
