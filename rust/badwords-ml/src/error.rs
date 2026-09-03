//! Errors from loading or running the model.

use std::fmt;
use std::path::PathBuf;

/// Anything that can go wrong loading or running the model.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A file the model directory must contain is absent.
    MissingFile { path: PathBuf },
    /// A file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `config.json` could not be parsed.
    Config {
        path: PathBuf,
        source: serde_json::Error,
    },
    /// `config.json` does not say what the outputs mean.
    MissingLabels { path: PathBuf },
    /// The tokenizer could not be loaded or applied.
    Tokenizer(String),
    /// ONNX Runtime could not load or run the model.
    ///
    /// Carries the message rather than the runtime's own error type: `ort` is
    /// still a release candidate, and its error type should not be part of
    /// this crate's public API.
    Runtime(String),
    /// The model produced something other than one score per label.
    UnexpectedOutput { expected: usize, got: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFile { path } => {
                write!(
                    f,
                    "model directory is incomplete: `{}` is missing",
                    path.display()
                )
            }
            Self::Io { path, source } => write!(f, "cannot read `{}`: {source}", path.display()),
            Self::Config { path, source } => {
                write!(f, "cannot parse `{}`: {source}", path.display())
            }
            Self::MissingLabels { path } => write!(
                f,
                "`{}` has no id2label, so the outputs cannot be named",
                path.display()
            ),
            Self::Tokenizer(message) => write!(f, "tokenizer: {message}"),
            Self::Runtime(message) => write!(f, "onnxruntime: {message}"),
            Self::UnexpectedOutput { expected, got } => {
                write!(
                    f,
                    "model returned {got} scores per text, expected {expected}"
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Config { source, .. } => Some(source),
            _ => None,
        }
    }
}

// `ort` errors are generic over what the failed call would have returned, so
// this covers `Error<()>`, `Error<SessionBuilder>` and the rest in one impl.
impl<R> From<ort::Error<R>> for Error {
    fn from(source: ort::Error<R>) -> Self {
        Self::Runtime(source.to_string())
    }
}
