//! Read access to the active library. Every read goes through the Cache API under the
//! active version. There is no network fallback: a missing file is an error to show.

use crate::content_cache;
use crate::install;
use crate::lru::Lru;
use crate::store::ActiveLibrary;
use cast_iron_schema::{
    Catalog, ContentManifest, CoverageReport, EntryId, EntryKind, EntryStub, LegalNotices,
    RuleEntry, SourceInfo,
};
use cast_iron_search::{Hit, Query, SearchIndex};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use serde::de::DeserializeOwned;
use std::cell::RefCell;
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
        Ok(Library(SendWrapper::new(Rc::new(Inner {
            active: active.clone(),
            manifest,
            catalog,
            cache,
            shards: RefCell::new(Lru::new(SHARD_CACHE)),
            index: RefCell::new(None),
            legal: RefCell::new(None),
            coverage: RefCell::new(None),
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
