//! Cast Iron web client: platform layer.
//!
//! - [`route`]: hash routes. Pure parsing, plus `use_route`, `navigate`, scroll helpers.
//! - [`library`]: read access to the active library (catalog, entries, search, legal).
//! - [`store`]: IndexedDB for bookmarks, collections, and preferences.
//! - [`install`]: the first-run and update installer.
//! - [`app`]: the shell. [`views`]: one module per screen.
//!
//! The crate compiles for the host too, so the pure logic has native tests. Browser calls
//! panic if run natively; tests only touch the pure parts.

pub mod app;
pub mod content_cache;
pub mod fmt;
pub mod install;
pub mod js;
pub mod library;
pub mod lru;
pub mod route;
pub mod store;
pub mod theme;
pub mod views;

/// Entry point for the browser build.
pub fn start() {
    console_error_panic_hook::set_once();
    theme::apply_mirrored();
    leptos::mount::mount_to_body(app::App);
}
