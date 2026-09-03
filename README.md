<div align="center">

# 🚫 BadWords

**High-performance profanity filter for Python, Rust, and JavaScript (WebAssembly)  
with multilingual support and evasion detection.**

---

[![Tests](https://github.com/FlacSy/badwords/actions/workflows/tests.yml/badge.svg?style=flat-square)](https://github.com/FlacSy/badwords/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-badwords.flacsy.dev-0D9488?style=flat-square)](https://badwords.flacsy.dev)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey?style=flat-square)]()

[![Python](https://img.shields.io/badge/python-3.10%20%7C%203.11%20%7C%203.12%20%7C%203.13-3D7A3D?style=flat-square&logo=python&logoColor=white)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/rust-1.78+-orange?style=flat-square)](https://www.rust-lang.org/)
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
[Options](#-options) •
[Benchmarks](#-benchmarks) •
[ML scoring](#-ml-scoring) •
[Languages](#-supported-languages) •
[Evasion Detection](#-evasion-detection) •
[Migration](#-migrating-from-2x) •
[Documentation](https://badwords.flacsy.dev)

</div>

---

## 📖 Description

`BadWords` cleans up user-generated content. Unlike plain keyword matching it normalizes text first — NFKC, case folding, confusable characters, homoglyphs and cyrillic/latin transliteration — and then looks the result up in a dictionary of 20330 entries across 25 languages (16981 of them distinct), with optional fuzzy, phrase and substring matching on top.

**Architecture:** the engine is Rust. Python gets it through PyO3 with the GIL released around every call, JavaScript through WebAssembly, and Rust projects can depend on the core directly. All three share one set of word lists and one language registry.

## 📦 Installation

### Requirements
- **Python:** 3.10+
- **Rust (if you use the crate):** 1.78+

### PyPI
```bash
pip install badwords-py
```

### From GitHub
```bash
pip install git+https://github.com/FlacSy/badwords.git
```

### Cargo / npm
```bash
cargo add badwords-core
npm install badwords-wasm
```

---

## ⚡ Quick Start

```python
from badwords import ProfanityFilter

p = ProfanityFilter()
p.init(languages=["en", "ru"])   # or p.init() for all 25

p.is_profane("hello world")      # False
p.is_profane("sonofabitch")      # True

# Censoring keeps everything that is not part of a match,
# including the punctuation attached to the word.
p.censor("hey shit, ok")         # "hey ****, ok"

# find() says what matched, where, and in which language.
for m in p.find("what a shitty, damn mess"):
    print(m.matched_text, m.start, m.end, m.word, m.language, m.kind)
```

### Your own words and a whitelist

```python
p.add_words(["spam_link_v1", "scam_bot_99"])
p.add_whitelist(["assessment"])   # never reported, whatever the rules say

p.word_count()        # entries in the dictionary
p.contains_word("x")  # after normalization
```

### Many texts at once

One call into Rust instead of many, with the GIL released for the whole batch:

```python
p.is_profane_many(["hello", "sonofabitch", "fine"])   # [False, True, False]
p.find_many(texts)                                    # list[list[Match]]
p.censor_many(texts, "*")                             # list[str]
```

---

## 🎛 Options

Every call takes an `Options`; without one the filter's default is used. Each field is off by default, because each detector trades false negatives for false positives.

```python
from badwords import Options, ProfanityFilter

p = ProfanityFilter()
p.init(["en"], options=Options(collapse_repeats=True))  # default for this filter

p.is_profane("text", Options(match_threshold=0.9))      # just this call
```

| Field | Type | Default | What it does |
|---|---|---|---|
| `match_threshold` | `float` | `1.0` | Similarity a fuzzy match needs. `1.0` is exact only. |
| `match_mode` | `"token"` / `"substring"` | `"token"` | Substring also matches entries inside a longer word. |
| `split_on_punctuation` | `bool` | `False` | Also test the pieces a token splits into: `you.shit`. |
| `collapse_repeats` | `bool` | `False` | Also test with repeated letters collapsed: `shiiit`, `ffuck`. |
| `leetspeak` | `bool` | `False` | Read digits as letters: `sh1t`. |
| `phrases` | `bool` | `True` | Match multi-word entries across consecutive words. |
| `min_substring_len` | `int` | `6` | In substring mode, ignore entries shorter than this. |
| `max_matches` | `int / None` | `None` | Stop after this many matches. |

`Options.aggressive()` turns on every detector at threshold 0.9 in substring mode. It is a starting point for measurement, not a recommended default — see the false-positive table below.

Each match is a frozen dataclass:

| Field | Description |
|---|---|
| `word` | The dictionary entry that matched, as written in the word list. |
| `matched_text` | The matched slice, always equal to `text[start:end]`. |
| `start`, `end` | Byte offsets into the original text. |
| `language` | Language the entry came from, or `None` for words you added. |
| `score` | Similarity; `1.0` for anything but a fuzzy match. |
| `kind` | `exact`, `fuzzy`, `leet`, `collapsed`, `substring` or `phrase`. |

---

## 🧩 Evasion Detection

Normalization is always on: `hеllo` with a Cyrillic `е`, decorative Unicode, diacritics and mixed scripts all fold to the same form before lookup. The rest is opt-in:

```python
from badwords import Options, ProfanityFilter

p = ProfanityFilter()
p.init(["en"])

strict = Options(split_on_punctuation=True, collapse_repeats=True, leetspeak=True)

p.is_profane("shiiit")             # False
p.is_profane("shiiit", strict)     # True — collapse_repeats
p.is_profane("you.shit", strict)   # True — split_on_punctuation
p.is_profane("wh0re", strict)      # True — leetspeak
```

Some leet spellings are in the word lists as literal entries — `sh1t` and `a55`
ship in `en` — so they match without `leetspeak` at all. The flag is for the
spellings nobody wrote down.

### What each detector costs

Measured against 73302 clean words from `/usr/share/dict/american-english` with all 25 languages loaded. `cargo run --release -p badwords-core --bin fp_report --features substring` reproduces the table, and `tests/false_positives.rs` fails the build if the budgets regress.

| Mode | False positives | Rate |
|---|---|---|
| default (exact) | 0 | 0.000% |
| `split_on_punctuation` | 0 | 0.000% |
| `collapse_repeats` | 0 | 0.000% |
| `leetspeak` | 0 | 0.000% |
| `match_threshold=0.95` | 591 | 0.806% |
| `match_threshold=0.90` | 5521 | 7.532% |
| substring, `min_substring_len=6` | 274 | 0.374% |
| substring, `min_substring_len=7` | 78 | 0.106% |
| `Options.aggressive()` | 5658 | 7.719% |

The three character-level detectors are free of false positives on that corpus; fuzzy matching and substring mode are not. Substring is more expensive here than it will be for most users, because a short entry in one language occurs inside ordinary words of another — with English alone the rate at the default length is 0.150%.

---

## ⏱ Benchmarks

| CPU | RAM | OS |
|-----|-----|----|
| Intel® Core™ i7-10700KF @ 3.80GHz (8C/16T) | 32 GB DDR4 3200MHz | Ubuntu 24.04.2 LTS |

en + ru loaded, default options (exact matching), release builds.

**Rust** — criterion, `make bench-rust`:

| Benchmark | Median |
|---|---|
| `is_profane`, clean text | 9.6 µs |
| `is_profane`, text with a match | 1.6 µs |
| `censor` | 3.1 µs |
| `is_profane_many`, 5 texts | 12.6 µs |
| `find_into`, caller-owned scratch | 9.7 µs |
| `find`, substring mode | 5.3 µs |
| `find`, `match_threshold=0.9` | 184 µs |

**Python** — pytest-benchmark medians, `make bench-python`:

| Benchmark | Median |
|---|---|
| `is_profane`, clean text | 9.0 µs |
| `is_profane`, text with a match | 4.0 µs |
| `censor` | 4.2 µs |
| `find`, clean text | 9.1 µs |
| `is_profane_many`, 5 texts | 15.5 µs |
| the same 5 texts, one call each | 21.2 µs |
| `is_profane`, `match_threshold=0.9` | 18.2 µs |

Two things the numbers show. A match is *cheaper* than clean text, because
matching stops at the first hit while clean text is normalized to the end. And
the batch API is worth using: five texts in one call cost 15.5 µs against
21.2 µs one at a time, since the GIL is released once instead of five times.

### vs glin-profanity

Rule-based mode, en+ru, same texts through each library's own API.
Run: `make bench-compare` (needs `pip install glin-profanity`). BadWords is
timed over 50,000 iterations, glin-profanity over 1,000.

| Scenario | BadWords | glin-profanity 3.3.0 |
|----------|----------|----------------------|
| Clean text (43 chars) | 10.3 µs | 4316 µs |
| Bad word (8 chars) | 3.0 µs | 886 µs |
| Censor | 3.1 µs | 1401 µs |
| 5 texts | 17.8 µs (one batch call) | 9798 µs (five calls) |

Roughly 300–550×, which is what a Rust core buys against a pure-Python one.

### ML mode

The optional model is XLM-RoBERTa, INT8-quantized, run through ONNX Runtime —
no torch on the inference path. 100 iterations each, same machine.

| Scenario | BadWords ML | glin-profanity (transformer) |
|----------|-------------|------------------------------|
| Clean text (43 chars) | 6.8 ms | 27.9 ms |
| Bad word (8 chars) | 5.1 ms | 26.4 ms |
| 5 texts | 12.2 ms (2.4 ms/text, batched) | 130.0 ms |

Cost scales with length, not just count: these are short texts, and a 400-character
comment filling the 128-token window costs several times more — the held-out
evaluation, on comments up to 400 characters, averages about 48 ms per text.
Batch what you can.

Model timings are indicative. Repeated runs on this machine varied by up to 2x
with whatever else it was doing, while the *ratio* to glin's transformer held
at roughly 4x across every run. Rule-based numbers are far steadier, within
about 15%.

A model call is three orders of magnitude more expensive than a rule call —
see [ML scoring](#-ml-scoring) for what to do about that.

---

## 🤖 ML scoring

```bash
pip install "badwords-py[ml]"     # onnxruntime + transformers, no torch
```

The rules answer "is this word in the dictionary". The model answers "how
toxic is this text, and in what way" — seven independent probabilities, not a
single verdict:

```python
from badwords.ml import ToxicityPredictor

predictor = ToxicityPredictor()
scores = predictor.predict_scores("you are a worthless idiot")

scores.toxicity       # 0.97
scores["insult"]      # 0.92
scores.strongest()    # ("toxicity", 0.97)
scores.above(0.5)     # [("toxicity", 0.97), ("insult", 0.92)]
scores.as_dict()      # every axis
```

| Axis | What it means |
|------|---------------|
| `toxicity` | Overall. This is the axis a single number comes from. |
| `severe_toxicity` | Toxic enough that annotators agreed strongly. Rare. |
| `obscene` | Obscene language. |
| `threat` | A threat of violence. |
| `insult` | Directed at a person. |
| `identity_attack` | Aimed at a group or identity. |
| `sexual_explicit` | Sexually explicit content. |

The axis names come from the model's own config, so a model you train yourself
with a different label set stays usable.

### Quality

Measured on 10,000 held-out rows — `civil_comments` and `toxic_conversations`
test splits, filtered against every row training touched. `make ml-evaluate`
reproduces it. AUC is threshold-free; "best F1" comes with the threshold that
reaches it, because one threshold does not fit seven axes of very different
rarity.

| Axis | AUC | Average precision | Best F1 | at threshold |
|------|-----|-------------------|---------|--------------|
| `toxicity` | 0.977 | 0.978 | **0.923** | 0.20 |
| `insult` | 0.957 | 0.915 | 0.843 | 0.30 |
| `identity_attack` | 0.966 | 0.628 | 0.630 | 0.30 |
| `sexual_explicit` | 0.969 | 0.647 | 0.619 | 0.30 |
| `threat` | 0.972 | 0.531 | 0.571 | 0.30 |
| `obscene` | 0.955 | 0.596 | 0.570 | 0.40 |
| `severe_toxicity` | — | — | — | — |

Measured on the INT8 model, the one the release ships: quantizing costs about
0.3 points of AUC against fp32.

Read AUC and average precision together. Every axis ranks well, but the rare
ones — `threat` is 1% of rows — have far lower average precision, which is
what you feel in practice: at a useful precision their recall is modest. Treat
`toxicity` as the axis to decide on and the narrow ones as the *reason* to show
("blocked, reads as a threat"), not as triggers of their own.

`severe_toxicity` has no row in the test set where a majority of annotators
agreed, so there is nothing to score it against at all.

The narrow axes are supervised by English `civil_comments` alone; overall
toxicity is supervised by every source. Rebalancing the training mix towards
Russian lifted `toxicity` (0.906 → 0.923 F1) and cost the narrow axes several
points — English rows in the mix were halved to make room.

### Quality in Russian

English and Russian are measured separately, because they behave differently.
1000 rows each: the English ones from the held-out split above, the Russian
ones from `AlexSham/Toxic_Russian_Comments`.

| | English | Russian |
|---|---|---|
| AUC | 0.971 | 0.997 |
| Best F1 | 0.900 @0.25 | 0.976 @0.15 |
| At the 0.3 default (P / R / F1) | 0.931 / 0.865 / 0.897 | 0.978 / 0.968 / 0.973 |
| Rules alone (P / R / F1) | 0.873 / 0.274 / 0.417 | 0.969 / 0.502 / 0.661 |

Two things are worth reading off this. The **rules behave differently per
language** — 50% recall in Russian against 27% in English, because Russian
profanity is lexical and a dictionary catches it, while English toxicity is
more often built from ordinary words. And the model's thresholds now sit close
together (0.25 and 0.15), which one default can serve; an earlier
English-heavy training mix put them at 0.30 and 0.70, and no single default
could.

The Russian figure is in-domain: the model trained on other rows of that same
corpus. Read it as "good on Russian of this kind", not as a cross-corpus
guarantee.

Neither half replaces the other. The dictionary cannot see toxicity built out
of ordinary words, which is most of it in English; the model has no notion of
the project-specific words you add to the dictionary yourself, and no interest
in matching them exactly. That is what `HybridFilter` is for.

### Rules first, model second

```python
from badwords.ml import HybridFilter

hybrid = HybridFilter(languages=["en", "ru"])

result = hybrid.check("you are a worthless waste of oxygen")
result.is_profane     # True
result.decided_by     # "model" — no dictionary entry to go on
result.scores         # every axis
result.rule_score     # what the rules made of it
```

A certain dictionary hit answers immediately and never reaches the model;
**everything else** does, text the rules found nothing in included. Treating
"the rules saw nothing" as "clean" is what makes a hybrid score worse than the
model it wraps.

The model is downloaded on first use, not on construction — `download_model()`
warms the cache deliberately, and `BADWORDS_ML_PATH` points at your own.

### From Rust

```toml
[dependencies]
badwords-ml = "3"
```

```rust
use badwords_ml::ToxicityModel;

let model = ToxicityModel::open_located()?;   // BADWORDS_ML_PATH or the shared cache
let scores = model.predict("you are an idiot")?;

scores.toxicity();        // 0.94
scores.get("insult");     // Some(0.93)
scores.above(0.5);        // [("toxicity", 0.94), ("insult", 0.93)]
```

`HybridFilter` is there too, with the same semantics as the Python one. The
crate reads the same model directory the Python package downloads, so the two
languages score identically — the only difference is batch padding, worth a few
hundredths on an INT8 model.

Requires Rust 1.88 (`ort`'s own minimum); `badwords-core` still builds on 1.78.

---

## 🌍 Supported Languages

25 languages, keyed by ISO 639-1 where one exists. Entry counts as shipped:

| Code | Language | Entries | Code | Language | Entries |
|------|----------|---------|------|----------|---------|
| `pl` | Polish | 6974 | `id` | Indonesian | 236 |
| `ru` | Russian | 3905 | `cs` | Czech | 217 |
| `uk` | Ukrainian | 2109 | `da` | Danish | 207 |
| `tr` | Turkish | 1031 | `es_419` | Spanish (Latin America) | 184 |
| `en` | English | 796 | `sv` | Swedish | 181 |
| `el` | Greek | 591 | `th` | Thai | 172 |
| `es` | Spanish | 584 | `ro` | Romanian | 145 |
| `it` | Italian | 409 | `pt_br` | Portuguese (Brazil) | 121 |
| `nl` | Dutch | 342 | `no` | Norwegian | 92 |
| `ko` | Korean | 341 | `fi` | Finnish | 303 |
| `fr` | French | 295 | `ja` | Japanese | 290 |
| `hu` | Hungarian | 286 | `pt` | Portuguese | 271 |
| `de` | German | 248 | | | |

```python
p.available_languages()   # every language that could be loaded
p.loaded_languages()      # the ones actually loaded, as canonical codes
p.load_languages(["de"])  # add to what is already loaded
p.unload_languages(["de"])
```

### Pre-3.0 codes

The old codes still work as aliases: `cz`→`cs`, `du`→`nl`, `gr`→`el`, `po`→`pt`, `sp`→`es`, `tu`→`tr`, `ua`→`uk`.

Four of them collide with a real ISO 639-1 language and emit a `DeprecationWarning`:

| Old | Now | Why |
|---|---|---|
| `br` | `pt_br` | `br` is Breton in ISO; this list is Brazilian Portuguese. |
| `in` | `id` | `in` is a retired code for Indonesian. |
| `lt` | `es_419` | `lt` is Lithuanian in ISO; this list is Latin-American Spanish. |
| `sw` | `sv` | `sw` is Swahili in ISO; this list is Swedish. |

---

## 🚀 Full Integration Example

```python
from badwords import Options, ProfanityFilter

MODERATION = Options(
    split_on_punctuation=True,
    collapse_repeats=True,
    leetspeak=True,
)


def monitor_chat() -> None:
    p = ProfanityFilter()
    p.init(["en", "ru", "de"], options=MODERATION)

    # Project-specific words, and words that must never be flagged.
    p.add_words(["spam_link_v1", "scam_bot_99"])
    p.add_whitelist(["assessment"])

    user_input = "Hey! Check out this cr4p"

    matches = p.find(user_input)
    if matches:
        reason = ", ".join(f"{m.word} ({m.kind})" for m in matches)
        print(f"Message blocked: {reason}")
        print("Shown instead:", p.censor(user_input))
    else:
        print("Message accepted")


if __name__ == "__main__":
    monitor_chat()
```

---

## 🦀 Rust API (badwords-core)

Published on [crates.io](https://crates.io/crates/badwords-core):

```toml
[dependencies]
badwords-core = "3"
```

```rust
use badwords_core::{Options, ProfanityFilter};

let mut filter = ProfanityFilter::builder()
    .embedded()                    // word lists compiled into the crate
    .languages(["en", "ru"])       // or .all_languages()
    .build()?;

let opts = Options::new();

filter.is_profane("hello world", opts);        // false
filter.censor("hey shit, ok", '*', opts);      // "hey ****, ok"

for m in filter.find("what a shitty, damn mess", opts) {
    println!("{:?} at {}..{} ({:?})", m.matched_text, m.start, m.end, m.kind);
}

filter.add_words(&["custombad"]);
filter.add_whitelist(&["assessment"]);
```

`find_first` short-circuits on the first match, and `find_into` reuses caller-owned `Scratch` and output buffers so a hot loop allocates nothing.

### Crate features

| Feature | Default | What it adds |
|---|---|---|
| `fs-resources` | ✅ | Load word lists from a directory at runtime. |
| `embedded-data` | ✅ | Normalization tables and the language registry compiled in. |
| `embedded-words` | ✅ | Every word list compiled in (~250 KB). |
| `embedded-words-min` | | English and Russian only (~82 KB), for WebAssembly. |
| `substring` | | `MatchMode::Substring` via Aho-Corasick. |

Examples: `cargo run --example rust_basic`, `rust_chat_moderation`, `rust_specific_languages`.

## 🌐 WebAssembly (JavaScript / TypeScript)

The same Rust code compiled for the browser and Node.js, with English and Russian built in.

```bash
npm install badwords-wasm
```

```javascript
import init, { ProfanityFilter } from 'badwords-wasm';

await init();                       // browser build only
const filter = new ProfanityFilter();

filter.isProfane('hello');          // false
filter.censor('hey shit, ok', '*'); // "hey ****, ok"
filter.find('what a shitty mess');  // [{ word, matchedText, start, end, language, score, kind }]

filter.addWords(['spam_link']);
filter.addWhitelist(['assessment']);
```

Options are a plain object, reusable and checked — an unknown key is an error rather than a silently disabled detector:

```javascript
filter.isProfane('sh1t', { leetspeak: true, collapseRepeats: true });
filter.setOptions({ matchThreshold: 0.9 });   // default for later calls
```

`Match` and `MatchOptions` are real TypeScript interfaces in the generated `.d.ts`. `filterText`, `isBad` and `getLanguages` still work and are marked `@deprecated`.

### Optional languages (npm)

```bash
npm install badwords-wasm @badwords/languages
```

```javascript
import de from '@badwords/languages/de';
import uk from '@badwords/languages/uk';

filter.addWords(de);
filter.addWords(uk);
```

Available: `cs`, `da`, `de`, `el`, `en`, `es`, `es_419`, `fi`, `fr`, `hu`, `id`, `it`, `ja`, `ko`, `nl`, `no`, `pl`, `pt`, `pt_br`, `ro`, `ru`, `sv`, `th`, `tr`, `uk`, plus the pre-3.0 codes as aliases. See [@badwords/languages](https://www.npmjs.com/package/@badwords/languages).

### Build from source

```bash
cargo install wasm-pack
make wasm          # browser  -> rust/badwords-wasm/pkg-web/
make wasm-nodejs   # Node.js  -> rust/badwords-wasm/pkg-node/
```

Examples: `examples/wasm/browser/`, `examples/wasm/node/` (JavaScript and TypeScript).

---

## 🔀 Migrating from 2.x

Everything from 2.x still works and still behaves identically — the deprecated paths are checked against 8736 recorded 2.3.1 responses. They warn, so the move can be gradual.

| 2.x | 3.0 |
|---|---|
| `p.filter_text(text)` | `p.is_profane(text)` |
| `p.filter_text(text, replace_character="*")` | `p.censor(text, "*")` |
| `p.filter_text(text, match_threshold=0.95)` | `p.is_profane(text, Options(match_threshold=0.95))` |
| `p.get_all_languages()` | `p.loaded_languages()` / `p.available_languages()` |
| `ProfanityFilter::new(dir, ..)` (Rust) | `ProfanityFilter::builder()…build()?` |
| `filter.isBad(text)` (JS) | `filter.isProfane(text)` |

Two behaviour changes worth knowing about:

- **Censoring keeps punctuation.** `censor("hey shit, ok")` returns `"hey ****, ok"`; 2.x returned `"hey ***** ok"` because it replaced the whole whitespace-delimited token. `filter_text` keeps the old behaviour.
- **Multi-word entries match now.** They could not before, so phrases in the shipped lists never fired. Phrase matching needs two or more consecutive words, so it cannot produce a single-word false positive; turn it off with `Options(phrases=False)`.

---

## 🔧 Building from source

Requires Rust, Python and [maturin](https://www.maturin.rs/).

```bash
python -m venv .venv && source .venv/bin/activate
pip install maturin
make develop        # debug build, installed into the venv
make test           # rust + python + wasm
make bench          # criterion + pytest-benchmark
```

`make develop` mirrors `rust/badwords-core/resources/` into `python/badwords/resource/` first; `make check-resources` verifies the mirror, and CI fails if it drifts.

## 📚 Documentation

Full documentation (Python, Rust, JavaScript) with examples and API reference: **[badwords.flacsy.dev](https://badwords.flacsy.dev)** (EN / RU).

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Word-list changes have their own checklist there — every entry needs a source, and the false-positive budget is enforced by CI.

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

Word lists incorporate material from [LDNOOBW](https://github.com/LDNOOBW/List-of-Dirty-Naughty-Obscene-and-Otherwise-Bad-Words) (CC BY 4.0) and [washyourmouthoutwithsoap](https://github.com/thisandagain/washyourmouthoutwithsoap) (MIT); see `NOTICE` and `rust/badwords-core/resources/words/SOURCES.md`.

<div align="center">
<sub>Developed with ❤️ by <a href="https://github.com/FlacSy">FlacSy</a></sub>
</div>
