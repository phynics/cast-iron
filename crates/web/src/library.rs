//! Read access to the active library. Every read goes through the Cache API under the
//! active version. There is no network fallback: a missing file is an error to show.

use crate::content_cache;
use crate::graph::{BookLookup, Relations};
use crate::install;
use crate::lru::Lru;
use crate::store::{self, ActiveLibrary};
use cast_iron_schema::{
    BOOKS_FILE, BookIndex, Catalog, ContentManifest, CoverageReport, EntryId, EntryKind, EntryStub,
    LegalNotices, RELATIONS_FILE, RelationIndex, RuleEntry, SPELL_LISTS_FILE, SourceInfo,
    SpellLists,
};
use cast_iron_search::{Hit, Query, SearchIndex};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;
use web_sys::Cache;

/// Parsed shards to keep in memory.
const SHARD_CACHE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryError {
    /// A file that the manifest lists is not in the cache.
    Missing(String),
    /// A file is there but does not parse, or does not hold what the catalog promised.
    Corrupt {
        path: String,
        message: String,
    },
    UnknownEntry(String),
    Storage(String),
    BookSelection(String),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LibraryError::Missing(path) => write!(
                f,
                "The library file {path} is missing from this device. Open Settings and reinstall the library."
            ),
            LibraryError::Corrupt { path, message } => {
                write!(f, "The library file {path} is damaged: {message}")
            }
            LibraryError::UnknownEntry(id) => write!(f, "No entry {id} exists in this library."),
            LibraryError::Storage(message) => write!(f, "Cannot read the library: {message}"),
            LibraryError::BookSelection(message) => {
                write!(f, "Cannot read your active-book choices: {message}")
            }
        }
    }
}

impl std::error::Error for LibraryError {}

struct Inner {
    active: ActiveLibrary,
    manifest: ContentManifest,
    catalog: Catalog,
    cache: Cache,
    shards: RefCell<Lru<String, Rc<Vec<RuleEntry>>>>,
    index: RefCell<Option<Rc<SearchIndex>>>,
    legal: RefCell<Option<Rc<LegalNotices>>>,
    coverage: RefCell<Option<Rc<CoverageReport>>>,
    /// Schema-3 files. The outer `Option` is "not loaded yet"; the inner one is "this library
    /// does not have the file", which is a normal state for schemas 1 and 2.
    relations: RefCell<Option<Option<Rc<Relations>>>>,
    spell_lists: RefCell<Option<Option<Rc<SpellLists>>>>,
    books: RefCell<Option<Option<Rc<BookLookup>>>>,
    disabled_books: RefCell<BTreeSet<String>>,
    selection_revision: RwSignal<u64>,
    selection_write_in_flight: RefCell<bool>,
}

/// True when the manifest lists `name` in the version directory. Libraries of schema 1 and 2
/// have no relation, spell-list, or book files, and every reader must treat them as absent.
pub fn manifest_has_file(manifest: &ContentManifest, name: &str) -> bool {
    let path = format!("{}/{name}", manifest.content_version);
    manifest.files.iter().any(|f| f.path == path)
}

/// The active library. Cloning is cheap and shares caches. Get it with [`use_library`].
///
/// The wrapper makes the handle `Send + Sync` so that Leptos context accepts it; the browser
/// runs this app on one thread, so it is never touched from another.
#[derive(Clone)]
pub struct Library(SendWrapper<Rc<Inner>>);

/// The library of the current Reader. Panics outside the Reader subtree.
pub fn use_library() -> Library {
    expect_context::<Library>()
}

impl Library {
    /// Loads an activated library: checks that its cache holds every manifest file, then reads
    /// the catalog. The search index, legal text, and coverage report load on first use.
    pub async fn load(active: &ActiveLibrary) -> Result<Library, LibraryError> {
        let manifest = install::check_installed(active)
            .await
            .map_err(LibraryError::Storage)?;
        let cache = content_cache::open(&active.content_version)
            .await
            .map_err(LibraryError::Storage)?;
        let catalog: Catalog = read_json(&cache, &active.content_version, "catalog.json").await?;
        let books = if manifest_has_file(&manifest, BOOKS_FILE) {
            let index: BookIndex = read_json(&cache, &active.content_version, BOOKS_FILE).await?;
            Some(Rc::new(BookLookup::new(index)))
        } else {
            None
        };
        let disabled_books = store::disabled_books()
            .await
            .map_err(|e| LibraryError::BookSelection(e.to_string()))?;
        Ok(Library(SendWrapper::new(Rc::new(Inner {
            active: active.clone(),
            manifest,
            catalog,
            cache,
            shards: RefCell::new(Lru::new(SHARD_CACHE)),
            index: RefCell::new(None),
            legal: RefCell::new(None),
            coverage: RefCell::new(None),
            relations: RefCell::new(None),
            spell_lists: RefCell::new(None),
            books: RefCell::new(Some(books)),
            disabled_books: RefCell::new(disabled_books),
            selection_revision: RwSignal::new(0),
            selection_write_in_flight: RefCell::new(false),
        }))))
    }

