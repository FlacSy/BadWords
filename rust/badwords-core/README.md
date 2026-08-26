# badwords-core

High-performance profanity filter: the Rust core behind
[badwords-py](https://pypi.org/project/badwords-py/) and
[badwords-wasm](https://www.npmjs.com/package/badwords-wasm).

Detection is normalization plus dictionary lookup. Text is folded to a
canonical form - NFKC, case, confusable characters, homoglyphs, latin/cyrillic
transliteration - and looked up in a hash set, with optional fuzzy, phrase and
substring matching on top. Word lists for 25 languages are compiled in.

```toml
[dependencies]
badwords-core = "3"
```

```rust
use badwords_core::{Options, ProfanityFilter};

let filter = ProfanityFilter::builder()
    .embedded()
    .languages(["en", "ru"])
    .build()?;

let opts = Options::new();

assert!(!filter.is_profane("hello world", opts));
assert_eq!(filter.censor("hey shit, ok", '*', opts), "hey ****, ok");

for m in filter.find("what a shitty day", opts) {
    println!("{:?} at {}..{} ({:?})", m.matched_text, m.start, m.end, m.language);
}
# Ok::<(), badwords_core::Error>(())
```

## Options

`Options::default()` is exact whole-word matching. Every evasion detector is
opt-in, because each one trades false negatives for false positives. Measured
against 73,000 clean English words:

| Option | Catches | False positives |
|--------|---------|-----------------|
| default | `f.u.c.k`, homoglyphs, fullwidth, transliteration | 0 |
| `split_on_punctuation` | `you.fuck`, `hey-shit` | 0 |
| `collapse_repeats` | `fuuuck`, `ffuck` | 0 |
| `leetspeak` | `sh1t`, `@ss`, `p0rn` | 0 |
| `match_threshold(0.95)` | typos | 0.81% |
| `MatchMode::Substring` | `xxfuckxx` | 0.37% at the default minimum length of 6 |

Substring matching costs less with fewer languages loaded: 0.15% with English
alone against 0.37% with all twenty-five, because most of the cost is a short
entry in one language occurring inside ordinary words of another.

`cargo run --bin fp_report --features substring` reproduces the table.

## Features

- `fs-resources` (default) - load word lists from a directory
- `embedded-data` (default) - normalization tables and the language registry
- `embedded-words` (default) - all 25 word lists, about 250 KB
- `embedded-words-min` - English and Russian only, about 82 KB, for WebAssembly
- `substring` - `MatchMode::Substring`, via `aho-corasick`

## Languages

Codes are ISO 639-1 where one exists. The codes used before 3.0.0 keep working
as aliases; `br`, `in`, `lt` and `sw` are deprecated because each is the ISO
code of a *different* language, and using one produces a `LanguageWarning`
naming the replacement.

## Word lists

See `resources/words/SOURCES.md` for provenance and `NOTICE` for the required
attributions.

## License

MIT. See `LICENSE` and `NOTICE`.
