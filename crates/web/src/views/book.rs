//! The book view: one publication's chapters in the order of the book, as an expanded contents
//! list, and the previous/next control that an entry shows for its place in a book.
//!
//! Nothing in this module sorts. Every list below follows the draft order in `books.json`.
//! Contents leaves out every kind that has its own list page and the `In this book` links reach
//! (see `cast_iron_schema::books::LEAVE_OUT_LINKED_KINDS`); the page keeps every chapter's real
//! path, so a chapter link,
//! `c=3.2` in the address and the trail above an entry all mean what they meant.
//!
//! A node that is itself an entry collapses to one row when its own page lists every entry below
//! it (`BookNode::lists_children`, computed by the compiler). Reading order does not change:
//! previous and next still visit every child in book order.

use crate::fmt;
use crate::graph::{BookLookup, node_at};
use crate::library::{Library, use_library};
use crate::route::{Route, toc_id, use_route};
use crate::views::continue_reading;
use crate::views::icons;
use crate::views::util::{Shared, local_resource, set_title};
use cast_iron_schema::books::{
    CATCH_ALL_LABEL, CATCH_ALL_TITLE, ChapterRow, Hide, chapter_rows as schema_chapter_rows,
    hidden_from_contents, listed_children, listed_under,
};
use cast_iron_schema::{BookNode, BookTree, EntryId, EntryKind};
use leptos::prelude::*;
use leptos::task::spawn_local;
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

/// The kinds a book holds, with their counts, in the order of Home's category directory. Empty
/// kinds and Rules are left out: Rules have no list page, and the kind links only go to one.
pub fn kinds_in_book(counts: &[(EntryKind, usize)]) -> Vec<(EntryKind, usize)> {
    let (primary, more) = crate::views::categories::arrange(counts);
    primary.into_iter().chain(more).collect()
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

/// True when a node that is itself an entry shows as one row, because its own page already
/// lists every entry below it.
///
/// The compiler records the body-link test on [`BookNode::lists_children`]; the book page must
/// not load an entry body to decide a row. A hidden entry cannot be the row's link, so its node
/// keeps the branch for whatever its children still show.
pub fn hub_collapses(node: &BookNode, hide: Hide<'_>) -> bool {
    node.lists_children && node.entry.as_ref().is_some_and(|entry| !hide(entry))
}

/// The label of a chapter or section row.
pub fn chapter_label(title: &str) -> String {
    let title = if title == CATCH_ALL_TITLE {
        CATCH_ALL_LABEL
    } else {
        title
    };
    crate::views::util::section_label(title)
}

/// The chapter pages of a book, in Contents order, with the labels the reader sees.
pub fn chapter_rows(book: &BookTree, hide: Hide<'_>) -> Vec<ChapterRow> {
    schema_chapter_rows(book, hide)
        .into_iter()
        .map(|row| ChapterRow {
            title: chapter_label(&row.title),
            ..row
        })
        .collect()
}

/// One entry in a book's Contents reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingStop {
    pub entry: EntryId,
    pub title: String,
    pub path: Vec<usize>,
}

/// The entries a chapter page's reading order visits: every Contents-visible entry in book
/// order. A hidden kind is skipped, whether it is a leaf or a chapter of its own, exactly as
/// Contents leaves it out.
pub fn reading_order(book: &BookTree, hide: Hide<'_>) -> Vec<ReadingStop> {
    fn walk(node: &BookNode, path: &mut Vec<usize>, hide: Hide<'_>, out: &mut Vec<ReadingStop>) {
        if let Some(entry) = &node.entry
            && !hide(entry)
        {
            out.push(ReadingStop {
                entry: entry.clone(),
                title: node.title.clone(),
                path: path.clone(),
            });
        }
        for (index, child) in node.children.iter().enumerate() {
            path.push(index);
            walk(child, path, hide, out);
            path.pop();
        }
    }
    let mut out = Vec::new();
    let mut path = Vec::new();
    for (index, chapter) in book.chapters.iter().enumerate() {
        path.push(index);
        walk(chapter, &mut path, hide, &mut out);
        path.pop();
    }
    out
}

/// A neighbour in the reading order. `chapter` is the chapter page the stop belongs to, and
/// `crossing` says the step leaves the current chapter, which the footer then names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingNeighbor {
    pub entry: EntryId,
    pub title: String,
    pub chapter: String,
    pub crossing: bool,
}

