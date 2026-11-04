//! IndexedDB: the user's own state. Library files live in the Cache API, not here.
//!
//! Database `cast-iron`, version 1:
//! - `meta`: out-of-line string keys. `active_library`, `pending_install`, `theme`.
//! - `bookmarks`: key path `entry_id`.
//! - `collections`: key path `id`.
//!
//! To change the shape, raise [`DB_VERSION`] and add an `if old_version < N` step in
//! [`stores_to_create`] and [`upgrade`]. Never edit an earlier step.

use crate::js;
use cast_iron_schema::{AliasIndex, EntryId, EntryKind};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use wasm_bindgen::prelude::*;
use web_sys::{
    IdbDatabase, IdbObjectStore, IdbObjectStoreParameters, IdbTransaction, IdbTransactionMode,
};

pub const DB_NAME: &str = "cast-iron";
pub const DB_VERSION: u32 = 1;

pub const META: &str = "meta";
pub const BOOKMARKS: &str = "bookmarks";
pub const COLLECTIONS: &str = "collections";

pub const KEY_ACTIVE_LIBRARY: &str = "active_library";
pub const KEY_PENDING_INSTALL: &str = "pending_install";
pub const KEY_THEME: &str = "theme";
pub const KEY_DISABLED_BOOKS: &str = "disabled_books";
pub const KEY_RECENT: &str = "recent";
pub const KEY_ID_MIGRATION: &str = "id_migration";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound(String),
    Backend(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound(what) => write!(f, "{what} was not found"),
            StoreError::Backend(message) => write!(f, "local storage error: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<JsValue> for StoreError {
    fn from(value: JsValue) -> Self {
        StoreError::Backend(js::describe(&value))
    }
}

impl From<serde_wasm_bindgen::Error> for StoreError {
    fn from(error: serde_wasm_bindgen::Error) -> Self {
        StoreError::Backend(error.to_string())
    }
}

type Result<T> = std::result::Result<T, StoreError>;

// ---- Records ----------------------------------------------------------------------------

/// The library that the reader uses. This record alone says that a library is ready.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveLibrary {
    pub content_version: String,
    pub schema_version: u32,
    pub entry_count: u32,
    pub total_bytes: u64,
    /// Milliseconds since the Unix epoch.
    pub activated_at: f64,
}

/// Set when a download starts and cleared on activation. It lets the next launch say
/// that an install was interrupted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingInstall {
    pub content_version: String,
    pub started_at: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub entry_id: EntryId,
    /// The title when bookmarked, so the list still reads if a later library drops the entry.
    pub title_snapshot: String,
    pub kind: EntryKind,
    pub added_at: f64,
    #[serde(default)]
    pub collection_ids: Vec<String>,
}

impl Bookmark {
    pub fn new(entry_id: EntryId, title: impl Into<String>, kind: EntryKind, now: f64) -> Self {
        Self {
            entry_id,
            title_snapshot: title.into(),
            kind,
            added_at: now,
            collection_ids: Vec::new(),
        }
    }

    pub fn in_collection(&self, collection_id: &str) -> bool {
        self.collection_ids.iter().any(|id| id == collection_id)
    }

    /// Adds a collection id once. Returns whether the bookmark changed.
    pub fn add_collection(&mut self, collection_id: &str) -> bool {
        if self.in_collection(collection_id) {
            return false;
        }
        self.collection_ids.push(collection_id.to_string());
        true
    }

    /// Removes a collection id. Returns whether the bookmark changed.
    pub fn remove_collection(&mut self, collection_id: &str) -> bool {
        let before = self.collection_ids.len();
        self.collection_ids.retain(|id| id != collection_id);
        self.collection_ids.len() != before
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub created_at: f64,
}

/// One recently viewed entry. Newest first in the stored list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recent {
    pub id: EntryId,
    pub title: String,
}

/// The once-per-alias-map marker. The value is the hash of `aliases.json`, not the content
/// version, because the map is append-only and a new alias can make an old id resolvable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdMigration {
    pub aliases_sha256: String,
}

