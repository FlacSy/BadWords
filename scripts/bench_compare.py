#!/usr/bin/env python3
"""Compare BadWords vs glin-profanity (rule-based + ML). Run: python scripts/bench_compare.py"""

import platform
import sys
import time
from pathlib import Path

try:
    from importlib.metadata import version
except ImportError:
    def version(pkg: str) -> str:
        return "unknown"


def bench(func, *args, iterations: int = 10_000, **kwargs) -> float:
    """Return mean µs per call."""
    start = time.perf_counter()
    for _ in range(iterations):
        func(*args, **kwargs)
    return (time.perf_counter() - start) * 1e6 / iterations


def _throughput(us: float, texts_per_call: int = 1) -> str:
    """Ops/sec from µs per call. texts_per_call=5 for batch."""
    if us <= 0:
        return "N/A"
    ops = texts_per_call * 1e6 / us
    if ops >= 1000:
        return f"{ops / 1000:.1f} K/s"
    return f"{ops:.0f}/s"


def main() -> None:
    # BadWords
    from badwords import ProfanityFilter

    bw = ProfanityFilter()
    bw.init(languages=["en", "ru"])

    # glin-profanity (rule-based; glin is slower, use fewer iterations)
    from glin_profanity import Filter

    glin = Filter({"languages": ["english", "russian"]})
    glin_replace = Filter({"languages": ["english", "russian"], "replace_with": "*"})

    n_bw, n_glin = 50_000, 1_000  # glin ~500x slower

    texts_clean = "Hello, this is a clean message for testing."
    texts_bad = "fuck off"
    texts_batch = [
        "Hello world",
        "Clean message",
        "Some text with potential",
        "Another clean one",
        "Final test string",
    ]
    batch_total_len = sum(len(t) for t in texts_batch)

    # Metadata
    cpu = platform.processor() or platform.machine() or "unknown"
    try:
        glin_ver = version("glin-profanity")
    except Exception:
        glin_ver = "unknown"
    print("Benchmark: BadWords vs glin-profanity (rule-based, en+ru)")
    print("-" * 55)
    print(f"CPU:            {cpu}")
    print(f"Python:         {sys.version.split()[0]}")
    print(f"glin-profanity: {glin_ver}")
    print(f"Iterations:     BadWords {n_bw:,}  |  glin {n_glin:,}")
    print(f"Text lengths:   clean {len(texts_clean)} chars  |  bad {len(texts_bad)} chars  |  batch {batch_total_len} chars (5 texts)")
    print("-" * 55)

    # Clean
    bw_clean = bench(bw.filter_text, texts_clean, iterations=n_bw)
    glin_clean = bench(glin.is_profane, texts_clean, iterations=n_glin)
    print(f"Clean text:     BadWords {bw_clean:>7.2f} µs ({_throughput(bw_clean)})  |  glin {glin_clean:>7.2f} µs ({_throughput(glin_clean)})")

    # Bad
    bw_bad = bench(bw.filter_text, texts_bad, iterations=n_bw)
    glin_bad = bench(glin.is_profane, texts_bad, iterations=n_glin)
    print(f"Bad word:       BadWords {bw_bad:>7.2f} µs ({_throughput(bw_bad)})  |  glin {glin_bad:>7.2f} µs ({_throughput(glin_bad)})")

    # Censor
    def bw_censor():
        bw.filter_text(texts_bad, replace_character="*")

    def glin_censor_fn():
        glin_replace.check_profanity(texts_bad)

    bw_c = bench(bw_censor, iterations=n_bw)
    glin_c = bench(glin_censor_fn, iterations=n_glin)
    print(f"Censor:         BadWords {bw_c:>7.2f} µs ({_throughput(bw_c)})  |  glin {glin_c:>7.2f} µs ({_throughput(glin_c)})")

    # Batch
    def bw_batch():
        for t in texts_batch:
            bw.filter_text(t)

    def glin_batch():
        for t in texts_batch:
            glin.is_profane(t)

    bw_b = bench(bw_batch, iterations=n_bw)
    glin_b = bench(glin_batch, iterations=n_glin)
    print(f"5 texts batch:  BadWords {bw_b:>7.2f} µs ({_throughput(bw_b, 5)})  |  glin {glin_b:>7.2f} µs ({_throughput(glin_b, 5)})")

    print("-" * 55)
    print()

    # --- ML benchmarks ---
    n_ml = 100  # ML is much slower
    import os
    _prev_hf = os.environ.pop("HF_HUB_DISABLE_PROGRESS_BARS", None)
    os.environ["HF_HUB_DISABLE_PROGRESS_BARS"] = "1"

    # Results: {scenario: {backend: time_us or None}}
    ml_results: dict[str, dict[str, float | str | None]] = {
        "Clean text": {"bw": None, "glin_light": None, "glin_trans": None},
        "Bad word": {"bw": None, "glin_light": None, "glin_trans": None},
        "5 texts batch": {"bw": None, "glin_light": None, "glin_trans": None},
    }

    print("Benchmark: ML mode (en+ru, 100 iter each)")
    print("-" * 55)

    # glin ML lightweight (profanity-check)
    glin_ml_light = None
    try:
        from glin_profanity.ml import HybridFilter

        glin_ml_light = HybridFilter({
            "languages": ["english", "russian"],
            "enable_ml": True,
            "ml_type": "lightweight",
            "preload_ml": True,
        })
        if not glin_ml_light.is_ml_ready():
            glin_ml_light = None
    except Exception:
        pass

    # glin ML transformer (detoxify)
    glin_ml_trans = None
    try:
        from glin_profanity.ml import HybridFilter

        glin_ml_trans = HybridFilter({
            "languages": ["english", "russian"],
            "enable_ml": True,
            "ml_type": "transformer",
            "preload_ml": True,
        })
        if not glin_ml_trans.is_ml_ready():
            glin_ml_trans = None
    except Exception:
        pass

    # BadWords ML (XLM-RoBERTa ONNX)
    bw_ml_model = None
    bw_ml_tok = None
    ml_models_dir = Path(__file__).parent.parent / "ml" / "models"
    if (ml_models_dir / "model.onnx").exists():
        try:
            from optimum.onnxruntime import ORTModelForSequenceClassification
            from transformers import AutoTokenizer

            bw_ml_model = ORTModelForSequenceClassification.from_pretrained(str(ml_models_dir))
            bw_ml_tok = AutoTokenizer.from_pretrained(str(ml_models_dir), fix_mistral_regex=True)
        except Exception:
            pass

    # Run benchmarks for each scenario
    def _run_ml_bench(backend: str, fn, scenario: str) -> None:
        try:
            t = bench(fn, iterations=n_ml)
            ml_results[scenario][backend] = t
        except Exception as e:
            ml_results[scenario][backend] = str(e)[:30]

    # Clean
    if glin_ml_light:
        _run_ml_bench("glin_light", lambda: glin_ml_light.check_profanity_hybrid(texts_clean), "Clean text")
    if glin_ml_trans:
        _run_ml_bench("glin_trans", lambda: glin_ml_trans.check_profanity_hybrid(texts_clean), "Clean text")
    if bw_ml_model is not None and bw_ml_tok is not None:
        def _bw_clean():
            inp = bw_ml_tok(texts_clean, return_tensors="pt", truncation=True, max_length=128)
            bw_ml_model(**inp).logits.softmax(dim=-1)[0, 1].item()
        _run_ml_bench("bw", _bw_clean, "Clean text")

    # Bad word
    if glin_ml_light:
        _run_ml_bench("glin_light", lambda: glin_ml_light.check_profanity_hybrid(texts_bad), "Bad word")
    if glin_ml_trans:
        _run_ml_bench("glin_trans", lambda: glin_ml_trans.check_profanity_hybrid(texts_bad), "Bad word")
    if bw_ml_model is not None and bw_ml_tok is not None:
        def _bw_bad():
            inp = bw_ml_tok(texts_bad, return_tensors="pt", truncation=True, max_length=128)
            bw_ml_model(**inp).logits.softmax(dim=-1)[0, 1].item()
        _run_ml_bench("bw", _bw_bad, "Bad word")

    # Batch (5 texts)
    if glin_ml_light:
        def _glin_light_batch():
            for t in texts_batch:
                glin_ml_light.check_profanity_hybrid(t)
        _run_ml_bench("glin_light", _glin_light_batch, "5 texts batch")
    if glin_ml_trans:
        def _glin_trans_batch():
            for t in texts_batch:
                glin_ml_trans.check_profanity_hybrid(t)
        _run_ml_bench("glin_trans", _glin_trans_batch, "5 texts batch")
    if bw_ml_model is not None and bw_ml_tok is not None:
        def _bw_batch():
            for t in texts_batch:
                inp = bw_ml_tok(t, return_tensors="pt", truncation=True, max_length=128)
                bw_ml_model(**inp).logits.softmax(dim=-1)[0, 1].item()
        _run_ml_bench("bw", _bw_batch, "5 texts batch")

    # Print ML results table
    texts_per_scenario = {"Clean text": 1, "Bad word": 1, "5 texts batch": 5}

    def _fmt(v, texts: int = 1) -> str:
        if isinstance(v, (int, float)):
            return f"{v:>7.0f} µs ({_throughput(v, texts)})"
        if v:
            return f"N/A ({v})"
        return "N/A"

    print(f"{'Scenario':<16} {'BadWords ML':>24} {'glin light':>24} {'glin trans':>24}")
    print("-" * 90)
    for scenario in ml_results:
        r = ml_results[scenario]
        n = texts_per_scenario[scenario]
        bw_s = _fmt(r["bw"], n) if r["bw"] else "N/A"
        gl_s = _fmt(r["glin_light"], n) if r["glin_light"] else "N/A"
        gt_s = _fmt(r["glin_trans"], n) if r["glin_trans"] else "N/A"
        print(f"{scenario:<16} {bw_s:>24} {gl_s:>24} {gt_s:>24}")

    if _prev_hf is not None:
        os.environ["HF_HUB_DISABLE_PROGRESS_BARS"] = _prev_hf
    elif "HF_HUB_DISABLE_PROGRESS_BARS" in os.environ:
        os.environ.pop("HF_HUB_DISABLE_PROGRESS_BARS")

    print("-" * 55)


if __name__ == "__main__":
    main()
