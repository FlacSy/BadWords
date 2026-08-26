//! BadWords for the browser and Node.js.
//!
//! English and Russian are compiled in through `badwords-core`'s
//! `embedded-words-min` feature (~82 KB rather than ~250 KB for all of them).
//! Other languages come from the `@badwords/languages` package via
//! [`ProfanityFilter::add_words`] or [`ProfanityFilter::add_words_from_text`].

use badwords_core::{MatchMode, Options as CoreOptions, Processing, ProfanityFilter as CoreFilter};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// How a single call should match.
///
/// Passed as a plain object, so it can be reused across calls and written
/// inline: `filter.isProfane(text, { collapseRepeats: true })`. Every evasion
/// detector is off by default, so `{}` reproduces badwords-wasm 2.x behaviour.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct MatchOptions {
    /// Similarity a fuzzy match needs. `1.0` is exact only.
    pub match_threshold: f64,
    /// Also test the pieces a token splits into on inner punctuation.
    pub split_on_punctuation: bool,
    /// Also test forms with runs of repeated letters collapsed.
    pub collapse_repeats: bool,
    /// Also test a form with digits and symbols read as letters.
    pub leetspeak: bool,
    /// Match multi-word entries against consecutive words.
    pub phrases: bool,
    /// Match entries occurring inside a longer word.
    pub substring: bool,
    /// In substring mode, ignore entries shorter than this.
    pub min_substring_len: usize,
    /// Stop after this many matches.
    pub max_matches: Option<usize>,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            match_threshold: 1.0,
            split_on_punctuation: false,
            collapse_repeats: false,
            leetspeak: false,
            phrases: true,
            substring: false,
            min_substring_len: 6,
            max_matches: None,
        }
    }
}

impl From<MatchOptions> for CoreOptions {
    fn from(options: MatchOptions) -> Self {
        Self::new()
            .threshold(options.match_threshold)
            .split_on_punctuation(options.split_on_punctuation)
            .collapse_repeats(options.collapse_repeats)
            .leetspeak(options.leetspeak)
            .phrases(options.phrases)
            .min_substring_len(options.min_substring_len)
            .max_matches(options.max_matches)
            .match_mode(if options.substring {
                MatchMode::Substring
            } else {
                MatchMode::Token
            })
    }
}

/// TypeScript declarations for the plain objects crossing the boundary.
#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_TYPES: &'static str = r#"
/** How a single call should match. Every field is optional. */
export interface MatchOptions {
    /** Similarity a fuzzy match needs. 1.0 (the default) is exact only. */
    matchThreshold?: number;
    /** Also test the pieces a token splits into on inner punctuation. */
    splitOnPunctuation?: boolean;
    /** Also test forms with runs of repeated letters collapsed. */
    collapseRepeats?: boolean;
    /** Also test a form with digits and symbols read as letters. */
    leetspeak?: boolean;
    /** Match multi-word entries against consecutive words. Default true. */
    phrases?: boolean;
    /** Match entries occurring inside a longer word. */
    substring?: boolean;
    /** In substring mode, ignore entries shorter than this. Default 6. */
    minSubstringLen?: number;
    /** Stop after this many matches. */
    maxMatches?: number;
}

/** One detected occurrence. */
export interface Match {
    /** The dictionary entry that matched, as written in the word list. */
    word: string;
    /** The matched slice of the input. */
    matchedText: string;
    /** Byte offset into the original text. */
    start: number;
    /** Byte offset into the original text, exclusive. */
    end: number;
    /** Language the entry came from, or null for added words. */
    language: string | null;
    /** Similarity; 1.0 for anything but a fuzzy match. */
    score: number;
    /** How it was found. */
    kind: "exact" | "fuzzy" | "leet" | "collapsed" | "substring" | "phrase";
}
"#;

/// One detected occurrence, serialized to a plain object.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchOut {
    word: String,
    matched_text: String,
    start: usize,
    end: usize,
    language: Option<String>,
    score: f64,
    kind: &'static str,
}

impl From<badwords_core::Match> for MatchOut {
    fn from(m: badwords_core::Match) -> Self {
        use badwords_core::MatchKind::{Collapsed, Exact, Fuzzy, Leet, Phrase, Substring};
        let kind = match m.kind {
            Exact => "exact",
            Fuzzy => "fuzzy",
            Leet => "leet",
            Collapsed => "collapsed",
            Substring => "substring",
            Phrase => "phrase",
            _ => "unknown",
        };
        Self {
            word: m.word,
            matched_text: m.matched_text,
            start: m.start,
            end: m.end,
            language: m.language,
            score: m.score,
            kind,
        }
    }
}

