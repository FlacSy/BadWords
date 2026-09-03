//! Dictionary storage: entries, language provenance, phrases and whitelist.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::fuzzy::{charset_mask, FuzzyIndex};

pub(crate) type EntryId = u32;

/// Bit reserved in `lang_mask` for words added through `add_words`.
pub(crate) const CUSTOM_LANG_BIT: u64 = 1 << 63;

/// Highest language index that fits alongside the custom bit.
pub(crate) const MAX_LANGUAGES: usize = 63;

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    /// Normalized form. Phrases keep single spaces between their tokens.
    pub form: Box<str>,
    /// The line as written in the word list, for reporting.
    pub raw: Box<str>,
    /// Bit per language, plus [`CUSTOM_LANG_BIT`].
    pub lang_mask: u64,
    pub token_count: u8,
    pub char_len: u16,
    pub charset: u64,
}

impl Entry {
    pub(crate) fn is_phrase(&self) -> bool {
        self.token_count > 1
    }
}

/// Word storage with one lookup on the hot path.
///
/// Provenance lives in the entry rather than the key, so the exact-match probe
/// costs the same as the plain hash set it replaces. Entries duplicated across
/// languages merge and OR their language bits.
#[derive(Debug, Default, Clone)]
pub(crate) struct Dictionary {
    entries: Vec<Entry>,
    by_form: FxHashMap<Box<str>, EntryId>,
    /// First token of a phrase to the phrases starting with it.
    phrase_by_first: FxHashMap<Box<str>, Vec<EntryId>>,
    max_phrase_tokens: usize,
    fuzzy: FuzzyIndex,
    lang_names: Vec<String>,
    whitelist: FxHashSet<Box<str>>,
}

impl Dictionary {
    /// Bit index for a language, allocating one on first use.
    pub(crate) fn language_bit(&mut self, code: &str) -> u64 {
        if let Some(pos) = self.lang_names.iter().position(|n| n == code) {
            return 1u64 << pos;
        }
        if self.lang_names.len() >= MAX_LANGUAGES {
            // Out of bits: the entry still matches, it just loses provenance.
            return 0;
        }
        self.lang_names.push(code.to_string());
        1u64 << (self.lang_names.len() - 1)
    }

    fn language_index(&self, code: &str) -> Option<u64> {
        self.lang_names
            .iter()
            .position(|n| n == code)
            .map(|pos| 1u64 << pos)
    }

    /// Add a normalized form, merging with an existing identical form.
    pub(crate) fn insert(&mut self, form: &str, raw: &str, lang_mask: u64) {
        if form.is_empty() {
            return;
        }
        if let Some(&id) = self.by_form.get(form) {
            self.entries[id as usize].lang_mask |= lang_mask;
            return;
        }

        let token_count = form.split(' ').filter(|t| !t.is_empty()).count().min(255) as u8;
        let id = self.entries.len() as EntryId;
        self.entries.push(Entry {
            form: form.into(),
            raw: raw.into(),
            lang_mask,
            token_count,
            char_len: form.chars().count().min(u16::MAX as usize) as u16,
            charset: charset_mask(form),
        });
        self.by_form.insert(form.into(), id);
    }

    /// Drop a form, or just its custom bit when a language also provides it.
    pub(crate) fn remove(&mut self, form: &str) -> bool {
        let Some(&id) = self.by_form.get(form) else {
            return false;
        };
        let entry = &mut self.entries[id as usize];
        entry.lang_mask &= !CUSTOM_LANG_BIT;
        if entry.lang_mask == 0 {
            entry.form = "".into();
            entry.char_len = 0;
            self.by_form.remove(form);
        }
        true
    }

    /// Drop every entry belonging only to these languages.
    pub(crate) fn remove_languages(&mut self, codes: &[String]) {
        let mask = codes
            .iter()
            .filter_map(|c| self.language_index(c))
            .fold(0u64, |acc, bit| acc | bit);
        if mask == 0 {
            return;
        }
        for entry in &mut self.entries {
            if entry.lang_mask & mask == 0 {
                continue;
            }
            entry.lang_mask &= !mask;
            if entry.lang_mask == 0 {
                self.by_form.remove(&entry.form);
                entry.form = "".into();
                entry.char_len = 0;
            }
        }
    }

