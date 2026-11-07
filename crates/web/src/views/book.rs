//! The book view: one publication's chapters in the order of the book, as a contents tree, and
//! the previous/next control that an entry shows for its place in a book.
//!
//! Nothing in this module sorts. Every list below follows the draft order in `books.json`.

use crate::graph::{BookLookup, entries_under, node_at};
use crate::library::{Library, use_library};
use crate::route::{Route, current_hash, replace, toc_id, use_route};
use crate::views::active_books::save_book_active;
use crate::views::util::{Shared, book_count_line, local_resource, set_title};
use cast_iron_schema::{BookNode, EntryId};
use leptos::prelude::*;
use std::collections::HashSet;

/// Distinct entries that a book's contents tree reaches. Every placement counts once, whether
/// or not the book is active and whether or not the entry's first placement is another book.
pub fn book_entry_count(nodes: &[BookNode]) -> usize {
    fn walk<'a>(nodes: &'a [BookNode], seen: &mut HashSet<&'a EntryId>) {
        for node in nodes {
            if let Some(entry) = &node.entry {
                seen.insert(entry);
            }
            walk(&node.children, seen);
        }
    }
    let mut seen = HashSet::new();
    walk(nodes, &mut seen);
    seen.len()
}

/// The chapter path that a route asks for.
fn chapter_of(route: &Route, source: &str) -> Option<Vec<usize>> {
    match route {
        Route::Book {
            source: s,
            chapter: Some(path),
        } if s == source => Some(path.clone()),
        _ => None,
    }
}

/// True when `path` starts with `prefix`: the node is the target or one of its ancestors.
pub fn on_path(prefix: &[usize], target: &[usize]) -> bool {
    target.starts_with(prefix)
}

/// The path to record in the URL after a toggle: the node when it opened, its parent when it
/// closed, and nothing when a top-level chapter closed.
pub fn path_after_toggle(path: &[usize], opened: bool) -> Option<Vec<usize>> {
    if opened {
        Some(path.to_vec())
    } else {
        path.split_last()
            .map(|(_, parent)| parent.to_vec())
            .filter(|p| !p.is_empty())
    }
}

/// The book's own Active row: a toggle row (DESIGN.md, Toggles). The whole row is a `<label>`
/// that holds a native checkbox; the line under it says what the state means. The choice is
/// saved at once, the same stored choice that Active books writes.
#[component]
fn ActiveRow(library: Library, source: String) -> impl IntoView {
    let revision = library.selection_revision();
    let error = RwSignal::new(None::<String>);
    let state_library = library.clone();
    let state_source = source.clone();
    let is_active = move || {
        revision.get();
        state_library.book_active(&state_source)
    };
    let state = is_active.clone();
    view! {
        <div class="book-active-row">
            <label class="toggle-row">
                <input type="checkbox" aria-label="Active" aria-describedby="book-active-state"
                    prop:checked=is_active
                    on:change=move |event| {
                        let input = event_target::<web_sys::HtmlInputElement>(&event);
                        save_book_active(&library, source.clone(), input, error);
                    } />
                <span class="book-card-text">
                    <span class="cat-name">"Active"</span>
                    <span class="cat-blurb" id="book-active-state">
                        {move || if state() {
                            "Search and lists include this book."
                        } else {
                            "Search and lists leave this book out. You can still read its contents here."
                        }}
                    </span>
                </span>
            </label>
            {move || error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })}
        </div>
    }
}

