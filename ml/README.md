# ML Training Pipeline

Data preparation and model training for `badwords-py[ml]`.

## Setup

```bash
cd ml
pip install -r requirements.txt
```

### CUDA (GPU)

Install PyTorch with CUDA **before** other deps:

```bash
# CUDA 12.4 (or cu121 for older drivers)
pip install torch --index-url https://download.pytorch.org/whl/cu124
```

Check: `python -c "import torch; print(torch.cuda.is_available())"` → `True`

Trainer uses GPU automatically when available.

## Usage

### 1. Prepare data

```bash
# Quick (~30k): EN + RU + 9 languages
python prepare_data.py --preset multilingual

# Full (~600k): SetFit + civil_comments + paradetox
python prepare_data.py --preset multilingual --max-total 600000

# Max (~1M+): all data, no cap
python prepare_data.py --preset multilingual

# English only
python prepare_data.py --preset toxic_conversations --max-samples 200000

# Single dataset
python prepare_data.py --preset single --dataset SetFit/toxic_conversations
```

### 2. Train model

```bash
# Default: xlm-roberta (best quality), then quantize
python train.py --epochs 2 --batch-size 8

# Lighter: distilbert (faster training)
python train.py --model distilbert-base-multilingual-cased --epochs 2 --batch-size 32
```

Output: `models/` (ONNX + tokenizer)

### 3. Quantize (~4x smaller, recommended)

```bash
python quantize_model.py
# xlm-roberta: 500MB -> ~135MB
# distilbert: 250MB -> ~65MB
```

## Datasets

| Preset | Sources | Languages | Size |
|--------|---------|-----------|------|
| `multilingual` | SetFit + paradetox + ru_paradetox + multilingual_paradetox | EN, RU, UK, DE, ES, AR, ZH, HI, AM | 30k+ |
| `toxic_conversations` | SetFit/toxic_conversations | EN | 1.8M |
| `civil_comments` | google/civil_comments | EN | 2M |
| `paradetox` | s-nlp/paradetox | EN | 20k |
| `ru_paradetox` | s-nlp/ru_paradetox | RU | 12k |

## Model

- **Default:** `xlm-roberta-base` (best quality, ~135MB after quantize)
- **Lighter:** `distilbert-base-multilingual-cased` (~65MB after quantize, faster training)
- Task: binary classification (offensive probability)
- Output: ONNX for inference
