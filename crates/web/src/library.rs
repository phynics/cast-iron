//! Read access to the active library. Every read goes through the Cache API under the
//! active version. There is no network fallback: a missing file is an error to show.

use crate::content_cache;
use crate::graph::{BookLookup, Relations};
use crate::install;
use crate::lru::Lru;
use crate::store::{self, ActiveLibrary};
use cast_iron_schema::{
    ALIASES_FILE, AliasIndex, BOOKS_FILE, BookIndex, Catalog, ContentManifest, CoverageReport,
    EntryId, EntryKind, EntryStub, LegalNotices, RELATIONS_FILE, RelationIndex, RetirementReason,
    RuleEntry, SPELL_LISTS_FILE, SourceInfo, SpellLists,
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

/// What an entry id means in the active library. Resolution is one step: an alias target
/// never resolves again, because the generated map forbids chains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The id is a catalog entry.
    Canonical(EntryId),
    /// A legacy id with a canonical target in this library.
    Aliased { canonical: EntryId, legacy: EntryId },
    /// The map retired this legacy id. The stored row keeps its snapshot.
    Retired {
        legacy: EntryId,
        reason: RetirementReason,
    },
    /// Neither an entry nor a mapped legacy id.
    Unknown(EntryId),
}

struct Inner {
    active: ActiveLibrary,
    manifest: ContentManifest,
    catalog: Catalog,
    cache: Cache,
    shards: RefCell<Lru<String, Rc<Vec<RuleEntry>>>>,
    index: RefCell<Option<Rc<SearchIndex>>>,
    legal: RefCell<Option<Rc<LegalNotices>>>,
    coverage: RefCell<Option<Rc<CoverageReport>>>,
    /// The legacy alias map, loaded eagerly so resolution is synchronous. Empty for a
    /// library built before the canonical cutover.
    aliases: Rc<AliasIndex>,
    /// The manifest hash of `aliases.json`, used as the idempotent migration marker.
    aliases_sha256: String,
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
        let (aliases, aliases_sha256) = if manifest_has_file(&manifest, ALIASES_FILE) {
            let path = format!("{}/{ALIASES_FILE}", manifest.content_version);
            let bytes = read_file(&cache, &active.content_version, ALIASES_FILE).await?;
            let aliases: AliasIndex =
                serde_json::from_slice(&bytes).map_err(|e| LibraryError::Corrupt {
                    path: path.clone(),
                    message: e.to_string(),
                })?;
            aliases
                .validate()
                .map_err(|message| LibraryError::Corrupt { path, message })?;
            let sha256 = manifest
                .files
                .iter()
                .find(|file| file.path == format!("{}/{ALIASES_FILE}", manifest.content_version))
                .map(|file| file.sha256.clone())
                .unwrap_or_default();
            (Rc::new(aliases), sha256)
        } else {
            (Rc::new(AliasIndex::default()), String::new())
        };
        let disabled_books = store::disabled_books()
            .await
            .map_err(|e| LibraryError::BookSelection(e.to_string()))?;
        let library = Library(SendWrapper::new(Rc::new(Inner {
            active: active.clone(),
            manifest,
            catalog,
            cache,
            shards: RefCell::new(Lru::new(SHARD_CACHE)),
            index: RefCell::new(None),
            legal: RefCell::new(None),
            coverage: RefCell::new(None),
            aliases,
            aliases_sha256,
            relations: RefCell::new(None),
            spell_lists: RefCell::new(None),
            books: RefCell::new(Some(books)),
            disabled_books: RefCell::new(disabled_books),
            selection_revision: RwSignal::new(0),
            selection_write_in_flight: RefCell::new(false),
        })));
        // A refused transaction is not fatal: every read still resolves the old id, and the
        // next load retries. This is deliberately weaker than the disabled-books read.
        let _ = library.migrate_stored_ids().await;
        Ok(library)
    }

    /// One-step legacy-to-canonical lookup. A canonical catalog id always wins; a retired id
    /// reports its reason and is left to its snapshot; an unmapped id is unknown.
    pub fn resolve(&self, id: &EntryId) -> Resolution {
        resolve_in(&self.0.catalog, &self.0.aliases, id)
    }

    /// Rewrites stored bookmark and recent ids to canonical targets. Idempotent: the marker
    /// records the alias file hash, so a second load with the same map changes nothing.
    pub async fn migrate_stored_ids(&self) -> Result<bool, store::StoreError> {
        if self.0.aliases.is_empty() || self.0.aliases_sha256.is_empty() {
            return Ok(false);
        }
        let catalog = &self.0.catalog;
        store::migrate_entry_ids(
            &self.0.aliases,
            &self.0.aliases_sha256,
            |id| catalog_has(catalog, id),
            |id| self.stub(id).map(|stub| stub.kind),
        )
        .await
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

fn catalog_has(catalog: &Catalog, id: &EntryId) -> bool {
    catalog
        .entries
        .binary_search_by(|stub| stub.id.cmp(id))
        .is_ok()
        || catalog.entries.iter().any(|stub| stub.id == *id)
}

fn resolve_in(catalog: &Catalog, aliases: &AliasIndex, id: &EntryId) -> Resolution {
    if catalog_has(catalog, id) {
        return Resolution::Canonical(id.clone());
    }
    if let Some(alias) = aliases.alias(id) {
        return Resolution::Aliased {
            canonical: alias.target.clone(),
            legacy: id.clone(),
        };
    }
    if let Some(retired) = aliases.retirement(id) {
        return Resolution::Retired {
            legacy: id.clone(),
            reason: retired.reason,
        };
    }
    Resolution::Unknown(id.clone())
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
        for name in [RELATIONS_FILE, SPELL_LISTS_FILE, BOOKS_FILE, ALIASES_FILE] {
            assert!(!manifest_has_file(&old, name), "{name}");
        }
        let new = manifest(&[
            "catalog.json",
            "relations.json",
            "books.json",
            "aliases.json",
        ]);
        assert!(manifest_has_file(&new, RELATIONS_FILE));
        assert!(manifest_has_file(&new, BOOKS_FILE));
        assert!(manifest_has_file(&new, ALIASES_FILE));
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

    fn aliases() -> AliasIndex {
        serde_json::from_str(
            r#"{
                "format": 1,
                "legacy": {"count": 4, "sha256": ""},
                "entries": [
                    {"old": "legacy/a", "target": "ref/rule/a", "basis": "direct"},
                    {"old": "legacy/b", "target": "ref/rule/b", "basis": "derived"},
                    {"old": "legacy/b-2", "target": "ref/rule/b", "basis": "shared"}
                ],
                "retired": [
                    {"old": "legacy/retired", "reason": "no_source", "note": "no counterpart"}
                ]
            }"#,
        )
        .unwrap()
    }

    fn catalog(ids: &[&str]) -> Catalog {
        let mut entries: Vec<EntryStub> = ids
            .iter()
            .map(|id| EntryStub {
                id: EntryId::parse(*id).unwrap(),
                kind: EntryKind::Rule,
                title: id.to_string(),
                source: "s".into(),
                parent: None,
                shard: "entries/rule-000.json".into(),
            })
            .collect();
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Catalog {
            sources: vec![],
            entries,
        }
    }

    fn id(value: &str) -> EntryId {
        EntryId::parse(value).unwrap()
    }

    #[test]
    fn alias_file_rejects_a_format_it_does_not_read_and_an_unsorted_map() {
        assert!(aliases().validate().is_ok());
        let mut wrong_format = aliases();
        wrong_format.format = 2;
        assert!(wrong_format.validate().is_err());
        let mut unsorted = aliases();
        unsorted.entries.swap(0, 1);
        assert!(unsorted.validate().is_err());
    }

    #[test]
    fn resolve_returns_canonical_aliased_retired_and_unknown() {
        let aliases = aliases();
        let catalog = catalog(&["ref/rule/a", "ref/rule/b"]);
        assert_eq!(
            resolve_in(&catalog, &aliases, &id("ref/rule/a")),
            Resolution::Canonical(id("ref/rule/a"))
        );
        assert_eq!(
            resolve_in(&catalog, &aliases, &id("legacy/a")),
            Resolution::Aliased {
                canonical: id("ref/rule/a"),
                legacy: id("legacy/a"),
            }
        );
        assert_eq!(
            resolve_in(&catalog, &aliases, &id("legacy/retired")),
            Resolution::Retired {
                legacy: id("legacy/retired"),
                reason: RetirementReason::NoSource,
            }
        );
        assert_eq!(
            resolve_in(&catalog, &aliases, &id("legacy/missing")),
            Resolution::Unknown(id("legacy/missing"))
        );
    }

    #[test]
    fn resolve_is_one_step_even_if_a_map_contains_a_chain() {
        let aliases: AliasIndex = serde_json::from_str(
            r#"{
                "format": 1,
                "legacy": {"count": 2, "sha256": ""},
                "entries": [
                    {"old": "legacy/a", "target": "ref/rule/b", "basis": "direct"},
                    {"old": "legacy/b", "target": "ref/rule/c", "basis": "direct"}
                ],
                "retired": []
            }"#,
        )
        .unwrap();
        let catalog = catalog(&["ref/rule/b", "ref/rule/c"]);
        // `legacy/a` stops at `ref/rule/b`; it never follows the second row to `ref/rule/c`.
        assert_eq!(
            resolve_in(&catalog, &aliases, &id("legacy/a")),
            Resolution::Aliased {
                canonical: id("ref/rule/b"),
                legacy: id("legacy/a"),
            }
        );
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
