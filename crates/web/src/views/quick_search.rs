//! Quick search: the full-screen layer that the header search icon opens.
//!
//! The layer is one native modal `<dialog>`, mounted once in the Reader frame. It holds the only
//! search field outside the results page, so the header never keeps a typed query. Opening it
//! pushes one history entry, so Back closes the layer and returns to the page that was open, with
//! its scroll position. Submitting replaces that entry with the destination, so Back from the
//! results leaves no extra step behind.

use crate::discovery::Discovery;
use crate::js;
use crate::library::{Library, use_library};
use crate::route::{self, Route, use_route};
use crate::store::{self, ContinuePoint};
use crate::views::book_tier::{Tier, preferred_source};
use crate::views::continue_reading;
use crate::views::icons;
use crate::views::quick_rows::{
    self, GroupKind, List, MAX_SUGGESTIONS, Row, Suggestion, ViewedRow, dedupe_hits,
    shortest_completion_first, step, valid_active,
};
use crate::views::recent::{self, Recent};
use crate::views::recent_searches::{self, RecentSearch, normalize};
use crate::views::scope::{Scope, ScopePill};
use crate::views::search::discovery_for;
use crate::views::util::{addon_name, kind_name};
use cast_iron_schema::{EntryId, EntryKind};
use cast_iron_search::Query;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::BTreeSet;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// The id of the header button, which receives focus when the layer closes.
pub const OPEN_BUTTON_ID: &str = "quick-search-open";
/// The pause after the last keystroke before the suggestions are looked up.
const SUGGEST_DEBOUNCE_MS: u32 = 100;
/// How many ranked hits to read before the reprints are collapsed into suggestions.
const LOOKUP_LIMIT: usize = 64;
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
    /// The highlighted row, which the field names through `aria-activedescendant`.
    active: RwSignal<Option<usize>>,
    /// False after Escape closed the suggestion list, until the reader types or presses Down.
    list_open: RwSignal<bool>,
    suggestions: RwSignal<Vec<Suggestion>>,
    recents: RwSignal<Vec<RecentSearch>>,
    /// The most recently read book's Continue row, when that book is active.
    continue_row: RwSignal<Option<ViewedRow>>,
    /// Whether Recently viewed shows every row or only the first five.
    viewed_expanded: RwSignal<bool>,
    /// Numbers the suggestion lookups, so a slow answer for an old text is dropped.
    lookup: StoredValue<u64>,
    /// The page that is open, which the scope is taken from at opening.
    route: ReadSignal<Route>,
    /// The Reader's library, captured at setup so an event handler keeps the context it needs.
    library: StoredValue<Library, LocalStorage>,
    /// What the search is limited to. Set at opening from a list page; the reader removes it
    /// with the pill. It limits the suggestions and the results page that Enter opens.
    scope: RwSignal<Option<Scope>>,
}

pub fn use_quick_search() -> QuickSearch {
    expect_context::<QuickSearch>()
}

fn marker_is_current() -> bool {
    // The entry's state is shared with the scroll memory; only the `quickSearch` field is ours.
    route::current_entry_has_flag(route::QUICK_SEARCH_FIELD)
}

/// The recently viewed rows the library still shows: aliases resolve to their canonical entry,
/// and a retired entry, an unknown id or an entry whose book is inactive is left out.
fn viewed_rows(library: &Library, recents: &[Recent]) -> Vec<ViewedRow> {
    let mut seen = BTreeSet::new();
    recents
        .iter()
        .filter_map(|item| {
            let id = continue_reading::live_target(library, &item.id)?;
            if !library.is_entry_active(&id) || !seen.insert(id.clone()) {
                return None;
            }
            let stub = library.stub(&id)?;
            let book = library
                .canonical_source_for_entry(&id)
                .and_then(|source| library.source(&source))
                .map(|source| source.title.clone());
            let kind = kind_name(stub.kind);
            let detail = match book {
                Some(book) => format!("{kind} · {book}"),
                None => kind.to_string(),
            };
            Some(ViewedRow {
                id,
                title: stub.title.clone(),
                detail,
                book: None,
            })
        })
        .collect()
}

