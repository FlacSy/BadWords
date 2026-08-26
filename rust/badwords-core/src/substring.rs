//! Substring matching for glued evasion (`fuckyou`).
//!
//! Off by default: an entry occurring inside a longer word is the Scunthorpe
//! problem, so this is opt-in, guarded by a minimum entry length and by the
//! whitelist. Measured on a clean 73k-word English corpus, the false-positive
//! rate is 2.35% at length 4, 0.64% at 5, 0.23% at 6 (the default) and 0.03% at 7.

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

use crate::dict::{Dictionary, EntryId};

/// Automaton over every single-token entry, built on first use.
#[derive(Debug)]
pub(crate) struct Automaton {
    inner: AhoCorasick,
    /// Pattern index to entry id.
    ids: Vec<EntryId>,
}

impl Automaton {
    pub(crate) fn build(dict: &Dictionary) -> Option<Self> {
        let mut patterns: Vec<&str> = Vec::new();
        let mut ids: Vec<EntryId> = Vec::new();
        for (id, entry) in dict.single_token_entries() {
            patterns.push(&entry.form);
            ids.push(id);
        }
        if patterns.is_empty() {
            return None;
        }
        let inner = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build(&patterns)
            .ok()?;
        Some(Self { inner, ids })
    }

    /// First entry of at least `min_len` characters occurring in `haystack`.
    ///
    /// `min_len` is applied per hit rather than at build time, because it is a
    /// per-call option while the automaton is built once.
    pub(crate) fn find(
        &self,
        haystack: &str,
        min_len: usize,
        dict: &Dictionary,
    ) -> Option<EntryId> {
        for hit in self.inner.find_iter(haystack) {
            let id = self.ids[hit.pattern().as_usize()];
            let entry = dict.entry(id);
            if (entry.char_len as usize) < min_len {
                continue;
            }
            if dict.is_whitelisted(&entry.form) {
                continue;
            }
            return Some(id);
        }
        None
    }
}
