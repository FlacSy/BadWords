//! Load specific languages only
//!
//! Run: cargo run --example rust_specific_languages

use badwords_core::{default_resource_dir, ProfanityFilter};

fn main() {
    let resource_dir = default_resource_dir();
    let mut filter = ProfanityFilter::new(&resource_dir, true, true, true, true);

    // Load only English and German
    let languages = vec!["en".to_string(), "de".to_string()];
    filter.init(Some(&languages)).expect("Failed to init");

    println!("Loaded languages: {:?}", filter.get_all_languages());

    // Fuzzy matching with threshold
    filter.add_words(&["badword".to_string()]);
    let (found, _) = filter.filter_text("badw0rd", 0.9, None);
    println!("Fuzzy match 'badw0rd': {}", found);
}
