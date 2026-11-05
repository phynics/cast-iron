//! The one quiet action in the Reader header, and the dialog that Install can open.
//!
//! The header shows at most one action. Today that is Install; the enum leaves room for the
//! app update action, which takes priority over Install when it applies.

use crate::pwa::{InstallCtx, restore_focus, use_install};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// What the header action does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderActionKind {
    Install,
}

impl HeaderActionKind {
    /// The visible label. Narrow screens show only the icon.
    fn label(self) -> &'static str {
        match self {
            Self::Install => "Install",
        }
    }

    /// The accessible name, which says more than the label.
    fn aria_label(self) -> &'static str {
        match self {
            Self::Install => "Install Cast Iron as an app",
        }
    }

    fn icon(self) -> AnyView {
        match self {
            Self::Install => view! {
                <path d="M12 4v10"/><path d="m8 10 4 4 4-4"/><path d="M5 19h14"/>
            }
            .into_any(),
        }
    }

    fn activate(self, install: InstallCtx, opener: Option<web_sys::HtmlElement>) {
        match self {
            Self::Install => install.request(opener),
        }
    }
}

/// The action that applies right now, if any.
fn current_action(install: InstallCtx) -> Option<HeaderActionKind> {
    install
        .show_in_header()
        .then_some(HeaderActionKind::Install)
}

#[component]
pub fn HeaderAction() -> impl IntoView {
    let install = use_install();
    let current = Memo::new(move |_| current_action(install));
    move || current.get().map(|kind| view! { <ActionButton kind /> })
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
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor"
                stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                {kind.icon()}
            </svg>
            <span class="header-action-label">{kind.label()}</span>
        </button>
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
