//! "Did you mean": a query-time spelling correction over the words that name documents.
//!
//! Nothing here is stored. The name-term table is derived from the document metadata (titles
//! and aliases) the first time a correction is asked for, so the serialized index and
//! `INDEX_FORMAT_VERSION` do not change. Every ordering is total and breaks ties by term text,
//! so the same index and query always give the same answer.

use crate::text::scan;
use crate::{DocView, Filters, SearchIndex};
use std::sync::OnceLock;

/// A query with at most this many results (entries and book sections together) is weak enough to
/// correct, provided none of them names every word of the query. Callers that already ran the
/// search compare their own total with this before asking, so a broad query costs nothing extra.
pub const CORRECTION_MAX_HITS: usize = 3;

/// Words shorter than this are never changed: one edit rewrites a short word into another one.
const MIN_WORD_LEN: usize = 4;
/// Longest query that is corrected, in words.
const MAX_WORDS: usize = 6;
/// Words that may be replaced in one query. More misspellings than this are not a typo.
const MAX_CHANGED_WORDS: usize = 3;
/// Replacement words tried per misspelled word.
const CANDIDATES_PER_WORD: usize = 3;
/// Corrected queries that are searched, in order of total edit distance.
const MAX_COMBINATIONS: usize = 6;
/// Name terms checked for the active books when deciding whether a prefix is already a word.
const PREFIX_PROBE: usize = 64;

/// A suggested replacement for a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// The corrected query: normalized words, with every unchanged word as the reader typed it.
    pub text: String,
    /// Total edit distance from the typed words, at least 1.
    pub distance: usize,
}

/// Every word of a title or alias, sorted, with the documents that carry it.
#[derive(Debug, Clone, Default)]
pub(crate) struct NameTerms {
    terms: Vec<String>,
    docs: Vec<Vec<u32>>,
}

