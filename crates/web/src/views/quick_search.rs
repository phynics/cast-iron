//! Quick search: the full-screen layer that the header search icon opens.
//!
//! The layer is one native modal `<dialog>`, mounted once in the Reader frame. It holds the only
//! search field outside the results page, so the header never keeps a typed query. Opening it
//! pushes one history entry, so Back closes the layer and returns to the page that was open, with
//! its scroll position. Submitting replaces that entry with the destination, so Back from the
//! results leaves no extra step behind.

use crate::js;
use crate::route::{self, Route, use_route};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// The history state of the entry that Quick search pushes.
const HISTORY_MARKER: &str = "cast-iron:quick-search";
/// The id of the header button, which receives focus when the layer closes.
pub const OPEN_BUTTON_ID: &str = "quick-search-open";
/// Searches offered when the reader has nothing recent to repeat.
pub const STARTERS: [&str; 4] = ["fireball", "grapple", "cure light", "dragon"];

/// The state of the layer. Provided once by [`QuickSearchLayer`]; every field is `Copy`.
#[derive(Clone, Copy)]
pub struct QuickSearch {
    open: RwSignal<bool>,
    text: RwSignal<String>,
    dialog: NodeRef<leptos::html::Dialog>,
    input: NodeRef<leptos::html::Input>,
    /// The control that had focus when the layer opened. Focus returns to it.
    opener: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
    /// True while the history entry that this layer pushed is the current one.
    pushed: StoredValue<bool>,
    /// The page scroll position at open, restored when the layer is cancelled.
    scroll: StoredValue<(f64, f64)>,
    /// Counts openings, so a late timer from one opening cannot close the next.
    session: StoredValue<u64>,
}

pub fn use_quick_search() -> QuickSearch {
    expect_context::<QuickSearch>()
}

fn history() -> Option<web_sys::History> {
    web_sys::window()?.history().ok()
}

fn marker_is_current() -> bool {
    history()
        .and_then(|history| history.state().ok())
        .and_then(|state| state.as_string())
        .is_some_and(|state| state == HISTORY_MARKER)
}

/// Keeps Tab and Shift+Tab inside the layer. A modal dialog already blocks the page behind it,
/// but a browser lets focus leave through its own toolbar; the reader should wrap instead.
fn wrap_tab_focus(dialog: &web_sys::HtmlDialogElement, event: &web_sys::KeyboardEvent) {
    let Ok(list) = dialog.query_selector_all("input:not([disabled]), button:not([disabled])")
    else {
        return;
    };
    let controls: Vec<web_sys::HtmlElement> = (0..list.length())
        .filter_map(|index| list.item(index))
        .filter_map(|node| node.dyn_into().ok())
        .collect();
    let (Some(first), Some(last)) = (controls.first(), controls.last()) else {
        return;
    };
    let active = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element());
    let at = |control: &web_sys::HtmlElement| {
        active
            .as_ref()
            .is_some_and(|active| control.is_same_node(Some(active)))
    };
    let (edge, target) = if event.shift_key() {
        (first, last)
    } else {
        (last, first)
    };
    if at(edge) {
        event.prevent_default();
        let _ = target.focus();
    }
}

/// Focuses `element` without scrolling the page: the reader's scroll position is restored on its
/// own, and a sticky header control would otherwise pull the page toward its natural position.
fn focus_element(element: &web_sys::HtmlElement) {
    if element.is_connected() {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = element.focus_with_options(&options);
    }
}

impl QuickSearch {
    /// Creates the state and provides it to the components below. Call once, in the Reader.
    pub fn provide() -> Self {
        let quick = Self::new();
        provide_context(quick);
        quick
    }

    fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            text: RwSignal::new(String::new()),
            dialog: NodeRef::new(),
            input: NodeRef::new(),
            opener: StoredValue::new_local(None),
            pushed: StoredValue::new(false),
            scroll: StoredValue::new((0.0, 0.0)),
            session: StoredValue::new(0),
        }
    }

    pub fn is_open(self) -> bool {
        self.open.get_untracked()
    }

    /// Opens the layer and focuses its field. Call it from the event handler that the reader's
    /// tap or key press runs, not from an effect: a phone shows the keyboard only for a focus
    /// that happens inside the gesture.
    pub fn open(self) {
        if self.open.get_untracked() {
            return;
        }
        let Some(dialog) = self.dialog.get_untracked() else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let opener = window
            .document()
            .and_then(|document| document.active_element())
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            .filter(|element| element.tag_name() != "BODY");
        self.opener.set_value(opener);
        self.scroll.set_value((
            window.scroll_x().unwrap_or(0.0),
            window.scroll_y().unwrap_or(0.0),
        ));
        let pushed = history().is_some_and(|history| {
            history
                .push_state_with_url(&wasm_bindgen::JsValue::from_str(HISTORY_MARKER), "", None)
                .is_ok()
        });
        self.pushed.set_value(pushed);
        self.session.update_value(|session| *session += 1);
        self.text.set(String::new());
        self.open.set(true);
        let _ = dialog.show_modal();
        if let Some(input) = self.input.get_untracked() {
            let _ = input.focus();
        }
    }

    /// Cancel, Escape and Back. Goes through the history entry when there is one, so the
    /// browser's own Back and this button end in the same place.
    fn dismiss(self) {
        if !self.open.get_untracked() {
            return;
        }
        if self.pushed.get_value() && marker_is_current() {
            route::go_back();
            // `popstate` closes the layer. If the browser never reports it, close anyway.
            let session = self.session.get_value();
            leptos::task::spawn_local(async move {
                js::sleep(400).await;
                if self.open.try_get_untracked() == Some(true)
                    && self.session.try_get_value() == Some(session)
                {
                    self.finish_close(true);
                }
            });
        } else {
            self.finish_close(true);
        }
    }

    /// Closes the layer. `restore` puts focus and scroll back where they were; leaving for
    /// another page does not, because that page takes focus itself.
    fn finish_close(self, restore: bool) {
        self.pushed.set_value(false);
        if let Some(dialog) = self.dialog.get_untracked()
            && dialog.open()
        {
            dialog.close();
        }
        self.open.set(false);
        self.text.set(String::new());
        let opener = self.opener.try_update_value(Option::take).flatten();
        if !restore {
            return;
        }
        let target = opener.filter(|element| element.is_connected()).or_else(|| {
            web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(OPEN_BUTTON_ID))
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        });
        if let Some(target) = target {
            focus_element(&target);
        }
        let (x, y) = self.scroll.get_value();
        if let Some(window) = web_sys::window() {
            window.scroll_to_with_x_and_y(x, y);
        }
    }

    /// Leaves for `to`: the layer's history entry becomes that page, so Back goes to the page
    /// that was open before the layer.
    fn go(self, to: Route) {
        let replace = self.pushed.get_value() && marker_is_current();
        self.finish_close(false);
        if replace {
            route::replace(&to);
        } else {
            route::navigate(&to);
        }
    }

    fn submit(self) {
        let text = self.text.get_untracked();
        let text = text.trim();
        if !text.is_empty() {
            self.go(Route::search(text));
        }
    }
}

/// The header button that opens Quick search. On the results page, which already has the one
/// search field, it focuses that field instead.
#[component]
pub fn QuickSearchButton() -> impl IntoView {
    let quick = use_quick_search();
    let current = use_route();
    view! {
        <button
            id=OPEN_BUTTON_ID
            class="header-search-btn"
            type="button"
            aria-label="Search"
            aria-haspopup="dialog"
            on:click=move |_| {
                if matches!(current.get_untracked(), Route::Search { .. }) {
                    focus_results_field();
                } else {
                    quick.open();
                }
            }
        >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
                stroke-linecap="round" aria-hidden="true">
                <circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/>
            </svg>
        </button>
    }
}

/// Focuses and selects the field of the results page.
pub fn focus_results_field() -> bool {
    let Some(input) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("search-input"))
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return false;
    };
    let _ = input.focus();
    input.select();
    true
}

