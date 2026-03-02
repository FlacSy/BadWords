# BadWords Examples

Structure: `examples/rust/`, `examples/python/`, `examples/wasm/browser/`, `examples/wasm/node/`

## Rust

Requires: `cargo` (Rust), repo clone (for resources)

```bash
# Basic usage
cargo run --example rust_basic

# Chat moderation
cargo run --example rust_chat_moderation

# Specific languages
cargo run --example rust_specific_languages
```

When using `badwords-core` from crates.io: `use badwords_core::{default_resource_dir, ProfanityFilter}`.

## Python

Requires: `badwords-py` installed (`pip install -e .` or `pip install badwords-py`)

```bash
# Basic usage
python examples/python/basic.py

# Chat moderation
python examples/python/chat_moderation.py

# All languages
python examples/python/all_languages.py
```

Or from project root with package installed:

```bash
cd /path/to/BadWords
pip install -e .
python examples/python/basic.py
```