/// The previous and next Contents-visible entries around `id` in `book`. Crossing a chapter
/// boundary keeps the next chapter's first entry, so reading never stops at a chapter end.
pub fn reading_neighbors(
    book: &BookTree,
    hide: Hide<'_>,
    id: &EntryId,
) -> (Option<ReadingNeighbor>, Option<ReadingNeighbor>) {
    let order = reading_order(book, hide);
    let Some(at) = order.iter().position(|stop| &stop.entry == id) else {
        return (None, None);
    };
    let rows = chapter_rows(book, hide);
    let chapter_of = |path: &[usize]| {
        rows.iter()
            .position(|row| path.starts_with(&row.path))
            .map(|index| (index, rows[index].title.clone()))
    };
    let here = chapter_of(&order[at].path);
    let step = |at: usize| {
        let stop = &order[at];
        let chapter = chapter_of(&stop.path);
        ReadingNeighbor {
            entry: stop.entry.clone(),
            title: stop.title.clone(),
            chapter: chapter
                .as_ref()
                .map_or_else(String::new, |(_, title)| title.clone()),
            crossing: chapter.as_ref().map(|(index, _)| *index)
                != here.as_ref().map(|(index, _)| *index),
        }
    };
    let before = at.checked_sub(1).map(step);
    let after = (at + 1 < order.len()).then(|| step(at + 1));
    (before, after)
}

/// The one line under `Contents`. Every book's order is a draft until its evidence is complete
/// (PRODUCT.md), so a book with review items says so, quietly; the long notes stay in
/// `docs/research/book-order-evidence.md`.
pub const REVIEW_LINE: &str = "Chapter order not yet checked against the printed book";

/// The review line for a book, or `None` when the compiler recorded no open question.
pub fn review_line(review: &[String]) -> Option<&'static str> {
    (!review.is_empty()).then_some(REVIEW_LINE)
}

/// The one line that a book page shows while the book is turned off. It states what the reader
/// loses, and nothing else: the choice itself lives on Manage books.
pub const BOOK_OFF_LINE: &str =
    "This book is turned off. Its entries don't appear in search or lists.";

#[component]
pub fn BookView(source: String) -> impl IntoView {
    let library = use_library();
    let info = library.source(&source).cloned();
    set_title(info.as_ref().map_or("Book", |s| s.title.as_str()));
    let lookup = {
        let library = library.clone();
        local_resource(async move { library.books().await })
    };
    // The reader's place in this book, read once per mount. The page remounts when Back returns
    // from an entry, so a visit made there shows here without a reload.
    let saved = RwSignal::new(None::<EntryId>);
    {
        let library = library.clone();
        let source = source.clone();
        spawn_local(async move {
            let points = continue_reading::load(&library).await;
            saved.set(
                continue_reading::for_book(&points, &source).map(|point| point.entry_id.clone()),
            );
        });
    }
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
    // The same line and the same count as the book's row on Books. The catalog count is known
    // at once, so the line does not wait for `books.json`.
    let meta = info.as_ref().map(|s| {
        let entries = library
            .source_entry_counts()
            .get(&s.id)
            .copied()
            .unwrap_or(0);
        fmt::book_meta(s.publication_date.as_deref(), entries)
    });
    // The notices are printed once, at the bottom, each distinct notice once.
    let notices = info
        .as_ref()
        .and_then(|s| s.copyright.as_deref())
        .map(fmt::notice_lines)
        .unwrap_or_default()
        .into_iter()
        .map(|line| view! { <p class="copyright">{line}</p> })
        .collect_view();
    // The page shows one plain line while the book is off, and nothing when it is on. The
    // selection is read reactively so a change written elsewhere reaches this page too.
    let revision = library.selection_revision();
    let state_library = library.clone();
    let state_source = source.clone();
    let book_off = move || {
        revision.get();
        !state_library.book_active(&state_source)
    };
    // The `In this book` links count the catalog entries whose canonical book this is, so they
    // agree with the Books row and reach that same set whether or not the book is active.
    let kind_counts = {
        let mut counts = Vec::with_capacity(EntryKind::ALL.len());
        for kind in EntryKind::ALL {
            let n = library
                .catalog()
                .entries
                .iter()
                .filter(|stub| stub.kind == kind && library.stub_is_in_source(stub, &source))
                .count();
            counts.push((kind, n));
        }
        kinds_in_book(&counts)
    };
    let kind_links = (!kind_counts.is_empty()).then(|| {
        let rows = kind_counts
            .iter()
            .map(|&(kind, n)| {
                let href = Route::browse_kind(kind)
                    .with_source(Some(source.clone()))
                    .href();
                view! {
                    <li>
                        <a class="jump" href=href>
                            <span class="book-kind-label">{kind.label()}</span>
                            <span class="book-kind-count">{fmt::count(n)}</span>
                        </a>
                    </li>
                }
            })
            .collect_view();
        view! {
            <section class="book-in-this-book" aria-labelledby="book-in-this-book-head">
                <h2 id="book-in-this-book-head">"In this book"</h2>
                <ul class="jump-list book-kinds">{rows}</ul>
            </section>
        }
    });
    // The saved place in this book, under the meta line. A point whose entry left the library
    // was already dropped by the load, so this only fails when there is nothing to show.
    let continue_link = {
        let library = library.clone();
        let source = source.clone();
        move || {
            let id = saved.get()?;
            let stub = library.stub(&id)?;
            let href = Route::Entry {
                id,
                anchor: None,
                book: Some(source.clone()),
            }
            .href();
            Some(view! {
                <p class="continue-reading" data-testid="continue-reading">
                    <a href=href>"Continue reading: "{stub.title.clone()}</a>
                </p>
            })
        }
    };
    view! {
        <div class="wide book">
            <h1>{title}</h1>
            {meta.map(|line| view! { <p class="muted book-meta-line" data-testid="source-count">{line}</p> })}
            {move || book_off().then(|| view! {
                <p class="muted book-off-line" data-testid="book-off-line">{BOOK_OFF_LINE}</p>
            })}
            {continue_link}
            {kind_links}
            <section class="book-contents" aria-labelledby="book-contents-head">
                <h2 id="book-contents-head">"Contents"</h2>
                {move || {
                    let source = source.clone();
                    let library = library.clone();
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
                                <Contents lookup=Shared::from_rc(books.clone()) library book=index target />
                            }.into_any(),
                            None => view! {
                                <p class="notice">"This library has no chapter order for this book."</p>
                            }.into_any(),
                        },
                    })
                }}
            </section>
            <footer class="colophon">
                {notices}
                <p class="muted"><a href=Route::Legal.href()>"Legal and attribution"</a></p>
            </footer>
        </div>
    }
}

