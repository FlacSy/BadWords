//! `ProfanityFilter` must stay `Send + Sync`.
//!
//! The Python bindings release the GIL around matching, and the batch API hands
//! the filter to worker threads. Anything cached behind a `Cell`/`RefCell` on
//! the struct would break both, so this is a hard constraint rather than a
//! nice-to-have.

use badwords_core::{Match, Options, ProfanityFilter, Scratch};

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn public_types_are_send_and_sync() {
    assert_send_sync::<ProfanityFilter>();
    assert_send_sync::<Options>();
    assert_send_sync::<Match>();
    assert_send_sync::<Scratch>();
}

#[test]
fn a_shared_filter_can_be_used_from_several_threads() {
    let filter = ProfanityFilter::builder()
        .embedded()
        .languages(["en"])
        .build()
        .unwrap();
    let filter = std::sync::Arc::new(filter);

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let filter = std::sync::Arc::clone(&filter);
            std::thread::spawn(move || {
                let text = if i % 2 == 0 {
                    "a shitty day"
                } else {
                    "a fine day"
                };
                filter.is_profane(text, Options::new())
            })
        })
        .collect();

    let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results, [true, false, true, false]);
}
