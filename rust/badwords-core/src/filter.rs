//! The filter: construction, language and dictionary management, matching.

use crate::dict::{Dictionary, CUSTOM_LANG_BIT};
use crate::error::{Error, LanguageWarning};
use crate::lang::LanguageRegistry;
use crate::matcher::{self, Ctx, Match, Scratch};
use crate::options::{Options, Processing};
use crate::processor::TextProcessor;
use crate::resources::{DataFile, ResourceSource};

/// A profanity filter.
///
/// Build one with [`ProfanityFilter::builder`], load languages, then match:
///
/// ```no_run
/// use badwords_core::{Options, ProfanityFilter};
///
/// let filter = ProfanityFilter::builder()
///     .embedded()
///     .languages(["en", "ru"])
///     .build()?;
///
/// assert!(filter.is_profane("some bad text", Options::new()));
/// # Ok::<(), badwords_core::Error>(())
/// ```
#[derive(Debug)]
pub struct ProfanityFilter {
    pub(crate) source: ResourceSource,
    pub(crate) processor: TextProcessor,
    pub(crate) processing: Processing,
    pub(crate) registry: LanguageRegistry,
    pub(crate) available: Vec<String>,
    pub(crate) loaded: Vec<String>,
    pub(crate) dict: Dictionary,
    pub(crate) warnings: Vec<LanguageWarning>,
    pub(crate) default_options: Options,
    /// Legacy `get_all_languages` reports available languages until `init` runs.
    pub(crate) initialized: bool,
    #[cfg(feature = "substring")]
    pub(crate) automaton: std::sync::OnceLock<Option<crate::substring::Automaton>>,
}

impl ProfanityFilter {
    /// Start configuring a filter.
    #[must_use]
    pub fn builder() -> ProfanityFilterBuilder {
        ProfanityFilterBuilder::default()
    }

    /// Read resources from a directory laid out like the crate's `resources/`.
    ///
    /// No languages are loaded yet; call [`ProfanityFilter::load_languages`].
    ///
    /// # Errors
    /// [`Error::Io`] or [`Error::Json`] if the normalization tables are missing
    /// or malformed. 2.x ignored those failures and returned a filter that
    /// silently detected nothing.
    #[cfg(feature = "fs-resources")]
    pub fn try_new(resource_dir: &std::path::Path, processing: Processing) -> Result<Self, Error> {
        Self::from_source(ResourceSource::directory(resource_dir), processing)
    }

    /// Use the resources compiled into the binary.
    ///
    /// # Errors
    /// [`Error::Json`] if an embedded table fails to parse, which would mean a
    /// corrupt build.
    #[cfg(feature = "embedded-data")]
    pub fn embedded(processing: Processing) -> Result<Self, Error> {
        Self::from_source(ResourceSource::Embedded, processing)
    }

    /// Build from an explicit resource source.
    ///
    /// # Errors
    /// [`Error::Io`], [`Error::Json`] or [`Error::MissingResource`].
    pub fn from_source(source: ResourceSource, processing: Processing) -> Result<Self, Error> {
        let mut processor = TextProcessor::new(
            processing.normalize_text,
            processing.aggressive_normalize,
            processing.transliterate,
            processing.replace_homoglyphs,
        );

        let unicode = source
            .data(DataFile::UnicodeMappings)?
            .ok_or(Error::MissingResource {
                what: "data/unicode_mappings.json",
            })?;
        let homoglyphs = source
            .data(DataFile::Homoglyphs)?
            .ok_or(Error::MissingResource {
                what: "data/homoglyphs.json",
            })?;
        let translit = source
            .data(DataFile::Transliteration)?
            .ok_or(Error::MissingResource {
                what: "data/transliteration.json",
            })?;
        processor.load_from_str(&unicode, &homoglyphs, &translit)?;

        let files = source.word_list_files();
        let mut registry = match source.data(DataFile::Languages)? {
            Some(json) => {
                let mut registry = LanguageRegistry::parse(&json)?;
                registry.retain_files(&files);
                registry
            }
            None => LanguageRegistry::default(),
        };
        if registry.is_empty() {
            // No registry shipped: fall back to file stems, no aliases.
            registry = LanguageRegistry::from_codes(
                files
                    .iter()
                    .filter_map(|f| f.strip_suffix(".txt"))
                    .map(str::to_owned),
            );
        }
        let available: Vec<String> = registry.canonical_codes().map(str::to_owned).collect();

        Ok(Self {
            source,
            processor,
            processing,
            registry,
            available,
            loaded: Vec::new(),
            dict: Dictionary::default(),
            warnings: Vec::new(),
            default_options: Options::default(),
            initialized: false,
            #[cfg(feature = "substring")]
            automaton: std::sync::OnceLock::new(),
        })
    }