#[component]
pub fn BookView(source: String) -> impl IntoView {
    let library = use_library();
    let info = library.source(&source).cloned();
    set_title(info.as_ref().map_or("Book", |s| s.title.as_str()));
    let lookup = {
        let library = library.clone();
        local_resource(async move { library.books().await })
    };
    let current = use_route();
    let initial = chapter_of(&current.get_untracked(), &source);
    let target = RwSignal::new(initial);
    {
        // Back and forward change the chapter in the URL without rebuilding this page.
        let source = source.clone();
        Effect::new(move |_| {
            let wanted = chapter_of(&current.get(), &source);
            if wanted != target.get_untracked() {
                target.set(wanted);
            }
        });
    }

    let title = info
        .as_ref()
        .map_or_else(|| source.clone(), |s| s.title.clone());
    let copyright = info.as_ref().and_then(|s| s.copyright.clone());
    let header_source = source.clone();
    let active_row = view! { <ActiveRow library=library.clone() source=source.clone() /> };
    view! {
        <div class="wide book">
            <nav class="crumbs" aria-label="Breadcrumb">
                <ol>
                    <li><a href=Route::Books.href()>"Books"</a></li>
                </ol>
            </nav>
            <h1>{title}</h1>
            {active_row}
            {move || {
                // The header waits for the same resource as the contents tree, so it counts
                // exactly the entries the tree can show.
                let source = header_source.clone();
                lookup.with(|state| match state {
                    Some(Ok(Some(books))) => match books.books().iter().position(|b| b.source == source) {
                        Some(index) => {
                            let n = books.book_at(index).map_or(0, |book| book_entry_count(&book.chapters));
                            view! { <p class="muted" data-testid="source-count">{book_count_line(n)}</p> }.into_any()
                        }
                        None => ().into_any(),
                    },
                    _ => ().into_any(),
                })
            }}
            {copyright.map(|line| view! { <p class="copyright">{line}</p> })}
            {move || {
                let source = source.clone();
                lookup.with(|state| match state {
                    None => view! { <p class="status" role="status">"Loading contents…"</p> }.into_any(),
                    Some(Err(error)) => view! {
                        <div class="error" role="alert">
                            <p>{error.to_string()}</p>
                            <p><a href=Route::Settings.href()>"Open Settings"</a></p>
                        </div>
                    }.into_any(),
                    Some(Ok(None)) => view! {
                        <p class="notice">"This library has no chapter order. Open Settings to check for a library update."</p>
                        <p><a href=Route::Settings.href()>"Open Settings"</a></p>
                    }.into_any(),
                    Some(Ok(Some(books))) => match books.books().iter().position(|b| b.source == source) {
                        Some(index) => view! {
                            <Contents lookup=Shared::from_rc(books.clone()) book=index source target />
                        }.into_any(),
                        None => view! {
                            <p class="notice">"This library has no chapter order for this book."</p>
                        }.into_any(),
                    },
                })
            }}
            <p class="muted"><a href=Route::Legal.href()>"Legal and attribution"</a></p>
        </div>
    }
}

/// The draft notice and the contents tree of one book.
#[component]
fn Contents(
    lookup: Shared<BookLookup>,
    book: usize,
    source: String,
    target: RwSignal<Option<Vec<usize>>>,
) -> impl IntoView {
    let Some(tree) = lookup.book_at(book) else {
        return ().into_any();
    };
    let label = format!("Contents of {}", tree.title);
    let chapters = tree.chapters.len();
    let items = (0..chapters)
        .map(|i| {
            view! { <TocNode lookup=lookup.clone() book source=source.clone() path=vec![i] target /> }
        })
        .collect_view();
    view! {
        <OrderNote basis=tree.basis.clone() review=tree.review.clone() />
        <nav class="toc" aria-label=label>
            <ol class="toc-list toc-top">{items}</ol>
        </nav>
    }
    .into_any()
}

/// Says in plain words that the order is a draft, and how the list was made.
#[component]
fn OrderNote(basis: String, review: Vec<String>) -> impl IntoView {
    let has_questions = !review.is_empty();
    let questions = review
        .into_iter()
        .map(|q| view! { <li>{q}</li> })
        .collect_view();
    let has_basis = !basis.is_empty();
    view! {
        <aside class="order-note" aria-labelledby="order-note-head">
            <h2 id="order-note-head" class="order-note-head">"Book contents still need review"</h2>
            <p>
                "This contents view is a draft. The source note below records what has been checked against printed contents. "
                "Sections may still be missing, misplaced, or titled differently in print."
            </p>
            <details class="order-note-more">
                <summary>"How this list was made"</summary>
                {has_basis.then(|| view! { <p>{basis}</p> })}
                {has_questions.then(|| view! {
                    <p class="order-note-label">"Open questions"</p>
                    <ul>{questions}</ul>
                })}
            </details>
        </aside>
    }
}

