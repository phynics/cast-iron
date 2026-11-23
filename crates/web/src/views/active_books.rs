//! Manage books (`#/books/manage`): the presets and one checkbox per book. The old address
//! `#/books/active` parses to the same route.
//!
//! The page is a route, not local state: every change writes the stored choice at once through
//! `Library::update_active_books`, so nothing can be left half edited, a reload stays on the
//! page, and Settings can link to it. Books is the place to read; this is the place to change.
//! The presets are page-wide bulk actions, and Undo reverses the last one until the next change.
//! The `Find a book` field (#587) sits under the count line, above the presets, and narrows the
//! groups; a series bulk button still acts on its whole series. The share section (#408, moved
//! here from Books in #504) is split in two: its error and question sit above the field, and its
//! disclosure sits between the presets and the book groups. A code or a link changes the active
//! set, as a preset does, so it is a change made here. Each side keeps its own Undo line. An Undo
//! belongs to the selection revision that its change produced (see [`Recent`]), so any later
//! change that succeeds ends every Undo on the page, and an Undo that was clicked while another
//! change was saving does nothing.
//! An Adventures series group also carries its own on/off bulk button (#159); a tier itself
//! still has no control.

use crate::book_presets::{self, Change, Preset};
use crate::library::{Library, Planned, use_library};
use crate::route::{self, Route, replace, use_route};
use crate::views::book_find::{self, Find, Page as FindPage};
use crate::views::browse::{
    BookListEntry, BulkFactory, CountFactory, RowFactory, SeriesRender, book_rows, groups_view,
};
use crate::views::icons;
use crate::views::share_set::share_parts;
use crate::views::util::{Shared, set_title};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::Cell;
use wasm_bindgen::JsCast;

// ---- how the page is entered and left -------------------------------------------------------

thread_local! {
    /// Set by Done, so Books puts focus back on the `Manage books ›` link it was left from.
    static FOCUS_LINK_NEXT: Cell<bool> = const { Cell::new(false) };
}

/// The history-state field that marks the entry that the Books link pushed, so that Done can go
/// Back to that Books entry (which restores its scroll position) instead of stacking a new one.
/// It belongs to the entry, not to the page that is mounted: the entry that a share link pushes
/// on the same page has no mark, and Back from it would land on Manage books, not on Books.
/// The link sets it when it pushes the entry (see [`open_from_books`]), never beforehand.
const OPENED_FROM_BOOKS: &str = "openedFromBooks";

/// The id of the `Manage books ›` link on Books.
pub const MANAGE_LINK_ID: &str = "manage-books-link";

/// The click on the `Manage books ›` link. A plain click pushes the Manage books entry here and
/// marks that very entry, so Done knows it can go Back. A click that asks for something else
/// (another tab or window by Ctrl, Cmd or Shift, a download by Alt) is left to the browser: it
/// pushes nothing in this tab, so it marks nothing, and nothing is left over for the next page
/// that Manage books opens from somewhere else.
pub fn open_from_books(event: web_sys::MouseEvent) {
    if event.default_prevented()
        || event.button() != 0
        || event.ctrl_key()
        || event.meta_key()
        || event.shift_key()
        || event.alt_key()
    {
        return;
    }
    event.prevent_default();
    route::push_flagged(&Route::books_manage(), OPENED_FROM_BOOKS);
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

/// Like [`focus_after_frame`], but scrolls the element into view when it is off screen: the
/// question that a share link or a typed code opens can sit below the first screen.
pub fn focus_and_reveal_after_frame(id: &'static str) {
    request_animation_frame(move || {
        request_animation_frame(move || {
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(id))
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = element.focus();
            }
        });
    });
}

