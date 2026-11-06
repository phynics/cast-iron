//! Lightweight eligibility snapshot for ordinary discovery.

use crate::graph::{Relations, filtered_spell_lists};
use crate::library::{Library, LibraryError};
use cast_iron_schema::SpellLists;
use cast_iron_schema::{EntryKind, EntryStub};
use cast_iron_search::{FilterCounts, Filters, Hit, HitSummary, Query};
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone)]
pub struct Discovery {
    library: Library,
    eligible_ids: BTreeSet<String>,
}

impl Discovery {
    pub(crate) fn new(library: Library) -> Self {
        let eligible_ids = library
            .catalog()
            .entries
            .iter()
            .filter(|stub| library.is_stub_active(stub))
            .map(|stub| stub.id.to_string())
            .collect();
        Self {
            library,
            eligible_ids,
        }
    }

    pub fn active_ids(&self) -> &BTreeSet<String> {
        &self.eligible_ids
    }
    pub fn is_eligible(&self, id: &str) -> bool {
        self.eligible_ids.contains(id)
    }
    pub fn entries(&self) -> impl Iterator<Item = &EntryStub> {
        self.library
            .catalog()
            .entries
            .iter()
            .filter(|stub| self.is_eligible(stub.id.as_str()))
    }
    pub fn entries_of_kind(&self, kind: EntryKind) -> impl Iterator<Item = &EntryStub> {
        self.entries().filter(move |stub| stub.kind == kind)
    }
    pub fn entries_of_source<'a>(&'a self, source: &'a str) -> impl Iterator<Item = &'a EntryStub> {
        self.entries()
            .filter(move |stub| self.library.stub_is_in_source(stub, source))
    }
    pub fn kind_counts(&self) -> Vec<(EntryKind, usize)> {
        EntryKind::ALL
            .into_iter()
            .map(|kind| (kind, self.entries_of_kind(kind).count()))
            .filter(|(_, count)| *count > 0)
            .collect()
    }
    pub fn source_counts(&self) -> Vec<(String, usize)> {
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for stub in self.entries() {
            if let Some(source) = self.library.canonical_source_for_entry(&stub.id) {
                *counts.entry(source).or_default() += 1;
            }
        }
        counts.into_iter().collect()
    }
    pub async fn search(&self, query: &Query) -> Result<Vec<Hit>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.search_eligible(query, |id| self.is_eligible(id)))
    }
    /// Ranking data for the whole result set, without snippets. The page
    /// materializes only the rows it shows.
    pub async fn search_summaries(&self, query: &Query) -> Result<Vec<HitSummary>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.search_summaries_eligible(query, |id| self.is_eligible(id)))
    }
    /// Counts for the search filter chips: kinds, sources and facet values over the eligible
    /// entries that match `text`, each applying every active filter except its own group.
    pub async fn filter_counts(
        &self,
        text: &str,
        filters: &Filters,
    ) -> Result<FilterCounts, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.filter_counts_eligible(text, filters, |id| self.is_eligible(id)))
    }
    pub async fn facet_fields(&self) -> Result<Vec<String>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.facet_fields_eligible(|id| self.is_eligible(id)))
    }
    pub async fn facet_values(&self, field: &str) -> Result<Vec<(String, u32)>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.facet_values_eligible(field, |id| self.is_eligible(id)))
    }

    /// Relationship graph projected for ordinary discovery. Both endpoints must be active.
    pub async fn relations(&self) -> Result<Option<Rc<Relations>>, LibraryError> {
        Ok(self
            .library
            .relations()
            .await?
            .map(|graph| Rc::new(graph.filtered(|id| self.is_eligible(id.as_str())))))
    }

    /// Spell lists projected for ordinary discovery. Spell positions remain canonical, while
    /// disabled spells and empty levels/lists are removed. A disabled class link is cleared.
    pub async fn spell_lists(&self) -> Result<Option<Rc<SpellLists>>, LibraryError> {
        Ok(self.library.spell_lists().await?.map(|lists| {
            Rc::new(filtered_spell_lists(&lists, |id| {
                self.is_eligible(id.as_str())
            }))
        }))
    }
}