/// One row of the contents. A node with children is a disclosure whose children are built when
/// it first opens, so a book of two thousand entries costs a few dozen rows at first.
#[component]
fn TocNode(
    lookup: Shared<BookLookup>,
    book: usize,
    source: String,
    path: Vec<usize>,
    target: RwSignal<Option<Vec<usize>>>,
) -> impl IntoView {
    let Some(tree) = lookup.book_at(book) else {
        return ().into_any();
    };
    let Some(node) = node_at(tree, &path) else {
        return ().into_any();
    };
    let title = crate::views::util::section_label(&node.title);
    let id = toc_id(&path);
    let child_count = node.children.len();
    let review = node.review.clone();
    let under = entries_under(node);
    let is_current = {
        let path = path.clone();
        move || target.with(|t| t.as_deref() == Some(path.as_slice()))
    };

    if child_count == 0 {
        let link = lookup
            .target_of(tree, &path)
            .map(|(entry, anchor)| Route::Entry { id: entry, anchor }.href());
        let aria_current = is_current.clone();
        return match link {
            Some(href) => view! {
                <li class="toc-leaf">
                    <a id=id href=href aria-current=move || aria_current().then_some("location")>{title}</a>
                    {review.map(|why| view! { <span class="toc-flag" title=why>"check order"</span> })}
                </li>
            }
            .into_any(),
            None => view! { <li class="toc-leaf"><span id=id>{title}</span>{review.map(|why| view! { <span class="toc-flag" title=why>"check order"</span> })}</li> }.into_any(),
        };
    }

    let own_link = node.entry.as_ref().map(|entry| {
        Route::Entry {
            id: entry.clone(),
            anchor: node.anchor.clone(),
        }
        .href()
    });
    let opened =
        RwSignal::new(target.with_untracked(|t| t.as_deref().is_some_and(|t| on_path(&path, t))));
    {
        let path = path.clone();
        Effect::new(move |_| {
            if target.with(|t| t.as_deref().is_some_and(|t| on_path(&path, t))) {
                opened.set(true);
            }
        });
    }
    let on_toggle = {
        let path = path.clone();
        let source = source.clone();
        move |event: leptos::ev::Event| {
            // The browser fires `toggle` in a later task than the click. A Back or a link that
            // lands first leaves this page, and a late event must not write this book's URL over
            // the entry that the reader is now on. Read the address bar, not the route signal:
            // the signal follows the address a moment later.
            // An old `#/browse/source/<id>` link shows the same page, so it counts as this book.
            let on_this_book = matches!(
                Route::parse(&current_hash()),
                Route::Book { source: shown, .. } | Route::BrowseSource(shown) if shown == source
            );
            if !on_this_book {
                return;
            }
            let open = event_target::<web_sys::HtmlDetailsElement>(&event).open();
            // The page opens ancestors of the target itself; only a difference from the signal
            // is the reader's own doing.
            if open == opened.get_untracked() {
                return;
            }
            opened.set(open);
            let chapter = path_after_toggle(&path, open);
            target.set(chapter.clone());
            replace(&Route::Book {
                source: source.clone(),
                chapter,
            });
        }
    };
    let children = {
        let lookup = lookup.clone();
        let path = path.clone();
        let source = source.clone();
        let title = title.clone();
        let review = review.clone();
        move || {
            opened.get().then(|| {
                let items = (0..child_count)
                    .map(|i| {
                        let mut child = path.clone();
                        child.push(i);
                        view! {
                            <TocNode lookup=lookup.clone() book source=source.clone() path=child target />
                        }
                    })
                    .collect_view();
                let own = own_link.clone().map(|href| {
                    view! {
                        <li class="toc-leaf toc-self">
                            <a href=href>{format!("Read {title}")}</a>
                        </li>
                    }
                });
                let note = review.clone().map(|why| {
                    view! {
                        <li class="toc-note">
                            <strong>"Position unverified. "</strong>{why}
                        </li>
                    }
                });
                view! { <ul class="toc-list">{note}{own}{items}</ul> }
            })
        }
    };
    let flagged = review.is_some();
    view! {
        <li class="toc-branch">
            <details class="toc-details" prop:open=move || opened.get() on:toggle=on_toggle>
                <summary id=id aria-current=move || is_current().then_some("location")>
                    <span class="toc-title">{title}</span>
                    {flagged.then(|| view! { <span class="toc-flag">"check order"</span> })}
                    <span class="toc-count" aria-label=format!("{under} entries")>{under}</span>
                </summary>
                {children}
            </details>
        </li>
    }
    .into_any()
}

