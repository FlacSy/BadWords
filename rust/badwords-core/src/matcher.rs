//! The matching pipeline.

use crate::dict::{Dictionary, EntryId};
use crate::options::{MatchMode, Options, SpanMode};
use crate::processor::TextProcessor;
use crate::tokenize::{self, Segment};

/// How a match was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MatchKind {
    /// The token is a dictionary entry.
    Exact,
    /// The token is similar enough to an entry.
    Fuzzy,
    /// Matched after reading digits and symbols as letters.
    Leet,
    /// Matched after collapsing runs of repeated characters.
    Collapsed,
    /// An entry occurs inside the token.
    Substring,
    /// Consecutive tokens matched a multi-word entry.
    Phrase,
}

/// One detected occurrence.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Match {
    /// Dictionary entry that matched, as written in the word list.
    pub word: String,
    /// The matched slice of the input; always equal to `text[start..end]`.
    pub matched_text: String,
    /// Byte offset into the original text.
    pub start: usize,
    /// Byte offset into the original text, exclusive.
    pub end: usize,
    /// Language the entry came from, if it came from a word list.
    pub language: Option<String>,
    /// Similarity: `1.0` for anything but a fuzzy match.
    pub score: f64,
    /// How it was found.
    pub kind: MatchKind,
}

/// Reusable buffers, so that matching allocates nothing per call.
///
/// Hoist one out of a hot loop and pass it to
/// [`ProfanityFilter::find_into`](crate::ProfanityFilter::find_into).
#[derive(Debug, Default, Clone)]
pub struct Scratch {
    segments: Vec<Segment>,
    consumed: Vec<bool>,
    bases: Vec<Option<String>>,
    subtokens: Vec<(usize, usize)>,
    forms: Vec<(String, MatchKind)>,
    collapsed: Vec<String>,
    q_chars: Vec<char>,
    e_chars: Vec<char>,
    flags: Vec<bool>,
}

impl Scratch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Everything the pipeline reads from.
pub(crate) struct Ctx<'a> {
    pub dict: &'a Dictionary,
    pub processor: &'a TextProcessor,
    #[cfg(feature = "substring")]
    pub automaton: Option<&'a crate::substring::Automaton>,
}

/// Run the pipeline. With `first_only`, returns as soon as one match is found.
///
/// Segments are walked once, left to right, and a segment's normalized form is
/// computed only when it is reached. Normalization dominates the cost of
/// matching, so a hit early in the text must not pay for the whole of it.
pub(crate) fn find_into(
    ctx: &Ctx<'_>,
    text: &str,
    opts: Options,
    scratch: &mut Scratch,
    out: &mut Vec<Match>,
    first_only: bool,
) {
    out.clear();
    if text.is_empty() {
        return;
    }

    tokenize::segments(text, &mut scratch.segments);
    let count = scratch.segments.len();
    if count == 0 {
        return;
    }

    scratch.bases.clear();
    scratch.bases.resize(count, None);
    scratch.consumed.clear();
    scratch.consumed.resize(count, false);

    let whole = opts.span == SpanMode::WholeSegment;
    let phrases = opts.phrases && ctx.dict.has_phrases();

    let mut index = 0usize;
    while index < count {
        if scratch.consumed[index] {
            index += 1;
            continue;
        }

        ensure_base(ctx, text, scratch, index, whole);
        if scratch.bases[index]
            .as_deref()
            .unwrap_or_default()
            .is_empty()
        {
            index += 1;
            continue;
        }

        // A phrase starting here wins over a single-word match on its first
        // word, which is why it is tried first.
        if phrases {
            if let Some(consumed) = match_phrase(ctx, text, scratch, out, index, whole) {
                if first_only {
                    return;
                }
                index = consumed;
                continue;
            }
        }

        let seg = scratch.segments[index];
        let (span_start, span_end) = primary_span(&seg, whole);
        let base = scratch.bases[index].take().unwrap_or_default();
        let found = match_unit(ctx, text, (span_start, span_end), &base, opts, scratch, out);
        scratch.bases[index] = Some(base);

        if found {
            if first_only {
                return;
            }
            index += 1;
            continue;
        }

        if opts.split_on_punctuation {
            tokenize::subtokens(text, span_start, span_end, &mut scratch.subtokens);
            let pieces = std::mem::take(&mut scratch.subtokens);
            for &(piece_start, piece_end) in &pieces {
                let sub_base = ctx.processor.process_text(&text[piece_start..piece_end]);
                let hit = match_unit(
                    ctx,
                    text,
                    (piece_start, piece_end),
                    &sub_base,
                    opts,
                    scratch,
                    out,
                );
                if hit && first_only {
                    scratch.subtokens = pieces;
                    return;
                }
            }
            scratch.subtokens = pieces;
        }

        index += 1;
    }

    resolve_overlaps(out);
    if let Some(max) = opts.max_matches {
        out.truncate(max);
    }
}

