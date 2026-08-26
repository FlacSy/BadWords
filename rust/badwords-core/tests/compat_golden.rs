//! Replays a behavioural snapshot of 2.3.1 through the deprecated API.
//!
//! This is what proves "default behaviour is unchanged" in 3.0.0. Everything
//! else is assertion by inspection. See `tests/fixtures/README.md`.

#![allow(deprecated)]

use std::path::PathBuf;

use badwords_core::ProfanityFilter;
use serde::Deserialize;

#[derive(Deserialize)]
struct Header {
    dict: Vec<String>,
}

#[derive(Deserialize)]
struct Case {
    cfg: [bool; 4],
    text: String,
    /// `[threshold, replace_char, found, output]`
    results: Vec<(f64, Option<String>, bool, Option<String>)>,
}

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
}

#[test]
fn legacy_filter_text_matches_2_3_1() {
    let raw = include_str!("fixtures/legacy_golden.jsonl");
    let mut lines = raw.lines().filter(|l| !l.trim().is_empty());

    let header: Header =
        serde_json::from_str(lines.next().expect("fixture is empty")).expect("bad header");

    let mut checked = 0usize;
    let mut current: Option<([bool; 4], ProfanityFilter)> = None;

    for line in lines {
        let case: Case = serde_json::from_str(line).expect("bad case");

        // Cases are grouped by config, so the filter is rebuilt only 4 times.
        let filter = match &mut current {
            Some((cfg, f)) if *cfg == case.cfg => f,
            _ => {
                let mut f = ProfanityFilter::new(
                    &resource_dir(),
                    case.cfg[0],
                    case.cfg[1],
                    case.cfg[2],
                    case.cfg[3],
                );
                f.init(Some(&[])).expect("init with no languages");
                f.add_words(&header.dict);
                current = Some((case.cfg, f));
                &mut current.as_mut().unwrap().1
            }
        };

        for (threshold, replace, want_found, want_output) in case.results {
            let replace_char = replace.as_ref().and_then(|s| s.chars().next());
            let (found, output) = filter.filter_text(&case.text, threshold, replace_char);

            assert_eq!(
                found, want_found,
                "found mismatch for {:?} cfg={:?} threshold={} replace={:?}",
                case.text, case.cfg, threshold, replace
            );
            assert_eq!(
                output, want_output,
                "output mismatch for {:?} cfg={:?} threshold={} replace={:?}",
                case.text, case.cfg, threshold, replace
            );
            checked += 1;
        }
    }

    assert_eq!(checked, 8736, "fixture case count changed");
}
