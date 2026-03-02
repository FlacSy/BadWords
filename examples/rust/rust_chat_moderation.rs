//! Chat moderation example
//!
//! Run: cargo run --example rust_chat_moderation

use badwords_core::{default_resource_dir, ProfanityFilter};

fn main() {
    let resource_dir = default_resource_dir();
    let mut filter = ProfanityFilter::new(&resource_dir, true, true, true, true);

    filter.init(Some(&["en".to_string(), "ru".to_string()])).expect("Failed to init");
    filter.add_words(&["spam_link".to_string(), "scam_bot".to_string()]);

    let messages = [
        "Hey! Check out this cool link",
        "Hello, how are you?",
        "Visit spam_link for free stuff",
        "This is scam_bot trying to reach you",
    ];

    for msg in messages {
        let (found, _) = filter.filter_text(msg, 1.0, None);
        let status = if found { "BLOCKED" } else { "OK" };
        println!("[{}] {}", status, msg);
    }
}
