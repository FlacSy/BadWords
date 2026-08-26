#!/usr/bin/env python3
"""Train a toxicity classifier and export it to ONNX.

Fine-tunes XLM-RoBERTa (or any sequence-classification checkpoint) on the data
prepared by prepare_data.py, then exports an ONNX model for inference.

Run: make ml-train
"""

import argparse
import sys
from pathlib import Path

import numpy as np
import pandas as pd
from datasets import Dataset
from optimum.onnxruntime import ORTModelForSequenceClassification
from sklearn.metrics import accuracy_score, precision_recall_fscore_support
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
from prepare_data import OUTPUT_DIR

#: Index 1 is the toxic class throughout the pipeline; recorded in the exported
#: config so that inference does not have to assume it.
ID2LABEL = {0: "clean", 1: "toxic"}
LABEL2ID = {"clean": 0, "toxic": 1}

MODELS_DIR = Path(__file__).parent / "models"
DEFAULT_DATA = OUTPUT_DIR / "train.csv"


def load_data(path: Path) -> Dataset:
    """Load CSV, return HuggingFace Dataset."""
    df = pd.read_csv(path)
    df = df.rename(columns={"comment_text": "text"})
    return Dataset.from_pandas(df[["text", "label"]])


def compute_metrics(eval_pred) -> dict[str, float]:
    """Accuracy, precision, recall and F1 for the toxic class.

    Without this the trainer reports only eval_loss, which says nothing about
    whether the model is usable as a moderation signal.
    """
    logits, labels = eval_pred
    predictions = np.argmax(logits, axis=-1)
    precision, recall, f1, _ = precision_recall_fscore_support(
        labels,
        predictions,
        average="binary",
        pos_label=1,
        zero_division=0,
    )
    return {
        "accuracy": float(accuracy_score(labels, predictions)),
        "precision": float(precision),
        "recall": float(recall),
        "f1": float(f1),
    }


def main() -> None:
    """Train and export."""
    parser = argparse.ArgumentParser(description="Train toxicity classifier")
    parser.add_argument(
        "--data",
        type=Path,
        default=DEFAULT_DATA,
        help="Path to train.csv",
    )
    parser.add_argument(
        "--model",
        type=str,
        default="xlm-roberta-base",
        help="Model: xlm-roberta-base (default, quantize to ~250MB) or distilbert-base-multilingual-cased",
    )
    parser.add_argument(
        "--epochs",
        type=int,
        default=2,
        help="Training epochs (default: 2)",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=8,
        help="Batch size (default: 8 for xlm-roberta, 32 for distilbert)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=MODELS_DIR / "checkpoints",
        help="Checkpoint directory",
    )
    parser.add_argument(
        "--onnx-dir",
        type=Path,
        default=MODELS_DIR,
        help="ONNX output directory",
    )
    parser.add_argument(
        "--full-dataset",
        action="store_true",
        help="Use 100%% of data for training (no eval split)",
    )
    parser.add_argument(
        "--max-length",
        type=int,
        default=128,
        help="Sequence length in tokens (default: 128, matching inference)",
    )
    parser.add_argument(
        "--lr",
        type=float,
        default=2e-5,
        help="Learning rate (default: 2e-5)",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed (default: 42)",
    )
    parser.add_argument(
        "--gradient-accumulation",
        type=int,
        default=4,
        help="Gradient accumulation steps; effective batch = batch-size x this",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=4,
        help="Dataloader workers (default: 4)",
    )
    fp16 = parser.add_mutually_exclusive_group()
    fp16.add_argument(
        "--fp16",
        dest="fp16",
        action="store_true",
        help="Mixed precision (default when a GPU is present)",
    )
    fp16.add_argument(
        "--no-fp16",
        dest="fp16",
        action="store_false",
        help="Full precision; required on CPU-only machines",
    )
    parser.set_defaults(fp16=None)
    args = parser.parse_args()

    if args.fp16 is None:
        # fp16 was unconditional before, which made training fail outright on a
        # machine without a GPU.
        try:
            import torch

            args.fp16 = torch.cuda.is_available()
        except ImportError:
            args.fp16 = False
    set_seed(args.seed)

    if not args.data.exists():
        raise FileNotFoundError(f"Data not found: {args.data}. Run prepare_data.py first.")

    dataset = load_data(args.data)
    if args.full_dataset:
        train_ds = dataset
        eval_ds = None
        print(f"Full dataset: {len(train_ds)} samples (no eval split)")
    else:
        split = dataset.train_test_split(test_size=0.1, seed=42)
        train_ds = split["train"]
        eval_ds = split["test"]
        print(f"Train: {len(train_ds)}, Eval: {len(eval_ds)}")

    tokenizer = AutoTokenizer.from_pretrained(args.model)

    def tokenize(examples):
        return tokenizer(
            examples["text"],
            truncation=True,
            padding="max_length",
            max_length=args.max_length,
        )

    train_ds = train_ds.map(tokenize, batched=True, remove_columns=["text"])
    train_ds.set_format("torch")
    if eval_ds is not None:
        eval_ds = eval_ds.map(tokenize, batched=True, remove_columns=["text"])
        eval_ds.set_format("torch")

    model = AutoModelForSequenceClassification.from_pretrained(
        args.model,
        num_labels=2,
        id2label=ID2LABEL,
        label2id=LABEL2ID,
    )

    training_args = TrainingArguments(
        output_dir=str(args.output_dir),
        num_train_epochs=args.epochs,
        per_device_train_batch_size=args.batch_size,
        per_device_eval_batch_size=args.batch_size,
        gradient_accumulation_steps=args.gradient_accumulation,
        learning_rate=args.lr,
        seed=args.seed,
        fp16=args.fp16,  # mixed precision; requires a GPU
        dataloader_num_workers=args.workers,
        dataloader_pin_memory=True,
        eval_strategy="no" if eval_ds is None else "epoch",
        save_strategy="epoch",
        load_best_model_at_end=eval_ds is not None,
        metric_for_best_model="eval_loss",
    )

    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=train_ds,
        eval_dataset=eval_ds,
        compute_metrics=compute_metrics if eval_ds is not None else None,
    )

    trainer.train()
    trainer.save_model(str(args.output_dir / "final"))

    # Export to ONNX
    args.onnx_dir.mkdir(parents=True, exist_ok=True)
    ort_model = ORTModelForSequenceClassification.from_pretrained(
        str(args.output_dir / "final"),
        export=True,
    )
    ort_model.save_pretrained(str(args.onnx_dir))
    tokenizer.save_pretrained(str(args.onnx_dir))

    print(f"Model exported to {args.onnx_dir}")
    print("Next: make ml-quantize, then make ml-package")


if __name__ == "__main__":
    main()