/// The layer itself. Mount it once, inside the Reader frame.
#[component]
pub fn QuickSearchLayer() -> impl IntoView {
    let quick = use_quick_search();
    let route = use_route();

    // Back, or the browser's close request, ends the layer through the history entry.
    let popstate = window_event_listener(leptos::ev::popstate, move |_| {
        if quick.open.get_untracked() && quick.pushed.get_value() {
            quick.finish_close(true);
        }
    });
    on_cleanup(move || popstate.remove());

    // A route change that the layer did not make, such as a typed address, closes it.
    Effect::new(move |previous: Option<Route>| {
        let now = route.get();
        if previous.is_some_and(|before| before != now) && quick.open.get_untracked() {
            quick.finish_close(false);
        }
        now
    });

    let trimmed = move || quick.text.with(|text| text.trim().to_owned());
    view! {
        <dialog
            node_ref=quick.dialog
            class="quick-search"
            aria-label="Quick search"
            aria-modal="true"
            // Escape, and a phone's Back button, ask the dialog to cancel. The layer closes
            // itself so that its history entry goes too.
            on:cancel=move |ev: web_sys::Event| {
                ev.prevent_default();
                quick.dismiss();
            }
            on:keydown=move |ev| {
                match ev.key().as_str() {
                    "Escape" => {
                        ev.prevent_default();
                        quick.dismiss();
                    }
                    "Tab" => {
                        if let Some(dialog) = quick.dialog.get_untracked() {
                            wrap_tab_focus(&dialog, &ev);
                        }
                    }
                    _ => {}
                }
            }
            on:close=move |_| quick.dismiss()
        >
            <form class="quick-search-bar" role="search" on:submit=move |ev| {
                ev.prevent_default();
                quick.submit();
            }>
                <div class="search-field">
                    <svg class="search-field-icon" viewBox="0 0 24 24" fill="none"
                        stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
                        <circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/>
                    </svg>
                    <input
                        id="quick-search-input"
                        node_ref=quick.input
                        type="search"
                        placeholder="Search spells, feats, rules…"
                        aria-label="Search the rules"
                        autocomplete="off"
                        autocapitalize="off"
                        spellcheck="false"
                        enterkeyhint="search"
                        prop:value=move || quick.text.get()
                        on:input=move |ev| quick.text.set(event_target_value(&ev))
                    />
                    <Show when=move || quick.text.with(|text| !text.is_empty())>
                        <button class="search-clear" type="button" aria-label="Clear search"
                            on:click=move |_| {
                                quick.text.set(String::new());
                                if let Some(input) = quick.input.get_untracked() {
                                    let _ = input.focus();
                                }
                            }>
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
                                stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
                                <path d="m7 7 10 10M17 7 7 17"/>
                            </svg>
                        </button>
                    </Show>
                </div>
                <button class="quick-search-cancel" type="button" on:click=move |_| quick.dismiss()>
                    "Cancel"
                </button>
            </form>
            <Show when=move || quick.open.get()>
                <div class="quick-search-body">
                    {move || {
                        let query = trimmed();
                        if query.is_empty() {
                            view! {
                                <h2 class="quick-search-heading">"Try searching for"</h2>
                                <ul class="quick-search-list">
                                    {STARTERS.into_iter().map(|starter| view! {
                                        <li>
                                            <button class="quick-search-row" type="button"
                                                on:click=move |_| quick.go(Route::search(starter))>
                                                <span class="quick-search-name">{starter}</span>
                                            </button>
                                        </li>
                                    }).collect_view()}
                                </ul>
                            }.into_any()
                        } else {
                            let label = format!("See all results for “{query}”");
                            view! {
                                <ul class="quick-search-list">
                                    <li>
                                        <button class="quick-search-row quick-search-all" type="button"
                                            on:click=move |_| quick.submit()>
                                            <span class="quick-search-name">{label}</span>
                                        </button>
                                    </li>
                                </ul>
                            }.into_any()
                        }
                    }}
                </div>
            </Show>
        </dialog>
    }
}
