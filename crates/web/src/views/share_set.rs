//! Sharing the active books (#408): on Manage books, a disclosure that shows the code for the books
//! that are on now (Copy code, Copy link) and takes a code, and the confirmation that every code
//! and every `#/books/manage?set=<code>` link opens. A link made while the section sat on Books
//! (`#/books?set=<code>`) is rewritten to that address by the router. Nothing changes until the reader presses `Use this set`;
//! the write goes through `Library::apply_active_plan`, and Undo restores the earlier set while
//! nothing else has changed it (see `active_books::Recent`).
//!
//! The codec and the plan are pure and native-tested (`book_code`, `book_share`); this file is
//! only the page around them.

use crate::book_code::{self, Shared};
use crate::book_presets;
use crate::book_share::{self, Offer};
use crate::library::{Library, use_library};
use crate::route::{self, Route, replace, use_route};
use crate::views::active_books::{Recent, focus_and_reveal_after_frame, live_recent};
use crate::views::icons;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// The id of the confirmation heading. Focus moves there when a question appears.
const CONFIRM_HEADING: &str = "share-confirm-heading";

/// The history-state field that marks the entry that holds a share link. Back and Forward change
/// the entry without changing the page, so the route alone cannot say that the reader left the
/// entry whose question is on screen: `#/books/manage` is also the address of the entry before it.
/// The mark is a number that is drawn once for the entry and lives only in that entry's history
/// state. An entry that another page pushes on top of it (Quick search) does not copy it, so a
/// task that finishes later and finds the mark on the entry on screen knows that entry is the one
/// it began on.
const LINK_ENTRY: &str = route::SHARE_LINK_FIELD;

#[derive(Clone)]
struct Panel {
    library: Library,
    /// The text in the `Enter a code` field.
    entered: RwSignal<String>,
    /// The line under the field when a typed code could not be read. It sits inside the
    /// disclosure, which the reader has open while typing.
    problem: RwSignal<Option<String>>,
    /// An error the reader must see whatever the disclosure's state: a link that did not open a
    /// set, or a write that failed. It sits outside the disclosure.
    notice: RwSignal<Option<String>>,
    /// The code that waits for an answer.
    pending: RwSignal<Option<Shared>>,
    /// What the last change made here did, with its Undo: `Applied Paizo Everything.`, `Now using
    /// 27 books.`, and `Restored your earlier choice.` It lasts while nothing else has changed
    /// the books.
    recent: RwSignal<Option<Recent>>,
    /// The line next to the copy buttons.
    copied: RwSignal<Option<String>>,
    /// Counts the reviews: a new code, a cancel, an answer, and leaving the entry that held the
    /// question each start the next number. A write that finishes late closes the question or
    /// reports its failure only while the number it started under is still the current one, so
    /// it can end the review it answered and no other.
    review_no: Arc<AtomicU64>,
    /// The mark of the history entry whose link opened the question or the error on screen, or
    /// zero (a mark is never zero). The question belongs to that entry: Back leaves it behind.
    link_entry: Arc<AtomicU32>,
    /// The mark of an entry whose link was answered while the reader was on another entry (Quick
    /// search's, say), so its address still holds the code. It is taken out when the reader is
    /// back on that entry. Zero when there is none.
    settled: Arc<AtomicU32>,
    /// The text that the current review read from the `Enter a code` field (empty for a link).
    /// Ending that review clears the field only while it still holds this text: a code that the
    /// reader pasted meanwhile and has not yet reviewed is theirs.
    reviewed: Arc<Mutex<String>>,
    /// False once the view is gone. A write that finishes after that has no screen to update.
    alive: Arc<AtomicBool>,
}

thread_local! {
    /// The marks of the link entries whose link was answered (cancelled, applied or failed). The
    /// entry that Quick search leaves behind over such an entry keeps the link's address; going
    /// Forward into it is no new link. It lives as long as the loaded app, not the panel:
    /// Settings unmounts the panel, and Forward must still find the marks when the panel is built
    /// again. A reload forgets them, and a leftover link entry then asks once more.
    static HANDLED: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
}

