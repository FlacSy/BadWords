# ML design — `badwords-py[ml]`

Optional machine-learning layer, installed with `pip install 'badwords-py[ml]'`.

## Why a hybrid

The rules answer in microseconds; the model takes milliseconds. On most text
the rules are also certain: an exact dictionary hit needs no second opinion,
and text with nothing resembling an entry needs none either. The model is worth
calling for the band in between - a fuzzy match good enough to be suspicious
but not good enough to act on.

```
text -> rules -> best fuzzy score
                       |
        score >= call_max            -> profane, model not called
        score <  call_min            -> clean,   model not called
        call_min <= score < call_max -> model(text) -> prob >= decision_threshold
```

## API

```python
from badwords.ml import HybridFilter

f = HybridFilter(
    languages=["en", "ru"],
    call_range=(0.90, 0.99),    # rule scores in this band go to the model
    decision_threshold=0.5,     # model probability at or above this is profane
)

f.is_profane("you are a dikhead")   # True
result = f.check("you are a dikhead")
result.decided_by                   # "model"
result.rule_score                   # 0.97
result.ml_score                     # 0.98
result.matches                      # what the rules found

f.check_many(texts)                 # model calls batched into one pass
```

`f.filter` is the underlying `ProfanityFilter`, for adding words or a
whitelist; `f.predictor` is the `ToxicityPredictor`, whose `load()` warms the
model up so the first real call is not slow.

### Choosing `call_range`

`(0.90, 0.99)` is the cheap default: the model only sees near-misses, so the
hybrid costs almost nothing but cannot catch toxicity the rules had no hint of.

`(0.0, 0.99)` escalates everything the rules did not decide outright. Recall
goes up, and so does the number of model calls - on a sample of four messages,
one call becomes three.

## The model alone

```python
from badwords.ml import ToxicityPredictor

predictor = ToxicityPredictor()          # loads nothing yet
predictor.predict("some text")           # 0.0 - 1.0
predictor.predict_batch(["a", "b"])      # one pass
predictor.is_toxic("some text")          # bool
```

Construction never touches the disk or the network; the model is loaded on the
first prediction, or explicitly with `load()`.

`predict_batch` pads to the longest text in the batch. The shipped model is
INT8-quantized and not perfectly invariant to padding, so a batched score can
differ from `predict` on the same text by a few hundredths. Use `predict` when
a text sits near the threshold.

## Runtime

`onnxruntime` is driven directly rather than through `optimum`, which depends
on torch unconditionally - several hundred megabytes for an inference path that
does not use it. Checked against the previous torch path on the same inputs:
the probabilities agree to 3e-11.

The tokenizer is loaded without `fix_mistral_regex`, matching how the model was
trained. The 2.x inference path passed that flag while training did not, so
inference tokenized differently from training; on sample inputs the flag moves
probabilities by up to 0.096.

```toml
[project.optional-dependencies]
ml = ["onnxruntime>=1.16", "transformers>=4.36", "numpy>=1.24"]
```

## Labels

Index 1 is the toxic class throughout the pipeline. Models exported from 3.0.0
onwards record `id2label` in `config.json`, and inference reads it; older
exports carry no labels, and index 1 is assumed.

## Model storage

Resolution order:

1. `BADWORDS_ML_PATH` - a directory holding the model
2. `ml/models/` when running from a source checkout
3. `~/.cache/badwords/ml/model` (or `$XDG_CACHE_HOME`)
4. the `badwords-ml-model.zip` asset of a GitHub release, downloaded into the cache

A directory counts as a model only when it holds `model.onnx`, `config.json`,
`tokenizer.json` and `tokenizer_config.json`. The download streams to a
temporary directory, verifies those files are present and only then moves it
into place, so an interrupted download cannot leave a broken model cached.
Archive entries that would write outside the target are rejected.

```python
from badwords.ml import download_model

download_model()                  # into the cache, skipped if already complete
download_model(force=True)        # re-download
download_model(tag="v3.0.0")      # pin a release instead of tracking latest
```

## Publishing a model

```bash
make ml-prepare     # build the training set
make ml-train       # fine-tune and export to ONNX
make ml-package     # quantize, then zip into badwords-ml-model.zip
```

Upload `badwords-ml-model.zip` to a GitHub release. The artifact is about
206 MB compressed and 266 MB on disk.
