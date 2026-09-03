#!/usr/bin/env python3
"""Measure a model on the held-out split, axis by axis.

Reports ROC-AUC (threshold-free) and the best F1 with the threshold that
achieves it, for every axis the test data annotates. The rarer axes have too
few positives for a fixed 0.5 threshold to say anything, which is why the
sweep is here rather than a single number.

Run: make ml-evaluate
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import numpy as np
import pandas as pd
from sklearn.metrics import average_precision_score, roc_auc_score

sys.path.insert(0, str(Path(__file__).resolve().parent))
from labels import DECISION_THRESHOLD, LABELS, TEXT_COLUMN
from prepare_data import OUTPUT_DIR

MODELS_DIR = Path(__file__).parent / "models"


def sweep(truth: np.ndarray, scores: np.ndarray) -> tuple[float, float]:
    """Best F1 over thresholds, and the threshold that reaches it."""
    best = (0.0, DECISION_THRESHOLD)
    positives = float((truth == 1).sum())
    if positives == 0:
        return best
    for threshold in np.arange(0.05, 0.96, 0.05):
        predicted = scores >= threshold
        true_positive = float((predicted & (truth == 1)).sum())
        if true_positive == 0:
            continue
        precision = true_positive / float(predicted.sum())
        recall = true_positive / positives
        f1 = 2 * precision * recall / (precision + recall)
        if f1 > best[0]:
            best = (f1, float(threshold))
    return best


def main() -> None:
    parser = argparse.ArgumentParser(description="Evaluate a model on held-out data")
    parser.add_argument("--model-dir", type=Path, default=None, help="Defaults to ml/models")
    parser.add_argument("--data", type=Path, default=OUTPUT_DIR / "test.csv")
    parser.add_argument("--limit", type=int, default=None, help="Score only the first N rows")
    parser.add_argument("--batch-size", type=int, default=32)
    args = parser.parse_args()

    if not args.data.exists():
        raise FileNotFoundError(f"{args.data} not found. Run prepare_data.py first.")

    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))
    from badwords.ml import ToxicityPredictor

    model_dir = args.model_dir
    if model_dir is None and (MODELS_DIR / "model.onnx").exists():
        model_dir = MODELS_DIR

    predictor = ToxicityPredictor(model_dir)
    predictor.load()
    print(f"Model axes: {', '.join(predictor.labels)}")

    frame = pd.read_csv(args.data)
    if args.limit:
        frame = frame.head(args.limit)
    texts = frame[TEXT_COLUMN].astype(str).tolist()
    print(f"Scoring {len(texts)} held-out rows")

    started = time.perf_counter()
    scored = []
    for start in range(0, len(texts), args.batch_size):
        scored.extend(predictor.predict_scores_batch(texts[start : start + args.batch_size]))
    elapsed = time.perf_counter() - started
    print(f"{elapsed:.1f}s total, {elapsed / len(texts) * 1000:.1f} ms per text\n")

    print(f"{'axis':<18}{'rows':>7}{'pos':>7}{'AUC':>8}{'AP':>8}{'best F1':>9}{'at':>7}")
    print("-" * 62)
    for axis in LABELS:
        if axis not in frame.columns or axis not in predictor.labels:
            continue
        annotated = frame[axis].notna().to_numpy()
        truth = (frame[axis].fillna(0).to_numpy() >= DECISION_THRESHOLD).astype(int)[annotated]
        values = np.array([row.get(axis) for row in scored])[annotated]
        if truth.min() == truth.max():
            print(
                f"{axis:<18}{annotated.sum():>7}{int(truth.sum()):>7}{'-':>8}{'-':>8}"
                f"{'-':>9}{'-':>7}   (no positives to score against)"
            )
            continue
        auc = roc_auc_score(truth, values)
        ap = average_precision_score(truth, values)
        f1, threshold = sweep(truth, values)
        print(
            f"{axis:<18}{annotated.sum():>7}{int(truth.sum()):>7}"
            f"{auc:>8.3f}{ap:>8.3f}{f1:>9.3f}{threshold:>7.2f}"
        )


if __name__ == "__main__":
    main()
