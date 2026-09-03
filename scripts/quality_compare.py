#!/usr/bin/env python3
"""Compare quality of BadWords vs glin-profanity (rule-based + ML).

Run: python scripts/quality_compare.py
Requires: pip install badwords-py glin-profanity

Optional: pip install datasets  (for 1000+1000 samples from HuggingFace)
Optional ML: pip install 'badwords-py[ml]' glin-profanity[ml]
"""

from __future__ import annotations

import argparse
import random
import sys

# Curated: (text, expected_label) where 1=toxic, 0=clean
QUALITY_TEST_CASES: list[tuple[str, int]] = [
    # --- Clean (0) ---
    ("Hello, how are you today?", 0),
    ("Have a nice day!", 0),
    ("The class is great.", 0),
    ("I'm a professional assassin.", 0),
    ("Scunthorpe is a town in England.", 0),
    ("This is a stupid idea.", 0),
    ("Привет, как дела?", 0),
    ("Спасибо за помощь!", 0),
    ("The bass is low.", 0),
    ("I passed the exam.", 0),
    ("   ", 0),
    ("", 0),
    # --- Obvious profanity EN (1) ---
    ("fuck off", 1),
    ("shit happens", 1),
    ("you are an asshole", 1),
    ("son of a bitch", 1),
    ("damn it", 1),
    ("bitch please", 1),
    ("bastard", 1),
    ("dumbass", 1),
    # --- Obvious profanity RU (1) ---
    ("Иди нахуй", 1),
    ("сука", 1),
    ("блять", 1),
    ("пизда", 1),
    ("хуй", 1),
    ("мудак", 1),
    # --- Evasion (leetspeak, homoglyphs) ---
    ("f4ck you", 1),
    ("sh1t", 1),
    ("b1tch", 1),
    ("a$$hole", 1),
    ("f.u.c.k", 1),
    ("f u c k", 1),
    # --- Context / toxic intent ---
    ("You are stupid and worthless", 1),
    ("Go kill yourself", 1),
    ("I hate you", 1),
]


def _load_from_huggingface(
    n_toxic: int = 1000,
    n_clean: int = 1000,
    seed: int = 42,
    max_len: int = 512,
) -> list[tuple[str, int]]:
    """Load samples from SetFit/toxic_conversations. Returns (text, label) list."""
    from datasets import load_dataset

    # Load first 60k rows (~5k toxic at 8%) to get enough samples without full 1.8M
    print("Loading SetFit/toxic_conversations (first 60k rows)...")
    ds = load_dataset("SetFit/toxic_conversations", split="train[:60000]")
    toxic = [
        row
        for row in ds
        if row["label"] == 1 and row["text"].strip() and len(row["text"]) <= max_len
    ]
    clean = [
        row
        for row in ds
        if row["label"] == 0 and row["text"].strip() and len(row["text"]) <= max_len
    ]
    rng = random.Random(seed)
    rng.shuffle(toxic)
    rng.shuffle(clean)
    n_t = min(len(toxic), n_toxic)
    n_c = min(len(clean), n_clean)
    if n_t < n_toxic or n_c < n_clean:
        print(f"  Note: got {n_t} toxic, {n_c} clean (requested {n_toxic}, {n_clean})")
    samples = [(t["text"], 1) for t in toxic[:n_t]]
    samples += [(c["text"], 0) for c in clean[:n_c]]
    rng.shuffle(samples)
    return samples


def _metrics(tp: int, fp: int, tn: int, fn: int) -> dict[str, float]:
    """Compute accuracy, precision, recall, F1."""
    acc = (tp + tn) / (tp + fp + tn + fn) if (tp + fp + tn + fn) > 0 else 0.0
    prec = tp / (tp + fp) if (tp + fp) > 0 else 0.0
    rec = tp / (tp + fn) if (tp + fn) > 0 else 0.0
    f1 = 2 * prec * rec / (prec + rec) if (prec + rec) > 0 else 0.0
    return {"accuracy": acc, "precision": prec, "recall": rec, "f1": f1}


def _run_filter(
    name: str,
    pred_fn,
    cases: list[tuple[str, int]],
) -> tuple[dict[str, float], list[tuple[str, int, int, str]]]:
    """Run filter on samples, return metrics and details."""
    tp, fp, tn, fn = 0, 0, 0, 0
    details: list[tuple[str, int, int, str]] = []  # (text, expected, pred, status)

    for text, expected in cases:
        try:
            pred = 1 if pred_fn(text) else 0
        except Exception as e:
            pred = -1
            status = f"ERR: {e!r}"
        else:
            status = "✓" if pred == expected else "✗"
            if pred == 1 and expected == 1:
                tp += 1
            elif pred == 1 and expected == 0:
                fp += 1
            elif pred == 0 and expected == 1:
                fn += 1
            else:
                tn += 1

        details.append((text, expected, pred, status))

    m = _metrics(tp, fp, tn, fn)
    return m, details


