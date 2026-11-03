//! The Cache API side of the library: one cache per content version, named
//! `cast-iron-content-<version>`. Keys are the document-relative `content/<path>` URLs.
//! The service worker never touches these caches.

use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Cache, CacheStorage, Request, RequestCache, RequestInit, Response};

use crate::js::describe;

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

/// True when the cache holds the file. Does not read the body.
pub async fn contains(cache: &Cache, path: &str) -> Result<bool, String> {
    let found = JsFuture::from(cache.match_with_str(&url(path)))
        .await
        .map_err(|e| describe(&e))?;
    Ok(!found.is_undefined())
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
pub async fn fetch_network(path: &str) -> Result<Vec<u8>, FetchError> {
    let init = RequestInit::new();
    init.set_cache(RequestCache::NoStore);
    let request = Request::new_with_str_and_init(&url(path), &init)
        .map_err(|e| FetchError::Network(describe(&e)))?;
    let window = web_sys::window().ok_or_else(|| FetchError::Network("no window".into()))?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| FetchError::Network(describe(&e)))?
        .dyn_into()
        .map_err(|_: JsValue| FetchError::Network("not a response".into()))?;
    if !response.ok() {
        return Err(FetchError::Status(response.status()));
    }
    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|e| FetchError::Network(describe(&e)))?,
    )
    .await
    .map_err(|e| FetchError::Network(describe(&e)))?;
    Ok(Uint8Array::new(&buffer).to_vec())
}
