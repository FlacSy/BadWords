//! BadWords core - profanity filter logic.

mod filter;
mod processor;

pub use filter::{NotSupportedLanguage, ProfanityFilter};
pub use processor::TextProcessor;

/// Resource directory path when running from workspace (for examples).
pub fn default_resource_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../python/badwords/resource")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_test_filter() -> ProfanityFilter {
        let words_by_lang: HashMap<String, String> = [
            ("en".to_string(), "bad\nword\ntest".to_string()),
            ("ru".to_string(), "плохо\nслово".to_string()),
        ]
        .into();

        ProfanityFilter::new_from_embedded(
            "{}",
            "{}",
            "{}",
            &words_by_lang,
            vec!["en".to_string(), "ru".to_string()],
            true,
            true,
            false,
            false,
        )
        .unwrap()
    }

    #[test]
    fn test_clean_text_returns_false() {
        let filter = make_test_filter();
        let (found, _) = filter.filter_text("hello world", 1.0, None);
        assert!(!found);
    }

    #[test]
    fn test_bad_text_returns_true() {
        let filter = make_test_filter();
        let (found, _) = filter.filter_text("bad", 1.0, None);
        assert!(found);
    }

    #[test]
    fn test_add_words_detection() {
        let mut filter = make_test_filter();
        filter.add_words(&["custombad".to_string()]);
        let (found, _) = filter.filter_text("custombad", 1.0, None);
        assert!(found);
    }

    #[test]
    fn test_censor_replaces_word() {
        let mut filter = make_test_filter();
        filter.add_words(&["bad".to_string()]);
        let (found, result) = filter.filter_text("a bad thing", 1.0, Some('*'));
        assert!(found);
        assert_eq!(result, Some("a *** thing".to_string()));
    }

    #[test]
    fn test_get_all_languages() {
        let filter = make_test_filter();
        let langs = filter.get_all_languages();
        assert_eq!(langs, &["en", "ru"]);
    }

    #[test]
    fn test_similar() {
        let filter = make_test_filter();
        assert!((filter.similar("hello", "hello") - 1.0).abs() < 0.001);
        assert!(filter.similar("abc", "xyz") < 1.0);
    }
}