impl NameTerms {
    fn build(docs: &[crate::store::DocMeta]) -> Self {
        // Collect (term, document) pairs and sort once: far fewer allocations than a map of
        // lists, which matters because the first correction builds this on the main thread.
        let mut pairs: Vec<(String, u32)> = Vec::new();
        for (ord, doc) in docs.iter().enumerate() {
            let ord = ord as u32;
            let names =
                std::iter::once(doc.title.as_str()).chain(doc.aliases.iter().map(String::as_str));
            for name in names {
                for token in scan(name) {
                    pairs.push((token.norm, ord));
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        let mut terms: Vec<String> = Vec::new();
        let mut lists: Vec<Vec<u32>> = Vec::new();
        for (term, ord) in pairs {
            if terms.last() == Some(&term) {
                lists.last_mut().expect("a list per term").push(ord);
            } else {
                terms.push(term);
                lists.push(vec![ord]);
            }
        }
        Self { terms, docs: lists }
    }

    /// Number of distinct name terms.
    pub(crate) fn len(&self) -> usize {
        self.terms.len()
    }
}

/// Lazily built [`NameTerms`]. Equality ignores it: it is a cache of the metadata, not state.
#[derive(Debug, Clone, Default)]
pub(crate) struct LazyNames(OnceLock<NameTerms>);

impl PartialEq for LazyNames {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// One replacement for a misspelled word.
#[derive(Debug, Clone)]
struct Candidate {
    term: usize,
    cost: usize,
    /// The term matched only through its leading letters, not as a whole word.
    prefix_only: bool,
}

impl SearchIndex {
    fn name_terms(&self) -> &NameTerms {
        self.names
            .0
            .get_or_init(|| NameTerms::build(&self.meta.docs))
    }

    /// Builds the table of name terms now instead of at the first correction, and says how many
    /// distinct terms it holds. For benchmarks and for warming before the first keystroke.
    pub fn warm_corrections(&self) -> usize {
        self.name_terms().len()
    }

    /// Offers a corrected spelling for a query that finds little.
    ///
    /// A correction is offered only when all of these hold:
    /// - the query has at most [`CORRECTION_MAX_HITS`] results among the documents that `accept`
    ///   allows and `filters` keep, and none of them names every query word (a *strong* result is
    ///   one that does, with the last word allowed to be a prefix);
    /// - at least one word of four or more letters is not the start of any name that the same
    ///   documents carry, so a word that is already right is never changed;
    /// - replacing those words with names that are within one edit (two for words of eight
    ///   letters or more; a swap of neighbours counts as one) finds a strong result.
    ///
    /// Numbers and words under four letters stay as typed. `accept` is where the caller applies
    /// its active books: a name that only turned-off books carry is never suggested. The
    /// corrected query keeps the caller's filters, so the caller runs it with its current scope.
    ///
    /// The cost is one search of the typed query plus at most a few of corrected ones. It decodes
    /// the postings of the query's words, so a caller that has the result count already should
    /// skip the call when that count exceeds [`CORRECTION_MAX_HITS`].
    pub fn suggest_correction(
        &self,
        text: &str,
        filters: &Filters,
        accept: impl Fn(DocView<'_>) -> bool,
    ) -> Option<Correction> {
        let words: Vec<String> = scan(text).into_iter().map(|t| t.norm).collect();
        if words.is_empty() || words.len() > MAX_WORDS {
            return None;
        }
        let allowed = |ord: u32| {
            let ord = ord as usize;
            ord < self.meta.docs.len() && accept(self.view(ord)) && self.passes(ord, filters)
        };

        // A query that already has a strong result needs nothing.
        if let Some(scored) = self.matching_docs(&words, |doc| allowed(doc as u32))
            && (scored.len() > CORRECTION_MAX_HITS
                || scored
                    .iter()
                    .any(|&(doc, _)| self.names_every_word(doc, &words)))
        {
            return None;
        }

        let names = self.name_terms();
        let last = words.len() - 1;
        // For each word: the replacements to try, or `None` when it stays as typed.
        let mut options: Vec<Option<Vec<Candidate>>> = Vec::with_capacity(words.len());
        let mut changed = 0;
        for (i, word) in words.iter().enumerate() {
            if !is_correctable(word) || self.is_known(names, word, i == last, &allowed) {
                options.push(None);
                continue;
            }
            changed += 1;
            if changed > MAX_CHANGED_WORDS {
                return None;
            }
            let found = self.candidates(names, word, i == last, &allowed);
            if found.is_empty() {
                return None;
            }
            options.push(Some(found));
        }
        if changed == 0 {
            return None;
        }

        // Try the cheapest combinations first. Equal totals fall back to each word's rank.
        let mut picks: Vec<(usize, Vec<usize>)> = vec![(0, Vec::new())];
        for option in &options {
            let mut next = Vec::new();
            for (cost, chosen) in &picks {
                match option {
                    None => {
                        let mut chosen = chosen.clone();
                        chosen.push(0);
                        next.push((*cost, chosen));
                    }
                    Some(candidates) => {
                        for (rank, candidate) in candidates.iter().enumerate() {
                            let mut chosen = chosen.clone();
                            chosen.push(rank);
                            next.push((cost + candidate.cost, chosen));
                        }
                    }
                }
            }
            picks = next;
        }
        picks.sort();
        for (distance, chosen) in picks.into_iter().take(MAX_COMBINATIONS) {
            let corrected: Vec<String> = words
                .iter()
                .zip(&options)
                .zip(&chosen)
                .map(|((word, option), &rank)| match option {
                    None => word.clone(),
                    Some(candidates) => names.terms[candidates[rank].term].clone(),
                })
                .collect();
            let Some(scored) = self.matching_docs(&corrected, |doc| allowed(doc as u32)) else {
                continue;
            };
            if scored
                .iter()
                .any(|&(doc, _)| self.names_every_word(doc, &corrected))
            {
                return Some(Correction {
                    text: corrected.join(" "),
                    distance,
                });
            }
        }
        None
    }

    /// Whether the title or an alias of `doc` holds every word, the last as a prefix.
    fn names_every_word(&self, doc: u32, words: &[String]) -> bool {
        let Some(meta) = self.meta.docs.get(doc as usize) else {
            return false;
        };
        let mut name_words: Vec<String> = scan(&meta.title).into_iter().map(|t| t.norm).collect();
        for alias in &meta.aliases {
            name_words.extend(scan(alias).into_iter().map(|t| t.norm));
        }
        let last = words.len() - 1;
        words.iter().enumerate().all(|(i, word)| {
            name_words
                .iter()
                .any(|name| name == word || (i == last && name.starts_with(word.as_str())))
        })
    }

    /// Whether `word` is already a name that the allowed documents carry. The last word counts
    /// when it starts one, because the reader may still be typing it.
    fn is_known(
        &self,
        names: &NameTerms,
        word: &str,
        is_last: bool,
        allowed: &impl Fn(u32) -> bool,
    ) -> bool {
        let viable = |term: usize| names.docs[term].iter().any(|&ord| allowed(ord));
        if let Ok(term) = names.terms.binary_search_by(|t| t.as_str().cmp(word))
            && viable(term)
        {
            return true;
        }
        if !is_last {
            return false;
        }
        let start = names.terms.partition_point(|t| t.as_str() < word);
        names.terms[start..]
            .iter()
            .take_while(|t| t.starts_with(word))
            .take(PREFIX_PROBE)
            .enumerate()
            .any(|(k, _)| viable(start + k))
    }

    /// Replacements for a misspelled word, best first, each carried by an allowed document.
    fn candidates(
        &self,
        names: &NameTerms,
        word: &str,
        is_last: bool,
        allowed: &impl Fn(u32) -> bool,
    ) -> Vec<Candidate> {
        let typed: Vec<char> = word.chars().collect();
        let budget = edit_budget(typed.len());
        let mut buffer: Vec<char> = Vec::new();
        let mut found: Vec<Candidate> = Vec::new();
        for (term, name) in names.terms.iter().enumerate() {
            buffer.clear();
            buffer.extend(name.chars());
            let whole = if buffer.len().abs_diff(typed.len()) <= budget {
                edit_distance(&typed, &buffer, budget)
            } else {
                None
            };
            // The last word may be unfinished: compare it with the start of the name too, but
            // only one edit away, since a prefix is a looser match than a whole word.
            let prefix = if is_last && whole.is_none() {
                (typed.len().saturating_sub(1)..=typed.len() + 1)
                    .filter(|&k| k <= buffer.len())
                    .filter_map(|k| edit_distance(&typed, &buffer[..k], 1))
                    .min()
            } else {
                None
            };
            let (cost, prefix_only) = match (whole, prefix) {
                (Some(cost), _) => (cost, false),
                (None, Some(cost)) => (cost, true),
                (None, None) => continue,
            };
            // A word cannot be corrected into itself.
            if cost == 0 && !prefix_only {
                continue;
            }
            found.push(Candidate {
                term,
                cost,
                prefix_only,
            });
        }
        // Fewest edits, then whole words before prefixes, then the name that more documents
        // carry, then the term text: a total order, so ties never depend on how it was built.
        found.sort_by(|a, b| {
            a.cost
                .cmp(&b.cost)
                .then(a.prefix_only.cmp(&b.prefix_only))
                .then(names.docs[b.term].len().cmp(&names.docs[a.term].len()))
                .then(names.terms[a.term].cmp(&names.terms[b.term]))
        });
        found
            .into_iter()
            .filter(|candidate| names.docs[candidate.term].iter().any(|&ord| allowed(ord)))
            .take(CANDIDATES_PER_WORD)
            .collect()
    }
}

/// A word is corrected only if it is long enough and holds a letter.
fn is_correctable(word: &str) -> bool {
    word.chars().count() >= MIN_WORD_LEN && word.chars().any(|c| !c.is_ascii_digit())
}

/// Edits allowed for a word of `len` letters.
fn edit_budget(len: usize) -> usize {
    if len >= 8 { 2 } else { 1 }
}

/// Optimal string alignment distance (insert, delete, substitute, swap neighbours), or `None`
/// when it exceeds `max`.
fn edit_distance(a: &[char], b: &[char], max: usize) -> Option<usize> {
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let width = b.len() + 1;
    let mut rows = [
        vec![0usize; width],
        vec![0usize; width],
        vec![0usize; width],
    ];
    for (j, cell) in rows[1].iter_mut().enumerate() {
        *cell = j;
    }
    // rows[0] holds i-2, rows[1] holds i-1, rows[2] is being filled.
    for i in 1..=a.len() {
        rows[2][0] = i;
        let mut row_min = rows[2][0];
        for j in 1..=b.len() {
            let same = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[1][j] + 1)
                .min(rows[2][j - 1] + 1)
                .min(rows[1][j - 1] + same);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[0][j - 2] + 1);
            }
            rows[2][j] = best;
            row_min = row_min.min(best);
        }
        if row_min > max {
            return None;
        }
        rows.rotate_left(1);
        // After the rotation rows[1] is the finished row and rows[2] is scratch.
    }
    let distance = rows[1][b.len()];
    (distance <= max).then_some(distance)
}

#[cfg(test)]
pub(crate) fn edit_distance_for_tests(a: &str, b: &str, max: usize) -> Option<usize> {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    edit_distance(&a, &b, max)
}