/// Fill in a segment's normalized form if it has not been computed yet.
fn ensure_base(ctx: &Ctx<'_>, text: &str, scratch: &mut Scratch, index: usize, whole: bool) {
    if scratch.bases[index].is_some() {
        return;
    }
    let (start, end) = primary_span(&scratch.segments[index], whole);
    let base = if start < end {
        ctx.processor.process_text(&text[start..end])
    } else {
        String::new()
    };
    scratch.bases[index] = Some(base);
}

/// Try every multi-word entry beginning at `index`.
///
/// Exact only: comparing a four-token phrase against arbitrary token pairs
/// fuzzily is both meaningless and slow. Segments that normalize to nothing -
/// a lone dash, say - are skipped rather than breaking a phrase, so an entry
/// written `son - of a bitch` still matches `son of a bitch`.
///
/// Returns the index to continue from when a phrase matched.
fn match_phrase(
    ctx: &Ctx<'_>,
    text: &str,
    scratch: &mut Scratch,
    out: &mut Vec<Match>,
    index: usize,
    whole: bool,
) -> Option<usize> {
    let first_base = scratch.bases[index].as_deref()?;
    let candidates = ctx.dict.phrase_candidates(first_base)?.to_vec();
    let max_tokens = ctx.dict.max_phrase_tokens();
    let count = scratch.segments.len();

    // Positions of the next few segments that carry content.
    let mut window: Vec<usize> = Vec::with_capacity(max_tokens);
    window.push(index);
    let mut cursor = index + 1;
    while window.len() < max_tokens && cursor < count {
        ensure_base(ctx, text, scratch, cursor, whole);
        if !scratch.bases[cursor]
            .as_deref()
            .unwrap_or_default()
            .is_empty()
        {
            window.push(cursor);
        }
        cursor += 1;
    }

    'candidates: for id in candidates {
        let entry = ctx.dict.entry(id);
        let tokens = entry.token_count as usize;
        if tokens < 2 || tokens > window.len() {
            continue;
        }

        let mut expected = entry.form.split(' ');
        for &segment in &window[..tokens] {
            let Some(token) = expected.next() else {
                continue 'candidates;
            };
            if scratch.consumed[segment]
                || scratch.bases[segment].as_deref().unwrap_or_default() != token
            {
                continue 'candidates;
            }
        }
        if window[..tokens].iter().any(|&segment| {
            ctx.dict
                .is_whitelisted(scratch.bases[segment].as_deref().unwrap_or_default())
        }) {
            continue;
        }

        let (start, _) = primary_span(&scratch.segments[index], whole);
        let last = window[tokens - 1];
        let (_, end) = primary_span(&scratch.segments[last], whole);
        out.push(make_match(
            ctx,
            text,
            start,
            end,
            id,
            1.0,
            MatchKind::Phrase,
        ));
        for &segment in &window[..tokens] {
            scratch.consumed[segment] = true;
        }
        return Some(last + 1);
    }

    None
}

fn primary_span(seg: &Segment, whole: bool) -> (usize, usize) {
    if whole {
        (seg.start, seg.end)
    } else {
        (seg.core_start, seg.core_end)
    }
}