/// Leaves the page: Back when the Books link pushed this very entry, otherwise Books in its place.
fn done() {
    FOCUS_LINK_NEXT.with(|flag| flag.set(true));
    let history = web_sys::window().and_then(|window| window.history().ok());
    match history {
        Some(history) if route::current_entry_has_flag(OPENED_FROM_BOOKS) => {
            let _ = history.back();
        }
        _ => replace(&Route::books()),
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

/// What the last change on this page did, for the line under the controls that made it, and the
/// Undo that reverses it. It belongs to the selection revision that the change produced: it is
/// shown only while the selection is still at that revision, so any later change that succeeds
/// ends it, whichever control made that change. A write that fails changes nothing, and the
/// line stays. An Undo is also run against that revision (see
/// [`Library::apply_active_plan_at`]), so one that was clicked while another change was saving
/// does nothing once that change has landed.
#[derive(Clone)]
pub struct Recent {
    revision: u64,
    pub text: String,
    undo: Option<Change>,
}

impl Recent {
    /// A change that was written, with Undo to put the books back.
    pub fn applied(planned: &Planned, text: String) -> Recent {
        Recent {
            revision: planned.revision,
            text,
            undo: Some(planned.change.inverse()),
        }
    }

    /// An Undo that was written: the books are back, and there is nothing left to undo.
    pub fn restored(planned: &Planned) -> Recent {
        Recent {
            revision: planned.revision,
            text: "Restored your earlier choice.".to_string(),
            undo: None,
        }
    }

    /// The change that Undo writes, if the line has an Undo.
    pub fn undo_change(&self) -> Option<Change> {
        self.undo.clone()
    }

    /// The selection revision that Undo is bound to.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn has_undo(&self) -> bool {
        self.undo.is_some()
    }
}

/// The change to describe now: the last one made from `recent`, while no later change has
/// succeeded. Read it inside a view and the line follows the selection.
pub fn live_recent(
    library: &Library,
    recent: RwSignal<Option<Recent>>,
) -> impl Fn() -> Option<Recent> + Clone + 'static + use<> {
    let revision = library.selection_revision();
    move || {
        let now = revision.get();
        recent.get().filter(|recent| recent.revision == now)
    }
}

#[derive(Clone)]
struct Panel {
    library: Library,
    error: RwSignal<Option<String>>,
    recent: RwSignal<Option<Recent>>,
}

impl Panel {
    fn apply(&self, preset: Preset) {
        self.error.set(None);
        let panel = self.clone();
        spawn_local(async move {
            // The change is worked out inside the write path, after any earlier write has
            // landed. Two quick taps would otherwise plan the second preset from the set that
            // the first one is still replacing, and leave a stray book on or off.
            let library = panel.library.clone();
            let planned = library
                .apply_active_plan(|is_active| {
                    let books = book_presets::books_of(library.sources());
                    book_presets::plan(preset, &books, is_active)
                })
                .await;
            match planned {
                // The set already matched: nothing was written, so there is nothing to undo.
                Ok(planned) if planned.change.is_empty() => {}
                Ok(planned) => panel.recent.set(Some(Recent::applied(
                    &planned,
                    format!("Applied {}.", preset.label()),
                ))),
                Err(failure) => panel.error.set(Some(format!(
                    "Could not save the book selection: {failure}"
                ))),
            }
        });
    }

    /// Turns every book of one series group on or off. The change is planned inside the write
    /// path, so a quick second tap lands on the set the first write left.
    fn set_books(&self, ids: Vec<String>, active: bool) {
        self.error.set(None);
        let panel = self.clone();
        spawn_local(async move {
            let planned = panel
                .library
                .apply_active_plan(|is_active| {
                    let mut change = Change::default();
                    for id in &ids {
                        let already = is_active(id);
                        if active && !already {
                            change.activate.push(id.clone());
                        } else if !active && already {
                            change.deactivate.push(id.clone());
                        }
                    }
                    change
                })
                .await;
            if let Err(failure) = planned {
                panel.error.set(Some(format!(
                    "Could not save the book selection: {failure}"
                )));
            }
        });
    }

