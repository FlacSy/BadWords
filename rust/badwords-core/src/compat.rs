//! The 2.x API, kept working.
//!
//! Every entry point here is deprecated but behaves as it did in 2.3.1, which
//! `tests/compat_golden.rs` checks against a snapshot taken before the rewrite.
//!
//! Two deliberate deviations:
//!
//! - `init` can now be called more than once. In 2.x the second call failed,
//!   because the first overwrote the list of *available* languages with the
//!   list of *loaded* ones. That was the bug.
//! - `new` no longer silently succeeds on a broken resource directory beyond
//!   what 2.x did: it still cannot fail, but the resulting filter is explicitly
//!   empty rather than half-initialized.

#![allow(deprecated)]

use std::collections::HashMap;

use crate::error::NotSupportedLanguage;
use crate::options::{Processing, SpanMode};
use crate::resources::ResourceSource;
use crate::ProfanityFilter;

impl ProfanityFilter {
    /// Create a filter from a resource directory.
    ///
    /// # Deprecated
    /// Resource-loading failures are swallowed. Use
    /// [`ProfanityFilter::try_new`] or [`ProfanityFilter::builder`].
    #[cfg(feature = "fs-resources")]
    #[deprecated(
        since = "3.0.0",
        note = "use `ProfanityFilter::try_new` or `::builder()`; `new` swallows resource errors"
    )]
    #[must_use]
    pub fn new(
        resource_dir: &std::path::Path,
        normalize_text: bool,
        aggressive_normalize: bool,
        transliterate: bool,
        replace_homoglyphs: bool,
    ) -> Self {
        let processing = Processing {
            normalize_text,
            aggressive_normalize,
            transliterate,
            replace_homoglyphs,
        };
        Self::try_new(resource_dir, processing).unwrap_or_else(|_| Self::empty(processing))
    }

    /// Create a filter from in-memory resources.
    ///
    /// # Deprecated
    /// Use [`ProfanityFilter::builder`] with [`ResourceSource::InMemory`].
    ///
    /// # Errors
    /// If the tables cannot be parsed.
    #[deprecated(
        since = "3.0.0",
        note = "use `ProfanityFilter::builder().source(ResourceSource::InMemory { .. })`"
    )]
    #[allow(clippy::too_many_arguments)]
    pub fn new_from_embedded(
        unicode_mappings_json: &str,
        homoglyphs_json: &str,
        transliteration_json: &str,
        words_by_lang: &HashMap<String, String>,
        languages: Vec<String>,
        normalize_text: bool,
        aggressive_normalize: bool,
        transliterate: bool,
        replace_homoglyphs: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let processing = Processing {
            normalize_text,
            aggressive_normalize,
            transliterate,
            replace_homoglyphs,
        };
        let source = ResourceSource::InMemory {
            unicode_mappings: unicode_mappings_json.to_string(),
            homoglyphs: homoglyphs_json.to_string(),
            transliteration: transliteration_json.to_string(),
            languages: None,
            words: words_by_lang.clone(),
        };
        let mut filter = Self::from_source(source, processing)?;
        // 2.x silently skipped codes that were not plain letters.
        let wanted: Vec<String> = languages
            .into_iter()
            .filter(|code| code.chars().all(char::is_alphabetic))
            .collect();
        filter.reload_languages(&wanted)?;
        Ok(filter)
    }

    /// Load word lists.
    ///
    /// # Deprecated
    /// Use [`ProfanityFilter::load_languages`] or
    /// [`ProfanityFilter::reload_languages`], which report which code failed.
    ///
    /// # Errors
    /// [`NotSupportedLanguage`] if a code is unknown.
    #[deprecated(
        since = "3.0.0",
        note = "use `load_languages` / `reload_languages`, which return a typed error"
    )]
    pub fn init(&mut self, languages: Option<&[String]>) -> Result<(), NotSupportedLanguage> {
        match languages {
            Some(codes) => self.reload_languages(codes).map_err(Into::into),
            None => self.load_all_languages().map_err(Into::into),
        }
    }

    /// Check or censor text.
    ///
    /// # Deprecated
    /// Use [`ProfanityFilter::is_profane`], [`ProfanityFilter::censor`] or
    /// [`ProfanityFilter::find`]. Returns `(found, censored)`, where `censored`
    /// is `None` unless `replace_char` was given and something matched.
    #[deprecated(since = "3.0.0", note = "use `is_profane`, `censor` or `find`")]
    #[must_use]
    pub fn filter_text(
        &self,
        text: &str,
        match_threshold: f64,
        replace_char: Option<char>,
    ) -> (bool, Option<String>) {
        // 2.x censored the whole whitespace-delimited token, punctuation and
        // all, and never matched multi-word entries.
        let opts = self
            .default_options
            .threshold(match_threshold)
            .phrases(false)
            .span(SpanMode::WholeSegment);

        match replace_char {
            None => (self.is_profane(text, opts), None),
            Some(ch) => {
                let matches = self.find(text, opts);
                if matches.is_empty() {
                    (false, None)
                } else {
                    (true, Some(self.apply_censor(text, &matches, ch)))
                }
            }
        }
    }

    /// Loaded languages, or the available ones before `init` has run.
    ///
    /// # Deprecated
    /// The name is misleading. Use [`ProfanityFilter::loaded_languages`] or
    /// [`ProfanityFilter::available_languages`].
    #[deprecated(
        since = "3.0.0",
        note = "returns LOADED languages; use `loaded_languages()` or `available_languages()`"
    )]
    #[must_use]
    pub fn get_all_languages(&self) -> &[String] {
        if self.initialized {
            self.loaded_languages()
        } else {
            self.available_languages()
        }
    }
}
