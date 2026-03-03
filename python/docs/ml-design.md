# ML Design — badwords-py[ml]

Optional ML enhancement for `badwords-py`, activated via `pip install badwords-py[ml]`.

## Overview

- **Scope:** Python only
- **Integration:** ML works at the decision boundary — when rule-based filter is uncertain
- **Output:** ML returns `float` (probability 0.0–1.0)
- **Config:** Thresholds and behavior configurable via config

## Architecture

```
Text → Rules (fuzzy match) → score
        ↓
   score ∈ [call_min, call_max]?  (decision boundary)
        ↓ yes
   ML(text) → float(prob)
        ↓
   prob >= decision_threshold → bad : clean
```

## Config Structure

```python
ml_config = {
    "enabled": True,
    "call_threshold_min": 0.90,   # Call ML when fuzzy score >= 0.90
    "call_threshold_max": 0.99,   # And < 0.99 (uncertainty zone)
    "decision_threshold": 0.5,    # ML: prob >= 0.5 → bad
}
```

Example usage:

```python
p.init(
    languages=["en", "ru"],
    ml_config={
        "call_min": 0.90,
        "call_max": 0.99,
        "decision": 0.5,
    }
)
```

## API

```python
# ML returns probability
ml_score: float = ml_model.predict(text)  # 0.0 .. 1.0
is_bad = ml_score >= config["decision_threshold"]
```

## pyproject.toml

```toml
[project.optional-dependencies]
dev = ["pytest>=7.0", "pytest-benchmark>=4.0"]
ml = ["onnxruntime>=1.16"]  # + model (TBD)
```

## Model Storage (GitHub Releases)

Hybrid approach:

1. **BADWORDS_ML_PATH** — env var with path to model dir (model.onnx, config, tokenizer)
2. **Local dev** — `ml/models/` in repo (when running from source)
3. **Cache** — `~/.cache/badwords/ml/` (or `$XDG_CACHE_HOME/badwords/ml`)
4. **Download** — from GitHub Releases asset `badwords-ml-model.zip` → extract to cache

### Publishing model

```bash
make ml-train ml-quantize   # train and quantize
make ml-package             # creates dist/badwords-ml-model.zip
```

Upload `dist/badwords-ml-model.zip` to a GitHub Release as an asset.
