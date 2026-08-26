//! Where a filter reads its word lists and normalization tables from.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::error::Error;

/// Name of a normalization table, as stored under `resources/data/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataFile {
    UnicodeMappings,
    Homoglyphs,
    Transliteration,
    Languages,
}

impl DataFile {
    #[cfg(feature = "fs-resources")]
    pub(crate) fn file_name(self) -> &'static str {
        match self {
            Self::UnicodeMappings => "unicode_mappings.json",
            Self::Homoglyphs => "homoglyphs.json",
            Self::Transliteration => "transliteration.json",
            Self::Languages => "languages.json",
        }
    }
}

/// Where resources come from.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ResourceSource {
    /// A directory laid out like `resources/`: `data/*.json` and `words/*.txt`.
    #[cfg(feature = "fs-resources")]
    Directory(std::path::PathBuf),
    /// The tables compiled into the binary.
    #[cfg(feature = "embedded-data")]
    Embedded,
    /// Caller-supplied contents. Use for bundlers and tests.
    InMemory {
        unicode_mappings: String,
        homoglyphs: String,
        transliteration: String,
        languages: Option<String>,
        /// Keyed by language code or by file name.
        words: HashMap<String, String>,
    },
}

impl ResourceSource {
    /// A directory of resources.
    #[cfg(feature = "fs-resources")]
    #[must_use]
    pub fn directory(path: impl Into<std::path::PathBuf>) -> Self {
        Self::Directory(path.into())
    }

    /// Read one normalization table.
    ///
    /// `Languages` is optional everywhere: a directory without it falls back to
    /// discovering codes from file stems.
    pub(crate) fn data(&self, which: DataFile) -> Result<Option<Cow<'_, str>>, Error> {
        match self {
            #[cfg(feature = "fs-resources")]
            Self::Directory(dir) => {
                let path = dir.join("data").join(which.file_name());
                match std::fs::read_to_string(&path) {
                    Ok(text) => Ok(Some(Cow::Owned(strip_bom(text)))),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        if which == DataFile::Languages {
                            Ok(None)
                        } else {
                            Err(Error::Io { path, source: e })
                        }
                    }
                    Err(e) => Err(Error::Io { path, source: e }),
                }
            }
            #[cfg(feature = "embedded-data")]
            Self::Embedded => Ok(Some(Cow::Borrowed(match which {
                DataFile::UnicodeMappings => crate::embedded::UNICODE_MAPPINGS,
                DataFile::Homoglyphs => crate::embedded::HOMOGLYPHS,
                DataFile::Transliteration => crate::embedded::TRANSLITERATION,
                DataFile::Languages => crate::embedded::LANGUAGES,
            }))),
            Self::InMemory {
                unicode_mappings,
                homoglyphs,
                transliteration,
                languages,
                ..
            } => Ok(match which {
                DataFile::UnicodeMappings => Some(Cow::Borrowed(unicode_mappings.as_str())),
                DataFile::Homoglyphs => Some(Cow::Borrowed(homoglyphs.as_str())),
                DataFile::Transliteration => Some(Cow::Borrowed(transliteration.as_str())),
                DataFile::Languages => languages.as_deref().map(Cow::Borrowed),
            }),
        }
    }

    /// Read one word list by file name, e.g. `pt_br.txt`.
    pub(crate) fn word_list(&self, file: &str) -> Result<Option<Cow<'_, str>>, Error> {
        match self {
            #[cfg(feature = "fs-resources")]
            Self::Directory(dir) => {
                let path = dir.join("words").join(file);
                match std::fs::read_to_string(&path) {
                    Ok(text) => Ok(Some(Cow::Owned(strip_bom(text)))),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(e) => Err(Error::Io { path, source: e }),
                }
            }
            #[cfg(feature = "embedded-data")]
            Self::Embedded => Ok(crate::embedded::word_list(file).map(Cow::Borrowed)),
            Self::InMemory { words, .. } => {
                let stem = file.strip_suffix(".txt").unwrap_or(file);
                Ok(words
                    .get(file)
                    .or_else(|| words.get(stem))
                    .map(|s| Cow::Borrowed(s.as_str())))
            }
        }
    }

    /// Word list file names this source can provide.
    pub(crate) fn word_list_files(&self) -> Vec<String> {
        let mut files = match self {
            #[cfg(feature = "fs-resources")]
            Self::Directory(dir) => list_txt_files(&dir.join("words")),
            #[cfg(feature = "embedded-data")]
            Self::Embedded => crate::embedded::word_list_files(),
            Self::InMemory { words, .. } => words
                .keys()
                .map(|k| {
                    if k.ends_with(".txt") {
                        k.clone()
                    } else {
                        format!("{k}.txt")
                    }
                })
                .collect(),
        };
        files.sort();
        files.dedup();
        files
    }
}

#[cfg(feature = "fs-resources")]
fn list_txt_files(dir: &std::path::Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension()? == "txt").then(|| path.file_name()?.to_str().map(str::to_owned))?
        })
        .collect()
}

/// Word lists are edited by hand in many editors; a leading BOM is common and
/// would otherwise end up glued to the first entry.
#[cfg(feature = "fs-resources")]
fn strip_bom(mut text: String) -> String {
    if text.starts_with('\u{feff}') {
        text.remove(0);
    }
    text
}
