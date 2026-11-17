//! Lightweight eligibility snapshot for ordinary discovery.

use crate::graph::{Relations, filtered_spell_lists};
use crate::library::{Library, LibraryError};
use cast_iron_schema::SpellLists;
use cast_iron_schema::{EntryKind, EntryStub};
use cast_iron_search::{
    BOOK_SECTION_KIND, Correction, FilterCounts, Filters, Hit, HitSummary, Query,
};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

/// The eligible id set of one selection revision, shared by every [`Discovery`] at that
/// revision. `Library::discovery` keeps one of these, so a page that asks once per entry kind
/// does not rebuild the set each time.
#[derive(Default)]
pub(crate) struct DiscoveryCache {
    cached: RefCell<Option<(u64, Rc<BTreeSet<String>>)>>,
}

impl DiscoveryCache {
    /// The ids for `revision`, building them only when the cache is empty or holds another
    /// revision. A new library has its own cache, so an installed library never reads the old
    /// one's ids.
    pub(crate) fn ids(
        &self,
        revision: u64,
        build: impl FnOnce() -> BTreeSet<String>,
    ) -> Rc<BTreeSet<String>> {
        if let Some((cached_revision, ids)) = self.cached.borrow().as_ref()
            && *cached_revision == revision
        {
            return Rc::clone(ids);
        }
        let ids = Rc::new(build());
        *self.cached.borrow_mut() = Some((revision, Rc::clone(&ids)));
        ids
    }
}

#[derive(Clone)]
pub struct Discovery {
    library: Library,
    /// Shared with every other snapshot at the same selection revision.
    eligible_ids: Rc<BTreeSet<String>>,
}

impl Discovery {
    pub(crate) fn new(library: Library, eligible_ids: Rc<BTreeSet<String>>) -> Self {
        Self {
            library,
            eligible_ids,
        }
    }

    pub fn active_ids(&self) -> &BTreeSet<String> {
        self.eligible_ids.as_ref()
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
        self.kind_counts_counted(|_| true)
    }
    /// Kind totals over the rows a caller also counts. A revision selection hides the revision
    /// row and a removed or merged base, so neither inflates a chip total.
    pub fn kind_counts_counted(&self, counted: impl Fn(&str) -> bool) -> Vec<(EntryKind, usize)> {
        EntryKind::ALL
            .into_iter()
            .map(|kind| {
                (
                    kind,
                    self.entries_of_kind(kind)
                        .filter(|stub| counted(stub.id.as_str()))
                        .count(),
                )
            })
            .filter(|(_, count)| *count > 0)
            .collect()
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
    /// A corrected spelling for a query that finds little, over the entries of the active books
    /// (and, with `sections`, the sections of active books). `None` when nothing needs fixing
    /// or no near name is in an active book. The caller keeps its own filters and scope.
    pub async fn suggest_correction(
        &self,
        text: &str,
        filters: &Filters,
        sections: bool,
    ) -> Result<Option<Correction>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.suggest_correction(text, filters, |doc| {
            if doc.kind == BOOK_SECTION_KIND {
                sections && self.library.book_active(doc.source)
            } else {
                self.is_eligible(doc.id)
            }
        }))
    }
    /// Ranking data for book-section documents in active books, without snippets. A section
    /// document is not an entry id, so it is accepted by its kind and its book's state instead
    /// of the catalog id set. An inactive book contributes no section.
    pub async fn section_summaries(&self, query: &Query) -> Result<Vec<HitSummary>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.search_summaries_docs(query, |doc| {
            doc.kind == BOOK_SECTION_KIND && self.library.book_active(doc.source)
        }))
    }
    /// Counts for the search filter chips: kinds, sources and facet values over the eligible
    /// entries that match `text`, each applying every active filter except its own group.
    pub async fn filter_counts(
        &self,
        text: &str,
        filters: &Filters,
    ) -> Result<FilterCounts, LibraryError> {
        self.filter_counts_counted(text, filters, |_| true).await
    }
    /// Counts over the rows a caller also counts. A revision selection passes its hidden set
    /// here so a base and its revision count once, and a removed or merged feat counts not at
    /// all while its book is active.
    pub async fn filter_counts_counted(
        &self,
        text: &str,
        filters: &Filters,
        counted: impl Fn(&str) -> bool,
    ) -> Result<FilterCounts, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.filter_counts_eligible(text, filters, |id| self.is_eligible(id) && counted(id)))
    }
    pub async fn facet_fields(&self) -> Result<Vec<String>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.facet_fields_eligible(|id| self.is_eligible(id)))
    }
    pub async fn facet_values(&self, field: &str) -> Result<Vec<(String, u32)>, LibraryError> {
        self.facet_values_counted(field, |_| true).await
    }
    pub async fn facet_values_counted(
        &self,
        field: &str,
        counted: impl Fn(&str) -> bool,
    ) -> Result<Vec<(String, u32)>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index.facet_values_eligible(field, |id| self.is_eligible(id) && counted(id)))
    }

    /// The ids of the eligible entries that carry `value` in facet `field`. A list page filters
    /// its A to Z list with them.
    pub async fn facet_entry_ids(
        &self,
        field: &str,
        value: &str,
    ) -> Result<std::collections::HashSet<String>, LibraryError> {
        let index = self.library.search_index().await?;
        Ok(index
            .facet_ids_eligible(field, value, |id| self.is_eligible(id))
            .into_iter()
            .map(str::to_owned)
            .collect())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn the_same_revision_reuses_the_cached_ids() {
        let cache = DiscoveryCache::default();
        let builds = Cell::new(0);
        let first = cache.ids(7, || {
            builds.set(builds.get() + 1);
            BTreeSet::from(["a".to_string()])
        });
        // A second call at the same revision must not run its builder at all.
        let second = cache.ids(7, || {
            builds.set(builds.get() + 1);
            BTreeSet::from(["b".to_string()])
        });
        assert_eq!(builds.get(), 1);
        assert!(Rc::ptr_eq(&first, &second));
        assert!(second.contains("a"));
        assert!(!second.contains("b"));
    }

    #[test]
    fn a_new_revision_rebuilds_the_ids() {
        let cache = DiscoveryCache::default();
        let builds = Cell::new(0);
        let build = |value: &str| {
            let value = value.to_string();
            let builds = &builds;
            move || {
                builds.set(builds.get() + 1);
                BTreeSet::from([value.clone()])
            }
        };
        cache.ids(1, build("a"));
        let next = cache.ids(2, build("b"));
        assert_eq!(builds.get(), 2);
        assert_eq!(next.as_ref(), &BTreeSet::from(["b".to_string()]));
    }

    #[test]
    fn a_library_swap_starts_with_no_cached_ids() {
        // A new library has a new `Inner` and so a new cache. Its first call must build, even
        // at the revision number the old library ended on.
        let old = DiscoveryCache::default();
        old.ids(3, || BTreeSet::from(["old".to_string()]));
        let swapped = DiscoveryCache::default();
        let built = Cell::new(false);
        let ids = swapped.ids(3, || {
            built.set(true);
            BTreeSet::from(["new".to_string()])
        });
        assert!(built.get());
        assert_eq!(ids.as_ref(), &BTreeSet::from(["new".to_string()]));
    }
}
