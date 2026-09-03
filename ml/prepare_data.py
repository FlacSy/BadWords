#!/usr/bin/env python3
"""Prepare multi-label training data from Hugging Face datasets.

Every row carries seven targets (see `labels.py`). `civil_comments` supervises
all of them; the multilingual sources know whether a text is toxic and nothing
more, so their remaining six are left empty and masked out of the loss rather
than being guessed at as zero - "not annotated" is not "not toxic".

Writes train.csv, validation.csv and test.csv. The test split comes from each
source's own held-out split where it has one, so a model can be measured on
rows the training run could not have seen.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import pandas as pd
from datasets import load_dataset
from labels import LABELS, TEXT_COLUMN

OUTPUT_DIR = Path(__file__).parent / "data" / "processed"

MIN_LENGTH = 3
MAX_LENGTH = 512

# Sources that annotate every axis, as (dataset, text column).
FULL_LABEL_SOURCES = {
    "civil_comments": ("google/civil_comments", "text"),
}

# Sources that only know overall toxicity, as (dataset, text column, label column).
BINARY_SOURCES = {
    "toxic_conversations": ("SetFit/toxic_conversations", "text", "label"),
    # Russian. Without a source this size the mix is overwhelmingly English,
    # and the model loses to its own predecessor on Russian text - measured,
    # not assumed.
    "toxic_russian": ("AlexSham/Toxic_Russian_Comments", "text", "label"),
}

# Parallel toxic/neutral pairs: the toxic side is 1, the rewritten side is 0.
PARADETOX_SOURCES = {
    "paradetox": "s-nlp/paradetox",
    "ru_paradetox": "s-nlp/ru_paradetox",
    "multilingual_paradetox": "textdetox/multilingual_paradetox",
}

PARADETOX_TOXIC_COLUMNS = (
    "input",
    "source",
    "toxic",
    "en_toxic_comment",
    "ru_toxic_comment",
    "toxic_sentence",
)
PARADETOX_NEUTRAL_COLUMNS = (
    "output",
    "target",
    "detox",
    "en_neutral_comment",
    "ru_neutral_comment",
    "neutral_sentence",
)


def _frame(texts: pd.Series, values: dict[str, pd.Series | float]) -> pd.DataFrame:
    """Build a frame with the full label space, unknown axes left as NaN."""
    df = pd.DataFrame({TEXT_COLUMN: texts.astype(str)})
    for label in LABELS:
        df[label] = values.get(label, float("nan"))
    return df


def _clean(df: pd.DataFrame) -> pd.DataFrame:
    df = df.dropna(subset=[TEXT_COLUMN])
    df[TEXT_COLUMN] = df[TEXT_COLUMN].astype(str).str.strip()
    df = df[df[TEXT_COLUMN].str.len().between(MIN_LENGTH, MAX_LENGTH)]
    return df.drop_duplicates(subset=[TEXT_COLUMN])


def load_full_labels(dataset: str, text_col: str, split: str) -> pd.DataFrame:
    """Load a source that annotates all seven axes."""
    print(f"  {dataset} [{split}] (all axes)")
    raw = load_dataset(dataset, split=split).to_pandas()
    values = {label: raw[label].astype(float) for label in LABELS if label in raw.columns}
    missing = [label for label in LABELS if label not in raw.columns]
    if missing:
        print(f"    warning: {dataset} does not annotate {missing}")
    return _clean(_frame(raw[text_col], values))


def load_binary(dataset: str, text_col: str, label_col: str, split: str) -> pd.DataFrame:
    """Load a source that only knows overall toxicity."""
    print(f"  {dataset} [{split}] (toxicity only)")
    raw = load_dataset(dataset, split=split).to_pandas()
    labels = pd.to_numeric(raw[label_col], errors="coerce").astype(float)
    return _clean(_frame(raw[text_col], {LABELS[0]: labels}))


def load_paradetox(dataset: str) -> pd.DataFrame:
    """Load parallel toxic/neutral pairs as two rows each."""
    print(f"  {dataset} (toxicity only, parallel pairs)")
    frames = []
    splits = load_dataset(dataset)
    for split_name in splits:
        raw = splits[split_name].to_pandas()
        toxic_col = next((c for c in PARADETOX_TOXIC_COLUMNS if c in raw.columns), None)
        neutral_col = next((c for c in PARADETOX_NEUTRAL_COLUMNS if c in raw.columns), None)
        if not toxic_col or not neutral_col:
            print(f"    skip {split_name}: columns {list(raw.columns)}")
            continue
        frames.append(_frame(raw[toxic_col], {LABELS[0]: 1.0}))
        frames.append(_frame(raw[neutral_col], {LABELS[0]: 0.0}))
    if not frames:
        return _frame(pd.Series(dtype=str), {})
    return _clean(pd.concat(frames, ignore_index=True))


def reserve_test_rows(
    df: pd.DataFrame, fraction: float, seed: int
) -> tuple[pd.DataFrame, pd.DataFrame]:
    """Split a source into (training pool, reserved test rows).

    Sources with their own held-out split contribute it directly; everything
    else has to reserve rows here, or it never gets measured at all. The
    multilingual sources are exactly the "everything else" - without this the
    test set is English-only and the model's Russian quality is a guess.
    """
    if df.empty:
        return df, df
    reserved = df.sample(frac=fraction, random_state=seed)
    return df.drop(reserved.index), reserved


def balance(df: pd.DataFrame, ratio: float, cap: int | None, seed: int) -> pd.DataFrame:
    """Sample down to `ratio` toxic rows, capped at `cap` in total.

    Balancing is on overall toxicity only: the rarer axes (threat, identity
    attack) are far too sparse to balance without throwing away most of the
    data, so they are left at their natural rate and the loss handles it.
    """
    toxic = df[df[LABELS[0]].fillna(0) >= 0.5]
    clean = df[df[LABELS[0]].fillna(0) < 0.5]
    n_clean = int(len(toxic) * (1 - ratio) / ratio) if ratio > 0 else len(clean)
    sampled = pd.concat([toxic, clean.sample(n=min(n_clean, len(clean)), random_state=seed)])
    sampled = sampled.sample(frac=1, random_state=seed)
    if cap and len(sampled) > cap:
        sampled = sampled.head(cap)
    return sampled


def report(name: str, df: pd.DataFrame) -> None:
    """Print how many rows supervise each axis, and how many are positive."""
    print(f"\n{name}: {len(df)} rows")
    for label in LABELS:
        known = df[label].notna().sum()
        positive = (df[label].fillna(0) >= 0.5).sum()
        share = f"{positive / known:.2%}" if known else "-"
        print(f"  {label:<18} annotated {known:>7}  positive {positive:>6} ({share})")


def main() -> None:
    parser = argparse.ArgumentParser(description="Prepare multi-label training data")
    parser.add_argument(
        "--max-per-source",
        type=int,
        default=400_000,
        help="Cap rows taken from each source before balancing",
    )
    parser.add_argument(
        "--max-total", type=int, default=300_000, help="Cap rows in the training split"
    )
    parser.add_argument(
        "--positive-ratio", type=float, default=0.35, help="Share of toxic rows after balancing"
    )
    parser.add_argument(
        "--validation-size", type=int, default=8_000, help="Rows held out for validation"
    )
    parser.add_argument(
        "--test-fraction",
        type=float,
        default=0.1,
        help="Share of a source without its own test split to reserve for testing",
    )
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--output-dir", type=Path, default=OUTPUT_DIR)
    args = parser.parse_args()

    print("Loading training sources")
    frames = []
    for dataset, text_col in FULL_LABEL_SOURCES.values():
        frames.append(load_full_labels(dataset, text_col, "train"))
    for dataset, text_col, label_col in BINARY_SOURCES.values():
        frames.append(load_binary(dataset, text_col, label_col, "train"))
    reserved = []
    for dataset in PARADETOX_SOURCES.values():
        try:
            frame = load_paradetox(dataset)
        except Exception as exc:
            print(f"    skip {dataset}: {exc}")
            continue
        # These sources ship no test split of their own.
        keep, held = reserve_test_rows(frame, args.test_fraction, args.seed)
        frames.append(keep)
        reserved.append(held)
        print(f"    reserved {len(held)} rows for the test split")

    frames = [
        f.sample(n=min(len(f), args.max_per_source), random_state=args.seed)
        for f in frames
        if len(f)
    ]
    if not frames:
        raise RuntimeError("no sources loaded")

    pool = pd.concat(frames, ignore_index=True).drop_duplicates(subset=[TEXT_COLUMN])
    report("Pool", pool)

    balanced = balance(pool, args.positive_ratio, args.max_total, args.seed)

    print("\nLoading held-out sources")
    held_out = []
    for dataset, text_col in FULL_LABEL_SOURCES.values():
        held_out.append(load_full_labels(dataset, text_col, "test"))
    for dataset, text_col, label_col in BINARY_SOURCES.values():
        held_out.append(load_binary(dataset, text_col, label_col, "test"))

    test = pd.concat([*held_out, *reserved], ignore_index=True).drop_duplicates(
        subset=[TEXT_COLUMN]
    )
    # Anything the training pool touched cannot be part of the test set.
    test = test[~test[TEXT_COLUMN].isin(set(pool[TEXT_COLUMN]))]
    test = balance(test, 0.5, 10_000, args.seed)

    validation = balanced.head(args.validation_size)
    train = balanced.iloc[args.validation_size :]

    report("Train", train)
    report("Validation", validation)
    report("Test (held out)", test)

    args.output_dir.mkdir(parents=True, exist_ok=True)
    for name, frame in (("train", train), ("validation", validation), ("test", test)):
        path = args.output_dir / f"{name}.csv"
        frame.to_csv(path, index=False)
        print(f"\nSaved {len(frame)} rows to {path}")


if __name__ == "__main__":
    main()