/// Try one unit against the dictionary. Returns whether a match was pushed.
fn match_unit(
    ctx: &Ctx<'_>,
    text: &str,
    span: (usize, usize),
    base: &str,
    opts: Options,
    scratch: &mut Scratch,
    out: &mut Vec<Match>,
) -> bool {
    let (start, end) = span;
    if base.is_empty() {
        return false;
    }
    if ctx.dict.is_whitelisted(base) {
        return false;
    }

    build_forms(ctx, text, span, base, opts, scratch);
    let forms = std::mem::take(&mut scratch.forms);

    let mut hit = None;
    for (form, kind) in &forms {
        if form.is_empty() || ctx.dict.is_whitelisted(form) {
            continue;
        }
        if let Some(id) = ctx.dict.lookup(form) {
            if !ctx.dict.entry(id).is_phrase() {
                hit = Some((id, 1.0, *kind));
                break;
            }
        }
    }

    if hit.is_none() && opts.is_fuzzy() && !ctx.dict.fuzzy().is_empty() {
        for (form, kind) in &forms {
            if form.is_empty() || ctx.dict.is_whitelisted(form) {
                continue;
            }
            scratch.q_chars.clear();
            scratch.q_chars.extend(form.chars());
            let found = ctx.dict.fuzzy().best(
                &scratch.q_chars,
                opts.match_threshold,
                |id, buf| ctx.dict.entry_chars(id, buf),
                // A single token fuzzy-matching a multi-word entry only makes
                // sense in the legacy flat-set mode.
                |id| !opts.phrases || !ctx.dict.is_phrase(id),
                &mut scratch.e_chars,
                &mut scratch.flags,
            );
            if let Some((id, score)) = found {
                let kind = if *kind == MatchKind::Exact {
                    MatchKind::Fuzzy
                } else {
                    *kind
                };
                hit = Some((id, score, kind));
                break;
            }
        }
    }

    #[cfg(feature = "substring")]
    if hit.is_none() && opts.match_mode == MatchMode::Substring {
        if let Some(automaton) = ctx.automaton {
            if let Some(id) = automaton.find(base, opts.min_substring_len, ctx.dict) {
                hit = Some((id, 1.0, MatchKind::Substring));
            }
        }
    }
    #[cfg(not(feature = "substring"))]
    let _ = MatchMode::Substring;

    scratch.forms = forms;

    match hit {
        Some((id, score, kind)) => {
            out.push(make_match(ctx, text, start, end, id, score, kind));
            true
        }
        None => false,
    }
}

/// Candidate forms for a unit, base form first.
fn build_forms(
    ctx: &Ctx<'_>,
    text: &str,
    span: (usize, usize),
    base: &str,
    opts: Options,
    scratch: &mut Scratch,
) {
    let (start, end) = span;
    scratch.forms.clear();
    scratch.forms.push((base.to_string(), MatchKind::Exact));

    if opts.leetspeak {
        // Leet substitution happens on the raw slice, before normalization,
        // because `@` and `$` do not survive the alphanumeric filter.
        if let Some(decoded) = tokenize::leet_form(&text[start..end]) {
            let normalized = ctx.processor.process_text(&decoded);
            if normalized != base {
                scratch.forms.push((normalized, MatchKind::Leet));
            }
        }
    }

    if opts.collapse_repeats {
        let collapsed = std::mem::take(&mut scratch.collapsed);
        let mut collapsed = collapsed;
        tokenize::collapse_forms(base, &mut collapsed);
        for form in &collapsed {
            scratch.forms.push((form.clone(), MatchKind::Collapsed));
        }
        scratch.collapsed = collapsed;
    }
}

fn make_match(
    ctx: &Ctx<'_>,
    text: &str,
    start: usize,
    end: usize,
    id: EntryId,
    score: f64,
    kind: MatchKind,
) -> Match {
    let entry = ctx.dict.entry(id);
    Match {
        word: entry.raw.to_string(),
        matched_text: text[start..end].to_string(),
        start,
        end,
        language: ctx.dict.language_of(id).map(str::to_owned),
        score,
        kind,
    }
}

/// Keep matches non-overlapping, preferring the earliest and then the longest.
fn resolve_overlaps(out: &mut Vec<Match>) {
    if out.len() < 2 {
        return;
    }
    out.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    let mut kept: Vec<Match> = Vec::with_capacity(out.len());
    let mut last_end = 0usize;
    for m in out.drain(..) {
        if kept.is_empty() || m.start >= last_end {
            last_end = m.end;
            kept.push(m);
        }
    }
    *out = kept;
}
