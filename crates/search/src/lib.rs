//! Offline search: normalization, deterministic index building, BM25 ranking
//! with title boosts, prefix matching, filters, and highlighted snippets.
//! Compiles for native and `wasm32-unknown-unknown`.

mod build;
mod correct;
mod mask;
mod query;
mod store;
mod text;

#[cfg(test)]
mod tests;

pub use build::IndexBuilder;
pub use correct::{CORRECTION_MAX_HITS, Correction};
pub use mask::EligibleMask;
#[cfg(feature = "profile")]
pub use query::SearchTiming;
pub use query::{Highlighter, HitSummary};
pub use store::IndexError;
pub use text::{normalize, tokenize};

use store::{MetaSection, PostingsSection, TextSection};

/// Bumped whenever the serialized layout changes.
///
/// 2: document metadata carries a book section's chapter id and anchor. A version 1 library is
/// rejected cleanly and the reader is sent to Settings for an update.
pub const INDEX_FORMAT_VERSION: u32 = 2;

/// The kind of a book-section document: a node of a book's contents tree. Its id is not an
/// entry id; the result opens `chapter` at `anchor` on the book's chapter page.
pub const BOOK_SECTION_KIND: &str = "book-section";

/// The rank of a source that has no assigned rank. A lower rank sorts first on a relevance tie,
/// so the neutral rank sorts last. See [`SearchIndex::set_source_ranks`].
pub const NEUTRAL_SOURCE_RANK: u8 = u8::MAX;

/// A document to index.
#[derive(Debug, Clone, Default)]
pub struct DocInput {
    pub id: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub kind: String,
    pub source: String,
    pub section: String,
    pub facets: Vec<(String, String)>,
    pub text: String,
    /// For a book-section document, the permanent id of the chapter page it opens.
    pub chapter: String,
    /// For a book-section document, the anchor of its row on that page. Empty for the chapter.
    pub anchor: String,
}

/// Borrowed view of a stored document's metadata.
#[derive(Debug, Clone, Copy)]
pub struct DocView<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub kind: &'a str,
    pub source: &'a str,
    pub section: &'a str,
    pub facets: &'a [(String, String)],
    pub chapter: &'a str,
    pub anchor: &'a str,
}

/// Kinds and sources are OR within a list, AND across lists. Facets are OR
/// within one field and AND across fields. Comparison is case-insensitive.
#[derive(Default, Clone, Debug)]
pub struct Filters {
    pub kinds: Vec<String>,
    pub sources: Vec<String>,
    pub facets: Vec<(String, String)>,
}

impl Filters {
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty() && self.sources.is_empty() && self.facets.is_empty()
    }
}

/// A search request. `limit` 0 means the default of 50.
#[derive(Default, Clone, Debug)]
pub struct Query {
    pub text: String,
    pub filters: Filters,
    pub limit: usize,
}

/// Preview text with byte ranges of highlighted matches.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snippet {
    pub text: String,
    pub matches: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub ord: u32,
    pub id: String,
    pub title: String,
    pub kind: String,
    pub source: String,
    pub score: f32,
    pub title_matches: Vec<(usize, usize)>,
    pub snippet: Snippet,
    /// The first alias of the document that holds every query word, when no word of the title
    /// matched.
    /// Filled by [`SearchIndex::materialize_hit`] and the full search. An alias is a name that
    /// the compiler added, such as a class feature heading (#540), so the caller can tell that
    /// the query named a part of the entry and open it there. Not part of the index format.
    pub alias: Option<String>,
}

/// Counts behind the filter chips; see [`SearchIndex::filter_counts_eligible`]. Facet keys are
/// `(lowercase field, value as indexed)`; kinds and sources are keyed as indexed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilterCounts {
    /// Documents that match the query text and every active filter: the full result count.
    pub total: u32,
    pub kinds: std::collections::BTreeMap<String, u32>,
    pub sources: std::collections::BTreeMap<String, u32>,
    pub facets: std::collections::BTreeMap<(String, String), u32>,
}

