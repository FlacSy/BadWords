//! Basic usage of badwords-rs
//!
//! Run: cargo run --example rust_basic

use badwords_core::{default_resource_dir, ProfanityFilter};

fn main() {
    let resource_dir = default_resource_dir();
    let mut filter = ProfanityFilter::new(&resource_dir, true, true, true, true);

    filter.init(None).expect("Failed to init");

    // Check clean text
    let (found, _) = filter.filter_text("hello world", 1.0, None);
    println!("'hello world' contains profanity: {}", found);

    // Check text with profanity
    let (found, _) = filter.filter_text("sonofabitch", 1.0, None);
    println!("'sonofabitch' contains profanity: {}", found);

    // Add custom words
    filter.add_words(&["custombad".to_string()]);
    let (found, _) = filter.filter_text("custombad", 1.0, None);
    println!("'custombad' (custom) contains profanity: {}", found);

    // Censoring (add word for predictable result)
    filter.add_words(&["bad".to_string()]);
    let (found, result) = filter.filter_text("a bad word", 1.0, Some('*'));
    if let (true, Some(censored)) = (found, result) {
        println!("Censored: {}", censored);
    }

    println!("\nAvailable languages: {:?}", filter.get_all_languages());
}
