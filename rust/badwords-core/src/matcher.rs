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
    bases: Vec<String>,
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
    if scratch.segments.is_empty() {
        return;
    }

    let whole = opts.span == SpanMode::WholeSegment;

    // Base form of each segment's primary unit.
    scratch.bases.clear();
    for seg in &scratch.segments {
        let (s, e) = primary_span(seg, whole);
        scratch.bases.push(if s < e {
            ctx.processor.process_text(&text[s..e])
        } else {
            String::new()
        });
    }

    scratch.consumed.clear();
    scratch.consumed.resize(scratch.segments.len(), false);

    if opts.phrases && ctx.dict.has_phrases() {
        phrase_pass(ctx, text, opts, scratch, out, whole, first_only);
        if first_only && !out.is_empty() {
            return;
        }
    }

    for idx in 0..scratch.segments.len() {
        if scratch.consumed[idx] {
            continue;
        }
        let seg = scratch.segments[idx];
        let (span_start, span_end) = primary_span(&seg, whole);
        if span_start >= span_end && !whole {
            continue;
        }

        // The glued form is always tested first, so `f.u.c.k` keeps matching.
        let base = std::mem::take(&mut scratch.bases[idx]);
        let found = match_unit(ctx, text, (span_start, span_end), &base, opts, scratch, out);
        scratch.bases[idx] = base;

        if found {
            if first_only {
                return;
            }
            continue;
        }

        if opts.split_on_punctuation {
            tokenize::subtokens(text, span_start, span_end, &mut scratch.subtokens);
            let pieces = std::mem::take(&mut scratch.subtokens);
            for &(s, e) in &pieces {
                let sub_base = ctx.processor.process_text(&text[s..e]);
                if match_unit(ctx, text, (s, e), &sub_base, opts, scratch, out) && first_only {
                    scratch.subtokens = pieces;
                    return;
                }
            }
            scratch.subtokens = pieces;
        }
    }

    resolve_overlaps(out);
    if let Some(max) = opts.max_matches {
        out.truncate(max);
    }
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

/// Match multi-word entries against runs of consecutive segments.
///
/// Exact only: comparing a four-token phrase against arbitrary token pairs
/// fuzzily is both meaningless and slow.
fn phrase_pass(
    ctx: &Ctx<'_>,
    text: &str,
    opts: Options,
    scratch: &mut Scratch,
    out: &mut Vec<Match>,
    whole: bool,
    first_only: bool,
) {
    let n = scratch.segments.len();
    let max_tokens = ctx.dict.max_phrase_tokens();
    let mut i = 0usize;

    while i < n {
        if scratch.consumed[i] || scratch.bases[i].is_empty() {
            i += 1;
            continue;
        }
        let Some(candidates) = ctx.dict.phrase_candidates(&scratch.bases[i]) else {
            i += 1;
            continue;
        };

        let mut matched = 0usize;
        'candidates: for &id in candidates {
            let entry = ctx.dict.entry(id);
            let k = entry.token_count as usize;
            if k < 2 || k > max_tokens || i + k > n {
                continue;
            }
            let mut tokens = entry.form.split(' ');
            for offset in 0..k {
                let Some(token) = tokens.next() else {
                    continue 'candidates;
                };
                if scratch.consumed[i + offset] || scratch.bases[i + offset] != token {
                    continue 'candidates;
                }
            }
            if (i..i + k).any(|idx| ctx.dict.is_whitelisted(&scratch.bases[idx])) {
                continue;
            }

            let (start, _) = primary_span(&scratch.segments[i], whole);
            let (_, end) = primary_span(&scratch.segments[i + k - 1], whole);
            out.push(make_match(
                ctx,
                text,
                start,
                end,
                id,
                1.0,
                MatchKind::Phrase,
            ));
            for idx in i..i + k {
                scratch.consumed[idx] = true;
            }
            matched = k;
            break;
        }

        if matched > 0 {
            if first_only {
                return;
            }
            i += matched;
        } else {
            i += 1;
        }
        let _ = opts;
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