    pub fn active(&self) -> &ActiveLibrary {
        &self.0.active
    }

    pub fn version(&self) -> &str {
        &self.0.active.content_version
    }

    pub fn manifest(&self) -> &ContentManifest {
        &self.0.manifest
    }

    pub fn catalog(&self) -> &Catalog {
        &self.0.catalog
    }

    pub fn sources(&self) -> &[SourceInfo] {
        &self.0.catalog.sources
    }

    pub fn selection_revision(&self) -> RwSignal<u64> {
        self.0.selection_revision
    }

    pub fn book_active(&self, canonical_id: &str) -> bool {
        !self.0.disabled_books.borrow().contains(canonical_id)
    }

    pub fn is_entry_active(&self, id: &EntryId) -> bool {
        let Some(stub) = self.stub(id) else {
            return false;
        };
        !is_book_disabled(
            &self.0.disabled_books.borrow(),
            canonical_book_source(
                self.0.books.borrow().as_ref().and_then(Option::as_deref),
                id,
                &stub.source,
            ),
        )
    }

    pub fn canonical_source_for_entry(&self, id: &EntryId) -> Option<String> {
        let stub = self.stub(id)?;
        let books = self.0.books.borrow();
        Some(
            canonical_book_source(books.as_ref().and_then(Option::as_deref), id, &stub.source)
                .to_owned(),
        )
    }

