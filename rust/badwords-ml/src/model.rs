//! The ONNX model: load a directory, score text.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ort::session::{builder::GraphOptimizationLevel, Session, SessionInputValue};
use ort::value::Tensor;
use serde::Deserialize;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::error::Error;
use crate::scores::Scores;

/// Longest input in tokens. Matches the Python side and the training run.
pub const DEFAULT_MAX_LENGTH: usize = 128;

const MODEL_FILE: &str = "model.onnx";
const CONFIG_FILE: &str = "config.json";
const TOKENIZER_FILE: &str = "tokenizer.json";

/// How logits become probabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Activation {
    /// One independent probability per axis: the multi-label model.
    Sigmoid,
    /// Probabilities across mutually exclusive classes: the pre-3.1 binary model.
    Softmax,
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    id2label: HashMap<String, String>,
    #[serde(default)]
    problem_type: Option<String>,
}

/// A loaded toxicity model.
///
/// Scoring takes `&self`, so one model can be shared across threads; ONNX
/// Runtime wants `&mut` for a run, which a mutex provides.
///
/// ```no_run
/// use badwords_ml::ToxicityModel;
///
/// let model = ToxicityModel::open("model")?;
/// let scores = model.predict("you are an idiot")?;
/// println!("{:.2} {:?}", scores.toxicity(), scores.strongest());
/// # Ok::<(), badwords_ml::Error>(())
/// ```
pub struct ToxicityModel {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    labels: Arc<[Box<str>]>,
    activation: Activation,
    max_length: usize,
}

impl ToxicityModel {
    /// Load a model directory produced by `ml/train.py`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, Error> {
        Self::builder(dir).build()
    }

    /// Load whichever model [`crate::locate_model`] finds.
    pub fn open_located() -> Result<Self, Error> {
        Self::open(crate::locate_model()?)
    }

    /// Configure a load.
    pub fn builder(dir: impl AsRef<Path>) -> ToxicityModelBuilder {
        ToxicityModelBuilder {
            dir: dir.as_ref().to_path_buf(),
            max_length: DEFAULT_MAX_LENGTH,
            threads: None,
        }
    }

    /// Score one text.
    pub fn predict(&self, text: &str) -> Result<Scores, Error> {
        Ok(self.predict_batch(&[text])?.remove(0))
    }

    /// Score several texts in one pass.
    ///
    /// Texts are padded to the longest in the batch; on an INT8 model that can
    /// move a score by a few hundredths against scoring the same text alone.
    pub fn predict_batch<S: AsRef<str>>(&self, texts: &[S]) -> Result<Vec<Scores>, Error> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let inputs: Vec<&str> = texts.iter().map(AsRef::as_ref).collect();
        let encodings = self
            .tokenizer
            .encode_batch(inputs, true)
            .map_err(|error| Error::Tokenizer(error.to_string()))?;

        let rows = encodings.len();
        let columns = encodings.first().map_or(0, |first| first.get_ids().len());
        let shape = [rows as i64, columns as i64];

        let mut ids = Vec::with_capacity(rows * columns);
        let mut mask = Vec::with_capacity(rows * columns);
        for encoding in &encodings {
            ids.extend(encoding.get_ids().iter().map(|&id| i64::from(id)));
            mask.extend(encoding.get_attention_mask().iter().map(|&m| i64::from(m)));
        }

        let mut session = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Only feed what the graph declares: XLM-R exports carry no
        // token_type_ids, and passing an unknown input is an error.
        let wanted: Vec<String> = session
            .inputs()
            .iter()
            .map(|input| input.name().to_string())
            .collect();
        let mut feed: Vec<(Cow<'_, str>, SessionInputValue<'_>)> = Vec::with_capacity(wanted.len());
        for name in &wanted {
            let data = match name.as_str() {
                "input_ids" => ids.clone(),
                "attention_mask" => mask.clone(),
                "token_type_ids" => vec![0i64; rows * columns],
                _ => continue,
            };
            feed.push((
                Cow::Owned(name.clone()),
                SessionInputValue::from(Tensor::from_array((shape, data))?),
            ));
        }

        let outputs = session.run(feed)?;
        let (_, logits) = outputs[0].try_extract_tensor::<f32>()?;

        let width = self.labels.len();
        if logits.len() != rows * width {
            return Err(Error::UnexpectedOutput {
                expected: rows * width,
                got: logits.len(),
            });
        }

        Ok(logits
            .chunks(width)
            .map(|row| Scores::new(Arc::clone(&self.labels), self.activate(row)))
            .collect())
    }

    /// The axes this model scores, in the order it emits them.
    pub fn labels(&self) -> &[Box<str>] {
        &self.labels
    }

    /// The axis names joined for printing.
    pub fn label_list(&self) -> String {
        self.labels
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Longest input in tokens; anything past it is truncated.
    pub fn max_length(&self) -> usize {
        self.max_length
    }

    fn activate(&self, logits: &[f32]) -> Box<[f32]> {
        match self.activation {
            Activation::Sigmoid => logits.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect(),
            Activation::Softmax => {
                let peak = logits.iter().copied().fold(f32::MIN, f32::max);
                let exponentiated: Vec<f32> = logits.iter().map(|&x| (x - peak).exp()).collect();
                let total: f32 = exponentiated.iter().sum();
                exponentiated.into_iter().map(|x| x / total).collect()
            }
        }
    }
}

