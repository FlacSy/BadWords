#!/usr/bin/env python3
"""Prepare training data from Hugging Face datasets.

Supports multiple datasets and multilingual sources.
"""

import argparse
from pathlib import Path

import pandas as pd
from datasets import load_dataset

TOXIC_COLUMNS = [
    "toxic",
    "severe_toxic",
    "obscene",
    "threat",
    "insult",
    "identity_hate",
]
TEXT_COLUMN = "comment_text"
OUTPUT_DIR = Path(__file__).parent / "data" / "processed"

# Preset: (dataset_name, text_col, label_source)
# label_source: "label" | "toxic_*" | "toxicity" (>=0.5) | "paradetox" (input=1, output=0)
DATASET_PRESETS = {
    # English, large
    "civil_comments": ("google/civil_comments", "text", "toxicity"),
    "toxic_conversations": ("SetFit/toxic_conversations", "text", "label"),
    # English, parallel detox
    "paradetox": ("s-nlp/paradetox", None, "paradetox"),
    # Russian
    "ru_paradetox": ("s-nlp/ru_paradetox", None, "paradetox"),
    # Multilingual (9 langs: en, ru, uk, de, es, ar, zh, hi, am)
    "multilingual_paradetox": ("textdetox/multilingual_paradetox", None, "paradetox"),
}


def load_single(
    dataset_name: str,
    label_source: str,
    text_col: str | None,
    max_samples: int | None,
    min_length: int,
    max_length: int,
) -> pd.DataFrame:
    """Load and process a single dataset."""
    print(f"  Loading {dataset_name}...")
    dataset = load_dataset(dataset_name)
    split = "train" if "train" in dataset else list(dataset.keys())[0]
    df = dataset[split].to_pandas()

    if label_source == "paradetox":
        # toxic = 1, neutral/detox = 0
        input_col = next(
            (
                c
                for c in [
                    "input",
                    "source",
                    "toxic",
                    "en_toxic_comment",
                    "ru_toxic_comment",
                    "toxic_sentence",
                ]
                if c in df.columns
            ),
            None,
        )
        output_col = next(
            (
                c
                for c in [
                    "output",
                    "target",
                    "detox",
                    "en_neutral_comment",
                    "ru_neutral_comment",
                    "neutral_sentence",
                ]
                if c in df.columns
            ),
            None,
        )
        if not input_col or not output_col:
            raise ValueError(
                f"ParaDetox format needs toxic/neutral columns. Columns: {list(df.columns)}"
            )
        toxic_df = df[[input_col]].rename(columns={input_col: TEXT_COLUMN})
        toxic_df["label"] = 1
        clean_df = df[[output_col]].rename(columns={output_col: TEXT_COLUMN})
        clean_df["label"] = 0
        df = pd.concat([toxic_df, clean_df], ignore_index=True)
    else:
        text_col = text_col or next(
            (
                c
                for c in ["comment_text", "text", "comment", "sentence", "content"]
                if c in df.columns
            ),
            None,
        )
        if not text_col:
            raise ValueError(f"Text column not found. Columns: {list(df.columns)}")
        df = df.rename(columns={text_col: TEXT_COLUMN})

        if label_source == "label":
            df["label"] = df["label"].astype(int)
        elif label_source == "toxicity":
            # civil_comments: toxicity 0-1, threshold 0.5
            tox_col = next((c for c in ["toxicity", "toxic"] if c in df.columns), None)
            if not tox_col:
                raise ValueError(
                    f"Toxicity column not found. Columns: {list(df.columns)}"
                )
            df["label"] = (df[tox_col].fillna(0) >= 0.5).astype(int)
        elif label_source.startswith("toxic"):
            toxic_cols = [c for c in TOXIC_COLUMNS if c in df.columns]
            df["label"] = df[toxic_cols].max(axis=1).astype(int)

    df = df.dropna(subset=[TEXT_COLUMN])
    df[TEXT_COLUMN] = df[TEXT_COLUMN].astype(str)
    df = df[df[TEXT_COLUMN].str.strip().str.len().between(min_length, max_length)]
    df = df.drop_duplicates(subset=[TEXT_COLUMN])

    if max_samples and len(df) > max_samples:
        df = df.sample(n=max_samples, random_state=42)

    return df[["comment_text", "label"]]


