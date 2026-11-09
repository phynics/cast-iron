//! Read access to the active library. Every read goes through the Cache API under the
//! active version. There is no network fallback: a missing file is an error to show.

use crate::book_presets::{self, Change};
use crate::content_cache;
use crate::graph::{BookLookup, Relations};
use crate::install;
use crate::lru::Lru;
use crate::revisions::EffectiveRevisions;
use crate::store::{self, ActiveLibrary};
use crate::views::book_tier::source_ranks;
use cast_iron_schema::{
    ALIASES_FILE, AliasIndex, BACKLINKS_FILE, BOOKS_FILE, BacklinkIndex, BacklinkSource,
    BacklinkTarget, BookIndex, Catalog, ContentManifest, CoverageReport, EntryId, EntryKind,
    EntryStub, LegalNotices, OPTION_GROUPS_FILE, OptionGroupIndex, RELATIONS_FILE, REPLACES_FILE,
    RelationIndex, ReplacesIndex, RetirementReason, RuleEntry, SPELL_LISTS_FILE, SourceInfo,
    SpellLists, backlink_shard, backlink_shard_path,
};
use cast_iron_search::{Hit, Query, SearchIndex};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use serde::de::DeserializeOwned;
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;
use web_sys::Cache;

/// Parsed shards to keep in memory.
const SHARD_CACHE: usize = 8;
/// Parsed reverse-link shards to keep in memory.
const BACKLINK_SHARD_CACHE: usize = 4;

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
    /// Schema-4 class-option groups. The outer `Option` is "not loaded yet"; the inner one is
    /// "this library has no such file".
    option_groups: RefCell<Option<Option<Rc<OptionGroupIndex>>>>,
    /// Schema-4 typed replacements for archetypes and class options.
    replaces: RefCell<Option<Option<Rc<ReplacesIndex>>>>,
    /// The optional reverse body-link descriptor. The outer `Option` is "not loaded yet";
    /// the inner one is "this library has no such file".
    backlinks: RefCell<Option<Option<Rc<BacklinkIndex>>>>,
    backlink_shards: RefCell<Lru<u32, Rc<Vec<BacklinkTarget>>>>,
    disabled_books: RefCell<BTreeSet<String>>,
    /// Catalog entries per canonical source, counted once on first use. The catalog and the book
    /// placements do not change for a loaded library.
    source_counts: OnceCell<BTreeMap<String, usize>>,
    /// The eligible id set of the current selection revision, shared by every `discovery()` call
    /// until the reader changes the active books or a new library is installed.
    discovery: crate::discovery::DiscoveryCache,
    /// The effective revision state of the current selection revision. A new revision rebuilds
    /// it once; lists, chains, pages and count predicates all read the same value.
    revisions: RefCell<Option<(u64, Option<Rc<EffectiveRevisions>>)>>,
    selection_revision: RwSignal<u64>,
    selection_write_in_flight: RefCell<bool>,
    /// Set when IndexedDB refused the id rewrite. The library still reads correctly through
    /// `resolve`; Settings tells the reader that the conversion waits for storage.
    migration_refused: RwSignal<bool>,
}

