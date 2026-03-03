.PHONY: develop build test test-rust test-python test-wasm bench bench-rust bench-python bench-compare wasm wasm-nodejs npm-publish lang-packages npm-publish-languages

develop:
	cd python && maturin develop
	@SO=$$(find .venv -name "_native*.so" 2>/dev/null | head -1); \
	if [ -n "$$SO" ]; then mkdir -p _native && cp "$$SO" _native/; fi

build:
	cd python && maturin build

# Build for PyPI (manylinux wheels, requires Docker)
build-pypi:
	docker run --rm -v $(PWD):/io -w /io ghcr.io/pyo3/maturin build --release -o dist

test: test-rust test-python test-wasm

test-rust:
	cargo test -p badwords-core

test-python:
	@if [ -d .venv ]; then .venv/bin/python -m pytest tests/ -v; \
	else python3 -m pytest tests/ -v; fi

test-wasm:
	cd rust/badwords-wasm && wasm-pack test --node

bench: bench-rust bench-python

bench-compare:
	@echo "BadWords vs glin-profanity (requires: pip install glin-profanity)"
	@if [ -d .venv ]; then .venv/bin/python scripts/bench_compare.py; \
	else python3 scripts/bench_compare.py; fi

bench-rust:
	cargo bench -p badwords-core

bench-python:
	@if [ -d .venv ]; then .venv/bin/python -m pytest tests/bench_filter.py -v --benchmark-only; \
	else python3 -m pytest tests/bench_filter.py -v --benchmark-only; fi

# WebAssembly build for browser
wasm:
	cd rust/badwords-wasm && wasm-pack build --target web --out-dir pkg

# WebAssembly build for Node.js
wasm-nodejs:
	cd rust/badwords-wasm && wasm-pack build --target nodejs --out-dir pkg

npm-publish:
	cd rust/badwords-wasm/pkg && npm publish

lang-packages:
	python3 scripts/generate-lang-packages.py

npm-publish-languages:
	cd js/languages && npm publish --access public

# ML training (requires: pip install -r ml/requirements.txt)
ml-prepare:
	cd ml && python prepare_data.py --preset multilingual

# Full dataset (~600k samples, ~8-10h training with xlm-roberta)
ml-prepare-full:
	cd ml && python prepare_data.py --preset multilingual --max-total 600000

# Max dataset (no cap, ~1M+ samples, ~15-20h)
ml-prepare-max:
	cd ml && python prepare_data.py --preset multilingual

ml-train:
	cd ml && python train.py

ml-test:
	cd ml && python test_inference.py

# Quantize model: 500MB -> ~135MB
ml-quantize:
	cd ml && python quantize_model.py

# Package ML model for GitHub Release (upload as badwords-ml-model.zip)
# Quantizes model first (~4x smaller)
ml-package: ml-quantize
	@if [ ! -f ml/models/model.onnx ]; then echo "Run ml-train first"; exit 1; fi
	(cd ml/models && zip -r ../../badwords-ml-model.zip . -x "checkpoints/*" -x "checkpoints/*/*")
	@echo "Created badwords-ml-model.zip — upload to GitHub Release"
