//! Settings: theme, library, storage, app version, updates, verification, and a link to Legal.

use crate::app::use_app;
use crate::fmt;
use crate::install::{
    self, UpdateCheck,
    plan::{Phase, Progress},
};
use crate::js::{self, StorageEstimate};
use crate::library::use_library;
use crate::pwa::use_install;
use crate::route::{self, Route};
use crate::store::ActiveLibrary;
use crate::theme::{self, Theme};
use crate::updates::use_updates;
use crate::views::installer::{InstallMode, InstallPanel};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, JsValue};

/// Opens Settings on its Updates section, without reloading the app. A header action must not
/// apply an update by surprise; it brings the reader to the controls that do.
pub fn open_updates() {
    // On Settings already, the route does not change, so the screen is not rebuilt and nothing
    // would move. Scroll and focus here instead.
    if Route::parse(&route::current_hash()) == Route::Settings {
        focus_updates();
        return;
    }
    route::navigate_hash(route::settings_updates_href());
}

/// Brings the Updates section into view and moves focus to it.
fn focus_updates() {
    let Some(section) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(route::UPDATES_SECTION_ID))
    else {
        return;
    };
    section.scroll_into_view();
    if let Ok(section) = section.dyn_into::<web_sys::HtmlElement>() {
        let _ = section.focus();
    }
}

#[derive(Clone)]
enum UpdateState {
    Idle,
    Checking,
    UpToDate,
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
    let install_ctx = use_install();
    let library = use_library();
    let updates = use_updates();
    let active = library.active().clone();

    // The header's update action asks for the Updates section. The Reader's route effect scrolls
    // to the top and focuses <main> one animation frame later, so wait two frames to land on the
    // section instead.
    Effect::new(move |_| {
        if route::wants_updates_section(&route::current_hash()) {
            request_animation_frame(|| request_animation_frame(focus_updates));
        }
    });

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

    // Settings moves the Updates section to the top while something waits. That remounts it, so
    // focus falls to <body>; remember whether it was inside and put it back on the moved section.
    let focus_inside_updates = RwSignal::new(false);
    let focus_handle = window_event_listener_untyped("focusin", move |event| {
        let inside = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|element| element.closest("#set-updates").ok().flatten())
            .is_some();
        focus_inside_updates.set(inside);
    });
    on_cleanup(move || focus_handle.remove());

    let verify = RwSignal::new(VerifyState::Idle);

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

    // The conversion is retried on the next load, so this is a notice, not an error.
    let conversion_pending = library.migration_refused();
    // Updates come first while something waits, so the reader sees the action without scrolling.
    let waiting =
        Memo::new(move |_| app.shell_update_ready.get() || updates.library_update.get().is_some());
    Effect::new(move |previous: Option<bool>| {
        let now = waiting.get();
        if previous == Some(false) && now && focus_inside_updates.get_untracked() {
            request_animation_frame(|| request_animation_frame(focus_updates));
        }
        now
    });

    view! {
        <h1>"Settings"</h1>

        {move || waiting.get().then(|| view! { <UpdatesSection /> })}

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
                <div><dt>"Entries"</dt><dd data-testid="library-entries">{fmt::count(active.entry_count)}</dd></div>
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
            {conversion_pending.then(|| view! {
                <p class="notice" role="status" data-testid="migration-notice">
                    "Saved items will be converted when storage is available."
                </p>
            })}
            <p><a href=Route::Legal.href()>"Legal and attribution, and library coverage"</a></p>
        </section>

        {move || (!waiting.get()).then(|| view! { <UpdatesSection /> })}

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
            {move || if install_ctx.standalone() {
                view! {
                    <p data-testid="app-install">
                        <span class="badge">"Installed"</span>
                        " Cast Iron is running as an installed app."
                    </p>
                }.into_any()
            } else if install_ctx.offer_in_settings() {
                view! {
                    <div class="settings-install" data-testid="app-install">
                        <p>"Install Cast Iron as an app for full-screen reading and an icon on your device."</p>
                        <p>
                            <button class="btn" type="button" on:click=move |event| {
                                let opener = event
                                    .current_target()
                                    .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok());
                                install_ctx.request(opener);
                            }>"Install app"</button>
                        </p>
                    </div>
                }.into_any()
            } else {
                view! {
                    <p class="muted" data-testid="app-install">
                        "Your browser's menu may offer Install or Add to Home Screen."
                    </p>
                }.into_any()
            }}
        </section>
    }
}

