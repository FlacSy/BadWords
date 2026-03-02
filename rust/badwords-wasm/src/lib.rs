//! BadWords WebAssembly - profanity filter for browser and Node.js.
//!
//! Uses embedded resources (no filesystem). Exports ProfanityFilter to JavaScript.

use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use badwords_core::ProfanityFilter as CoreFilter;

// Embedded resources - from python/badwords/resource/
const UNICODE_MAPPINGS: &str = include_str!("../../../python/badwords/resource/data/unicode_mappings.json");
const HOMOGLYPHS: &str = include_str!("../../../python/badwords/resource/data/homoglyphs.json");
const TRANSLITERATION: &str = include_str!("../../../python/badwords/resource/data/transliteration.json");
const WORDS_EN: &str = include_str!("../../../python/badwords/resource/words/en.txt");
const WORDS_RU: &str = include_str!("../../../python/badwords/resource/words/ru.txt");

/// Profanity filter for JavaScript. Uses embedded English and Russian word lists.
#[wasm_bindgen]
pub struct ProfanityFilter {
    inner: CoreFilter,
}

#[wasm_bindgen]
impl ProfanityFilter {
    /// Create a new filter with English and Russian languages.
    #[wasm_bindgen(constructor)]
    pub fn new(
        normalize_text: Option<bool>,
        aggressive_normalize: Option<bool>,
        transliterate: Option<bool>,
        replace_homoglyphs: Option<bool>,
    ) -> Result<ProfanityFilter, JsValue> {
        let mut words_by_lang = HashMap::new();
        words_by_lang.insert("en".to_string(), WORDS_EN.to_string());
        words_by_lang.insert("ru".to_string(), WORDS_RU.to_string());

        let inner = CoreFilter::new_from_embedded(
            UNICODE_MAPPINGS,
            HOMOGLYPHS,
            TRANSLITERATION,
            &words_by_lang,
            vec!["en".to_string(), "ru".to_string()],
            normalize_text.unwrap_or(true),
            aggressive_normalize.unwrap_or(true),
            transliterate.unwrap_or(true),
            replace_homoglyphs.unwrap_or(true),
        )
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
        let threshold = match_threshold.unwrap_or(1.0);
        let ch = replace_char.and_then(|s| s.chars().next());
        let (found, replaced) = self.inner.filter_text(text, threshold, ch);

        if found {
            if let Some(r) = replaced {
                JsValue::from_str(&r)
            } else {
                JsValue::from_bool(true)
            }
        } else {
            JsValue::from_bool(false)
        }
    }

    #[wasm_bindgen(js_name = isBad)]
    pub fn is_bad(&self, text: &str, match_threshold: Option<f64>) -> bool {
        let threshold = match_threshold.unwrap_or(1.0);
        let (found, _) = self.inner.filter_text(text, threshold, None);
        found
    }

    #[wasm_bindgen(js_name = censor)]
    pub fn censor(&self, text: &str, replace_char: &str, match_threshold: Option<f64>) -> String {
        let threshold = match_threshold.unwrap_or(1.0);
        let ch = replace_char.chars().next().unwrap_or('*');
        let (_found, replaced) = self.inner.filter_text(text, threshold, Some(ch));
        replaced.unwrap_or_else(|| text.to_string())
    }

    #[wasm_bindgen(js_name = addWords)]
    pub fn add_words(&mut self, words: Vec<String>) {
        self.inner.add_words(&words);
    }

    #[wasm_bindgen(js_name = getLanguages)]
    pub fn get_languages(&self) -> Vec<JsValue> {
        self.inner
            .get_all_languages()
            .iter()
            .map(|s| JsValue::from_str(s))
            .collect()
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
