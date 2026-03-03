#!/usr/bin/env python3
"""Test trained toxicity model inference.

Categories: obvious, edge cases, evasion, context, false positives, multilingual.
"""

from pathlib import Path

from optimum.onnxruntime import ORTModelForSequenceClassification
from transformers import AutoTokenizer

MODELS_DIR = Path(__file__).parent / "models"

# Expected: 1=toxic, 0=clean
TEST_CASES = [
    ("Поздравзяю теперь ты не тупой", 1),
]


def predict(model, tokenizer, text: str) -> float:
    """Return toxic probability (0-1)."""
    inputs = tokenizer(text, return_tensors="pt", truncation=True, max_length=128)
    outputs = model(**inputs)
    probs = outputs.logits.softmax(dim=-1)
    return probs[0, 1].item()


def main() -> None:
    print("Loading model...")
    model = ORTModelForSequenceClassification.from_pretrained(str(MODELS_DIR))
    tokenizer = AutoTokenizer.from_pretrained(str(MODELS_DIR), fix_mistral_regex=True)

    print("\n" + "=" * 70)
    print("Toxicity scores (1.0 = toxic, 0.5 threshold)")
    print("=" * 70)

    correct = 0
    for text, expected in TEST_CASES:
        prob = predict(model, tokenizer, text)
        pred = 1 if prob >= 0.5 else 0
        ok = "✓" if pred == expected else "✗"
        if pred == expected:
            correct += 1
        label = "TOXIC" if pred == 1 else "clean"
        exp_str = "toxic" if expected == 1 else "clean"
        print(f"  {prob:.3f} [{label:5}] {ok} (exp: {exp_str})  {text!r}")

    print("=" * 70)
    print(
        f"Accuracy: {correct}/{len(TEST_CASES)} ({100 * correct / len(TEST_CASES):.0f}%)"
    )
    print("Note: evasion (leetspeak, spacing), indirect RU insults often missed.")
    print("=" * 70)


if __name__ == "__main__":
    main()
