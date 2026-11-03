//! Settings: theme, library, storage, app version, updates, verification, and a link to Legal.

use crate::app::use_app;
use crate::fmt;
use crate::install::{
    self, UpdateCheck,
    plan::{Phase, Progress},
};
use crate::js::{self, StorageEstimate};
use crate::library::use_library;
use crate::route::Route;
use crate::store::ActiveLibrary;
use crate::theme::{self, Theme};
use crate::views::installer::{InstallMode, InstallPanel};
use cast_iron_schema::ContentManifest;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;

#[derive(Clone)]
enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(ContentManifest),
    Failed(String),
}

#[derive(Clone)]
enum VerifyState {
    Idle,
    Running(Progress),
    Passed(install::VerifyReport),
    Failed(String),
}

fn format_date(ms: f64) -> String {
    let date = js_sys::Date::new(&JsValue::from_f64(ms));
    String::from(date.to_locale_string("en-GB", &JsValue::UNDEFINED))
}

#[component]
pub fn Settings() -> impl IntoView {
    let app = use_app();
    let library = use_library();
    let active = library.active().clone();

    let estimate = RwSignal::new(None::<StorageEstimate>);
    let persisted = RwSignal::new(None::<bool>);
    let refresh_storage = move || {
        spawn_local(async move {
            estimate.set(Some(js::storage_estimate().await));
            persisted.set(Some(js::storage_persisted().await));
        });
    };
    refresh_storage();

    let shell = RwSignal::new(None::<Option<String>>);
    spawn_local(async move { shell.set(Some(js::shell_version().await)) });

    let update = RwSignal::new(UpdateState::Idle);
    let verify = RwSignal::new(VerifyState::Idle);

    let check_update = {
        let active = active.clone();
        move |_| {
            let active = active.clone();
            update.set(UpdateState::Checking);
            spawn_local(async move {
                // Also ask for a newer app shell. A failure here changes nothing the user can act on.
                let _ = js::check_shell_update().await;
                update.set(match install::check_for_update(&active).await {
                    Ok(UpdateCheck::UpToDate) => UpdateState::UpToDate,
                    Ok(UpdateCheck::Available(manifest)) => UpdateState::Available(manifest),
                    Err(failure) => UpdateState::Failed(failure.message),
                });
            });
        }
    };

    let run_verify = {
        let library = library.clone();
        move |_| {
            let manifest = library.manifest().clone();
            verify.set(VerifyState::Running(Progress::default()));
            spawn_local(async move {
                let report =
                    move |_: Phase, progress: Progress| verify.set(VerifyState::Running(progress));
                verify.set(match install::verify_installed(&manifest, &report).await {
                    Ok(done) => VerifyState::Passed(done),
                    Err(failure) => VerifyState::Failed(failure.message),
                });
            });
        }
    };

    let on_update_done = Callback::new(move |active: ActiveLibrary| app.activate(active));

    view! {
        <h1>"Settings"</h1>

        <section class="settings-section" aria-labelledby="set-theme">
            <h2 id="set-theme">"Appearance"</h2>
            <fieldset class="choice">
                <legend>"Theme"</legend>
                {Theme::ALL.into_iter().map(|choice| view! {
                    <label class="choice-item">
                        <input
                            type="radio"
                            name="theme"
                            value=choice.slug()
                            prop:checked=move || app.theme.get() == choice
                            on:change=move |_| {
                                app.theme.set(choice);
                                spawn_local(theme::set(choice));
                            }
                        />
                        <span>{choice.label()}</span>
                    </label>
                }).collect_view()}
            </fieldset>
        </section>

        <section class="settings-section" aria-labelledby="set-library">
            <h2 id="set-library">"Library"</h2>
            <dl class="facts" data-testid="library-facts">
                <div><dt>"Version"</dt><dd data-testid="library-version">{active.content_version.clone()}</dd></div>
                <div><dt>"Entries"</dt><dd data-testid="library-entries">{active.entry_count}</dd></div>
                <div><dt>"Size on this device"</dt><dd data-testid="library-size">{fmt::bytes(active.total_bytes)}</dd></div>
                <div><dt>"Content format"</dt><dd>{active.schema_version}</dd></div>
                <div><dt>"Installed"</dt><dd>{format_date(active.activated_at)}</dd></div>
            </dl>
            <p>
                <button class="btn" type="button" on:click=run_verify
                    prop:disabled=move || matches!(verify.get(), VerifyState::Running(_))>
                    "Verify library"
                </button>
            </p>
            <div role="status" aria-live="polite" data-testid="verify-status">
                {move || match verify.get() {
                    VerifyState::Idle => String::new(),
                    VerifyState::Running(p) => format!("Verifying: {}", p.summary()),
                    VerifyState::Passed(done) => format!(
                        "All {} files verified against their checksums ({}).",
                        done.files,
                        fmt::bytes(done.bytes)
                    ),
                    VerifyState::Failed(message) => format!("Verification failed: {message}"),
                }}
            </div>
            <p><a href=Route::Legal.href()>"Legal and attribution, and library coverage"</a></p>
        </section>

        <section class="settings-section" aria-labelledby="set-updates">
            <h2 id="set-updates">"Updates"</h2>
            <p>"Library updates download when you ask. App updates become ready in the background; reload to apply them. Your bookmarks and collections stay."</p>
            <p>
                <button class="btn" type="button" on:click=check_update
                    prop:disabled=move || !app.online.get() || matches!(update.get(), UpdateState::Checking)>
                    "Check for updates"
                </button>
            </p>
            <Show when=move || !app.online.get()>
                <p class="muted">"You are offline. Connect to check for updates."</p>
            </Show>
            <div role="status" aria-live="polite" data-testid="update-status">
                {move || match update.get() {
                    UpdateState::Idle => String::new(),
                    UpdateState::Checking => "Checking...".to_string(),
                    UpdateState::UpToDate => "The library is up to date.".to_string(),
                    UpdateState::Available(_) => String::new(),
                    UpdateState::Failed(message) => message,
                }}
            </div>
            {move || match update.get() {
                UpdateState::Available(manifest) => view! {
                    <InstallPanel mode=InstallMode::Update manifest on_done=on_update_done />
                }.into_any(),
                _ => ().into_any(),
            }}
            <Show when=move || app.shell_update_ready.get()>
                <p class="banner-inline">
                    "App update ready. "
                    <button class="btn" type="button" on:click=|_| js::apply_shell_update()>"Reload"</button>
                </p>
            </Show>
        </section>

        <section class="settings-section" aria-labelledby="set-storage">
            <h2 id="set-storage">"Storage"</h2>
            <dl class="facts">
                <div><dt>"Used"</dt><dd data-testid="storage-usage">{move || match estimate.get() {
                    None => "Checking...".to_string(),
                    Some(e) => e.usage.map_or("Not reported".to_string(), |u| fmt::bytes(u as u64)),
                }}</dd></div>
                <div><dt>"Available to this app"</dt><dd data-testid="storage-quota">{move || match estimate.get() {
                    None => "Checking...".to_string(),
                    Some(e) => e.quota.map_or("Not reported".to_string(), |q| fmt::bytes(q as u64)),
                }}</dd></div>
                <div><dt>"Persistent storage"</dt><dd data-testid="storage-persisted">{move || match persisted.get() {
                    None => "Checking...".to_string(),
                    Some(true) => "Granted. The browser will not clear the library on its own.".to_string(),
                    Some(false) => "Not granted. The browser may clear the library if the device runs low on space.".to_string(),
                }}</dd></div>
            </dl>
            <Show when=move || persisted.get() == Some(false)>
                <p>
                    <button class="btn" type="button" on:click=move |_| {
                        spawn_local(async move {
                            let _ = js::storage_persist().await;
                            refresh_storage();
                        });
                    }>"Request persistent storage"</button>
                </p>
            </Show>
        </section>

        <section class="settings-section" aria-labelledby="set-app">
            <h2 id="set-app">"App"</h2>
            <dl class="facts">
                <div><dt>"App version"</dt><dd data-testid="shell-version">{move || match shell.get() {
                    None => "Checking...".to_string(),
                    Some(Some(version)) => version,
                    Some(None) => "Not available until the app has been reloaded once".to_string(),
                }}</dd></div>
            </dl>
        </section>
    }
}
