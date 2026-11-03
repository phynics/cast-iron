//! Typed access to `js/bridge.js` and a few small browser helpers.
//! Everything else in the crate stays in Rust.

use js_sys::{Function, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbRequest, IdbTransaction};

#[wasm_bindgen(module = "/js/bridge.js")]
extern "C" {
    #[wasm_bindgen(js_name = idbOpen, catch)]
    pub async fn idb_open(
        name: &str,
        version: u32,
        on_upgrade: &Function,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = idbRequest, catch)]
    pub async fn idb_request(request: &IdbRequest) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = idbTxDone)]
    pub fn idb_tx_done(transaction: &IdbTransaction) -> Promise;
    #[wasm_bindgen(js_name = sha256Hex, catch)]
    async fn sha256_hex_js(buffer: &JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = randomUUID)]
    pub fn random_uuid() -> String;
    #[wasm_bindgen(js_name = storagePersist, catch)]
    async fn storage_persist_js() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = storagePersisted, catch)]
    async fn storage_persisted_js() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = storageEstimate, catch)]
    async fn storage_estimate_js() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = registerServiceWorker, catch)]
    async fn register_service_worker_js(
        url: &str,
        scope: &str,
        on_waiting: &Function,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = checkShellUpdate, catch)]
    async fn check_shell_update_js() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = applyShellUpdate)]
    pub fn apply_shell_update();
    #[wasm_bindgen(js_name = shellVersion, catch)]
    async fn shell_version_js() -> Result<JsValue, JsValue>;
}

/// A readable message for anything that JavaScript can throw.
pub fn describe(value: &JsValue) -> String {
    if let Some(text) = value.as_string() {
        return text;
    }
    if let Ok(message) = Reflect::get(value, &JsValue::from_str("message"))
        && let Some(message) = message.as_string()
    {
        let name = Reflect::get(value, &JsValue::from_str("name"))
            .ok()
            .and_then(|n| n.as_string())
            .filter(|n| n != "Error");
        return match name {
            Some(name) => format!("{name}: {message}"),
            None => message,
        };
    }
    format!("{value:?}")
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub async fn sleep(ms: u32) {
    let promise = Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ =
                window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        }
    });
    let _ = JsFuture::from(promise).await;
}

/// Lowercase hex SHA-256, from `crypto.subtle`. Fails outside a secure context.
pub async fn sha256_hex(bytes: &[u8]) -> Result<String, String> {
    let buffer = Uint8Array::from(bytes).buffer();
    sha256_hex_js(&buffer)
        .await
        .map_err(|e| describe(&e))?
        .as_string()
        .ok_or_else(|| "sha256 returned no text".to_string())
}

/// Asks for persistent storage. False when unsupported or refused.
pub async fn storage_persist() -> bool {
    storage_persist_js().await.is_ok_and(|v| v.is_truthy())
}

pub async fn storage_persisted() -> bool {
    storage_persisted_js().await.is_ok_and(|v| v.is_truthy())
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StorageEstimate {
    pub usage: Option<f64>,
    pub quota: Option<f64>,
}

pub async fn storage_estimate() -> StorageEstimate {
    let Ok(value) = storage_estimate_js().await else {
        return StorageEstimate::default();
    };
    let field = |name: &str| {
        Reflect::get(&value, &JsValue::from_str(name))
            .ok()
            .and_then(|v| v.as_f64())
    };
    StorageEstimate {
        usage: field("usage"),
        quota: field("quota"),
    }
}

/// Registers `sw.js` relative to the document base. `on_waiting` fires when a newer shell waits.
pub async fn register_service_worker(on_waiting: impl Fn() + 'static) -> Result<bool, String> {
    let callback = Closure::<dyn Fn()>::new(on_waiting);
    let result = register_service_worker_js("sw.js", "./", callback.as_ref().unchecked_ref()).await;
    // The worker can announce a waiting update at any time, so the callback lives for the page.
    callback.forget();
    result.map(|v| v.is_truthy()).map_err(|e| describe(&e))
}

/// Looks for a newer `sw.js` now. Needs the network.
pub async fn check_shell_update() -> Result<(), String> {
    check_shell_update_js()
        .await
        .map(|_| ())
        .map_err(|e| describe(&e))
}

/// The shell version that the controlling service worker reports.
pub async fn shell_version() -> Option<String> {
    shell_version_js().await.ok().and_then(|v| v.as_string())
}
