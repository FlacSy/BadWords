//! Finding a model directory on disk.
//!
//! The same places the Python package looks, minus the download: fetching a
//! 200 MB asset over the network is the Python side's job, and this crate
//! stays free of an HTTP client. Point `BADWORDS_ML_PATH` at a directory, or
//! let `badwords.ml.download_model()` populate the shared cache.

use std::path::{Path, PathBuf};

use crate::error::Error;

/// Files a usable model directory must contain.
pub const REQUIRED_FILES: [&str; 4] = [
    "model.onnx",
    "config.json",
    "tokenizer.json",
    "tokenizer_config.json",
];

/// Whether a directory holds every file the model needs.
pub fn is_complete(dir: impl AsRef<Path>) -> bool {
    let dir = dir.as_ref();
    REQUIRED_FILES.iter().all(|name| dir.join(name).is_file())
}

/// The cache directory the Python package downloads into.
pub fn cache_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("badwords").join("ml"))
}

/// Locate a model: `BADWORDS_ML_PATH` first, then the shared cache.
pub fn locate_model() -> Result<PathBuf, Error> {
    if let Some(override_path) = std::env::var_os("BADWORDS_ML_PATH") {
        let path = PathBuf::from(override_path);
        if is_complete(&path) {
            return Ok(path);
        }
        let missing = REQUIRED_FILES
            .iter()
            .find(|name| !path.join(name).is_file())
            .unwrap_or(&REQUIRED_FILES[0]);
        return Err(Error::MissingFile {
            path: path.join(missing),
        });
    }

    let cached = cache_dir()
        .map(|dir| dir.join("model"))
        .ok_or(Error::MissingFile {
            path: PathBuf::from("$XDG_CACHE_HOME/badwords/ml/model"),
        })?;
    if is_complete(&cached) {
        return Ok(cached);
    }
    Err(Error::MissingFile {
        path: cached.join(REQUIRED_FILES[0]),
    })
}
