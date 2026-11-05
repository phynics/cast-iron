use crate::build::cap_text;
use crate::text::{Tok, scan};
use crate::{FilterCounts, Filters, Hit, Query, SearchIndex, Snippet, normalize};

const DEFAULT_LIMIT: usize = 50;
const MAX_PREFIX_TERMS: usize = 512;
const PREFIX_WEIGHT: f32 = 0.5;
const W_TITLE: f32 = 5.0;
const W_ALIAS: f32 = 3.0;
const W_TEXT: f32 = 1.0;
const K1: f32 = 1.2;
const B: f32 = 0.75;

const BOOST_EXACT_TITLE: f32 = 1000.0;
const BOOST_TITLE_PREFIX: f32 = 500.0;
const BOOST_EXACT_ALIAS: f32 = 450.0;
const BOOST_ALL_IN_TITLE: f32 = 200.0;

const SNIP_BEFORE: usize = 50;
const SNIP_LEN: usize = 200;
const SNIP_SNAP: usize = 20;

/// Query phases that the native benchmark can time separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Tokenize,
    Lookup,
    Scoring,
    Sort,
    Materialize,
    Snippets,
}

/// Phase recorder. Release builds use [`NoTimer`], whose methods inline away.
trait PhaseTimer {
    #[inline]
    fn enter(&mut self, _phase: Phase) {}

    #[inline]
    fn exit(&mut self, _phase: Phase) {}
}

struct NoTimer;

impl PhaseTimer for NoTimer {}

/// Per-query timings from [`SearchIndex::search_profiled`]. Native only.
#[cfg(feature = "profile")]
#[derive(Clone, Copy, Debug, Default)]
pub struct SearchTiming {
    pub tokenize: std::time::Duration,
    pub lookup: std::time::Duration,
    pub scoring: std::time::Duration,
    pub sort: std::time::Duration,
    pub materialize: std::time::Duration,
    pub snippets: std::time::Duration,
    pub total: std::time::Duration,
    pub hits: usize,
}

#[cfg(feature = "profile")]
struct ProfilingTimer {
    at: std::time::Instant,
    timing: SearchTiming,
}

#[cfg(feature = "profile")]
impl PhaseTimer for ProfilingTimer {
    #[inline]
    fn enter(&mut self, _phase: Phase) {
        self.at = std::time::Instant::now();
    }

    #[inline]
    fn exit(&mut self, phase: Phase) {
        let elapsed = self.at.elapsed();
        let field = match phase {
            Phase::Tokenize => &mut self.timing.tokenize,
            Phase::Lookup => &mut self.timing.lookup,
            Phase::Scoring => &mut self.timing.scoring,
            Phase::Sort => &mut self.timing.sort,
            Phase::Materialize => &mut self.timing.materialize,
            Phase::Snippets => &mut self.timing.snippets,
        };
        *field += elapsed;
    }
}

#[derive(Clone, Copy, Default)]
struct TokHit {
    score: f32,
    in_title: bool,
}

/// Per-token matches indexed by document ordinal. A dense table beats a tree
/// for common terms such as "of", whose postings cover most of the corpus.
#[derive(Default)]
struct TokenHits {
    by_doc: Vec<Option<TokHit>>,
    docs: Vec<u32>,
}

impl TokenHits {
    fn new(docs: usize) -> Self {
        Self {
            by_doc: vec![None; docs],
            docs: Vec::new(),
        }
    }

    #[inline]
    fn entry(&mut self, doc: u32) -> &mut TokHit {
        let slot = &mut self.by_doc[doc as usize];
        if slot.is_none() {
            *slot = Some(TokHit::default());
            self.docs.push(doc);
        }
        slot.as_mut().expect("just inserted")
    }

    #[inline]
    fn get(&self, doc: u32) -> Option<TokHit> {
        self.by_doc[doc as usize]
    }
}

/// Decides which text tokens count as query matches for highlighting.
#[derive(Clone, Debug)]
struct Matcher {
    exact: Vec<String>,
    prefix: Vec<String>,
}