    /// A filter with no resources, for the infallible legacy constructor.
    #[cfg(feature = "fs-resources")]
    pub(crate) fn empty(processing: Processing) -> Self {
        Self {
            source: ResourceSource::InMemory {
                unicode_mappings: "{}".to_string(),
                homoglyphs: "{}".to_string(),
                transliteration: "{\"cyrillic_to_latin\":{}}".to_string(),
                languages: None,
                words: std::collections::HashMap::new(),
            },
            processor: TextProcessor::new(
                processing.normalize_text,
                processing.aggressive_normalize,
                processing.transliterate,
                processing.replace_homoglyphs,
            ),
            processing,
            registry: LanguageRegistry::default(),
            available: Vec::new(),
            loaded: Vec::new(),
            dict: Dictionary::default(),
            warnings: Vec::new(),
            default_options: Options::default(),
            initialized: false,
            #[cfg(feature = "substring")]
            automaton: std::sync::OnceLock::new(),
        }
    }

    // -- languages ------------------------------------------------------------

    /// Every language this filter could load. Never changes after construction.
    #[must_use]
    pub fn available_languages(&self) -> &[String] {
        &self.available
    }

    /// Languages whose word lists are loaded, as canonical codes.
    #[must_use]
    pub fn loaded_languages(&self) -> &[String] {
        &self.loaded
    }

    /// The language registry, for code and alias lookups.
    #[must_use]
    pub fn registry(&self) -> &LanguageRegistry {
        &self.registry
    }

    /// Non-fatal diagnostics collected so far, newest last.
    #[must_use]
    pub fn warnings(&self) -> &[LanguageWarning] {
        &self.warnings
    }

