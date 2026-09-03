//! Resource loading and the two shipped resource trees.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use badwords_core::{Error, Options, Processing, ProfanityFilter, ResourceSource};

fn resources() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
}

/// 2.x swallowed resource errors with `.ok()` and returned a filter that
/// silently detected nothing.
#[test]
fn missing_resources_are_an_error() {
    let err = ProfanityFilter::try_new(Path::new("/nonexistent/badwords"), Processing::default())
        .expect_err("should not succeed");
    assert!(matches!(err, Error::Io { .. }), "got {err:?}");
    assert!(err.to_string().contains("/nonexistent/badwords"));
}

#[test]
fn malformed_tables_are_an_error() {
    let source = ResourceSource::InMemory {
        unicode_mappings: "{ not json".to_string(),
        homoglyphs: "{}".to_string(),
        transliteration: r#"{"cyrillic_to_latin":{}}"#.to_string(),
        languages: None,
        words: HashMap::new(),
    };
    let err = ProfanityFilter::from_source(source, Processing::default()).expect_err("should fail");
    assert!(matches!(err, Error::Json { .. }), "got {err:?}");
}

/// The deprecated constructor cannot fail, but must not pretend it worked.
#[test]
#[allow(deprecated)]
fn legacy_new_yields_an_empty_filter_instead_of_panicking() {
    let f = ProfanityFilter::new(Path::new("/nonexistent/badwords"), true, true, true, true);
    assert!(f.available_languages().is_empty());
    assert!(!f.is_profane("shit", Options::new()));
}

#[test]
fn directory_and_embedded_resources_agree() {
    let from_dir = ProfanityFilter::builder()
        .resource_dir(resources())
        .all_languages()
        .build()
        .unwrap();
    let embedded = ProfanityFilter::builder()
        .embedded()
        .all_languages()
        .build()
        .unwrap();

    assert_eq!(
        from_dir.available_languages(),
        embedded.available_languages()
    );
    assert_eq!(from_dir.word_count(), embedded.word_count());
}

#[test]
fn in_memory_resources_work() {
    let mut words = HashMap::new();
    words.insert("en".to_string(), "badword\nanother".to_string());

    let source = ResourceSource::InMemory {
        unicode_mappings: "{}".to_string(),
        homoglyphs: "{}".to_string(),
        transliteration: r#"{"cyrillic_to_latin":{}}"#.to_string(),
        languages: None,
        words,
    };
    let mut f = ProfanityFilter::from_source(source, Processing::default()).unwrap();
    f.load_all_languages().unwrap();

    assert_eq!(f.loaded_languages(), ["en"]);
    assert!(f.is_profane("badword", Options::new()));
}

/// A byte-order mark must not end up glued to the first entry.
#[test]
fn byte_order_marks_are_stripped() {
    let mut words = HashMap::new();
    words.insert("en".to_string(), "\u{feff}badword\nsecond".to_string());

    let source = ResourceSource::InMemory {
        unicode_mappings: "{}".to_string(),
        homoglyphs: "{}".to_string(),
        transliteration: r#"{"cyrillic_to_latin":{}}"#.to_string(),
        languages: None,
        words,
    };
    let mut f = ProfanityFilter::from_source(source, Processing::none()).unwrap();
    f.load_all_languages().unwrap();
    assert!(f.contains_word("badword"));
}

/// `python/badwords/resource/` is a generated mirror of the crate's canonical
/// copy; `make sync-resources` regenerates it.
#[test]
fn python_mirror_is_in_sync() {
    let crate_dir = resources();
    let mirror = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../python/badwords/resource");
    if !mirror.exists() {
        // Building from a published crate: there is no repository around us.
        return;
    }

    let mut checked = 0usize;
    for sub in ["data", "words"] {
        let src = crate_dir.join(sub);
        for entry in std::fs::read_dir(&src).unwrap().flatten() {
            let name = entry.file_name();
            let mirrored = mirror.join(sub).join(&name);
            assert!(
                mirrored.exists(),
                "missing from mirror: {sub}/{}",
                name.to_string_lossy()
            );
            assert_eq!(
                std::fs::read(entry.path()).unwrap(),
                std::fs::read(&mirrored).unwrap(),
                "mirror differs: {sub}/{} - run `make sync-resources`",
                name.to_string_lossy()
            );
            checked += 1;
        }
        let mirrored_count = std::fs::read_dir(mirror.join(sub)).unwrap().count();
        let src_count = std::fs::read_dir(&src).unwrap().count();
        assert_eq!(mirrored_count, src_count, "extra files in mirror {sub}/");
    }
    assert!(checked > 25);
}
