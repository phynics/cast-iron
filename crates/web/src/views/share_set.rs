//! Sharing the active books (#408): on Books, a disclosure that shows the code for the books that
//! are on now (Copy code, Copy link) and takes a code, and the confirmation that every code and
//! every `#/books?set=<code>` link opens. Nothing changes until the reader presses `Use this set`;
//! the write goes through `Library::apply_active_plan`, and Undo restores the earlier set.
//!
//! The codec and the plan are pure and native-tested (`book_code`, `book_share`); this file is
//! only the page around them.

use crate::book_code::{self, Shared};
use crate::book_presets::Change;
use crate::book_share::{self, Offer};
use crate::library::{Library, use_library};
use crate::route::{Route, replace, use_route};
use crate::views::active_books::focus_after_frame;
use crate::views::icons;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// The id of the confirmation heading. Focus moves there when a question appears.
const CONFIRM_HEADING: &str = "share-confirm-heading";

#[derive(Clone)]
struct Panel {
    library: Library,
    /// The route signal, held here because a write finishes outside any reactive owner.
    route: ReadSignal<Route>,
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
    /// `Used the shared set.` and `Restored your earlier choice.`
    status: RwSignal<Option<String>>,
    /// What Undo writes. It lasts until the next change on this page, as for the presets.
    undo: RwSignal<Option<Change>>,
    /// The line next to the copy buttons.
    copied: RwSignal<Option<String>>,
}

impl Panel {
    /// Reads `text` as a code. A good code waits for an answer; a bad one says why and changes
    /// nothing. Neither touches Undo: the earlier answer stays undoable until another selection
    /// change succeeds. `from_link` puts an error outside the disclosure, which a link's reader
    /// has not opened.
    fn review(&self, text: &str, from_link: bool) {
        self.notice.set(None);
        match book_code::decode(text) {
            Ok(shared) => {
                self.problem.set(None);
                self.pending.set(Some(shared));
                focus_after_frame(CONFIRM_HEADING);
            }
            Err(error) => {
                self.pending.set(None);
                if from_link {
                    self.problem.set(None);
                    self.notice.set(Some(format!(
                        "This link did not open a set of books. {}",
                        error.message()
                    )));
                } else {
                    self.problem.set(Some(error.message()));
                }
            }
        }
    }

    /// Ends the question, applied or not, and takes the code out of the address so a reload or
    /// Back does not ask again.
    fn close(&self) {
        self.pending.set(None);
        self.entered.set(String::new());
        if matches!(self.route.get_untracked(), Route::BooksSet(_)) {
            replace(&Route::Books);
        }
    }

    fn apply(&self) {
        let Some(shared) = self.pending.get_untracked() else {
            return;
        };
        self.copied.set(None);
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
            match result {
                Ok(change) if change.is_empty() => {}
                Ok(change) => {
                    panel
                        .status
                        .set(Some("Now using the shared set of books.".to_string()));
                    panel.undo.set(Some(change.inverse()));
                }
                Err(failure) => panel.notice.set(Some(format!(
                    "Could not save the book selection: {failure}"
                ))),
            }
            panel.close();
        });
    }

    fn undo(&self) {
        let Some(change) = self.undo.get_untracked() else {
            return;
        };
        self.copied.set(None);
        let panel = self.clone();
        spawn_local(async move {
            match panel
                .library
                .update_active_books(&change.activate, &change.deactivate)
                .await
            {
                Ok(()) => {
                    panel.undo.set(None);
                    panel
                        .status
                        .set(Some("Restored your earlier choice.".to_string()));
                }
                // The status line is outside the disclosure, so a link's reader sees it too.
                Err(failure) => panel.status.set(Some(format!(
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

/// The address of the page without its hash: where a link to this app starts.
fn page_address() -> String {
    web_sys::window()
        .and_then(|window| window.location().href().ok())
        .map(|href| href.split('#').next().unwrap_or("").to_string())
        .unwrap_or_default()
}

/// The question, the share disclosure and the status line. It renders nothing for a library
/// whose books carry no slots (compiled before #408): such a library cannot name a set.
#[component]
pub fn ShareSet() -> impl IntoView {
    let library = use_library();
    if book_share::slot_map(library.sources()).is_none() {
        return ().into_any();
    }
    let route = use_route();
    let panel = Panel {
        library: library.clone(),
        route,
        entered: RwSignal::new(String::new()),
        problem: RwSignal::new(None),
        notice: RwSignal::new(None),
        pending: RwSignal::new(None),
        status: RwSignal::new(None),
        undo: RwSignal::new(None),
        copied: RwSignal::new(None),
    };

    // A link opens the question. The address keeps the code until the reader answers.
    let linked = panel.clone();
    Effect::new(move |_| {
        if let Route::BooksSet(code) = route.get() {
            linked.entered.set(String::new());
            linked.review(&code, true);
            if linked.pending.get_untracked().is_none() {
                // A bad link has nothing left to answer; its error stays on the page.
                replace(&Route::Books);
            }
        }
    });

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
                        <button type="button" class="btn" on:click=move |_| cancel.close()>
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
    let status = panel.status;
    let undo_signal = panel.undo;
    let problem = panel.problem;
    let notice = panel.notice;
    let copied = panel.copied;
    view! {
        {move || notice.get().map(|message| view! {
            <p class="error share-notice" role="alert" data-testid="share-notice">{message}</p>
        })}
        {question}
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
        <div class="undo-line share-status" role="status" class:has-status=move || status.get().is_some()>
            {move || status.get()}
            {move || undo_signal.get().is_some().then(|| {
                let panel = undo_panel.clone();
                view! { <button type="button" class="text-btn" on:click=move |_| panel.undo()>"Undo"</button> }
            })}
        </div>
    }
    .into_any()
}
