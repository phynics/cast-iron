//! The Cache API side of the library: one cache per content version, named
//! `cast-iron-content-<version>`. Keys are the document-relative `content/<path>` URLs.
//! The service worker never touches these caches.

use futures::future::Either;
use js_sys::Uint8Array;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, Cache, CacheStorage, Request, Response};

use crate::install::plan::{STALL_WINDOW_MS, StallWatch};
use crate::js::{self, describe};

pub const PREFIX: &str = "cast-iron-content-";

pub fn cache_name(version: &str) -> String {
    format!("{PREFIX}{version}")
}

/// The request URL for a manifest-relative path. It resolves against the document base.
pub fn url(path: &str) -> String {
    format!("content/{path}")
}

fn storage() -> Result<CacheStorage, String> {
    web_sys::window()
        .ok_or_else(|| "no window".to_string())?
        .caches()
        .map_err(|e| format!("Cache storage is unavailable: {}", describe(&e)))
}

/// Opens the cache, and creates it when it does not exist.
pub async fn open(version: &str) -> Result<Cache, String> {
    let cache = JsFuture::from(storage()?.open(&cache_name(version)))
        .await
        .map_err(|e| describe(&e))?;
    cache.dyn_into().map_err(|_| "not a cache".to_string())
}

/// Opens the cache only when it exists. `open` would create an empty one.
pub async fn open_existing(version: &str) -> Result<Option<Cache>, String> {
    let name = cache_name(version);
    let has = JsFuture::from(storage()?.has(&name))
        .await
        .map_err(|e| describe(&e))?;
    if has.is_truthy() {
        open(version).await.map(Some)
    } else {
        Ok(None)
    }
}

/// Names of every content cache, whatever the version.
pub async fn list_names() -> Result<Vec<String>, String> {
    let keys = JsFuture::from(storage()?.keys())
        .await
        .map_err(|e| describe(&e))?;
    Ok(js_sys::Array::from(&keys)
        .iter()
        .filter_map(|k| k.as_string())
        .filter(|k| k.starts_with(PREFIX))
        .collect())
}

pub async fn delete(name: &str) -> Result<(), String> {
    JsFuture::from(storage()?.delete(name))
        .await
        .map(|_| ())
        .map_err(|e| describe(&e))
}

/// Reads a file's bytes from the cache. `None` when the cache has no such file.
pub async fn read(cache: &Cache, path: &str) -> Result<Option<Vec<u8>>, String> {
    let found = JsFuture::from(cache.match_with_str(&url(path)))
        .await
        .map_err(|e| describe(&e))?;
    if found.is_undefined() {
        return Ok(None);
    }
    let response: Response = found.dyn_into().map_err(|_| "not a response".to_string())?;
    let buffer = JsFuture::from(response.array_buffer().map_err(|e| describe(&e))?)
        .await
        .map_err(|e| describe(&e))?;
    Ok(Some(Uint8Array::new(&buffer).to_vec()))
}

/// Reads a stored gzip file and decodes it in the browser. `None` when the cache has no such
/// file. The caller has checked that the browser can ([`crate::js::gunzip_supported`]).
pub async fn read_gunzip(cache: &Cache, path: &str) -> Result<Option<Vec<u8>>, String> {
    crate::js::cache_read_gunzip(cache, &url(path)).await
}

/// True when the cache holds the file. Does not read the body.
pub async fn contains(cache: &Cache, path: &str) -> Result<bool, String> {
    let found = JsFuture::from(cache.match_with_str(&url(path)))
        .await
        .map_err(|e| describe(&e))?;
    Ok(!found.is_undefined())
}

/// The paths of the files a cache holds, without reading any body. Fewer than the real count is
/// possible only when the browser dropped entries; the caller treats the answer as progress, not
/// proof, because the installer re-verifies every file before it uses one.
pub async fn cached_paths(cache: &Cache) -> Result<Vec<String>, String> {
    let keys = JsFuture::from(cache.keys())
        .await
        .map_err(|e| describe(&e))?;
    Ok(js_sys::Array::from(&keys)
        .iter()
        .filter_map(|request| request.dyn_into::<Request>().ok())
        .map(|request| request.url())
        .collect())
}

pub async fn put(cache: &Cache, path: &str, bytes: &[u8]) -> Result<(), String> {
    let mut body = bytes.to_vec();
    let response = Response::new_with_opt_u8_array(Some(&mut body)).map_err(|e| describe(&e))?;
    JsFuture::from(cache.put_with_str(&url(path), &response))
        .await
        .map(|_| ())
        .map_err(|e| describe(&e))
}

/// Why a network read failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// No answer: offline, DNS, or a blocked request.
    Network(String),
    Status(u16),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Network(message) => write!(f, "network error ({message})"),
            FetchError::Status(code) => write!(f, "the server answered HTTP {code}"),
        }
    }
}

/// Fetches `content/<path>` from the network, bypassing the HTTP cache. Only the installer
/// and the update check call this.
///
/// A request that gets no bytes for [`STALL_WINDOW_MS`] is aborted and fails as a network error,
/// so the installer's retries take over (#768). The bridge streams the body so that this side can
/// count progress; the decision is [`StallWatch`], which is tested natively.
pub async fn fetch_network(path: &str) -> Result<Vec<u8>, FetchError> {
    let controller = AbortController::new().map_err(|e| FetchError::Network(describe(&e)))?;
    let progress = Rc::new(Cell::new(0u64));
    let on_progress = {
        let progress = Rc::clone(&progress);
        Closure::<dyn Fn()>::new(move || progress.set(progress.get() + 1))
    };
    let address = url(path);
    let fetch = js::fetch_bytes(&address, &controller, on_progress.as_ref().unchecked_ref());
    let watchdog = async {
        let mut watch = StallWatch::default();
        loop {
            js::sleep(STALL_WINDOW_MS).await;
            if watch.stalled(progress.get()) {
                return;
            }
        }
    };
    futures::pin_mut!(fetch, watchdog);
    let value = match futures::future::select(fetch, watchdog).await {
        Either::Left((result, _)) => result.map_err(|e| FetchError::Network(describe(&e)))?,
        Either::Right(((), _)) => {
            controller.abort();
            return Err(FetchError::Network(format!(
                "no data for {} seconds",
                STALL_WINDOW_MS / 1000
            )));
        }
    };
    if let Some(status) = value.as_f64() {
        return Err(FetchError::Status(status as u16));
    }
    Ok(Uint8Array::from(value).to_vec())
}