/// Newest bookmark first, with the entry id as a tie-break so the order is stable.
pub fn sort_bookmarks(bookmarks: &mut [Bookmark]) {
    bookmarks.sort_by(|a, b| {
        b.added_at
            .total_cmp(&a.added_at)
            .then_with(|| a.entry_id.cmp(&b.entry_id))
    });
}

/// Collections by name, case-insensitively, then by creation time.
pub fn sort_collections(collections: &mut [Collection]) {
    collections.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.created_at.total_cmp(&b.created_at))
    });
}

// ---- Schema and migrations ---------------------------------------------------------------

pub struct StoreSpec {
    pub name: &'static str,
    pub key_path: Option<&'static str>,
}

const STORES_V1: [StoreSpec; 3] = [
    StoreSpec {
        name: META,
        key_path: None,
    },
    StoreSpec {
        name: BOOKMARKS,
        key_path: Some("entry_id"),
    },
    StoreSpec {
        name: COLLECTIONS,
        key_path: Some("id"),
    },
];

/// The object stores that an upgrade from `old_version` must create. `0` means a new database.
pub fn stores_to_create(old_version: u32) -> Vec<&'static StoreSpec> {
    let mut stores = Vec::new();
    if old_version < 1 {
        stores.extend(STORES_V1.iter());
    }
    // if old_version < 2 { stores.extend(STORES_V2.iter()); }
    stores
}

fn upgrade(db: &IdbDatabase, old_version: u32, _transaction: &IdbTransaction) -> Result<()> {
    for spec in stores_to_create(old_version) {
        match spec.key_path {
            Some(key_path) => {
                let params = IdbObjectStoreParameters::new();
                params.set_key_path(&JsValue::from_str(key_path));
                db.create_object_store_with_optional_parameters(spec.name, &params)?;
            }
            None => {
                db.create_object_store(spec.name)?;
            }
        }
    }
    // Later versions: migrate records here with `_transaction.object_store(..)`, keyed on `old_version`.
    Ok(())
}

// ---- Connection and transactions ----------------------------------------------------------

thread_local! {
    static DB: RefCell<Option<IdbDatabase>> = const { RefCell::new(None) };
}

async fn database() -> Result<IdbDatabase> {
    if let Some(db) = DB.with(|db| db.borrow().clone()) {
        return Ok(db);
    }
    let on_upgrade = Closure::once_into_js(
        move |db: IdbDatabase,
              old_version: u32,
              transaction: IdbTransaction|
              -> std::result::Result<(), JsValue> {
            upgrade(&db, old_version, &transaction).map_err(|e| JsValue::from_str(&e.to_string()))
        },
    );
    let db: IdbDatabase = js::idb_open(DB_NAME, DB_VERSION, on_upgrade.unchecked_ref())
        .await?
        .dyn_into()?;
    // Another tab may need to upgrade later: step aside.
    let close = db.clone();
    let on_version_change = Closure::<dyn FnMut()>::new(move || {
        close.close();
        DB.with(|db| *db.borrow_mut() = None);
    });
    db.set_onversionchange(Some(on_version_change.as_ref().unchecked_ref()));
    on_version_change.forget();
    DB.with(|slot| *slot.borrow_mut() = Some(db.clone()));
    Ok(db)
}

/// One transaction. The `done` promise exists before any request, so no outcome is missed.
struct Tx {
    tx: IdbTransaction,
    done: js_sys::Promise,
}

impl Tx {
    async fn begin(stores: &[&str], write: bool) -> Result<Tx> {
        let db = database().await?;
        let names = js_sys::Array::new();
        for name in stores {
            names.push(&JsValue::from_str(name));
        }
        let mode = if write {
            IdbTransactionMode::Readwrite
        } else {
            IdbTransactionMode::Readonly
        };
        let tx = db.transaction_with_str_sequence_and_mode(&names, mode)?;
        let done = js::idb_tx_done(&tx);
        Ok(Tx { tx, done })
    }

    fn store(&self, name: &str) -> Result<IdbObjectStore> {
        Ok(self.tx.object_store(name)?)
    }

