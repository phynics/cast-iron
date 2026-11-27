//! Settings (#584, #747): seven sections in a fixed order. "Version & updates" comes first and
//! never moves.

use crate::app::use_app;
use crate::book_presets;
use crate::density::{self, Density};
use crate::fmt;
use crate::install::{
    self, UpdateCheck,
    plan::{Phase, Progress},
};
use crate::js::{self, StorageEstimate};
use crate::library::use_library;
use crate::pwa::use_install;
use crate::report;
use crate::route::{self, Route};
use crate::store::ActiveLibrary;
use crate::theme::{self, Theme};
use crate::updates::use_updates;
use crate::views::icons;
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
    let iso = format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    );
    fmt::day_month_year(&iso).unwrap_or(iso)
}

const PERSISTED_GRANTED: &str = "Granted. The browser will not clear the library on its own.";
/// The longest of the row's texts. The row reserves its height, so the answer cannot move the page.
const PERSISTED_DENIED: &str =
    "Not granted. The browser may clear the library if the device runs low on space.";

/// What the request answered. The refusal says what to do next, because the browser gives no reason.
const PERSIST_REQUEST_REFUSED: &str =
    "Not granted. On iPhone, add Cast Iron to the Home Screen; other browsers decide on their own.";
const STORAGE_STATUS_ID: &str = "storage-request-status";

/// Moves focus to the storage status line, for when the request button has just gone away.
fn focus_storage_status() {
    let Some(line) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(STORAGE_STATUS_ID))
    else {
        return;
    };
    if let Ok(line) = line.dyn_into::<web_sys::HtmlElement>() {
        let _ = line.focus();
    }
}