impl Matcher {
    fn is_match(&self, t: &str) -> bool {
        self.exact.iter().any(|q| q == t) || self.prefix.iter().any(|p| t.starts_with(p.as_str()))
    }

    fn ranges(&self, toks: &[Tok]) -> Vec<(usize, usize)> {
        toks.iter()
            .filter(|t| self.is_match(&t.norm))
            .map(|t| (t.start, t.end))
            .collect()
    }
}

/// Ranking-only result: enough to group, count, and order results without
/// scanning document text for a snippet. `SearchIndex::materialize_hit` fills
/// the highlighted title and snippet for the rows that the page displays.
#[derive(Clone, Debug, PartialEq)]
pub struct HitSummary {
    pub ord: u32,
    pub id: String,
    pub title: String,
    pub kind: String,
    pub source: String,
    pub score: f32,
}

impl HitSummary {
    /// Converts to a display hit with an empty snippet. Fill it with
    /// [`SearchIndex::materialize_hit`] before rendering.
    pub fn into_hit(self) -> Hit {
        Hit {
            ord: self.ord,
            id: self.id,
            title: self.title,
            kind: self.kind,
            source: self.source,
            score: self.score,
            title_matches: Vec::new(),
            snippet: Snippet::default(),
        }
    }
}

/// Reusable query pattern for [`SearchIndex::materialize_hit`].
#[derive(Clone, Debug)]
pub struct Highlighter {
    matcher: Matcher,
}

impl SearchIndex {
    fn passes(&self, ord: usize, f: &Filters) -> bool {
        let d = &self.meta.docs[ord];
        if !f.kinds.is_empty() && !f.kinds.iter().any(|k| k.eq_ignore_ascii_case(&d.kind)) {
            return false;
        }
        if !f.sources.is_empty() && !f.sources.iter().any(|s| s.eq_ignore_ascii_case(&d.source)) {
            return false;
        }
        // Facets are OR within one field and AND across fields. Compare in place: the old
        // implementation lowercased every field and value for every document, which allocated
        // millions of strings over one full-library count.
        for (i, (field, _)) in f.facets.iter().enumerate() {
            if f.facets[..i]
                .iter()
                .any(|(earlier, _)| earlier.eq_ignore_ascii_case(field))
            {
                continue;
            }
            let found = f.facets.iter().any(|(filter_field, filter_value)| {
                filter_field.eq_ignore_ascii_case(field)
                    && d.facets.iter().any(|(doc_field, doc_value)| {
                        doc_field.eq_ignore_ascii_case(field)
                            && doc_value.eq_ignore_ascii_case(filter_value)
                    })
            });
            if !found {
                return false;
            }
        }
        true
    }

    /// (term index, weight) pairs for one query token.
    fn expand(&self, q: &str, allow_prefix_always: bool) -> (Vec<(usize, f32)>, bool) {
        let terms = &self.postings.terms;
        let mut out = Vec::new();
        let exact = terms.binary_search_by(|t| t.as_str().cmp(q)).ok();
        if let Some(i) = exact {
            out.push((i, 1.0));
        }
        let use_prefix = allow_prefix_always || exact.is_none();
        if use_prefix {
            let start = terms.partition_point(|t| t.as_str() < q);
            out.extend(
                terms[start..]
                    .iter()
                    .take_while(|t| t.starts_with(q))
                    .enumerate()
                    .map(|(k, _)| (start + k, PREFIX_WEIGHT))
                    .filter(|(i, _)| Some(*i) != exact)
                    .take(MAX_PREFIX_TERMS),
            );
        }
        (out, use_prefix)
    }

