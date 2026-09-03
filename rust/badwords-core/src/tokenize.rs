//! Segmentation and candidate-form generation.
//!
//! Byte spans are produced here and nowhere else. Derived forms (leet,
//! collapsed repeats) never change a span, which keeps span arithmetic
//! independent of normalization.

/// A whitespace-delimited piece of the input, plus its alphanumeric core.
///
/// `core_*` is what a match reports by default, so that punctuation around a
/// word survives censoring: `"hey fuck, ok"` censors to `"hey ****, ok"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Segment {
    pub start: usize,
    pub end: usize,
    pub core_start: usize,
    pub core_end: usize,
}

/// Split on whitespace and trim each piece to its alphanumeric core.
pub(crate) fn segments(text: &str, out: &mut Vec<Segment>) {
    out.clear();
    let mut start = 0usize;
    let mut in_word = false;

    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if in_word {
                out.push(with_core(text, start, i));
                in_word = false;
            }
        } else if !in_word {
            start = i;
            in_word = true;
        }
    }
    if in_word {
        out.push(with_core(text, start, text.len()));
    }
}

fn with_core(text: &str, start: usize, end: usize) -> Segment {
    let slice = &text[start..end];
    let core_start = slice
        .char_indices()
        .find(|(_, c)| c.is_alphanumeric())
        .map_or(end, |(i, _)| start + i);
    let core_end = slice
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphanumeric())
        .map_or(core_start, |(i, c)| start + i + c.len_utf8());

    Segment {
        start,
        end,
        core_start,
        core_end,
    }
}

/// Pieces of `text[start..end]` separated by runs of non-alphanumeric chars.
///
/// Only used when `split_on_punctuation` is on. The glued form is checked
/// separately and first, so `f.u.c.k` keeps matching `fuck` either way while
/// `fuck-you` and `you.fuck` start matching too.
pub(crate) fn subtokens(text: &str, start: usize, end: usize, out: &mut Vec<(usize, usize)>) {
    out.clear();
    let slice = &text[start..end];
    let mut piece_start: Option<usize> = None;

    for (i, c) in slice.char_indices() {
        if c.is_alphanumeric() {
            if piece_start.is_none() {
                piece_start = Some(i);
            }
        } else if let Some(s) = piece_start.take() {
            out.push((start + s, start + i));
        }
    }
    if let Some(s) = piece_start {
        out.push((start + s, end));
    }

    // A single piece is just the glued form again.
    if out.len() < 2 {
        out.clear();
    }
}

/// `4 -> a`, `3 -> e`, ... Applied to the raw token before normalization,
/// because `@` and `$` are dropped by the alphanumeric filter.
fn leet_char(c: char) -> Option<char> {
    Some(match c {
        '4' => 'a',
        '3' => 'e',
        '1' => 'i',
        '0' => 'o',
        '5' => 's',
        '7' => 't',
        '@' => 'a',
        '$' => 's',
        _ => return None,
    })
}

/// Leet-decoded form of a raw token, or `None` when the guards reject it.
///
/// Guards, all needed:
/// - a token with no letters is skipped, else `455` reads as `ass`
/// - `123`, `1st`, `100k` and friends are skipped
/// - the result is an *extra* candidate: the literal form is still tested, so
///   dictionary entries that are themselves leet (`a55`, `5hit`) keep matching
pub(crate) fn leet_form(raw: &str) -> Option<String> {
    if !raw.chars().any(char::is_alphabetic) {
        return None;
    }
    if is_number_like(raw) {
        return None;
    }

    let mut changed = false;
    let decoded: String = raw
        .chars()
        .map(|c| match leet_char(c) {
            Some(mapped) => {
                changed = true;
                mapped
            }
            None => c,
        })
        .collect();

    changed.then_some(decoded)
}

/// `123`, `1st`, `2nd`, `100k`: digits followed by at most two letters.
fn is_number_like(raw: &str) -> bool {
    let mut chars = raw.chars();
    let mut digits = 0usize;
    let mut letters = 0usize;

    for c in chars.by_ref() {
        if c.is_ascii_digit() {
            if letters > 0 {
                return false;
            }
            digits += 1;
        } else if c.is_alphabetic() {
            letters += 1;
        } else {
            return false;
        }
    }
    digits > 0 && letters <= 2
}

/// Forms with runs of repeated characters collapsed.
///
/// Runs of exactly two are only touched at the start of the word. Collapsing
/// every doubled letter would make `book`, `boot`, `cook`, `cassette` and
/// `assess` profane; these three rules produce no false positive on the whole
/// American English dictionary.
pub(crate) fn collapse_forms(base: &str, out: &mut Vec<String>) {
    out.clear();
    if base.is_empty() {
        return;
    }
    let chars: Vec<char> = base.chars().collect();

    let c1 = collapse_runs(&chars, 1);
    if c1 != base {
        out.push(c1);
    }
    let c2 = collapse_runs(&chars, 2);
    if c2 != base && !out.contains(&c2) {
        out.push(c2);
    }
    if chars.len() >= 3 && chars[0] == chars[1] && chars[1] != chars[2] {
        let c3: String = chars[1..].iter().collect();
        if !out.contains(&c3) {
            out.push(c3);
        }
    }
}

/// Collapse every run of three or more identical chars down to `keep`.
fn collapse_runs(chars: &[char], keep: usize) -> String {
    let mut out = String::with_capacity(chars.len());
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        let mut run = 1usize;
        while i + run < chars.len() && chars[i + run] == c {
            run += 1;
        }
        let emit = if run >= 3 { keep.min(run) } else { run };
        for _ in 0..emit {
            out.push(c);
        }
        i += run;
    }
    out
}