/// The Updates section. It lists what is waiting, each with the control that applies it, then
/// the automatic-check switch and the explicit check. Settings renders it first while an update
/// waits, and in its usual place otherwise.
#[component]
fn UpdatesSection() -> impl IntoView {
    let app = use_app();
    let updates = use_updates();
    let status = RwSignal::new(UpdateState::Idle);
    let on_done = Callback::new(move |active: ActiveLibrary| app.activate(active));

    let check_now = Callback::new(move |_: ()| {
        status.set(UpdateState::Checking);
        spawn_local(async move {
            // Also ask for a newer app shell. A failure here changes nothing the user can act on.
            let _ = js::check_shell_update().await;
            status.set(match updates.check_now().await {
                Ok(UpdateCheck::UpToDate) => UpdateState::UpToDate,
                Ok(UpdateCheck::Available(_)) => UpdateState::Idle,
                Err(message) => UpdateState::Failed(message),
            });
        });
    });

    view! {
        <section class="settings-section" id="set-updates" tabindex="-1" aria-labelledby="set-updates-title">
            <h2 id="set-updates-title">"Updates"</h2>

            {move || {
                let Some(found) = updates.library_update.get() else {
                    return ().into_any();
                };
                let manifest = found.manifest.clone();
                let size = (found.bytes > 0).then(|| format!(" ({})", fmt::bytes(found.bytes)));
                let name = format!("{}{}", found.version, size.unwrap_or_default());
                let online = app.online.get();
                view! {
                    <div class="update-row" data-testid="library-update">
                        <p><strong>"Newer rules library: "</strong>{name}</p>
                        <p class="muted">"Your stars and book choices are kept."</p>
                        // Only a manifest from this session may mount the panel: the panel fetches
                        // one on mount, which would bypass the switch and the daily budget.
                        {match (manifest, online) {
                            (Some(manifest), true) => view! {
                                <InstallPanel mode=InstallMode::Update manifest on_done />
                            }.into_any(),
                            (Some(_), false) => view! {
                                <p class="muted">"Connect to download it."</p>
                            }.into_any(),
                            (None, _) => view! {
                                <p class="update-prompt">
                                    <button class="btn" type="button" on:click=move |_| check_now.run(())
                                        prop:disabled=move || !app.online.get()>
                                        "Check now"
                                    </button>
                                    <span class="muted">"This app reads the file list from the manifest first."</span>
                                </p>
                            }.into_any(),
                        }}
                    </div>
                }.into_any()
            }}

            {move || app.shell_update_ready.get().then(|| view! {
                <p class="banner-inline" data-testid="app-update">
                    "App update ready. "
                    <button class="btn" type="button" on:click=|_| js::apply_shell_update()>"Reload"</button>
                </p>
            })}

            <fieldset class="choice update-auto">
                <legend>"Automatic checks"</legend>
                <label class="choice-item">
                    <input
                        type="checkbox"
                        prop:checked=move || updates.auto.get()
                        on:change=move |event| {
                            let enabled = event
                                .target()
                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                .is_some_and(|input| input.checked());
                            updates.set_auto(enabled);
                        }
                    />
                    <span>"Check for updates automatically"</span>
                </label>
            </fieldset>
            <p class="muted">
                "Once a day when online, reading only this app's own manifest. The library never downloads without you."
            </p>

            <p class="update-check">
                <button class="btn" type="button" on:click=move |_| check_now.run(())
                    prop:disabled=move || !app.online.get() || matches!(status.get(), UpdateState::Checking)>
                    "Check now"
                </button>
                <span class="muted" data-testid="last-checked">
                    {move || match updates.last_checked.get() {
                        Some(ms) => format!("Last checked: {}", format_date(ms)),
                        None => "Last checked: not yet".to_string(),
                    }}
                </span>
            </p>
            <Show when=move || !app.online.get()>
                <p class="muted">"You are offline. Connect to check for updates."</p>
            </Show>
            <div role="status" aria-live="polite" data-testid="update-status">
                {move || match status.get() {
                    UpdateState::Idle => String::new(),
                    UpdateState::Checking => "Checking...".to_string(),
                    UpdateState::UpToDate => "The library is up to date.".to_string(),
                    UpdateState::Failed(message) => message,
                }}
            </div>
        </section>
    }
}
