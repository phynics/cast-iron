//! The update check: the scheduling rule, the state that the header and Settings read, and the
//! one network request the app makes on its own.
//!
//! The owner narrowed the offline rule for this (#206): the automatic check is on by default,
//! can be switched off, runs at most once a day, only while online, and fetches only the app's
//! own `content/manifest.json`. It never downloads the library; downloading still needs a tap.
//! It never blocks a read and it fails silently, because nothing the reader is doing depends on
//! its answer.

use crate::app::use_app;
use crate::install::{self, UpdateCheck};
use crate::js;
use crate::store::{self, ActiveLibrary, UpdateCheckRecord};
use cast_iron_schema::ContentManifest;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// The shortest gap between automatic checks: one day.
pub const CHECK_INTERVAL_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

/// Whether an automatic check should run now. A missing `last_checked` means the app has never
/// asked; a clock that went backwards waits instead of asking again.
pub fn should_check(now: f64, last_checked: Option<f64>, online: bool, enabled: bool) -> bool {
    enabled && online && last_checked.is_none_or(|last| now - last >= CHECK_INTERVAL_MS)
}

/// A newer library that a check found. The manifest is kept for this session so Download needs
/// no second fetch; after a reload only the version and size are known until the next check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryUpdate {
    pub version: String,
    pub bytes: u64,
    pub manifest: Option<ContentManifest>,
}

/// App-wide update state. Get it with [`use_updates`].
#[derive(Clone, Copy)]
pub struct UpdateCtx {
    /// A newer library that the last check found, and that is not the active one.
    pub library_update: RwSignal<Option<LibraryUpdate>>,
    /// When the last check ran, in milliseconds since the Unix epoch.
    pub last_checked: RwSignal<Option<f64>>,
    /// The Settings switch, "Check for updates automatically". On by default.
    pub auto: RwSignal<bool>,
    /// The active library's version, which a check compares against the manifest.
    active_version: RwSignal<String>,
    online: RwSignal<bool>,
}

pub fn use_updates() -> UpdateCtx {
    expect_context::<UpdateCtx>()
}

impl UpdateCtx {
    /// Creates the state, reads what the device remembers, and asks for a check when the rule
    /// allows one. Call once, in the Reader.
    pub fn provide(active: ActiveLibrary) -> Self {
        let app = use_app();
        let ctx = Self {
            library_update: RwSignal::new(None),
            last_checked: RwSignal::new(None),
            auto: RwSignal::new(true),
            active_version: RwSignal::new(active.content_version),
            online: app.online,
        };
        spawn_local(async move {
            let record = store::update_check().await.ok().flatten();
            ctx.auto.set(store::auto_update_check_enabled().await);
            if let Some(record) = record {
                ctx.last_checked.set(Some(record.last_checked));
                ctx.show_known(&record);
            }
            ctx.maybe_check();
        });
        ctx.follow_visibility();
        ctx
    }

    /// Runs the automatic check when the rule allows it. Called at app start and whenever the
    /// document becomes visible again.
    pub fn maybe_check(self) {
        if should_check(
            js::now_ms(),
            self.last_checked.get_untracked(),
            self.online.get_untracked(),
            self.auto.get_untracked(),
        ) {
            spawn_local(async move {
                let _ = self.check_now().await;
            });
        }
    }

    /// Asks for the app's own manifest and records the answer. Never downloads the library.
    /// Errors are returned for an explicit check to show; the automatic caller ignores them.
    pub async fn check_now(self) -> Result<UpdateCheck, String> {
        let active_version = self.active_version.get_untracked();
        let checked_at = js::now_ms();
        let result = install::check_for_update(&active_version).await;
        // The rate limit counts the attempt, so a failing server is not asked on every focus.
        self.last_checked.set(Some(checked_at));
        let record = match &result {
            Ok(UpdateCheck::Available(manifest)) => {
                self.library_update.set(Some(LibraryUpdate {
                    version: manifest.content_version.clone(),
                    bytes: manifest.total_bytes,
                    manifest: Some(manifest.clone()),
                }));
                UpdateCheckRecord {
                    last_checked: checked_at,
                    known_version: Some(manifest.content_version.clone()),
                    known_bytes: manifest.total_bytes,
                }
            }
            Ok(UpdateCheck::UpToDate) => {
                self.library_update.set(None);
                UpdateCheckRecord {
                    last_checked: checked_at,
                    known_version: Some(active_version),
                    known_bytes: 0,
                }
            }
            Err(_) => {
                let known = self.library_update.get_untracked();
                UpdateCheckRecord {
                    last_checked: checked_at,
                    known_version: known.as_ref().map(|update| update.version.clone()),
                    known_bytes: known.as_ref().map_or(0, |update| update.bytes),
                }
            }
        };
        let _ = store::set_update_check(&record).await;
        result.map_err(|failure| failure.message)
    }

    /// Stores the switch and follows it at once. A failed write leaves the default for next time.
    pub fn set_auto(self, enabled: bool) {
        self.auto.set(enabled);
        spawn_local(async move {
            let _ = store::set_auto_update_check(enabled).await;
        });
    }

    /// Shows a newer library that an earlier session found, before this session has checked.
    fn show_known(self, record: &UpdateCheckRecord) {
        let known = record
            .known_version
            .as_ref()
            .filter(|version| **version != self.active_version.get_untracked());
        self.library_update.set(known.map(|version| LibraryUpdate {
            version: version.clone(),
            bytes: record.known_bytes,
            manifest: None,
        }));
    }

    /// Checks again when a hidden tab becomes visible, as the app start rule does.
    fn follow_visibility(self) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let watched = document.clone();
        let callback = Closure::<dyn FnMut()>::new(move || {
            if watched.visibility_state() == web_sys::VisibilityState::Visible {
                self.maybe_check();
            }
        })
        .into_js_value();
        let _ =
            document.add_event_listener_with_callback("visibilitychange", callback.unchecked_ref());
        // `on_cleanup` needs a `Send + Sync` closure; the values stay on this thread.
        let document = send_wrapper::SendWrapper::new(document);
        let callback = send_wrapper::SendWrapper::new(callback);
        on_cleanup(move || {
            let _ = document
                .remove_event_listener_with_callback("visibilitychange", callback.unchecked_ref());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_once_a_day_only_when_online_and_enabled() {
        let day = CHECK_INTERVAL_MS;
        assert!(should_check(0.0, None, true, true));
        assert!(!should_check(0.0, None, false, true));
        assert!(!should_check(0.0, None, true, false));
        assert!(!should_check(day - 1.0, Some(0.0), true, true));
        assert!(should_check(day, Some(0.0), true, true));
        assert!(should_check(day + 5.0, Some(0.0), true, true));
        // A clock that moved backwards waits instead of asking again.
        assert!(!should_check(0.0, Some(day), true, true));
        // Neither the online state nor the switch can be papered over by an old timestamp.
        assert!(!should_check(day, Some(0.0), false, true));
        assert!(!should_check(day, Some(0.0), true, false));
    }
}
