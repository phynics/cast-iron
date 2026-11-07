//! Lightweight eligibility snapshot for ordinary discovery.

use crate::graph::{BookLookup, Relations, filtered_spell_lists};
use crate::library::{Library, LibraryError, canonical_book_source, count_by_source};
use cast_iron_schema::SpellLists;
use cast_iron_schema::{EntryKind, EntryStub};
use cast_iron_search::{FilterCounts, Filters, Hit, HitSummary, Query};
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

/// The ids of the catalog entries whose canonical book is active, in one pass. The set is what
/// discovery tests every search hit, list row, and relation against.
pub(crate) fn eligible_ids(
    entries: &[EntryStub],
    books: Option<&BookLookup>,
    disabled: &BTreeSet<String>,
) -> BTreeSet<String> {
    entries
        .iter()
        .filter(|stub| !disabled.contains(canonical_book_source(books, &stub.id, &stub.source)))
        .map(|stub| stub.id.to_string())
        .collect()
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
        EntryKind::ALL
            .into_iter()
            .map(|kind| (kind, self.entries_of_kind(kind).count()))
            .filter(|(_, count)| *count > 0)
            .collect()
    }
    pub fn source_counts(&self) -> Vec<(String, usize)> {
        self.library
            .with_books(|books| count_by_source(self.entries(), books))
            .into_iter()
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
    use cast_iron_schema::{BookIndex, BookNode, BookTree, EntryId};
    use std::cell::Cell;
    use std::collections::BTreeMap;

    fn stub(id: &str, source: &str) -> EntryStub {
        EntryStub {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Rule,
            title: id.to_string(),
            source: source.into(),
            parent: None,
            shard: "entries/rule-000.json".into(),
        }
    }

    fn placed(entry: &EntryId) -> BookNode {
        BookNode {
            title: "Chapter".into(),
            entry: Some(entry.clone()),
            anchor: None,
            review: None,
            children: vec![],
        }
    }

    /// Three catalog sources, one entry moved to another book, and one book with no entries.
    fn fixture() -> (Vec<EntryStub>, BookLookup) {
        let entries = vec![
            stub("a/one", "legacy-a"),
            stub("a/two", "legacy-a"),
            stub("b/one", "legacy-b"),
            stub("c/one", "legacy-c"),
        ];
        let lookup = BookLookup::new(BookIndex {
            books: vec![BookTree {
                source: "book-b".into(),
                title: "B".into(),
                basis: String::new(),
                review: vec![],
                chapters: vec![placed(&entries[0].id), placed(&entries[2].id)],
            }],
        });
        (entries, lookup)
    }

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

    #[test]
    fn eligibility_uses_the_canonical_book_and_the_disabled_set() {
        let (entries, lookup) = fixture();
        // `a/one` and `b/one` are placed in book-b, so disabling book-b leaves the unplaced
        // entries: `a/two` and `c/one`.
        let disabled = BTreeSet::from(["book-b".to_string()]);
        let ids = eligible_ids(&entries, Some(&lookup), &disabled);
        assert_eq!(
            ids,
            BTreeSet::from(["a/two".to_string(), "c/one".to_string()])
        );
        // Without a book index a disabled book id hides no catalog source, because every entry
        // stays under its catalog source; disabling `legacy-c` hides only `c/one`.
        let disabled = BTreeSet::from(["legacy-c".to_string()]);
        let ids = eligible_ids(&entries, None, &disabled);
        assert_eq!(
            ids,
            BTreeSet::from([
                "a/one".to_string(),
                "a/two".to_string(),
                "b/one".to_string(),
            ])
        );
    }

    /// The allocating per-entry lookup that discovery's source counts used before: resolve the
    /// stub's canonical source into an owned `String` for every eligible entry.
    fn allocating_counts(
        eligible: &[&EntryStub],
        books: Option<&BookLookup>,
    ) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::<String, usize>::new();
        for stub in eligible {
            let source = canonical_book_source(books, &stub.id, &stub.source).to_owned();
            *counts.entry(source).or_default() += 1;
        }
        counts
    }

    #[test]
    fn one_pass_eligible_source_counts_match_the_allocating_lookup() {
        let (entries, lookup) = fixture();
        let disabled = BTreeSet::from(["legacy-c".to_string()]);
        for books in [Some(&lookup), None] {
            let eligible: Vec<&EntryStub> = entries
                .iter()
                .filter(|stub| {
                    !disabled.contains(canonical_book_source(books, &stub.id, &stub.source))
                })
                .collect();
            let old = allocating_counts(&eligible, books);
            let new = count_by_source(eligible.iter().copied(), books);
            assert_eq!(old, new);
        }
        let eligible: Vec<&EntryStub> = entries.iter().collect();
        let counts = count_by_source(eligible.iter().copied(), Some(&lookup));
        assert_eq!(counts.get("book-b"), Some(&2));
        assert_eq!(counts.get("legacy-a"), Some(&1));
        assert_eq!(counts.get("legacy-c"), Some(&1));
        assert!(!counts.contains_key("legacy-b"));
    }
}
