//! One text's score on every axis the model was trained for.

use std::fmt;
use std::sync::Arc;

/// The axis every model annotates, and the one a single number should come from.
pub const TOXICITY: &str = "toxicity";

/// Per-axis probabilities for one text.
///
/// The axes come from the model's own `config.json`, so a model with a
/// different label set stays usable: ask by name, or iterate.
///
/// ```no_run
/// # use badwords_ml::{Scores, ToxicityModel};
/// # let model = ToxicityModel::open("model")?;
/// let scores = model.predict("you are an idiot")?;
/// scores.toxicity();              // 0.0 - 1.0
/// scores.get("insult");           // Some(0.93)
/// scores.strongest();             // ("insult", 0.93)
/// # Ok::<(), badwords_ml::Error>(())
/// ```
#[derive(Clone, PartialEq)]
pub struct Scores {
    labels: Arc<[Box<str>]>,
    values: Box<[f32]>,
}

impl Scores {
    pub(crate) fn new(labels: Arc<[Box<str>]>, values: Box<[f32]>) -> Self {
        Self { labels, values }
    }

    /// Probability for a named axis, or `None` if the model has no such axis.
    pub fn get(&self, label: &str) -> Option<f32> {
        self.labels
            .iter()
            .position(|candidate| candidate.as_ref() == label)
            .map(|index| self.values[index])
    }

    /// Probability on the overall-toxicity axis, `0.0` if the model lacks one.
    ///
    /// `toxic` is accepted as well, which is what the pre-3.1 binary model
    /// calls the same thing.
    pub fn toxicity(&self) -> f32 {
        self.get(TOXICITY)
            .or_else(|| self.get("toxic"))
            .unwrap_or_default()
    }

    /// The highest-scoring axis and its probability.
    pub fn strongest(&self) -> (&str, f32) {
        let mut best = (0usize, f32::MIN);
        for (index, &value) in self.values.iter().enumerate() {
            if value > best.1 {
                best = (index, value);
            }
        }
        (self.labels[best.0].as_ref(), best.1)
    }

    /// Axes scoring at or above `threshold`, strongest first.
    pub fn above(&self, threshold: f32) -> Vec<(&str, f32)> {
        let mut hits: Vec<(&str, f32)> = self
            .iter()
            .filter(|&(_, value)| value >= threshold)
            .collect();
        hits.sort_by(|a, b| b.1.total_cmp(&a.1));
        hits
    }

    /// Every axis with its probability, in the model's own order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f32)> {
        self.labels
            .iter()
            .map(Box::as_ref)
            .zip(self.values.iter().copied())
    }

    /// The axis names, in the order the model emits them.
    pub fn labels(&self) -> &[Box<str>] {
        &self.labels
    }

    /// Join the axis names, for printing.
    pub fn label_list(&self) -> String {
        self.labels
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The raw probabilities, in the order the model emits them.
    pub fn values(&self) -> &[f32] {
        &self.values
    }
}

impl fmt::Debug for Scores {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}
