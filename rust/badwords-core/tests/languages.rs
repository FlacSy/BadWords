//! Language codes, aliases and the available/loaded split.

use badwords_core::{Error, LanguageWarning, Options, ProfanityFilter};

fn filter() -> ProfanityFilter {
    ProfanityFilter::builder().embedded().build().unwrap()
}

/// 2.x overwrote the list of AVAILABLE languages with the list of LOADED ones,
/// so a second `init` with a wider set failed.
#[test]
fn languages_can_be_loaded_twice() {
    let mut f = filter();
    let available_before = f.available_languages().len();

    f.load_languages(&["en"]).expect("first load");
    assert_eq!(f.loaded_languages(), ["en"]);

    f.load_languages(&["ru"]).expect("second load");
    assert_eq!(f.loaded_languages(), ["en", "ru"]);
    assert_eq!(f.available_languages().len(), available_before);
}

#[test]
fn reload_replaces_the_loaded_set() {
    let mut f = filter();
    f.reload_languages(&["en", "ru"]).unwrap();
    f.reload_languages(&["de"]).unwrap();
    assert_eq!(f.loaded_languages(), ["de"]);
}

#[test]
fn unload_drops_only_its_own_words() {
    let mut f = filter();
    f.load_languages(&["en", "ru"]).unwrap();
    assert!(f.is_profane("shit", Options::new()));

    f.unload_languages(&["en"]).unwrap();
    assert_eq!(f.loaded_languages(), ["ru"]);
    assert!(!f.is_profane("shit", Options::new()));
    assert!(f.is_profane("хуй", Options::new()));
}

#[test]
fn iso_codes_and_legacy_aliases_both_resolve() {
    let f = filter();
    for (alias, canonical) in [
        ("sp", "es"),
        ("du", "nl"),
        ("po", "pt"),
        ("gr", "el"),
        ("ua", "uk"),
        ("cz", "cs"),
        ("tu", "tr"),
        ("br", "pt_br"),
        ("in", "id"),
        ("sw", "sv"),
        ("lt", "es_419"),
    ] {
        assert_eq!(
            f.resolve_language(alias).unwrap(),
            canonical,
            "alias {alias}"
        );
        assert_eq!(f.resolve_language(canonical).unwrap(), canonical);
    }
}

#[test]
fn codes_are_normalized_before_lookup() {
    let f = filter();
    for code in ["ES_419", "es-419", " es_419 ", "Es-419"] {
        assert_eq!(f.resolve_language(code).unwrap(), "es_419", "code {code:?}");
    }
}

/// `lt` was Latin-American Spanish, `sw` was Swedish, `br` was Brazilian
/// Portuguese and `in` is a retired code - all four collide with a different
/// real language, so using them is reported.
#[test]
fn misleading_aliases_warn() {
    for (alias, canonical) in [
        ("lt", "es_419"),
        ("sw", "sv"),
        ("br", "pt_br"),
        ("in", "id"),
    ] {
        let mut f = filter();
        f.load_languages(&[alias]).unwrap();
        assert_eq!(f.loaded_languages(), [canonical]);

        let warned = f.warnings().iter().any(|w| {
            matches!(w, LanguageWarning::DeprecatedAlias { requested, canonical: c, .. }
                if requested == alias && c == canonical)
        });
        assert!(warned, "no deprecation warning for {alias}");
    }
}

#[test]
fn plain_aliases_do_not_warn() {
    let mut f = filter();
    f.load_languages(&["sp"]).unwrap();
    assert_eq!(f.loaded_languages(), ["es"]);
    assert!(
        !f.warnings()
            .iter()
            .any(|w| matches!(w, LanguageWarning::DeprecatedAlias { .. })),
        "unexpected deprecation warning"
    );
}

#[test]
fn unknown_language_reports_the_code() {
    let mut f = filter();
    match f.load_languages(&["xx"]) {
        Err(Error::UnknownLanguage { code, available }) => {
            assert_eq!(code, "xx");
            assert!(available.contains(&"en".to_string()));
        }
        other => panic!("expected UnknownLanguage, got {other:?}"),
    }
    assert!(
        f.loaded_languages().is_empty(),
        "nothing should have loaded"
    );
}

/// The word list path comes from the registry, so a caller-supplied string
/// never reaches the filesystem.
#[test]
fn language_codes_cannot_traverse_paths() {
    let mut f = filter();
    for attempt in [
        "../../../etc/passwd",
        "../en",
        "en/../../secret",
        "/etc/passwd",
    ] {
        assert!(
            matches!(
                f.load_languages(&[attempt]),
                Err(Error::UnknownLanguage { .. })
            ),
            "{attempt:?} was not rejected"
        );
    }
}

#[test]
fn every_shipped_language_loads_and_matches_its_own_words() {
    let f = filter();
    let codes: Vec<String> = f.available_languages().to_vec();
    assert_eq!(codes.len(), 25);

    for code in codes {
        let mut one = filter();
        one.load_languages(&[&code]).unwrap();
        assert!(one.word_count() > 0, "{code} loaded no words");
    }
}
