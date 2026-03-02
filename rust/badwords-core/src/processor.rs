//! Text processing: normalization, transliteration, homoglyphs.

use std::collections::HashMap;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct TextProcessor {
    pub normalize_text: bool,
    pub aggressive_normalize: bool,
    pub transliterate: bool,
    pub replace_homoglyphs: bool,
    unicode_mappings: HashMap<char, char>,
    homoglyph_map: HashMap<char, char>,
    cyrillic_to_latin: HashMap<char, String>,
    latin_to_cyrillic: HashMap<String, char>,
}

#[derive(Debug, Deserialize)]
struct UnicodeMappingsFile {
    #[serde(flatten)]
    categories: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct HomoglyphsFile(HashMap<String, Vec<String>>);

#[derive(Debug, Deserialize)]
struct TransliterationFile {
    cyrillic_to_latin: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CharacterFrequencyFile(HashMap<String, Vec<String>>);

impl TextProcessor {
    pub fn new(
        normalize_text: bool,
        aggressive_normalize: bool,
        transliterate: bool,
        replace_homoglyphs: bool,
    ) -> Self {
        Self {
            normalize_text,
            aggressive_normalize,
            transliterate,
            replace_homoglyphs,
            unicode_mappings: HashMap::new(),
            homoglyph_map: HashMap::new(),
            cyrillic_to_latin: HashMap::new(),
            latin_to_cyrillic: HashMap::new(),
        }
    }

    pub fn load_from_dir(&mut self, data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let unicode_content = std::fs::read_to_string(data_dir.join("unicode_mappings.json"))?;
        let homoglyph_content = std::fs::read_to_string(data_dir.join("homoglyphs.json"))?;
        let translit_content = std::fs::read_to_string(data_dir.join("transliteration.json"))?;
        self.load_from_str(&unicode_content, &homoglyph_content, &translit_content)
    }

    /// Load processor data from string content (no filesystem). Use for WASM/embedded.
    pub fn load_from_str(
        &mut self,
        unicode_mappings_json: &str,
        homoglyphs_json: &str,
        transliteration_json: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.unicode_mappings = Self::parse_unicode_mappings(unicode_mappings_json)?;
        self.homoglyph_map = Self::parse_homoglyph_map(homoglyphs_json)?;

        if self.transliterate {
            let (cyrillic_to_latin, latin_to_cyrillic) =
                Self::parse_transliteration(transliteration_json)?;
            self.cyrillic_to_latin = cyrillic_to_latin;
            self.latin_to_cyrillic = latin_to_cyrillic;
        }

        Ok(())
    }

    fn parse_unicode_mappings(content: &str) -> Result<HashMap<char, char>, Box<dyn std::error::Error>> {
        let data: UnicodeMappingsFile = serde_json::from_str(content)?;

        let mut mappings = HashMap::new();
        for category in data.categories.values() {
            for (k, v) in category {
                if let (Some(from), Some(to)) = (k.chars().next(), v.chars().next()) {
                    mappings.insert(from, to);
                }
            }
        }
        Ok(mappings)
    }

    fn parse_homoglyph_map(content: &str) -> Result<HashMap<char, char>, Box<dyn std::error::Error>> {
        let data: HomoglyphsFile = serde_json::from_str(content)?;

        let mut map = HashMap::new();
        for (standard, variants) in data.0 {
            let standard_char = standard.chars().next();
            if let Some(std_c) = standard_char {
                for variant in variants {
                    if let Some(var_c) = variant.chars().next() {
                        map.insert(var_c, std_c);
                    }
                }
            }
        }
        Ok(map)
    }

    fn parse_transliteration(
        content: &str,
    ) -> Result<(HashMap<char, String>, HashMap<String, char>), Box<dyn std::error::Error>> {
        let data: TransliterationFile = serde_json::from_str(content)?;

        let mut cyrillic_to_latin = HashMap::new();
        let mut latin_to_cyrillic = HashMap::new();

        for (k, v) in data.cyrillic_to_latin {
            if let Some(cyr) = k.chars().next() {
                cyrillic_to_latin.insert(cyr, v.clone());
                latin_to_cyrillic.insert(v, cyr);
            }
        }
        Ok((cyrillic_to_latin, latin_to_cyrillic))
    }

    /// Single-pass normalization: unicode mappings + filter + collapse whitespace.
    /// Merges normalize_text and aggressive_normalize to avoid duplicate passes.
    fn normalize_unicode_and_filter(&self, text: &str, allow_underscore: bool) -> String {
        let text = text.nfkc().collect::<String>();
        let filtered: String = text
            .to_lowercase()
            .chars()
            .map(|c| *self.unicode_mappings.get(&c).unwrap_or(&c))
            .filter(|c| {
                c.is_alphanumeric()
                    || c.is_whitespace()
                    || (allow_underscore && *c == '_')
            })
            .collect();
        filtered.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn transliterate_char(&self, c: char, to_latin: bool) -> String {
        if to_latin {
            self.cyrillic_to_latin
                .get(&c)
                .cloned()
                .unwrap_or_else(|| c.to_string())
        } else {
            let mut s = String::new();
            s.push(c);
            self.latin_to_cyrillic
                .get(&s)
                .map(|&cyr| cyr.to_string())
                .unwrap_or_else(|| c.to_string())
        }
    }

    fn transliterate(&self, text: &str, to_latin: bool) -> String {
        if to_latin {
            text.chars()
                .map(|c| self.transliterate_char(c, true))
                .collect::<Vec<_>>()
                .join("")
        } else {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = text.chars().collect();
            while i < chars.len() {
                let mut matched = false;
                for len in (1..=4).rev() {
                    if i + len <= chars.len() {
                        let chunk: String = chars[i..i + len].iter().collect();
                        if let Some(&cyr) = self.latin_to_cyrillic.get(&chunk) {
                            result.push(cyr);
                            i += len;
                            matched = true;
                            break;
                        }
                    }
                }
                if !matched {
                    result.push(chars[i]);
                    i += 1;
                }
            }
            result
        }
    }

    fn replace_homoglyphs(&self, text: &str) -> String {
        text.chars()
            .map(|c| *self.homoglyph_map.get(&c).unwrap_or(&c))
            .collect()
    }

    #[inline]
    fn contains_cyrillic(text: &str) -> bool {
        text.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c))
    }

    pub fn process_text(&self, text: &str) -> String {
        let mut txt = if self.normalize_text || self.aggressive_normalize {
            let allow_underscore = self.normalize_text && !self.aggressive_normalize;
            self.normalize_unicode_and_filter(text, allow_underscore)
        } else {
            text.to_string()
        };

        if self.transliterate {
            if Self::contains_cyrillic(&txt) {
                txt = self.transliterate(&txt, true);
            }
            txt = self.transliterate(&txt, false);
        }
        if self.replace_homoglyphs {
            txt = self.replace_homoglyphs(&txt);
        }
        txt
    }
}
