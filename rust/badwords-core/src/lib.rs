//! High-performance profanity filter.
//!
//! Detection is normalization plus dictionary lookup: text is folded to a
//! canonical form (NFKC, case, confusable characters, homoglyphs,
//! transliteration) and looked up in a hash set, with optional fuzzy, phrase
//! and substring matching on top.
//!
//! ```no_run
//! use badwords_core::{Options, ProfanityFilter};
//!
//! let filter = ProfanityFilter::builder()
//!     .embedded()
//!     .languages(["en", "ru"])
//!     .build()?;
//!
//! let opts = Options::new();
//! assert!(!filter.is_profane("hello world", opts));
//! println!("{}", filter.censor("some bad text", '*', opts));
//! # Ok::<(), badwords_core::Error>(())
//! ```
//!
//! # Options
//!
//! [`Options::default`] reproduces 2.x behaviour: exact whole-token matching.
//! Every evasion detector added in 3.0.0 is opt-in, because each one trades
//! false negatives for false positives - see the field docs on [`Options`].
//!
//! # Features
//!
//! - `fs-resources` (default) - load word lists from a directory
//! - `embedded-data` (default) - normalization tables compiled in
//! - `embedded-words` (default) - every word list compiled in
//! - `embedded-words-min` - English and Russian only, for WebAssembly
//! - `substring` - [`MatchMode::Substring`] via Aho-Corasick

mod compat;
mod dict;
mod embedded;
mod error;
mod filter;
mod fuzzy;
mod lang;
mod matcher;
mod options;
mod processor;
mod resources;
#[cfg(feature = "substring")]
mod substring;
mod tokenize;

#[allow(deprecated)]
pub use error::NotSupportedLanguage;
pub use error::{Error, LanguageWarning};
pub use filter::{ProfanityFilter, ProfanityFilterBuilder};
pub use lang::{LanguageInfo, LanguageRegistry, Resolved};
pub use matcher::{Match, MatchKind, Scratch};
pub use options::{MatchMode, Options, Processing};
pub use processor::TextProcessor;
pub use resources::ResourceSource;

/// Path to the word lists inside a checkout of this repository.
///
/// # Deprecated
/// Resolves through `CARGO_MANIFEST_DIR`, so it only points anywhere real when
/// building from the repository. Use [`ProfanityFilter::embedded`] instead.
#[cfg(feature = "fs-resources")]
#[deprecated(
    since = "3.0.0",
    note = "only valid inside this repository; use `ProfanityFilter::embedded()`"
)]
#[must_use]
pub fn default_resource_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
}
