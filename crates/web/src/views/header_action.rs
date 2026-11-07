//! The one quiet action in the Reader header, and the dialog that Install can open.
//!
//! The header shows at most one action. An update takes priority over Install, because it is the
//! one the reader can act on right away. The action opens Settings → Updates; it never applies
//! an update by itself.

use crate::app::use_app;
use crate::pwa::{InstallCtx, restore_focus, use_install};
use crate::updates::use_updates;
use crate::views::settings::open_updates;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// What the header action does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderActionKind {
    Install,
    /// An update is waiting. `app` and `library` say which; at least one of them is true.
    Update {
        app: bool,
        library: bool,
    },
}

impl HeaderActionKind {
    /// The visible label. Narrow screens show only the icon.
    fn label(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Update { .. } => "Update ready",
        }
    }

    /// The accessible name, which says more than the label. It names what is waiting and where
    /// the action goes.
    fn aria_label(self) -> &'static str {
        match self {
            Self::Install => "Install Cast Iron as an app",
            Self::Update {
                app: true,
                library: false,
            } => "App update ready. Open Settings",
            Self::Update {
                app: false,
                library: true,
            } => "Library update available. Open Settings",
            Self::Update { .. } => "Updates available. Open Settings",
        }
    }

    /// Announced once when the action appears. The button is not a live region.
    fn status(self) -> Option<&'static str> {
        match self {
            Self::Install => None,
            Self::Update {
                app: true,
                library: false,
            } => Some("App update ready."),
            Self::Update {
                app: false,
                library: true,
            } => Some("Library update ready."),
            Self::Update { .. } => Some("Updates ready."),
        }
    }

    fn icon(self) -> AnyView {
        match self {
            Self::Install => view! {
                <path d="M12 4v10"/><path d="m8 10 4 4 4-4"/><path d="M5 19h14"/>
            }
            .into_any(),
            Self::Update { .. } => view! {
                <path d="M20 12a8 8 0 1 1-2.6-5.9"/><path d="M20 4v6h-6"/>
            }
            .into_any(),
        }
    }

    /// Update marks the icon with a pip, so the waiting update is visible without a label.
    fn has_pip(self) -> bool {
        matches!(self, Self::Update { .. })
    }

    fn activate(self, install: InstallCtx, opener: Option<web_sys::HtmlElement>) {
        match self {
            Self::Install => install.request(opener),
            // The header never applies an update by surprise: it opens the controls that do.
            Self::Update { .. } => open_updates(),
        }
    }
}

/// The action that applies right now, if any. An update outranks Install: it is the one the
/// reader can apply now, and the header has room for only one button.
fn choose_action(
    app_update: bool,
    library_update: bool,
    install_should_show: bool,
) -> Option<HeaderActionKind> {
    if app_update || library_update {
        Some(HeaderActionKind::Update {
            app: app_update,
            library: library_update,
        })
    } else {
        install_should_show.then_some(HeaderActionKind::Install)
    }
}

#[component]
pub fn HeaderAction() -> impl IntoView {
    let app = use_app();
    let install = use_install();
    let updates = use_updates();
    let current = Memo::new(move |_| {
        choose_action(
            app.shell_update_ready.get(),
            updates.library_update.get().is_some(),
            install.show_in_header(),
        )
    });
    view! {
        // Announced once, when the update becomes ready. The button is not a live region, so it
        // cannot re-announce itself.
        <span class="sr-only" role="status">
            {move || current.get().and_then(HeaderActionKind::status).unwrap_or_default()}
        </span>
        {move || current.get().map(|kind| view! { <ActionButton kind /> })}
    }
}

#[component]
fn ActionButton(kind: HeaderActionKind) -> impl IntoView {
    let install = use_install();
    view! {
        <button
            class="header-action-btn"
            type="button"
            aria-label=kind.aria_label()
            on:click=move |event| {
                let opener = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok());
                kind.activate(install, opener);
            }
        >
            <span class="header-action-icon">
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor"
                    stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    {kind.icon()}
                </svg>
                {kind.has_pip().then(|| view! {
                    <span class="header-action-pip" aria-hidden="true"></span>
                })}
            </span>
            <span class="header-action-label">{kind.label()}</span>
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(app: bool, library: bool) -> HeaderActionKind {
        HeaderActionKind::Update { app, library }
    }

    #[test]
    fn update_outranks_install_in_the_one_header_slot() {
        assert_eq!(choose_action(false, false, false), None);
        assert_eq!(
            choose_action(false, false, true),
            Some(HeaderActionKind::Install)
        );
        assert_eq!(choose_action(true, false, false), Some(update(true, false)));
        assert_eq!(choose_action(true, false, true), Some(update(true, false)));
        assert_eq!(
            choose_action(false, true, true),
            Some(update(false, true)),
            "a library update shows even when Install would"
        );
        assert_eq!(choose_action(false, true, false), Some(update(false, true)));
    }

    #[test]
    fn the_accessible_name_says_what_is_waiting_and_where_it_goes() {
        assert_eq!(
            update(true, false).aria_label(),
            "App update ready. Open Settings"
        );
        assert_eq!(
            update(false, true).aria_label(),
            "Library update available. Open Settings"
        );
        assert_eq!(
            update(true, true).aria_label(),
            "Updates available. Open Settings"
        );
        assert_eq!(update(true, true).label(), "Update ready");
        assert_eq!(HeaderActionKind::Install.status(), None);
        assert_eq!(update(false, true).status(), Some("Library update ready."));
        assert_eq!(update(true, true).status(), Some("Updates ready."));
    }
}

/// How to add the app to the Home Screen, for browsers that have no install prompt.
/// Opened by [`InstallCtx::request`]. Esc and Done close it, and focus returns to the control
/// that opened it.
#[component]
pub fn InstallGuide() -> impl IntoView {
    let install = use_install();
    let dialog = NodeRef::<leptos::html::Dialog>::new();

    Effect::new(move |_| {
        let wanted = install.guide_open.get();
        let Some(dialog) = dialog.get() else {
            return;
        };
        if wanted && !dialog.open() {
            let _ = dialog.show_modal();
        } else if !wanted && dialog.open() {
            dialog.close();
        }
    });

    view! {
        <dialog
            node_ref=dialog
            class="install-sheet"
            aria-labelledby="install-sheet-title"
            // Esc and `close()` both end here.
            on:close=move |_| {
                install.guide_open.set(false);
                restore_focus(install.take_opener());
            }
        >
            <h2 id="install-sheet-title">"Add Cast Iron to your Home Screen"</h2>
            <ol class="install-steps">
                <li>
                    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
                        stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <path d="M12 15V4"/><path d="m8 8 4-4 4 4"/><path d="M7 11H5.5v9h13v-9H17"/>
                    </svg>
                    <span>"Tap Share in Safari's toolbar."</span>
                </li>
                <li>
                    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
                        stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <rect x="4" y="4" width="16" height="16" rx="3"/><path d="M12 8v8M8 12h8"/>
                    </svg>
                    <span>"Choose Add to Home Screen."</span>
                </li>
                <li>
                    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
                        stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <path d="m5 12.5 4.5 4.5L19 7.5"/>
                    </svg>
                    <span>"Tap Add to finish."</span>
                </li>
            </ol>
            <button class="btn install-sheet-done" type="button"
                on:click=move |_| install.guide_open.set(false)>"Done"</button>
        </dialog>
    }
}