/// The stored disabled set, after the first-run book default (`book_presets::defaults_action`)
/// and the third-party arrival rule.
/// A new install gets the `Paizo Rulebooks` choice written once; every other profile reads what
/// it stored, or an empty set (every book active) when it never stored anything. A third-party
/// book that the profile has not seen before (#189) starts disabled, and the profile's set of
/// seen third-party books is recorded so a later update can tell an arrival from a choice.
async fn settle_first_run_defaults(
    sources: &[SourceInfo],
    migrate_book_sources: bool,
) -> Result<BTreeSet<String>, LibraryError> {
    let selection = |e: store::StoreError| LibraryError::BookSelection(e.to_string());
    let marker = store::book_defaults_marker().await.map_err(selection)?;
    let mut stored = store::stored_disabled_books().await.map_err(selection)?;
    if migrate_book_sources
        && !store::book_source_migration_done()
            .await
            .map_err(selection)?
    {
        stored = store::settle_book_source_migration(stored.as_ref())
            .await
            .map_err(selection)?;
    }
    let known_third_party = store::known_third_party().await.map_err(selection)?;
    let books = book_presets::books_of(sources);
    let settled = book_presets::settle_books(
        marker.as_deref(),
        stored.as_ref(),
        known_third_party.as_ref(),
        &books,
    );
    // This runs inside `Library::load`, before the `Library` handle exists, so no selection
    // write from the UI can overlap it. `apply_active_plan`'s in-flight flag guards writes
    // that the reader starts; there is nothing to guard here. Two tabs that load at once can
    // still race on `disabled_books`, exactly as the first-run seed already could: both
    // compute the same set for the same catalog, so the last write is the same choice.
    if settled.settle_marker {
        store::settle_book_defaults(Some(&settled.disabled))
            .await
            .map_err(selection)?;
    } else {
        let changed = stored
            .as_ref()
            .map_or(!settled.disabled.is_empty(), |stored| {
                stored != &settled.disabled
            });
        if changed {
            store::set_disabled_books(&settled.disabled)
                .await
                .map_err(selection)?;
        }
    }
    if known_third_party.as_ref() != Some(&settled.known_third_party) {
        store::set_known_third_party(&settled.known_third_party)
            .await
            .map_err(selection)?;
    }
    if settled.notice {
        store::queue_third_party_notice().await.map_err(selection)?;
    }
    Ok(settled.disabled)
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
        catalog
            .validate()
            .map_err(|message| LibraryError::Corrupt {
                path: format!("{}/catalog.json", active.content_version),
                message,
            })?;
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
        let migrate_book_sources = books
            .as_ref()
            .is_some_and(|books| !crate::book_source::has_retired_book(books));
        let disabled_books =
            settle_first_run_defaults(&catalog.sources, migrate_book_sources).await?;
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
            option_groups: RefCell::new(None),
            replaces: RefCell::new(None),
            backlinks: RefCell::new(None),
            backlink_shards: RefCell::new(Lru::new(BACKLINK_SHARD_CACHE)),
            disabled_books: RefCell::new(disabled_books),
            source_counts: OnceCell::new(),
            discovery: crate::discovery::DiscoveryCache::default(),
            revisions: RefCell::new(None),
            selection_revision: RwSignal::new(0),
            selection_write_in_flight: RefCell::new(false),
            migration_refused: RwSignal::new(false),
        })));
        // A refused transaction is not fatal: every read still resolves the old id, and the
        // next load retries. Settings shows the non-blocking conversion notice meanwhile.
        let migration = library.migrate_stored_ids().await;
        if conversion_is_pending(&migration) {
            if let Err(error) = &migration {
                leptos::logging::warn!("the saved-item id conversion was refused: {error}");
            }
            library.0.migration_refused.set(true);
        }
        // A kind can change between library versions (class options were `rule` before schema
        // 4). The stored snapshot is only a fallback for an entry the library no longer holds,
        // but keep it true while the entry is here.
        let kinds = library.clone();
        if let Err(error) =
            store::refresh_bookmark_kinds(|id| kinds.stub(id).map(|stub| stub.kind)).await
        {
            leptos::logging::warn!("the bookmark kind snapshots were not refreshed: {error}");
        }
        Ok(library)
    }

    /// One-step legacy-to-canonical lookup. A canonical catalog id always wins; a retired id
    /// reports its reason and is left to its snapshot; an unmapped id is unknown.
    pub fn resolve(&self, id: &EntryId) -> Resolution {
        resolve_in(&self.0.catalog, &self.0.aliases, id)
    }

    /// True when a stored row under `stored` is the saved item for `target`: either the row
    /// already uses the id, or the alias map sends it to `target`. This is the star contract,
    /// so a legacy row that a refused migration left behind still reads as pressed.
    pub fn resolves_to(&self, stored: &EntryId, target: &EntryId) -> bool {
        resolves_to_in(&self.0.catalog, &self.0.aliases, stored, target)
    }

    /// The id that a stored row is shown and saved under: the alias target when the map has
    /// one, otherwise the stored id itself. Retired and unknown rows keep their own id so
    /// that their snapshot stays visible.
    pub fn canonical_target(&self, stored: &EntryId) -> EntryId {
        canonical_target_in(&self.0.catalog, &self.0.aliases, stored)
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

    /// True when the last id rewrite was refused, so saved items still hold legacy ids.
    pub fn migration_refused(&self) -> bool {
        self.0.migration_refused.get_untracked()
    }

    pub fn book_active(&self, canonical_id: &str) -> bool {
        !self.0.disabled_books.borrow().contains(canonical_id)
    }

    /// True when the reader has turned off at least one book. Search uses it to skip the
    /// hidden-match count when every book is active.
    pub fn has_inactive_books(&self) -> bool {
        !self.0.disabled_books.borrow().is_empty()
    }

    pub fn is_entry_active(&self, id: &EntryId) -> bool {
        let Some(stub) = self.stub(id) else {
            return false;
        };
        self.is_stub_active(stub)
    }

    /// The active-book test for a catalog stub, without a second id lookup. Discovery asks this
    /// for every entry in the catalog.
    pub(crate) fn is_stub_active(&self, stub: &EntryStub) -> bool {
        let disabled = self.0.disabled_books.borrow();
        self.with_books(|books| {
            let source = canonical_book_source(books, &stub.id, &stub.source);
            !is_book_disabled(&disabled, source)
        })
    }

    /// Runs `f` with the book lookup, or `None` for a library without `books.json`. The closure
    /// form keeps the `RefCell` borrow alive for the call without cloning the lookup.
    pub(crate) fn with_books<R>(&self, f: impl FnOnce(Option<&BookLookup>) -> R) -> R {
        let books = self.0.books.borrow();
        f(books.as_ref().and_then(Option::as_deref))
    }

    /// True when the catalog stub belongs to `source` under the canonical-source rule. It
    /// allocates nothing, unlike comparing [`Self::canonical_source_for_entry`].
    pub(crate) fn stub_is_in_source(&self, stub: &EntryStub, source: &str) -> bool {
        self.with_books(|books| canonical_book_source(books, &stub.id, &stub.source) == source)
    }

    /// How many catalog entries each canonical source holds, whether or not the book is active.
    /// A source with no entries has no row. The first call makes one pass over the catalog; later
    /// calls reuse the result.
    pub fn source_entry_counts(&self) -> &BTreeMap<String, usize> {
        self.0.source_counts.get_or_init(|| {
            self.with_books(|books| count_by_source(&self.0.catalog.entries, books))
        })
    }

    pub fn canonical_source_for_entry(&self, id: &EntryId) -> Option<String> {
        let stub = self.stub(id)?;
        Some(
            self.with_books(|books| {
                canonical_book_source(books, &stub.id, &stub.source).to_owned()
            }),
        )
    }

    pub async fn set_book_active(
        &self,
        canonical_id: &str,
        active: bool,
    ) -> Result<(), LibraryError> {
        let ids = [canonical_id.to_owned()];
        if active {
            self.update_active_books(&ids, &[]).await
        } else {
            self.update_active_books(&[], &ids).await
        }
    }

    /// Activates and deactivates many books with one stored write and one revision bump, so a
    /// preset refreshes discovery once. It changes only the disabled set: downloads and
    /// stars stay, and ids that are in neither list keep their state.
    pub async fn update_active_books(
        &self,
        activate: &[String],
        deactivate: &[String],
    ) -> Result<(), LibraryError> {
        let (activate, deactivate) = (activate.to_vec(), deactivate.to_vec());
        self.apply_active_plan(move |_| Change {
            activate,
            deactivate,
        })
        .await
        .map(|_| ())
    }

    /// Like [`Self::update_active_books`], but the caller states what it wants and the change is
    /// worked out inside the write path, after any earlier write has finished. `plan` receives
    /// the state at that moment (`true` for an active book) and returns the change to make. A
    /// preset must use this: a plan made from the screen's state before a second tap's earlier
    /// write landed would keep a stray book active. Returns the change that was applied, which is
    /// what Undo reverses.
    pub async fn apply_active_plan<F>(&self, plan: F) -> Result<Change, LibraryError>
    where
        F: FnOnce(&dyn Fn(&str) -> bool) -> Change,
    {
        // A write that arrives while another is saving waits its turn, so no control has to
        // disable itself (which would drop keyboard focus) and two quick toggles both land. The
        // new set is computed only after the earlier write has finished.
        loop {
            {
                let mut busy = self.0.selection_write_in_flight.borrow_mut();
                if !*busy {
                    *busy = true;
                    break;
                }
            }
            crate::js::sleep(5).await;
        }
        struct Reset<'a>(&'a RefCell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                *self.0.borrow_mut() = false;
            }
        }
        let _reset = Reset(&self.0.selection_write_in_flight);
        let (change, next) = commit_plan(&self.0.disabled_books.borrow(), plan);
        if next == *self.0.disabled_books.borrow() {
            return Ok(change);
        }
        store::set_disabled_books(&next)
            .await
            .map_err(|e| LibraryError::Storage(e.to_string()))?;
        *self.0.disabled_books.borrow_mut() = next;
        self.0
            .selection_revision
            .update(|revision| *revision = revision.wrapping_add(1));
        Ok(change)
    }

    /// The eligibility snapshot for the current active-book selection. Every call at the same
    /// `selection_revision` shares one eligible id set, so a page that asks per entry kind does
    /// not rebuild the 31k-id set each time. A new revision, or a new library, rebuilds it.
    pub fn discovery(&self) -> crate::discovery::Discovery {
        let revision = self.0.selection_revision.get_untracked();
        let eligible = self.0.discovery.ids(revision, || {
            self.with_books(|books| {
                crate::discovery::eligible_ids(
                    &self.0.catalog.entries,
                    books,
                    &self.0.disabled_books.borrow(),
                )
            })
        });
        crate::discovery::Discovery::new(self.clone(), eligible)
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
        let mut index = SearchIndex::from_bytes(&bytes).map_err(|e| LibraryError::Corrupt {
            path: format!("{}/search.bin", self.version()),
            message: format!("{e:?}"),
        })?;
        // Equal relevance orders by product line. The ranks come from the catalog, so a library
        // installed before this ranking, or one without categories, still loads and searches.
        index.set_source_ranks(source_ranks(&self.0.catalog.sources));
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

    /// The effective revision state for the current active-book selection. `None` when the
    /// library has no relation index. The first call at a selection revision reads the relation
    /// graph and the two merged revisions' printed links; every later call reuses the same value.
    pub async fn revisions(&self) -> Result<Option<Rc<EffectiveRevisions>>, LibraryError> {
        let revision = self.0.selection_revision.get_untracked();
        if let Some((cached, state)) = self.0.revisions.borrow().as_ref()
            && *cached == revision
        {
            return Ok(state.clone());
        }
        let Some(relations) = self.relations().await? else {
            *self.0.revisions.borrow_mut() = Some((revision, None));
            return Ok(None);
        };
        // A merged revision names its destination in the printed body. Read only the two links
        // whose own book is active; a merge target lets the graph point the base's dependents
        // at the feat that absorbed it.
        let mut merge_targets = BTreeMap::new();
        for (revision_id, label, _) in relations.revision_edges() {
            if label != Some("merged") || !self.is_entry_active(revision_id) {
                continue;
            }
            if let Ok(entry) = self.entry(revision_id).await
                && let Some(link) = entry.links.first()
            {
                merge_targets.insert(revision_id.clone(), link.target.clone());
            }
        }
        let discovery = self.discovery();
        let state = Rc::new(EffectiveRevisions::build(
            &relations,
            |id| self.is_entry_active(id),
            |id| self.stub(id).map(|stub| stub.source.clone()),
            |id| merge_targets.get(id).cloned(),
            |id| discovery.is_eligible(id.as_str()),
        ));
        *self.0.revisions.borrow_mut() = Some((revision, Some(state.clone())));
        Ok(Some(state))
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

    /// The declared class-option groups and the entries each holds. `Ok(None)` for a library
    /// without `option-groups.json`, which is every schema-3 library.
    pub async fn option_groups(&self) -> Result<Option<Rc<OptionGroupIndex>>, LibraryError> {
        self.lazy(
            &self.0.option_groups,
            OPTION_GROUPS_FILE,
            |index: OptionGroupIndex| index,
        )
        .await
    }

    /// The printed items that archetypes and class options replace. `Ok(None)` for a library
    /// without `replaces.json`.
    pub async fn replaces(&self) -> Result<Option<Rc<ReplacesIndex>>, LibraryError> {
        self.lazy(&self.0.replaces, REPLACES_FILE, |index: ReplacesIndex| {
            index
        })
        .await
    }

    /// The entries whose bodies link to `target`, or `Ok(None)` when the library has no
    /// reverse index or no row for the target. Only the target's shard is read.
    pub async fn backlinks(
        &self,
        target: &EntryId,
    ) -> Result<Option<Vec<BacklinkSource>>, LibraryError> {
        let Some(index) = self.backlink_index().await? else {
            return Ok(None);
        };
        let shard = backlink_shard(target.as_str(), index.shards);
        let rows = self.backlink_shard(shard).await?;
        Ok(rows
            .iter()
            .find(|row| &row.0 == target)
            .map(|row| row.1.clone()))
    }

    async fn backlink_index(&self) -> Result<Option<Rc<BacklinkIndex>>, LibraryError> {
        if let Some(loaded) = self.0.backlinks.borrow().clone() {
            return Ok(loaded);
        }
        let loaded = if manifest_has_file(&self.0.manifest, BACKLINKS_FILE) {
            let path = format!("{}/{BACKLINKS_FILE}", self.version());
            let bytes = read_file(&self.0.cache, self.version(), BACKLINKS_FILE).await?;
            let index = parse_backlink_index(&bytes)
                .map_err(|message| LibraryError::Corrupt { path, message })?;
            Some(Rc::new(index))
        } else {
            None
        };
        *self.0.backlinks.borrow_mut() = Some(loaded.clone());
        Ok(loaded)
    }

    async fn backlink_shard(&self, shard: u32) -> Result<Rc<Vec<BacklinkTarget>>, LibraryError> {
        if let Some(hit) = self.0.backlink_shards.borrow_mut().get(&shard) {
            return Ok(hit);
        }
        let path = backlink_shard_path(shard);
        let parsed: Vec<BacklinkTarget> = read_json(&self.0.cache, self.version(), &path).await?;
        let parsed = Rc::new(parsed);
        self.0
            .backlink_shards
            .borrow_mut()
            .insert(shard, parsed.clone());
        Ok(parsed)
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

/// Whether the load should show the Settings conversion notice. Only a refused transaction
/// counts; `Ok(false)` means the alias hash already matched and nothing waits for storage.
fn conversion_is_pending(result: &Result<bool, store::StoreError>) -> bool {
    result.is_err()
}

fn catalog_has(catalog: &Catalog, id: &EntryId) -> bool {
    debug_assert!(
        catalog
            .entries
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id),
        "catalog entries are sorted by unique id"
    );
    catalog
        .entries
        .binary_search_by(|stub| stub.id.cmp(id))
        .is_ok()
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

fn resolves_to_in(
    catalog: &Catalog,
    aliases: &AliasIndex,
    stored: &EntryId,
    target: &EntryId,
) -> bool {
    stored == target
        || matches!(
            resolve_in(catalog, aliases, stored),
            Resolution::Aliased { canonical, .. } if canonical == *target
        )
}

fn canonical_target_in(catalog: &Catalog, aliases: &AliasIndex, stored: &EntryId) -> EntryId {
    match resolve_in(catalog, aliases, stored) {
        Resolution::Aliased { canonical, .. } => canonical,
        _ => stored.clone(),
    }
}

/// Placement in a newer library is authoritative; catalog source is the explicit legacy and
/// unplaced-entry fallback. The schema has no separate stable source field to consult.
pub(crate) fn canonical_book_source<'a>(
    books: Option<&'a BookLookup>,
    id: &EntryId,
    catalog_source: &'a str,
) -> &'a str {
    books
        .and_then(|lookup| lookup.source_for_entry(id))
        .unwrap_or(catalog_source)
}

/// One pass over `entries`: entries per canonical source. A source's key is allocated once,
/// when its first entry arrives. Both the whole-catalog count and the eligible-entry count of
/// a discovery snapshot use this.
pub(crate) fn count_by_source<'a, 'b>(
    entries: impl IntoIterator<Item = &'a EntryStub>,
    books: Option<&'b BookLookup>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::<String, usize>::new();
    for stub in entries {
        let source = canonical_book_source(books, &stub.id, &stub.source);
        match counts.get_mut(source) {
            Some(count) => *count += 1,
            None => {
                counts.insert(source.to_owned(), 1);
            }
        }
    }
    counts
}

fn is_book_disabled(disabled: &BTreeSet<String>, canonical_id: &str) -> bool {
    disabled.contains(canonical_id)
}

/// The disabled set after one update. Ids in neither list, including ids that this catalog
/// does not know, keep their state. An id in both lists ends active.
fn next_disabled(
    current: &BTreeSet<String>,
    activate: &[String],
    deactivate: &[String],
) -> BTreeSet<String> {
    let mut next = current.clone();
    next.extend(deactivate.iter().cloned());
    for id in activate {
        next.remove(id);
    }
    next
}

/// Works out one write from the stored set as it is now: what `plan` asks for, and the disabled
/// set that results. The write path calls this after it owns the write slot, so a plan never
/// starts from a set that an earlier write is about to change.
fn commit_plan(
    current: &BTreeSet<String>,
    plan: impl FnOnce(&dyn Fn(&str) -> bool) -> Change,
) -> (Change, BTreeSet<String>) {
    let change = plan(&|id| !is_book_disabled(current, id));
    let next = next_disabled(current, &change.activate, &change.deactivate);
    (change, next)
}

async fn read_file(cache: &Cache, version: &str, relative: &str) -> Result<Vec<u8>, LibraryError> {
    let path = format!("{version}/{relative}");
    content_cache::read(cache, &path)
        .await
        .map_err(LibraryError::Storage)?
        .ok_or(LibraryError::Missing(path))
}

/// Reads the reverse-index descriptor: valid JSON that names the format this client reads and
/// an addressable shard count. A descriptor this client cannot read is an error, and the entry
/// screen answers an error by showing no "Referenced by" panel, so the entry stays readable.
fn parse_backlink_index(bytes: &[u8]) -> Result<BacklinkIndex, String> {
    let index: BacklinkIndex = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    index.validate()?;
    Ok(index)
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

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn bulk_update_changes_only_the_listed_books_and_keeps_unknown_ids() {
        let current: BTreeSet<String> = ids(&["kept-unknown", "a"]).into_iter().collect();
        let next = next_disabled(&current, &ids(&["a"]), &ids(&["b", "c"]));
        assert_eq!(
            next.into_iter().collect::<Vec<_>>(),
            ids(&["b", "c", "kept-unknown"])
        );
        // Repeating the same update changes nothing, so the caller skips the write.
        let again = next_disabled(&current, &[], &[]);
        assert_eq!(again, current);
    }

    #[test]
    fn a_book_in_both_lists_ends_active() {
        let next = next_disabled(&BTreeSet::new(), &ids(&["a"]), &ids(&["a", "b"]));
        assert_eq!(next.into_iter().collect::<Vec<_>>(), ids(&["b"]));
    }

    /// The books of a small shelf, in the order the presets see them.
    fn shelf() -> Vec<book_presets::PresetBook<'static>> {
        use crate::views::book_tier::Tier;
        let book = |id, tier| book_presets::PresetBook {
            id,
            tier,
            third_party: false,
        };
        vec![
            book("core", Tier::Rulebooks),
            book("apg", Tier::Rulebooks),
            book("companion", Tier::PlayerCompanions),
            book("setting", Tier::CampaignSetting),
            book("path", Tier::Adventures),
        ]
    }

    fn disabled(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn a_second_preset_is_planned_from_the_set_the_first_left() {
        let books = shelf();
        let start = BTreeSet::new();
        // Two taps in a row: Paizo Rulebooks, then Paizo Rulebooks & More.
        let (_, after_first) = commit_plan(&start, |active| {
            book_presets::plan(book_presets::Preset::PaizoRulebooks, &books, active)
        });
        assert_eq!(after_first, disabled(&["companion", "path", "setting"]));
        let (change, after_second) = commit_plan(&after_first, |active| {
            book_presets::plan(book_presets::Preset::PaizoRulebooksAndMore, &books, active)
        });
        // Exactly the second preset's books are active: nothing stray is left on or off.
        assert_eq!(after_second, disabled(&["path"]));
        assert_eq!(change.activate, ["companion", "setting"]);
        assert_eq!(change.deactivate, Vec::<String>::new());
    }

    #[test]
    fn a_plan_made_from_a_stale_set_leaves_a_stray_book() {
        // This is the race that `apply_active_plan` closes: the second plan is made while the
        // first write has not landed, so it sees every book active and activates nothing.
        let books = shelf();
        let start = BTreeSet::new();
        let stale = book_presets::plan(book_presets::Preset::PaizoRulebooksAndMore, &books, |_| {
            true
        });
        let (_, after_first) = commit_plan(&start, |active| {
            book_presets::plan(book_presets::Preset::PaizoRulebooks, &books, active)
        });
        let stale_result = next_disabled(&after_first, &stale.activate, &stale.deactivate);
        assert_ne!(stale_result, disabled(&["path"]));
        assert!(stale_result.contains("companion"));
    }

    #[test]
    fn a_plan_that_changes_nothing_writes_nothing() {
        let books = shelf();
        let current = disabled(&["companion", "path", "setting"]);
        let (change, next) = commit_plan(&current, |active| {
            book_presets::plan(book_presets::Preset::PaizoRulebooks, &books, active)
        });
        assert!(change.is_empty());
        assert_eq!(next, current);
    }

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
            corpus_tag: None,
        }
    }

    #[test]
    fn a_schema_two_library_has_none_of_the_schema_three_files() {
        let old = manifest(&["catalog.json", "legal.json", "entries/spell-000.json"]);
        for name in [
            RELATIONS_FILE,
            SPELL_LISTS_FILE,
            BOOKS_FILE,
            ALIASES_FILE,
            BACKLINKS_FILE,
        ] {
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
    fn a_corrupt_backlink_descriptor_is_rejected_so_the_panel_stays_hidden() {
        let good = br#"{"format":1,"shards":16,"links":3,"targets":2}"#;
        assert_eq!(parse_backlink_index(good).unwrap().shards, 16);

        // Each of these makes the loader return an error, which the entry screen turns into
        // an absent panel rather than a broken page. Unparsable JSON has no path in
        // `BacklinkIndex::validate`, so the loader's parse step owns that case.
        for (case, bytes) in [
            (
                "wrong format",
                br#"{"format":2,"shards":16,"links":0,"targets":0}"#.as_slice(),
            ),
            (
                "zero shards",
                br#"{"format":1,"shards":0,"links":0,"targets":0}"#.as_slice(),
            ),
            (
                "too many shards",
                br#"{"format":1,"shards":101,"links":0,"targets":0}"#.as_slice(),
            ),
            ("unparsable json", b"{not json".as_slice()),
        ] {
            assert!(parse_backlink_index(bytes).is_err(), "{case}");
        }
    }

    #[test]
    fn only_a_refused_migration_asks_the_reader_to_wait() {
        assert!(!conversion_is_pending(&Ok(false)));
        assert!(!conversion_is_pending(&Ok(true)));
        assert!(conversion_is_pending(&Err(store::StoreError::Backend(
            "quota".into()
        ))));
    }

    #[test]
    fn catalog_lookup_uses_the_sorted_order_without_a_scan() {
        let catalog = catalog(&["ref/rule/a", "ref/rule/b", "ref/rule/c"]);
        assert!(catalog_has(&catalog, &id("ref/rule/a")));
        assert!(catalog_has(&catalog, &id("ref/rule/c")));
        assert!(!catalog_has(&catalog, &id("ref/rule/between")));
        assert!(!catalog_has(&catalog, &id("0")));
    }

    #[test]
    fn every_alias_row_of_a_sorted_catalog_resolves_to_a_catalog_target() {
        // The cutover fixture: two shared rows and one derived row name a target each, and
        // the direct row is the 1:1 control. Sortedness is what makes the target lookups
        // agree with `catalog_has`, so a many:1 row is never dropped from the merge plan.
        let catalog = catalog(&["ref/race/trait", "ref/rule/direct", "ref/rule/shared"]);
        assert!(catalog.validate().is_ok());
        let aliases: AliasIndex = serde_json::from_str(
            r#"{
                "format": 1,
                "legacy": {"count": 5, "sha256": ""},
                "entries": [
                    {"old": "legacy/direct", "target": "ref/rule/direct", "basis": "direct"},
                    {"old": "legacy/shared-a", "target": "ref/rule/shared", "basis": "shared"},
                    {"old": "legacy/shared-b", "target": "ref/rule/shared", "basis": "shared"},
                    {"old": "legacy/trait-a", "target": "ref/race/trait", "basis": "derived"},
                    {"old": "legacy/trait-b", "target": "ref/race/trait", "basis": "shared"}
                ],
                "retired": []
            }"#,
        )
        .unwrap();
        assert!(aliases.validate().is_ok());
        for (old, target) in [
            ("legacy/direct", "ref/rule/direct"),
            ("legacy/shared-a", "ref/rule/shared"),
            ("legacy/shared-b", "ref/rule/shared"),
            ("legacy/trait-a", "ref/race/trait"),
            ("legacy/trait-b", "ref/race/trait"),
        ] {
            assert!(catalog_has(&catalog, &id(target)), "{target}");
            assert_eq!(
                resolve_in(&catalog, &aliases, &id(old)),
                Resolution::Aliased {
                    canonical: id(target),
                    legacy: id(old),
                },
                "{old}"
            );
        }
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
    fn a_stored_legacy_row_resolves_to_its_canonical_target() {
        let aliases = aliases();
        let catalog = catalog(&["ref/rule/a", "ref/rule/b"]);
        assert!(resolves_to_in(
            &catalog,
            &aliases,
            &id("legacy/a"),
            &id("ref/rule/a")
        ));
        // The canonical row itself counts, which is what makes the star pressed.
        assert!(resolves_to_in(
            &catalog,
            &aliases,
            &id("ref/rule/a"),
            &id("ref/rule/a")
        ));
        // Another target, a retired id, and an unknown id do not.
        assert!(!resolves_to_in(
            &catalog,
            &aliases,
            &id("legacy/a"),
            &id("ref/rule/b")
        ));
        assert!(!resolves_to_in(
            &catalog,
            &aliases,
            &id("legacy/retired"),
            &id("ref/rule/a")
        ));
        assert!(resolves_to_in(
            &catalog,
            &aliases,
            &id("legacy/retired"),
            &id("legacy/retired")
        ));
        assert!(!resolves_to_in(
            &catalog,
            &aliases,
            &id("legacy/missing"),
            &id("ref/rule/a")
        ));
    }

    #[test]
    fn canonical_target_keeps_retired_and_unknown_rows_under_their_own_id() {
        let aliases = aliases();
        let catalog = catalog(&["ref/rule/a", "ref/rule/b"]);
        assert_eq!(
            canonical_target_in(&catalog, &aliases, &id("legacy/a")),
            id("ref/rule/a")
        );
        assert_eq!(
            canonical_target_in(&catalog, &aliases, &id("legacy/retired")),
            id("legacy/retired")
        );
        assert_eq!(
            canonical_target_in(&catalog, &aliases, &id("legacy/missing")),
            id("legacy/missing")
        );
        assert_eq!(
            canonical_target_in(&catalog, &aliases, &id("ref/rule/b")),
            id("ref/rule/b")
        );
    }

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

    fn placed(entry: &EntryId) -> cast_iron_schema::BookNode {
        cast_iron_schema::BookNode {
            title: "Chapter".into(),
            entry: Some(entry.clone()),
            anchor: None,
            review: None,
            children: vec![],
        }
    }

    /// The per-source count the Books page used to compute: one catalog scan per source.
    fn naive_count(entries: &[EntryStub], books: Option<&BookLookup>, source: &str) -> usize {
        entries
            .iter()
            .filter(|stub| {
                Some(canonical_book_source(books, &stub.id, &stub.source)) == Some(source)
            })
            .count()
    }

    #[test]
    fn one_pass_source_counts_match_a_scan_per_source() {
        let entries = vec![
            stub("a/one", "legacy-a"),
            stub("a/two", "legacy-a"),
            stub("b/one", "legacy-b"),
            stub("b/two", "legacy-b"),
            stub("c/one", "legacy-c"),
        ];
        // `a/one` moves to book-b; `b/two` is placed in its own book; `a/two`, `b/one` and
        // `c/one` keep their catalog source.
        let lookup = BookLookup::new(BookIndex {
            books: vec![
                cast_iron_schema::BookTree {
                    source: "book-b".into(),
                    title: "B".into(),
                    basis: String::new(),
                    review: vec![],
                    chapters: vec![placed(&entries[0].id), placed(&entries[3].id)],
                },
                cast_iron_schema::BookTree {
                    source: "book-empty".into(),
                    title: "Empty".into(),
                    basis: String::new(),
                    review: vec![],
                    chapters: vec![],
                },
            ],
        });
        for books in [Some(&lookup), None] {
            let counts = count_by_source(&entries, books);
            let sources = [
                "legacy-a",
                "legacy-b",
                "legacy-c",
                "book-b",
                "book-empty",
                "nope",
            ];
            for source in sources {
                assert_eq!(
                    counts.get(source).copied().unwrap_or(0),
                    naive_count(&entries, books, source),
                    "{source}"
                );
            }
            assert_eq!(counts.values().sum::<usize>(), entries.len());
            // A source with no entries has no key.
            assert!(!counts.contains_key("book-empty"));
        }
        let counts = count_by_source(&entries, Some(&lookup));
        assert_eq!(counts.get("book-b"), Some(&2));
        assert_eq!(counts.get("legacy-a"), Some(&1));
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
