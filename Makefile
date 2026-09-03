PYTHON := $(shell if [ -d .venv ]; then echo .venv/bin/python; else echo python3; fi)
# Same interpreter, reached from inside ml/
PY := $(shell if [ -d .venv ]; then echo ../.venv/bin/python; else echo python3; fi)

.PHONY: sync-resources check-resources sync-version check-version fp-report develop build build-pypi test test-rust test-python test-wasm bench bench-rust bench-python bench-compare quality-compare lint lint-fix format format-fix wasm wasm-nodejs wasm-typecheck npm-publish npm-publish-nodejs lang-packages npm-publish-languages ml-prepare ml-prepare-full ml-train ml-export ml-evaluate ml-test ml-quantize ml-package

# The canonical resources live in the crate; python/badwords/resource is a
# mirror so that maturin ships them inside the wheel.
sync-resources:
	rsync -a --delete rust/badwords-core/resources/ python/badwords/resource/

check-resources:
	@diff -r rust/badwords-core/resources python/badwords/resource \
		&& echo "resources in sync" \
		|| (echo "run: make sync-resources"; exit 1)

develop: sync-resources
	maturin develop
	@SO=$$(find .venv -name "_native*.so" 2>/dev/null | head -1); \
	if [ -n "$$SO" ]; then mkdir -p _native && cp "$$SO" _native/; fi

build: sync-resources
	maturin build

# Build for PyPI (manylinux wheels, requires Docker)
build-pypi:
	docker run --rm --user $$(id -u):$$(id -g) -v $(PWD):/io -w /io \
		ghcr.io/pyo3/maturin build --release -o dist

test: test-rust test-python test-wasm

# Same command CI runs: the substring tests are behind a feature flag, and
# `-p badwords-core` alone leaves badwords-py and badwords-wasm unbuilt.
test-rust:
	cargo test --workspace --all-features

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
	@$(PYTHON) scripts/refine-wasm-types.py

# WebAssembly build for Node.js
wasm-nodejs:
	cd rust/badwords-wasm && wasm-pack build --target nodejs --out-dir pkg-node
	@$(PYTHON) scripts/refine-wasm-types.py

# Typecheck the TypeScript example against the generated declarations
wasm-typecheck: wasm-nodejs
	cd examples/wasm/node && npm install --silent --no-fund --no-audit && npx tsc --noEmit

# Publish the browser build (run `make wasm` first)
npm-publish:
	cd rust/badwords-wasm/pkg-web && npm publish

# Publish the Node.js build (run `make wasm-nodejs` first)
npm-publish-nodejs:
	cd rust/badwords-wasm/pkg-node && npm publish

# Propagate the workspace version to pyproject.toml and the npm packages
sync-version:
	@if [ -d .venv ]; then .venv/bin/python scripts/sync_version.py; \
	else python3 scripts/sync_version.py; fi

check-version:
	@if [ -d .venv ]; then .venv/bin/python scripts/sync_version.py --check; \
	else python3 scripts/sync_version.py --check; fi

# Measure the false-positive cost of each opt-in detector
fp-report:
	cargo run --release -p badwords-core --bin fp_report --features substring

lang-packages: sync-version
	@if [ -d .venv ]; then .venv/bin/python scripts/generate-lang-packages.py; \
	else python3 scripts/generate-lang-packages.py; fi

npm-publish-languages:
	cd js/languages && npm publish --access public

# ML training (requires: pip install -r ml/requirements.txt)
ml-prepare:
	cd ml && $(PY) prepare_data.py

# Bigger pool, longer training
ml-prepare-full:
	cd ml && $(PY) prepare_data.py --max-per-source 800000 --max-total 600000

ml-train:
	cd ml && $(PY) train.py

# Re-export a trained checkpoint without training it again
ml-export:
	cd ml && $(PY) export.py

# Per-axis AUC and best-F1 on the held-out split
ml-evaluate:
	cd ml && $(PY) evaluate.py

# ML tests live with the rest of the suite; they skip without a model
ml-test:
	@if [ -d .venv ]; then .venv/bin/python -m pytest tests/test_ml.py -v; \
	else python3 -m pytest tests/test_ml.py -v; fi

# Quantize model: 500MB -> ~135MB
ml-quantize:
	cd ml && $(PY) quantize_model.py

# Package the model for the GitHub Release (upload as badwords-ml-model.zip).
# Quantize first, with `make ml-quantize` - this target must not do it itself,
# because quantizing an already-quantized model is not a no-op.
# The asset name carries the model generation: code written for the old binary
# model must never be handed a multi-label one. See ASSET_NAME in
# python/badwords/ml/_paths.py.
ML_ASSET := badwords-ml-model-v2.zip

ml-package:
	@if [ ! -f ml/models/model.onnx ]; then echo "no model: run make ml-train"; exit 1; fi
	@python3 -c "import json,sys; c=json.load(open('ml/models/config.json')); \
	  sys.exit(0) if c.get('id2label') and c.get('problem_type') else \
	  (print('ml/models/config.json has no id2label/problem_type; re-run make ml-export'), sys.exit(1))"
	@size=$$(stat -c%s ml/models/model.onnx); \
	  if [ $$size -gt 500000000 ]; then \
	    echo "model.onnx is $$((size/1000000)) MB - that is the fp32 export; run make ml-quantize first"; \
	    exit 1; \
	  fi
	@rm -f $(ML_ASSET)
	(cd ml/models && zip -qr ../../$(ML_ASSET) . -x "checkpoints/*" -x "checkpoints/*/*")
	@echo "Created $(ML_ASSET) ($$(du -h $(ML_ASSET) | cut -f1)) - upload to the GitHub Release"
	@echo "Keep badwords-ml-model.zip (the old binary model) on that release too:"
	@echo "  pre-3.1 clients look for it by name, and a multi-label model read"
	@echo "  by that code reports 0.0004 for obvious profanity."
