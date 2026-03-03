#!/usr/bin/env python3
"""Train toxicity classifier and export to ONNX.

Fine-tunes DistilBERT on prepared data, exports model for inference.
"""

import argparse
from pathlib import Path

import pandas as pd
from datasets import Dataset
from transformers import (
    AutoModelForSequenceClassification,
    AutoTokenizer,
    Trainer,
    TrainingArguments,
)
from optimum.onnxruntime import ORTModelForSequenceClassification

from prepare_data import OUTPUT_DIR

MODELS_DIR = Path(__file__).parent / "models"
DEFAULT_DATA = OUTPUT_DIR / "train.csv"


def load_data(path: Path) -> Dataset:
    """Load CSV, return HuggingFace Dataset."""
    df = pd.read_csv(path)
    df = df.rename(columns={"comment_text": "text"})
    return Dataset.from_pandas(df[["text", "label"]])


def main() -> None:
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
    args = parser.parse_args()

    if not args.data.exists():
        raise FileNotFoundError(
            f"Data not found: {args.data}. Run prepare_data.py first."
        )

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
            max_length=64,
        )

    train_ds = train_ds.map(tokenize, batched=True, remove_columns=["text"])
    train_ds.set_format("torch")
    if eval_ds is not None:
        eval_ds = eval_ds.map(tokenize, batched=True, remove_columns=["text"])
        eval_ds.set_format("torch")

    model = AutoModelForSequenceClassification.from_pretrained(
        args.model,
        num_labels=2,
    )

    training_args = TrainingArguments(
        output_dir=str(args.output_dir),
        num_train_epochs=args.epochs,
        per_device_train_batch_size=args.batch_size,
        per_device_eval_batch_size=args.batch_size,
        gradient_accumulation_steps=4,  # effective batch = batch_size * 4
        fp16=True,  # mixed precision, saves VRAM
        dataloader_num_workers=4,
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


if __name__ == "__main__":
    main()
