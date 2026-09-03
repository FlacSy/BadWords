//! Indexed fuzzy matching.
//!
//! The 2.x matcher compared every token against every dictionary entry with
//! `strsim::jaro_winkler`, which also allocates a flag vector per call - about
//! 14 000 allocations per token on the full multilingual dictionary.
//!
//! This module keeps the results identical and skips the work instead, using an
//! upper bound on Jaro-Winkler that can be computed from precomputed per-entry
//! metadata:
//!
//! ```text
//! m   <= min(|q|, |e|, D_qe, D_eq)      D_qe = positions of q whose char occurs in e
//! J   <= (m/|q| + m/|e| + 1) / 3        transpositions only lower J
//! JW  <= 0.4 + 0.6 * J                  prefix bonus is at most 0.1 * 4 * (1 - J)
//! ```
//!
//! The bound holds on both sides of the `sim > 0.7` gate in `strsim`, because
//! `0.4 + 0.6x >= x` for `x <= 1`. Property tests check both the bound and the
//! equivalence of the index with a brute-force scan.

/// Bit of the 64-wide character-set mask a char falls into.
#[inline]
fn char_bit(c: char) -> u64 {
    1u64 << ((c as u32) & 63)
}

/// Character-set mask of a string. Collisions are safe: they only widen the set
/// and therefore loosen the bound.
#[must_use]
pub(crate) fn charset_mask(s: &str) -> u64 {
    s.chars().fold(0u64, |mask, c| mask | char_bit(c))
}

#[must_use]
pub(crate) fn charset_mask_chars(chars: &[char]) -> u64 {
    chars.iter().fold(0u64, |mask, &c| mask | char_bit(c))
}

/// Jaro similarity, allocation-free. Port of `strsim::generic_jaro` (0.11.1).
#[must_use]
pub(crate) fn jaro_chars(a: &[char], b: &[char], flags: &mut Vec<bool>) -> f64 {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 && b_len == 0 {
        return 1.0;
    } else if a_len == 0 || b_len == 0 {
        return 0.0;
    }

    let search_range = (a_len.max(b_len) / 2).saturating_sub(1);

    flags.clear();
    flags.resize(a_len + b_len, false);
    let (a_flags, b_flags) = flags.split_at_mut(a_len);

    let mut matches = 0usize;
    for (i, &a_elem) in a.iter().enumerate() {
        let min_bound = i.saturating_sub(search_range);
        let max_bound = b_len.min(i + search_range + 1);

        for (j, &b_elem) in b.iter().enumerate().take(max_bound) {
            if min_bound <= j && a_elem == b_elem && !b_flags[j] {
                a_flags[i] = true;
                b_flags[j] = true;
                matches += 1;
                break;
            }
        }
    }

    let mut transpositions = 0usize;
    if matches != 0 {
        let mut b_iter = b_flags.iter().zip(b.iter());
        for (a_flag, ch1) in a_flags.iter().zip(a.iter()) {
            if *a_flag {
                for (b_flag, ch2) in b_iter.by_ref() {
                    if !*b_flag {
                        continue;
                    }
                    if ch1 != ch2 {
                        transpositions += 1;
                    }
                    break;
                }
            }
        }
    }
    transpositions /= 2;

    if matches == 0 {
        0.0
    } else {
        ((matches as f64 / a_len as f64)
            + (matches as f64 / b_len as f64)
            + ((matches - transpositions) as f64 / matches as f64))
            / 3.0
    }
}

/// Jaro-Winkler similarity, allocation-free. Port of `strsim::generic_jaro_winkler`.
#[must_use]
pub(crate) fn jaro_winkler_chars(a: &[char], b: &[char], flags: &mut Vec<bool>) -> f64 {
    let sim = jaro_chars(a, b, flags);
    if sim > 0.7 {
        let prefix_length = a
            .iter()
            .take(4)
            .zip(b.iter())
            .take_while(|(x, y)| x == y)
            .count();
        sim + 0.1 * prefix_length as f64 * (1.0 - sim)
    } else {
        sim
    }
}

/// Upper bound on `jaro_winkler_chars` given only lengths and the match ceiling.
#[inline]
#[must_use]
fn jw_upper_bound(m_ub: usize, q_len: usize, e_len: usize) -> f64 {
    if m_ub == 0 || q_len == 0 || e_len == 0 {
        return 0.0;
    }
    let j_ub = (m_ub as f64 / q_len as f64 + m_ub as f64 / e_len as f64 + 1.0) / 3.0;
    0.4 + 0.6 * j_ub
}

