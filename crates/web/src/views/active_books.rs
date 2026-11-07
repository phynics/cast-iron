//! Active books (`#/books/active`): the presets and one checkbox per book.
//!
//! The page is a route, not local state: every change writes the stored choice at once through
//! `Library::update_active_books`, so nothing can be left half edited, a reload stays on the
//! page, and Settings can link to it. Books is the place to read; this is the place to change.
//! There are no bulk controls on a group. The presets are the only bulk action, and Undo
//! reverses the last one until the next change.

use crate::book_presets::{self, Change, Preset};
use crate::library::{Library, use_library};
use crate::route::{Route, replace};
use crate::views::browse::{BookListEntry, book_groups, book_summary, tier_section};
use crate::views::util::set_title;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::Cell;
use wasm_bindgen::JsCast;

// ---- how the page is entered and left -------------------------------------------------------

thread_local! {
    /// Set when the `Active books ›` link on Books is followed, so Done can go Back to that
    /// Books entry (which restores its scroll position) instead of stacking a new one.
    static OPENED_FROM_BOOKS: Cell<bool> = const { Cell::new(false) };
    /// Set by Done, so Books puts focus back on the `Active books ›` link it was left from.
    static FOCUS_LINK_NEXT: Cell<bool> = const { Cell::new(false) };
}

/// The id of the `Active books ›` link on Books.
pub const ACTIVE_LINK_ID: &str = "active-books-link";

/// Call from the Books link, before the route changes.
pub fn note_opened_from_books() {
    OPENED_FROM_BOOKS.with(|flag| flag.set(true));
}

/// Reads and clears the request that Done left. Books calls it once, when it is built.
pub fn take_link_focus() -> bool {
    FOCUS_LINK_NEXT.with(|flag| flag.replace(false))
}

/// Focuses an element after the frame has settled: the shell moves focus to `main` and restores
/// the scroll offset in its own animation frame, and this must come after.
pub fn focus_after_frame(id: &'static str) {
    request_animation_frame(move || {
        request_animation_frame(move || {
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(id))
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = element.focus_with_options(&options);
            }
        });
    });
}

/// Leaves the page: Back when Books is the previous entry, otherwise Books in its place.
fn done(opened_from_books: bool) {
    FOCUS_LINK_NEXT.with(|flag| flag.set(true));
    let history = web_sys::window().and_then(|window| window.history().ok());
    match history {
        Some(history) if opened_from_books => {
            let _ = history.back();
        }
        _ => replace(&Route::Books),
    }
}

// ---- writing a choice -----------------------------------------------------------------------

/// Saves a checkbox that the reader changed, now. The write waits for an earlier one, so the
/// checkbox is never disabled (it would lose keyboard focus). On failure the box goes back to the
/// stored state and the error line says why.
pub fn save_book_active(
    library: &Library,
    id: String,
    input: web_sys::HtmlInputElement,
    error: RwSignal<Option<String>>,
) {
    let active = input.checked();
    error.set(None);
    let library = library.clone();
    spawn_local(async move {
        if let Err(failure) = library.set_book_active(&id, active).await {
            input.set_checked(library.book_active(&id));
            error.set(Some(format!(
                "Could not save the book selection: {failure}"
            )));
        }
    });
}

/// The last preset, kept so Undo can reverse it. It lasts until the next change.
#[derive(Clone)]
struct Applied {
    undo: Change,
}

#[derive(Clone)]
struct Panel {
    library: Library,
    error: RwSignal<Option<String>>,
    status: RwSignal<Option<String>>,
    applied: RwSignal<Option<Applied>>,
}

impl Panel {
    /// A single-book change replaces the preset message: Undo would no longer describe the set.
    fn clear_status(&self) {
        self.status.set(None);
        self.applied.set(None);
    }

    fn apply(&self, preset: Preset) {
        let library = self.library.clone();
        let books = book_presets::books_of(library.sources());
        let change = book_presets::plan(preset, &books, |id| library.book_active(id));
        if change.is_empty() {
            return;
        }
        self.error.set(None);
        let panel = self.clone();
        spawn_local(async move {
            match panel
                .library
                .update_active_books(&change.activate, &change.deactivate)
                .await
            {
                Ok(()) => {
                    panel
                        .status
                        .set(Some(format!("Applied {}.", preset.label())));
                    panel.applied.set(Some(Applied {
                        undo: change.inverse(),
                    }));
                }
                Err(failure) => panel.error.set(Some(format!(
                    "Could not save the book selection: {failure}"
                ))),
            }
        });
    }

    fn undo(&self) {
        let Some(applied) = self.applied.get_untracked() else {
            return;
        };
        self.error.set(None);
        let panel = self.clone();
        spawn_local(async move {
            match panel
                .library
                .update_active_books(&applied.undo.activate, &applied.undo.deactivate)
                .await
            {
                Ok(()) => {
                    panel.applied.set(None);
                    panel
                        .status
                        .set(Some("Restored your earlier choice.".to_string()));
                }
                Err(failure) => panel.error.set(Some(format!(
                    "Could not save the book selection: {failure}"
                ))),
            }
        });
    }
}

// ---- views ----------------------------------------------------------------------------------

