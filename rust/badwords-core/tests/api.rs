//! The 3.0 matching API.

use badwords_core::{MatchKind, MatchMode, Options, ProfanityFilter};

fn filter() -> ProfanityFilter {
    ProfanityFilter::builder()
        .embedded()
        .languages(["en"])
        .build()
        .expect("embedded resources")
}

#[test]
fn censor_keeps_punctuation() {
    let f = filter();
    assert_eq!(
        f.censor("hey fuck, ok", '*', Options::new()),
        "hey ****, ok"
    );
    assert_eq!(f.censor("(fuck)!", '*', Options::new()), "(****)!");
    assert_eq!(f.censor("...fuck...", '#', Options::new()), "...####...");
}

#[test]
fn censor_returns_input_when_nothing_matched() {
    let f = filter();
    let text = "a perfectly ordinary sentence";
    assert_eq!(f.censor(text, '*', Options::new()), text);
}

#[test]
fn censor_with_custom_replacement() {
    let f = filter();
    let out = f.censor_with("that is shit", |m| format!("[{}]", m.word), Options::new());
    assert_eq!(out, "that is [shit]");
}

#[test]
fn match_spans_point_into_the_original_text() {
    let f = filter();
    let text = "well, shit happens";
    let matches = f.find(text, Options::new());
    assert_eq!(matches.len(), 1);
    let m = &matches[0];
    assert_eq!(&text[m.start..m.end], m.matched_text);
    assert_eq!(m.matched_text, "shit");
    assert_eq!(m.kind, MatchKind::Exact);
    assert_eq!(m.language.as_deref(), Some("en"));
    assert!((m.score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn is_profane_agrees_with_find() {
    let f = filter();
    for text in [
        "hello world",
        "shit happens",
        "",
        "   ",
        "a shit b fuck c",
        "punctuation!!!",
    ] {
        assert_eq!(
            f.is_profane(text, Options::new()),
            !f.find(text, Options::new()).is_empty(),
            "disagreement on {text:?}"
        );
    }
}

#[test]
fn find_first_matches_find() {
    let f = filter();
    let text = "shit and fuck";
    assert_eq!(
        f.find_first(text, Options::new()),
        f.find(text, Options::new()).into_iter().next()
    );
}

#[test]
fn matches_are_sorted_and_disjoint() {
    let f = filter();
    let matches = f.find("shit fuck bitch, damn", Options::new());
    assert!(matches.len() >= 3);
    for pair in matches.windows(2) {
        assert!(
            pair[0].end <= pair[1].start,
            "overlapping matches: {pair:?}"
        );
    }
}

#[test]
fn max_matches_truncates() {
    let f = filter();
    let opts = Options::new().max_matches(Some(1));
    assert_eq!(f.find("shit fuck bitch", opts).len(), 1);
}

#[test]
fn dictionary_can_be_edited() {
    let mut f = filter();
    let opts = Options::new();

    f.add_words(&["custombad"]);
    assert!(f.is_profane("custombad", opts));
    assert!(f.contains_word("custombad"));

    f.remove_words(&["custombad"]);
    assert!(!f.is_profane("custombad", opts));

    // A word a language also provides survives removal of the custom copy.
    f.remove_words(&["shit"]);
    assert!(f.is_profane("shit", opts));

    let before = f.word_count();
    assert!(before > 0);
    f.clear_words();
    assert_eq!(f.word_count(), 0);
    assert!(!f.is_profane("shit", opts));
}

#[test]
fn batch_matches_single() {
    let f = filter();
    let opts = Options::new();
    let texts = ["hello", "shit", "clean text", "what the fuck"];

    assert_eq!(
        f.is_profane_many(&texts, opts),
        texts
            .iter()
            .map(|t| f.is_profane(t, opts))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        f.find_many(&texts, opts),
        texts.iter().map(|t| f.find(t, opts)).collect::<Vec<_>>()
    );
    assert_eq!(
        f.censor_many(&texts, '*', opts),
        texts
            .iter()
            .map(|t| f.censor(t, '*', opts))
            .collect::<Vec<_>>()
    );
}

#[test]
fn evasion_detectors_are_off_by_default() {
    // A word of our own, so that literal entries in en.txt (which ships `sh1t`,
    // `a55`, `fuckyou` and friends) cannot mask the behaviour under test.
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["badword"]);
    let default = Options::new();

    for text in [
        "you.badword",
        "badword-you",
        "hey/badword",
        "baaadword",
        "bbadword",
        "b4dword",
    ] {
        assert!(
            !f.is_profane(text, default),
            "{text:?} matched with default options"
        );
    }
    // Punctuation is stripped rather than split on, so anything that glues back
    // into an entry has always matched, and must keep matching.
    assert!(f.is_profane("b.a.d.w.o.r.d", default));
    assert!(f.is_profane("bad-word", default));
}

#[test]
fn split_on_punctuation_catches_inner_separators() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["badword"]);
    let opts = Options::new().split_on_punctuation(true);
    for text in ["you.badword", "badword-you", "hey/badword", "hey_badword"] {
        assert!(f.is_profane(text, opts), "{text:?} not caught");
    }
    // Still works, because the glued form is tested first.
    assert!(f.is_profane("b.a.d.w.o.r.d", opts));
}