    /// Clear collected warnings.
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }

    /// Resolve a code or alias to its canonical form.
    ///
    /// # Errors
    /// [`Error::UnknownLanguage`] if it is neither.
    pub fn resolve_language(&self, code: &str) -> Result<&str, Error> {
        self.registry.resolve(code).map(|r| r.canonical)
    }

    /// Load languages in addition to those already loaded.
    ///
    /// # Errors
    /// [`Error::UnknownLanguage`] if any code is unknown; nothing is loaded then.
    pub fn load_languages<S: AsRef<str>>(&mut self, codes: &[S]) -> Result<(), Error> {
        let resolved = self.resolve_all(codes)?;
        for code in resolved {
            self.load_one(&code)?;
        }
        self.finish_edit();
        self.initialized = true;
        Ok(())
    }

    /// Replace the loaded set with exactly these languages.
    ///
    /// # Errors
    /// [`Error::UnknownLanguage`] if any code is unknown; nothing changes then.
    pub fn reload_languages<S: AsRef<str>>(&mut self, codes: &[S]) -> Result<(), Error> {
        let resolved = self.resolve_all(codes)?;
        self.unload_all();
        for code in &resolved {
            self.load_one(code)?;
        }
        self.finish_edit();
        self.initialized = true;
        Ok(())
    }

    /// Load every available language.
    ///
    /// # Errors
    /// [`Error::Io`] if a word list cannot be read.
    pub fn load_all_languages(&mut self) -> Result<(), Error> {
        let all = self.available.clone();
        self.reload_languages(&all)
    }

    /// Unload languages, dropping the entries only they provided.
    ///
    /// # Errors
    /// [`Error::UnknownLanguage`] if any code is unknown.
    pub fn unload_languages<S: AsRef<str>>(&mut self, codes: &[S]) -> Result<(), Error> {
        let resolved = self.resolve_all(codes)?;
        self.dict.remove_languages(&resolved);
        self.loaded.retain(|c| !resolved.contains(c));
        self.finish_edit();
        Ok(())
    }

    fn resolve_all<S: AsRef<str>>(&mut self, codes: &[S]) -> Result<Vec<String>, Error> {
        let mut resolved = Vec::with_capacity(codes.len());
        let mut warnings = Vec::new();
        for code in codes {
            let code = code.as_ref();
            let hit = self.registry.resolve(code)?;
            if let Some(note) = hit.deprecated {
                warnings.push(LanguageWarning::DeprecatedAlias {
                    requested: code.to_string(),
                    canonical: hit.canonical.to_string(),
                    note: note.to_string(),
                });
            }
            resolved.push(hit.canonical.to_string());
        }
        self.warnings.extend(warnings);
        resolved.dedup();
        Ok(resolved)
    }

    fn load_one(&mut self, canonical: &str) -> Result<(), Error> {
        if self.loaded.iter().any(|c| c == canonical) {
            self.warnings.push(LanguageWarning::AlreadyLoaded {
                code: canonical.to_string(),
            });
            return Ok(());
        }
        let Some(info) = self.registry.info(canonical) else {
            return Err(Error::UnknownLanguage {
                code: canonical.to_string(),
                available: self.available.clone(),
            });
        };
        let file = info.file.clone();
        let Some(content) = self.source.word_list(&file)? else {
            return Err(Error::MissingResource { what: "word list" });
        };

        let bit = self.dict.language_bit(canonical);
        let mut count = 0usize;
        // Word lists are hand-edited; a leading BOM would otherwise be glued to
        // the first entry whenever normalization is off.
        let content = content.trim_start_matches('\u{feff}').to_owned();
        for line in content.lines() {
            let raw = line.trim();
            if raw.is_empty() {
                continue;
            }
            let form = self.processor.process_text(raw);
            if form.is_empty() {
                continue;
            }
            self.dict.insert(&form, raw, bit);
            count += 1;
        }
        if count == 0 {
            self.warnings.push(LanguageWarning::EmptyWordList {
                code: canonical.to_string(),
            });
        }
        self.loaded.push(canonical.to_string());
        Ok(())
    }

    fn unload_all(&mut self) {
        self.dict.clear();
        self.loaded.clear();
    }

    // -- dictionary -----------------------------------------------------------

    /// Add words on top of the loaded languages.
    pub fn add_words<S: AsRef<str>>(&mut self, words: &[S]) {
        for word in words {
            let raw = word.as_ref();
            let form = self.processor.process_text(raw);
            if !form.is_empty() {
                self.dict.insert(&form, raw, CUSTOM_LANG_BIT);
            }
        }
        self.finish_edit();
    }

    /// Remove words. A word a loaded language also provides stays.
    pub fn remove_words<S: AsRef<str>>(&mut self, words: &[S]) {
        for word in words {
            let form = self.processor.process_text(word.as_ref());
            if !form.is_empty() {
                self.dict.remove(&form);
            }
        }
        self.finish_edit();
    }

    /// Drop every word, including those from loaded languages.
    pub fn clear_words(&mut self) {
        self.dict.clear();
        self.loaded.clear();
        self.finish_edit();
    }

    /// Number of distinct entries.
    #[must_use]
    pub fn word_count(&self) -> usize {
        self.dict.len()
    }

    /// Whether a word is in the dictionary, after normalization.
    #[must_use]
    pub fn contains_word(&self, word: &str) -> bool {
        let form = self.processor.process_text(word);
        !form.is_empty() && self.dict.lookup(&form).is_some()
    }

    // -- whitelist ------------------------------------------------------------

    /// Never report these words, even when a rule would match them.
    ///
    /// The whitelist is consulted per token: in [`MatchMode::Substring`] mode
    /// whitelisting `assess` suppresses the `ass` hit inside it.
    ///
    /// [`MatchMode::Substring`]: crate::MatchMode::Substring
    pub fn add_whitelist<S: AsRef<str>>(&mut self, words: &[S]) {
        for word in words {
            let form = self.processor.process_text(word.as_ref());
            self.dict.whitelist_insert(&form);
        }
    }

    /// Drop words from the whitelist.
    pub fn remove_whitelist<S: AsRef<str>>(&mut self, words: &[S]) {
        for word in words {
            let form = self.processor.process_text(word.as_ref());
            self.dict.whitelist_remove(&form);
        }
    }

    /// Empty the whitelist.
    pub fn clear_whitelist(&mut self) {
        self.dict.whitelist_clear();
    }

    /// Whether a word is whitelisted, after normalization.
    #[must_use]
    pub fn is_whitelisted(&self, word: &str) -> bool {
        let form = self.processor.process_text(word);
        self.dict.is_whitelisted(&form)
    }

    // -- matching -------------------------------------------------------------

    /// Whether the text contains profanity. Stops at the first match.
    #[must_use]
    pub fn is_profane(&self, text: &str, opts: Options) -> bool {
        let mut scratch = Scratch::new();
        let mut out = Vec::new();
        matcher::find_into(&self.ctx(), text, opts, &mut scratch, &mut out, true);
        !out.is_empty()
    }

    /// Every match, sorted by position and non-overlapping.
    #[must_use]
    pub fn find(&self, text: &str, opts: Options) -> Vec<Match> {
        let mut scratch = Scratch::new();
        let mut out = Vec::new();
        matcher::find_into(&self.ctx(), text, opts, &mut scratch, &mut out, false);
        out
    }

    /// The first match, if any.
    #[must_use]
    pub fn find_first(&self, text: &str, opts: Options) -> Option<Match> {
        let mut scratch = Scratch::new();
        let mut out = Vec::new();
        matcher::find_into(&self.ctx(), text, opts, &mut scratch, &mut out, true);
        out.into_iter().next()
    }

    /// [`ProfanityFilter::find`] reusing caller-owned buffers.
    pub fn find_into(
        &self,
        text: &str,
        opts: Options,
        scratch: &mut Scratch,
        out: &mut Vec<Match>,
    ) {
        matcher::find_into(&self.ctx(), text, opts, scratch, out, false);
    }

    /// Replace every match with `replace_char`, keeping everything else.
    ///
    /// Punctuation attached to a word survives, and whitespace inside a matched
    /// phrase is preserved: `"hey fuck, ok"` becomes `"hey ****, ok"`.
    #[must_use]
    pub fn censor(&self, text: &str, replace_char: char, opts: Options) -> String {
        let matches = self.find(text, opts);
        if matches.is_empty() {
            return text.to_string();
        }
        self.apply_censor(text, &matches, replace_char)
    }

    /// Replace every match with the output of `f`.
    #[must_use]
    pub fn censor_with<F>(&self, text: &str, mut f: F, opts: Options) -> String
    where
        F: FnMut(&Match) -> String,
    {
        let matches = self.find(text, opts);
        if matches.is_empty() {
            return text.to_string();
        }
        let mut out = String::with_capacity(text.len());
        let mut prev = 0usize;
        for m in &matches {
            out.push_str(&text[prev..m.start]);
            out.push_str(&f(m));
            prev = m.end;
        }
        out.push_str(&text[prev..]);
        out
    }

    pub(crate) fn apply_censor(&self, text: &str, matches: &[Match], replace_char: char) -> String {
        let mut out = String::with_capacity(text.len());
        let mut prev = 0usize;
        for m in matches {
            out.push_str(&text[prev..m.start]);
            for c in text[m.start..m.end].chars() {
                if c.is_whitespace() {
                    out.push(c);
                } else {
                    out.push(replace_char);
                }
            }
            prev = m.end;
        }
        out.push_str(&text[prev..]);
        out
    }

    // -- batch ----------------------------------------------------------------

    /// [`ProfanityFilter::is_profane`] over many texts, reusing one scratch buffer.
    #[must_use]
    pub fn is_profane_many<S: AsRef<str>>(&self, texts: &[S], opts: Options) -> Vec<bool> {
        let ctx = self.ctx();
        let mut scratch = Scratch::new();
        let mut out = Vec::new();
        texts
            .iter()
            .map(|text| {
                matcher::find_into(&ctx, text.as_ref(), opts, &mut scratch, &mut out, true);
                !out.is_empty()
            })
            .collect()
    }

    /// [`ProfanityFilter::find`] over many texts.
    #[must_use]
    pub fn find_many<S: AsRef<str>>(&self, texts: &[S], opts: Options) -> Vec<Vec<Match>> {
        let ctx = self.ctx();
        let mut scratch = Scratch::new();
        texts
            .iter()
            .map(|text| {
                let mut out = Vec::new();
                matcher::find_into(&ctx, text.as_ref(), opts, &mut scratch, &mut out, false);
                out
            })
            .collect()
    }

    /// [`ProfanityFilter::censor`] over many texts.
    #[must_use]
    pub fn censor_many<S: AsRef<str>>(
        &self,
        texts: &[S],
        replace_char: char,
        opts: Options,
    ) -> Vec<String> {
        let ctx = self.ctx();
        let mut scratch = Scratch::new();
        let mut out = Vec::new();
        texts
            .iter()
            .map(|text| {
                let text = text.as_ref();
                matcher::find_into(&ctx, text, opts, &mut scratch, &mut out, false);
                if out.is_empty() {
                    text.to_string()
                } else {
                    self.apply_censor(text, &out, replace_char)
                }
            })
            .collect()
    }

    // -- utility --------------------------------------------------------------

    /// Jaro-Winkler similarity of two strings.
    #[must_use]
    pub fn similar(&self, a: &str, b: &str) -> f64 {
        strsim::jaro_winkler(a, b)
    }

    /// The normalized form of a text, as the matcher sees it.
    #[must_use]
    pub fn normalize(&self, text: &str) -> String {
        self.processor.process_text(text)
    }

    /// The text processor in use.
    #[must_use]
    pub fn processor(&self) -> &TextProcessor {
        &self.processor
    }

    /// The processing options this filter was built with.
    #[must_use]
    pub fn processing(&self) -> Processing {
        self.processing
    }

    /// Options used by the deprecated 2.x entry points.
    #[must_use]
    pub fn default_options(&self) -> Options {
        self.default_options
    }

    fn ctx(&self) -> Ctx<'_> {
        Ctx {
            dict: &self.dict,
            processor: &self.processor,
            #[cfg(feature = "substring")]
            automaton: self
                .automaton
                .get_or_init(|| crate::substring::Automaton::build(&self.dict))
                .as_ref(),
        }
    }

    /// Rebuild indices after a batch of dictionary edits.
    fn finish_edit(&mut self) {
        self.dict.reindex();
        #[cfg(feature = "substring")]
        {
            self.automaton = std::sync::OnceLock::new();
        }
    }
}

