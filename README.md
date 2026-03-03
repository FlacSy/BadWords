<div align="center">

# 🚫 BadWords

**High-performance profanity filter for Python, Rust, and JavaScript (WebAssembly)  
with multilingual support and evasion detection.**

---

[![Tests](https://github.com/FlacSy/badwords/actions/workflows/tests.yml/badge.svg?style=flat-square)](https://github.com/FlacSy/badwords/actions/workflows/tests.yml)
[![Format](https://github.com/FlacSy/badwords/actions/workflows/format.yml/badge.svg?style=flat-square)](https://github.com/FlacSy/badwords/actions/workflows/format.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-badwords.flacsy.dev-0D9488?style=flat-square)](https://badwords.flacsy.dev)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey?style=flat-square)]()

[![Python](https://img.shields.io/badge/python-3.10%20%7C%203.11%20%7C%203.12%20%7C%203.13-3D7A3D?style=flat-square&logo=python&logoColor=white)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange?style=flat-square)](https://www.rust-lang.org/)
[![JavaScript](https://img.shields.io/badge/JavaScript-ES6+-yellow?style=flat-square&logo=javascript)](https://developer.mozilla.org/en-US/docs/Web/JavaScript)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue?style=flat-square&logo=typescript)](https://www.typescriptlang.org/)

[![PyPI](https://img.shields.io/pypi/v/badwords-py?style=flat-square&color=FFD43B)](https://pypi.org/project/badwords-py/)
[![npm (badwords-wasm)](https://img.shields.io/npm/v/badwords-wasm?style=flat-square&color=CB3837)](https://www.npmjs.com/package/badwords-wasm)
[![npm (@badwords/languages)](https://img.shields.io/npm/v/@badwords/languages?style=flat-square&color=CB3837)](https://www.npmjs.com/package/@badwords/languages)
[![crates.io](https://img.shields.io/crates/v/badwords-core?style=flat-square&color=F74C00)](https://crates.io/crates/badwords-core)
[![badwords-py](https://static.pepy.tech/personalized-badge/badwords-py?period=total&units=international_system&left_color=black&right_color=green&left_text=badwords-py)](https://pepy.tech/projects/badwords-py)
[![bdw (legacy)](https://static.pepy.tech/personalized-badge/bdw?period=total&units=international_system&left_color=black&right_color=gray&left_text=bdw)](https://pepy.tech/projects/bdw)

---

[Installation](#-installation) •
[Quick Start](#-quick-start) •
[Benchmarks](#-benchmarks) •
[Supported Languages](#-supported-languages) •
[Evasion Detection](#-advanced-evasion-detection) •
[Documentation](https://badwords.flacsy.dev)

</div>
---

## 📖 Description

`BadWords` is a sophisticated profanity filtering library designed to clean up user-generated content. Unlike simple keyword matching, it uses **similarity scoring**, **homoglyph detection**, and **transliteration** to catch even the most cleverly disguised insults.

**Architecture:** The core is implemented in Rust for performance. Python provides a thin API layer with full type hints for IDE/linter support. The Rust library can also be used directly from Rust projects.

## 📦 Installation

### Requirements
- **Recommended:** Python 3.13
- **Minimum:** Python 3.10+

### Install via GitHub
```bash
pip install git+[https://github.com/FlacSy/badwords.git](https://github.com/FlacSy/badwords.git)

```

### Install via PyPI
```bash
pip install badwords-py
```

---

## ⚡ Quick Start

### Basic Initialization

```python
from badwords import ProfanityFilter

# Initialize filter
p = ProfanityFilter()

# Load specific languages (e.g., English and Russian)
p.init(languages=["en", "ru"])

# Or load ALL 26+ supported languages
p.init()

```

### Checking and Filtering Text

```python
text = "Some very b4d text here"

# 1. Simple check (Returns Boolean)
is_bad = p.filter_text(text)
print(is_bad) # True

# 2. Censoring text (Returns String)
clean_text = p.filter_text(text, replace_character="*")
print(clean_text) # "Some very *** text here"

```

---

## ⏱ Benchmarks

| CPU | GPU | RAM | OS |
|-----|-----|-----|----|
| x86_64 i7 Intel® Core™ i7-10700KF × 16 | NVIDIA GeForce RTX™ 3070 | 64 GB DDR4 3200MHz | Ubuntu 24.04.2 LTS | 


Rule-based matching (en+ru, `match_threshold=1.0`). Run: `make bench`

| Scenario | Rust (badwords-core) | Python (badwords-py) |
|----------|----------------------|----------------------|
| Clean text (no match) | ~7.6 µs (~130 K/s) | ~7.7 µs (~130 K/s) |
| Bad word (match) | ~3.1 µs (~320 K/s) | ~2.7 µs (~370 K/s) |
| Censor (replace) | ~2.8 µs (~360 K/s) | ~2.5 µs (~400 K/s) |
| 5 texts batch | ~15 µs (~330 K/s) | ~16 µs (~310 K/s) |

*Python uses Rust via PyO3, overhead minimal.*

### vs glin-profanity

Rule-based mode, en+ru. Run: `make bench-compare` (requires `pip install glin-profanity`)

| Scenario | BadWords | glin-profanity |
|----------|----------|----------------|
| Clean text | ~7 µs (~140 K/s) | ~4.4 ms (~230/s) |
| Bad word | ~1.3 µs (~770 K/s) | ~0.2 ms (~5 K/s) |
| Censor | ~1.8 µs (~560 K/s) | ~1.4 ms (~700/s) |
| 5 texts batch | ~16 µs (~310 K/s) | ~10 ms (~500/s) |

*BadWords is ~100–600× faster (Rust core vs pure Python).*

### ML mode

`pip install glin-profanity[ml]` + `make bench-compare`. 100 iter each.

| Scenario | BadWords ML (ONNX) | glin transformer |
|----------|--------------------|-------------------|
| Clean text (43 chars) | ~6.5 ms (~150/s) | ~27 ms (~37/s) |
| Bad word (8 chars) | ~4.6 ms (~220/s) | ~21 ms (~47/s) |
| 5 texts batch (82 chars) | ~24 ms (~210/s) | ~107 ms (~47/s) |

*BadWords ML (XLM-RoBERTa) ~3–4× faster than glin transformer.*

---

## 🛠 Methods & API

### `filter_text(text, match_threshold=1.0, replace_character=None)`

The core method of the library.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `text` | `str` | Required | Input text to check. |
| `match_threshold` | `float` | `1.0` | Similarity threshold (1.0 = exact match, 0.95 = fuzzy). |
| `replace_character` | `str/None` | `None` | If provided, returns censored string. If None, returns bool. |

> [!WARNING]
> **Performance Tip:** Using `match_threshold < 1.0` enables fuzzy matching which is slower. Use `1.0` for high-traffic real-time filtering, or `0.95` for a good balance.

---

## 🧩 Advanced Evasion Detection

Standard filters are easy to bypass. `BadWords` is built to detect:

* **Homoglyphs:** Detects `hеllo` (using Cyrillic 'е') or `h4llo` (numbers).
* **Transliteration:** Automatically handles mapping between Cyrillic and Latin alphabets.
* **Normalization:** Strips diacritics, special characters, and decorative Unicode symbols.
* **Similarity Analysis:** Uses fuzzy matching to find words with deliberate typos.

### Examples of detected evasions:

```python
_filter.filter_text("hеllо")  # Mixed alphabets (Cyrillic + Latin) -> DETECTED
_filter.filter_text("h3ll0")  # Character substitution -> DETECTED
_filter.filter_text("h⍺llo")  # Mathematical/Greek symbols -> DETECTED
_filter.filter_text("привет") # Transliterated matches -> DETECTED

```

---

## 🌍 Supported Languages

`BadWords` supports **25 languages** out of the box:

| Code | Language | Code | Language | Code | Language |
|------|----------|------|----------|------|----------|
| `en` | English | `ru` | Russian | `ua` | Ukrainian |
| `de` | German | `fr` | French | `it` | Italian |
| `sp` | Spanish | `pl` | Polish | `cz` | Czech |
| `ja` | Japanese | `ko` | Korean | `th` | Thai |
| `br` | Portuguese (BR) | `da` | Danish | `du` | Dutch |
| `fi` | Finnish | `gr` | Greek | `hu` | Hungarian |
| `in` | Indonesian | `lt` | Lithuanian | `no` | Norwegian |
| `po` | Portuguese | `ro` | Romanian | `sw` | Swedish |
| `tu` | Turkish | | | | |

*Use `p.get_all_languages()` in code. Full list with word counts: [badwords.flacsy.dev](https://badwords.flacsy.dev/reference/languages/)*

---

## 🚀 Full Integration Example

```python
from badwords import ProfanityFilter

def monitor_chat():
    # Setup for a global chat
    profanity_filter = ProfanityFilter()
    profanity_filter.init(["en", "ru", "de"])
    
    # Custom project-specific banned words
    profanity_filter.add_words(["spam_link_v1", "scam_bot_99"])

    user_input = "Hey! Check out this b.a.d.w.o.r.d"
    
    # Moderate with high accuracy
    is_offensive = profanity_filter.filter_text(user_input, match_threshold=0.95)
    
    if is_offensive:
        print("Message blocked: Contains restricted language.")
    else:
        # Proceed with processing
        pass

if __name__ == "__main__":
    monitor_chat()

```

---

## 🦀 Rust API (badwords-core)

Published on [crates.io](https://crates.io/crates/badwords-core):

```toml
[dependencies]
badwords-core = "2"
```

```rust
use badwords_core::{ProfanityFilter, default_resource_dir};

let resource_dir = default_resource_dir();
let mut filter = ProfanityFilter::new(&resource_dir, true, true, true, true);
filter.init(None).unwrap();
filter.add_words(&["custom".to_string()]);
let (found, _) = filter.filter_text("hello", 1.0, None);
```

## 🌐 WebAssembly (JavaScript/TypeScript)

Same Rust code for browser and Node.js, compiled to WASM.

### Build

```bash
# Browser
make wasm

# Node.js
make wasm-nodejs
```

### Frontend (browser)

```html
<script type="module">
  import init, { ProfanityFilter } from './path/to/badwords_wasm.js';
  await init();
  const filter = new ProfanityFilter();
  console.log(filter.isBad('text'));      // boolean
  console.log(filter.censor('text', '*')); // string
</script>
```

### Backend (Node.js)

```javascript
const { ProfanityFilter } = require('badwords-wasm');
const filter = new ProfanityFilter();
filter.isBad('hello');           // false
filter.censor('bad word', '*');  // "*** word"
filter.addWords(['custom']);
```

### Optional languages (npm)

Built-in: en and ru. Additional languages via `@badwords/languages`:

```bash
npm install badwords-wasm @badwords/languages
```

```javascript
import init, { ProfanityFilter } from 'badwords-wasm';
import de from '@badwords/languages/de';
import ua from '@badwords/languages/ua';

await init();
const filter = new ProfanityFilter();
filter.addWords(de);
filter.addWords(ua);
```

Available: br, cz, da, de, du, en, fi, fr, gr, hu, in, it, ja, ko, lt, no, pl, po, ro, ru, sp, sw, th, tu, ua. See [@badwords/languages](https://www.npmjs.com/package/@badwords/languages).

Examples: `examples/wasm/browser/`, `examples/wasm/node/`

## 🔧 Building from source

Requires: Rust, Python, maturin

```bash
python -m venv .venv && source .venv/bin/activate  # Linux/macOS
pip install maturin
make develop
# or: cd python && maturin build && pip install target/wheels/badwords_py-*.whl
```

## 🌐 WebAssembly (browser & Node.js)

Build the WASM package (requires [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)):

```bash
cargo install wasm-pack
make wasm
```

Output: `rust/badwords-wasm/pkg/` (npm package `badwords-wasm`)

- **Browser:** Use the generated JS with a bundler or static server. See `examples/wasm/browser/`
- **Node.js:** `import init, { ProfanityFilter } from 'badwords-wasm'` after `npm install`. See `examples/wasm/node/`
- **Publish to npm:** `make wasm` or `make wasm-nodejs`, then `make npm-publish`
- **Optional languages:** `@badwords/languages` — `make lang-packages` then `make npm-publish-languages`

## 📚 Documentation

Full documentation (Python, Rust, JavaScript) with examples and API reference: **[badwords.flacsy.dev](https://badwords.flacsy.dev)** (EN / RU).

## 🤝 Contributing

Contributions are what make the open-source community an amazing place to learn, inspire, and create.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

<div align="center">
<sub>Developed with ❤️ by <a href="https://github.com/FlacSy">FlacSy</a></sub>
</div>
