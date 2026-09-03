# badwords-ml

Multi-label toxicity scoring for [badwords](https://github.com/FlacSy/badwords),
on ONNX Runtime.

`badwords-core` answers "is this word in the dictionary". This crate answers
"how toxic is this text, and in what way" - one probability per axis, with the
axis names read from the model's own config rather than hardcoded.

```rust
use badwords_ml::ToxicityModel;

let model = ToxicityModel::open("path/to/model")?;
let scores = model.predict("you are an idiot")?;

scores.toxicity();          // 0.94
scores.get("insult");       // Some(0.93)
scores.strongest();         // ("toxicity", 0.94)
scores.above(0.5);          // [("toxicity", 0.94), ("insult", 0.93)]
# Ok::<(), badwords_ml::Error>(())
```

The shipped model scores seven axes: `toxicity`, `severe_toxicity`,
`obscene`, `threat`, `insult`, `identity_attack` and `sexual_explicit`.

## Rules first

A model call costs milliseconds; a dictionary lookup costs microseconds. When
the rules find an entry outright they are also right, so that verdict needs no
second opinion:

```rust
use badwords_core::ProfanityFilter;
use badwords_ml::{HybridFilter, ToxicityModel};

let filter = ProfanityFilter::builder().embedded().languages(["en", "ru"]).build()?;
let hybrid = HybridFilter::new(filter, ToxicityModel::open("path/to/model")?);

let result = hybrid.check("you are a worthless waste of oxygen")?;
result.is_profane;   // true
result.decided_by;   // Decision::Model - no dictionary entry to go on
result.scores;       // Some(Scores { .. })
# Ok::<(), Box<dyn std::error::Error>>(())
```

Everything the rules do *not* settle goes to the model, text they found nothing
in included. Treating "the rules saw nothing" as "clean" is exactly what makes
a hybrid score worse than the model it wraps: on held-out English rows the
dictionary alone reaches 27% recall (in Russian it reaches 50%, because that
language's profanity is lexical - the split is worth measuring per language).

## Getting the model

The crate loads a directory; it does not download one. Either point
`BADWORDS_ML_PATH` at a model directory, or let the Python package populate
the shared cache (`badwords.ml.download_model()`), which
[`locate_model`](https://docs.rs/badwords-ml) then finds:

```rust
let model = badwords_ml::ToxicityModel::open_located()?;
# Ok::<(), badwords_ml::Error>(())
```

A model directory holds `model.onnx`, `config.json`, `tokenizer.json` and
`tokenizer_config.json` - what `ml/train.py` and `ml/export.py` produce.

## Requirements

Rust 1.88 or newer, which is what `ort` requires; `badwords-core` itself still
builds on 1.78. ONNX Runtime binaries are fetched by `ort` at build time.

## License

MIT.