/// One chapter page: the chapter at full width, its first-level sections as headings and every
/// deeper level as a plain indented list. Nothing here folds, except that a section whose own
/// page lists its children shows only its link; the page is for reading downward.
#[component]
pub fn ChapterView(
    source: String,
    chapter: String,
    /// A section row to scroll to after the chapter page renders.
    anchor: Option<String>,
) -> impl IntoView {
    let library = use_library();
    let lookup = {
        let library = library.clone();
        local_resource(async move { library.books().await })
    };
    let current = use_route();
    {
        // The chapter page arrives after `books.json` loads, so the frame's own anchor scroll
        // may run before the row exists. Repeat it when the content is on screen.
        Effect::new(move |_| {
            let wanted = anchor
                .clone()
                .or_else(|| current.with(|route| route.anchor().map(|a| a.into_owned())));
            let ready = lookup.with(|state| matches!(state, Some(Ok(Some(_)))));
            if let (true, Some(anchor)) = (ready, wanted) {
                request_animation_frame(move || {
                    crate::route::scroll_to_anchor(&anchor);
                });
            }
        });
    }
    let revision = library.selection_revision();
    let state_library = library.clone();
    let state_source = source.clone();
    let book_off = move || {
        revision.get();
        !state_library.book_active(&state_source)
    };
    let source_for_view = source.clone();
    view! {
        <div class="wide book chapter-page">
            {move || {
                let source = source_for_view.clone();
                let library = library.clone();
                lookup.with(|state| match state {
                    None => view! { <p class="status" role="status">"Loading chapter…"</p> }.into_any(),
                    Some(Err(error)) => view! {
                        <div class="error" role="alert"><p>{error.to_string()}</p></div>
                    }.into_any(),
                    Some(Ok(None)) => view! {
                        <p class="notice">"This library has no chapter order. Open Settings to check for a library update."</p>
                    }.into_any(),
                    Some(Ok(Some(books))) => {
                        let Some(tree) = books.book(&source) else {
                            return view! {
                                <p class="notice">"This library has no chapter order for this book."</p>
                            }.into_any();
                        };
                        let hide = |id: &EntryId| {
                            library
                                .stub(id)
                                .is_some_and(|stub| hidden_from_contents(stub.kind))
                        };
                        let rows = chapter_rows(tree, &hide);
                        let Some(row) = rows.iter().find(|row| row.id == chapter) else {
                            return view! {
                                <p class="notice">"This chapter is not in this book."</p>
                                <p><a href=Route::Book { source: source.clone(), chapter: None }.href()>"Back to the book"</a></p>
                            }.into_any();
                        };
                        let Some(node) = node_at(tree, &row.path) else {
                            return ().into_any();
                        };
                        set_title(&row.title);
                        let book_href = Route::Book {
                            source: source.clone(),
                            chapter: None,
                        }
                        .href();
                        let own = node
                            .entry
                            .as_ref()
                            .filter(|entry| !hide(entry))
                            .map(|entry| {
                                Route::Entry {
                                    id: entry.clone(),
                                    anchor: node.anchor.clone(),
                                    book: Some(source.clone()),
                                }
                                .href()
                            });
                        let sections = listed_children(node, &hide)
                            .into_iter()
                            .filter_map(|index| {
                                let child = node.children.get(index)?;
                                let mut path = row.path.clone();
                                path.push(index);
                                Some(chapter_section(child, &source, &hide, &path))
                            })
                            .collect_view();
                        view! {
                            <nav class="crumbs" aria-label="Breadcrumb">
                                <ol>
                                    <li><a href=Route::Books.href()>"Books"</a></li>
                                    <li><a href=book_href>{tree.title.clone()}</a></li>
                                    <li><span aria-current="page">{row.title.clone()}</span></li>
                                </ol>
                            </nav>
                            <h1 data-testid="chapter-title">{row.title.clone()}</h1>
                            {book_off().then(|| view! {
                                <p class="muted book-off-line" data-testid="book-off-line">{BOOK_OFF_LINE}</p>
                            })}
                            {own.map(|href| view! {
                                <p class="chapter-own"><a class="jump" href=href>"Read this chapter"</a></p>
                            })}
                            <div class="chapter-sections">{sections}</div>
                        }.into_any()
                    }
                })
            }}
        </div>
    }
}