/// Whether the address on screen still carries a share link's code.
fn address_holds_link() -> bool {
    matches!(Route::parse(&route::current_hash()), Route::BooksSet(_))
}

impl Panel {
    /// Starts the next review and returns its number.
    fn next_review(&self) -> u64 {
        self.review_no.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn current_review(&self) -> u64 {
        self.review_no.load(Ordering::Relaxed)
    }

    /// Reads `text` as a code. A good code waits for an answer; a bad one says why and changes
    /// nothing. Neither touches Undo: the earlier answer stays undoable until another selection
    /// change succeeds. `from_link` puts an error outside the disclosure, which a link's reader
    /// has not opened, and ties the question to the history entry that holds the link.
    fn review(&self, text: &str, from_link: bool) {
        self.next_review();
        self.settled.store(0, Ordering::Relaxed);
        self.notice.set(None);
        // A copy note describes the code that was on screen; a new question makes it stale.
        self.copied.set(None);
        let mark = if from_link {
            route::mark_current_entry(LINK_ENTRY)
        } else {
            0
        };
        self.link_entry.store(mark, Ordering::Relaxed);
        *self.reviewed.lock().unwrap() = if from_link {
            String::new()
        } else {
            text.to_string()
        };
        match book_code::decode(text) {
            Ok(shared) => {
                self.problem.set(None);
                self.pending.set(Some(shared));
                focus_and_reveal_after_frame(CONFIRM_HEADING);
            }
            Err(error) => {
                self.pending.set(None);
                if from_link {
                    self.problem.set(None);
                    self.notice.set(Some(link_message(error)));
                } else {
                    self.problem.set(Some(error.message()));
                }
            }
        }
    }

    /// Whether the entry on screen is a layer that Quick search pushed over a link entry whose
    /// link has been answered.
    fn is_handled_layer(&self) -> bool {
        route::current_entry_mark(route::SHARE_LINK_LAYER_FIELD)
            .is_some_and(|mark| HANDLED.with_borrow(|handled| handled.contains(&mark)))
    }

    /// The reader moved to another history entry, by Back or Forward: the question and the error
    /// that a link opened belong to the entry that was left. The same entry with a new address
    /// (the rewrite that takes the code out) is not leaving, and neither is the entry that Quick
    /// search pushes on top of it: that is the same page with a layer over it, and Forward from
    /// the entry reaches it after the layer has closed.
    fn left_link_entry(&self) {
        let mark = self.link_entry.load(Ordering::Relaxed);
        if mark == 0
            || route::current_entry_mark(LINK_ENTRY) == Some(mark)
            || route::current_entry_has_flag(route::QUICK_SEARCH_FIELD)
        {
            return;
        }
        self.link_entry.store(0, Ordering::Relaxed);
        self.next_review();
        self.pending.set(None);
        self.notice.set(None);
    }

    /// Back or Forward fired `popstate`. When the reader is back on an entry whose link was
    /// answered meanwhile, the address gets rewritten now; any other move may leave a question.
    fn moved_to_entry(&self) {
        let settled = self.settled.load(Ordering::Relaxed);
        if settled != 0 && route::current_entry_mark(LINK_ENTRY) == Some(settled) {
            self.settled.store(0, Ordering::Relaxed);
            if address_holds_link() {
                replace(&Route::books_manage());
            }
            return;
        }
        self.left_link_entry();
    }

    /// The reader cancelled the question on screen. A write that is still saving it answers
    /// nothing now: its books are saved all the same, and the line for them says so.
    fn cancel(&self) {
        self.next_review();
        self.finish(route::current_entry_mark(LINK_ENTRY));
    }

    /// Ends the question, applied or not, and takes its code out of the address so a reload or
    /// Back does not ask again. `asked_on` is the mark of the entry the reader was on when
    /// answering (`None`: an entry that holds no link). The address that gets rewritten is that
    /// entry's, and only while it is the entry on screen: when the reader has since opened
    /// another entry on top (Quick search), that one is not rewritten, and the link's own is
    /// rewritten when the reader comes back to it.
    fn finish(&self, asked_on: Option<u32>) {
        let own = self.link_entry.swap(0, Ordering::Relaxed);
        HANDLED.with_borrow_mut(|handled| {
            handled.extend(asked_on.into_iter().chain((own != 0).then_some(own)));
        });
        self.pending.set(None);
        if self.entered.get_untracked() == *self.reviewed.lock().unwrap() {
            self.entered.set(String::new());
        }
        if !address_holds_link() {
            return;
        }
        if route::current_entry_mark(LINK_ENTRY) == asked_on {
            replace(&Route::books_manage());
        } else if let Some(mark) = asked_on {
            self.settled.store(mark, Ordering::Relaxed);
        }
    }

    fn apply(&self) {
        let Some(shared) = self.pending.get_untracked() else {
            return;
        };
        self.copied.set(None);
        let review = self.current_review();
        let asked_on = route::current_entry_mark(LINK_ENTRY);
        let panel = self.clone();
        spawn_local(async move {
            // Worked out inside the write, after any earlier write has landed, as a preset is.
            let library = panel.library.clone();
            let result = library
                .apply_active_plan(|is_active| {
                    book_share::offer(&shared, library.sources(), is_active)
                        .map(|offer| offer.change)
                        .unwrap_or_default()
                })
                .await;
            // The reader may have left the page while this saved. The books are saved either way;
            // nothing on a page that is gone can be updated.
            if !panel.alive.load(Ordering::Relaxed) {
                return;
            }
            // Whether the question on screen is still the one this write answers. The reader may
            // have reviewed another code, cancelled, or left the entry while it saved.
            let current = panel.current_review() == review;
            match result {
                Ok(planned) if planned.change.is_empty() => {}
                // The books changed, whatever the screen shows now: the line follows the books.
                Ok(planned) => {
                    let text = applied_line(&panel.library);
                    panel.recent.set(Some(Recent::applied(&planned, text)));
                }
                // A failure belongs to its own review: next to another question it would read as
                // that question's failure.
                Err(failure) if current => panel.notice.set(Some(format!(
                    "Could not save the book selection: {failure}"
                ))),
                Err(_) => {}
            }
            if current {
                panel.next_review();
                panel.finish(asked_on);
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
        self.copied.set(None);
        let panel = self.clone();
        spawn_local(async move {
            // Run against the revision that this Undo describes, once every earlier write has
            // landed: a change that was saving when Undo was clicked has since replaced the set,
            // and Undo must not put back books that it chose on purpose.
            let written = panel
                .library
                .apply_active_plan_at(recent.revision(), move |_| change)
                .await;
            if !panel.alive.load(Ordering::Relaxed) {
                return;
            }
            match written {
                Ok(Some(planned)) => panel.recent.set(Some(Recent::restored(&planned))),
                // Another change landed first, so this Undo no longer describes the set.
                Ok(None) => {}
                // Nothing changed: the line, and its Undo, stay.
                Err(failure) => panel.notice.set(Some(format!(
                    "Could not save the book selection: {failure}"
                ))),
            }
        });
    }

    fn copy(&self, text: String, done: &'static str) {
        let copied = self.copied;
        copied.set(None);
        spawn_local(async move {
            match crate::js::copy_text(&text).await {
                Ok(()) => copied.set(Some(done.to_string())),
                Err(_) => copied.set(Some(
                    "Could not copy. Press and hold the code to select it.".to_string(),
                )),
            }
        });
    }
}

/// What a bad link says. It names the link's code, then the same advice as a typed code.
fn link_message(error: book_code::CodeError) -> String {
    let advice = error.message();
    match advice.strip_prefix("This code ") {
        Some(rest) => format!("This link's code {rest}"),
        None => format!("This link's code could not be read. {advice}"),
    }
}

/// The status line after a set was applied: the preset it made, else how many books are on.
/// Read from the library after the write, so it describes what the reader now has.
fn applied_line(library: &Library) -> String {
    let books = book_presets::books_of(library.sources());
    match book_presets::matching(&books, |id| library.book_active(id)) {
        Some(preset) => format!("Applied {}.", preset.label()),
        None => {
            let active = library
                .sources()
                .iter()
                .filter(|source| library.book_active(&source.id))
                .count();
            format!(
                "Now using {} {}.",
                crate::fmt::count(active),
                if active == 1 { "book" } else { "books" }
            )
        }
    }
}

/// The address of the page without its hash: where a link to this app starts.
fn page_address() -> String {
    web_sys::window()
        .and_then(|window| window.location().href().ok())
        .map(|href| href.split('#').next().unwrap_or("").to_string())
        .unwrap_or_default()
}

/// The share section in two places on Manage books: the error and the question go above the
/// `Find a book` field, where a link's question is seen first, and the disclosure with its status
/// line goes after the presets.
pub struct ShareParts {
    pub question: AnyView,
    pub disclosure: AnyView,
}

/// The error and the question, the share disclosure and the status line. They render nothing for
/// a library whose books carry no slots (compiled before #408): such a library cannot name a set.
/// Call it where the page is built: it holds the state of both parts.
pub fn share_parts() -> ShareParts {
    let library = use_library();
    if book_share::slot_map(library.sources()).is_none() {
        return ShareParts {
            question: ().into_any(),
            disclosure: ().into_any(),
        };
    }
    let route = use_route();
    let panel = Panel {
        library: library.clone(),
        entered: RwSignal::new(String::new()),
        problem: RwSignal::new(None),
        notice: RwSignal::new(None),
        pending: RwSignal::new(None),
        recent: RwSignal::new(None),
        copied: RwSignal::new(None),
        review_no: Arc::new(AtomicU64::new(0)),
        link_entry: Arc::new(AtomicU32::new(0)),
        settled: Arc::new(AtomicU32::new(0)),
        reviewed: Arc::new(Mutex::new(String::new())),
        alive: Arc::new(AtomicBool::new(true)),
    };
    let alive = panel.alive.clone();
    on_cleanup(move || alive.store(false, Ordering::Relaxed));

    // A link opens the question. The address keeps the code until the reader answers or types in
    // Find a book. Leaving the link's history entry by Back or Forward takes the question away.
    let linked = panel.clone();
    crate::views::util::effect(move || match route.get() {
        Route::BooksSet(code) => {
            // The entry that Quick search left over an answered link keeps the link's address.
            if linked.is_handled_layer() {
                replace(&Route::books_manage());
                return;
            }
            linked.entered.set(String::new());
            linked.review(&code, true);
            if linked.pending.get_untracked().is_none() {
                // A bad link has nothing left to answer; its error stays on the page.
                replace(&Route::books_manage());
            }
        }
        _ => linked.left_link_entry(),
    });
    // Back or Forward between two entries with the same address (a link answered or rejected
    // there, and the entry before it; the entry that Quick search pushes) fires `popstate` and
    // no `hashchange`: the route does not change, so the effect above cannot see it.
    let popped = panel.clone();
    let history = window_event_listener(leptos::ev::popstate, move |_| popped.moved_to_entry());
    on_cleanup(move || history.remove());

    let revision = library.selection_revision();
    let code_library = library.clone();
    let code = Memo::new(move |_| {
        revision.get();
        book_share::code_for(code_library.sources(), |id| code_library.book_active(id))
    });
    let offer_library = library.clone();
    let offer = {
        let pending = panel.pending;
        move || -> Option<Offer> {
            revision.get();
            let shared = pending.get()?;
            book_share::offer(&shared, offer_library.sources(), |id| {
                offer_library.book_active(id)
            })
        }
    };

    let question = {
        let panel = panel.clone();
        move || {
            let offer = offer()?;
            let apply = panel.clone();
            let cancel = panel.clone();
            let empty = offer.is_empty();
            let skipped = offer.skipped_line();
            Some(view! {
                <section class="share-confirm" aria-labelledby=CONFIRM_HEADING>
                    <h2 id=CONFIRM_HEADING class="sub-head" tabindex="-1">
                        {if empty { "Same books" } else { "Use this set of books?" }}
                    </h2>
                    <p class="share-summary" data-testid="share-summary">{offer.summary()}</p>
                    <p class="muted share-result" data-testid="share-result">
                        {format!("Afterwards: {}", offer.result_line())}
                    </p>
                    {skipped.map(|line| view! {
                        <p class="muted share-skipped" data-testid="share-skipped">{line}</p>
                    })}
                    <div class="share-actions">
                        {(!empty).then(|| view! {
                            <button type="button" class="btn btn-primary"
                                on:click=move |_| apply.apply()>"Use this set"</button>
                        })}
                        <button type="button" class="btn"
                            on:click=move |_| cancel.cancel()>
                            {if empty { "Close" } else { "Cancel" }}
                        </button>
                    </div>
                </section>
            })
        }
    };

    let copy_code = panel.clone();
    let copy_link = panel.clone();
    let submit = panel.clone();
    let typed = panel.clone();
    let value = panel.clone();
    let undo_panel = panel.clone();
    let live = live_recent(&library, panel.recent);
    let live_text = live.clone();
    let live_class = live.clone();
    let problem = panel.problem;
    let notice = panel.notice;
    let copied = panel.copied;
    let question = view! {
        {move || notice.get().map(|message| view! {
            <p class="error share-notice" role="alert" data-testid="share-notice">{message}</p>
        })}
        {question}
    }
    .into_any();
    let disclosure = view! {
        <details class="share-set">
            <summary>
                {icons::chevron_right("disclosure-chevron")}
                <span>"Share these books or enter a code"</span>
            </summary>
            <div class="share-body">
                <p class="muted">"A code names the books that are on. Send it to another player, who enters it here."</p>
                <p class="share-code-line">
                    <code class="share-code" data-testid="share-code">{move || code.get()}</code>
                </p>
                <div class="share-actions">
                    <button type="button" class="btn" on:click=move |_| {
                        if let Some(text) = code.get_untracked() {
                            copy_code.copy(text, "Copied the code.");
                        }
                    }>"Copy code"</button>
                    <button type="button" class="btn" on:click=move |_| {
                        if let Some(text) = code.get_untracked() {
                            let link = format!("{}{}", page_address(), Route::BooksSet(text).href());
                            copy_link.copy(link, "Copied the link.");
                        }
                    }>"Copy link"</button>
                </div>
                <p class="muted share-note" role="status">{move || copied.get()}</p>
                <form class="share-form" on:submit=move |event| {
                    event.prevent_default();
                    let text = submit.entered.get_untracked();
                    submit.review(&text, false);
                }>
                    <label for="share-entry">"Enter a code"</label>
                    <div class="share-entry">
                        <input id="share-entry" type="text" autocomplete="off" autocapitalize="characters"
                            spellcheck="false"
                            prop:value=move || value.entered.get()
                            on:input=move |event| {
                                let input = event_target::<web_sys::HtmlInputElement>(&event);
                                typed.entered.set(input.value());
                                typed.problem.set(None);
                            } />
                        <button type="submit" class="btn">"Review"</button>
                    </div>
                    {move || problem.get().map(|message| view! {
                        <p class="error" role="alert" data-testid="share-problem">{message}</p>
                    })}
                </form>
            </div>
        </details>
        <div class="undo-line share-status" role="status" class:has-status=move || live_class().is_some()>
            {move || live_text().map(|recent| recent.text)}
            {move || live().is_some_and(|recent| recent.has_undo()).then(|| {
                let panel = undo_panel.clone();
                view! { <button type="button" class="text-btn" on:click=move |_| panel.undo()>"Undo"</button> }
            })}
        </div>
    }
    .into_any();
    ShareParts {
        question,
        disclosure,
    }
}
