//! Settings, Diagnostics (#423): the timings, storage and memory that this device recorded, as
//! plain text. The text on screen is the text that Copy puts on the clipboard. Nothing is sent
//! anywhere (rule 1).

use crate::diagnostics::{self, LibraryFacts, Report, Snapshot, StorageFacts};
use crate::fmt;
use crate::js;
use crate::library::{Library, use_library};
use crate::route::Route;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Reads everything the text needs. It also stores this launch, so the launch has its number.
async fn collect(library: &Library) -> Report {
    diagnostics::persist().await;
    let stored = diagnostics::load().await;
    let launch = diagnostics::current();
    let estimate = js::storage_estimate().await;
    let persisted = js::storage_persisted().await;
    let shell = js::shell_info().await;
    let window = web_sys::window();
    let manifest = library.manifest();
    let active = library.active();
    let snapshot = Snapshot {
        made_at: js_sys::Date::new_0().to_iso_string().into(),
        app: shell.map_or_else(
            || "not available until the app has been reloaded once".to_string(),
            |info| fmt::app_label(info.date.as_deref(), &info.version),
        ),
        library: LibraryFacts {
            label: fmt::corpus_label(manifest.corpus_tag.as_deref(), &active.content_version),
            content_version: active.content_version.clone(),
            schema_version: active.schema_version,
            entries: u64::from(manifest.entry_count),
            files: manifest.files.len(),
            bytes: manifest.total_bytes,
        },
        device: window
            .as_ref()
            .and_then(|w| w.navigator().user_agent().ok())
            .unwrap_or_default(),
        online_now: window.as_ref().is_none_or(|w| w.navigator().on_line()),
        storage: StorageFacts {
            usage: estimate.usage.map(|v| v as u64),
            quota: estimate.quota.map(|v| v as u64),
            persisted: Some(persisted),
        },
        memory: diagnostics::js_heap(),
        install: stored.install.clone(),
        earlier: diagnostics::earlier_launches(&stored, &launch),
        launch,
        typing_pause_ms: crate::views::search::DEBOUNCE_MS,
    };
    diagnostics::report(&snapshot)
}

#[component]
pub fn Diagnostics() -> impl IntoView {
    let library = use_library();
    let report = RwSignal::new(None::<Report>);
    let status = RwSignal::new(String::new());

    let refresh = {
        let library = library.clone();
        move || {
            let library = library.clone();
            spawn_local(async move {
                report.try_set(Some(collect(&library).await));
            });
        }
    };
    refresh();

    let copy = move |_| {
        let Some(text) = report.get_untracked().map(|r| r.text()) else {
            return;
        };
        status.set(String::new());
        spawn_local(async move {
            let line = match js::copy_text(&text).await {
                Ok(()) => "Copied the results.".to_string(),
                Err(error) => format!(
                    "Could not copy: {error} Press and hold the text below to select it, then copy it."
                ),
            };
            status.try_set(line);
        });
    };

    view! {
        <h1>"Diagnostics"</h1>
        <p>
            "What this device recorded about Cast Iron: how long it took to start, search and open pages, "
            "and how much storage and memory it uses. Nothing is sent anywhere. "
            "Copy results puts the text below on your clipboard, so you can paste it into a report."
        </p>
        <p class="actions">
            <button class="btn btn-primary" type="button" on:click=copy
                prop:disabled=move || report.get().is_none()>
                "Copy results"
            </button>
            <button class="btn" type="button" on:click=move |_| refresh()>"Refresh"</button>
        </p>
        <div role="status" aria-live="polite" data-testid="diagnostics-status">{move || status.get()}</div>
        {move || match report.get() {
            None => view! { <p class="muted" role="status">"Collecting..."</p> }.into_any(),
            Some(report) => report
                .sections
                .into_iter()
                .map(|section| view! {
                    <section class="settings-section diagnostics-section" aria-label=section.title>
                        <h2>{section.title}</h2>
                        <pre data-testid="diagnostics-text" tabindex="0">{section.lines.join("\n")}</pre>
                    </section>
                })
                .collect_view()
                .into_any(),
        }}
        <p><a href=Route::Settings.href()>"Back to Settings"</a></p>
    }
}