/// A first-level section of a chapter: an `h2`, then the deeper nodes as one plain list.
fn chapter_section(node: &BookNode, source: &str, hide: Hide<'_>, path: &[usize]) -> AnyView {
    let title = chapter_label(&node.title);
    let id = toc_id(path);
    let heading = match entry_link(node, source, hide) {
        Some(href) => view! { <a href=href>{title.clone()}</a> }.into_any(),
        None => view! { {title.clone()} }.into_any(),
    };
    let children = listed_children(node, hide);
    let list = (!hub_collapses(node, hide) && !children.is_empty()).then(|| {
        let items = children
            .into_iter()
            .filter_map(|index| {
                node.children.get(index).map(|child| {
                    let mut child_path = path.to_vec();
                    child_path.push(index);
                    chapter_list_item(child, source, hide, &child_path)
                })
            })
            .collect_view();
        view! { <ul class="chapter-list">{items}</ul> }
    });
    view! {
        <section class="chapter-section">
            <h2 id=id>{heading}</h2>
            {list}
        </section>
    }
    .into_any()
}

/// One deeper node: a label or entry link, with its children in a nested plain list.
fn chapter_list_item(node: &BookNode, source: &str, hide: Hide<'_>, path: &[usize]) -> AnyView {
    fn walk(node: &BookNode, source: &str, hide: Hide<'_>, path: &mut Vec<usize>) -> AnyView {
        let title = chapter_label(&node.title);
        let label = match entry_link(node, source, hide) {
            Some(href) => view! { <a href=href>{title.clone()}</a> }.into_any(),
            None => view! { <span>{title.clone()}</span> }.into_any(),
        };
        let children = listed_children(node, hide);
        let list = (!hub_collapses(node, hide) && !children.is_empty()).then(|| {
            let items = children
                .into_iter()
                .filter_map(|index| {
                    let child = node.children.get(index)?;
                    path.push(index);
                    let item = walk(child, source, hide, path);
                    path.pop();
                    Some(item)
                })
                .collect_view();
            view! { <ul class="chapter-list">{items}</ul> }
        });
        let id = toc_id(path);
        view! { <li id=id>{label}{list}</li> }.into_any()
    }
    let mut path = path.to_vec();
    walk(node, source, hide, &mut path)
}

/// An entry link for a chapter-page row, with the book context that keeps previous/next on this
/// book after a reload.
fn entry_link(node: &BookNode, source: &str, hide: Hide<'_>) -> Option<String> {
    let entry = node.entry.as_ref().filter(|entry| !hide(entry))?;
    Some(
        Route::Entry {
            id: entry.clone(),
            anchor: node.anchor.clone(),
            book: Some(source.to_string()),
        }
        .href(),
    )
}

/// The review line and the expanded contents list of one book.
#[component]
fn Contents(
    lookup: Shared<BookLookup>,
    library: Library,
    book: usize,
    target: RwSignal<Option<Vec<usize>>>,
) -> impl IntoView {
    let Some(tree) = lookup.book_at(book) else {
        return ().into_any();
    };
    let label = format!("Contents of {}", tree.title);
    let hide = |id: &EntryId| {
        library
            .stub(id)
            .is_some_and(|stub| hidden_from_contents(stub.kind))
    };
    let chapters = chapter_rows(tree, &hide);
    let review = review_line(&tree.review);
    if chapters.is_empty() {
        return view! {
            {review.map(|line| view! { <p class="muted review-line">{line}</p> })}
            <p class="muted">"This book has no rules chapters to show. Find its entries in In this book above."</p>
        }
        .into_any();
    }
    let items = chapters
        .into_iter()
        .map(|chapter| {
            view! { <TocNode lookup=lookup.clone() library=library.clone() book path=chapter.path.clone() target chapter=Some(chapter) /> }
        })
        .collect_view();
    view! {
        {review.map(|line| view! { <p class="muted review-line">{line}</p> })}
        <nav class="toc" aria-label=label>
            <ol class="toc-list toc-top">{items}</ol>
        </nav>
    }
    .into_any()
}

