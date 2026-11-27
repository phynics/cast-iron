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

/// Whether a half-downloaded version that an earlier session left behind (`pending_install`) is
/// no longer worth resuming. It is when the last check named another version as the newest one:
/// the partial download belongs to a library that is not offered any more.
pub fn pending_is_stale(pending: &str, known: Option<&str>) -> bool {
    known.is_some_and(|known| known != pending)
}

/// How far a download that stopped got. The files are counted from the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PausedProgress {
    pub files_done: usize,
    /// Zero when the marker does not say (written by an older release).
    pub files_total: usize,
}

impl PausedProgress {
    /// The status line, such as `Update paused · 98 of 277 files`.
    pub fn label(&self) -> String {
        if self.files_total == 0 {
            "Update paused".to_string()
        } else {
            format!(
                "Update paused · {} of {} files",
                self.files_done.min(self.files_total),
                self.files_total
            )
        }
    }
}

/// A newer library that a check found. The manifest is kept for this session so Download needs
/// no second fetch; after a reload only the version and size are known until the next check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryUpdate {
    pub version: String,
    pub bytes: u64,
    pub manifest: Option<ContentManifest>,
    /// Set when an earlier session started downloading this version and stopped. Nothing
    /// downloads until the reader taps Resume.
    pub paused: Option<PausedProgress>,
    /// One-shot: the reader tapped Resume, so the installer panel starts by itself on mount.
    pub resume: bool,
}

