//! BadWords WebAssembly - profanity filter for browser and Node.js.
//!
//! Resources come from `badwords-core`'s `embedded-words-min` feature, which
//! compiles in English and Russian only (~82 KB rather than ~250 KB).

use badwords_core::{Options, Processing, ProfanityFilter as CoreFilter};
use wasm_bindgen::prelude::*;

/// Profanity filter for JavaScript. Uses embedded English and Russian word lists.
#[wasm_bindgen]
pub struct ProfanityFilter {
    inner: CoreFilter,
}

#[wasm_bindgen]
impl ProfanityFilter {
    /// Create a new filter with English and Russian loaded.
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
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(ProfanityFilter { inner })
    }

    #[wasm_bindgen(js_name = filterText)]
    pub fn filter_text(
        &self,
        text: &str,
        replace_char: Option<String>,
        match_threshold: Option<f64>,
    ) -> JsValue {
        let opts = self.options(match_threshold);
        match replace_char.and_then(|s| s.chars().next()) {
            None => JsValue::from_bool(self.inner.is_profane(text, opts)),
            Some(ch) => {
                let matches = self.inner.find(text, opts);
                if matches.is_empty() {
                    JsValue::from_bool(false)
                } else {
                    JsValue::from_str(&self.inner.censor(text, ch, opts))
                }
            }
        }
    }

    #[wasm_bindgen(js_name = isBad)]
    pub fn is_bad(&self, text: &str, match_threshold: Option<f64>) -> bool {
        self.inner.is_profane(text, self.options(match_threshold))
    }

    #[wasm_bindgen(js_name = censor)]
    pub fn censor(&self, text: &str, replace_char: &str, match_threshold: Option<f64>) -> String {
        let ch = replace_char.chars().next().unwrap_or('*');
        self.inner.censor(text, ch, self.options(match_threshold))
    }

    #[wasm_bindgen(js_name = addWords)]
    pub fn add_words(&mut self, words: Vec<String>) {
        self.inner.add_words(&words);
    }

    #[wasm_bindgen(js_name = getLanguages)]
    pub fn get_languages(&self) -> Vec<JsValue> {
        self.inner
            .loaded_languages()
            .iter()
            .map(|s| JsValue::from_str(s))
            .collect()
    }
}

impl ProfanityFilter {
    fn options(&self, match_threshold: Option<f64>) -> Options {
        Options::new().threshold(match_threshold.unwrap_or(1.0))
    }
}

#[cfg(test)]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_clean_text_returns_false() {
        let filter = ProfanityFilter::new(None, None, None, None).unwrap();
        assert!(!filter.is_bad("hello world", None));
    }

    #[wasm_bindgen_test]
    fn test_bad_text_returns_true() {
        let filter = ProfanityFilter::new(None, None, None, None).unwrap();
        assert!(filter.is_bad("sonofabitch", None));
    }

    #[wasm_bindgen_test]
    fn test_add_words_detection() {
        let mut filter = ProfanityFilter::new(None, None, None, None).unwrap();
        filter.add_words(vec!["custombad".to_string()]);
        assert!(filter.is_bad("custombad", None));
    }

    #[wasm_bindgen_test]
    fn test_censor_replaces() {
        let mut filter = ProfanityFilter::new(None, None, None, None).unwrap();
        filter.add_words(vec!["bad".to_string()]);
        let result = filter.censor("a bad word", "*", None);
        assert!(result.contains("*"));
        assert!(!result.contains("bad"));
    }

    #[wasm_bindgen_test]
    fn test_get_languages() {
        let filter = ProfanityFilter::new(None, None, None, None).unwrap();
        let langs = filter.get_languages();
        assert!(langs.len() >= 2);
    }
}