/// One row of the contents. Every branch is expanded: a nested list carries its children, so the
/// reader sees the whole book at once and no row is a disclosure. A hub whose own page lists its
/// children is the one exception: it keeps its title, leader and count and adds no nested list.
#[component]
fn TocNode(
    lookup: Shared<BookLookup>,
    library: Library,
    book: usize,
    path: Vec<usize>,
    target: RwSignal<Option<Vec<usize>>>,
    /// Set on a top-level row: this row opens its chapter page instead of a leaf entry.
    chapter: Option<ChapterRow>,
) -> impl IntoView {
    let Some(tree) = lookup.book_at(book) else {
        return ().into_any();
    };
    let Some(node) = node_at(tree, &path) else {
        return ().into_any();
    };
    let title = chapter_label(&node.title);
    let id = toc_id(&path);
    let chapter_link = chapter.as_ref().map(|chapter| {
        Route::BookChapter {
            source: tree.source.clone(),
            chapter: chapter.id.clone(),
            anchor: None,
        }
        .href()
    });
    let hide = |entry: &EntryId| {
        library
            .stub(entry)
            .is_some_and(|stub| hidden_from_contents(stub.kind))
    };
    // The rows that Contents lists, as indices into the real children: a path keeps its meaning.
    let listed = listed_children(node, &hide);
    let under = listed_under(node, &hide);
    let is_current = {
        let path = path.clone();
        move || target.with(|t| t.as_deref() == Some(path.as_slice()))
    };

    if listed.is_empty() {
        let link = chapter_link.clone().or_else(|| {
            lookup.target_of(tree, &path).map(|(entry, anchor)| {
                Route::Entry {
                    id: entry,
                    anchor,
                    book: Some(tree.source.clone()),
                }
                .href()
            })
        });
        let aria_current = is_current.clone();
        return match link {
            Some(href) => view! {
                <li class="toc-leaf">
                    <a id=id href=href aria-current=move || aria_current().then_some("location")>{title}</a>
                </li>
            }
            .into_any(),
            None => view! { <li class="toc-leaf"><span id=id>{title}</span></li> }.into_any(),
        };
    }

    // A top-level row opens its chapter page. A nested chapter that is itself an entry links to
    // that entry; a bare heading is a plain row. A hub row opens its own entry page instead: that
    // page already carries the list, so the chapter page would only repeat it.
    let collapsed = hub_collapses(node, &hide);
    let entry_link = node
        .entry
        .as_ref()
        .filter(|entry| !hide(entry))
        .map(|entry| {
            Route::Entry {
                id: entry.clone(),
                anchor: node.anchor.clone(),
                book: Some(tree.source.clone()),
            }
            .href()
        });
    let own_link = if collapsed {
        entry_link
    } else {
        chapter_link.clone().or(entry_link)
    };
    let children = (!collapsed).then(|| {
        let lookup = lookup.clone();
        let library = library.clone();
        move || {
            listed
                .iter()
                .map(|&i| {
                    let mut child = path.clone();
                    child.push(i);
                    view! {
                        <TocNode lookup=lookup.clone() library=library.clone() book path=child target chapter=None />
                    }
                })
                .collect_view()
        }
    });
    // The dotted leader is decorative: it is hidden from assistive technology and from the
    // link name, and CSS shows it on top-level rows only.
    let leader = || view! { <span class="toc-leader" aria-hidden="true"></span> };
    let row = match own_link {
        Some(href) => view! {
            <a id=id href=href aria-current=move || is_current().then_some("location")>
                <span class="toc-title">{title}</span>
                {leader()}
            </a>
        }
        .into_any(),
        None => view! {
            <span id=id aria-current=move || is_current().then_some("location")>
                <span class="toc-title">{title}</span>
                {leader()}
            </span>
        }
        .into_any(),
    };
    let class = if collapsed {
        "toc-branch toc-hub"
    } else {
        "toc-branch"
    };
    view! {
        <li class=class>
            <div class="toc-row">
                {row}
                <span class="toc-count" aria-label=fmt::pluralize(under, "entry", "entries")>{fmt::count(under)}</span>
            </div>
            {children.map(|children| view! { <ol class="toc-list">{children}</ol> })}
        </li>
    }
    .into_any()
}