#[test]
fn collapse_repeats_catches_stretched_words() {
    let mut f = filter();
    f.add_words(&["badword"]);
    let opts = Options::new().collapse_repeats(true);
    for text in [
        "fuuuck",
        "ffuck",
        "shiiit",
        "fuuuuuuck",
        "baaadword",
        "bbadword",
    ] {
        assert!(f.is_profane(text, opts), "{text:?} not caught");
    }
    // Ordinary doubled letters must stay clean.
    for text in ["book", "boot", "cook", "assess", "cassette", "bookkeeper"] {
        assert!(!f.is_profane(text, opts), "{text:?} false positive");
    }
}

#[test]
fn leetspeak_reads_digits_as_letters() {
    let mut f = filter();
    f.add_words(&["badword"]);
    let opts = Options::new().leetspeak(true);
    for text in ["b4dword", "badw0rd", "b4dw0rd", "p0rn"] {
        assert!(f.is_profane(text, opts), "{text:?} not caught");
    }
    // Guards: number-like tokens must not be reinterpreted.
    for text in [
        "1st", "2nd", "100k", "mp3", "h2o", "ps5", "win7", "404", "1337", "12345",
    ] {
        assert!(!f.is_profane(text, opts), "{text:?} false positive");
    }
}

#[test]
fn literal_leet_entries_still_match_without_the_flag() {
    let f = filter();
    // en.txt ships `a55` and `5hit` as literal entries.
    assert!(f.is_profane("a55", Options::new()));
}

#[test]
fn substring_mode_catches_glued_words() {
    let mut f = filter();
    f.add_words(&["badword"]);
    let opts = Options::new().match_mode(MatchMode::Substring);

    assert!(f.is_profane("xxbadwordxx", opts));
    assert!(!f.is_profane("xxbadwordxx", Options::new()));

    let m = &f.find("xxbadwordxx", opts)[0];
    assert_eq!(m.kind, MatchKind::Substring);
    // The whole token is reported, so censoring hides the carrier word too.
    assert_eq!(m.matched_text, "xxbadwordxx");
}

#[test]
fn substring_respects_minimum_length() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["ass"]);
    let short = Options::new()
        .match_mode(MatchMode::Substring)
        .min_substring_len(3);
    let long = Options::new()
        .match_mode(MatchMode::Substring)
        .min_substring_len(6);

    assert!(f.is_profane("classic", short));
    assert!(!f.is_profane("classic", long));
}

#[test]
fn fuzzy_matching_finds_typos() {
    let mut f = ProfanityFilter::builder().embedded().build().unwrap();
    f.add_words(&["badword"]);
    let opts = Options::new().threshold(0.9);

    assert!(f.is_profane("badwrod", opts));
    assert!(!f.is_profane("badwrod", Options::new()));
    assert_eq!(f.find("badwrod", opts)[0].kind, MatchKind::Fuzzy);
}

#[test]
fn similar_is_jaro_winkler() {
    let f = filter();
    assert!((f.similar("hello", "hello") - 1.0).abs() < f64::EPSILON);
    assert!((f.similar("hello", "hallo") - strsim::jaro_winkler("hello", "hallo")).abs() < 1e-12);
}