/// Profanity filter.
#[wasm_bindgen]
pub struct ProfanityFilter {
    inner: CoreFilter,
    options: MatchOptions,
}

#[wasm_bindgen]
impl ProfanityFilter {
    /// Create a filter with English and Russian loaded.
    #[wasm_bindgen(constructor)]
    pub fn new(
        normalize_text: Option<bool>,
        aggressive_normalize: Option<bool>,
        transliterate: Option<bool>,
        replace_homoglyphs: Option<bool>,
    ) -> Result<ProfanityFilter, JsValue> {
        let processing = Processing {
            normalize_text: normalize_text.unwrap_or(true),
            aggressive_normalize: aggressive_normalize.unwrap_or(true),
            transliterate: transliterate.unwrap_or(true),
            replace_homoglyphs: replace_homoglyphs.unwrap_or(true),
        };
        let inner = CoreFilter::builder()
            .embedded()
            .processing(processing)
            .all_languages()
            .build()
            .map_err(to_js)?;

        Ok(ProfanityFilter {
            inner,
            options: MatchOptions::default(),
        })
    }

    /// Set the options used when a call does not pass its own.
    #[wasm_bindgen(js_name = setOptions)]
    pub fn set_options(&mut self, options: Option<js_sys::Object>) -> Result<(), JsValue> {
        self.options = parse_options(options)?;
        Ok(())
    }

    /// The options used when a call does not pass its own.
    #[wasm_bindgen(js_name = getOptions, unchecked_return_type = "MatchOptions")]
    pub fn get_options(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.options).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // -- matching -------------------------------------------------------------

    /// Whether the text contains profanity.
    #[wasm_bindgen(js_name = isProfane)]
    pub fn is_profane(&self, text: &str, options: Option<js_sys::Object>) -> Result<bool, JsValue> {
        Ok(self.inner.is_profane(text, self.resolve(options)?))
    }

    /// Every match, sorted by position and non-overlapping.
    #[wasm_bindgen(unchecked_return_type = "Match[]")]
    pub fn find(&self, text: &str, options: Option<js_sys::Object>) -> Result<JsValue, JsValue> {
        let matches: Vec<MatchOut> = self
            .inner
            .find(text, self.resolve(options)?)
            .into_iter()
            .map(MatchOut::from)
            .collect();
        // `language: null` rather than a missing key, so that consumers can
        // rely on the shape.
        matches
            .serialize(&serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true))
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Replace every match with `replaceChar`, keeping everything else.
    ///
    /// Punctuation attached to a word survives: `censor("hey shit, ok", "*")`
    /// gives `"hey ****, ok"`.
    pub fn censor(
        &self,
        text: &str,
        replace_char: &str,
        options: Option<js_sys::Object>,
    ) -> Result<String, JsValue> {
        let ch = replace_char.chars().next().unwrap_or('*');
        Ok(self.inner.censor(text, ch, self.resolve(options)?))
    }

    // -- dictionary -----------------------------------------------------------

    /// Add words on top of the loaded languages.
    #[wasm_bindgen(js_name = addWords)]
    pub fn add_words(&mut self, words: Vec<String>) {
        self.inner.add_words(&words);
    }

    /// Add every non-empty line of `text` as a word.
    ///
    /// Takes a whole word list in one call, rather than marshalling an array
    /// across the JavaScript boundary one string at a time.
    #[wasm_bindgen(js_name = addWordsFromText)]
    pub fn add_words_from_text(&mut self, text: &str) {
        let words: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        self.inner.add_words(&words);
    }

    /// Remove words. A word a loaded language also provides stays.
    #[wasm_bindgen(js_name = removeWords)]
    pub fn remove_words(&mut self, words: Vec<String>) {
        self.inner.remove_words(&words);
    }

    /// Drop every word, including those from loaded languages.
    #[wasm_bindgen(js_name = clearWords)]
    pub fn clear_words(&mut self) {
        self.inner.clear_words();
    }

    /// Number of distinct entries.
    #[wasm_bindgen(js_name = wordCount)]
    #[must_use]
    pub fn word_count(&self) -> usize {
        self.inner.word_count()
    }

    // -- whitelist ------------------------------------------------------------