impl LibraryUpdate {
    fn found(manifest: &ContentManifest) -> Self {
        Self {
            version: manifest.content_version.clone(),
            bytes: manifest.total_bytes,
            manifest: Some(manifest.clone()),
            paused: None,
            resume: false,
        }
    }
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
    /// A check is running. Guards against a start event and a visibility event, or a manual
    /// check, from asking the network twice at once.
    checking: RwSignal<bool>,
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
            checking: RwSignal::new(false),
            active_version: RwSignal::new(active.content_version),
            online: app.online,
        };
        spawn_local(async move {
            let record = store::update_check().await.ok().flatten();
            ctx.auto.set(store::auto_update_check_enabled().await);
            if let Some(record) = &record {
                ctx.last_checked.set(Some(record.last_checked));
                ctx.show_known(record);
            }
            ctx.restore_paused(record.as_ref()).await;
            ctx.maybe_check();
        });
        ctx.follow_visibility();
        ctx
    }

    /// Runs the automatic check when the rule allows it. Called at app start and whenever the
    /// document becomes visible again. The flag is set before the task starts, so two events in
    /// one turn cannot both spawn a fetch.
    pub fn maybe_check(self) {
        if self.checking.get_untracked() {
            return;
        }
        if should_check(
            js::now_ms(),
            self.last_checked.get_untracked(),
            self.online.get_untracked(),
            self.auto.get_untracked(),
        ) {
            self.checking.set(true);
            spawn_local(async move {
                let _ = self.run_check().await;
                self.checking.set(false);
            });
        }
    }

    /// The explicit check. It shares the in-flight guard with the automatic one.
    pub async fn check_now(self) -> Result<UpdateCheck, String> {
        if self.checking.get_untracked() {
            return Err("A check is already running.".to_string());
        }
        self.checking.set(true);
        let result = self.run_check().await;
        self.checking.set(false);
        result
    }

    /// Asks for the app's own manifest and records the answer. Never downloads the library.
    /// Errors are returned for an explicit check to show; the automatic caller ignores them.
    async fn run_check(self) -> Result<UpdateCheck, String> {
        let active_version = self.active_version.get_untracked();
        let checked_at = js::now_ms();
        let result = install::check_for_update(&active_version).await;
        // The rate limit counts the attempt, so a failing server is not asked on every focus.
        self.last_checked.set(Some(checked_at));
        let record = match &result {
            Ok(UpdateCheck::Available(manifest)) => {
                let mut found = LibraryUpdate::found(manifest);
                if let Some(paused) = self.library_update.get_untracked().and_then(|previous| {
                    if previous.version == manifest.content_version {
                        previous.paused
                    } else {
                        discard_paused(&previous);
                        None
                    }
                }) {
                    found.paused = Some(paused);
                }
                self.library_update.set(Some(found));
                UpdateCheckRecord {
                    last_checked: checked_at,
                    known_version: Some(manifest.content_version.clone()),
                    known_bytes: manifest.total_bytes,
                }
            }
            Ok(UpdateCheck::UpToDate) => {
                if let Some(previous) = self.library_update.get_untracked() {
                    discard_paused(&previous);
                }
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
            paused: None,
            resume: false,
        }));
    }

    /// Shows a download that an earlier session started and left unfinished, as a paused update.
    /// It only reads: the cache's key list and the marker. Nothing is fetched and nothing
    /// downloads until the reader taps Resume (AGENTS.md rule 1). A marker for a version that
    /// the last check no longer offers is dropped with its partial cache.
    async fn restore_paused(self, record: Option<&UpdateCheckRecord>) {
        let Ok(Some(pending)) = store::pending_install().await else {
            return;
        };
        let version = pending.content_version;
        if version == self.active_version.get_untracked() {
            return;
        }
        let known = record.and_then(|record| record.known_version.as_deref());
        if pending_is_stale(&version, known) {
            install::discard_pending(&version).await;
            return;
        }
        let paused = PausedProgress {
            files_done: install::cached_file_count(&version).await,
            files_total: pending.files_total as usize,
        };
        self.library_update.update(|slot| match slot {
            Some(update) if update.version == version => update.paused = Some(paused),
            _ => {
                *slot = Some(LibraryUpdate {
                    version,
                    bytes: record.map_or(0, |record| record.known_bytes),
                    manifest: None,
                    paused: Some(paused),
                    resume: false,
                });
            }
        });
    }

    /// Resume, for a paused update. It reads the app's own manifest if this session has none
    /// (the reader asked for it), then hands the download to the installer panel, which scans the
    /// cache and fetches only what is missing. When the manifest names a different newest
    /// version, the paused one is stale: its partial cache is dropped and the newest is offered
    /// instead, without downloading anything.
    pub async fn resume(self) -> Result<(), String> {
        let Some(update) = self.library_update.get_untracked() else {
            return Ok(());
        };
        let manifest = match &update.manifest {
            Some(manifest) => manifest.clone(),
            None => install::fetch_manifest()
                .await
                .map_err(|failure| failure.message)?,
        };
        if manifest.content_version == update.version {
            let mut found = LibraryUpdate::found(&manifest);
            found.resume = true;
            self.library_update.set(Some(found));
            return Ok(());
        }
        install::discard_pending(&update.version).await;
        let checked_at = js::now_ms();
        self.last_checked.set(Some(checked_at));
        let active = self.active_version.get_untracked();
        let up_to_date = manifest.content_version == active;
        self.library_update
            .set((!up_to_date).then(|| LibraryUpdate::found(&manifest)));
        let _ = store::set_update_check(&UpdateCheckRecord {
            last_checked: checked_at,
            known_version: Some(manifest.content_version.clone()),
            known_bytes: if up_to_date { 0 } else { manifest.total_bytes },
        })
        .await;
        Ok(())
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

/// Drops the partial download of a paused update that is no longer the newest. An update that is
/// not paused has no start from an earlier session, and an install of this session may be running
/// on it, so it is left alone.
fn discard_paused(previous: &LibraryUpdate) {
    if previous.paused.is_some() {
        let version = previous.version.clone();
        spawn_local(async move { install::discard_pending(&version).await });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_is_stale_only_when_the_newest_version_is_another() {
        assert!(!pending_is_stale("v2", Some("v2")));
        assert!(pending_is_stale("v2", Some("v3")));
        // The active version is the newest: nothing newer is offered, so the marker is stale.
        assert!(pending_is_stale("v2", Some("v1")));
        // Nothing is known: keep it, and let Resume find out.
        assert!(!pending_is_stale("v2", None));
    }

    #[test]
    fn the_paused_line_counts_files_and_never_overstates() {
        let line = |files_done, files_total| {
            PausedProgress {
                files_done,
                files_total,
            }
            .label()
        };
        assert_eq!(line(98, 277), "Update paused · 98 of 277 files");
        assert_eq!(line(0, 277), "Update paused · 0 of 277 files");
        assert_eq!(line(300, 277), "Update paused · 277 of 277 files");
        assert_eq!(line(5, 0), "Update paused");
    }

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
