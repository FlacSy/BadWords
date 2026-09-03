//! Multi-label toxicity scoring for [`badwords`], on ONNX Runtime.
//!
//! The rule-based engine in `badwords-core` answers "is this word in the
//! dictionary". This crate answers "how toxic is this text, and in what way",
//! with one probability per axis - toxicity, insult, threat, and so on, taken
//! from the model's own config rather than hardcoded.
//!
//! ```no_run
//! use badwords_ml::ToxicityModel;
//!
//! let model = ToxicityModel::open("model")?;
//! let scores = model.predict("you are an idiot")?;
//!
//! println!("{:.2}", scores.toxicity());
//! for (axis, value) in scores.above(0.5) {
//!     println!("{axis}: {value:.2}");
//! }
//! # Ok::<(), badwords_ml::Error>(())
//! ```
//!
//! [`HybridFilter`] puts the rules in front: they answer outright when they
//! can, and the model is asked only when they cannot.
//!
//! The model directory is the one `ml/train.py` writes - `model.onnx`,
//! `config.json` and `tokenizer.json` - the same directory the Python package
//! downloads and caches.

mod error;
mod hybrid;
mod locate;
mod model;
mod scores;

pub use error::Error;
pub use hybrid::{
    Decision, HybridFilter, HybridResult, DEFAULT_CERTAIN_AT, DEFAULT_DECISION_THRESHOLD,
};
pub use locate::{cache_dir, is_complete, locate_model, REQUIRED_FILES};
pub use model::{ToxicityModel, ToxicityModelBuilder, DEFAULT_MAX_LENGTH};
pub use scores::{Scores, TOXICITY};
