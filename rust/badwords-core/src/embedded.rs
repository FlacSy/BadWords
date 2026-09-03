//! Resources compiled into the binary.
//!
//! Feature-sliced so that WebAssembly builds can take the two big languages
//! (~82 KB) instead of all twenty-five (~250 KB):
//!
//! - `embedded-data` - the normalization tables and the language registry
//! - `embedded-words` - every shipped word list
//! - `embedded-words-min` - English and Russian only

#![cfg(feature = "embedded-data")]

pub(crate) const UNICODE_MAPPINGS: &str = include_str!("../resources/data/unicode_mappings.json");
pub(crate) const HOMOGLYPHS: &str = include_str!("../resources/data/homoglyphs.json");
pub(crate) const TRANSLITERATION: &str = include_str!("../resources/data/transliteration.json");
pub(crate) const LANGUAGES: &str = include_str!("../resources/data/languages.json");

macro_rules! word_lists {
    ($($code:literal),* $(,)?) => {
        pub(crate) const WORD_LISTS: &[(&str, &str)] = &[
            $(($code, include_str!(concat!("../resources/words/", $code, ".txt")))),*
        ];
    };
}

#[cfg(feature = "embedded-words")]
word_lists![
    "cs", "da", "de", "el", "en", "es", "es_419", "fi", "fr", "hu", "id", "it", "ja", "ko", "nl",
    "no", "pl", "pt", "pt_br", "ro", "ru", "sv", "th", "tr", "uk",
];

#[cfg(all(feature = "embedded-words-min", not(feature = "embedded-words")))]
word_lists!["en", "ru"];

#[cfg(not(any(feature = "embedded-words", feature = "embedded-words-min")))]
pub(crate) const WORD_LISTS: &[(&str, &str)] = &[];

/// Word list contents for `file`, e.g. `pt_br.txt`.
pub(crate) fn word_list(file: &str) -> Option<&'static str> {
    let stem = file.strip_suffix(".txt").unwrap_or(file);
    WORD_LISTS
        .iter()
        .find(|(code, _)| *code == stem)
        .map(|(_, content)| *content)
}

/// File names of every embedded word list.
pub(crate) fn word_list_files() -> Vec<String> {
    WORD_LISTS
        .iter()
        .map(|(code, _)| format!("{code}.txt"))
        .collect()
}