impl FilterCounts {
    pub fn kind(&self, kind: &str) -> u32 {
        self.kinds.get(kind).copied().unwrap_or(0)
    }
    pub fn source(&self, source: &str) -> u32 {
        self.sources.get(source).copied().unwrap_or(0)
    }
    pub fn facet(&self, field: &str, value: &str) -> u32 {
        self.facets
            .get(&(field.to_lowercase(), value.to_string()))
            .copied()
            .unwrap_or(0)
    }
}

/// Document lists for every filter value, built once from the metadata. Counting a chip then
/// reads a membership list instead of scanning every document once per filter.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct DocLists {
    pub kinds: std::collections::BTreeMap<String, Vec<u32>>,
    pub sources: std::collections::BTreeMap<String, Vec<u32>>,
    /// Lowercase field -> value as indexed -> document ordinals, ascending.
    pub facets: std::collections::BTreeMap<String, std::collections::BTreeMap<String, Vec<u32>>>,
    /// Counts over every document, with no query and no filters.
    pub totals: FilterCounts,
}

/// The books a document belongs to beyond its own `source`, by document ordinal. Plain data
/// from the caller (see [`SearchIndex::set_extra_sources`]); never serialized.
pub(crate) type ExtraSources = std::collections::BTreeMap<u32, Vec<String>>;

impl DocLists {
    fn build(docs: &[store::DocMeta], extra: &ExtraSources) -> Self {
        let mut lists = Self::default();
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for (ord, doc) in docs.iter().enumerate() {
            let ord = ord as u32;
            lists.kinds.entry(doc.kind.clone()).or_default().push(ord);
            lists
                .sources
                .entry(doc.source.clone())
                .or_default()
                .push(ord);
            for book in extra.get(&ord).into_iter().flatten() {
                lists.sources.entry(book.clone()).or_default().push(ord);
            }
            seen.clear();
            for (field, value) in &doc.facets {
                if seen.contains(&(field.as_str(), value.as_str())) {
                    continue;
                }
                seen.push((field, value));
                lists
                    .facets
                    .entry(field.to_lowercase())
                    .or_default()
                    .entry(value.clone())
                    .or_default()
                    .push(ord);
            }
        }
        lists.totals.total = docs.len() as u32;
        for (kind, ords) in &lists.kinds {
            lists.totals.kinds.insert(kind.clone(), ords.len() as u32);
        }
        for (source, ords) in &lists.sources {
            lists
                .totals
                .sources
                .insert(source.clone(), ords.len() as u32);
        }
        for (field, values) in &lists.facets {
            for (value, ords) in values {
                lists
                    .totals
                    .facets
                    .insert((field.clone(), value.clone()), ords.len() as u32);
            }
        }
        lists
    }
}

/// An immutable, queryable index.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchIndex {
    pub(crate) meta: MetaSection,
    pub(crate) postings: PostingsSection,
    pub(crate) texts: TextSection,
    pub(crate) doc_lists: DocLists,
    /// Tie-break rank per document ordinal, empty until [`SearchIndex::set_source_ranks`] runs.
    /// Not serialized: the client derives it from the catalog, so the format is unchanged.
    pub(crate) doc_ranks: Vec<u8>,
    /// Printing recency per document ordinal, empty until [`SearchIndex::set_printing_recency`]
    /// runs. A larger value is a newer printing; 0 is undated. Not serialized.
    pub(crate) doc_recency: Vec<u32>,
    /// The other books a document belongs to, besides its own `source`. Empty until
    /// [`SearchIndex::set_extra_sources`] runs. Not serialized, so the format is unchanged.
    pub(crate) extra_sources: ExtraSources,
    /// The words of titles and aliases, built when the first correction is asked for. Not
    /// serialized, so the format is unchanged.
    pub(crate) names: correct::LazyNames,
}

