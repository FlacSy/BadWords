//! PyO3 bindings.
//!
//! Thin layer: options arrive as primitives, matches leave as tuples, and the
//! GIL is released around anything that touches the matcher.

// pyo3 0.22's macros expand to a `cfg(feature = "gil-refs")` check the crate
// does not declare, and to an `Into<PyErr>` that clippy reads as redundant.
#![allow(unexpected_cfgs, clippy::useless_conversion)]

use std::sync::Arc;

use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::IntoPyObjectExt;

use badwords_core::{
    Error, LanguageWarning, MatchMode, Options, Processing, ProfanityFilter, ResourceSource,
};

create_exception!(
    _native,
    NotSupportedLanguage,
    PyValueError,
    "Raised when a language code is neither a known language nor a known alias."
);

/// `(word, matched_text, start, end, language, score, kind)`
type MatchTuple = (String, String, usize, usize, Option<String>, f64, String);

fn to_py_err(err: Error) -> PyErr {
    match err {
        Error::UnknownLanguage { .. } => NotSupportedLanguage::new_err(err.to_string()),
        other => PyRuntimeError::new_err(other.to_string()),
    }
}

#[allow(clippy::too_many_arguments)]
fn options(
    match_threshold: f64,
    match_mode: &str,
    split_on_punctuation: bool,
    collapse_repeats: bool,
    leetspeak: bool,
    phrases: bool,
    min_substring_len: usize,
    max_matches: Option<usize>,
) -> PyResult<Options> {
    let mode = match match_mode {
        "token" => MatchMode::Token,
        "substring" => MatchMode::Substring,
        other => {
            return Err(PyValueError::new_err(format!(
                "match_mode must be 'token' or 'substring', got {other:?}"
            )))
        }
    };
    Ok(Options::new()
        .threshold(match_threshold)
        .match_mode(mode)
        .split_on_punctuation(split_on_punctuation)
        .collapse_repeats(collapse_repeats)
        .leetspeak(leetspeak)
        .phrases(phrases)
        .min_substring_len(min_substring_len)
        .max_matches(max_matches))
}

fn kind_name(kind: badwords_core::MatchKind) -> String {
    use badwords_core::MatchKind::{Collapsed, Exact, Fuzzy, Leet, Phrase, Substring};
    match kind {
        Exact => "exact",
        Fuzzy => "fuzzy",
        Leet => "leet",
        Collapsed => "collapsed",
        Substring => "substring",
        Phrase => "phrase",
        _ => "unknown",
    }
    .to_string()
}

fn to_tuples(matches: Vec<badwords_core::Match>) -> Vec<MatchTuple> {
    matches
        .into_iter()
        .map(|m| {
            (
                m.word,
                m.matched_text,
                m.start,
                m.end,
                m.language,
                m.score,
                kind_name(m.kind),
            )
        })
        .collect()
}

#[pyclass(name = "ProfanityFilter", module = "badwords._native")]
struct PyProfanityFilter {
    inner: Arc<ProfanityFilter>,
}

impl PyProfanityFilter {
    /// The filter is shared with `Arc` so that mutation can rebuild it while a
    /// matching call may still hold a reference.
    fn get_mut(&mut self) -> PyResult<&mut ProfanityFilter> {
        Arc::get_mut(&mut self.inner).ok_or_else(|| {
            PyRuntimeError::new_err("filter is in use by another thread; retry the mutation")
        })
    }
}

