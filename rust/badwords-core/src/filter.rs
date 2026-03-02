//! Profanity filter - main API.

use std::collections::HashMap;
use std::path::Path;

use rustc_hash::FxHashSet;
use strsim::jaro_winkler;

use crate::processor::TextProcessor;

#[derive(Debug)]
pub struct ProfanityFilter {
    words_dir: Option<std::path::PathBuf>,
    #[allow(dead_code)]
    data_dir: Option<std::path::PathBuf>,
    processor: TextProcessor,
    language_files: Vec<String>,
    bad_words: FxHashSet<String>,
}

#[derive(Debug)]
pub struct NotSupportedLanguage;

impl std::fmt::Display for NotSupportedLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "This language is not supported")
    }
}

impl std::error::Error for NotSupportedLanguage {}

impl ProfanityFilter {
    pub fn new(
        resource_dir: &Path,
        normalize_text: bool,
        aggressive_normalize: bool,
        transliterate: bool,
        replace_homoglyphs: bool,
    ) -> Self {
        let words_dir = resource_dir.join("words");
        let data_dir = resource_dir.join("data");

        let mut processor = TextProcessor::new(
            normalize_text,
            aggressive_normalize,
            transliterate,
            replace_homoglyphs,
        );
        processor.load_from_dir(&data_dir).ok();

        let language_files = Self::discover_languages(&words_dir);
        let bad_words = FxHashSet::default();

        Self {
            words_dir: Some(words_dir),
            data_dir: Some(data_dir),
            processor,
            language_files,
            bad_words,
        }
    }

    /// Create filter from embedded string data (no filesystem). Use for WASM/embedded.
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
        let mut processor = TextProcessor::new(
            normalize_text,
            aggressive_normalize,
            transliterate,
            replace_homoglyphs,
        );
        processor.load_from_str(
            unicode_mappings_json,
            homoglyphs_json,
            transliteration_json,
        )?;

        let bad_words = Self::load_bad_words_from_content(&processor, words_by_lang, &languages);

        Ok(Self {
            words_dir: None,
            data_dir: None,
            processor,
            language_files: languages,
            bad_words,
        })
    }

    fn load_bad_words_from_content(
        processor: &TextProcessor,
        words_by_lang: &HashMap<String, String>,
        languages: &[String],
    ) -> FxHashSet<String> {
        let mut bad_words = FxHashSet::default();
        for lang in languages {
            let lang = lang.to_lowercase().trim().to_string();
            if !lang.chars().all(|c| c.is_alphabetic()) {
                continue;
            }
            if let Some(content) = words_by_lang.get(&lang) {
                for line in content.lines() {
                    let processed = processor.process_text(line);
                    if !processed.is_empty() {
                        bad_words.insert(processed);
                    }
                }
            }
        }
        bad_words
    }

    pub fn init(
        &mut self,
        languages: Option<&[String]>,
    ) -> Result<(), NotSupportedLanguage> {
        if let Some(langs) = languages {
            let available: FxHashSet<_> = self.language_files.iter().collect();
            for lang in langs {
                if !available.contains(lang) {
                    return Err(NotSupportedLanguage);
                }
            }
            self.language_files = langs.to_vec();
        }

        if self.words_dir.is_some() {
            self.bad_words = self.load_bad_words();
        }
        Ok(())
    }

    fn discover_languages(words_dir: &Path) -> Vec<String> {
        let mut langs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(words_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "txt") {
                    if let Some(stem) = path.file_stem() {
                        if let Some(s) = stem.to_str() {
                            langs.push(s.to_string());
                        }
                    }
                }
            }
        }
        langs.sort();
        langs
    }

    fn load_bad_words(&self) -> FxHashSet<String> {
        let mut bad_words = FxHashSet::default();
        let words_dir = match &self.words_dir {
            Some(p) => p,
            None => return bad_words,
        };

        for lang in &self.language_files {
            let lang = lang.to_lowercase().trim().to_string();
            if !lang.chars().all(|c| c.is_alphabetic()) {
                continue;
            }

            let path = words_dir.join(format!("{}.txt", lang));
            if !path.exists() {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    let processed = self.processor.process_text(line);
                    if !processed.is_empty() {
                        bad_words.insert(processed);
                    }
                }
            }
        }

        bad_words
    }

    pub fn add_words(&mut self, words: &[String]) {
        for word in words {
            let processed = self.processor.process_text(word);
            if !processed.is_empty() {
                self.bad_words.insert(processed);
            }
        }
    }

    pub fn similar(&self, a: &str, b: &str) -> f64 {
        jaro_winkler(a, b) as f64
    }

    /// Returns: (found, optional_replaced_text)
    pub fn filter_text(
        &self,
        text: &str,
        match_threshold: f64,
        replace_char: Option<char>,
    ) -> (bool, Option<String>) {
        // Collect (original_word, byte_start, byte_end) for each word
        let mut word_spans: Vec<(&str, usize, usize)> = Vec::new();
        let mut start = 0;
        let mut in_word = false;
        for (i, c) in text.char_indices() {
            if c.is_whitespace() {
                if in_word {
                    word_spans.push((&text[start..i], start, i));
                    in_word = false;
                }
            } else if !in_word {
                start = i;
                in_word = true;
            }
        }
        if in_word {
            word_spans.push((&text[start..], start, text.len()));
        }

        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        for (original_word, byte_start, byte_end) in &word_spans {
            let processed = self.processor.process_text(original_word);
            if processed.is_empty() {
                continue;
            }

            let mut found = self.bad_words.contains(&processed);
            if !found && match_threshold < 1.0 && match_threshold > 0.0 {
                for bad in &self.bad_words {
                    if self.similar(&processed, bad) > match_threshold {
                        found = true;
                        break;
                    }
                }
            }

            if found {
                if let Some(ch) = replace_char {
                    let replacement: String =
                        std::iter::repeat(ch).take(original_word.chars().count()).collect();
                    replacements.push((*byte_start, *byte_end, replacement));
                } else {
                    return (true, None);
                }
            }
        }

        if replacements.is_empty() {
            return (false, None);
        }

        // Apply replacements from end to start to preserve indices
        replacements.sort_by_key(|(s, _, _)| std::cmp::Reverse(*s));
        let mut result = text.to_string();
        for (start, end, repl) in replacements {
            result.replace_range(start..end, &repl);
        }
        (true, Some(result))
    }

    pub fn get_all_languages(&self) -> &[String] {
        &self.language_files
    }
}