impl SearchIndex {
    pub(crate) fn from_parts(
        meta: MetaSection,
        postings: PostingsSection,
        texts: TextSection,
    ) -> Self {
        let doc_lists = DocLists::build(&meta.docs, &ExtraSources::new());
        Self {
            meta,
            postings,
            texts,
            doc_lists,
            doc_ranks: Vec::new(),
            doc_recency: Vec::new(),
            extra_sources: ExtraSources::new(),
            names: correct::LazyNames::default(),
        }
    }

    /// Assigns each source a rank for breaking ties between equally relevant results. A lower
    /// rank sorts first. Relevance always wins: the rank only orders documents whose scores are
    /// exactly equal, ahead of the title and id tie-breaks. A source that is not listed gets
    /// [`NEUTRAL_SOURCE_RANK`], and an index that never receives ranks ranks every document the
    /// same. The ranks are plain data from the caller and are not part of the serialized index.
    pub fn set_source_ranks<'a>(&mut self, ranks: impl IntoIterator<Item = (&'a str, u8)>) {
        let by_source: std::collections::BTreeMap<&str, u8> = cast_iron_sort::map_of(ranks);
        self.doc_ranks = self
            .meta
            .docs
            .iter()
            .map(|doc| {
                by_source
                    .get(doc.source.as_str())
                    .copied()
                    .unwrap_or(NEUTRAL_SOURCE_RANK)
            })
            .collect();
    }

    /// Assigns each document a printing recency, as `(document id, recency)` pairs, for breaking
    /// ties between equally relevant results of the same rule printed in several books. A larger
    /// recency is a newer printing and sorts first; 0 means undated and sorts last. The recency
    /// is a further tie-break after the score and the source rank, ahead of the title and id
    /// tie-breaks, so relevance and product line still win. A document that is not listed counts
    /// as undated, and a pair whose id is not a document is ignored. Plain data from the caller
    /// (the catalog), so the serialized format is unchanged. Each call replaces the earlier pairs.
    pub fn set_printing_recency<'a>(&mut self, recency: impl IntoIterator<Item = (&'a str, u32)>) {
        let mut pairs = recency.into_iter().peekable();
        if pairs.peek().is_none() {
            self.doc_recency = Vec::new();
            return;
        }
        let ords: std::collections::HashMap<&str, usize> = self
            .meta
            .docs
            .iter()
            .enumerate()
            .map(|(ord, doc)| (doc.id.as_str(), ord))
            .collect();
        let mut by_doc = vec![0u32; self.meta.docs.len()];
        for (id, value) in pairs {
            if let Some(&ord) = ords.get(id) {
                by_doc[ord] = value;
            }
        }
        self.doc_recency = by_doc;
    }

    /// Tells the index which further books each entry belongs to, as `(document id, book)`
    /// pairs. The index records one `source` per document; an entry printed in several books
    /// belongs to all of them. A source filter then keeps a document that any of its books
    /// matches, and a source chip counts it once under each book. Ranking and `Hit::source`
    /// still use the document's own source. A pair whose id is not a document is ignored.
    /// Plain data from the caller (the catalog), so the serialized format is unchanged. Each
    /// call replaces the earlier pairs.
    pub fn set_extra_sources<'a>(&mut self, pairs: impl IntoIterator<Item = (&'a str, &'a str)>) {
        let mut ords: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        let mut extra = ExtraSources::new();
        let mut wanted: Vec<(&str, &str)> = pairs.into_iter().collect();
        if !wanted.is_empty() {
            ords.extend(
                self.meta
                    .docs
                    .iter()
                    .enumerate()
                    .map(|(ord, doc)| (doc.id.as_str(), ord as u32)),
            );
        }
        cast_iron_sort::sort(&mut wanted);
        wanted.dedup();
        for (id, book) in wanted {
            let Some(&ord) = ords.get(id) else { continue };
            if self.meta.docs[ord as usize].source != book {
                extra.entry(ord).or_default().push(book.to_owned());
            }
        }
        self.doc_lists = DocLists::build(&self.meta.docs, &extra);
        self.extra_sources = extra;
    }

    /// True when the document belongs to `source`: its own source, or an extra book. Case is
    /// ignored, like every filter.
    pub(crate) fn in_source(&self, ord: usize, source: &str) -> bool {
        self.meta.docs[ord].source.eq_ignore_ascii_case(source)
            || self
                .extra_sources
                .get(&(ord as u32))
                .is_some_and(|books| books.iter().any(|book| book.eq_ignore_ascii_case(source)))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        store::encode(
            INDEX_FORMAT_VERSION,
            &self.meta,
            &self.postings,
            &self.texts,
        )
    }

    /// Never panics; corrupt or truncated input yields `Err`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IndexError> {
        let (m, p, t) = store::decode(INDEX_FORMAT_VERSION, bytes)?;
        Ok(Self::from_parts(m, p, t))
    }

    pub fn doc_count(&self) -> usize {
        self.meta.docs.len()
    }

    pub fn doc(&self, ord: u32) -> Option<DocView<'_>> {
        let d = self.meta.docs.get(ord as usize)?;
        Some(DocView {
            id: &d.id,
            title: &d.title,
            kind: &d.kind,
            source: &d.source,
            section: &d.section,
            facets: &d.facets,
            chapter: &d.chapter,
            anchor: &d.anchor,
        })
    }

    /// Values of a facet field with the number of docs having each, sorted by value.
    pub fn facet_values(&self, field: &str) -> Vec<(String, u32)> {
        self.doc_lists
            .facets
            .get(&field.to_lowercase())
            .map(|values| {
                values
                    .iter()
                    .map(|(value, ords)| (value.clone(), ords.len() as u32))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn facet_values_eligible(
        &self,
        field: &str,
        eligible: impl Fn(&str) -> bool,
    ) -> Vec<(String, u32)> {
        let Some(values) = self.doc_lists.facets.get(&field.to_lowercase()) else {
            return Vec::new();
        };
        values
            .iter()
            .filter_map(|(value, ords)| {
                let count = ords
                    .iter()
                    .filter(|&&ord| {
                        self.meta
                            .docs
                            .get(ord as usize)
                            .is_some_and(|doc| eligible(&doc.id))
                    })
                    .count() as u32;
                (count > 0).then(|| (value.clone(), count))
            })
            .collect()
    }

    /// The ids of the eligible documents that carry `value` in facet `field`, in index order.
    /// The field and the value match without regard to case, like a search filter does. A list
    /// page uses this to filter its A to Z list in place.
    pub fn facet_ids_eligible(
        &self,
        field: &str,
        value: &str,
        eligible: impl Fn(&str) -> bool,
    ) -> Vec<&str> {
        let Some(values) = self.doc_lists.facets.get(&field.to_lowercase()) else {
            return Vec::new();
        };
        let mut ords: Vec<u32> = values
            .iter()
            .filter(|(indexed, _)| indexed.eq_ignore_ascii_case(value))
            .flat_map(|(_, ords)| ords.iter().copied())
            .collect();
        cast_iron_sort::sort(&mut ords);
        ords.dedup();
        ords.into_iter()
            .filter_map(|ord| self.meta.docs.get(ord as usize))
            .filter(|doc| eligible(&doc.id))
            .map(|doc| doc.id.as_str())
            .collect()
    }

    pub fn facet_fields(&self) -> Vec<String> {
        self.facet_fields_eligible(|_| true)
    }

    pub fn facet_fields_eligible(&self, eligible: impl Fn(&str) -> bool) -> Vec<String> {
        let set: std::collections::BTreeSet<&str> = cast_iron_sort::set_of(
            self.meta
                .docs
                .iter()
                .filter(|d| eligible(&d.id))
                .flat_map(|d| d.facets.iter().map(|(f, _)| f.as_str())),
        );
        set.into_iter().map(String::from).collect()
    }
}