/// One book as a toggle row: the whole row is a `<label>` that holds a native checkbox. The
/// row has no chevron and does not navigate.
fn toggle_row(row: BookListEntry, panel: Panel) -> AnyView {
    let meta = book_summary(&row);
    let meta_id = format!("book-meta-{}", row.id);
    let label = format!("Active {}", row.title);
    let revision = panel.library.selection_revision();
    let state_library = panel.library.clone();
    let state_id = row.id.clone();
    let is_active = move || {
        revision.get();
        state_library.book_active(&state_id)
    };
    let id = row.id;
    let title = row.title;
    let described = meta_id.clone();
    view! {
        <li class="book-directory-item" data-book-id=id.clone()>
            <label class="toggle-row">
                <input type="checkbox" aria-label=label aria-describedby=described
                    prop:checked=is_active
                    on:change={
                        let id = id.clone();
                        move |event| {
                            let input = event_target::<web_sys::HtmlInputElement>(&event);
                            panel.clear_status();
                            save_book_active(&panel.library, id.clone(), input, panel.error);
                        }
                    } />
                <span class="book-card-text">
                    <span class="cat-name">{title}</span>
                    <span class="cat-blurb" id=meta_id>{meta}</span>
                </span>
            </label>
        </li>
    }
    .into_any()
}

/// The check mark of the preset that matches the active set.
fn check_mark() -> impl IntoView {
    view! {
        <svg class="preset-check" viewBox="0 0 24 24" width="20" height="20" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="m5 12.5 4.5 4.5L19 7.5"/>
        </svg>
    }
}

/// The presets: 44 px button rows, one pressed when it matches the active set.
fn preset_list(panel: &Panel) -> impl IntoView + use<> {
    let library = &panel.library;
    let books = book_presets::books_of(library.sources());
    let offered = book_presets::offered(&books);
    let rows = offered
        .into_iter()
        .map(|preset| {
            let size = book_presets::size(preset, &books);
            let noun = if size == 1 { "book" } else { "books" };
            let revision = library.selection_revision();
            let pressed_library = library.clone();
            let pressed = move || {
                revision.get();
                let books = book_presets::books_of(pressed_library.sources());
                book_presets::matching(&books, |id| pressed_library.book_active(id))
                    == Some(preset)
            };
            let check = pressed.clone();
            let apply = panel.clone();
            let size_id = format!("preset-size-{}", preset as u8);
            let size_id_for_button = size_id.clone();
            view! {
                <li>
                    <button type="button" class="preset-row" aria-label=preset.label()
                        aria-describedby=size_id_for_button
                        aria-pressed=move || if pressed() { "true" } else { "false" }
                        on:click=move |_| apply.apply(preset)>
                        <span class="preset-name">{preset.label()}</span>
                        <span class="preset-size" id=size_id>{format!("{} {noun}", crate::fmt::count(size))}</span>
                        <span class="preset-mark">{move || check().then(check_mark)}</span>
                    </button>
                </li>
            }
        })
        .collect_view();
    view! { <ul class="preset-list">{rows}</ul> }
}

/// Active books: the presets, then the same tier groups as Books with a checkbox per book.
#[component]
pub fn ActiveBooks() -> impl IntoView {
    set_title("Active books");
    let library = use_library();
    let opened_from_books = OPENED_FROM_BOOKS.with(|flag| flag.replace(false));
    let panel = Panel {
        library: library.clone(),
        error: RwSignal::new(None),
        status: RwSignal::new(None),
        applied: RwSignal::new(None),
    };
    // Entering moves focus to the heading, after the shell has moved it to `main`.
    focus_after_frame("active-books-heading");

    let counts = library.source_entry_counts();
    let rows: Vec<BookListEntry> = library
        .sources()
        .iter()
        .map(|source| BookListEntry {
            id: source.id.clone(),
            title: source.title.clone(),
            category: source.category.clone(),
            publication_date: source.publication_date.clone(),
            count: counts.get(&source.id).copied().unwrap_or(0),
        })
        .collect();
    let total = rows.len();
    let revision = library.selection_revision();
    let count_library = library.clone();
    let active = move || {
        revision.get();
        count_library
            .sources()
            .iter()
            .filter(|source| count_library.book_active(&source.id))
            .count()
    };
    let empty = active.clone();
    let line = move || book_presets::count_line(active(), total, None);
    let presets = preset_list(&panel);
    let groups = book_groups(&rows, None)
        .into_iter()
        .filter_map(|group| {
            let tier = group.tier?;
            let ids: Vec<String> = group.books.iter().map(|&i| rows[i].id.clone()).collect();
            let items = group
                .books
                .into_iter()
                .map(|i| toggle_row(rows[i].clone(), panel.clone()))
                .collect_view();
            Some(tier_section(tier, ids, items, library.clone()))
        })
        .collect_view();
    let undo_panel = panel.clone();
    let status_panel = panel.clone();
    let error = panel.error;
    view! {
        <div class="wide active-books">
            <div class="head-row">
                <h1 id="active-books-heading" tabindex="-1">"Active books"</h1>
                <button type="button" class="text-btn" on:click=move |_| done(opened_from_books)>"Done"</button>
            </div>
            <p class="muted books-count" aria-live="polite">{line}</p>
            {move || (empty() == 0).then(|| view! {
                <p class="muted">"Search and lists show nothing until a book is active."</p>
            })}
            <section aria-labelledby="presets-heading">
                <h2 id="presets-heading" class="sub-head">"Presets"</h2>
                {presets}
                <p class="undo-line" role="status">
                    {move || status_panel.status.get()}
                    {move || undo_panel.applied.get().is_some().then(|| {
                        let panel = undo_panel.clone();
                        view! { <button type="button" class="text-btn" on:click=move |_| panel.undo()>"Undo"</button> }
                    })}
                </p>
            </section>
            {move || error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })}
            {groups}
        </div>
    }
}