    pub async fn set_book_active(
        &self,
        canonical_id: &str,
        active: bool,
    ) -> Result<(), LibraryError> {
        {
            let mut busy = self.0.selection_write_in_flight.borrow_mut();
            if *busy {
                return Err(LibraryError::Storage(
                    "A book selection update is already being saved. Try again shortly.".into(),
                ));
            }
            *busy = true;
        }
        struct Reset<'a>(&'a RefCell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                *self.0.borrow_mut() = false;
            }
        }
        let _reset = Reset(&self.0.selection_write_in_flight);
        let mut next = self.0.disabled_books.borrow().clone();
        if active {
            next.remove(canonical_id);
        } else {
            next.insert(canonical_id.to_owned());
        }
        if next == *self.0.disabled_books.borrow() {
            return Ok(());
        }
        store::set_disabled_books(&next)
            .await
            .map_err(|e| LibraryError::Storage(e.to_string()))?;
        *self.0.disabled_books.borrow_mut() = next;
        self.0
            .selection_revision
            .update(|revision| *revision = revision.wrapping_add(1));
        Ok(())
    }

    pub fn discovery(&self) -> crate::discovery::Discovery {
        crate::discovery::Discovery::new(self.clone())
    }

    pub fn source(&self, id: &str) -> Option<&SourceInfo> {
        self.0.catalog.sources.iter().find(|s| s.id == id)
    }

    /// The catalog stub for an id. Needs no file read.
    pub fn stub(&self, id: &EntryId) -> Option<&EntryStub> {
        let entries = &self.0.catalog.entries;
        match entries.binary_search_by(|stub| stub.id.cmp(id)) {
            Ok(i) => Some(&entries[i]),
            // The catalog promises sorted ids; a scan keeps lookups right if that ever slips.
            Err(_) => entries.iter().find(|stub| stub.id == *id),
        }
    }

    pub fn entries_of_kind(&self, kind: EntryKind) -> impl Iterator<Item = &EntryStub> {
        self.0
            .catalog
            .entries
            .iter()
            .filter(move |s| s.kind == kind)
    }

    pub fn entries_of_source<'a>(&'a self, source: &'a str) -> impl Iterator<Item = &'a EntryStub> {
        self.0
            .catalog
            .entries
            .iter()
            .filter(move |s| s.source == source)
    }

    /// Entry counts per kind, in `EntryKind::ALL` order, without empty kinds.
    pub fn kind_counts(&self) -> Vec<(EntryKind, usize)> {
        EntryKind::ALL
            .into_iter()
            .map(|kind| (kind, self.entries_of_kind(kind).count()))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    /// The full entry, from its shard. Shards are parsed once and kept in a small LRU.
    pub async fn entry(&self, id: &EntryId) -> Result<RuleEntry, LibraryError> {
        let stub = self
            .stub(id)
            .ok_or_else(|| LibraryError::UnknownEntry(id.to_string()))?;
        let shard = self.shard(&stub.shard).await?;
        shard
            .iter()
            .find(|entry| entry.id == *id)
            .cloned()
            .ok_or_else(|| LibraryError::Corrupt {
                path: stub.shard.clone(),
                message: format!("the shard does not contain {id}"),
            })
    }

    async fn shard(&self, path: &str) -> Result<Rc<Vec<RuleEntry>>, LibraryError> {
        if let Some(hit) = self.0.shards.borrow_mut().get(&path.to_string()) {
            return Ok(hit);
        }
        let parsed: Vec<RuleEntry> = read_json(&self.0.cache, self.version(), path).await?;
        let parsed = Rc::new(parsed);
        self.0
            .shards
            .borrow_mut()
            .insert(path.to_string(), parsed.clone());
        Ok(parsed)
    }

    /// The search index, loaded from `search.bin` on first use.
    pub async fn search_index(&self) -> Result<Rc<SearchIndex>, LibraryError> {
        if let Some(index) = self.0.index.borrow().clone() {
            return Ok(index);
        }
        let bytes = read_file(&self.0.cache, self.version(), "search.bin").await?;
        let index = SearchIndex::from_bytes(&bytes).map_err(|e| LibraryError::Corrupt {
            path: format!("{}/search.bin", self.version()),
            message: format!("{e:?}"),
        })?;
        let index = Rc::new(index);
        *self.0.index.borrow_mut() = Some(index.clone());
        Ok(index)
    }

    /// Runs a search against the local index.
    pub async fn search(&self, query: &Query) -> Result<Vec<Hit>, LibraryError> {
        Ok(self.search_index().await?.search(query))
    }

    pub async fn legal(&self) -> Result<Rc<LegalNotices>, LibraryError> {
        if let Some(legal) = self.0.legal.borrow().clone() {
            return Ok(legal);
        }
        let legal: LegalNotices = read_json(&self.0.cache, self.version(), "legal.json").await?;
        let legal = Rc::new(legal);
        *self.0.legal.borrow_mut() = Some(legal.clone());
        Ok(legal)
    }

    /// Whether this library has the relation index. False for schemas 1 and 2: the screens that
    /// need it then hide their panels instead of failing.
    pub fn has_relations(&self) -> bool {
        manifest_has_file(&self.0.manifest, RELATIONS_FILE)
    }

    pub fn has_spell_lists(&self) -> bool {
        manifest_has_file(&self.0.manifest, SPELL_LISTS_FILE)
    }

    pub fn has_books(&self) -> bool {
        manifest_has_file(&self.0.manifest, BOOKS_FILE)
    }

    /// The relation index, with its inverse lookup, loaded on first use and parsed once.
    /// `Ok(None)` means that this library has no such file.
    pub async fn relations(&self) -> Result<Option<Rc<Relations>>, LibraryError> {
        self.lazy(&self.0.relations, RELATIONS_FILE, |index: RelationIndex| {
            Relations::new(index)
        })
        .await
    }

    /// Spell lists per class and level. `Ok(None)` for a library without `lists.json`.
    pub async fn spell_lists(&self) -> Result<Option<Rc<SpellLists>>, LibraryError> {
        self.lazy(
            &self.0.spell_lists,
            SPELL_LISTS_FILE,
            |lists: SpellLists| lists,
        )
        .await
    }

    /// The books in original chapter order. `Ok(None)` for a library without `books.json`.
    pub async fn books(&self) -> Result<Option<Rc<BookLookup>>, LibraryError> {
        self.lazy(&self.0.books, BOOKS_FILE, |index: BookIndex| {
            BookLookup::new(index)
        })
        .await
    }

    async fn lazy<T: DeserializeOwned, U>(
        &self,
        slot: &RefCell<Option<Option<Rc<U>>>>,
        file: &str,
        build: impl FnOnce(T) -> U,
    ) -> Result<Option<Rc<U>>, LibraryError> {
        if let Some(loaded) = slot.borrow().clone() {
            return Ok(loaded);
        }
        let loaded = if manifest_has_file(&self.0.manifest, file) {
            let parsed: T = read_json(&self.0.cache, self.version(), file).await?;
            Some(Rc::new(build(parsed)))
        } else {
            None
        };
        *slot.borrow_mut() = Some(loaded.clone());
        Ok(loaded)
    }

    pub async fn coverage(&self) -> Result<Rc<CoverageReport>, LibraryError> {
        if let Some(coverage) = self.0.coverage.borrow().clone() {
            return Ok(coverage);
        }
        let coverage: CoverageReport =
            read_json(&self.0.cache, self.version(), "coverage.json").await?;
        let coverage = Rc::new(coverage);
        *self.0.coverage.borrow_mut() = Some(coverage.clone());
        Ok(coverage)
    }
}