/// The Continue row for the most recently read book, when that book is active. The entry title
/// is the muted line; the book stays in the link so opening the row keeps the reading context.
fn continue_viewed_row(library: &Library, points: &[ContinuePoint]) -> Option<ViewedRow> {
    let active: Vec<ContinuePoint> = points
        .iter()
        .filter(|point| library.book_active(&point.source))
        .cloned()
        .collect();
    let point = store::latest_continue_point(&active)?;
    let source = library.source(&point.source)?;
    let id = continue_reading::live_target(library, &point.entry_id)?;
    if !library.is_entry_active(&id) {
        return None;
    }
    let stub = library.stub(&id)?;
    Some(ViewedRow {
        id,
        title: format!("Continue {}", source.title),
        detail: stub.title.clone(),
        book: Some(point.source.clone()),
    })
}

/// Forgets one recently viewed entry, including any stored row that resolves to it through an
/// alias, and writes the shortened list.
fn remove_viewed(recents: RwSignal<Vec<Recent>>, library: &Library, id: EntryId) {
    let library = library.clone();
    spawn_local(async move {
        recent::remove(recents, |stored| library.resolves_to(stored, &id)).await;
    });
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
            active: RwSignal::new(None),
            list_open: RwSignal::new(true),
            suggestions: RwSignal::new(Vec::new()),
            recents: RwSignal::new(Vec::new()),
            continue_row: RwSignal::new(None),
            viewed_expanded: RwSignal::new(false),
            lookup: StoredValue::new(0),
            route: use_route(),
            library: StoredValue::new_local(use_library()),
            scope: RwSignal::new(None),
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
        let pushed = route::push_flagged_entry(route::QUICK_SEARCH_FIELD);
        self.pushed.set_value(pushed);
        self.session.update_value(|session| *session += 1);
        self.text.set(String::new());
        self.active.set(None);
        self.list_open.set(true);
        // A list page hands its kind to the search; every other page searches everything. A
        // book-filtered list also names its book in the pill, so read the title from the library.
        let scope = Scope::of_page(&self.route.get_untracked()).map(|scope| {
            let title = scope.source.as_deref().and_then(|source| {
                crate::library::use_library()
                    .source(source)
                    .map(|s| s.title.clone())
            });
            scope.with_source_title(title)
        });
        self.scope.set(scope);
        self.continue_row.set(None);
        self.viewed_expanded.set(false);
        self.open.set(true);
        spawn_local(async move {
            let list = recent_searches::load().await;
            self.recents.try_set(list);
        });
        {
            let library = self.library.get_value();
            spawn_local(async move {
                let points = continue_reading::load(&library).await;
                self.continue_row
                    .try_set(continue_viewed_row(&library, &points));
            });
        }
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
        self.scope.set(None);
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
            // The layer's entry becomes another page, so it starts without the old page's state.
            route::replace_as_new_page(&to);
        } else {
            route::navigate(&to);
        }
    }

    /// Runs a search for `text` in the scope the layer has: remembers it with that scope, then
    /// opens the results page.
    fn search(self, text: &str) {
        self.run(RecentSearch::new(text, self.scope.get_untracked()));
    }

    /// Runs a search in the scope it carries. A recent search brings its own scope, whatever
    /// the page the layer opened from.
    fn run(self, search: RecentSearch) {
        if search.text.is_empty() {
            return;
        }
        let route = search.route();
        spawn_local(async move {
            recent_searches::record(&search.text, search.scope).await;
        });
        self.go(route);
    }

    fn submit(self) {
        let text = self.text.get_untracked();
        self.search(&text);
    }

    fn open_row(self, row: &Row) {
        match row {
            Row::Entry(suggestion) => self.go(Route::Entry {
                id: suggestion.id.clone(),
                anchor: None,
                book: None,
            }),
            Row::Continue(viewed) => self.go(Route::Entry {
                id: viewed.id.clone(),
                anchor: None,
                book: viewed.book.clone(),
            }),
            Row::Viewed(viewed) => self.go(Route::Entry {
                id: viewed.id.clone(),
                anchor: None,
                book: None,
            }),
            Row::SeeAll(text) | Row::Starter(text) => self.search(text),
            Row::Search(search) => self.run(search.clone()),
        }
    }

    /// True while the suggestion list is shown for typed text, which Escape closes before it
    /// closes the layer.
    fn suggestions_shown(self) -> bool {
        self.list_open.get_untracked() && self.text.with_untracked(|text| !text.trim().is_empty())
    }

    fn focus_field(self) {
        if let Some(input) = self.input.get_untracked() {
            let _ = input.focus();
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
            // On the results page the button only focuses that page's field.
            aria-haspopup=move || {
                (!matches!(current.get(), Route::Search { .. })).then_some("dialog")
            }
            on:click=move |_| {
                if matches!(current.get_untracked(), Route::Search { .. }) {
                    focus_results_field();
                } else {
                    quick.open();
                }
            }
        >
            {icons::search()}
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

fn option_id(index: usize) -> String {
    format!("quick-search-option-{index}")
}

/// Keeps the highlighted row in view as Up and Down move it.
fn reveal_option(index: usize) {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(&option_id(index)))
    {
        let options = web_sys::ScrollIntoViewOptions::new();
        options.set_block(web_sys::ScrollLogicalPosition::Nearest);
        element.scroll_into_view_with_scroll_into_view_options(&options);
    }
}

/// Reads the suggestions for the typed text from the search engine, over the active books.
fn watch_suggestions(quick: QuickSearch) {
    let library = use_library();
    let cache = StoredValue::new_local(None::<(u64, Rc<Discovery>)>);
    Effect::new(move |_| {
        let query = quick.text.with(|text| normalize(text));
        let scope = quick.scope.get();
        let revision = library.selection_revision().get();
        let id = quick.lookup.get_value() + 1;
        quick.lookup.set_value(id);
        if !quick.open.get() || query.is_empty() {
            quick.suggestions.set(Vec::new());
            return;
        }
        let discovery = discovery_for(&library, cache, revision);
        let library = library.clone();
        spawn_local(async move {
            // Wait for the reader to pause; a newer keystroke makes this lookup stale.
            js::sleep(SUGGEST_DEBOUNCE_MS).await;
            if quick.lookup.try_get_value() != Some(id) {
                return;
            }
            let typed = query.clone();
            let query = Query {
                text: query,
                filters: scope.map(|scope| scope.filters()).unwrap_or_default(),
                limit: LOOKUP_LIMIT,
            };
            let Ok(hits) = discovery.search_summaries(&query).await else {
                return;
            };
            // A mythic suggestion names its label instead of the kind while its base's book may
            // be inactive, so the labels read the whole graph. A class option names its group,
            // which comes from the small derived index.
            let relations = library.relations().await.ok().flatten();
            let option_groups = library.option_groups().await.ok().flatten();
            let revisions = library.revisions().await.ok().flatten();
            // One suggestion per feat, under the original id, when a revision is in force.
            let hits = match revisions.as_ref() {
                Some(revisions) => crate::revisions::effective_hits(hits, revisions.view(), |id| {
                    discovery.is_eligible(id)
                }),
                None => hits,
            };
            let hits = shortest_completion_first(hits, &typed);
            let list: Vec<Suggestion> = dedupe_hits(hits, MAX_SUGGESTIONS)
                .into_iter()
                .filter_map(|hit| {
                    let id = EntryId::parse(hit.id.clone()).ok()?;
                    let kind = EntryKind::from_slug(&hit.kind).map_or("", kind_name);
                    let addon = relations
                        .as_ref()
                        .and_then(|relations| relations.addon_label(&id))
                        .map(addon_name);
                    let group = option_groups
                        .as_ref()
                        .and_then(|index| crate::class_options::group_of(index, &id))
                        .map(|group| group.singular.as_str());
                    let row_kind = match (addon, group) {
                        (Some(label), _) => label,
                        (None, Some(group)) => group,
                        (None, None) => kind,
                    };
                    // The index records one attribution source. When it is inactive, the entry's
                    // book placement is the source that kept it eligible, so name that instead
                    // (the same rule as the search result rows).
                    let canonical = library.canonical_source_for_entry(&id);
                    let source_id = preferred_source(
                        &hit.source,
                        canonical.as_deref(),
                        |id| library.book_active(id),
                        |id| library.source(id).map_or(Tier::Other, Tier::of),
                    );
                    let book = library
                        .source(source_id)
                        .map_or(source_id, |source| source.title.as_str());
                    Some(Suggestion {
                        id,
                        title: hit.title,
                        detail: format!("{row_kind} · {book}"),
                    })
                })
                .collect();
            // Only the newest lookup may show: the reader has typed on since the others began.
            if quick.lookup.try_get_value() == Some(id) {
                quick.suggestions.try_set(list);
            }
        });
    });
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

    watch_suggestions(quick);

    let app = crate::app::use_app();
    let library = use_library();
    let memo_library = library.clone();
    let list = Memo::new(move |_| {
        // The active-book choice decides which visits are visible, so read its revision too.
        let _ = memo_library.selection_revision().get();
        let text = quick.text.get();
        let suggestions = quick.suggestions.get();
        let recents = quick.recents.get();
        let continue_row = quick.continue_row.get();
        let expanded = quick.viewed_expanded.get();
        let viewed = app
            .recents
            .with(|recents| viewed_rows(&memo_library, recents));
        quick_rows::list(
            &text,
            &suggestions,
            &recents,
            &STARTERS,
            continue_row,
            &viewed,
            expanded,
        )
    });
    // The rows changed under the highlight, so it may name a row that is gone or a different
    // one. Drop it: Enter then searches the text instead of opening a row the reader did not see.
    Effect::new(move |_| {
        list.track();
        quick.active.set(None);
    });
    // The list is a popup only while the reader types. Recent searches, starter searches and
    // recently viewed entries are the content of the empty layer, so they stay expanded.
    let expanded = move || {
        let list = list.get();
        !list.is_empty() && (!list.is_typing() || quick.list_open.get())
    };
    let move_highlight = move |down: bool| {
        quick.list_open.set(true);
        let len = list.with_untracked(List::len);
        let next = step(quick.active.get_untracked(), len, down);
        quick.active.set(next);
        if let Some(index) = next {
            reveal_option(index);
        }
    };
    let remove_recent = move |search: RecentSearch| {
        spawn_local(async move {
            let list = recent_searches::remove(&search).await;
            quick.recents.try_set(list);
        });
        quick.focus_field();
    };
    let clear_recents = move |_| {
        spawn_local(async move {
            recent_searches::clear().await;
            quick.recents.try_set(Vec::new());
        });
        quick.focus_field();
    };
    let clear_viewed = move |_| {
        // Empty the screen now; the writes below are the durable copies.
        app.recents.set(Vec::new());
        quick.continue_row.set(None);
        quick.viewed_expanded.set(false);
        spawn_local(async move {
            recent::clear(app.recents).await;
            let _ = store::clear_continue_points().await;
        });
        quick.focus_field();
    };

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
                    // The first Escape closes the suggestion list, the second closes the layer.
                    "Escape" => {
                        ev.prevent_default();
                        if quick.suggestions_shown() {
                            quick.list_open.set(false);
                            quick.active.set(None);
                        } else {
                            quick.dismiss();
                        }
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
                    {icons::search_field()}
                    <input
                        id="quick-search-input"
                        node_ref=quick.input
                        type="search"
                        role="combobox"
                        aria-label=move || {
                            quick.scope.with(|scope| {
                                scope.as_ref().map_or_else(
                                    || "Search the rules".to_string(),
                                    Scope::placeholder,
                                )
                            })
                        }
                        aria-autocomplete="list"
                        aria-controls=move || {
                            list.with(|list| {
                                list.groups
                                    .iter()
                                    .map(|group| format!("quick-search-list-{}", group.kind.slug()))
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                        }
                        aria-expanded=move || expanded().to_string()
                        aria-activedescendant=move || {
                            let len = list.with(List::len);
                            valid_active(quick.active.get(), len)
                                .filter(|_| expanded())
                                .map(option_id)
                        }
                        placeholder=move || {
                            quick.scope.with(|scope| {
                                scope.as_ref().map_or_else(
                                    || "Search spells, feats, rules…".to_string(),
                                    Scope::placeholder,
                                )
                            })
                        }
                        autocomplete="off"
                        autocapitalize="off"
                        spellcheck="false"
                        enterkeyhint="search"
                        prop:value=move || quick.text.get()
                        on:input=move |ev| {
                            quick.text.set(event_target_value(&ev));
                            quick.active.set(None);
                            quick.list_open.set(true);
                        }
                        on:keydown=move |ev| {
                            match ev.key().as_str() {
                                "ArrowDown" | "ArrowUp" => {
                                    ev.prevent_default();
                                    move_highlight(ev.key() == "ArrowDown");
                                }
                                // A highlighted row opens itself; without one, Enter submits the text.
                                "Enter" if !ev.is_composing() => {
                                    let row = quick.active.get_untracked().filter(|_| expanded()).and_then(|index| {
                                        list.with_untracked(|list| list.row(index).cloned())
                                    });
                                    if let Some(row) = row {
                                        ev.prevent_default();
                                        quick.open_row(&row);
                                    }
                                }
                                // Delete forgets the highlighted recent search or visit.
                                "Delete" if quick.text.with_untracked(|text| text.is_empty()) => {
                                    let row = quick.active.get_untracked().and_then(|index| {
                                        list.with_untracked(|list| list.row(index).cloned())
                                    });
                                    match row {
                                        Some(Row::Search(search)) => {
                                            ev.prevent_default();
                                            quick.active.set(None);
                                            remove_recent(search);
                                        }
                                        Some(Row::Viewed(viewed)) => {
                                            ev.prevent_default();
                                            quick.active.set(None);
                                            remove_viewed(app.recents, &library, viewed.id);
                                            quick.focus_field();
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                    />
                    <Show when=move || quick.text.with(|text| !text.is_empty())>
                        <button class="search-clear" type="button" aria-label="Clear search"
                            on:click=move |_| {
                                quick.text.set(String::new());
                                quick.focus_field();
                            }>
                            {icons::close()}
                        </button>
                    </Show>
                </div>
                <button class="quick-search-cancel" type="button" on:click=move |_| quick.dismiss()>
                    "Cancel"
                </button>
            </form>
            // The scope sits under the field, where a reader looks next, and one tap removes it.
            {move || quick.scope.get().map(|scope| view! {
                <div class="quick-search-scope" role="group" aria-label="Search scope">
                    <span class="quick-search-scope-label">"Searching in"</span>
                    <ScopePill scope on_remove=Callback::new(move |()| {
                        quick.scope.set(None);
                        quick.focus_field();
                    }) />
                </div>
            })}
            <p class="sr-only" role="status">
                {move || {
                    // Say nothing while the list is closed: it shows no suggestions.
                    let typed = quick.text.with(|text| !text.trim().is_empty());
                    (typed && expanded()).then(|| match quick.suggestions.with(Vec::len) {
                        0 => "No suggestions".to_string(),
                        1 => "1 suggestion".to_string(),
                        n => format!("{} suggestions", crate::fmt::count(n)),
                    })
                }}
            </p>
            <Show when=move || quick.open.get()>
                <div class="quick-search-body">
                    {move || {
                        let list = list.get();
                        let viewed_total = list.viewed_total;
                        let hidden_viewed = list.hidden_viewed;
                        let mut index = 0;
                        let groups = list.groups.into_iter().map(|group| {
                            let label = group.kind.label();
                            let heading_id = format!("quick-search-heading-{}", group.kind.slug());
                            let list_id = format!("quick-search-list-{}", group.kind.slug());
                            let heading = if group.kind == GroupKind::Suggestions {
                                // The suggestion popup has no visible heading, but the listbox still
                                // needs a name.
                                view! { <h2 class="sr-only" id=heading_id.clone()>{label}</h2> }
                                    .into_any()
                            } else {
                                let clear = match group.kind {
                                    GroupKind::Recent => Some(view! {
                                        <button class="quick-search-clear-all" type="button"
                                            on:click=clear_recents>"Clear"</button>
                                    }.into_any()),
                                    GroupKind::Viewed => Some(view! {
                                        <button class="quick-search-clear-all" type="button"
                                            on:click=clear_viewed>"Clear"</button>
                                    }.into_any()),
                                    _ => None,
                                };
                                view! {
                                    <div class="quick-search-heading-row">
                                        <h2 class="quick-search-heading" id=heading_id.clone()>{label}</h2>
                                        {clear}
                                    </div>
                                }.into_any()
                            };
                            let items = group.rows.into_iter().map(|row| {
                                let i = index;
                                index += 1;
                                let remove = match &row {
                                    Row::Search(search) => {
                                        let search = search.clone();
                                        let name = format!("Remove “{}” from recent searches", search.name());
                                        Some(view! {
                                            <button class="quick-search-remove" type="button" aria-label=name
                                                on:click=move |_| remove_recent(search.clone())>
                                                {icons::close()}
                                            </button>
                                        }.into_any())
                                    }
                                    Row::Viewed(viewed) => {
                                        let id = viewed.id.clone();
                                        let name = format!("Remove “{}” from recently viewed", viewed.title);
                                        Some(view! {
                                            <button class="quick-search-remove" type="button" aria-label=name
                                                on:click=move |_| {
                                                    let library = quick.library.get_value();
                                                    remove_viewed(app.recents, &library, id.clone());
                                                    quick.focus_field();
                                                }>
                                                {icons::close()}
                                            </button>
                                        }.into_any())
                                    }
                                    _ => None,
                                };
                                let body = match &row {
                                    Row::Entry(suggestion) => view! {
                                        <span class="quick-search-name">{suggestion.title.clone()}</span>
                                        <span class="quick-search-meta">{suggestion.detail.clone()}</span>
                                    }.into_any(),
                                    Row::Continue(viewed) | Row::Viewed(viewed) => view! {
                                        <span class="quick-search-name">{viewed.title.clone()}</span>
                                        <span class="quick-search-meta">{viewed.detail.clone()}</span>
                                    }.into_any(),
                                    Row::SeeAll(text) => view! {
                                        <span class="quick-search-name">
                                            {format!("See all results for “{text}”")}
                                        </span>
                                    }.into_any(),
                                    Row::Search(search) => view! {
                                        <span class="quick-search-name">{search.text.clone()}</span>
                                        {search.scope_label().map(|label| view! {
                                            <span class="quick-search-meta">{label}</span>
                                        })}
                                    }.into_any(),
                                    Row::Starter(text) => view! {
                                        <span class="quick-search-name">{text.clone()}</span>
                                    }.into_any(),
                                };
                                let all = matches!(row, Row::SeeAll(_));
                                view! {
                                    <li class="quick-search-item" role="presentation">
                                        <div
                                            id=option_id(i)
                                            class="quick-search-row"
                                            class:quick-search-all=all
                                            role="option"
                                            aria-selected=move || (quick.active.get() == Some(i)).to_string()
                                            // Keep focus in the field, so the keyboard stays open.
                                            on:mousedown=move |ev| ev.prevent_default()
                                            on:click=move |_| quick.open_row(&row)
                                        >
                                            {body}
                                        </div>
                                        {remove}
                                    </li>
                                }
                            }).collect_view();
                            view! {
                                {heading}
                                <ul id=list_id class="quick-search-list" role="listbox"
                                    aria-label=label aria-labelledby=heading_id hidden=move || !expanded()>
                                    {items}
                                </ul>
                            }.into_any()
                        }).collect_view();
                        let more = (viewed_total > quick_rows::MAX_RECENT_VIEWED).then(|| view! {
                            <button
                                class="btn btn-quiet quick-search-more"
                                type="button"
                                aria-controls="quick-search-list-viewed"
                                aria-expanded=move || quick.viewed_expanded.get().to_string()
                                on:click=move |_| {
                                    quick.viewed_expanded.update(|open| *open = !*open);
                                    quick.focus_field();
                                }
                            >
                                {move || if quick.viewed_expanded.get() {
                                    "Show fewer".to_string()
                                } else {
                                    format!("Show more ({})", hidden_viewed)
                                }}
                            </button>
                        });
                        view! {
                            <div class="quick-search-groups">{groups}</div>
                            <Show when=move || {
                                quick.viewed_expanded.get()
                                    && quick.text.with(|text| text.trim().is_empty())
                            }>
                                <p class="sr-only" role="status">
                                    {format!(
                                        "Showing all {} recently viewed entries",
                                        crate::fmt::count(viewed_total)
                                    )}
                                </p>
                            </Show>
                            {more}
                        }
                    }}
                </div>
            </Show>
        </dialog>
    }
}