#[pymethods]
impl PyProfanityFilter {
    /// `resource_dir=None` uses the word lists compiled into the extension.
    #[new]
    #[pyo3(signature = (
        resource_dir=None,
        normalize_text=true,
        aggressive_normalize=true,
        transliterate=true,
        replace_homoglyphs=true,
    ))]
    fn new(
        resource_dir: Option<&str>,
        normalize_text: bool,
        aggressive_normalize: bool,
        transliterate: bool,
        replace_homoglyphs: bool,
    ) -> PyResult<Self> {
        let processing = Processing {
            normalize_text,
            aggressive_normalize,
            transliterate,
            replace_homoglyphs,
        };
        let source = match resource_dir {
            Some(dir) => ResourceSource::directory(dir),
            None => ResourceSource::Embedded,
        };
        let filter = ProfanityFilter::from_source(source, processing).map_err(to_py_err)?;
        Ok(Self {
            inner: Arc::new(filter),
        })
    }

    // -- languages ------------------------------------------------------------

    #[pyo3(signature = (languages=None))]
    fn load_languages(&mut self, languages: Option<&Bound<'_, PyList>>) -> PyResult<()> {
        let filter = self.get_mut()?;
        match languages {
            None => filter.load_all_languages().map_err(to_py_err),
            Some(list) => {
                let codes: Vec<String> = list.extract()?;
                filter.load_languages(&codes).map_err(to_py_err)
            }
        }
    }

    #[pyo3(signature = (languages=None))]
    fn reload_languages(&mut self, languages: Option<&Bound<'_, PyList>>) -> PyResult<()> {
        let filter = self.get_mut()?;
        match languages {
            None => filter.load_all_languages().map_err(to_py_err),
            Some(list) => {
                let codes: Vec<String> = list.extract()?;
                filter.reload_languages(&codes).map_err(to_py_err)
            }
        }
    }

    fn unload_languages(&mut self, languages: Vec<String>) -> PyResult<()> {
        self.get_mut()?
            .unload_languages(&languages)
            .map_err(to_py_err)
    }

    fn available_languages(&self) -> Vec<String> {
        self.inner.available_languages().to_vec()
    }

    fn loaded_languages(&self) -> Vec<String> {
        self.inner.loaded_languages().to_vec()
    }

    fn resolve_language(&self, code: &str) -> PyResult<String> {
        self.inner
            .resolve_language(code)
            .map(str::to_owned)
            .map_err(to_py_err)
    }

    /// `(kind, message)` for each diagnostic collected so far.
    fn take_warnings(&mut self) -> PyResult<Vec<(String, String)>> {
        let filter = self.get_mut()?;
        let warnings = filter
            .warnings()
            .iter()
            .map(|w| {
                let kind = match w {
                    LanguageWarning::DeprecatedAlias { .. } => "deprecated_alias",
                    LanguageWarning::AlreadyLoaded { .. } => "already_loaded",
                    LanguageWarning::EmptyWordList { .. } => "empty_word_list",
                    _ => "unknown",
                };
                (kind.to_string(), w.to_string())
            })
            .collect();
        filter.clear_warnings();
        Ok(warnings)
    }

    // -- dictionary -----------------------------------------------------------

    fn add_words(&mut self, words: Vec<String>) -> PyResult<()> {
        self.get_mut()?.add_words(&words);
        Ok(())
    }

    fn remove_words(&mut self, words: Vec<String>) -> PyResult<()> {
        self.get_mut()?.remove_words(&words);
        Ok(())
    }

    fn clear_words(&mut self) -> PyResult<()> {
        self.get_mut()?.clear_words();
        Ok(())
    }

    fn add_whitelist(&mut self, words: Vec<String>) -> PyResult<()> {
        self.get_mut()?.add_whitelist(&words);
        Ok(())
    }

    fn remove_whitelist(&mut self, words: Vec<String>) -> PyResult<()> {
        self.get_mut()?.remove_whitelist(&words);
        Ok(())
    }

    fn clear_whitelist(&mut self) -> PyResult<()> {
        self.get_mut()?.clear_whitelist();
        Ok(())
    }

    fn is_whitelisted(&self, word: &str) -> bool {
        self.inner.is_whitelisted(word)
    }

    fn word_count(&self) -> usize {
        self.inner.word_count()
    }

    fn contains_word(&self, word: &str) -> bool {
        self.inner.contains_word(word)
    }

    // -- matching -------------------------------------------------------------

    #[pyo3(signature = (text, *, match_threshold=1.0, match_mode="token",
        split_on_punctuation=false, collapse_repeats=false, leetspeak=false,
        phrases=true, min_substring_len=6, max_matches=None))]
    #[allow(clippy::too_many_arguments)]
    fn is_profane(
        &self,
        py: Python<'_>,
        text: &str,
        match_threshold: f64,
        match_mode: &str,
        split_on_punctuation: bool,
        collapse_repeats: bool,
        leetspeak: bool,
        phrases: bool,
        min_substring_len: usize,
        max_matches: Option<usize>,
    ) -> PyResult<bool> {
        let opts = options(
            match_threshold,
            match_mode,
            split_on_punctuation,
            collapse_repeats,
            leetspeak,
            phrases,
            min_substring_len,
            max_matches,
        )?;
        let filter = Arc::clone(&self.inner);
        Ok(py.detach(move || filter.is_profane(text, opts)))
    }

    #[pyo3(signature = (text, *, match_threshold=1.0, match_mode="token",
        split_on_punctuation=false, collapse_repeats=false, leetspeak=false,
        phrases=true, min_substring_len=6, max_matches=None))]
    #[allow(clippy::too_many_arguments)]
    fn find(
        &self,
        py: Python<'_>,
        text: &str,
        match_threshold: f64,
        match_mode: &str,
        split_on_punctuation: bool,
        collapse_repeats: bool,
        leetspeak: bool,
        phrases: bool,
        min_substring_len: usize,
        max_matches: Option<usize>,
    ) -> PyResult<Vec<MatchTuple>> {
        let opts = options(
            match_threshold,
            match_mode,
            split_on_punctuation,
            collapse_repeats,
            leetspeak,
            phrases,
            min_substring_len,
            max_matches,
        )?;
        let filter = Arc::clone(&self.inner);
        Ok(py.detach(move || to_tuples(filter.find(text, opts))))
    }

    #[pyo3(signature = (text, replace_character, *, match_threshold=1.0, match_mode="token",
        split_on_punctuation=false, collapse_repeats=false, leetspeak=false,
        phrases=true, min_substring_len=6, max_matches=None))]
    #[allow(clippy::too_many_arguments)]
    fn censor(
        &self,
        py: Python<'_>,
        text: &str,
        replace_character: &str,
        match_threshold: f64,
        match_mode: &str,
        split_on_punctuation: bool,
        collapse_repeats: bool,
        leetspeak: bool,
        phrases: bool,
        min_substring_len: usize,
        max_matches: Option<usize>,
    ) -> PyResult<String> {
        let ch = replace_character
            .chars()
            .next()
            .ok_or_else(|| PyValueError::new_err("replace_character must be a non-empty string"))?;
        let opts = options(
            match_threshold,
            match_mode,
            split_on_punctuation,
            collapse_repeats,
            leetspeak,
            phrases,
            min_substring_len,
            max_matches,
        )?;
        let filter = Arc::clone(&self.inner);
        Ok(py.detach(move || filter.censor(text, ch, opts)))
    }

    #[pyo3(signature = (texts, *, match_threshold=1.0, match_mode="token",
        split_on_punctuation=false, collapse_repeats=false, leetspeak=false,
        phrases=true, min_substring_len=6, max_matches=None))]
    #[allow(clippy::too_many_arguments)]
    fn is_profane_many(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
        match_threshold: f64,
        match_mode: &str,
        split_on_punctuation: bool,
        collapse_repeats: bool,
        leetspeak: bool,
        phrases: bool,
        min_substring_len: usize,
        max_matches: Option<usize>,
    ) -> PyResult<Vec<bool>> {
        let opts = options(
            match_threshold,
            match_mode,
            split_on_punctuation,
            collapse_repeats,
            leetspeak,
            phrases,
            min_substring_len,
            max_matches,
        )?;
        let filter = Arc::clone(&self.inner);
        Ok(py.detach(move || filter.is_profane_many(&texts, opts)))
    }

    #[pyo3(signature = (texts, *, match_threshold=1.0, match_mode="token",
        split_on_punctuation=false, collapse_repeats=false, leetspeak=false,
        phrases=true, min_substring_len=6, max_matches=None))]
    #[allow(clippy::too_many_arguments)]
    fn find_many(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
        match_threshold: f64,
        match_mode: &str,
        split_on_punctuation: bool,
        collapse_repeats: bool,
        leetspeak: bool,
        phrases: bool,
        min_substring_len: usize,
        max_matches: Option<usize>,
    ) -> PyResult<Vec<Vec<MatchTuple>>> {
        let opts = options(
            match_threshold,
            match_mode,
            split_on_punctuation,
            collapse_repeats,
            leetspeak,
            phrases,
            min_substring_len,
            max_matches,
        )?;
        let filter = Arc::clone(&self.inner);
        Ok(py.detach(move || {
            filter
                .find_many(&texts, opts)
                .into_iter()
                .map(to_tuples)
                .collect()
        }))
    }

    #[pyo3(signature = (texts, replace_character, *, match_threshold=1.0, match_mode="token",
        split_on_punctuation=false, collapse_repeats=false, leetspeak=false,
        phrases=true, min_substring_len=6, max_matches=None))]
    #[allow(clippy::too_many_arguments)]
    fn censor_many(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
        replace_character: &str,
        match_threshold: f64,
        match_mode: &str,
        split_on_punctuation: bool,
        collapse_repeats: bool,
        leetspeak: bool,
        phrases: bool,
        min_substring_len: usize,
        max_matches: Option<usize>,
    ) -> PyResult<Vec<String>> {
        let ch = replace_character
            .chars()
            .next()
            .ok_or_else(|| PyValueError::new_err("replace_character must be a non-empty string"))?;
        let opts = options(
            match_threshold,
            match_mode,
            split_on_punctuation,
            collapse_repeats,
            leetspeak,
            phrases,
            min_substring_len,
            max_matches,
        )?;
        let filter = Arc::clone(&self.inner);
        Ok(py.detach(move || filter.censor_many(&texts, ch, opts)))
    }

    /// The 2.x entry point, kept byte-identical through the core's compat layer.
    #[pyo3(signature = (text, match_threshold=1.0, replace_character=None))]
    #[allow(deprecated)]
    fn filter_text(
        &self,
        py: Python<'_>,
        text: &str,
        match_threshold: f64,
        replace_character: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let replace_char = replace_character.and_then(|s| s.chars().next());
        let filter = Arc::clone(&self.inner);
        let (found, censored) =
            py.detach(move || filter.filter_text(text, match_threshold, replace_char));

        // Still a bool-or-str union, because that is what 2.x returned and the
        // golden fixture holds it to that.
        if replace_character.is_some() {
            match (found, censored) {
                (true, Some(text)) => text.into_py_any(py),
                (true, None) => text.into_py_any(py),
                (false, _) => false.into_py_any(py),
            }
        } else {
            found.into_py_any(py)
        }
    }

    // -- utility --------------------------------------------------------------

    fn similar(&self, a: &str, b: &str) -> f64 {
        self.inner.similar(a, b)
    }

    fn normalize(&self, text: &str) -> String {
        self.inner.normalize(text)
    }
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyProfanityFilter>()?;
    m.add(
        "NotSupportedLanguage",
        m.py().get_type::<NotSupportedLanguage>(),
    )?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