def _print_results(name: str, m: dict[str, float], verbose: bool, details: list) -> None:
    print(f"\n{name}")
    print("-" * 50)
    print(f"  Accuracy:  {m['accuracy']:.2%}")
    print(f"  Precision: {m['precision']:.2%}")
    print(f"  Recall:    {m['recall']:.2%}")
    print(f"  F1:        {m['f1']:.2%}")

    if verbose:
        errors = [d for d in details if d[3] == "✗"]
        if errors:
            print(f"\n  Errors ({len(errors)}):")
            for text, exp, pred, _ in errors[:15]:
                exp_s = "toxic" if exp == 1 else "clean"
                pred_s = "toxic" if pred == 1 else "clean"
                print(f"    [{exp_s}→{pred_s}] {text!r}")
            if len(errors) > 15:
                print(f"    ... and {len(errors) - 15} more")


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare BadWords vs glin-profanity quality")
    parser.add_argument("-v", "--verbose", action="store_true", help="Show error details")
    parser.add_argument(
        "--curated",
        action="store_true",
        help="Use only curated test set (~34 samples)",
    )
    parser.add_argument(
        "-n",
        type=int,
        default=1000,
        metavar="N",
        help="Samples per class from HF dataset (default: 1000)",
    )
    parser.add_argument("--seed", type=int, default=42, help="Random seed for HF sampling")
    args = parser.parse_args()

    if args.curated:
        cases = [c for c in QUALITY_TEST_CASES if c[0].strip()]
        cases = [c for c in cases if c[0]]
        print("Using curated test set")
    else:
        try:
            cases = _load_from_huggingface(
                n_toxic=args.n,
                n_clean=args.n,
                seed=args.seed,
            )
        except ImportError:
            print("datasets not installed. Use: pip install datasets")
            print("Falling back to curated set.")
            cases = [c for c in QUALITY_TEST_CASES if c[0].strip()]
            cases = [c for c in cases if c[0]]
        except Exception as e:
            print(f"Failed to load HF dataset: {e}")
            sys.exit(1)

    print("=" * 60)
    print("Quality comparison: BadWords vs glin-profanity (en+ru)")
    print(
        f"Test cases: {len(cases)} (toxic={sum(1 for _, e in cases if e)} clean={sum(1 for _, e in cases if not e)})"
    )
    print("=" * 60)

    # --- BadWords rule-based ---
    from badwords import ProfanityFilter

    bw = ProfanityFilter()
    bw.init(languages=["en", "ru"])

    def bw_pred(text: str) -> bool:
        return bw.is_profane(text)

    m_bw, details_bw = _run_filter("BadWords (rule-based)", bw_pred, cases)
    _print_results("BadWords (rule-based)", m_bw, args.verbose, details_bw)

    results: list[tuple[str, dict[str, float]]] = [("BadWords (rule)", m_bw)]

    # --- glin rule-based ---
    try:
        from glin_profanity import Filter

        glin = Filter({"languages": ["english", "russian"]})

        def glin_pred(text: str) -> bool:
            return bool(glin.is_profane(text))

        m_glin, details_glin = _run_filter("glin-profanity (rule-based)", glin_pred, cases)
        _print_results("glin-profanity (rule-based)", m_glin, args.verbose, details_glin)
        results.append(("glin (rule)", m_glin))
    except ImportError as e:
        print(f"\nglin-profanity (rule-based): SKIPPED (ImportError: {e})")

    # --- BadWords ML ---
    # The 3.0 inference path: onnxruntime driven directly, no torch. The model
    # is used only if it is already cached, so a quality run never starts a
    # 206 MB download.
    try:
        from badwords.ml import ToxicityPredictor, get_model_dir

        pred = ToxicityPredictor(get_model_dir(download=False))
        pred.load()

        def bw_ml_pred(text: str) -> bool:
            return pred.is_toxic(text)

        m_bw_ml, details_bw_ml = _run_filter("BadWords (ML)", bw_ml_pred, cases)
        _print_results("BadWords (ML)", m_bw_ml, args.verbose, details_bw_ml)
        results.append(("BadWords (ML)", m_bw_ml))
    except Exception as e:
        print(f"\nBadWords (ML): SKIPPED ({e})")

    # --- glin ML ---
    try:
        from glin_profanity.ml import HybridFilter

        glin_ml = HybridFilter(
            {
                "languages": ["english", "russian"],
                "enable_ml": True,
                "ml_type": "transformer",
                "preload_ml": True,
            }
        )
        if glin_ml.is_ml_ready():

            def glin_ml_pred(text: str) -> bool:
                r = glin_ml.check_profanity_hybrid(text)
                if isinstance(r, dict):
                    return bool(r.get("is_toxic", r.get("contains_profanity", False)))
                return bool(r)

            m_glin_ml, details_glin_ml = _run_filter("glin-profanity (ML)", glin_ml_pred, cases)
            _print_results("glin-profanity (ML)", m_glin_ml, args.verbose, details_glin_ml)
            results.append(("glin (ML)", m_glin_ml))
        else:
            print("\nglin-profanity (ML): SKIPPED (ML not ready)")
    except ImportError:
        print("\nglin-profanity (ML): SKIPPED (pip install glin-profanity[ml])")
    except Exception as e:
        print(f"\nglin-profanity (ML): SKIPPED ({e})")

    # --- Summary table ---
    print("\n" + "=" * 60)
    print("Summary (Accuracy / F1)")
    print("=" * 60)
    for r in results:
        if isinstance(r, tuple):
            name, m = r
            print(f"  {name:<20} Acc: {m['accuracy']:.2%}  F1: {m['f1']:.2%}")

    print("=" * 60)


if __name__ == "__main__":
    main()
