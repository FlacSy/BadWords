#!/usr/bin/env python3
"""Train the multi-label toxicity model and export it to ONNX.

Seven sigmoid heads rather than one softmax pair, so a caller gets `insult`
0.91 / `threat` 0.02 instead of "toxic". Targets are the fraction of
annotators who picked an axis, which is a better signal than a thresholded
bit, and axes a source did not annotate are masked out of the loss instead of
being taught as zero.

Run: make ml-train
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import numpy as np
import pandas as pd
import torch
from datasets import Dataset
from sklearn.metrics import roc_auc_score
from torch import nn
from transformers import (
    AutoModelForSequenceClassification,
    AutoTokenizer,
    Trainer,
    TrainingArguments,
    set_seed,
)

# Importable whether run as `python train.py` from ml/ or `python ml/train.py`
# from the repository root.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from labels import DECISION_THRESHOLD, LABELS, TEXT_COLUMN
from prepare_data import OUTPUT_DIR

MODELS_DIR = Path(__file__).parent / "models"
ID2LABEL = dict(enumerate(LABELS))
LABEL2ID = {label: index for index, label in enumerate(LABELS)}


def load_split(path: Path) -> Dataset:
    """Read a prepared CSV into targets plus a mask of what is annotated."""
    frame = pd.read_csv(path)
    targets = frame[list(LABELS)].to_numpy(dtype=np.float32)
    mask = (~np.isnan(targets)).astype(np.float32)
    targets = np.nan_to_num(targets, nan=0.0)
    return Dataset.from_dict(
        {
            "text": frame[TEXT_COLUMN].astype(str).tolist(),
            "labels": targets.tolist(),
            "label_mask": mask.tolist(),
        }
    )


class MaskedBCETrainer(Trainer):
    """Binary cross-entropy over the axes each row actually annotates."""

    def compute_loss(
        self,
        model: nn.Module,
        inputs: dict,
        return_outputs: bool = False,  # noqa: FBT001, FBT002 - Trainer's signature
        **kwargs: object,  # noqa: ARG002 - Trainer passes num_items_in_batch
    ) -> torch.Tensor | tuple:
        """Loss over the annotated axes only."""
        labels = inputs.pop("labels")
        mask = inputs.pop("label_mask")
        outputs = model(**inputs)
        per_element = nn.functional.binary_cross_entropy_with_logits(
            outputs.logits, labels, reduction="none"
        )
        known = mask.sum()
        loss = (per_element * mask).sum() / torch.clamp(known, min=1.0)
        return (loss, outputs) if return_outputs else loss


def compute_metrics(eval_pred) -> dict[str, float]:
    """Per-axis ROC-AUC and best achievable F1, over annotated rows only.

    A fixed 0.5 threshold says almost nothing here: `severe_toxicity` has
    barely any row where a majority of annotators agreed, so its F1 at 0.5 is
    0.0 however well the head ranks. AUC is threshold-free, and the best-F1
    sweep reports the threshold a caller should actually use.
    """
    logits, labels = eval_pred
    if isinstance(labels, tuple):
        labels, mask = labels
    else:
        mask = np.ones_like(labels)

    probabilities = 1.0 / (1.0 + np.exp(-logits))
    metrics: dict[str, float] = {}
    aucs = []

    for index, label in enumerate(LABELS):
        annotated = mask[:, index] > 0
        if annotated.sum() < 2:
            continue
        truth = (labels[annotated, index] >= DECISION_THRESHOLD).astype(int)
        scores = probabilities[annotated, index]
        if truth.min() == truth.max():
            continue

        auc = float(roc_auc_score(truth, scores))
        metrics[f"auc_{label}"] = auc
        aucs.append(auc)

        best_f1, best_threshold = 0.0, DECISION_THRESHOLD
        for threshold in np.arange(0.05, 0.96, 0.05):
            predicted = scores >= threshold
            true_positive = float((predicted & (truth == 1)).sum())
            if true_positive == 0:
                continue
            precision = true_positive / float(predicted.sum())
            recall = true_positive / float((truth == 1).sum())
            f1 = 2 * precision * recall / (precision + recall)
            if f1 > best_f1:
                best_f1, best_threshold = f1, float(threshold)
        metrics[f"f1_{label}"] = best_f1
        metrics[f"threshold_{label}"] = best_threshold

    metrics["auc_macro"] = float(np.mean(aucs)) if aucs else 0.0
    return metrics


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Train the multi-label toxicity model")
    parser.add_argument("--data-dir", type=Path, default=OUTPUT_DIR)
    parser.add_argument("--model", type=str, default="xlm-roberta-base")
    parser.add_argument("--epochs", type=float, default=2.0)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--gradient-accumulation", type=int, default=2)
    parser.add_argument("--lr", type=float, default=2e-5)
    parser.add_argument("--max-length", type=int, default=128)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--output-dir", type=Path, default=MODELS_DIR / "checkpoints")
    parser.add_argument("--onnx-dir", type=Path, default=MODELS_DIR)
    parser.add_argument("--max-train", type=int, default=None, help="Cap training rows")
    parser.add_argument("--skip-export", action="store_true", help="Train without exporting ONNX")
    # XLM-R keeps 192M of its 278M parameters in the embedding matrix, and
    # AdamW holds two fp32 states per parameter. Freezing embeddings is what
    # makes this fit on an 8 GB card; the vocabulary is already well trained.
    embeddings = parser.add_mutually_exclusive_group()
    embeddings.add_argument("--freeze-embeddings", dest="freeze_embeddings", action="store_true")
    embeddings.add_argument(
        "--no-freeze-embeddings", dest="freeze_embeddings", action="store_false"
    )
    parser.set_defaults(freeze_embeddings=True)
    checkpointing = parser.add_mutually_exclusive_group()
    checkpointing.add_argument(
        "--gradient-checkpointing", dest="gradient_checkpointing", action="store_true"
    )
    checkpointing.add_argument(
        "--no-gradient-checkpointing", dest="gradient_checkpointing", action="store_false"
    )
    parser.set_defaults(gradient_checkpointing=None)
    fp16 = parser.add_mutually_exclusive_group()
    fp16.add_argument("--fp16", dest="fp16", action="store_true")
    fp16.add_argument("--no-fp16", dest="fp16", action="store_false")
    parser.set_defaults(fp16=None)
    args = parser.parse_args()
    if args.fp16 is None:
        args.fp16 = torch.cuda.is_available()
    if args.gradient_checkpointing is None:
        args.gradient_checkpointing = torch.cuda.is_available()
    return args


def main() -> None:
    args = parse_args()
    set_seed(args.seed)

    train_path = args.data_dir / "train.csv"
    validation_path = args.data_dir / "validation.csv"
    if not train_path.exists():
        raise FileNotFoundError(f"{train_path} not found. Run prepare_data.py first.")

    train_ds = load_split(train_path)
    if args.max_train:
        train_ds = train_ds.select(range(min(args.max_train, len(train_ds))))
    eval_ds = load_split(validation_path) if validation_path.exists() else None
    print(f"Train {len(train_ds)}" + (f", validation {len(eval_ds)}" if eval_ds else ""))

    tokenizer = AutoTokenizer.from_pretrained(args.model)

    def tokenize(batch: dict) -> dict:
        return tokenizer(
            batch["text"], truncation=True, padding="max_length", max_length=args.max_length
        )

    train_ds = train_ds.map(tokenize, batched=True, remove_columns=["text"])
    train_ds.set_format("torch")
    if eval_ds is not None:
        eval_ds = eval_ds.map(tokenize, batched=True, remove_columns=["text"])
        eval_ds.set_format("torch")

    model = AutoModelForSequenceClassification.from_pretrained(
        args.model,
        num_labels=len(LABELS),
        problem_type="multi_label_classification",
        id2label=ID2LABEL,
        label2id=LABEL2ID,
    )

    if args.freeze_embeddings:
        frozen = 0
        for parameter in model.base_model.embeddings.parameters():
            parameter.requires_grad = False
            frozen += parameter.numel()
        trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
        print(f"Froze {frozen / 1e6:.0f}M embedding parameters, {trainable / 1e6:.0f}M trainable")

    training_args = TrainingArguments(
        output_dir=str(args.output_dir),
        num_train_epochs=args.epochs,
        per_device_train_batch_size=args.batch_size,
        per_device_eval_batch_size=args.batch_size * 2,
        gradient_accumulation_steps=args.gradient_accumulation,
        learning_rate=args.lr,
        warmup_ratio=0.06,
        weight_decay=0.01,
        seed=args.seed,
        fp16=args.fp16,
        dataloader_num_workers=args.workers,
        dataloader_pin_memory=True,
        gradient_checkpointing=args.gradient_checkpointing,
        gradient_checkpointing_kwargs={"use_reentrant": False},
        logging_steps=100,
        eval_strategy="no" if eval_ds is None else "epoch",
        save_strategy="epoch",
        save_total_limit=2,
        load_best_model_at_end=eval_ds is not None,
        metric_for_best_model="auc_macro",
        greater_is_better=True,
        label_names=["labels", "label_mask"],
        report_to=[],
    )

    trainer = MaskedBCETrainer(
        model=model,
        args=training_args,
        train_dataset=train_ds,
        eval_dataset=eval_ds,
        compute_metrics=compute_metrics if eval_ds is not None else None,
    )

    trainer.train()
    final_dir = args.output_dir / "final"
    trainer.save_model(str(final_dir))
    tokenizer.save_pretrained(str(final_dir))

    if eval_ds is not None:
        print("\nValidation:")
        for key, value in sorted(trainer.evaluate().items()):
            print(f"  {key:<28} {value:.4f}")

    if args.skip_export:
        print(f"\nTrained model in {final_dir}; export it with: python export.py")
        return

    # Exporting is its own script so that a failure here does not cost another
    # training run.
    from export import export

    export(final_dir, args.onnx_dir)
    print(f"\nExported to {args.onnx_dir}")
    print("Next: python evaluate.py, then make ml-quantize and make ml-package")


if __name__ == "__main__":
    main()