    /// Waits for the commit. Dropping the transaction without this loses the failure signal.
    async fn finish(self) -> Result<()> {
        wasm_bindgen_futures::JsFuture::from(self.done).await?;
        Ok(())
    }
}

async fn request(request: Result<web_sys::IdbRequest>) -> Result<JsValue> {
    Ok(js::idb_request(&request?).await?)
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue> {
    Ok(value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?)
}

/// Decodes a record, or `None` for a missing key (`undefined`).
fn from_js<T: DeserializeOwned>(value: JsValue) -> Result<Option<T>> {
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    Ok(Some(serde_wasm_bindgen::from_value(value)?))
}

/// Decodes every record in an array. A record that no longer parses is skipped, not fatal.
fn from_js_all<T: DeserializeOwned>(array: JsValue) -> Vec<T> {
    js_sys::Array::from(&array)
        .iter()
        .filter_map(|item| serde_wasm_bindgen::from_value(item).ok())
        .collect()
}

// ---- meta -----------------------------------------------------------------------------------

pub async fn get_meta<T: DeserializeOwned>(key: &str) -> Result<Option<T>> {
    let tx = Tx::begin(&[META], false).await?;
    let store = tx.store(META)?;
    let value = request(store.get(&JsValue::from_str(key)).map_err(Into::into)).await?;
    tx.finish().await?;
    from_js(value)
}

pub async fn set_meta<T: Serialize>(key: &str, value: &T) -> Result<()> {
    let value = to_js(value)?;
    let tx = Tx::begin(&[META], true).await?;
    let store = tx.store(META)?;
    request(
        store
            .put_with_key(&value, &JsValue::from_str(key))
            .map_err(Into::into),
    )
    .await?;
    tx.finish().await
}

pub async fn remove_meta(key: &str) -> Result<()> {
    let tx = Tx::begin(&[META], true).await?;
    let store = tx.store(META)?;
    request(store.delete(&JsValue::from_str(key)).map_err(Into::into)).await?;
    tx.finish().await
}

pub async fn active_library() -> Result<Option<ActiveLibrary>> {
    get_meta(KEY_ACTIVE_LIBRARY).await
}

pub async fn pending_install() -> Result<Option<PendingInstall>> {
    get_meta(KEY_PENDING_INSTALL).await
}

pub async fn set_pending_install(pending: &PendingInstall) -> Result<()> {
    set_meta(KEY_PENDING_INSTALL, pending).await
}

/// Makes a verified library the active one, and clears the pending marker, in one transaction.
pub async fn activate_library(library: &ActiveLibrary) -> Result<()> {
    let value = to_js(library)?;
    let tx = Tx::begin(&[META], true).await?;
    let store = tx.store(META)?;
    request(
        store
            .put_with_key(&value, &JsValue::from_str(KEY_ACTIVE_LIBRARY))
            .map_err(Into::into),
    )
    .await?;
    request(
        store
            .delete(&JsValue::from_str(KEY_PENDING_INSTALL))
            .map_err(Into::into),
    )
    .await?;
    tx.finish().await
}

pub async fn theme_preference() -> Result<Option<String>> {
    get_meta(KEY_THEME).await
}

pub async fn set_theme_preference(theme: &str) -> Result<()> {
    set_meta(KEY_THEME, &theme).await
}

/// Canonical book IDs hidden from discovery. An absent value means every installed book is active.
pub async fn disabled_books() -> Result<std::collections::BTreeSet<String>> {
    Ok(get_meta(KEY_DISABLED_BOOKS).await?.unwrap_or_default())
}

pub async fn set_disabled_books(ids: &std::collections::BTreeSet<String>) -> Result<()> {
    set_meta(KEY_DISABLED_BOOKS, ids).await
}

// ---- bookmarks ------------------------------------------------------------------------------

/// Newest first.
pub async fn list_bookmarks() -> Result<Vec<Bookmark>> {
    let tx = Tx::begin(&[BOOKMARKS], false).await?;
    let store = tx.store(BOOKMARKS)?;
    let all = request(store.get_all().map_err(Into::into)).await?;
    tx.finish().await?;
    let mut bookmarks: Vec<Bookmark> = from_js_all(all);
    sort_bookmarks(&mut bookmarks);
    Ok(bookmarks)
}

pub async fn get_bookmark(id: &EntryId) -> Result<Option<Bookmark>> {
    let tx = Tx::begin(&[BOOKMARKS], false).await?;
    let store = tx.store(BOOKMARKS)?;
    let value = request(
        store
            .get(&JsValue::from_str(id.as_str()))
            .map_err(Into::into),
    )
    .await?;
    tx.finish().await?;
    from_js(value)
}

pub async fn put_bookmark(bookmark: &Bookmark) -> Result<()> {
    let value = to_js(bookmark)?;
    let tx = Tx::begin(&[BOOKMARKS], true).await?;
    let store = tx.store(BOOKMARKS)?;
    request(store.put(&value).map_err(Into::into)).await?;
    tx.finish().await
}

/// Atomically merges a previously validated batch; existing records are never replaced.
pub async fn merge_bookmarks(incoming: &[Bookmark]) -> Result<usize> {
    let tx = Tx::begin(&[BOOKMARKS], true).await?;
    let store = tx.store(BOOKMARKS)?;
    let all = request(store.get_all().map_err(Into::into)).await?;
    let current = from_js_all::<Bookmark>(all);
    let mut known: std::collections::BTreeSet<EntryId> =
        current.iter().map(|b| b.entry_id.clone()).collect();
    let mut added = 0;
    let mut prepared = Vec::new();
    for b in incoming {
        if known.insert(b.entry_id.clone()) {
            prepared.push(b);
            added += 1;
        }
    }
    for b in prepared {
        request(store.add(&to_js(b)?).map_err(Into::into)).await?;
    }
    tx.finish().await?;
    Ok(added)
}

pub async fn delete_bookmark(id: &EntryId) -> Result<()> {
    let tx = Tx::begin(&[BOOKMARKS], true).await?;
    let store = tx.store(BOOKMARKS)?;
    request(
        store
            .delete(&JsValue::from_str(id.as_str()))
            .map_err(Into::into),
    )
    .await?;
    tx.finish().await
}

// ---- canonical id migration ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct MigrationPlan {
    puts: Vec<Bookmark>,
    deletes: Vec<EntryId>,
    recent: Vec<Recent>,
    recent_changed: bool,
}

