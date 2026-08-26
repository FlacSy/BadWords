"""ML-based toxicity detection.

Requires: pip install 'badwords-py[ml]'

The model (XLM-RoBERTa, INT8) scores toxicity in many languages. The first run
downloads it from GitHub Releases: about 206 MB compressed, 266 MB on disk.

Run: python -m examples.python.ml_moderation
"""

from __future__ import annotations

import sys

try:
    from badwords.ml import HybridFilter, ToxicityPredictor
except ImportError:
    print("badwords-py[ml] is required: pip install 'badwords-py[ml]'", file=sys.stderr)
    sys.exit(1)

TOXIC_THRESHOLD = 0.5

MESSAGES = [
    "What a lovely afternoon",
    "you are a fucking idiot",
    "ты полный мудак",
    "Поздравляю, теперь ты не тупой",
    "Could you please review my pull request?",
]


def bar(score: float, width: int = 20) -> str:
    """Render a probability as a small text meter."""
    filled = round(score * width)
    return "#" * filled + "." * (width - filled)


def main() -> None:
    """Score messages with the model, then with the hybrid filter."""
    # Nothing is downloaded until the first prediction; call load() to control
    # exactly when that happens.
    predictor = ToxicityPredictor()

    print("model only")
    for score, message in zip(predictor.predict_batch(MESSAGES), MESSAGES, strict=True):
        verdict = "TOXIC" if score >= TOXIC_THRESHOLD else "ok   "
        print(f"  {verdict} {score:.3f} [{bar(score)}] {message}")

    # The hybrid filter answers from the rules whenever they are certain, and
    # only asks the model about the band in between.
    print("\nhybrid: rules first, model only when they are unsure")
    hybrid = HybridFilter(languages=["en", "ru"])
    for message in MESSAGES:
        result = hybrid.check(message)
        verdict = "BLOCKED" if result.is_profane else "ok     "
        detail = f"rules={result.rule_score:.2f}"
        if result.ml_score is not None:
            detail += f" model={result.ml_score:.3f}"
        print(f"  {verdict} by {result.decided_by:6} {detail:26} {message}")


if __name__ == "__main__":
    main()
