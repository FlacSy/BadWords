# Contributing

Thanks for taking the time. This document covers the things that are specific
to this project; the rest is ordinary GitHub work.

## Layout

```
rust/badwords-core     the engine, and the canonical word lists
rust/badwords-py       PyO3 bindings
rust/badwords-wasm     WebAssembly bindings
python/badwords        the Python API (resource/ is a generated mirror)
js/languages           the @badwords/languages npm package (generated)
ml/                    the training pipeline for the optional ML model
```

## Getting set up

```bash
python -m venv .venv && source .venv/bin/activate
pip install maturin pytest pytest-benchmark ruff
make develop
make test
```

WebAssembly work also needs `cargo install wasm-pack` and Node.js 20+.

## Before opening a pull request

```bash
make test          # Rust, Python and WebAssembly
make lint format   # ruff
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
```

## Word lists

The canonical lists are in `rust/badwords-core/resources/words`. Everything
else is generated from them:

```bash
make sync-resources   # -> python/badwords/resource, shipped in the wheel
make lang-packages    # -> js/languages
```

CI fails if either is stale. Entries are matched **literally** after
normalization - a line with regex metacharacters is a literal that can never
match, which is exactly the bug 3.0.0 fixed. See
`rust/badwords-core/resources/words/SOURCES.md`.

Prefer entries that come from a citable list, and be careful with anything
under five characters: short entries are what make substring matching
expensive.

## Changing detection

Two rules:

1. **A new detector defaults to off.** Every one of them trades false negatives
   for false positives, and that trade is the caller's to make.
2. **Measure the trade.** `make fp-report` counts how many of 73,000 ordinary
   English words each option set flags. `tests/false_positives.rs` holds the
   budgets; if your change moves them, say by how much in the pull request.

`tests/compat_golden.rs` replays 8736 recorded responses from 2.3.1 through the
deprecated API. If it fails, the deprecated path changed behaviour, which is
almost always a bug rather than an improvement.

The property tests in `tests/properties.rs` encode the invariants that are easy
to break: spans land on character boundaries, `is_profane` agrees with `find`,
censoring rewrites only what matched, and enabling any flag only ever *adds*
matches.

## Commits

Conventional Commits (`fix(core): ...`, `feat(python)!: ...`). Commit messages
carry the change and nothing else - no tool attribution, no generated footers.

## Releasing

See `RELEASE_CHECKLIST.md`. The version lives in `[workspace.package]` in the
root `Cargo.toml`; `python scripts/sync_version.py` propagates it everywhere
else and CI checks that it did.
