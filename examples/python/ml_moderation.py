"""ML-based toxicity detection.

Requires: pip install 'badwords-py[ml]'

The model (XLM-RoBERTa, INT8) scores seven axes at once, in many languages.
The first run downloads it from GitHub Releases: about 270 MB on disk.

Run: python -m examples.python.ml_moderation
"""

from __future__ import annotations

import sys

try:
    from badwords.ml import DEFAULT_THRESHOLD, HybridFilter, ToxicityPredictor
except ImportError:
    print("badwords-py[ml] is required: pip install 'badwords-py[ml]'", file=sys.stderr)
    sys.exit(1)

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
    """Score messages on every axis, then run the hybrid filter."""
    # Nothing is downloaded until the first prediction; call load() to control
    # exactly when that happens.
    predictor = ToxicityPredictor()

    print("model only - every axis, not just a verdict")
    for scores, message in zip(predictor.predict_scores_batch(MESSAGES), MESSAGES, strict=True):
        verdict = "TOXIC" if scores.toxicity >= DEFAULT_THRESHOLD else "ok   "
        print(f"  {verdict} {scores.toxicity:.3f} [{bar(scores.toxicity)}] {message}")
        # Everything the model is at all confident about, strongest first.
        detail = ", ".join(f"{axis} {value:.2f}" for axis, value in scores.above(0.25))
        if detail:
            print(f"        {detail}")

    # The hybrid answers from the rules when they are certain, and asks the
    # model about everything else - including text the rules found nothing in,
    # which is where most toxicity lives.
    print("\nhybrid: a dictionary hit answers alone, everything else goes to the model")
    hybrid = HybridFilter(languages=["en", "ru"])
    for message in MESSAGES:
        result = hybrid.check(message)
        verdict = "BLOCKED" if result.is_profane else "ok     "
        detail = f"rules={result.rule_score:.2f}"
        if result.scores is not None:
            axis, value = result.scores.strongest()
            detail += f" model={result.scores.toxicity:.3f} ({axis} {value:.2f})"
        print(f"  {verdict} by {result.decided_by:6} {detail:44} {message}")


if __name__ == "__main__":
    main()
