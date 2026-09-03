//! Load specific languages, and use the opt-in evasion detectors.
//!
//! Run: cargo run --example rust_specific_languages

use badwords_core::{Options, ProfanityFilter};

fn main() -> Result<(), badwords_core::Error> {
    // ISO 639-1 codes; the pre-3.0 codes still work as aliases.
    let mut filter = ProfanityFilter::builder()
        .embedded()
        .languages(["en", "de"])
        .build()?;

    println!("Loaded: {:?}", filter.loaded_languages());
    println!(
        "Available: {} languages",
        filter.available_languages().len()
    );

    filter.add_words(&["badword"]);

    // Everything below is off by default, because each one trades false
    // negatives for false positives.
    let fuzzy = Options::new().threshold(0.9);
    println!("fuzzy 'badwrod': {}", filter.is_profane("badwrod", fuzzy));

    let evasion = Options::new()
        .split_on_punctuation(true)
        .collapse_repeats(true)
        .leetspeak(true);
    for text in ["bad-word", "baaadword", "b4dword"] {
        println!(
            "{text:>12}: default={} evasion={}",
            filter.is_profane(text, Options::new()),
            filter.is_profane(text, evasion)
        );
    }

    // A deprecated alias still resolves, and says so.
    let mut legacy = ProfanityFilter::builder().embedded().build()?;
    legacy.load_languages(&["sw"])?;
    for warning in legacy.warnings() {
        println!("warning: {warning}");
    }
    Ok(())
}
