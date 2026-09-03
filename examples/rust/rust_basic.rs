//! Basic usage of badwords-core.
//!
//! Run: cargo run --example rust_basic

use badwords_core::{Options, ProfanityFilter};

fn main() -> Result<(), badwords_core::Error> {
    // Resources are compiled into the crate, so nothing has to be located at
    // runtime. Use `.resource_dir(path)` for a custom set of word lists.
    let mut filter = ProfanityFilter::builder()
        .embedded()
        .all_languages()
        .build()?;

    let opts = Options::new();

    println!(
        "'hello world' is profane: {}",
        filter.is_profane("hello world", opts)
    );
    println!(
        "'sonofabitch' is profane: {}",
        filter.is_profane("sonofabitch", opts)
    );

    filter.add_words(&["custombad"]);
    println!(
        "'custombad' is profane: {}",
        filter.is_profane("custombad", opts)
    );

    // Censoring keeps everything that is not part of a match, punctuation included.
    filter.add_words(&["bad"]);
    println!("Censored: {}", filter.censor("a bad word, ok?", '*', opts));

    // `find` reports where each match is and which language it came from.
    for m in filter.find("what a bad, shitty day", opts) {
        println!(
            "  {:?} at {}..{} (word {:?}, language {:?}, {:?})",
            m.matched_text, m.start, m.end, m.word, m.language, m.kind
        );
    }

    println!("\nLoaded languages: {}", filter.loaded_languages().len());
    Ok(())
}