/// The place of an entry in its book: the chapters above it and its neighbours in book order.
/// It loads `books.json` after the entry is on screen and shows nothing when the library has no
/// book order or the entry is in no book.
#[component]
pub fn BookPosition(library: Library, id: EntryId) -> impl IntoView {
    let lookup = local_resource(async move { library.books().await });
    move || {
        let id = id.clone();
        lookup.with(|state| {
            let Some(Ok(Some(books))) = state else {
                return ().into_any();
            };
            let Some(place) = books.place(&id) else {
                return ().into_any();
            };
            let Some(book) = books.book_at(place.book) else {
                return ().into_any();
            };
            let source = book.source.clone();
            let book_href = Route::Book {
                source: source.clone(),
                chapter: None,
            }
            .href();
            let mut crumbs =
                vec![view! { <li><a href=book_href>{book.title.clone()}</a></li> }.into_any()];
            for step in books.trail(&id) {
                let href = Route::Book {
                    source: source.clone(),
                    chapter: Some(step.path),
                }
                .href();
                crumbs.push(view! { <li><a href=href>{crate::views::util::section_label(&step.title)}</a></li> }.into_any());
            }
            let (before, after) = books.neighbors(&id);
            let step = |stop: Option<&crate::graph::BookStop>, next: bool| {
                stop.map(|stop| {
                    let href = Route::entry(stop.entry.clone()).href();
                    let (class, dir) = if next {
                        ("step step-next", "Next in book")
                    } else {
                        ("step step-prev", "Previous in book")
                    };
                    view! {
                        <a class=class href=href rel=if next { "next" } else { "prev" }>
                            <span class="step-dir">{dir}</span>
                            <span class="step-title">{crate::views::util::section_label(&stop.title)}</span>
                        </a>
                    }
                })
            };
            view! {
                <nav class="book-bar" aria-label="Position in the book" data-testid="book-bar">
                    <p class="book-where">"In the book · draft order"</p>
                    <ol class="book-trail">{crumbs}</ol>
                    <div class="book-steps">
                        {step(before, false)}
                        {step(after, true)}
                    </div>
                </nav>
            }
            .into_any()
        })
    }
}

/// Keeps the Reader frame from scrolling or moving focus when only the chapter in the URL of the
/// same book changes: that is a toggle, and the reader is already looking at it.
pub fn same_book(previous: &Route, next: &Route) -> bool {
    matches!((previous, next), (Route::Book { source: a, .. }, Route::Book { source: b, .. }) if a == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_entry_count_uses_every_placement_once() {
        let node = |title: &str, entry: Option<&str>, children: Vec<BookNode>| BookNode {
            title: title.to_string(),
            entry: entry.map(|s| EntryId::parse(s).unwrap()),
            anchor: None,
            review: None,
            children,
        };
        let tree = vec![
            node(
                "Chapter",
                None,
                vec![
                    node("Repeated", Some("a/one"), vec![]),
                    node("Nested", None, vec![node("First", Some("a/two"), vec![])]),
                    node("Repeated again", Some("a/one"), vec![]),
                ],
            ),
            node("Second", Some("a/three"), vec![]),
        ];
        assert_eq!(book_entry_count(&tree), 3);
        assert_eq!(book_entry_count(&[]), 0);
    }

    #[test]
    fn a_node_is_open_when_the_target_is_inside_it() {
        assert!(on_path(&[2], &[2, 1, 4]));
        assert!(on_path(&[2, 1], &[2, 1]));
        assert!(!on_path(&[2, 1], &[2]));
        assert!(!on_path(&[1], &[2, 1]));
        // Sibling prefixes do not match: chapter 1 is not chapter 10.
        assert!(!on_path(&[1], &[10]));
    }

    #[test]
    fn toggling_records_the_node_or_its_parent() {
        assert_eq!(path_after_toggle(&[3, 1], true), Some(vec![3, 1]));
        assert_eq!(path_after_toggle(&[3, 1], false), Some(vec![3]));
        assert_eq!(path_after_toggle(&[3], false), None);
        assert_eq!(path_after_toggle(&[3], true), Some(vec![3]));
    }

    #[test]
    fn only_a_chapter_change_inside_one_book_is_a_toggle() {
        let book = |source: &str, chapter: Option<Vec<usize>>| Route::Book {
            source: source.into(),
            chapter,
        };
        assert!(same_book(&book("a", None), &book("a", Some(vec![1]))));
        assert!(!same_book(&book("a", None), &book("b", None)));
        assert!(!same_book(&Route::Home, &book("a", None)));
    }

    #[test]
    fn the_chapter_of_a_route_belongs_to_its_own_book() {
        let route = Route::Book {
            source: "a".into(),
            chapter: Some(vec![2]),
        };
        assert_eq!(chapter_of(&route, "a"), Some(vec![2]));
        assert_eq!(chapter_of(&route, "b"), None);
        assert_eq!(chapter_of(&Route::Home, "a"), None);
    }
}