#[component]
pub fn Settings() -> impl IntoView {
    let app = use_app();
    let install_ctx = use_install();
    let library = use_library();
    let active = library.active().clone();

    // The header's update action asks for the Updates section. The Reader's route effect scrolls
    // to the top and focuses <main> one animation frame later, so wait two frames to land on the
    // section instead.
    crate::views::util::effect(move || {
        if route::wants_updates_section(&route::current_hash()) {
            request_animation_frame(|| request_animation_frame(focus_updates));
        }
    });

    let estimate = RwSignal::new(None::<StorageEstimate>);
    let persisted = RwSignal::new(None::<bool>);
    // The answer to the last "Request persistent storage" press; None until pressed.
    let persist_answer = RwSignal::new(None::<bool>);
    let refresh_storage = move || {
        spawn_local(async move {
            estimate.set(Some(js::storage_estimate().await));
            persisted.set(Some(js::storage_persisted().await));
        });
    };
    refresh_storage();

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

    // The same link as the footer's, for this page.
    let report_link = {
        let library = library.clone();
        Memo::new(move |_| {
            let active = library.active();
            report::for_route(
                &Route::Settings,
                None,
                app.app_version.get().as_deref().unwrap_or_default(),
                &active.content_version,
                active.schema_version,
            )
        })
    };

    // The Books row repeats the count line of the Books page.
    let books_revision = library.selection_revision();
    let books_library = library.clone();
    let books_line = move || {
        books_revision.get();
        let sources = books_library.sources();
        let active = sources
            .iter()
            .filter(|source| books_library.book_active(&source.id))
            .count();
        let books = book_presets::books_of(sources);
        let preset = book_presets::matching(&books, |id| books_library.book_active(id));
        book_presets::count_line(
            active,
            sources.len(),
            Some(book_presets::name_or_custom(preset)),
        )
    };

    view! {
        <h1>"Settings"</h1>

        <VersionSection />

        <section class="settings-section" aria-labelledby="set-appearance">
            <h2 id="set-appearance">"Appearance"</h2>
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
            <fieldset class="choice">
                <legend>"Spell row density"</legend>
                {Density::ALL.into_iter().map(|choice| view! {
                    <label class="choice-item">
                        <input
                            type="radio"
                            name=density::SETTINGS_RADIO_GROUP
                            value=choice.slug()
                            prop:checked=move || app.density.get() == choice
                            on:change=move |_| {
                                app.density.set(choice);
                                spawn_local(density::set(choice));
                            }
                        />
                        <span>{choice.label()}</span>
                    </label>
                }).collect_view()}
            </fieldset>
        </section>

        <section class="settings-section" aria-labelledby="set-books">
            <h2 id="set-books">"Books"</h2>
            <dl class="facts">
                <div><dt>"Active"</dt><dd data-testid="books-active">{books_line}</dd></div>
            </dl>
            <p>
                <a class="page-link" href=Route::books_manage().href()>
                    <span>"Manage books"</span>
                    {icons::chevron_right_large()}
                </a>
            </p>
        </section>

        <section class="settings-section" aria-labelledby="set-library">
            <h2 id="set-library">"Library"</h2>
            <dl class="facts">
                <div><dt>"Entries"</dt><dd data-testid="library-entries">{fmt::count(active.entry_count)}</dd></div>
                <div><dt>"Installed"</dt><dd>{format_date(active.activated_at)}</dd></div>
            </dl>
            <p>
                <button class="btn" type="button" on:click=run_verify
                    prop:disabled=move || matches!(verify.get(), VerifyState::Running(_))>
                    "Verify library"
                </button>
            </p>
            <p class="status" role="status" aria-live="polite" data-testid="verify-status">
                {move || match verify.get() {
                    VerifyState::Idle => String::new(),
                    VerifyState::Running(p) => format!("Verifying… {}", p.summary()),
                    VerifyState::Passed(done) => format!(
                        "Library verified. All {} files match their checksums ({}).",
                        done.files,
                        fmt::bytes(done.bytes)
                    ),
                    VerifyState::Failed(message) => format!("Verification failed: {message}"),
                }}
            </p>
            {conversion_pending.then(|| view! {
                <p class="notice" role="status" data-testid="migration-notice">
                    "Saved items will be converted when storage is available."
                </p>
            })}
        </section>

        <section class="settings-section" aria-labelledby="set-install">
            <h2 id="set-install">"Install"</h2>
            {move || if install_ctx.standalone() {
                view! {
                    <dl class="facts" data-testid="app-install">
                        <div><dt>"Installed as an app"</dt><dd>"Yes"</dd></div>
                    </dl>
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

        <section class="settings-section" aria-labelledby="set-storage">
            <h2 id="set-storage">"Storage"</h2>
            <dl class="facts">
                <div><dt>"Used"</dt><dd data-testid="storage-usage">{move || match estimate.get() {
                    None => "Checking…".to_string(),
                    Some(e) => e.usage.map_or("Not reported".to_string(), |u| fmt::bytes(u as u64)),
                }}</dd></div>
                <div><dt>"Available to this app"</dt><dd data-testid="storage-quota">{move || match estimate.get() {
                    None => "Checking…".to_string(),
                    Some(e) => e.quota.map_or("Not reported".to_string(), |q| fmt::bytes(q as u64)),
                }}</dd></div>
                <div><dt>"Persistent storage"</dt><dd class="reserve-text" data-testid="storage-persisted" data-reserve=PERSISTED_DENIED><span>{move || match persisted.get() {
                    None => "Checking…".to_string(),
                    Some(true) => PERSISTED_GRANTED.to_string(),
                    Some(false) => PERSISTED_DENIED.to_string(),
                }}</span></dd></div>
            </dl>
            <Show when=move || persisted.get() == Some(false)>
                <p>
                    <button class="btn" type="button" on:click=move |_| {
                        spawn_local(async move {
                            let granted = js::storage_persist().await;
                            persist_answer.set(Some(granted));
                            refresh_storage();
                            if granted {
                                // The button goes with the grant; keep focus on the page.
                                request_animation_frame(focus_storage_status);
                            }
                        });
                    }>"Request persistent storage"</button>
                </p>
            </Show>
            <div role="status" id=STORAGE_STATUS_ID tabindex="-1" data-testid="storage-request-status">
                {move || persist_answer.get().map(|granted| view! {
                    <p class="muted">{if granted { PERSISTED_GRANTED } else { PERSIST_REQUEST_REFUSED }}</p>
                })}
            </div>
        </section>

        <section class="settings-section" aria-labelledby="set-help">
            <h2 id="set-help">"Help"</h2>
            <p>
                <a class="page-link" href=Route::Diagnostics.href()>
                    <span>"Diagnostics"</span>
                    {icons::chevron_right_large()}
                </a>
            </p>
            <p>
                <a
                    class="page-link"
                    href=move || report_link.get().url
                    target="_blank"
                    rel="noopener noreferrer"
                    aria-label=move || report_link.get().label
                >
                    <span>"Report a problem"</span>
                    {icons::chevron_right_large()}
                </a>
            </p>
            <p>
                <a class="page-link" href=Route::Legal.href()>
                    <span>"Legal and attribution"</span>
                    {icons::chevron_right_large()}
                </a>
            </p>
        </section>
    }
}

/// What Resume is doing. The reader's tap fetches the manifest when this session has none.
#[derive(Clone)]
enum ResumeState {
    Idle,
    Working,
    Failed(String),
}

/// "Version & updates", always first: the app and the rules library that are installed, what
/// waits (one row each, with the control that applies it), the automatic-check switch, and the
/// explicit check.
#[component]
fn VersionSection() -> impl IntoView {
    let app = use_app();
    let updates = use_updates();
    let library = use_library();
    let active = library.active().clone();
    let corpus_label = fmt::corpus_label(
        library.manifest().corpus_tag.as_deref(),
        &active.content_version,
    );
    let book_count = library.sources().len();
    let shell = RwSignal::new(None::<Option<js::ShellInfo>>);
    spawn_local(async move { shell.set(Some(js::shell_info().await)) });
    let status = RwSignal::new(UpdateState::Idle);
    let resume = RwSignal::new(ResumeState::Idle);
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

    let resume_download = move |_| {
        resume.set(ResumeState::Working);
        spawn_local(async move {
            resume.set(match updates.resume().await {
                Ok(()) => ResumeState::Idle,
                Err(message) => ResumeState::Failed(message),
            });
        });
    };

    view! {
        <section class="settings-section" id="set-updates" tabindex="-1" aria-labelledby="set-updates-title">
            <h2 id="set-updates-title">"Version & updates"</h2>

            <dl class="facts" data-testid="library-facts">
                <div><dt>"App version"</dt><dd data-testid="shell-version">{move || match shell.get() {
                    None => "Checking…".to_string(),
                    Some(Some(info)) => fmt::app_label(info.date.as_deref(), &info.version),
                    Some(None) => "Not available until the app has been reloaded once".to_string(),
                }}</dd></div>
                <div><dt>"Rules library"</dt><dd data-testid="library-version">{corpus_label}</dd></div>
                <div><dt>"Size on this device"</dt><dd data-testid="library-size">{fmt::bytes(active.total_bytes)}</dd></div>
                <div><dt>"Books"</dt><dd data-testid="library-books">{fmt::count(book_count)}</dd></div>
            </dl>

            {move || {
                let Some(found) = updates.library_update.get() else {
                    return ().into_any();
                };
                let manifest = found.manifest.clone();
                // The tag when the manifest is known, otherwise the version that was recorded.
                let name = manifest
                    .as_ref()
                    .and_then(|manifest| manifest.corpus_tag.clone())
                    .unwrap_or_else(|| found.version.clone());
                let value = if found.bytes > 0 {
                    format!("{name} · {}", fmt::bytes(found.bytes))
                } else {
                    name
                };
                let online = app.online.get();
                // A paused update waits for the reader. Nothing downloads, and nothing is
                // fetched, until Resume is tapped.
                if let Some(paused) = found.paused {
                    return view! {
                        <div class="update-row" data-testid="library-update">
                            <dl class="facts">
                                <div><dt>"Update available"</dt><dd>{value}</dd></div>
                            </dl>
                            <p class="status" role="status" data-testid="update-paused">{paused.label()}</p>
                            <p class="update-prompt">
                                <button class="btn btn-primary" type="button" on:click=resume_download
                                    prop:disabled=move || !app.online.get()
                                        || matches!(resume.get(), ResumeState::Working)>
                                    "Resume"
                                </button>
                                <span class="muted">{move || if app.online.get() {
                                    "Files already downloaded are kept.".to_string()
                                } else {
                                    "Connect to resume.".to_string()
                                }}</span>
                            </p>
                            {move || match resume.get() {
                                ResumeState::Failed(message) => view! {
                                    <p class="error" role="alert">{message}</p>
                                }.into_any(),
                                _ => ().into_any(),
                            }}
                        </div>
                    }.into_any();
                }
                // One-shot: a later re-render of this row must not start the download again.
                let autostart = found.resume;
                if autostart {
                    updates.library_update.update_untracked(|slot| {
                        if let Some(slot) = slot {
                            slot.resume = false;
                        }
                    });
                }
                view! {
                    <div class="update-row" data-testid="library-update">
                        <dl class="facts">
                            <div><dt>"Update available"</dt><dd>{value}</dd></div>
                        </dl>
                        // Only a manifest from this session may mount the panel: the panel fetches
                        // one on mount, which would bypass the switch and the daily budget.
                        {match (manifest, online) {
                            (Some(manifest), true) => view! {
                                <InstallPanel mode=InstallMode::Update manifest autostart on_done />
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
                                    <span class="muted">"Check first, then download it."</span>
                                </p>
                            }.into_any(),
                        }}
                    </div>
                }.into_any()
            }}

            {move || app.shell_update_ready.get().then(|| view! {
                <div class="update-row" data-testid="app-update">
                    <dl class="facts">
                        <div><dt>"App"</dt><dd>"Update ready"</dd></div>
                    </dl>
                    <p>
                        <button class="btn" type="button" on:click=|_| js::apply_shell_update()>"Reload"</button>
                    </p>
                </div>
            })}

            <label class="toggle-row">
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
            <p class="muted">
                "Once a day when online, asking only this app's own server. The library never downloads without you."
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
            <p class="status" role="status" aria-live="polite" data-testid="update-status">
                {move || match status.get() {
                    UpdateState::Idle => String::new(),
                    UpdateState::Checking => "Checking…".to_string(),
                    UpdateState::UpToDate => "The library is up to date.".to_string(),
                    UpdateState::Failed(message) => message,
                }}
            </p>
        </section>
    }
}