/// Options for loading a model.
pub struct ToxicityModelBuilder {
    dir: PathBuf,
    max_length: usize,
    threads: Option<usize>,
}

impl ToxicityModelBuilder {
    /// Longest input in tokens (default [`DEFAULT_MAX_LENGTH`]).
    pub fn max_length(mut self, tokens: usize) -> Self {
        self.max_length = tokens;
        self
    }

    /// Threads ONNX Runtime may use for one run. Defaults to its own choice.
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// Load the model.
    pub fn build(self) -> Result<ToxicityModel, Error> {
        let model_path = self.require(MODEL_FILE)?;
        let config_path = self.require(CONFIG_FILE)?;
        let tokenizer_path = self.require(TOKENIZER_FILE)?;

        let config_text = std::fs::read_to_string(&config_path).map_err(|source| Error::Io {
            path: config_path.clone(),
            source,
        })?;
        let config: Config =
            serde_json::from_str(&config_text).map_err(|source| Error::Config {
                path: config_path.clone(),
                source,
            })?;

        let mut tokenizer =
            Tokenizer::from_file(&tokenizer_path).map_err(|e| Error::Tokenizer(e.to_string()))?;
        let pad_id = tokenizer.token_to_id("<pad>").unwrap_or(1);
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_id,
            pad_token: "<pad>".to_string(),
            ..Default::default()
        }));
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: self.max_length,
                ..Default::default()
            }))
            .map_err(|e| Error::Tokenizer(e.to_string()))?;

        let mut builder =
            Session::builder()?.with_optimization_level(GraphOptimizationLevel::Level3)?;
        if let Some(threads) = self.threads {
            builder = builder.with_intra_threads(threads)?;
        }
        let session = builder.commit_from_file(&model_path)?;

        // Models exported before 3.1 record no id2label, so fall back to what
        // the graph itself says: two outputs is the old clean/toxic head.
        let labels = match ordered_labels(&config) {
            Some(labels) => labels,
            None => fallback_labels(&session).ok_or(Error::MissingLabels { path: config_path })?,
        };
        let activation = match config.problem_type.as_deref() {
            Some("multi_label_classification") => Activation::Sigmoid,
            Some("single_label_classification") => Activation::Softmax,
            _ if labels.len() == 2 => Activation::Softmax,
            _ => Activation::Sigmoid,
        };

        Ok(ToxicityModel {
            session: Mutex::new(session),
            tokenizer,
            labels,
            activation,
            max_length: self.max_length,
        })
    }

    fn require(&self, name: &str) -> Result<PathBuf, Error> {
        let path = self.dir.join(name);
        if path.is_file() {
            Ok(path)
        } else {
            Err(Error::MissingFile { path })
        }
    }
}

/// Names for a model whose config does not provide any.
fn fallback_labels(session: &Session) -> Option<Arc<[Box<str>]>> {
    let ort::value::ValueType::Tensor { shape, .. } = session.outputs().first()?.dtype() else {
        return None;
    };
    let width = usize::try_from(*shape.last()?).ok()?;
    if width == 2 {
        return Some(vec!["clean".into(), "toxic".into()].into());
    }
    Some(
        (0..width)
            .map(|index| format!("label_{index}").into_boxed_str())
            .collect(),
    )
}

/// Label names ordered by their output index.
fn ordered_labels(config: &Config) -> Option<Arc<[Box<str>]>> {
    if config.id2label.is_empty() {
        return None;
    }
    let mut pairs: Vec<(usize, &String)> = config
        .id2label
        .iter()
        .filter_map(|(index, label)| index.parse::<usize>().ok().map(|index| (index, label)))
        .collect();
    pairs.sort_by_key(|&(index, _)| index);
    Some(
        pairs
            .into_iter()
            .map(|(_, label)| label.as_str().into())
            .collect(),
    )
}