impl MigrationPlan {
    fn changed(&self) -> bool {
        !self.puts.is_empty() || !self.deletes.is_empty() || self.recent_changed
    }
}

/// One bookmark after merging every stored row that resolves to the same target.
fn merge_rows(target: &EntryId, rows: &[&Bookmark], target_kind: Option<EntryKind>) -> Bookmark {
    assert!(!rows.is_empty(), "a merge group always has a row");
    let added_at = rows
        .iter()
        .map(|row| row.added_at)
        .min_by(f64::total_cmp)
        .expect("non-empty");
    let earliest: Vec<&Bookmark> = rows
        .iter()
        .copied()
        .filter(|row| row.added_at.total_cmp(&added_at) == std::cmp::Ordering::Equal)
        .collect();
    let kind = target_kind.unwrap_or_else(|| {
        earliest
            .iter()
            .map(|row| row.kind)
            .min()
            .expect("non-empty")
    });
    let title_snapshot = earliest
        .iter()
        .map(|row| row.title_snapshot.as_str())
        .min()
        .expect("non-empty")
        .to_string();
    let mut collection_ids: Vec<String> = rows
        .iter()
        .flat_map(|row| row.collection_ids.iter().cloned())
        .collect();
    collection_ids.sort();
    collection_ids.dedup();
    Bookmark {
        entry_id: target.clone(),
        title_snapshot,
        kind,
        added_at,
        collection_ids,
    }
}

