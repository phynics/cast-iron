//! Offline search: normalization, deterministic index building, BM25 ranking
//! with title boosts, prefix matching, filters, and highlighted snippets.
//! Compiles for native and `wasm32-unknown-unknown`.

mod build;
mod query;
mod store;
mod text;

#[cfg(test)]
mod tests;

pub use build::IndexBuilder;
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

impl DocLists {
    fn build(docs: &[store::DocMeta]) -> Self {
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
}

impl SearchIndex {
    pub(crate) fn from_parts(
        meta: MetaSection,
        postings: PostingsSection,
        texts: TextSection,
    ) -> Self {
        let doc_lists = DocLists::build(&meta.docs);
        Self {
            meta,
            postings,
            texts,
            doc_lists,
            doc_ranks: Vec::new(),
        }
    }

    /// Assigns each source a rank for breaking ties between equally relevant results. A lower
    /// rank sorts first. Relevance always wins: the rank only orders documents whose scores are
    /// exactly equal, ahead of the title and id tie-breaks. A source that is not listed gets
    /// [`NEUTRAL_SOURCE_RANK`], and an index that never receives ranks ranks every document the
    /// same. The ranks are plain data from the caller and are not part of the serialized index.
    pub fn set_source_ranks<'a>(&mut self, ranks: impl IntoIterator<Item = (&'a str, u8)>) {
        let by_source: std::collections::BTreeMap<&str, u8> = ranks.into_iter().collect();
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
        ords.sort_unstable();
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
        let set: std::collections::BTreeSet<&str> = self
            .meta
            .docs
            .iter()
            .filter(|d| eligible(&d.id))
            .flat_map(|d| d.facets.iter().map(|(f, _)| f.as_str()))
            .collect();
        set.into_iter().map(String::from).collect()
    }
}
