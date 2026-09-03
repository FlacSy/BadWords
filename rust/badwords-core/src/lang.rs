//! Language registry: canonical codes, human names and aliases.
//!
//! Codes are ISO 639-1 where one exists. The non-standard codes used before
//! 3.0.0 keep working as aliases; four of them are deprecated because they
//! collide with a different real language (see `resources/data/languages.json`).
//!
//! The registry is also what keeps caller-supplied strings out of file paths:
//! a word list is only ever opened through the `file` recorded here.

use std::collections::HashMap;

use serde::Deserialize;

use crate::error::Error;

#[derive(Debug, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    languages: HashMap<String, LanguageEntry>,
    #[serde(default)]
    aliases: HashMap<String, AliasEntry>,
}

#[derive(Debug, Deserialize)]
struct LanguageEntry {
    name: String,
    file: String,
}

#[derive(Debug, Deserialize)]
struct AliasEntry {
    target: String,
    #[serde(default)]
    deprecated: bool,
    #[serde(default)]
    note: String,
}

/// A language known to the filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInfo {
    /// Canonical code, e.g. `pt_br`.
    pub code: String,
    /// Human-readable name, e.g. `Portuguese (Brazil)`.
    pub name: String,
    /// Word list file name inside the resource directory.
    pub file: String,
}

/// Outcome of resolving a caller-supplied code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolved<'a> {
    /// The canonical code the input maps to.
    pub canonical: &'a str,
    /// Set when the input was a deprecated alias; explains what to use instead.
    pub deprecated: Option<&'a str>,
}

/// Canonical codes, their word list files, and the alias table.
#[derive(Debug, Clone, Default)]
pub struct LanguageRegistry {
    languages: Vec<LanguageInfo>,
    by_code: HashMap<String, usize>,
    aliases: HashMap<String, (String, Option<String>)>,
}

impl LanguageRegistry {
    /// Parse `languages.json`.
    pub fn parse(json: &str) -> Result<Self, Error> {
        let parsed: RegistryFile = serde_json::from_str(json).map_err(|source| Error::Json {
            resource: "languages.json".to_string(),
            source,
        })?;

        let mut languages: Vec<LanguageInfo> = parsed
            .languages
            .into_iter()
            .map(|(code, entry)| LanguageInfo {
                code,
                name: entry.name,
                file: entry.file,
            })
            .collect();
        languages.sort_by(|a, b| a.code.cmp(&b.code));

        let by_code = languages
            .iter()
            .enumerate()
            .map(|(i, l)| (l.code.clone(), i))
            .collect::<HashMap<_, _>>();

        let mut aliases = HashMap::new();
        for (alias, entry) in parsed.aliases {
            if !by_code.contains_key(&entry.target) {
                continue; // alias to a language that is not shipped: ignore
            }
            let note = entry.deprecated.then_some(entry.note);
            aliases.insert(alias, (entry.target, note));
        }

        Ok(Self {
            languages,
            by_code,
            aliases,
        })
    }

    /// Build a registry from bare file stems, for resource directories that
    /// ship no `languages.json`. Every stem is canonical and there are no aliases.
    pub fn from_codes<I, S>(codes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut languages: Vec<LanguageInfo> = codes
            .into_iter()
            .map(Into::into)
            .map(|code| LanguageInfo {
                name: code.clone(),
                file: format!("{code}.txt"),
                code,
            })
            .collect();
        languages.sort_by(|a, b| a.code.cmp(&b.code));
        languages.dedup_by(|a, b| a.code == b.code);

        let by_code = languages
            .iter()
            .enumerate()
            .map(|(i, l)| (l.code.clone(), i))
            .collect();

        Self {
            languages,
            by_code,
            aliases: HashMap::new(),
        }
    }

    /// Normalize a caller-supplied code: trim, lowercase, `-` to `_`.
    #[must_use]
    pub fn normalize_code(code: &str) -> String {
        code.trim().to_lowercase().replace('-', "_")
    }

    /// Resolve a code or alias to its canonical form.
    ///
    /// # Errors
    /// [`Error::UnknownLanguage`] if the code is neither a language nor an alias.
    pub fn resolve(&self, code: &str) -> Result<Resolved<'_>, Error> {
        let key = Self::normalize_code(code);

        if let Some(info) = self.by_code.get(&key).map(|&i| &self.languages[i]) {
            return Ok(Resolved {
                canonical: &info.code,
                deprecated: None,
            });
        }

        if let Some((target, note)) = self.aliases.get(&key) {
            let idx = self.by_code[target];
            return Ok(Resolved {
                canonical: &self.languages[idx].code,
                deprecated: note.as_deref(),
            });
        }

        Err(Error::UnknownLanguage {
            code: code.to_string(),
            available: self.languages.iter().map(|l| l.code.clone()).collect(),
        })
    }

    /// Look up a canonical code.
    #[must_use]
    pub fn info(&self, canonical: &str) -> Option<&LanguageInfo> {
        self.by_code.get(canonical).map(|&i| &self.languages[i])
    }

    /// All canonical codes, sorted.
    pub fn canonical_codes(&self) -> impl Iterator<Item = &str> {
        self.languages.iter().map(|l| l.code.as_str())
    }

    /// Every language, sorted by code.
    #[must_use]
    pub fn languages(&self) -> &[LanguageInfo] {
        &self.languages
    }

    /// Aliases pointing at a canonical code, sorted.
    pub fn aliases_for<'a>(&'a self, canonical: &'a str) -> impl Iterator<Item = &'a str> {
        let mut found: Vec<&str> = self
            .aliases
            .iter()
            .filter(|(_, (target, _))| target == canonical)
            .map(|(alias, _)| alias.as_str())
            .collect();
        found.sort_unstable();
        found.into_iter()
    }

    /// Restrict the registry to the languages actually present on disk.
    pub(crate) fn retain_files(&mut self, present: &[String]) {
        self.languages.retain(|l| present.contains(&l.file));
        self.by_code = self
            .languages
            .iter()
            .enumerate()
            .map(|(i, l)| (l.code.clone(), i))
            .collect();
        let live: Vec<String> = self.languages.iter().map(|l| l.code.clone()).collect();
        self.aliases.retain(|_, (target, _)| live.contains(target));
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }
}