def load_multilingual(max_samples_per_dataset: int | None = None) -> pd.DataFrame:
    """Load multilingual mix: EN (large) + RU + multilingual paradetox."""
    dfs = []

    # English: toxic_conversations (1.8M)
    try:
        df = load_single(
            "SetFit/toxic_conversations",
            "label",
            "text",
            max_samples=max_samples_per_dataset,
            min_length=3,
            max_length=512,
        )
        dfs.append(df)
    except Exception as e:
        print(f"  Skip toxic_conversations: {e}")

    # English: civil_comments (2M)
    try:
        df = load_single(
            "google/civil_comments",
            "toxicity",
            "text",
            max_samples=max_samples_per_dataset,
            min_length=3,
            max_length=512,
        )
        dfs.append(df)
    except Exception as e:
        print(f"  Skip civil_comments: {e}")

    # English + Russian + multilingual paradetox
    for name, (ds, _, src) in DATASET_PRESETS.items():
        if (
            name in ("paradetox", "ru_paradetox", "multilingual_paradetox")
            and src == "paradetox"
        ):
            try:
                df = load_single(ds, src, None, max_samples_per_dataset, 3, 512)
                dfs.append(df)
            except Exception as e:
                print(f"  Skip {name}: {e}")

    if not dfs:
        raise RuntimeError("No datasets loaded")
    return pd.concat(dfs, ignore_index=True).drop_duplicates(subset=[TEXT_COLUMN])


def balance(
    df: pd.DataFrame, ratio: float = 0.3, max_total: int | None = None
) -> pd.DataFrame:
    """Balance classes. ratio = fraction of positive samples. max_total caps result size."""
    pos = df[df["label"] == 1]
    neg = df[df["label"] == 0]
    n_pos = len(pos)
    n_neg = int(n_pos * (1 - ratio) / ratio) if ratio > 0 else len(neg)
    n_neg = min(n_neg, len(neg))
    pos_sampled = pos.sample(n=n_pos, random_state=42)
    neg_sampled = neg.sample(n=n_neg, random_state=42)
    result = pd.concat([pos_sampled, neg_sampled]).sample(frac=1, random_state=42)
    if max_total and len(result) > max_total:
        result = result.sample(n=max_total, random_state=42)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description="Prepare ML training data")
    parser.add_argument(
        "--preset",
        type=str,
        choices=["single", "multilingual"] + list(DATASET_PRESETS),
        default="multilingual",
        help="Preset: multilingual (EN+RU+9langs), single (use --dataset), or preset name",
    )
    parser.add_argument(
        "--dataset",
        type=str,
        default="SetFit/toxic_conversations",
        help="HuggingFace dataset when preset=single",
    )
    parser.add_argument(
        "--max-samples",
        type=int,
        default=None,
        help="Max samples per dataset (default: 200k for EN, 50k for paradetox)",
    )
    parser.add_argument(
        "--positive-ratio",
        type=float,
        default=0.3,
        help="Target ratio of positive samples (default: 0.3)",
    )
    parser.add_argument(
        "--max-total",
        type=int,
        default=None,
        help="Cap total samples after balance (for controlled training time)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=OUTPUT_DIR,
        help="Output directory",
    )
    args = parser.parse_args()

    if args.preset == "multilingual":
        df = load_multilingual(args.max_samples)
    elif args.preset == "single":
        preset = DATASET_PRESETS.get(args.dataset)
        if preset:
            ds_name, text_col, label_src = preset
        else:
            ds_name, text_col, label_src = args.dataset, "text", "label"
        df = load_single(ds_name, label_src, text_col, args.max_samples, 3, 512)
    else:
        ds_name, text_col, label_src = DATASET_PRESETS[args.preset]
        df = load_single(ds_name, label_src, text_col, args.max_samples, 3, 512)

    print(
        f"Total: {len(df)} samples, {df['label'].sum()} positive ({df['label'].mean():.2%})"
    )

    df_balanced = balance(df, ratio=args.positive_ratio, max_total=args.max_total)
    print(f"Balanced: {len(df_balanced)} samples")

    args.output_dir.mkdir(parents=True, exist_ok=True)
    out_path = args.output_dir / "train.csv"
    df_balanced.to_csv(out_path, index=False)
    print(f"Saved to {out_path}")


if __name__ == "__main__":
    main()