    fn undo(&self) {
        let live = live_recent(&self.library, self.recent);
        let Some(recent) = untrack(live) else {
            return;
        };
        let Some(change) = recent.undo_change() else {
            return;
        };
        self.error.set(None);
        let panel = self.clone();
        spawn_local(async move {
            // Run against the revision that this Undo describes, once every earlier write has
            // landed: a change that was saving when Undo was clicked has since replaced the set.
            let written = panel
                .library
                .apply_active_plan_at(recent.revision(), move |_| change)
                .await;
            match written {
                Ok(Some(planned)) => panel.recent.set(Some(Recent::restored(&planned))),
                // Another change landed first, so this Undo no longer describes the set.
                Ok(None) => {}
                // Nothing changed: the line, and its Undo, stay.
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
    let meta = crate::fmt::book_meta(row.publication_date.as_deref(), row.count);
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
                        <span class="preset-mark">{move || check().then(icons::preset_check)}</span>
                    </button>
                </li>
            }
        })
        .collect_view();
    view! { <ul class="preset-list">{rows}</ul> }
}

/// Manage books: the presets, then the same tier groups as Books with a checkbox per book.
#[component]
pub fn ActiveBooks() -> impl IntoView {
    set_title("Manage books");
    let library = use_library();
    let panel = Panel {
        library: library.clone(),
        error: RwSignal::new(None),
        recent: RwSignal::new(None),
    };
    // Entering moves focus to the heading, after the shell has moved it to `main`. A share link
    // opens with its question, which takes the focus instead.
    if !matches!(use_route().get_untracked(), Route::BooksSet(_)) {
        focus_after_frame("manage-books-heading");
    }

    let rows = book_rows(&library);
    let total = rows.len();
    let rows = Shared::new(rows);
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
    // Adventures rows are toggle rows; every series group shows its live active tally and a
    // bulk button that turns the whole group on or off.
    let row_panel = panel.clone();
    let row: Box<RowFactory> = Box::new(move |row| toggle_row(row, row_panel.clone()));
    let count_library = library.clone();
    let count_revision = library.selection_revision();
    let count: Box<CountFactory> = Box::new(move |ids: &[String]| {
        count_revision.get();
        let active = ids
            .iter()
            .filter(|id| count_library.book_active(id))
            .count();
        crate::views::browse::group_count_text(active, ids.len())
    });
    let bulk_library = library.clone();
    let bulk_revision = library.selection_revision();
    let bulk_panel = panel.clone();
    let bulk: Box<BulkFactory> = Box::new(move |label: &str, ids: Vec<String>| {
        let library = bulk_library.clone();
        let panel = bulk_panel.clone();
        let group = label.to_string();
        let state_library = library.clone();
        let state_ids = ids.clone();
        let all_active = move || {
            bulk_revision.get();
            state_ids.iter().all(|id| state_library.book_active(id))
        };
        let label_group = group.clone();
        let label_all = all_active.clone();
        let click_all = all_active.clone();
        let text_all = all_active;
        let click_ids = ids;
        let click_panel = panel;
        view! {
            <button type="button" class="text-btn series-bulk"
                aria-label=move || if label_all() {
                    format!("Turn off all {label_group} books")
                } else {
                    format!("Turn on all {label_group} books")
                }
                on:click=move |_| {
                    let turn_on = !click_all();
                    click_panel.set_books(click_ids.clone(), turn_on);
                }>
                {move || if text_all() { "Turn all off" } else { "Turn all on" }}
            </button>
        }
        .into_any()
    });
    let render = SeriesRender::new(row, count, Some(bulk));
    let find = Find::new(FindPage::Manage);
    let matched = book_find::matched(rows.clone(), find.needle);
    let groups = {
        let (rows, library, matched) = (rows.clone(), library.clone(), matched.clone());
        move || groups_view(&rows, &render.found(matched()), &library)
    };
    let found = move || matched().map_or(0, |ids| ids.len());
    let share = share_parts();
    let undo_panel = panel.clone();
    let live = live_recent(&library, panel.recent);
    let live_text = live.clone();
    let live_class = live.clone();
    let error = panel.error;
    view! {
        <div class="wide active-books">
            <div class="head-row">
                <h1 id="manage-books-heading" tabindex="-1">"Manage books"</h1>
                <button type="button" class="text-btn" on:click=move |_| done()>"Done"</button>
            </div>
            <p class="muted books-count" aria-live="polite">{line}</p>
            {move || (empty() == 0).then(|| view! {
                <p class="muted">"No books are active. Turn some on below."</p>
            })}
            {share.question}
            <div class="book-find-block">
                {find.field()}
                {find.status(found)}
            </div>
            <section aria-labelledby="presets-heading">
                <h2 id="presets-heading" class="sub-head">"Presets"</h2>
                {presets}
                <p class="undo-line" role="status" class:has-status=move || live_class().is_some()>
                    {move || live_text().map(|recent| recent.text)}
                    {move || live().is_some_and(|recent| recent.has_undo()).then(|| {
                        let panel = undo_panel.clone();
                        view! { <button type="button" class="text-btn" on:click=move |_| panel.undo()>"Undo"</button> }
                    })}
                </p>
            </section>
            {move || error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })}
            {share.disclosure}
            {groups}
        </div>
    }
}
