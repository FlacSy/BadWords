//! PyO3 bindings for Python.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;

use badwords_core::ProfanityFilter;

#[pyclass]
struct PyProfanityFilter {
    inner: ProfanityFilter,
}

#[pymethods]
impl PyProfanityFilter {
    #[new]
    #[pyo3(signature = (resource_dir, normalize_text=true, aggressive_normalize=true, transliterate=true, replace_homoglyphs=true))]
    fn new(
        resource_dir: &str,
        normalize_text: bool,
        aggressive_normalize: bool,
        transliterate: bool,
        replace_homoglyphs: bool,
    ) -> PyResult<Self> {
        let path = std::path::Path::new(resource_dir);
        let filter = ProfanityFilter::new(
            path,
            normalize_text,
            aggressive_normalize,
            transliterate,
            replace_homoglyphs,
        );
        Ok(Self { inner: filter })
    }

    #[pyo3(signature = (languages=None))]
    fn init(
        &mut self,
        languages: Option<&Bound<'_, PyList>>,
    ) -> PyResult<()> {
        let langs = languages.map(|list| {
            list.iter()
                .map(|o| o.extract::<String>())
                .collect::<Result<Vec<_>, _>>()
        });
        let langs = match langs {
            Some(Ok(l)) => Some(l),
            Some(Err(e)) => return Err(e),
            None => None,
        };
        self.inner
            .init(langs.as_deref())
            .map_err(|_| PyErr::new::<PyValueError, _>("This language is not supported"))?;
        Ok(())
    }

    fn add_words(&mut self, words: &Bound<'_, PyList>) -> PyResult<()> {
        let words: Vec<String> = words
            .iter()
            .map(|o| o.extract::<String>())
            .collect::<PyResult<_>>()?;
        self.inner.add_words(&words);
        Ok(())
    }

    fn similar(&self, a: &str, b: &str) -> f64 {
        self.inner.similar(a, b)
    }

    #[pyo3(signature = (text, match_threshold=1.0, replace_character=None))]
    fn filter_text(
        &self,
        text: &str,
        match_threshold: f64,
        replace_character: Option<&str>,
    ) -> PyResult<PyObject> {
        let threshold = if match_threshold == 0.0 {
            1.0
        } else {
            match_threshold
        };
        let replace_char = replace_character.and_then(|s| s.chars().next());

        let (found, replaced) = self.inner.filter_text(text, threshold, replace_char);

        Python::with_gil(|py| {
            if replace_character.is_some() {
                if found {
                    Ok(replaced
                        .unwrap_or_else(|| text.to_string())
                        .into_py(py))
                } else {
                    Ok(false.into_py(py))
                }
            } else {
                Ok(found.into_py(py))
            }
        })
    }

    fn get_all_languages(&self) -> Vec<String> {
        self.inner.get_all_languages().to_vec()
    }
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyProfanityFilter>()?;
    Ok(())
}