/// Entries sorted by character length, with a character-set mask each.
#[derive(Debug, Default, Clone)]
pub(crate) struct FuzzyIndex {
    /// Entry ids, ascending by `char_len`.
    ids: Vec<u32>,
    char_lens: Vec<u16>,
    charsets: Vec<u64>,
    /// `len_start[l]` is the first slot whose `char_len >= l`.
    len_start: Vec<u32>,
}

impl FuzzyIndex {
    /// Rebuild from `(entry id, char length, charset mask)` triples.
    pub(crate) fn build(mut items: Vec<(u32, u16, u64)>) -> Self {
        items.sort_unstable_by_key(|&(_, len, _)| len);

        let max_len = items.last().map_or(0, |&(_, len, _)| len) as usize;
        let mut len_start = vec![items.len() as u32; max_len + 2];
        for (slot, &(_, len, _)) in items.iter().enumerate().rev() {
            len_start[len as usize] = slot as u32;
        }
        // Make it monotone: len_start[l] must be the first slot with char_len >= l.
        for l in (0..len_start.len().saturating_sub(1)).rev() {
            len_start[l] = len_start[l].min(len_start[l + 1]);
        }

        Self {
            ids: items.iter().map(|&(id, _, _)| id).collect(),
            char_lens: items.iter().map(|&(_, len, _)| len).collect(),
            charsets: items.iter().map(|&(_, _, cs)| cs).collect(),
            len_start,
        }
    }

    fn slot_range(&self, q_len: usize, threshold: f64) -> (usize, usize) {
        // With m <= min(|q|,|e|), JW_ub collapses to 0.8 + 0.2 * (min/max), so a
        // candidate can only pass when min/max > 5t - 4.
        let r_min = 5.0 * threshold - 4.0;
        if r_min <= 0.0 || q_len == 0 {
            return (0, self.ids.len());
        }
        let lo = (q_len as f64 * r_min).ceil().max(1.0) as usize;
        let hi = (q_len as f64 / r_min).floor() as usize;

        let start = *self.len_start.get(lo).unwrap_or(&(self.ids.len() as u32)) as usize;
        let end = *self
            .len_start
            .get(hi.saturating_add(1))
            .unwrap_or(&(self.ids.len() as u32)) as usize;
        (start.min(self.ids.len()), end.min(self.ids.len()))
    }

    /// Best entry above `threshold` that `accept` allows, or `None`.
    ///
    /// `entry_chars` yields the characters of an entry by id; ties break on
    /// shorter entry first, then on id, so results are deterministic.
    pub(crate) fn best<F, A>(
        &self,
        query: &[char],
        threshold: f64,
        mut entry_chars: F,
        mut accept: A,
        e_buf: &mut Vec<char>,
        flags: &mut Vec<bool>,
    ) -> Option<(u32, f64)>
    where
        F: FnMut(u32, &mut Vec<char>),
        A: FnMut(u32) -> bool,
    {
        if query.is_empty() || !(0.0..1.0).contains(&threshold) {
            return None;
        }

        let q_len = query.len();
        let q_mask = charset_mask_chars(query);
        let (start, end) = self.slot_range(q_len, threshold);

        let mut best: Option<(u32, f64, u16)> = None;
        for slot in start..end {
            let e_len = self.char_lens[slot] as usize;
            if e_len == 0 {
                continue;
            }
            let e_mask = self.charsets[slot];

            // Cheap side first: how many of the query's characters can match at all.
            let d_qe = query.iter().filter(|&&c| e_mask & char_bit(c) != 0).count();
            if jw_upper_bound(d_qe.min(q_len).min(e_len), q_len, e_len) <= threshold {
                continue;
            }

            let id = self.ids[slot];
            if !accept(id) {
                continue;
            }
            entry_chars(id, e_buf);

            let d_eq = e_buf.iter().filter(|&&c| q_mask & char_bit(c) != 0).count();
            let m_ub = q_len.min(e_len).min(d_qe).min(d_eq);
            if jw_upper_bound(m_ub, q_len, e_len) <= threshold {
                continue;
            }

            let score = jaro_winkler_chars(query, e_buf, flags);
            if score <= threshold {
                continue;
            }
            let better = match best {
                None => true,
                Some((best_id, best_score, best_len)) => {
                    (
                        score,
                        std::cmp::Reverse(self.char_lens[slot]),
                        std::cmp::Reverse(id),
                    ) > (
                        best_score,
                        std::cmp::Reverse(best_len),
                        std::cmp::Reverse(best_id),
                    )
                }
            };
            if better {
                best = Some((id, score, self.char_lens[slot]));
            }
        }

        best.map(|(id, score, _)| (id, score))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}
