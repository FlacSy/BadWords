//! Tests that need a model on disk; they skip when there is none, the same way
//! the Python suite does.

use badwords_core::ProfanityFilter;
use badwords_ml::{HybridFilter, ToxicityModel};

fn model() -> Option<ToxicityModel> {
    match ToxicityModel::open_located() {
        Ok(model) => Some(model),
        Err(error) => {
            eprintln!("skipping: {error}");
            None
        }
    }
}

#[test]
fn scores_are_probabilities() {
    let Some(model) = model() else { return };
    let scores = model.predict("hello there, nice day").expect("predict");
    for (label, value) in scores.iter() {
        assert!(
            (0.0..=1.0).contains(&value),
            "{label} out of range: {value}"
        );
    }
    assert_eq!(scores.values().len(), model.labels().len());
}

#[test]
fn toxic_text_scores_above_clean_text() {
    let Some(model) = model() else { return };
    let clean = model.predict("the weather is lovely today").expect("clean");
    let toxic = model.predict("you are a worthless idiot").expect("toxic");
    assert!(
        toxic.toxicity() > clean.toxicity(),
        "toxic {:.3} !> clean {:.3}",
        toxic.toxicity(),
        clean.toxicity()
    );
}

#[test]
fn batch_agrees_with_single() {
    let Some(model) = model() else { return };
    let texts = ["you are an idiot", "have a nice day"];
    let batched = model.predict_batch(&texts).expect("batch");
    assert_eq!(batched.len(), 2);
    for (text, batch_scores) in texts.iter().zip(&batched) {
        let single = model.predict(text).expect("single");
        // Padding to the batch's longest row moves an INT8 model slightly.
        assert!(
            (single.toxicity() - batch_scores.toxicity()).abs() < 0.1,
            "{text}: {:.3} vs {:.3}",
            single.toxicity(),
            batch_scores.toxicity()
        );
    }
}

#[test]
fn empty_batch_is_empty() {
    let Some(model) = model() else { return };
    let empty: [&str; 0] = [];
    assert!(model.predict_batch(&empty).expect("batch").is_empty());
}

#[test]
fn the_rules_answer_without_the_model_when_they_are_certain() {
    let Some(model) = model() else { return };
    let filter = ProfanityFilter::builder()
        .embedded()
        .languages(["en"])
        .build()
        .expect("filter");
    let hybrid = HybridFilter::new(filter, model);

    let certain = hybrid.check("you are a shit").expect("check");
    assert!(certain.is_profane);
    assert_eq!(certain.decided_by, badwords_ml::Decision::Rules);
    assert!(certain.scores.is_none(), "the model should not be called");

    // Nothing in the dictionary: the model has to decide, and it is asked.
    let escalated = hybrid.check("what a lovely afternoon").expect("check");
    assert_eq!(escalated.decided_by, badwords_ml::Decision::Model);
    assert!(escalated.scores.is_some());
    assert!(!escalated.is_profane);
}