/// Builder for [`ProfanityFilter`].
#[derive(Debug, Default)]
pub struct ProfanityFilterBuilder {
    source: Option<ResourceSource>,
    processing: Processing,
    languages: Option<Vec<String>>,
    all_languages: bool,
    extra_words: Vec<String>,
    whitelist: Vec<String>,
    default_options: Option<Options>,
}

impl ProfanityFilterBuilder {
    /// Read resources from a directory.
    #[cfg(feature = "fs-resources")]
    #[must_use]
    pub fn resource_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.source = Some(ResourceSource::directory(path));
        self
    }

    /// Use the resources compiled into the binary.
    #[cfg(feature = "embedded-data")]
    #[must_use]
    pub fn embedded(mut self) -> Self {
        self.source = Some(ResourceSource::Embedded);
        self
    }

    /// Use an explicit resource source.
    #[must_use]
    pub fn source(mut self, source: ResourceSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Normalization settings.
    #[must_use]
    pub fn processing(mut self, processing: Processing) -> Self {
        self.processing = processing;
        self
    }

    /// Languages to load. Codes and aliases both work.
    #[must_use]
    pub fn languages<I, S>(mut self, codes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.languages = Some(codes.into_iter().map(|c| c.as_ref().to_owned()).collect());
        self
    }

    /// Load every available language.
    #[must_use]
    pub fn all_languages(mut self) -> Self {
        self.all_languages = true;
        self
    }

    /// Extra words on top of the loaded languages.
    #[must_use]
    pub fn extra_words<I, S>(mut self, words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.extra_words
            .extend(words.into_iter().map(|w| w.as_ref().to_owned()));
        self
    }

    /// Words that must never be reported.
    #[must_use]
    pub fn whitelist<I, S>(mut self, words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.whitelist
            .extend(words.into_iter().map(|w| w.as_ref().to_owned()));
        self
    }

    /// Options the deprecated 2.x entry points should use.
    #[must_use]
    pub fn default_options(mut self, options: Options) -> Self {
        self.default_options = Some(options);
        self
    }

    /// Build the filter.
    ///
    /// Non-fatal diagnostics (deprecated aliases, empty word lists) are readable
    /// afterwards through [`ProfanityFilter::warnings`].
    ///
    /// # Errors
    /// [`Error::MissingResource`] if no source was given, plus anything
    /// [`ProfanityFilter::from_source`] can return.
    pub fn build(self) -> Result<ProfanityFilter, Error> {
        let source = self.source.ok_or(Error::MissingResource {
            what: "resource source (call .embedded() or .resource_dir())",
        })?;
        let mut filter = ProfanityFilter::from_source(source, self.processing)?;

        if let Some(options) = self.default_options {
            filter.default_options = options;
        }
        if self.all_languages {
            filter.load_all_languages()?;
        } else if let Some(codes) = self.languages {
            filter.reload_languages(&codes)?;
        }
        if !self.extra_words.is_empty() {
            filter.add_words(&self.extra_words);
        }
        if !self.whitelist.is_empty() {
            filter.add_whitelist(&self.whitelist);
        }
        Ok(filter)
    }
}