/// Placement in a newer library is authoritative; catalog source is the explicit legacy and
/// unplaced-entry fallback. The schema has no separate stable source field to consult.
fn canonical_book_source<'a>(
    books: Option<&'a BookLookup>,
    id: &EntryId,
    catalog_source: &'a str,
) -> &'a str {
    books
        .and_then(|lookup| lookup.source_for_entry(id))
        .unwrap_or(catalog_source)
}

fn is_book_disabled(disabled: &BTreeSet<String>, canonical_id: &str) -> bool {
    disabled.contains(canonical_id)
}

async fn read_file(cache: &Cache, version: &str, relative: &str) -> Result<Vec<u8>, LibraryError> {
    let path = format!("{version}/{relative}");
    content_cache::read(cache, &path)
        .await
        .map_err(LibraryError::Storage)?
        .ok_or(LibraryError::Missing(path))
}

async fn read_json<T: DeserializeOwned>(
    cache: &Cache,
    version: &str,
    relative: &str,
) -> Result<T, LibraryError> {
    let bytes = read_file(cache, version, relative).await?;
    serde_json::from_slice(&bytes).map_err(|e| LibraryError::Corrupt {
        path: format!("{version}/{relative}"),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::ManifestFile;

    fn manifest(files: &[&str]) -> ContentManifest {
        ContentManifest {
            schema_version: 2,
            content_version: "v1".into(),
            total_bytes: 0,
            files: files
                .iter()
                .map(|p| ManifestFile {
                    path: format!("v1/{p}"),
                    sha256: String::new(),
                    bytes: 0,
                })
                .collect(),
            entry_count: 0,
            datasets: vec![],
        }
    }

    #[test]
    fn a_schema_two_library_has_none_of_the_schema_three_files() {
        let old = manifest(&["catalog.json", "legal.json", "entries/spell-000.json"]);
        for name in [RELATIONS_FILE, SPELL_LISTS_FILE, BOOKS_FILE] {
            assert!(!manifest_has_file(&old, name), "{name}");
        }
        let new = manifest(&["catalog.json", "relations.json", "books.json"]);
        assert!(manifest_has_file(&new, RELATIONS_FILE));
        assert!(manifest_has_file(&new, BOOKS_FILE));
        assert!(!manifest_has_file(&new, SPELL_LISTS_FILE));
        // A file of another version directory does not count.
        let mut other = manifest(&[]);
        other.files.push(ManifestFile {
            path: "v0/relations.json".into(),
            sha256: String::new(),
            bytes: 0,
        });
        assert!(!manifest_has_file(&other, RELATIONS_FILE));
    }

    #[test]
    fn book_selection_defaults_new_books_active_and_allows_zero_active() {
        let disabled = BTreeSet::new(); // missing preference means all active
        assert!(!is_book_disabled(&disabled, "new-book"));
        let disabled = BTreeSet::from(["a".to_owned(), "future".to_owned()]);
        assert!(is_book_disabled(&disabled, "a"));
        assert!(!is_book_disabled(&disabled, "new-book")); // unknown IDs are retained, new books active
        assert!(is_book_disabled(&BTreeSet::from(["a".to_owned()]), "a")); // zero active is valid
    }

    #[test]
    fn placement_wins_over_catalog_source_and_missing_placement_uses_stable_source() {
        let placed_id = EntryId::parse("b/combat").unwrap();
        let unplaced_id = EntryId::parse("b/unplaced").unwrap();
        let lookup = BookLookup::new(BookIndex {
            books: vec![cast_iron_schema::BookTree {
                source: "canonical-book".into(),
                title: "Display title is irrelevant".into(),
                basis: String::new(),
                review: vec![],
                chapters: vec![cast_iron_schema::BookNode {
                    title: "Combat".into(),
                    entry: Some(placed_id.clone()),
                    anchor: None,
                    review: None,
                    children: vec![],
                }],
            }],
        });
        assert_eq!(
            canonical_book_source(Some(&lookup), &placed_id, "attribution-source"),
            "canonical-book"
        );
        assert_eq!(
            canonical_book_source(Some(&lookup), &unplaced_id, "stable-catalog-id"),
            "stable-catalog-id"
        );
        assert_eq!(
            canonical_book_source(None, &placed_id, "legacy-source-id"),
            "legacy-source-id"
        );
    }
}
