//! Cast Iron web client: platform layer.
//!
//! - [`route`]: hash routes. Pure parsing, plus `use_route`, `navigate`, scroll helpers.
//! - [`library`]: read access to the active library (catalog, entries, search, legal, and the
//!   schema-3 relation, spell-list, and book files). [`graph`]: pure lookups over those files.
//! - [`store`]: IndexedDB for bookmarks, collections, and preferences.
//! - [`install`]: the first-run and update installer.
//! - [`updates`]: the opt-out daily check for a newer library, and the state the header reads.
//! - [`app`]: the shell. [`views`]: one module per screen.
//!
//! The crate compiles for the host too, so the pure logic has native tests. Browser calls
//! panic if run natively; tests only touch the pure parts.

pub mod app;
pub mod book_presets;
pub mod class_options;
pub mod content_cache;
pub mod density;
pub mod discovery;
pub mod fmt;
pub mod graph;
pub mod install;
pub mod js;
pub mod library;
pub mod lru;
pub mod pwa;
pub mod report;
pub mod revisions;
pub mod route;
pub mod store;
pub mod theme;
pub mod updates;
pub mod views;

/// Entry point for the browser build.
pub fn start() {
    console_error_panic_hook::set_once();
    theme::apply_mirrored();
    leptos::mount::mount_to_body(app::App);
}