fn rewrite_recent(
    recents: &[Recent],
    aliases: &AliasIndex,
    known: impl Fn(&EntryId) -> bool,
) -> (Vec<Recent>, bool) {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    let mut changed = false;
    for recent in recents {
        let id = aliases
            .alias(&recent.id)
            .filter(|alias| known(&alias.target))
            .map_or_else(|| recent.id.clone(), |alias| alias.target.clone());
        if id != recent.id {
            changed = true;
        }
        if !seen.insert(id.clone()) {
            changed = true;
            continue;
        }
        out.push(Recent {
            id,
            title: recent.title.clone(),
        });
    }
    (out, changed)
}

/// The full rewrite plan. Retired and unknown rows are left untouched; they render from
/// their snapshot. Aliased rows merge by target, and an existing canonical row at a target
/// joins the same group so a retry after a partial commit produces the same rows.
fn plan_migration(
    bookmarks: &[Bookmark],
    recents: &[Recent],
    aliases: &AliasIndex,
    known: impl Fn(&EntryId) -> bool,
    kind_of: impl Fn(&EntryId) -> Option<EntryKind>,
) -> MigrationPlan {
    let mut groups: BTreeMap<EntryId, Vec<&Bookmark>> = BTreeMap::new();
    for bookmark in bookmarks {
        let Some(alias) = aliases.alias(&bookmark.entry_id) else {
            continue;
        };
        if known(&alias.target) {
            groups
                .entry(alias.target.clone())
                .or_default()
                .push(bookmark);
        }
    }

    let mut puts = Vec::new();
    let mut deletes = Vec::new();
    for (target, mut rows) in groups {
        if let Some(existing) = bookmarks.iter().find(|row| row.entry_id == target)
            && !rows.iter().any(|row| row.entry_id == target)
        {
            rows.push(existing);
        }
        puts.push(merge_rows(&target, &rows, kind_of(&target)));
        for row in rows {
            if row.entry_id != target {
                deletes.push(row.entry_id.clone());
            }
        }
    }
    deletes.sort();
    deletes.dedup();
    let (recent, recent_changed) = rewrite_recent(recents, aliases, known);
    MigrationPlan {
        puts,
        deletes,
        recent,
        recent_changed,
    }
}

/// Rewrites stored bookmark and recent ids to their canonical targets in one transaction,
/// then writes the alias-file hash marker. A second run with the same map changes nothing.
/// A transaction that fails before commit leaves every row as it was.
pub async fn migrate_entry_ids(
    aliases: &AliasIndex,
    aliases_sha256: &str,
    known: impl Fn(&EntryId) -> bool,
    kind_of: impl Fn(&EntryId) -> Option<EntryKind>,
) -> Result<bool> {
    if aliases.is_empty() || aliases_sha256.is_empty() {
        return Ok(false);
    }
    let tx = Tx::begin(&[BOOKMARKS, META], true).await?;
    let meta = tx.store(META)?;
    let marker = from_js::<IdMigration>(
        request(
            meta.get(&JsValue::from_str(KEY_ID_MIGRATION))
                .map_err(Into::into),
        )
        .await?,
    )?;
    if marker.is_some_and(|marker| marker.aliases_sha256 == aliases_sha256) {
        tx.finish().await?;
        return Ok(false);
    }
    let recents = from_js::<Vec<Recent>>(
        request(meta.get(&JsValue::from_str(KEY_RECENT)).map_err(Into::into)).await?,
    )
    .ok()
    .flatten()
    .unwrap_or_default();
    let bookmark_store = tx.store(BOOKMARKS)?;
    let all = request(bookmark_store.get_all().map_err(Into::into)).await?;
    let bookmarks = from_js_all::<Bookmark>(all);
    let plan = plan_migration(&bookmarks, &recents, aliases, known, kind_of);
    for bookmark in &plan.puts {
        request(bookmark_store.put(&to_js(bookmark)?).map_err(Into::into)).await?;
    }
    for id in &plan.deletes {
        request(
            bookmark_store
                .delete(&JsValue::from_str(id.as_str()))
                .map_err(Into::into),
        )
        .await?;
    }
    if plan.recent_changed {
        request(
            meta.put_with_key(&to_js(&plan.recent)?, &JsValue::from_str(KEY_RECENT))
                .map_err(Into::into),
        )
        .await?;
    }
    let marker = IdMigration {
        aliases_sha256: aliases_sha256.to_string(),
    };
    request(
        meta.put_with_key(&to_js(&marker)?, &JsValue::from_str(KEY_ID_MIGRATION))
            .map_err(Into::into),
    )
    .await?;
    tx.finish().await?;
    Ok(plan.changed())
}

