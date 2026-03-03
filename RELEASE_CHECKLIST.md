# Release Checklist

## Versions (2.3.1)

| Package | File | Publication |
|---------|------|-------------|
| badwords-py | pyproject.toml, python/pyproject.toml | PyPI |
| badwords-core | rust/badwords-core/Cargo.toml | crates.io |
| badwords-wasm | rust/badwords-wasm/Cargo.toml, pkg/ | npm |
| @badwords/languages | js/languages/package.json | npm |

## Before release

- [ ] `make test` — all tests pass
- [ ] `make bench` — benchmarks run
- [ ] `make build` — Python wheel builds
- [ ] `make wasm` and `make wasm-nodejs` — WASM builds
- [ ] `make lang-packages` — language packages generated
- [ ] `make ml-package` — ML model (quantized) → badwords-ml-model.zip for GitHub Release

## Publishing

### PyPI (badwords-py)

PyPI rejects `linux_x86_64` wheels. Build in manylinux container:

```bash
docker run --rm -v $(pwd):/io ghcr.io/pyo3/maturin build --release -o dist
twine upload dist/badwords_py-*.whl
```

Or use GitHub Actions: `.github/workflows/release.yml` publishes to PyPI on release.

### crates.io (badwords-core)
```bash
cd rust/badwords-core && cargo publish
```

### npm (badwords-wasm)
```bash
make wasm-nodejs   # or make wasm for browser
make npm-publish
```

### npm (@badwords/languages)
```bash
make lang-packages
make npm-publish-languages
```

## Post-publish verification

- [ ] `pip install badwords-py` — installs
- [ ] `npm install badwords-wasm` — installs
- [ ] `npm install @badwords/languages` — installs
- [ ] Examples work (examples/rust, examples/python, examples/wasm)