/// The place of an entry in its book: the chapters above it and its neighbours in book order.
/// It loads `books.json` after the entry is on screen and shows nothing when the library has no
/// book order or the entry is in no book.
#[component]
pub fn BookPosition(library: Library, id: EntryId, book: Option<String>) -> impl IntoView {
    let lookup = {
        let library = library.clone();
        local_resource(async move { library.books().await })
    };
    move || {
        let id = id.clone();
        let book = book.clone();
        lookup.with(|state| {
            let Some(Ok(Some(books))) = state else {
                return ().into_any();
            };
            let hide = |entry: &EntryId| {
                library
                    .stub(entry)
                    .is_some_and(|stub| hidden_from_contents(stub.kind))
            };
            // The reader's book wins. Without context the first active book in book order is
            // the candidate; if it does not place this entry, the page shows no reading footer.
            let chosen = book
                .as_deref()
                .and_then(|source| books.books().iter().position(|tree| tree.source == source))
                .or_else(|| {
                    books
                        .books()
                        .iter()
                        .position(|tree| library.book_active(&tree.source))
                })
                .filter(|&index| {
                    books.book_at(index).is_some_and(|tree| {
                        reading_order(tree, &hide)
                            .iter()
                            .any(|stop| stop.entry == id)
                    })
                });
            let Some(index) = chosen else {
                return ().into_any();
            };
            let Some(book) = books.book_at(index) else {
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
            let trail = reading_order(book, &hide)
                .into_iter()
                .find(|stop| stop.entry == id)
                .map(|stop| stop.path)
                .unwrap_or_default();
            for depth in 1..trail.len() {
                let path = trail[..depth].to_vec();
                if let Some(node) = node_at(book, &path) {
                    let href = Route::Book {
                        source: source.clone(),
                        chapter: Some(path),
                    }
                    .href();
                    crumbs.push(view! { <li><a href=href>{chapter_label(&node.title)}</a></li> }.into_any());
                }
            }
            let (before, after) = reading_neighbors(book, &hide, &id);
            let step = |neighbor: Option<&ReadingNeighbor>, next: bool| {
                neighbor.map(|neighbor| {
                    let href = Route::Entry {
                        id: neighbor.entry.clone(),
                        anchor: None,
                        book: Some(source.clone()),
                    }
                    .href();
                    let dir = match (next, neighbor.crossing) {
                        (false, false) => "Previous".to_string(),
                        (false, true) => format!("Previous in {}", neighbor.chapter),
                        (true, false) => format!("Next in {}", book.title),
                        (true, true) => format!("Next in {} · {}", book.title, neighbor.chapter),
                    };
                    let class = if next { "step step-next" } else { "step step-prev" };
                    view! {
                        <a class=class href=href rel=if next { "next" } else { "prev" }>
                            <span class="step-dir">
                                {(!next).then(|| icons::arrow_left("step-arrow"))}
                                {dir}
                                {next.then(|| icons::arrow_right("step-arrow"))}
                            </span>
                            <span class="step-title">{crate::views::util::section_label(&neighbor.title)}</span>
                        </a>
                    }
                })
            };
            view! {
                <nav class="book-bar" aria-label="Position in the book" data-testid="book-bar">
                    <p class="book-where">"In the book · draft order"</p>
                    <ol class="book-trail">{crumbs}</ol>
                    <div class="book-steps">
                        {step(before.as_ref(), false)}
                        {step(after.as_ref(), true)}
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
    use cast_iron_schema::books::{contents_roots, linked_kind, shows_in_contents};

    fn node(title: &str, entry: Option<&str>, children: Vec<BookNode>) -> BookNode {
        BookNode {
            title: title.to_string(),
            entry: entry.map(|s| EntryId::parse(s).unwrap()),
            anchor: None,
            review: None,
            lists_children: false,
            children,
        }
    }

    /// Kinds by the middle part of the id, as the catalog would say them.
    fn kind_of(id: &EntryId) -> Option<EntryKind> {
        match id.as_str().split('/').nth(1)? {
            "spell" => Some(EntryKind::Spell),
            "feat" => Some(EntryKind::Feat),
            "magic-ring" | "item" => Some(EntryKind::Item),
            "class" => Some(EntryKind::Class),
            "prestige-class" => Some(EntryKind::PrestigeClass),
            "archetype" => Some(EntryKind::Archetype),
            "rule" => Some(EntryKind::Rule),
            "race" => Some(EntryKind::Race),
            _ => None,
        }
    }

    /// The rule with the exclusion on, as #158 runs it.
    fn hide(id: &EntryId) -> bool {
        kind_of(id).is_some_and(linked_kind)
    }

    #[test]
    fn the_exclusion_is_on_with_the_kind_links() {
        for kind in EntryKind::ALL {
            assert_eq!(hidden_from_contents(kind), linked_kind(kind), "{kind:?}");
        }
    }

    #[test]
    fn in_this_book_lists_home_order_without_rules_or_empty_kinds() {
        let counts: Vec<(EntryKind, usize)> = EntryKind::ALL
            .into_iter()
            .map(|kind| (kind, if kind == EntryKind::Rule { 7 } else { 1 }))
            .collect();
        let listed = kinds_in_book(&counts);
        let names: Vec<&str> = listed.iter().map(|(kind, _)| kind.label()).collect();
        assert_eq!(
            names,
            [
                "Spells",
                "Feats",
                "Classes",
                "Prestige classes",
                "Class options",
                "Races",
                "Traits",
                "Items",
                "Creatures",
                "Archetypes",
                "Racial traits",
                "Skills",
                "Afflictions",
                "Traps"
            ]
        );
        assert!(!listed.iter().any(|(kind, _)| *kind == EntryKind::Rule));
        assert!(
            kinds_in_book(&[(EntryKind::Spell, 0), (EntryKind::Trap, 2)])
                == vec![(EntryKind::Trap, 2)]
        );
    }

    #[test]
    fn contents_leaves_out_every_kind_with_a_list_page() {
        for kind in EntryKind::ALL {
            assert_eq!(linked_kind(kind), kind != EntryKind::Rule, "{kind:?}");
        }
    }

    #[test]
    fn a_chapter_is_listed_unless_everything_under_it_is_a_hidden_kind() {
        let spells = node(
            "Spells",
            None,
            vec![
                node("Acid Arrow", Some("ref/spell/acid-arrow"), vec![]),
                node("Alarm", Some("ref/spell/alarm"), vec![]),
            ],
        );
        assert!(!shows_in_contents(&spells, &hide));
        let mixed = node(
            "Combat",
            None,
            vec![
                node("Power Attack", Some("ref/feat/power-attack"), vec![]),
                node("Grapple", Some("ref/rule/grapple"), vec![]),
            ],
        );
        assert!(shows_in_contents(&mixed, &hide));
        assert_eq!(listed_children(&mixed, &hide), vec![1]);
        assert_eq!(listed_under(&mixed, &hide), 1);
        // A heading with no entry under it, and an entry the catalog does not know, stay.
        assert!(shows_in_contents(&node("Credits", None, vec![]), &hide));
        assert!(shows_in_contents(
            &node("Mystery", Some("ref/unknown/thing"), vec![]),
            &hide
        ));
        // A chapter with its own rule entry is listed even when its children are all hidden.
        let own = node(
            "Magic",
            Some("ref/rule/magic"),
            vec![node("Alarm", Some("ref/spell/alarm"), vec![])],
        );
        assert!(shows_in_contents(&own, &hide));
        assert!(listed_children(&own, &hide).is_empty());
        assert_eq!(listed_under(&own, &hide), 1);
    }

    #[test]
    fn the_catch_all_is_renamed_and_a_lone_one_shows_its_rows_at_the_top() {
        assert_eq!(
            chapter_label("New entries (page order)"),
            "More from this book"
        );
        assert_eq!(chapter_label("Page order"), "Page order");
        assert_eq!(chapter_label("Actions In Combat"), "Actions in Combat");

        let catch_all = node(
            "New entries (page order)",
            None,
            vec![
                node("Goblin", Some("ref/race/goblin"), vec![]),
                node("Fireball", Some("ref/spell/fireball"), vec![]),
                node("Rules", Some("ref/rule/rules"), vec![]),
            ],
        );
        let spells = node(
            "Spells",
            None,
            vec![node("Alarm", Some("ref/spell/alarm"), vec![])],
        );
        // Races and spells have list pages, so only the catch-all chapter remains. Alone, its
        // one remaining row is the top level and keeps its path in the whole tree.
        let books = vec![
            node("Races", Some("ref/race/human"), vec![]),
            spells.clone(),
            catch_all.clone(),
        ];
        assert_eq!(contents_roots(&books, &hide), vec![vec![2, 2]]);
        let alone = vec![spells, catch_all];
        assert_eq!(contents_roots(&alone, &hide), vec![vec![1, 2]]);
        // Nothing left to list.
        assert!(contents_roots(&alone[..1], &hide).is_empty());
    }

    #[test]
    fn the_review_line_is_shown_for_a_book_with_open_questions() {
        assert_eq!(
            review_line(&["The Classes tree is incomplete.".to_string()]),
            Some("Chapter order not yet checked against the printed book")
        );
        assert_eq!(review_line(&[]), None);
    }

    #[test]
    fn book_entry_count_uses_every_placement_once() {
        let node = |title: &str, entry: Option<&str>, children: Vec<BookNode>| BookNode {
            title: title.to_string(),
            entry: entry.map(|s| EntryId::parse(s).unwrap()),
            anchor: None,
            review: None,
            lists_children: false,
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

    fn chapter_tree() -> BookTree {
        BookTree {
            source: "core".into(),
            title: "Core Rulebook".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![
                node(
                    "Combat",
                    Some("core/rule/combat"),
                    vec![
                        node("Actions", Some("core/rule/actions"), vec![]),
                        node("Power Attack", Some("core/feat/power-attack"), vec![]),
                        node(
                            "Cover",
                            None,
                            vec![node(
                                "Partial Cover",
                                Some("core/rule/partial-cover"),
                                vec![],
                            )],
                        ),
                    ],
                ),
                node(
                    "Magic",
                    Some("core/rule/magic"),
                    vec![node(
                        "Spells",
                        None,
                        vec![node("Fireball", Some("core/spell/fireball"), vec![])],
                    )],
                ),
                node("Credits", None, vec![]),
            ],
        }
    }

    #[test]
    fn chapter_rows_follow_contents_and_carry_permanent_ids() {
        let tree = chapter_tree();
        let rows = chapter_rows(&tree, &hide);
        assert_eq!(
            rows.iter()
                .map(|row| (row.id.as_str(), row.title.as_str()))
                .collect::<Vec<_>>(),
            [
                ("combat", "Combat"),
                ("magic", "Magic"),
                ("credits", "Credits")
            ]
        );
        // The row's path is the real tree path, so a chapter that is an entry still opens.
        assert_eq!(rows[1].path, vec![1]);
    }

    #[test]
    fn a_hub_collapses_only_with_a_visible_entry() {
        let mut hub = node(
            "Combat",
            Some("core/rule/combat"),
            vec![node("Actions", Some("core/rule/actions"), vec![])],
        );
        assert!(!hub_collapses(&hub, &hide));
        hub.lists_children = true;
        assert!(hub_collapses(&hub, &hide));
        // A hidden hub entry has no row link, so the branch stays.
        let only_combat = |id: &EntryId| id.as_str() == "core/rule/combat";
        assert!(!hub_collapses(&hub, &only_combat));
        // A heading without an entry never collapses.
        hub.entry = None;
        assert!(!hub_collapses(&hub, &hide));
    }

    #[test]
    fn a_collapsed_hub_still_reads_through_its_children() {
        let mut tree = chapter_tree();
        tree.chapters[0].lists_children = true;
        let stops = reading_order(&tree, &hide);
        let order: Vec<&str> = stops.iter().map(|stop| stop.entry.as_str()).collect();
        assert_eq!(
            order,
            [
                "core/rule/combat",
                "core/rule/actions",
                "core/rule/partial-cover",
                "core/rule/magic"
            ]
        );
        let id = |s: &str| EntryId::parse(s).unwrap();
        let (_, after) = reading_neighbors(&tree, &hide, &id("core/rule/combat"));
        assert_eq!(after.unwrap().entry, id("core/rule/actions"));
    }

    #[test]
    fn reading_order_skips_the_kinds_contents_leaves_out() {
        let tree = chapter_tree();
        let order: Vec<(String, Vec<usize>)> = reading_order(&tree, &hide)
            .into_iter()
            .map(|stop| (stop.entry.as_str().to_string(), stop.path))
            .collect();
        assert_eq!(
            order,
            [
                ("core/rule/combat".to_string(), vec![0]),
                ("core/rule/actions".to_string(), vec![0, 0]),
                ("core/rule/partial-cover".to_string(), vec![0, 2, 0]),
                ("core/rule/magic".to_string(), vec![1]),
            ]
        );
        assert!(!order.iter().any(|(id, _)| id == "core/feat/power-attack"));
        assert!(!order.iter().any(|(id, _)| id == "core/spell/fireball"));
    }

    #[test]
    fn reading_neighbours_cross_chapters_stop_at_book_ends_and_skip_hidden_kinds() {
        let tree = chapter_tree();
        let id = |s: &str| EntryId::parse(s).unwrap();
        let (before, after) = reading_neighbors(&tree, &hide, &id("core/rule/combat"));
        assert!(before.is_none());
        let after = after.unwrap();
        assert_eq!(after.entry, id("core/rule/actions"));
        assert!(!after.crossing);

        // The feat is skipped, so the nested rule follows the heading directly.
        let (before, after) = reading_neighbors(&tree, &hide, &id("core/rule/actions"));
        assert_eq!(before.unwrap().entry, id("core/rule/combat"));
        assert_eq!(after.unwrap().entry, id("core/rule/partial-cover"));

        // The chapter end continues into the next chapter and names it.
        let (_, after) = reading_neighbors(&tree, &hide, &id("core/rule/partial-cover"));
        let after = after.unwrap();
        assert_eq!(after.entry, id("core/rule/magic"));
        assert!(after.crossing);
        assert_eq!(after.chapter, "Magic");
        let (before, _) = reading_neighbors(&tree, &hide, &id("core/rule/magic"));
        let before = before.unwrap();
        assert_eq!(before.entry, id("core/rule/partial-cover"));
        assert!(before.crossing);
        assert_eq!(before.chapter, "Combat");

        // The last visible entry has no next; a hidden kind is not on the walk at all.
        let (_, after) = reading_neighbors(&tree, &hide, &id("core/rule/magic"));
        assert!(after.is_none());
        assert_eq!(
            reading_neighbors(&tree, &hide, &id("core/feat/power-attack")),
            (None, None)
        );
        assert_eq!(
            reading_neighbors(&tree, &hide, &id("core/spell/fireball")),
            (None, None)
        );
    }

    #[test]
    fn only_a_chapter_change_inside_one_book_keeps_the_page_mounted() {
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
