//! Matching options.
//!
//! Every detector added in 3.0.0 is off by default: [`Options::default`]
//! reproduces 2.x behaviour except for phrase matching, which is inert in 2.x
//! because multi-word entries could never match at all.

/// How dictionary entries are looked for inside a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MatchMode {
    /// A token matches only as a whole. Default.
    #[default]
    Token,
    /// A token matches if a dictionary entry occurs anywhere inside it.
    ///
    /// Catches glued evasion (`fuckyou`) at the cost of Scunthorpe-style false
    /// positives; pair it with a whitelist and see [`Options::min_substring_len`].
    Substring,
}

/// Text normalization applied before matching, to both text and dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Processing {
    /// Apply NFKC, lowercase and the unicode confusable mappings.
    pub normalize_text: bool,
    /// Also drop underscores, not just punctuation.
    pub aggressive_normalize: bool,
    /// Fold latin and cyrillic onto one script.
    pub transliterate: bool,
    /// Fold cross-script lookalikes (cyrillic `с` to latin `c`).
    pub replace_homoglyphs: bool,
}

impl Default for Processing {
    fn default() -> Self {
        Self {
            normalize_text: true,
            aggressive_normalize: true,
            transliterate: true,
            replace_homoglyphs: true,
        }
    }
}

impl Processing {
    /// No normalization at all: entries match byte-for-byte.
    #[must_use]
    pub fn none() -> Self {
        Self {
            normalize_text: false,
            aggressive_normalize: false,
            transliterate: false,
            replace_homoglyphs: false,
        }
    }
}

/// Which part of a whitespace-separated segment a match reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SpanMode {
    /// The alphanumeric core, so surrounding punctuation survives censoring.
    #[default]
    Core,
    /// The whole segment including punctuation - 2.x behaviour.
    WholeSegment,
}

/// Per-call matching options.
///
/// Construct with [`Options::new`] (or `default()`) and set fields through the
/// builder methods; the struct is `#[non_exhaustive]` so that later releases can
/// add detectors without breaking callers.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Options {
    /// Similarity required for a fuzzy match. `1.0` means exact only.
    pub match_threshold: f64,
    /// Whole-token or substring lookup.
    pub match_mode: MatchMode,
    /// Also test the pieces a token splits into on inner punctuation, so that
    /// `fuck-you` and `you.fuck` are caught. The glued form is always tested
    /// first, so `f.u.c.k` keeps working either way.
    pub split_on_punctuation: bool,
    /// Also test forms with runs of repeated letters collapsed, so that
    /// `fuuuck` and `ffuck` are caught.
    pub collapse_repeats: bool,
    /// Also test a form with digits and symbols read as letters (`sh1t`, `@ss`).
    pub leetspeak: bool,
    /// Match multi-word dictionary entries against consecutive tokens.
    pub phrases: bool,
    /// In [`MatchMode::Substring`], ignore entries shorter than this.
    pub min_substring_len: usize,
    /// Stop after this many matches.
    pub max_matches: Option<usize>,
    pub(crate) span: SpanMode,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            match_threshold: 1.0,
            match_mode: MatchMode::Token,
            split_on_punctuation: false,
            collapse_repeats: false,
            leetspeak: false,
            phrases: true,
            min_substring_len: 6,
            max_matches: None,
            span: SpanMode::Core,
        }
    }
}

impl Options {
    /// Default options: exact whole-token matching.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every detector on, fuzzy at `0.9`, substring matching enabled.
    ///
    /// Convenient for offline analysis. Expect false positives; measure against
    /// your own corpus before using it on live traffic.
    #[must_use]
    pub fn aggressive() -> Self {
        Self {
            match_threshold: 0.9,
            match_mode: MatchMode::Substring,
            split_on_punctuation: true,
            collapse_repeats: true,
            leetspeak: true,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn threshold(mut self, threshold: f64) -> Self {
        self.match_threshold = threshold;
        self
    }

    #[must_use]
    pub fn match_mode(mut self, mode: MatchMode) -> Self {
        self.match_mode = mode;
        self
    }

    #[must_use]
    pub fn split_on_punctuation(mut self, yes: bool) -> Self {
        self.split_on_punctuation = yes;
        self
    }

    #[must_use]
    pub fn collapse_repeats(mut self, yes: bool) -> Self {
        self.collapse_repeats = yes;
        self
    }

    #[must_use]
    pub fn leetspeak(mut self, yes: bool) -> Self {
        self.leetspeak = yes;
        self
    }

    #[must_use]
    pub fn phrases(mut self, yes: bool) -> Self {
        self.phrases = yes;
        self
    }

    #[must_use]
    pub fn min_substring_len(mut self, len: usize) -> Self {
        self.min_substring_len = len;
        self
    }

    #[must_use]
    pub fn max_matches(mut self, max: Option<usize>) -> Self {
        self.max_matches = max;
        self
    }

    #[must_use]
    pub(crate) fn span(mut self, span: SpanMode) -> Self {
        self.span = span;
        self
    }

    /// Whether fuzzy matching is active for these options.
    pub(crate) fn is_fuzzy(&self) -> bool {
        self.match_threshold > 0.0 && self.match_threshold < 1.0
    }
}
