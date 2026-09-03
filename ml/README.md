# ML Training Pipeline

Data preparation, training, export and measurement for the model behind
`badwords-py[ml]` and the `badwords-ml` crate.

The model scores **seven axes at once** rather than answering yes or no:

| Axis | What it means |
|------|---------------|
| `toxicity` | Overall. This is the axis a single number comes from. |
| `severe_toxicity` | Toxic enough that annotators agreed strongly. Rare. |
| `obscene` | Obscene language. |
| `threat` | A threat of violence. |
| `insult` | Directed at a person. |
| `identity_attack` | Aimed at a group or identity. |
| `sexual_explicit` | Sexually explicit content. |

Targets are the *fraction of annotators* who picked each axis, not a
thresholded bit, and axes a source does not annotate are masked out of the
loss instead of being taught as zero - "not annotated" is not "not toxic".

## Setup

```bash
cd ml
pip install -r requirements.txt
```

### CUDA (GPU)

Install PyTorch with CUDA **before** other deps:

```bash
pip install torch --index-url https://download.pytorch.org/whl/cu124
```

Check: `python -c "import torch; print(torch.cuda.is_available())"` → `True`

## Usage

### 1. Prepare data

```bash
make ml-prepare          # ~220k train / 8k validation / 10k held-out test
make ml-prepare-full     # a larger pool and a longer run
```

Writes `data/processed/{train,validation,test}.csv`. The test split is built
from each source's own held-out split and then filtered against everything the
training pool touched, so a measurement on it means something.

### 2. Train

```bash
make ml-train
# or: python train.py --epochs 2 --batch-size 16 --gradient-accumulation 2
```

Embeddings are frozen by default (`--no-freeze-embeddings` to unfreeze):
XLM-R keeps 192M of its 278M parameters in the embedding matrix and AdamW
holds two fp32 states per parameter, so freezing them is what makes this fit
on an 8 GB card. Gradient checkpointing is on whenever CUDA is.

Training reports per-axis ROC-AUC and the best achievable F1 with the
threshold that reaches it. A fixed 0.5 would say almost nothing: on
`severe_toxicity` barely any row has majority agreement, so its F1 at 0.5 is
0.0 however well the head ranks.

### 3. Export and measure

```bash
make ml-export     # checkpoint -> ONNX; separate so a failed export costs no retraining
make ml-evaluate   # per-axis AUC, average precision and best F1 on the held-out split
```

The export refuses to finish unless `config.json` carries `id2label` and
`problem_type`. The model published before 3.1 carried neither, so every
caller had to assume index 1 meant "toxic".

### 4. Quantize and package

```bash
make ml-quantize   # fp32 ~1.1 GB -> INT8 ~270 MB
make ml-package    # -> badwords-ml-model.zip for the GitHub Release
```

Re-run `make ml-evaluate` after quantizing: INT8 is what users download, so
INT8 is the number worth publishing.

## Data sources

| Source | Axes | Languages |
|--------|------|-----------|
| `google/civil_comments` | all seven | EN |
| `SetFit/toxic_conversations` | toxicity only | EN |
| `AlexSham/Toxic_Russian_Comments` | toxicity only | RU |
| `s-nlp/paradetox` | toxicity only | EN |
| `s-nlp/ru_paradetox` | toxicity only | RU |
| `textdetox/multilingual_paradetox` | toxicity only | EN, RU, UK, DE, ES, AR, ZH, HI, AM |

Keep `--max-per-source` low enough that no single language dominates. A first
run capped the English sources at 400k against ~24k Russian rows, and the
resulting model lost to its own predecessor on Russian text - by 0.9 points of
AUC on an independent Russian corpus - while winning on English. Language
balance is a thing to measure, not to assume: `python evaluate.py` on an
English test set will happily report excellent numbers either way.

Only `civil_comments` annotates the six narrower axes, so those heads are
supervised by English data alone; overall toxicity is supervised by all of it.
A better multilingual fine-grained source is the obvious next improvement.

`--test-fraction` reserves rows from every source that ships no test split of
its own. Without it the test set is English-only, because only the English
sources have one - and the model's Russian quality is then a guess rather than
a measurement. For an independent check, `AlexSham/Toxic_Russian_Comments` is
not part of training at all.

## Model

- Base: `xlm-roberta-base`, seven sigmoid heads
- Sequence length 128, matching inference
- Output: ONNX, INT8-quantized for release
- Consumed by `badwords.ml.ToxicityPredictor` (Python) and
  `badwords_ml::ToxicityModel` (Rust), which both read the axis names from
  `config.json` rather than hardcoding them
