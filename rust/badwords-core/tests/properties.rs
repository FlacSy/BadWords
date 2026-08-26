//! Property tests.
//!
//! The load-bearing ones are `fuzzy_index_agrees_with_brute_force` (the index
//! must not silently lose matches) and `spans_are_always_valid` (span
//! arithmetic must never produce a non-boundary offset), plus
//! `enabling_a_flag_never_removes_a_match`, which is the machine-checkable
//! form of "defaults are unchanged".

use badwords_core::{MatchMode, Options, ProfanityFilter};
use proptest::prelude::*;

fn filter() -> ProfanityFilter {
    ProfanityFilter::builder()
        .embedded()
        .languages(["en", "ru"])
        .build()
        .unwrap()
}

/// Single-token dictionary entries, normalized, as the matcher stores them.
fn dictionary_forms(f: &ProfanityFilter) -> Vec<String> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/words");
    let mut forms = Vec::new();
    for code in ["en", "ru"] {
        let text = std::fs::read_to_string(dir.join(format!("{code}.txt"))).unwrap();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.contains(' ') {
                continue;
            }
            let form = f.normalize(line);
            if !form.is_empty() {
                forms.push(form);
            }
        }
    }
    forms.sort();
    forms.dedup();
    forms
}

/// Text that exercises the tokenizer: letters, digits, punctuation, whitespace,
/// non-ASCII and emoji.
fn text_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            "[a-z]{1,8}",
            "[A-Za-z0-9]{1,6}",
            r"[.,!?\-_/'\(\)]{1,3}",
            "[\u{0400}-\u{04FF}]{1,6}",
            Just("  ".to_string()),
            Just("\t".to_string()),
            Just("\u{200b}".to_string()),
            Just("🙂".to_string()),
            Just("é".to_string()),
            Just("ﬁ".to_string()),
        ],
        0..12,
    )
    .prop_map(|parts| parts.join(" "))
}

fn all_option_sets() -> Vec<Options> {
    let base = Options::new();
    vec![
        base,
        base.threshold(0.9),
        base.split_on_punctuation(true),
        base.collapse_repeats(true),
        base.leetspeak(true),
        base.phrases(false),
        base.match_mode(MatchMode::Substring),
        Options::aggressive(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Every reported span must be a valid slice of the input, and must equal
    /// the text the match claims to have matched.
    #[test]
    fn spans_are_always_valid(text in text_strategy()) {
        let f = filter();
        for opts in all_option_sets() {
            for m in f.find(&text, opts) {
                prop_assert!(m.start <= m.end, "inverted span {m:?}");
                prop_assert!(m.end <= text.len(), "span past the end: {m:?}");
                prop_assert!(text.is_char_boundary(m.start), "start not on a boundary: {m:?}");
                prop_assert!(text.is_char_boundary(m.end), "end not on a boundary: {m:?}");
                prop_assert_eq!(&text[m.start..m.end], m.matched_text.as_str());
            }
        }
    }

    /// The short-circuiting path and the collecting path must agree.
    #[test]
    fn is_profane_agrees_with_find(text in text_strategy()) {
        let f = filter();
        for opts in all_option_sets() {
            prop_assert_eq!(
                f.is_profane(&text, opts),
                !f.find(&text, opts).is_empty(),
                "disagreement on {:?}", text
            );
        }
    }

    /// Matches come back sorted by position and never overlap.
    #[test]
    fn matches_are_sorted_and_disjoint(text in text_strategy()) {
        let f = filter();
        for opts in all_option_sets() {
            let matches = f.find(&text, opts);
            for pair in matches.windows(2) {
                prop_assert!(pair[0].start <= pair[1].start);
                prop_assert!(pair[0].end <= pair[1].start, "overlap: {pair:?}");
            }
        }
    }

    /// Censoring rewrites matched spans and nothing else, and never changes the
    /// number of characters.
    #[test]
    fn censoring_preserves_everything_else(text in text_strategy()) {
        let f = filter();
        for opts in all_option_sets() {
            let censored = f.censor(&text, '*', opts);
            let matches = f.find(&text, opts);

            if matches.is_empty() {
                prop_assert_eq!(&censored, &text);
                continue;
            }

            // Byte offsets of the original are meaningless in the output (a
            // multi-byte char becomes a one-byte `*`), so compare per character.
            let original: Vec<(usize, char)> = text.char_indices().collect();
            let produced: Vec<char> = censored.chars().collect();
            prop_assert_eq!(
                original.len(), produced.len(),
                "character count changed: {:?} -> {:?}", text, censored
            );

            for (idx, &(offset, original_char)) in original.iter().enumerate() {
                let inside = matches.iter().any(|m| offset >= m.start && offset < m.end);
                if inside {
                    let expected = if original_char.is_whitespace() { original_char } else { '*' };
                    prop_assert_eq!(produced[idx], expected, "at char {}", idx);
                } else {
                    prop_assert_eq!(produced[idx], original_char, "at char {}", idx);
                }
            }
        }
    }

    /// Turning a detector on may add matches; it must never remove one.
    /// This is the machine-checkable form of "the defaults are unchanged".
    #[test]
    fn enabling_a_flag_never_removes_a_match(text in text_strategy()) {
        let f = filter();
        let base = Options::new();
        let baseline: Vec<(usize, usize)> =
            f.find(&text, base).iter().map(|m| (m.start, m.end)).collect();

        for opts in [
            base.split_on_punctuation(true),
            base.collapse_repeats(true),
            base.leetspeak(true),
            base.match_mode(MatchMode::Substring),
        ] {
            let widened = f.find(&text, opts);
            for span in &baseline {
                let covered = widened
                    .iter()
                    .any(|m| m.start <= span.0 && m.end >= span.1);
                prop_assert!(covered, "flag dropped the match at {span:?} in {text:?}");
            }
        }
    }

    /// The fuzzy index must find exactly what a brute-force scan would.
    #[test]
    fn fuzzy_index_agrees_with_brute_force(
        token in "[a-z]{2,10}",
        threshold in 0.75f64..0.99,
    ) {
        let f = filter();
        let forms = dictionary_forms(&f);
        let query = f.normalize(&token);
        prop_assume!(!query.is_empty());

        let best_brute = forms
            .iter()
            .map(|form| strsim::jaro_winkler(&query, form))
            .fold(0.0f64, f64::max);
        let expected = best_brute > threshold;

        let opts = Options::new().threshold(threshold);
        let found = f.find(&token, opts);
        prop_assert_eq!(
            !found.is_empty(), expected,
            "token {:?} threshold {} - brute force best {}", token, threshold, best_brute
        );

        if let Some(m) = found.first() {
            prop_assert!(
                (m.score - best_brute).abs() < 1e-9,
                "reported {} but the best is {}", m.score, best_brute
            );
        }
    }
}