    /// Never report these words, even when a rule would match them.
    #[wasm_bindgen(js_name = addWhitelist)]
    pub fn add_whitelist(&mut self, words: Vec<String>) {
        self.inner.add_whitelist(&words);
    }

    /// Drop words from the whitelist.
    #[wasm_bindgen(js_name = removeWhitelist)]
    pub fn remove_whitelist(&mut self, words: Vec<String>) {
        self.inner.remove_whitelist(&words);
    }

    /// Empty the whitelist.
    #[wasm_bindgen(js_name = clearWhitelist)]
    pub fn clear_whitelist(&mut self) {
        self.inner.clear_whitelist();
    }

    // -- languages ------------------------------------------------------------

    /// Languages compiled into this build.
    #[wasm_bindgen(js_name = loadedLanguages)]
    #[must_use]
    pub fn loaded_languages(&self) -> Vec<String> {
        self.inner.loaded_languages().to_vec()
    }

    /// Languages this build could load.
    #[wasm_bindgen(js_name = availableLanguages)]
    #[must_use]
    pub fn available_languages(&self) -> Vec<String> {
        self.inner.available_languages().to_vec()
    }

    // -- utility --------------------------------------------------------------

    /// Jaro-Winkler similarity of two strings.
    #[must_use]
    pub fn similar(&self, a: &str, b: &str) -> f64 {
        self.inner.similar(a, b)
    }

    /// The normalized form of a text, as the matcher sees it.
    #[must_use]
    pub fn normalize(&self, text: &str) -> String {
        self.inner.normalize(text)
    }

    // -- deprecated -----------------------------------------------------------

    /// Check or censor text.
    ///
    /// @deprecated since 3.0.0 - use `isProfane`, `censor` or `find`. The
    /// return type depends on the arguments, and censoring replaces the whole
    /// whitespace-delimited token including attached punctuation.
    #[wasm_bindgen(js_name = filterText)]
    #[allow(deprecated)]
    #[must_use]
    pub fn filter_text(
        &self,
        text: &str,
        replace_char: Option<String>,
        match_threshold: Option<f64>,
    ) -> JsValue {
        let threshold = match_threshold.unwrap_or(1.0);
        let ch = replace_char.and_then(|s| s.chars().next());
        let (found, censored) = self.inner.filter_text(text, threshold, ch);

        match (found, censored) {
            (true, Some(text)) => JsValue::from_str(&text),
            (true, None) => JsValue::from_bool(true),
            (false, _) => JsValue::from_bool(false),
        }
    }

    /// Whether the text contains profanity.
    ///
    /// @deprecated since 3.0.0 - use `isProfane`.
    #[wasm_bindgen(js_name = isBad)]
    #[must_use]
    pub fn is_bad(&self, text: &str, match_threshold: Option<f64>) -> bool {
        let options = MatchOptions {
            match_threshold: match_threshold.unwrap_or(1.0),
            ..MatchOptions::default()
        };
        self.inner.is_profane(text, options.into())
    }

    /// Loaded languages.
    ///
    /// @deprecated since 3.0.0 - use `loadedLanguages` or `availableLanguages`.
    #[wasm_bindgen(js_name = getLanguages)]
    #[must_use]
    pub fn get_languages(&self) -> Vec<String> {
        self.loaded_languages()
    }
}

impl ProfanityFilter {
    fn resolve(&self, options: Option<js_sys::Object>) -> Result<CoreOptions, JsValue> {
        match options {
            None => Ok(self.options.into()),
            Some(object) => Ok(parse_options(Some(object))?.into()),
        }
    }
}

/// Field names accepted in an options object.
const OPTION_KEYS: &[&str] = &[
    "matchThreshold",
    "splitOnPunctuation",
    "collapseRepeats",
    "leetspeak",
    "phrases",
    "substring",
    "minSubstringLen",
    "maxMatches",
];

fn parse_options(options: Option<js_sys::Object>) -> Result<MatchOptions, JsValue> {
    let Some(object) = options else {
        return Ok(MatchOptions::default());
    };

    // serde-wasm-bindgen ignores unknown keys, which turns a typo into silently
    // wrong behaviour. Reject them explicitly.
    for key in js_sys::Object::keys(&object).iter() {
        let Some(name) = key.as_string() else {
            continue;
        };
        if !OPTION_KEYS.contains(&name.as_str()) {
            return Err(JsValue::from_str(&format!(
                "unknown option {name:?}; expected one of {}",
                OPTION_KEYS.join(", ")
            )));
        }
    }

    serde_wasm_bindgen::from_value(object.into())
        .map_err(|e| JsValue::from_str(&format!("invalid options: {e}")))
}

