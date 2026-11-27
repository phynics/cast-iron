//! The library installer: a reusable panel and the first-run screen around it.

use crate::app::use_app;
use crate::fmt;
use crate::install::{
    self,
    plan::{FailureKind, Phase, Progress},
};
use crate::store::ActiveLibrary;
use cast_iron_schema::ContentManifest;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMode {
    /// No usable library yet.
    Fresh,
    /// A library is active and a newer one is available.
    Update,
}

/// The first-run screen. `notice` explains why the app is here when a library was expected.
#[component]
pub fn InstallerScreen(notice: Option<String>) -> impl IntoView {
    let app = use_app();
    let on_done = Callback::new(move |active: ActiveLibrary| app.activate(active));
    view! {
        <div class="app app-install">
            <header class="app-header">
                <span class="brand">
                    <span class="brand-mark" aria-hidden="true"></span>
                    <span class="sr-only">"Cast Iron"</span>
                </span>
            </header>
            <main id="main" tabindex="-1" class="install-main">
                <h1>"Install the rules library"</h1>
                <p class="lede">
                    "Cast Iron is an offline reference for the Pathfinder First Edition rules. "
                    "It needs to download its library once. After that it works without a connection."
                </p>
                {notice.map(|text| view! { <p class="notice" role="status">{text}</p> })}
                <InstallPanel mode=InstallMode::Fresh on_done />
            </main>
            <footer class="app-footer">
                <p>
                    "Cast Iron is an unofficial fan project. It is not published, endorsed, or approved by Paizo Inc."
                </p>
            </footer>
        </div>
    }
}