    /// Drop every entry, keeping the whitelist.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.by_form.clear();
        self.phrase_by_first.clear();
        self.max_phrase_tokens = 0;
        self.fuzzy = FuzzyIndex::default();
    }

    /// Rebuild the phrase and fuzzy indices. Call once after a batch of edits.
    pub(crate) fn reindex(&mut self) {
        self.phrase_by_first.clear();
        self.max_phrase_tokens = 0;

        let mut fuzzy_items = Vec::with_capacity(self.entries.len());
        for (idx, entry) in self.entries.iter().enumerate() {
            if entry.form.is_empty() {
                continue;
            }
            let id = idx as EntryId;
            if entry.is_phrase() {
                let first = entry.form.split(' ').next().unwrap_or_default();
                self.phrase_by_first
                    .entry(first.into())
                    .or_default()
                    .push(id);
                self.max_phrase_tokens = self.max_phrase_tokens.max(entry.token_count as usize);
            }
            // Phrases stay in the fuzzy index: 2.x kept every entry in one flat
            // set, so a token could fuzzy-match a phrase as a plain string. The
            // new API filters them out at query time instead.
            fuzzy_items.push((id, entry.char_len, entry.charset));
        }

        for ids in self.phrase_by_first.values_mut() {
            ids.sort_unstable_by_key(|&id| {
                (std::cmp::Reverse(self.entries[id as usize].token_count), id)
            });
        }

        self.fuzzy = FuzzyIndex::build(fuzzy_items);
    }

    pub(crate) fn lookup(&self, form: &str) -> Option<EntryId> {
        self.by_form.get(form).copied()
    }

    pub(crate) fn entry(&self, id: EntryId) -> &Entry {
        &self.entries[id as usize]
    }

    pub(crate) fn is_phrase(&self, id: EntryId) -> bool {
        self.entries[id as usize].is_phrase()
    }

    /// Lowest set language bit's name, for `Match::language`.
    pub(crate) fn language_of(&self, id: EntryId) -> Option<&str> {
        let mask = self.entries[id as usize].lang_mask & !CUSTOM_LANG_BIT;
        if mask == 0 {
            return None;
        }
        let bit = mask.trailing_zeros() as usize;
        self.lang_names.get(bit).map(String::as_str)
    }

    pub(crate) fn fuzzy(&self) -> &FuzzyIndex {
        &self.fuzzy
    }

    pub(crate) fn entry_chars(&self, id: EntryId, out: &mut Vec<char>) {
        out.clear();
        out.extend(self.entries[id as usize].form.chars());
    }

    pub(crate) fn phrase_candidates(&self, first_token: &str) -> Option<&[EntryId]> {
        self.phrase_by_first.get(first_token).map(Vec::as_slice)
    }

    pub(crate) fn max_phrase_tokens(&self) -> usize {
        self.max_phrase_tokens
    }

    pub(crate) fn has_phrases(&self) -> bool {
        !self.phrase_by_first.is_empty()
    }

    /// Live entries, i.e. excluding tombstones.
    pub(crate) fn len(&self) -> usize {
        self.by_form.len()
    }

    /// Single-token entries, for the substring automaton.
    #[cfg(feature = "substring")]
    pub(crate) fn single_token_entries(&self) -> impl Iterator<Item = (EntryId, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.form.is_empty() && !e.is_phrase())
            .map(|(idx, e)| (idx as EntryId, e))
    }

    // -- whitelist ------------------------------------------------------------

    pub(crate) fn whitelist_insert(&mut self, form: &str) {
        if !form.is_empty() {
            self.whitelist.insert(form.into());
        }
    }

    pub(crate) fn whitelist_remove(&mut self, form: &str) {
        self.whitelist.remove(form);
    }

    pub(crate) fn whitelist_clear(&mut self) {
        self.whitelist.clear();
    }

    pub(crate) fn is_whitelisted(&self, form: &str) -> bool {
        !self.whitelist.is_empty() && self.whitelist.contains(form)
    }
}
