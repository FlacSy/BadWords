//! Guards the false-positive cost of the opt-in detectors.
//!
//! Skips when the system word list is absent (Debian: `apt-get install
//! wamerican`), so it is informative locally and enforced in CI.

use std::collections::HashSet;
use std::path::Path;

use badwords_core::{MatchMode, Options, ProfanityFilter};

const CORPUS: &str = "/usr/share/dict/american-english";

fn clean_corpus(filter: &ProfanityFilter) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(Path::new(CORPUS)).ok()?;
    Some(
        text.lines()
            .map(|w| w.trim().trim_end_matches("'s").to_lowercase())
            .filter(|w| !w.is_empty() && !w.contains('\''))
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|w| !filter.contains_word(w))
            .collect(),
    )
}

fn rate(filter: &ProfanityFilter, words: &[String], opts: Options) -> (usize, f64) {
    let hits = words.iter().filter(|w| filter.is_profane(w, opts)).count();
    (hits, 100.0 * hits as f64 / words.len() as f64)
}

#[test]
fn opt_in_detectors_stay_within_budget() {
    let filter = ProfanityFilter::builder()
        .embedded()
        .all_languages()
        .build()
        .unwrap();
    let Some(words) = clean_corpus(&filter) else {
        eprintln!("skipping: {CORPUS} not present");
        return;
    };
    assert!(
        words.len() > 50_000,
        "corpus looks truncated: {}",
        words.len()
    );

    let base = Options::new();

    // These three must cost nothing at all; that is what their guard rules and
    // collapse thresholds were chosen for.
    for (name, opts) in [
        ("default", base),
        ("split_on_punctuation", base.split_on_punctuation(true)),
        ("collapse_repeats", base.collapse_repeats(true)),
        ("leetspeak", base.leetspeak(true)),
    ] {
        let (hits, pct) = rate(&filter, &words, opts);
        assert_eq!(hits, 0, "{name} flagged {hits} clean words ({pct:.3}%)");
    }

    // Substring matching cannot be free; the default length is chosen so that
    // it stays under a quarter of a percent.
    let (_, pct) = rate(
        &filter,
        &words,
        base.match_mode(MatchMode::Substring).min_substring_len(6),
    );
    assert!(
        pct < 0.30,
        "substring@6 false-positive rate rose to {pct:.3}%"
    );

    // A shorter minimum is much worse, which is why 6 is the default.
    let (_, short) = rate(
        &filter,
        &words,
        base.match_mode(MatchMode::Substring).min_substring_len(4),
    );
    assert!(short > pct, "expected a shorter minimum to be worse");
}