/// Fetches the manifest (unless given one), offers the download, runs it, and reports progress.
#[component]
pub fn InstallPanel(
    mode: InstallMode,
    #[prop(optional)] manifest: Option<ContentManifest>,
    // The reader already asked for this download (Resume), so it starts on mount.
    #[prop(optional)] autostart: bool,
    on_done: Callback<ActiveLibrary>,
) -> impl IntoView {
    let phase = RwSignal::new(if manifest.is_some() {
        Phase::Offer
    } else {
        Phase::Checking
    });
    let progress = RwSignal::new(Progress::default());
    let manifest = RwSignal::new(manifest);
    // Pressing Download or Retry unmounts the button that had focus, so the status line (always
    // mounted) takes it, and so does the error on a failure (#12).
    let status_ref = NodeRef::<leptos::html::Div>::new();
    let error_ref = NodeRef::<leptos::html::P>::new();
    Effect::new(move |_| {
        if let Some(error) = error_ref.get() {
            let _ = error.focus();
        }
    });

    let load_manifest = move || {
        phase.set(Phase::Checking);
        spawn_local(async move {
            match install::fetch_manifest().await {
                Ok(found) => {
                    manifest.set(Some(found));
                    phase.set(Phase::Offer);
                }
                Err(failure) => phase.set(Phase::Failed(failure)),
            }
        });
    };
    if manifest.get_untracked().is_none() {
        load_manifest();
    }

    let start = move || {
        if let Some(status) = status_ref.get_untracked() {
            let _ = status.focus();
        }
        let Some(found) = manifest.get_untracked() else {
            load_manifest();
            return;
        };
        phase.set(Phase::Scanning);
        spawn_local(async move {
            let report = move |next: Phase, done: Progress| {
                phase.set(next);
                progress.set(done);
            };
            match install::install(&found, &report).await {
                Ok(active) => {
                    phase.set(Phase::Done);
                    on_done.run(active);
                }
                Err(failure) => phase.set(Phase::Failed(failure)),
            }
        });
    };

    if autostart && manifest.get_untracked().is_some() {
        start();
        // The Resume button went away with the paused row; the status line takes focus.
        request_animation_frame(move || {
            if let Some(status) = status_ref.get_untracked() {
                let _ = status.focus();
            }
        });
    }

    let status = move || match phase.get() {
        Phase::Checking | Phase::Offer | Phase::Failed(_) => String::new(),
        Phase::Done => "The library is installed and ready for offline use.".to_string(),
        other => format!("{}: {}", other.label(), progress.get().summary()),
    };

    // What a screen reader announces: the phase, and the percentage only in tens, so a download
    // of many files says about ten things, not one per progress tick (#12). The memo drops a
    // value equal to the last one.
    let announce = Memo::new(move |_| match phase.get() {
        Phase::Downloading => match progress.get().percent() / 10 * 10 {
            0 | 100 => Phase::Downloading.label().to_string(),
            tens => format!("{}: {tens}%", Phase::Downloading.label()),
        },
        Phase::Done => "The library is installed and ready for offline use.".to_string(),
        Phase::Checking | Phase::Offer | Phase::Failed(_) => String::new(),
        other => other.label().to_string(),
    });

    view! {
        <section class="install-panel" aria-label="Library installer">
            // The facts belong to the first install. In Settings the update row already names the
            // version and the size, so the update form says neither again (#584).
            {move || (mode == InstallMode::Fresh).then(|| manifest.get()).flatten().map(|m| view! {
                <dl class="facts">
                    <div><dt>"Library version"</dt><dd data-testid="install-version">{
                        fmt::corpus_label(m.corpus_tag.as_deref(), &m.content_version)
                    }</dd></div>
                    <div><dt>"Download size"</dt><dd data-testid="download-size">{fmt::bytes(m.total_bytes)}</dd></div>
                    <div><dt>"Entries"</dt><dd>{fmt::count(m.entry_count)}</dd></div>
                </dl>
            })}
            {move || match phase.get() {
                Phase::Checking => view! { <p class="status" role="status">"Checking the library…"</p> }.into_any(),
                Phase::Offer => match mode {
                    InstallMode::Fresh => view! {
                        <p class="install-guidance">
                            <strong>"On iPhone?"</strong> " In Safari, use Share, then Add to Home Screen, and open Cast Iron from your Home Screen before downloading. "
                            "The Home Screen app may need its own library download, and stars saved in Safari may not carry over."
                        </p>
                        <p>"The library is saved on this device for offline use."</p>
                        <button class="btn btn-primary" type="button" on:click=move |_| start()>"Download"</button>
                    }.into_any(),
                    // The reader installed the app already: one button and one reassurance.
                    InstallMode::Update => view! {
                        <button class="btn btn-primary" type="button" on:click=move |_| start()>"Download update"</button>
                        <p class="muted">
                            "Your stars and book choices stay. The current library keeps working until the new one is verified."
                        </p>
                    }.into_any(),
                },
                Phase::Failed(failure) => {
                    let label = if failure.kind == FailureKind::Schema || manifest.get().is_none() {
                        "Check again"
                    } else {
                        "Retry"
                    };
                    view! {
                        <p class="error" role="alert" tabindex="-1" node_ref=error_ref>{failure.message}</p>
                        {(failure.kind == FailureKind::Integrity).then(|| view! {
                            <p>"Files that were verified are kept, so a retry continues where it stopped."</p>
                        })}
                        <button class="btn btn-primary" type="button" on:click=move |_| {
                            if matches!(phase.get_untracked(), Phase::Failed(ref f) if f.kind == FailureKind::Schema) {
                                manifest.set(None);
                            }
                            start();
                        }>{label}</button>
                    }.into_any()
                }
                Phase::Done => ().into_any(),
                _ => view! {
                    <progress
                        max="100"
                        prop:value=move || progress.get().percent() as f64
                        aria-label="Download progress"
                    ></progress>
                }.into_any(),
            }}
            // The detailed line changes on every progress tick, so it is not a live region; the
            // announcement is the visually hidden line after it.
            <div class="status" tabindex="-1" data-testid="install-status" node_ref=status_ref>{status}</div>
            <div class="sr-only" role="status" aria-live="polite" data-testid="install-announce">{announce}</div>
        </section>
    }
}