    fn token_hits<T: PhaseTimer>(&self, expansion: &[(usize, f32)], timer: &mut T) -> TokenHits {
        let n = self.meta.docs.len() as f32;
        let m = &self.meta;
        let avg = |a: f32| a.max(1.0);
        let idf_of = |df: f32| (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
        // A prefix such as "fire" expands to completions such as "firewood". Scoring each
        // completion with its own IDF lets the rarest completion win, which is noise. All
        // completions share the IDF of the set of documents that the prefix matches.
        let prefix_idf = {
            timer.enter(Phase::Lookup);
            let mut seen = vec![false; m.docs.len()];
            let mut count = 0u32;
            for &(ti, weight) in expansion {
                if weight < 1.0 {
                    for p in self.postings.decode(ti) {
                        if let Some(slot) = seen.get_mut(p.doc as usize)
                            && !*slot
                        {
                            *slot = true;
                            count += 1;
                        }
                    }
                }
            }
            timer.exit(Phase::Lookup);
            idf_of(count as f32)
        };
        let mut map = TokenHits::new(m.docs.len());
        for &(ti, weight) in expansion {
            let idf = if weight < 1.0 {
                prefix_idf
            } else {
                idf_of(self.postings.dfs[ti] as f32)
            };
            timer.enter(Phase::Lookup);
            let postings = self.postings.decode(ti);
            timer.exit(Phase::Lookup);
            timer.enter(Phase::Scoring);
            for p in postings {
                let Some(d) = m.docs.get(p.doc as usize) else {
                    continue;
                };
                let part = |tf: u32, len: u32, a: f32, w: f32| {
                    if tf == 0 {
                        return 0.0;
                    }
                    let tf = tf as f32;
                    w * tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * len as f32 / avg(a)))
                };
                let s = part(p.tf_title, d.len_title, m.avg_title, W_TITLE)
                    + part(p.tf_alias, d.len_alias, m.avg_alias, W_ALIAS)
                    + part(p.tf_text, d.len_text, m.avg_text, W_TEXT);
                let e = map.entry(p.doc);
                e.score += s * idf * weight;
                e.in_title |= p.tf_title > 0;
            }
            timer.exit(Phase::Scoring);
        }
        map
    }

    /// Counts for the filter chips of a search. The base set is every eligible document that
    /// matches the query text (all eligible documents for empty text). A kind, source or facet
    /// value counts the base documents that carry it and pass every *other* active filter, so a
    /// chip shows what choosing it would add or swap. With no text and no filters this is the
    /// library total of the eligible documents.
    pub fn filter_counts_eligible(
        &self,
        text: &str,
        filters: &Filters,
        eligible: impl Fn(&str) -> bool,
    ) -> FilterCounts {
        let n = self.meta.docs.len();
        let mut eligible_mask = vec![false; n];
        let mut all_eligible = true;
        for (i, doc) in self.meta.docs.iter().enumerate() {
            let ok = eligible(&doc.id);
            eligible_mask[i] = ok;
            all_eligible &= ok;
        }

        // Nothing is filtered: the precomputed library totals are the answer.
        if filters.is_empty() && text.trim().is_empty() && all_eligible {
            return self.doc_lists.totals.clone();
        }

        let qtokens: Vec<String> = scan(text).into_iter().map(|t| t.norm).collect();
        let base: Vec<u32> = if qtokens.is_empty() {
            (0..n)
                .filter(|&i| eligible_mask[i])
                .map(|i| i as u32)
                .collect()
        } else {
            match self.score_docs(&qtokens, |doc| eligible_mask[doc], &mut NoTimer) {
                Some((scored, _)) => scored.into_iter().map(|(doc, _)| doc).collect(),
                None => return FilterCounts::default(),
            }
        };

        // Each active filter group gets one bit. A document's failure mask then answers every
        // chip at once: the full result passes no bit, and a chip ignores its own group, so a
        // document counts for it when it fails no group or only that one. This replaces one
        // `passes` call per document per group with one evaluation per document.
        let mut next_bit = 0u32;
        let mut take_bit = || {
            let bit = 1u64 << next_bit;
            next_bit += 1;
            bit
        };
        let kind_bit = (!filters.kinds.is_empty()).then(&mut take_bit);
        let source_bit = (!filters.sources.is_empty()).then(&mut take_bit);
        let mut facet_bits: std::collections::BTreeMap<String, u64> = Default::default();
        for (field, _) in &filters.facets {
            facet_bits
                .entry(field.to_lowercase())
                .or_insert_with(&mut take_bit);
        }
        debug_assert!(next_bit <= 64, "too many active filter groups");

        let fail_mask = |doc: &crate::store::DocMeta| {
            let mut fail = 0u64;
            if let Some(bit) = kind_bit
                && !filters
                    .kinds
                    .iter()
                    .any(|kind| kind.eq_ignore_ascii_case(&doc.kind))
            {
                fail |= bit;
            }
            if let Some(bit) = source_bit
                && !filters
                    .sources
                    .iter()
                    .any(|source| source.eq_ignore_ascii_case(&doc.source))
            {
                fail |= bit;
            }
            for (field, bit) in &facet_bits {
                let found = filters.facets.iter().any(|(filter_field, filter_value)| {
                    filter_field.eq_ignore_ascii_case(field)
                        && doc.facets.iter().any(|(doc_field, doc_value)| {
                            doc_field.eq_ignore_ascii_case(field)
                                && doc_value.eq_ignore_ascii_case(filter_value)
                        })
                });
                if !found {
                    fail |= bit;
                }
            }
            fail
        };

        let mut counts = FilterCounts::default();
        // Reused per document so duplicate `(field, value)` pairs count once without allocating
        // a new set for every document.
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for ord in base {
            let doc = &self.meta.docs[ord as usize];
            let fail = fail_mask(doc);
            if fail == 0 {
                counts.total += 1;
            }
            if fail == 0 || Some(fail) == kind_bit {
                *counts.kinds.entry(doc.kind.clone()).or_default() += 1;
            }
            if fail == 0 || Some(fail) == source_bit {
                *counts.sources.entry(doc.source.clone()).or_default() += 1;
            }
            seen.clear();
            for (field, value) in &doc.facets {
                if seen.contains(&(field.as_str(), value.as_str())) {
                    continue;
                }
                seen.push((field, value));
                let bit = facet_bits.get(field.as_str()).copied().or_else(|| {
                    facet_bits
                        .iter()
                        .find(|(name, _)| name.eq_ignore_ascii_case(field))
                        .map(|(_, bit)| *bit)
                });
                let counts_here = match bit {
                    Some(bit) => fail == 0 || fail == bit,
                    None => fail == 0,
                };
                if counts_here {
                    let key = (field.to_lowercase(), value.clone());
                    *counts.facets.entry(key).or_default() += 1;
                }
            }
        }
        counts
    }

    pub fn search(&self, query: &Query) -> Vec<Hit> {
        self.search_eligible(query, |_| true)
    }

    /// Search only eligible document IDs, applying eligibility before the result limit.
    pub fn search_eligible(&self, query: &Query, eligible: impl Fn(&str) -> bool) -> Vec<Hit> {
        self.search_eligible_timed(query, eligible, &mut NoTimer)
    }

    /// Ranked results without snippets, for callers that only show a page.
    /// [`SearchIndex::materialize_hit`] fills the rows that are rendered.
    pub fn search_summaries(&self, query: &Query) -> Vec<HitSummary> {
        self.search_summaries_eligible(query, |_| true)
    }

    /// Summary-only search over eligible IDs, applying eligibility before the limit.
    pub fn search_summaries_eligible(
        &self,
        query: &Query,
        eligible: impl Fn(&str) -> bool,
    ) -> Vec<HitSummary> {
        let limit = result_limit(query);
        let Some((ranked, _)) = self.ranked(query, eligible, &mut NoTimer, limit) else {
            return Vec::new();
        };
        ranked
            .into_iter()
            .map(|(ord, score)| self.summary(ord, score))
            .collect()
    }

    /// Builds the reusable highlighter for materializing summary hits.
    pub fn highlighter(&self, query: &Query) -> Highlighter {
        let qtokens: Vec<String> = scan(&query.text).into_iter().map(|t| t.norm).collect();
        let matcher = self.matcher(&qtokens).unwrap_or(Matcher {
            exact: Vec::new(),
            prefix: Vec::new(),
        });
        Highlighter { matcher }
    }

    /// Fills a hit's highlighted title and snippet. The hit fields, score, and
    /// order must come from the same query and index.
    pub fn materialize_hit(&self, highlighter: &Highlighter, hit: &mut Hit) {
        self.fill_highlight(hit, &highlighter.matcher, &mut NoTimer);
    }

    /// Runs a search and reports the time spent in each phase. Native measurements only.
    #[cfg(feature = "profile")]
    pub fn search_profiled(
        &self,
        query: &Query,
        eligible: impl Fn(&str) -> bool,
    ) -> (Vec<Hit>, SearchTiming) {
        let started = std::time::Instant::now();
        let mut timer = ProfilingTimer {
            at: started,
            timing: SearchTiming::default(),
        };
        let hits = self.search_eligible_timed(query, eligible, &mut timer);
        let mut timing = timer.timing;
        timing.total = started.elapsed();
        timing.hits = hits.len();
        (hits, timing)
    }

    /// Summary-only search with per-phase timings. Native measurements only.
    #[cfg(feature = "profile")]
    pub fn search_summaries_profiled(
        &self,
        query: &Query,
        eligible: impl Fn(&str) -> bool,
    ) -> (Vec<HitSummary>, SearchTiming) {
        let started = std::time::Instant::now();
        let mut timer = ProfilingTimer {
            at: started,
            timing: SearchTiming::default(),
        };
        let limit = result_limit(query);
        let summaries = match self.ranked(query, eligible, &mut timer, limit) {
            Some((ranked, _)) => ranked
                .into_iter()
                .map(|(ord, score)| self.summary(ord, score))
                .collect(),
            None => Vec::new(),
        };
        let mut timing = timer.timing;
        timing.total = started.elapsed();
        timing.hits = summaries.len();
        (summaries, timing)
    }

    fn search_eligible_timed<T: PhaseTimer>(
        &self,
        query: &Query,
        eligible: impl Fn(&str) -> bool,
        timer: &mut T,
    ) -> Vec<Hit> {
        let limit = result_limit(query);
        let Some((ranked, matcher)) = self.ranked(query, eligible, timer, limit) else {
            return Vec::new();
        };
        ranked
            .into_iter()
            .map(|(ord, score)| self.make_hit(ord, score, &matcher, timer))
            .collect()
    }

    /// Ranks matching documents and keeps the best `limit`, in final display
    /// order. `None` when a query token matches no term at all.
    fn ranked<T: PhaseTimer>(
        &self,
        query: &Query,
        eligible: impl Fn(&str) -> bool,
        timer: &mut T,
        limit: usize,
    ) -> Option<(Vec<(u32, f32)>, Matcher)> {
        timer.enter(Phase::Tokenize);
        let qtokens: Vec<String> = scan(&query.text).into_iter().map(|t| t.norm).collect();
        timer.exit(Phase::Tokenize);
        if qtokens.is_empty() {
            if query.filters.is_empty() {
                return Some((Vec::new(), empty_matcher()));
            }
            timer.enter(Phase::Scoring);
            let mut ords: Vec<(String, &str, u32)> = (0..self.meta.docs.len())
                .filter(|&i| eligible(&self.meta.docs[i].id) && self.passes(i, &query.filters))
                .map(|i| {
                    (
                        normalize(&self.meta.docs[i].title),
                        self.meta.docs[i].id.as_str(),
                        i as u32,
                    )
                })
                .collect();
            timer.exit(Phase::Scoring);
            timer.enter(Phase::Sort);
            ords.sort();
            timer.exit(Phase::Sort);
            let ranked = ords
                .into_iter()
                .take(limit)
                .map(|(_, _, ord)| (ord, 0.0))
                .collect();
            return Some((ranked, empty_matcher()));
        }

        let (mut scored, matcher) = self.score_docs(
            &qtokens,
            |doc| eligible(&self.meta.docs[doc].id) && self.passes(doc, &query.filters),
            timer,
        )?;
        timer.enter(Phase::Sort);
        sort_scored(&mut scored, &self.meta.docs, &self.doc_ranks, limit);
        timer.exit(Phase::Sort);
        Some((scored, matcher))
    }

    fn summary(&self, ord: u32, score: f32) -> HitSummary {
        let d = &self.meta.docs[ord as usize];
        HitSummary {
            ord,
            id: d.id.clone(),
            title: d.title.clone(),
            kind: d.kind.clone(),
            source: d.source.clone(),
            score,
        }
    }

    fn matcher(&self, qtokens: &[String]) -> Option<Matcher> {
        let last = qtokens.len().checked_sub(1)?;
        let mut matcher = empty_matcher();
        for (i, q) in qtokens.iter().enumerate() {
            let (exp, prefixed) = self.expand(q, i == last);
            if exp.is_empty() {
                return None;
            }
            matcher.exact.push(q.clone());
            if prefixed {
                matcher.prefix.push(q.clone());
            }
        }
        Some(matcher)
    }

    /// Scores every document that holds all query tokens and that `accept` allows. `None` when a
    /// token matches no term at all.
    fn score_docs<T: PhaseTimer>(
        &self,
        qtokens: &[String],
        accept: impl Fn(usize) -> bool,
        timer: &mut T,
    ) -> Option<(Vec<(u32, f32)>, Matcher)> {
        let last = qtokens.len() - 1;
        let mut per_token = Vec::with_capacity(qtokens.len());
        let mut matcher = Matcher {
            exact: vec![],
            prefix: vec![],
        };
        for (i, q) in qtokens.iter().enumerate() {
            timer.enter(Phase::Lookup);
            let (exp, prefixed) = self.expand(q, i == last);
            timer.exit(Phase::Lookup);
            if exp.is_empty() {
                return None;
            }
            matcher.exact.push(q.clone());
            if prefixed {
                matcher.prefix.push(q.clone());
            }
            per_token.push(self.token_hits(&exp, timer));
        }
        let qn = qtokens.join(" ");
        let (first, rest) = per_token.split_first().expect("non-empty");
        let mut scored: Vec<(u32, f32)> = Vec::new();
        timer.enter(Phase::Scoring);
        for &doc in &first.docs {
            let h0 = first.get(doc).expect("touched document");
            if !accept(doc as usize) {
                continue;
            }
            let mut score = h0.score;
            let mut all_title = h0.in_title;
            let mut ok = true;
            for m in rest {
                match m.get(doc) {
                    Some(h) => {
                        score += h.score;
                        all_title &= h.in_title;
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                continue;
            }
            let d = &self.meta.docs[doc as usize];
            let nt = normalize(&d.title);
            let mut boost = 0.0f32;
            if nt == qn {
                boost = BOOST_EXACT_TITLE;
            } else if d.kind == "rule" && d.aliases.iter().any(|a| normalize(a) == qn) {
                // Exact named rule sections should beat a longer item title sharing the prefix.
                boost = 600.0;
            } else if nt.starts_with(&qn) {
                boost = BOOST_TITLE_PREFIX;
            } else if d.aliases.iter().any(|a| normalize(a) == qn) {
                boost = BOOST_EXACT_ALIAS;
            } else if all_title {
                boost = BOOST_ALL_IN_TITLE;
            }
            scored.push((doc, score + boost));
        }
        timer.exit(Phase::Scoring);
        Some((scored, matcher))
    }

    fn make_hit<T: PhaseTimer>(&self, ord: u32, score: f32, m: &Matcher, timer: &mut T) -> Hit {
        let d = &self.meta.docs[ord as usize];
        timer.enter(Phase::Materialize);
        let (id, title, kind, source) = (
            d.id.clone(),
            d.title.clone(),
            d.kind.clone(),
            d.source.clone(),
        );
        timer.exit(Phase::Materialize);
        let mut hit = Hit {
            ord,
            id,
            title,
            kind,
            source,
            score,
            title_matches: Vec::new(),
            snippet: Snippet::default(),
        };
        self.fill_highlight(&mut hit, m, timer);
        hit
    }

    fn fill_highlight<T: PhaseTimer>(&self, hit: &mut Hit, m: &Matcher, timer: &mut T) {
        timer.enter(Phase::Snippets);
        let d = &self.meta.docs[hit.ord as usize];
        let title_matches = m.ranges(&scan(&d.title));
        let body = self
            .texts
            .texts
            .get(hit.ord as usize)
            .map_or("", |s| cap_text(s));
        let snippet = snippet(body, m);
        timer.exit(Phase::Snippets);
        hit.title_matches = title_matches;
        hit.snippet = snippet;
    }
}

fn result_limit(query: &Query) -> usize {
    if query.limit == 0 {
        DEFAULT_LIMIT
    } else {
        query.limit
    }
}

fn empty_matcher() -> Matcher {
    Matcher {
        exact: Vec::new(),
        prefix: Vec::new(),
    }
}

/// Final result order: score, then source rank, then title, then id. The source rank only
/// separates exactly equal scores. A document without a rank counts as neutral.
fn sort_scored(
    scored: &mut Vec<(u32, f32)>,
    docs: &[crate::store::DocMeta],
    ranks: &[u8],
    limit: usize,
) {
    let rank = |ord: u32| {
        ranks
            .get(ord as usize)
            .copied()
            .unwrap_or(crate::NEUTRAL_SOURCE_RANK)
    };
    let order = |a: &(u32, f32), b: &(u32, f32)| {
        let (da, db) = (&docs[a.0 as usize], &docs[b.0 as usize]);
        b.1.total_cmp(&a.1)
            .then_with(|| rank(a.0).cmp(&rank(b.0)))
            .then_with(|| da.title.cmp(&db.title))
            .then_with(|| da.id.cmp(&db.id))
    };
    if limit < scored.len() {
        // Keep only the best `limit` rows before the final ordering pass.
        scored.select_nth_unstable_by(limit, order);
        scored.truncate(limit);
    }
    scored.sort_unstable_by(order);
}

fn snippet(body: &str, m: &Matcher) -> Snippet {
    let ranges = m.ranges(&scan(body));
    let mut start = 0;
    if let Some(&(first, _)) = ranges.first() {
        start = body[..first]
            .char_indices()
            .rev()
            .nth(SNIP_BEFORE - 1)
            .map_or(0, |(i, _)| i);
        if start > 0 {
            // Snap forward to a word start, within a small slack.
            let tail = &body[start..first];
            if let Some((i, c)) = tail
                .char_indices()
                .take(SNIP_SNAP)
                .find(|(_, c)| c.is_whitespace())
            {
                start += i + c.len_utf8();
            }
        }
    }
    let rest = &body[start..];
    let mut end = rest
        .char_indices()
        .nth(SNIP_LEN)
        .map_or(body.len(), |(i, _)| start + i);
    if end < body.len() {
        let head = &body[start..end];
        if let Some((i, _)) = head
            .char_indices()
            .rev()
            .take(SNIP_SNAP)
            .find(|(_, c)| c.is_whitespace())
            && start + i > start
        {
            end = start + i;
        }
    }
    let (pre, post) = (
        if start > 0 { "…" } else { "" },
        if end < body.len() { "…" } else { "" },
    );
    let shift = pre.len();
    let matches = ranges
        .into_iter()
        .filter(|&(a, b)| a >= start && b <= end)
        .map(|(a, b)| (a - start + shift, b - start + shift))
        .collect();
    Snippet {
        text: format!("{pre}{}{post}", &body[start..end]),
        matches,
    }
}
