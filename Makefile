.PHONY: develop build build-pypi test test-rust test-python test-wasm bench bench-rust bench-python bench-compare quality-compare lint lint-fix format format-fix wasm wasm-nodejs npm-publish npm-publish-nodejs lang-packages npm-publish-languages ml-prepare ml-prepare-full ml-train ml-test ml-quantize ml-package

develop:
	cd python && maturin develop
	@SO=$$(find .venv -name "_native*.so" 2>/dev/null | head -1); \
	if [ -n "$$SO" ]; then mkdir -p _native && cp "$$SO" _native/; fi

build:
	cd python && maturin build

# Build for PyPI (manylinux wheels, requires Docker)
build-pypi:
	docker run --rm --user $$(id -u):$$(id -g) -v $(PWD):/io -w /io \
		ghcr.io/pyo3/maturin build --release -o dist

test: test-rust test-python test-wasm

test-rust:
	cargo test -p badwords-core

test-python:
	@if [ -d .venv ]; then .venv/bin/python -m pytest tests/ -v -m "not benchmark"; \
	else python3 -m pytest tests/ -v -m "not benchmark"; fi

test-wasm:
	cd rust/badwords-wasm && wasm-pack test --node

bench: bench-rust bench-python

bench-compare:
	@echo "BadWords vs glin-profanity (requires: pip install glin-profanity)"
	@if [ -d .venv ]; then .venv/bin/python scripts/bench_compare.py; \
	else python3 scripts/bench_compare.py; fi

# Quality comparison: BadWords vs glin-profanity (accuracy, precision, recall, F1)
# Uses 1000+1000 from HuggingFace by default. Add --curated for quick test.
quality-compare:
	@echo "Quality: BadWords vs glin-profanity (requires: pip install glin-profanity datasets)"
	@if [ -d .venv ]; then .venv/bin/python scripts/quality_compare.py; \
	else python3 scripts/quality_compare.py; fi

bench-rust:
	cargo bench -p badwords-core

bench-python:
	@if [ -d .venv ]; then .venv/bin/python -m pytest tests/bench_filter.py -v --benchmark-only; \
	else python3 -m pytest tests/bench_filter.py -v --benchmark-only; fi

# Ruff: lint (check only)
lint:
	@if [ -d .venv ]; then .venv/bin/ruff check .; else ruff check .; fi

# Ruff: format check (CI)
format:
	@if [ -d .venv ]; then .venv/bin/ruff format --check .; else ruff format --check .; fi

# Ruff: format fix (apply formatting)
format-fix:
	@if [ -d .venv ]; then .venv/bin/ruff format .; else ruff format .; fi

# Ruff: lint with auto-fix
lint-fix:
	@if [ -d .venv ]; then .venv/bin/ruff check . --fix; else ruff check . --fix; fi

# WebAssembly build for browser
wasm:
	cd rust/badwords-wasm && wasm-pack build --target web --out-dir pkg-web

# WebAssembly build for Node.js
wasm-nodejs:
	cd rust/badwords-wasm && wasm-pack build --target nodejs --out-dir pkg-node

# Publish the browser build (run `make wasm` first)
npm-publish:
	cd rust/badwords-wasm/pkg-web && npm publish

# Publish the Node.js build (run `make wasm-nodejs` first)
npm-publish-nodejs:
	cd rust/badwords-wasm/pkg-node && npm publish

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
