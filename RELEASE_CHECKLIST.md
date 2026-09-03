# Release Checklist

The release is automated: `.github/workflows/release.yml` builds every wheel,
publishes to PyPI, crates.io and npm, and refuses to run when the tag and the
workspace version disagree. This file is what a human still has to do.

## Versions

`Cargo.toml` (`[workspace.package] version`) is the single source of truth.
`make sync-version` copies it everywhere else, and CI fails if anything drifts.

| Package | Published to | Version comes from |
|---------|--------------|--------------------|
| badwords-py | PyPI | pyproject.toml |
| badwords-core | crates.io | rust/badwords-core/Cargo.toml |
| badwords-wasm | crates.io, npm | rust/badwords-wasm/Cargo.toml |
| badwords-ml | crates.io | rust/badwords-ml/Cargo.toml |
| @badwords/languages | npm | js/languages/package.json |

## Before the tag

- [ ] `make sync-version` — propagate the new version, then commit
- [ ] `make test` — Rust, Python and WASM
- [ ] `make check-resources` — the wheel's copy of the word lists is in sync
- [ ] `make lang-packages` — regenerate the npm language packs; commit any diff
- [ ] `make fp-report` — false-positive budgets did not move unexpectedly
- [ ] `make bench` — no performance surprise
- [ ] README benchmark numbers still reflect reality
- [ ] `make ml-evaluate` — only when the model changed: per-axis AUC and F1 on
      the held-out split, measured on the **quantized** model, since that is
      what users download. Record the numbers in the README.
- [ ] `make ml-package` — only when the model changed. It refuses to package a
      model whose `config.json` lacks `id2label`/`problem_type`, or one that is
      still the fp32 export.

### Publishing a model

Two assets go on the release, and both matter:

| Asset | For | Notes |
|---|---|---|
| `badwords-ml-model-v2.zip` | 3.1+ | The multi-label model `make ml-package` builds |
| `badwords-ml-model.zip` | 2.x and 3.0 | The old binary model, carried forward unchanged |

Older clients look for `badwords-ml-model.zip` by name and would otherwise
find nothing. What they must *never* find under that name is a multi-label
model: their inference path reads output 1 through a softmax, which on the new
model is `severe_toxicity`. Measured, on the same text: 0.0004 where the answer
is 0.998. They would pass every insult through and report nothing wrong.

So: **a model whose outputs change shape or meaning gets a new asset name**,
`ASSET_NAME` in `python/badwords/ml/_paths.py` is bumped to match, and
`MODEL_RELEASE_TAG` there points at the release carrying it. Check that tag
before publishing — clients fetch from it rather than from "latest", so that a
later release cannot hand them a model they cannot read.

CI checks the first four on every pull request, so a green branch means they
already pass; run them locally when releasing from an unmerged state.

## Releasing

```bash
git tag v3.0.0 && git push origin v3.0.0
```

Then publish a GitHub Release for that tag. `release.yml` runs on publish:

1. **guard** — versions in sync and the tag matches the workspace version
2. **wheels** — manylinux x86_64/aarch64, musllinux, macOS x86_64/arm64,
   Windows x64, for CPython 3.10 through 3.13
3. **sdist**
4. **pypi** — trusted publishing from the `pypi` environment
5. **crates** — `badwords-core` first, then `badwords-wasm` and `badwords-ml`
   after the index catches up, because both resolve the core from crates.io
6. **npm** — `badwords-wasm` (browser build) and `@badwords/languages`

`workflow_dispatch` runs the same thing with `dry-run` defaulted to true:
everything is built, nothing is published. Use it before a real release.

## Manual publishing

Only when the workflow cannot run. Each step is what the corresponding job does.

```bash
# PyPI - PyPI rejects plain linux_x86_64 wheels, so build in manylinux
make build-pypi                       # docker, wheels land in dist/
twine upload dist/badwords_py-*.whl

# crates.io - core first, wasm after the index updates
cargo publish -p badwords-core
cargo publish -p badwords-wasm
cargo publish -p badwords-ml

# npm
make wasm && make npm-publish
make lang-packages && make npm-publish-languages
```

## After publishing

- [ ] `pip install badwords-py` in a clean venv — imports and matches
- [ ] `cargo add badwords-core` in a scratch crate — builds without the repo,
      which is what the embedded resources are for
- [ ] `npm install badwords-wasm @badwords/languages` — the Node example runs
- [ ] Examples still work: `examples/rust`, `examples/python`, `examples/wasm`
