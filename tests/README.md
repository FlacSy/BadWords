# Platform tests

## Rust (badwords-core)

```bash
make test-rust
# or
cargo test -p badwords-core
```

Tests in `rust/badwords-core/src/lib.rs` (`#[cfg(test)]` module).

## Python (badwords-py)

Requires installed package: `pip install .` or `make develop`.

```bash
make test-python
# or
pytest tests/ -v
```

Files: `test_profanity_filter.py`, `test_languages.py`, `test_integration.py`, `test_exceptions.py`.

## WASM (badwords-wasm)

```bash
make test-wasm
# or
cd rust/badwords-wasm && wasm-pack test --node
```

Tests in `rust/badwords-wasm/src/lib.rs` (`#[cfg(test)]` module with `#[wasm_bindgen_test]`).

## All platforms

```bash
make test
```
