//! Rules in front of the model.
//!
//! The rules are three orders of magnitude faster, and when they find a
//! dictionary entry outright they are also right - that verdict needs no
//! second opinion. What they cannot do is recognise toxicity built from
//! ordinary words, which is most of it: on held-out English rows the
//! dictionary alone reaches 27% recall against the model's 87%. In Russian it
//! reaches 50%, so how much the model adds depends on the language.
//!
//! So the split here is deliberately one-sided. A certain rule hit answers
//! immediately; *everything else* goes to the model, including text the rules
//! found nothing in. Treating "the rules saw nothing" as "clean" is what makes
//! a hybrid score worse than the model it wraps.

use badwords_core::{Match, Options, ProfanityFilter};

use crate::error::Error;
use crate::model::ToxicityModel;
use crate::scores::Scores;

/// Rule score at or above which the rules answer alone. `1.0` is an exact
/// dictionary hit; lower values let a fuzzy near-match decide too.
pub const DEFAULT_CERTAIN_AT: f64 = 1.0;

/// Model probability at or above which text counts as toxic.
///
/// Matches the Python side. Measured on held-out rows, F1 peaks at 0.25 in
/// English and 0.15 in Russian, so 0.3 serves both; each axis has its own best
/// threshold, and a moderation policy should set them per axis.
pub const DEFAULT_DECISION_THRESHOLD: f32 = 0.3;

/// Which half of the filter produced the verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// A dictionary hit the rules were sure of; the model was not called.
    Rules,
    /// The model was asked.
    Model,
}

/// What the hybrid decided, and on what evidence.
#[derive(Debug, Clone)]
pub struct HybridResult {
    /// The verdict.
    pub is_profane: bool,
    /// Best rule score, `0.0` when nothing matched.
    pub rule_score: f64,
    /// Per-axis probabilities, present only when the model was called.
    pub scores: Option<Scores>,
    /// Which half decided.
    pub decided_by: Decision,
    /// What the rules found, whoever decided.
    pub matches: Vec<Match>,
}

/// A rule filter and a model, used together.
pub struct HybridFilter {
    filter: ProfanityFilter,
    model: ToxicityModel,
    options: Options,
    certain_at: f64,
    threshold: f32,
}

impl HybridFilter {
    /// Build from an existing filter and model.
    ///
    /// The rule pass runs with every character-level evasion detector on:
    /// `split_on_punctuation`, `collapse_repeats` and `leetspeak` each flag
    /// zero of 73,302 ordinary English words, so they cost nothing here and
    /// they catch the spellings a dictionary alone misses.
    pub fn new(filter: ProfanityFilter, model: ToxicityModel) -> Self {
        let options = Options::new()
            .split_on_punctuation(true)
            .collapse_repeats(true)
            .leetspeak(true);
        Self {
            filter,
            model,
            options,
            certain_at: DEFAULT_CERTAIN_AT,
            threshold: DEFAULT_DECISION_THRESHOLD,
        }
    }

    /// Options for the rule pass.
    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Rule score at or above which the rules answer without the model.
    pub fn certain_at(mut self, score: f64) -> Self {
        self.certain_at = score;
        self
    }

    /// Model probability at or above which text counts as toxic.
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// The rule filter, for adding words or a whitelist.
    pub fn filter(&self) -> &ProfanityFilter {
        &self.filter
    }

    /// The rule filter, mutably.
    pub fn filter_mut(&mut self) -> &mut ProfanityFilter {
        &mut self.filter
    }

    /// The model.
    pub fn model(&self) -> &ToxicityModel {
        &self.model
    }

    /// Judge one text.
    pub fn check(&self, text: &str) -> Result<HybridResult, Error> {
        Ok(self.check_many(&[text])?.remove(0))
    }

    /// Judge several texts, with one batched model call for those that need it.
    pub fn check_many<S: AsRef<str>>(&self, texts: &[S]) -> Result<Vec<HybridResult>, Error> {
        let mut results: Vec<HybridResult> = Vec::with_capacity(texts.len());
        let mut pending: Vec<usize> = Vec::new();
        let mut pending_texts: Vec<&str> = Vec::new();

        for (index, text) in texts.iter().enumerate() {
            let text = text.as_ref();
            let matches = self.filter.find(text, self.options);
            let rule_score = matches
                .iter()
                .map(|found| found.score)
                .fold(0.0_f64, f64::max);

            if rule_score >= self.certain_at {
                results.push(HybridResult {
                    is_profane: true,
                    rule_score,
                    scores: None,
                    decided_by: Decision::Rules,
                    matches,
                });
            } else {
                pending.push(index);
                pending_texts.push(text);
                results.push(HybridResult {
                    is_profane: false,
                    rule_score,
                    scores: None,
                    decided_by: Decision::Model,
                    matches,
                });
            }
        }

        if !pending_texts.is_empty() {
            let scored = self.model.predict_batch(&pending_texts)?;
            for (slot, scores) in pending.into_iter().zip(scored) {
                let result = &mut results[slot];
                result.is_profane = scores.toxicity() >= self.threshold;
                result.scores = Some(scores);
            }
        }

        Ok(results)
    }

    /// Whether the text is profane, discarding the reasoning.
    pub fn is_profane(&self, text: &str) -> Result<bool, Error> {
        Ok(self.check(text)?.is_profane)
    }
}
