//! The whitelist: the lever for suppressing a rule that fires wrongly.

use badwords_core::{MatchMode, Options, ProfanityFilter};

#[test]
fn whitelisted_words_are_never_reported() {
    let mut f = ProfanityFilter::builder()
        .embedded()
        .languages(["en"])
        .build()
        .unwrap();
    let opts = Options::new();

    assert!(f.is_profane("damn", opts));
    f.add_whitelist(&["damn"]);
    assert!(!f.is_profane("damn", opts));
    assert!(f.is_whitelisted("damn"));

    f.remove_whitelist(&["damn"]);
    assert!(f.is_profane("damn", opts));
}

#[test]
fn whitelist_is_normalized_like_everything_else() {
    let mut f = ProfanityFilter::builder()
        .embedded()
        .languages(["en"])
        .build()
        .unwrap();
    f.add_whitelist(&["DAMN"]);
    assert!(!f.is_profane("damn", Options::new()));
    assert!(!f.is_profane("Damn!", Options::new()));
}

/// The whitelist is consulted per token, which is what lets it suppress a
/// substring hit inside a longer, innocent word.
#[test]
fn whitelist_suppresses_substring_matches() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["ass"]);
    let opts = Options::new()
        .match_mode(MatchMode::Substring)
        .min_substring_len(3);

    assert!(f.is_profane("classic", opts));
    f.add_whitelist(&["classic"]);
    assert!(!f.is_profane("classic", opts));
    // Other carriers still fire.
    assert!(f.is_profane("assess", opts));
}

#[test]
fn whitelist_suppresses_fuzzy_matches() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["badword"]);
    let opts = Options::new().threshold(0.9);

    assert!(f.is_profane("badwrod", opts));
    f.add_whitelist(&["badwrod"]);
    assert!(!f.is_profane("badwrod", opts));
}

#[test]
fn whitelist_suppresses_phrases() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["very bad phrase"]);
    assert!(f.is_profane("a very bad phrase", Options::new()));

    f.add_whitelist(&["bad"]);
    assert!(!f.is_profane("a very bad phrase", Options::new()));
}

#[test]
fn clearing_the_whitelist_restores_matching() {
    let mut f = ProfanityFilter::builder()
        .embedded()
        .languages(["en"])
        .build()
        .unwrap();
    f.add_whitelist(&["damn"]);
    assert!(!f.is_profane("damn", Options::new()));
    f.clear_whitelist();
    assert!(f.is_profane("damn", Options::new()));
}

#[test]
fn builder_accepts_a_whitelist() {
    let f = ProfanityFilter::builder()
        .embedded()
        .languages(["en"])
        .whitelist(["damn"])
        .build()
        .unwrap();
    assert!(!f.is_profane("damn", Options::new()));
}