// ---- collections ----------------------------------------------------------------------------

pub async fn list_collections() -> Result<Vec<Collection>> {
    let tx = Tx::begin(&[COLLECTIONS], false).await?;
    let store = tx.store(COLLECTIONS)?;
    let all = request(store.get_all().map_err(Into::into)).await?;
    tx.finish().await?;
    let mut collections: Vec<Collection> = from_js_all(all);
    sort_collections(&mut collections);
    Ok(collections)
}

pub async fn put_collection(collection: &Collection) -> Result<()> {
    let value = to_js(collection)?;
    let tx = Tx::begin(&[COLLECTIONS], true).await?;
    let store = tx.store(COLLECTIONS)?;
    request(store.put(&value).map_err(Into::into)).await?;
    tx.finish().await
}

/// Creates and saves a collection with a fresh id (`crypto.randomUUID()`).
pub async fn create_collection(name: &str) -> Result<Collection> {
    let collection = Collection {
        id: js::random_uuid(),
        name: name.trim().to_string(),
        created_at: js::now_ms(),
    };
    put_collection(&collection).await?;
    Ok(collection)
}

/// Deletes a collection and removes its id from every bookmark, in one transaction.
/// The bookmarks themselves stay.
pub async fn delete_collection(id: &str) -> Result<()> {
    let tx = Tx::begin(&[COLLECTIONS, BOOKMARKS], true).await?;
    let bookmark_store = tx.store(BOOKMARKS)?;
    let all = request(bookmark_store.get_all().map_err(Into::into)).await?;
    for mut bookmark in from_js_all::<Bookmark>(all) {
        if bookmark.remove_collection(id) {
            request(bookmark_store.put(&to_js(&bookmark)?).map_err(Into::into)).await?;
        }
    }
    let collections = tx.store(COLLECTIONS)?;
    request(
        collections
            .delete(&JsValue::from_str(id))
            .map_err(Into::into),
    )
    .await?;
    tx.finish().await
}

/// Puts an existing bookmark into an existing collection.
pub async fn add_to_collection(entry_id: &EntryId, collection_id: &str) -> Result<()> {
    change_membership(entry_id, collection_id, true).await
}

pub async fn remove_from_collection(entry_id: &EntryId, collection_id: &str) -> Result<()> {
    change_membership(entry_id, collection_id, false).await
}

