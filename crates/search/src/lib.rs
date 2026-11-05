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
pub const INDEX_FORMAT_VERSION: u32 = 1;

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

/// An immutable, queryable index.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchIndex {
    pub(crate) meta: MetaSection,
    pub(crate) postings: PostingsSection,
    pub(crate) texts: TextSection,
}

impl SearchIndex {
    pub(crate) fn from_parts(
        meta: MetaSection,
        postings: PostingsSection,
        texts: TextSection,
    ) -> Self {
        Self {
            meta,
            postings,
            texts,
        }
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
        })
    }

    /// Values of a facet field with the number of docs having each, sorted by value.
    pub fn facet_values(&self, field: &str) -> Vec<(String, u32)> {
        self.facet_values_eligible(field, |_| true)
    }

    pub fn facet_values_eligible(
        &self,
        field: &str,
        eligible: impl Fn(&str) -> bool,
    ) -> Vec<(String, u32)> {
        let mut map: std::collections::BTreeMap<&str, u32> = Default::default();
        for d in self.meta.docs.iter().filter(|d| eligible(&d.id)) {
            let mut seen: Vec<&str> = Vec::new();
            for (f, v) in &d.facets {
                if f.eq_ignore_ascii_case(field) && !seen.contains(&v.as_str()) {
                    seen.push(v);
                    *map.entry(v).or_default() += 1;
                }
            }
        }
        map.into_iter().map(|(v, c)| (v.to_string(), c)).collect()
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
