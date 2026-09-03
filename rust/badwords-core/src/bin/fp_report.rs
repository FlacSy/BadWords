//! Measures the false-positive cost of each opt-in detector.
//!
//! Run: `cargo run --release --bin fp_report --features substring`
//!
//! Reads a plain word list (one word per line) and reports how many of its
//! entries each option set flags. Words that are themselves in the profanity
//! dictionary are excluded first, so what remains is genuine over-matching.
//!
//! Default corpus is `/usr/share/dict/american-english` (Debian: `wamerican`).
//! Pass paths as arguments to measure other languages.

use std::collections::HashSet;
use std::path::PathBuf;

use badwords_core::{MatchMode, Options, ProfanityFilter};

const DEFAULT_CORPORA: &[&str] = &["/usr/share/dict/american-english"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let corpora: Vec<PathBuf> = if args.is_empty() {
        DEFAULT_CORPORA.iter().map(PathBuf::from).collect()
    } else {
        args.iter().map(PathBuf::from).collect()
    };

    let filter = ProfanityFilter::builder()
        .embedded()
        .all_languages()
        .build()
        .expect("embedded resources");

    println!(
        "dictionary: {} entries across {} languages\n",
        filter.word_count(),
        filter.loaded_languages().len()
    );

    let base = Options::new();
    let option_sets: &[(&str, Options)] = &[
        ("default (exact)", base),
        ("split_on_punctuation", base.split_on_punctuation(true)),
        ("collapse_repeats", base.collapse_repeats(true)),
        ("leetspeak", base.leetspeak(true)),
        ("fuzzy 0.95", base.threshold(0.95)),
        ("fuzzy 0.90", base.threshold(0.90)),
        (
            "substring len 4",
            base.match_mode(MatchMode::Substring).min_substring_len(4),
        ),
        (
            "substring len 5",
            base.match_mode(MatchMode::Substring).min_substring_len(5),
        ),
        (
            "substring len 6",
            base.match_mode(MatchMode::Substring).min_substring_len(6),
        ),
        (
            "substring len 7",
            base.match_mode(MatchMode::Substring).min_substring_len(7),
        ),
        ("aggressive()", Options::aggressive()),
    ];

    for path in corpora {
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("skipped (not readable): {}", path.display());
            continue;
        };

        // Drop possessives and anything the dictionary legitimately contains.
        let words: Vec<String> = text
            .lines()
            .map(|w| w.trim().trim_end_matches("'s").to_lowercase())
            .filter(|w| !w.is_empty() && !w.contains('\''))
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|w| !filter.contains_word(w))
            .collect();

        println!("== {} ({} clean words)", path.display(), words.len());
        for (name, opts) in option_sets {
            let flagged: Vec<&String> = words
                .iter()
                .filter(|w| filter.is_profane(w, *opts))
                .collect();
            let rate = 100.0 * flagged.len() as f64 / words.len() as f64;
            print!("  {name:<22} {:>6}  {rate:>6.3}%", flagged.len());
            if !flagged.is_empty() {
                let mut sample: Vec<&str> = flagged.iter().take(8).map(|w| w.as_str()).collect();
                sample.sort_unstable();
                print!("   e.g. {}", sample.join(", "));
            }
            println!();
        }
        println!();
    }
}