async fn change_membership(entry_id: &EntryId, collection_id: &str, add: bool) -> Result<()> {
    let tx = Tx::begin(&[BOOKMARKS, COLLECTIONS], true).await?;
    if add {
        let found = request(
            tx.store(COLLECTIONS)?
                .get_key(&JsValue::from_str(collection_id))
                .map_err(Into::into),
        )
        .await?;
        if found.is_undefined() {
            tx.tx.abort()?;
            let _ = tx.finish().await;
            return Err(StoreError::NotFound(format!("collection {collection_id}")));
        }
    }
    let bookmarks = tx.store(BOOKMARKS)?;
    let value = request(
        bookmarks
            .get(&JsValue::from_str(entry_id.as_str()))
            .map_err(Into::into),
    )
    .await?;
    let Some(mut bookmark) = from_js::<Bookmark>(value)? else {
        tx.tx.abort()?;
        let _ = tx.finish().await;
        return Err(StoreError::NotFound(format!("bookmark {entry_id}")));
    };
    let changed = if add {
        bookmark.add_collection(collection_id)
    } else {
        bookmark.remove_collection(collection_id)
    };
    if changed {
        request(bookmarks.put(&to_js(&bookmark)?).map_err(Into::into)).await?;
    }
    tx.finish().await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bookmark(id: &str, at: f64) -> Bookmark {
        Bookmark::new(EntryId::parse(id).unwrap(), id, EntryKind::Spell, at)
    }

    #[test]
    fn a_new_database_gets_every_store_and_a_current_one_gets_none() {
        let names: Vec<_> = stores_to_create(0).iter().map(|s| s.name).collect();
        assert_eq!(names, [META, BOOKMARKS, COLLECTIONS]);
        assert!(stores_to_create(DB_VERSION).is_empty());
    }

    #[test]
    fn key_paths_match_the_record_fields() {
        let record = serde_json::to_value(bookmark("a/b", 1.0)).unwrap();
        let collection = serde_json::to_value(Collection {
            id: "c".into(),
            name: "n".into(),
            created_at: 1.0,
        })
        .unwrap();
        for spec in stores_to_create(0) {
            match spec.name {
                BOOKMARKS => assert!(record.get(spec.key_path.unwrap()).is_some()),
                COLLECTIONS => assert!(collection.get(spec.key_path.unwrap()).is_some()),
                _ => assert!(spec.key_path.is_none()),
            }
        }
    }

    #[test]
    fn membership_changes_report_whether_anything_changed() {
        let mut b = bookmark("a/b", 1.0);
        assert!(b.add_collection("c1"));
        assert!(!b.add_collection("c1"));
        assert!(b.add_collection("c2"));
        assert!(b.remove_collection("c1"));
        assert!(!b.remove_collection("c1"));
        assert_eq!(b.collection_ids, ["c2"]);
    }

    #[test]
    fn bookmarks_sort_newest_first_with_stable_ties() {
        let mut list = vec![bookmark("b", 1.0), bookmark("a", 1.0), bookmark("c", 5.0)];
        sort_bookmarks(&mut list);
        let ids: Vec<_> = list.iter().map(|b| b.entry_id.as_str()).collect();
        assert_eq!(ids, ["c", "a", "b"]);
    }

    #[test]
    fn collections_sort_by_name_ignoring_case() {
        let c = |id: &str, name: &str| Collection {
            id: id.into(),
            name: name.into(),
            created_at: 0.0,
        };
        let mut list = vec![c("1", "banana"), c("2", "Apple")];
        sort_collections(&mut list);
        assert_eq!(list[0].name, "Apple");
    }

    fn aliases() -> AliasIndex {
        serde_json::from_str(
            r#"{
                "format": 1,
                "legacy": {"count": 4, "sha256": ""},
                "entries": [
                    {"old": "legacy/a", "target": "ref/rule/merged", "basis": "direct"},
                    {"old": "legacy/b", "target": "ref/rule/merged", "basis": "shared"},
                    {"old": "legacy/c", "target": "ref/rule/other", "basis": "direct"}
                ],
                "retired": [
                    {"old": "legacy/retired", "reason": "no_source"}
                ]
            }"#,
        )
        .unwrap()
    }

    fn id(value: &str) -> EntryId {
        EntryId::parse(value).unwrap()
    }

    fn row(entry: &str, title: &str, kind: EntryKind, at: f64, collections: &[&str]) -> Bookmark {
        Bookmark {
            entry_id: id(entry),
            title_snapshot: title.into(),
            kind,
            added_at: at,
            collection_ids: collections.iter().map(|value| value.to_string()).collect(),
        }
    }

    fn recent(entry: &str, title: &str) -> Recent {
        Recent {
            id: id(entry),
            title: title.into(),
        }
    }

    fn known(_: &EntryId) -> bool {
        true
    }

    #[test]
    fn migration_merges_by_target_with_min_time_sorted_union_and_target_kind() {
        let rows = vec![
            row("legacy/a", "Zulu", EntryKind::Spell, 10.0, &["b"]),
            row("legacy/b", "Alpha", EntryKind::Rule, 5.0, &["a", "b"]),
        ];
        let plan = plan_migration(&rows, &[], &aliases(), known, |_| Some(EntryKind::Feat));
        assert_eq!(plan.puts.len(), 1);
        let merged = &plan.puts[0];
        assert_eq!(merged.entry_id, id("ref/rule/merged"));
        assert_eq!(merged.added_at, 5.0);
        assert_eq!(merged.kind, EntryKind::Feat);
        assert_eq!(merged.title_snapshot, "Alpha"); // the earliest row
        assert_eq!(merged.collection_ids, ["a", "b"]);
        assert_eq!(plan.deletes, [id("legacy/a"), id("legacy/b")]);
    }

    #[test]
    fn migration_falls_back_to_the_earliest_kind_and_title_with_lexicographic_ties() {
        let rows = vec![
            row("legacy/a", "Zulu", EntryKind::Spell, 7.0, &[]),
            row("legacy/b", "Alpha", EntryKind::Rule, 7.0, &[]),
        ];
        let plan = plan_migration(&rows, &[], &aliases(), known, |_| None);
        let merged = &plan.puts[0];
        assert_eq!(merged.kind, EntryKind::Rule);
        assert_eq!(merged.title_snapshot, "Alpha");
        assert_eq!(merged.added_at, 7.0);
    }

    #[test]
    fn migration_leaves_canonical_retired_and_unknown_rows_untouched() {
        let rows = vec![
            row("ref/rule/merged", "Canonical", EntryKind::Rule, 1.0, &[]),
            row("legacy/retired", "Retired", EntryKind::Spell, 2.0, &[]),
            row("legacy/missing", "Missing", EntryKind::Spell, 3.0, &[]),
        ];
        let plan = plan_migration(&rows, &[], &aliases(), known, |_| None);
        assert!(plan.puts.is_empty());
        assert!(plan.deletes.is_empty());
    }

    #[test]
    fn migration_joins_an_existing_canonical_row_to_the_alias_group() {
        let rows = vec![
            row("legacy/a", "Old", EntryKind::Spell, 3.0, &["x"]),
            row("ref/rule/merged", "New", EntryKind::Rule, 1.0, &["y"]),
        ];
        let plan = plan_migration(&rows, &[], &aliases(), known, |_| None);
        assert_eq!(plan.puts.len(), 1);
        assert_eq!(plan.puts[0].entry_id, id("ref/rule/merged"));
        assert_eq!(plan.puts[0].title_snapshot, "New");
        assert_eq!(plan.puts[0].collection_ids, ["x", "y"]);
        assert_eq!(plan.deletes, [id("legacy/a")]);
    }

    #[test]
    fn migration_is_idempotent_on_its_own_output() {
        let aliases = aliases();
        let rows = vec![
            row("legacy/a", "Zulu", EntryKind::Spell, 10.0, &["b"]),
            row("legacy/b", "Alpha", EntryKind::Rule, 5.0, &["a"]),
        ];
        let recents = vec![recent("legacy/a", "Zulu"), recent("legacy/b", "Alpha")];
        let plan = plan_migration(&rows, &recents, &aliases, known, |_| None);
        let again = plan_migration(&plan.puts, &plan.recent, &aliases, known, |_| None);
        assert!(again.puts.is_empty());
        assert!(again.deletes.is_empty());
        assert!(!again.recent_changed);
        assert_eq!(again.recent, plan.recent);
        assert!(!again.changed());
    }

    #[test]
    fn recent_rewrite_resolves_dedupes_newest_first_and_keeps_retired() {
        let recents = vec![
            recent("legacy/a", "First"),
            recent("legacy/b", "Second"),
            recent("legacy/retired", "Old"),
            recent("legacy/c", "Other"),
        ];
        let (rewritten, changed) = rewrite_recent(&recents, &aliases(), known);
        assert!(changed);
        let ids: Vec<&str> = rewritten.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["ref/rule/merged", "legacy/retired", "ref/rule/other"]);
        assert_eq!(rewritten[0].title, "First"); // the newest row keeps its title
    }

    #[test]
    fn records_round_trip_through_json_and_accept_missing_collections() {
        let b = bookmark("core/x", 3.0);
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(serde_json::from_str::<Bookmark>(&json).unwrap(), b);
        let old = r#"{"entry_id":"core/x","title_snapshot":"X","kind":"spell","added_at":1}"#;
        assert!(
            serde_json::from_str::<Bookmark>(old)
                .unwrap()
                .collection_ids
                .is_empty()
        );
    }
}