fn to_js(err: badwords_core::Error) -> JsValue {
    JsValue::from_str(&err.to_string())
}

#[cfg(test)]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn filter() -> ProfanityFilter {
        ProfanityFilter::new(None, None, None, None).unwrap()
    }

    #[wasm_bindgen_test]
    fn clean_text_is_not_profane() {
        assert!(!filter().is_profane("hello world", None).unwrap());
    }

    #[wasm_bindgen_test]
    fn profane_text_is_detected() {
        assert!(filter().is_profane("sonofabitch", None).unwrap());
    }

    #[wasm_bindgen_test]
    fn added_words_are_detected() {
        let mut f = filter();
        f.add_words(vec!["custombad".to_string()]);
        assert!(f.is_profane("custombad", None).unwrap());
    }

    #[wasm_bindgen_test]
    fn words_can_be_added_from_text() {
        let mut f = filter();
        f.add_words_from_text("first\n\nsecond\n");
        assert!(f.is_profane("first", None).unwrap());
        assert!(f.is_profane("second", None).unwrap());
    }

    #[wasm_bindgen_test]
    fn censoring_keeps_punctuation() {
        let mut f = filter();
        f.add_words(vec!["bad".to_string()]);
        assert_eq!(f.censor("a bad, word", "*", None).unwrap(), "a ***, word");
    }

    #[wasm_bindgen_test]
    fn find_reports_positions() {
        let mut f = filter();
        f.add_words(vec!["bad".to_string()]);
        // Read the JavaScript object back field by field: this checks the
        // shape callers actually see, camelCase keys included.
        let value = f.find("a bad word", None).unwrap();
        let array: js_sys::Array = value.into();
        assert_eq!(array.length(), 1);

        let first = array.get(0);
        let field = |name: &str| js_sys::Reflect::get(&first, &name.into()).unwrap();
        assert_eq!(field("matchedText").as_string().unwrap(), "bad");
        assert_eq!(field("start").as_f64().unwrap(), 2.0);
        assert_eq!(field("kind").as_string().unwrap(), "exact");
        assert_eq!(field("word").as_string().unwrap(), "bad");
        assert!(field("language").is_null());
    }

    #[wasm_bindgen_test]
    fn whitelist_suppresses_a_match() {
        let mut f = filter();
        f.add_words(vec!["bad".to_string()]);
        f.add_whitelist(vec!["bad".to_string()]);
        assert!(!f.is_profane("a bad word", None).unwrap());
    }

    #[wasm_bindgen_test]
    fn options_enable_evasion_detection() {
        let mut f = filter();
        f.add_words(vec!["badword".to_string()]);
        let options: js_sys::Object = serde_wasm_bindgen::to_value(&MatchOptions {
            collapse_repeats: true,
            ..MatchOptions::default()
        })
        .unwrap()
        .into();

        assert!(!f.is_profane("baaadword", None).unwrap());
        assert!(f.is_profane("baaadword", Some(options.clone())).unwrap());
        // A plain object is reusable. A #[wasm_bindgen] struct would have been
        // moved into Rust by the first call and freed.
        assert!(f.is_profane("baaadword", Some(options)).unwrap());
    }

    #[wasm_bindgen_test]
    fn options_reject_unknown_fields() {
        let f = filter();
        let bad = js_sys::Object::new();
        js_sys::Reflect::set(&bad, &"nosuchoption".into(), &true.into()).unwrap();
        assert!(f.is_profane("text", Some(bad)).is_err());
    }

    #[wasm_bindgen_test]
    fn languages_are_reported() {
        let f = filter();
        assert!(f.loaded_languages().len() >= 2);
        assert!(f.available_languages().contains(&"en".to_string()));
    }

    #[wasm_bindgen_test]
    fn deprecated_api_still_works() {
        let mut f = filter();
        f.add_words(vec!["bad".to_string()]);
        assert!(f.is_bad("a bad word", None));
        assert_eq!(f.get_languages(), f.loaded_languages());
        // 2.x censored the whole token, punctuation included.
        assert_eq!(
            f.filter_text("a bad, word", Some("*".to_string()), None)
                .as_string()
                .unwrap(),
            "a **** word"
        );
    }
}
