//! Error and diagnostic types.

use std::fmt;
use std::path::PathBuf;

/// Anything that can go wrong while building or configuring a filter.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The language code is neither a known language nor a known alias.
    UnknownLanguage {
        code: String,
        available: Vec<String>,
    },
    /// A resource file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A resource file could not be parsed.
    Json {
        resource: String,
        source: serde_json::Error,
    },
    /// A resource the filter cannot work without is absent.
    MissingResource { what: &'static str },
    /// The requested combination of options cannot be honoured.
    InvalidOptions(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownLanguage { code, available } => {
                write!(f, "unsupported language `{code}`")?;
                if !available.is_empty() {
                    write!(f, " (available: {})", available.join(", "))?;
                }
                Ok(())
            }
            Self::Io { path, source } => {
                write!(f, "cannot read `{}`: {source}", path.display())
            }
            Self::Json { resource, source } => {
                write!(f, "cannot parse `{resource}`: {source}")
            }
            Self::MissingResource { what } => write!(f, "missing resource: {what}"),
            Self::InvalidOptions(why) => write!(f, "invalid options: {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A non-fatal diagnostic raised while loading languages.
///
/// Collected on the filter and readable through
/// [`ProfanityFilter::warnings`](crate::ProfanityFilter::warnings) rather than
/// logged, so that callers on any platform - including WebAssembly - can
/// surface them however they like.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LanguageWarning {
    /// A deprecated alias was used. `note` explains why it is deprecated.
    DeprecatedAlias {
        requested: String,
        canonical: String,
        note: String,
    },
    /// The language was already loaded; the request was a no-op.
    AlreadyLoaded { code: String },
    /// The language loaded, but its word list is empty.
    EmptyWordList { code: String },
}

impl fmt::Display for LanguageWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeprecatedAlias {
                requested,
                canonical,
                note,
            } => write!(
                f,
                "language code `{requested}` is deprecated, use `{canonical}`: {note}"
            ),
            Self::AlreadyLoaded { code } => write!(f, "language `{code}` was already loaded"),
            Self::EmptyWordList { code } => write!(f, "language `{code}` has an empty word list"),
        }
    }
}

/// Legacy error type of the 2.x API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[deprecated(
    since = "3.0.0",
    note = "use `Error::UnknownLanguage`, which carries the offending code"
)]
pub struct NotSupportedLanguage;

#[allow(deprecated)]
impl fmt::Display for NotSupportedLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "This language is not supported")
    }
}

#[allow(deprecated)]
impl std::error::Error for NotSupportedLanguage {}

#[allow(deprecated)]
impl From<Error> for NotSupportedLanguage {
    fn from(_: Error) -> Self {
        Self
    }
}
