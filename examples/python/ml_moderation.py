#!/usr/bin/env python3
"""ML-based toxicity detection example.

Requires: pip install 'badwords-py[ml]'

The ML model (XLM-RoBERTa) detects toxicity in multiple languages.
First run downloads the model from GitHub Releases (~135MB).
"""

import sys

try:
    from badwords.ml import ToxicityPredictor
except ImportError:
    print("Error: badwords-py[ml] required. Install with:", file=sys.stderr)
    print("  pip install 'badwords-py[ml]'", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    # Создаём предиктор (модель скачается при первом вызове predict)
    predictor = ToxicityPredictor()

    texts = [
        "Hello, how are you today?",
        "Have a nice day!",
        "You are stupid and worthless",
        "Поздравляю, теперь ты не тупой",
        "Иди нахуй",
    ]

    print("=" * 60)
    print("ML Toxicity Detection (0.0 = clean, 1.0 = toxic)")
    print("Threshold: 0.5")
    print("=" * 60)

    for text in texts:
        prob = predictor.predict(text)
        label = "TOXIC" if prob >= 0.5 else "clean"
        bar = "█" * int(prob * 20) + "░" * (20 - int(prob * 20))
        print(f"  {prob:.2f} [{label:5}] {bar}  {text!r}")

    print("=" * 60)


if __name__ == "__main__":
    main()
