//! Multi-word entries.
//!
//! Matching was per-token before 3.0.0, so every multi-word entry in the
//! shipped word lists was dead weight that could never fire.

use std::path::PathBuf;

use badwords_core::{MatchKind, Options, ProfanityFilter};

fn words_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/words")
}

/// Every multi-word entry, paired with the language it came from.
fn shipped_phrases() -> Vec<(String, String)> {
    let mut phrases = Vec::new();
    for entry in std::fs::read_dir(words_dir()).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "txt") {
            continue;
        }
        let code = path.file_stem().unwrap().to_string_lossy().into_owned();
        for line in std::fs::read_to_string(&path).unwrap().lines() {
            let line = line.trim();
            if line.contains(' ') && !line.is_empty() {
                phrases.push((code.clone(), line.to_string()));
            }
        }
    }
    phrases
}

#[test]
fn every_shipped_phrase_is_detected() {
    let phrases = shipped_phrases();
    assert!(!phrases.is_empty(), "no multi-word entries found");

    for (code, phrase) in &phrases {
        let f = ProfanityFilter::builder()
            .embedded()
            .languages([code])
            .build()
            .unwrap();

        let matches = f.find(phrase, Options::new());
        assert!(
            !matches.is_empty(),
            "{code}: phrase {phrase:?} not detected"
        );
        assert_eq!(matches[0].kind, MatchKind::Phrase, "{code}: {phrase:?}");
        assert_eq!(matches[0].matched_text, *phrase, "{code}: {phrase:?}");
    }
}

#[test]
fn phrases_match_inside_a_sentence() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["very bad phrase"]);

    let text = "please stop, this is a very bad phrase indeed";
    let m = &f.find(text, Options::new())[0];
    assert_eq!(m.matched_text, "very bad phrase");
    assert_eq!(&text[m.start..m.end], "very bad phrase");
}

#[test]
fn censoring_a_phrase_keeps_its_word_structure() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["very bad phrase"]);
    assert_eq!(
        f.censor("a very bad phrase here", '*', Options::new()),
        "a **** *** ****** here"
    );
}

#[test]
fn individual_words_of_a_phrase_are_not_flagged() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["very bad phrase"]);
    for word in ["very", "bad", "phrase", "very bad", "bad phrase"] {
        assert!(
            !f.is_profane(word, Options::new()),
            "{word:?} flagged on its own"
        );
    }
}

#[test]
fn phrases_can_be_turned_off() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["very bad phrase"]);
    let opts = Options::new().phrases(false);
    assert!(!f.is_profane("a very bad phrase here", opts));
}

#[test]
fn longest_phrase_wins() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["bad phrase", "bad phrase here"]);
    let m = &f.find("a bad phrase here", Options::new())[0];
    assert_eq!(m.matched_text, "bad phrase here");
}
