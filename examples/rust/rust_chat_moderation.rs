//! Chat moderation: block messages and censor them.
//!
//! Run: cargo run --example rust_chat_moderation

use badwords_core::{Options, ProfanityFilter};

fn main() -> Result<(), badwords_core::Error> {
    let mut filter = ProfanityFilter::builder()
        .embedded()
        .languages(["en", "ru"])
        .extra_words(["spam_link", "scam_bot"])
        // Words that must never be flagged, whatever a rule says.
        .whitelist(["assessment"])
        .build()?;

    let opts = Options::new();

    let messages = [
        "Hey! Check out this cool link",
        "Hello, how are you?",
        "Visit spam_link for free stuff",
        "This is scam_bot trying to reach you",
        "your assessment was wrong",
    ];

    for msg in messages {
        if filter.is_profane(msg, opts) {
            println!("[BLOCKED] {}", filter.censor(msg, '*', opts));
        } else {
            println!("[OK]      {msg}");
        }
    }

    // Batch API: one pass, one scratch buffer, no per-message allocation churn.
    let verdicts = filter.is_profane_many(&messages, opts);
    println!(
        "\nblocked {} of {}",
        verdicts.iter().filter(|v| **v).count(),
        messages.len()
    );

    let _ = &mut filter;
    Ok(())
}
