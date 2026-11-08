//! The book view: one publication's chapters in the order of the book, as a contents tree, and
//! the previous/next control that an entry shows for its place in a book.
//!
//! Nothing in this module sorts. Every list below follows the draft order in `books.json`.
//! Contents leaves out the kinds that the `In this book` links reach (see
//! [`LEAVE_OUT_LINKED_KINDS`]); the page keeps every chapter's real path, so a chapter link,
//! `c=3.2` in the address and the trail above an entry all mean what they meant.

use crate::fmt;
use crate::graph::{BookLookup, node_at};
use crate::library::{Library, use_library};
use crate::route::{Route, current_hash, replace, toc_id, use_route};
use crate::views::icons;
use crate::views::util::{Shared, local_resource, set_title};
use cast_iron_schema::{BookNode, EntryId, EntryKind};
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

/// Whether Contents leaves out the kinds that the `In this book` links reach.
///
/// The owner decided on 2026-11-07 that Contents does not list spells, feats, items and class
/// options, because a book holds thousands of them and the kind links reach them as list pages
/// filtered to the book. The links ship in #158, so the exclusion is on.
pub const LEAVE_OUT_LINKED_KINDS: bool = true;

/// The kinds that `In this book` reaches: spells, feats, items and the class options
/// (classes, prestige classes and archetypes).
pub fn linked_kind(kind: EntryKind) -> bool {
    matches!(
        kind,
        EntryKind::Spell
            | EntryKind::Feat
            | EntryKind::Item
            | EntryKind::Class
            | EntryKind::PrestigeClass
            | EntryKind::Archetype
            | EntryKind::ClassOption
    )
}

/// True when Contents leaves a kind out. Always false while [`LEAVE_OUT_LINKED_KINDS`] is off.
pub fn hidden_from_contents(kind: EntryKind) -> bool {
    LEAVE_OUT_LINKED_KINDS && linked_kind(kind)
}

/// Says whether Contents leaves an entry out. An entry the catalog does not know is kept.
pub type Hide<'a> = &'a dyn Fn(&EntryId) -> bool;

/// True when a node still has something to list. A node whose entries are all left out has
/// nothing; a node with no entry anywhere under it is kept as the compiler wrote it.
pub fn shows_in_contents(node: &BookNode, hide: Hide<'_>) -> bool {
    fn all_hidden(node: &BookNode, hide: Hide<'_>, seen: &mut bool) -> bool {
        if let Some(entry) = &node.entry {
            *seen = true;
            if !hide(entry) {
                return false;
            }
        }
        node.children
            .iter()
            .all(|child| all_hidden(child, hide, seen))
    }
    let mut seen = false;
    !(all_hidden(node, hide, &mut seen) && seen)
}

/// The indices of the children of `node` that Contents lists, in book order.
pub fn listed_children(node: &BookNode, hide: Hide<'_>) -> Vec<usize> {
    (0..node.children.len())
        .filter(|&i| shows_in_contents(&node.children[i], hide))
        .collect()
}

/// The entries under a node, itself included, that Contents lists: the number a chapter shows.
pub fn listed_under(node: &BookNode, hide: Hide<'_>) -> usize {
    let own = node.entry.as_ref().is_some_and(|entry| !hide(entry));
    usize::from(own)
        + node
            .children
            .iter()
            .map(|child| listed_under(child, hide))
            .sum::<usize>()
}

/// The title that the compiler gives the chapter of entries that no curated chapter places.
const CATCH_ALL_TITLE: &str = "New entries (page order)";

/// What a reader sees for it. The title says how the list was made, which a reader does not
/// need; the corpus carries the old title, so the page renames it (the pin bump that renames it
/// at the source is the corpus owner's).
const CATCH_ALL_LABEL: &str = "More from this book";

/// The label of a chapter or section row.
pub fn chapter_label(title: &str) -> String {
    if title == CATCH_ALL_TITLE {
        return CATCH_ALL_LABEL.to_string();
    }
    crate::views::util::section_label(title)
}

/// The paths of the rows at the top of Contents. A book whose one remaining chapter is the
/// catch-all shows that chapter's rows at the top, not a lone `More from this book` heading
/// over them. Paths are real: they index the whole tree, not the listed rows.
pub fn contents_roots(chapters: &[BookNode], hide: Hide<'_>) -> Vec<Vec<usize>> {
    let listed: Vec<usize> = (0..chapters.len())
        .filter(|&i| shows_in_contents(&chapters[i], hide))
        .collect();
    if let [only] = listed[..] {
        let node = &chapters[only];
        if node.title == CATCH_ALL_TITLE && node.entry.is_none() {
            return listed_children(node, hide)
                .into_iter()
                .map(|child| vec![only, child])
                .collect();
        }
    }
    listed.into_iter().map(|i| vec![i]).collect()
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
    view! {
        <div class="wide book">
            <h1>{title}</h1>
            {meta.map(|line| view! { <p class="muted book-meta-line" data-testid="source-count">{line}</p> })}
            {move || book_off().then(|| view! {
                <p class="muted book-off-line" data-testid="book-off-line">{BOOK_OFF_LINE}</p>
            })}
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
                                <Contents lookup=Shared::from_rc(books.clone()) library book=index source target />
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

/// The review line and the contents tree of one book.
#[component]
fn Contents(
    lookup: Shared<BookLookup>,
    library: Library,
    book: usize,
    source: String,
    target: RwSignal<Option<Vec<usize>>>,
) -> impl IntoView {
    let Some(tree) = lookup.book_at(book) else {
        return ().into_any();
    };
    let label = format!("Contents of {}", tree.title);
    let roots = contents_roots(&tree.chapters, &|id| {
        library
            .stub(id)
            .is_some_and(|stub| hidden_from_contents(stub.kind))
    });
    let review = review_line(&tree.review);
    if roots.is_empty() {
        return view! {
            {review.map(|line| view! { <p class="muted review-line">{line}</p> })}
            <p class="muted">"This book's entries are spells, feats, items and class options. Find them in In this book above."</p>
        }
        .into_any();
    }
    let items = roots
        .into_iter()
        .map(|path| {
            view! { <TocNode lookup=lookup.clone() library=library.clone() book source=source.clone() path target /> }
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

/// One row of the contents. A node with children is a disclosure whose children are built when
/// it first opens, so a book of two thousand entries costs a few dozen rows at first.
#[component]
fn TocNode(
    lookup: Shared<BookLookup>,
    library: Library,
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
    let title = chapter_label(&node.title);
    let id = toc_id(&path);
    let hide = |entry: &EntryId| {
        library
            .stub(entry)
            .is_some_and(|stub| hidden_from_contents(stub.kind))
    };
    // The rows that Contents lists, as indices into the real children: a path keeps its meaning.
    let listed = listed_children(node, &hide);
    let child_count = listed.len();
    let under = listed_under(node, &hide);
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
                </li>
            }
            .into_any(),
            None => view! { <li class="toc-leaf"><span id=id>{title}</span></li> }.into_any(),
        };
    }

    let own_link = node
        .entry
        .as_ref()
        .filter(|entry| !hide(entry))
        .map(|entry| {
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
        let library = library.clone();
        move || {
            opened.get().then(|| {
                let items = listed
                    .iter()
                    .map(|&i| {
                        let mut child = path.clone();
                        child.push(i);
                        view! {
                            <TocNode lookup=lookup.clone() library=library.clone() book source=source.clone() path=child target />
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
                view! { <ul class="toc-list">{own}{items}</ul> }
            })
        }
    };
    view! {
        <li class="toc-branch">
            <details class="toc-details" prop:open=move || opened.get() on:toggle=on_toggle>
                <summary id=id aria-current=move || is_current().then_some("location")>
                    {icons::chevron_right("disclosure-chevron")}
                    <span class="toc-title">{title}</span>
                    <span class="toc-count" aria-label=fmt::pluralize(under, "entry", "entries")>{fmt::count(under)}</span>
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
                            <span class="step-dir">
                                {(!next).then(|| icons::arrow_left("step-arrow"))}
                                {dir}
                                {next.then(|| icons::arrow_right("step-arrow"))}
                            </span>
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

    fn node(title: &str, entry: Option<&str>, children: Vec<BookNode>) -> BookNode {
        BookNode {
            title: title.to_string(),
            entry: entry.map(|s| EntryId::parse(s).unwrap()),
            anchor: None,
            review: None,
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
    fn contents_leaves_out_spells_feats_items_and_class_options_only() {
        for (kind, hidden) in [
            (EntryKind::Spell, true),
            (EntryKind::Feat, true),
            (EntryKind::Item, true),
            (EntryKind::Class, true),
            (EntryKind::PrestigeClass, true),
            (EntryKind::Archetype, true),
            (EntryKind::ClassOption, true),
            (EntryKind::Rule, false),
            (EntryKind::Race, false),
            (EntryKind::Skill, false),
            (EntryKind::Creature, false),
            (EntryKind::Trait, false),
            (EntryKind::RacialTrait, false),
            (EntryKind::Affliction, false),
            (EntryKind::Trap, false),
        ] {
            assert_eq!(linked_kind(kind), hidden, "{kind:?}");
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
        // Next to a listed chapter it stays a chapter, with the real index.
        let books = vec![
            node("Races", Some("ref/race/human"), vec![]),
            spells.clone(),
            catch_all.clone(),
        ];
        assert_eq!(contents_roots(&books, &hide), vec![vec![0], vec![2]]);
        // Alone, its rows are the top level and keep their paths in the whole tree.
        let alone = vec![spells, catch_all];
        assert_eq!(contents_roots(&alone, &hide), vec![vec![1, 0], vec![1, 2]]);
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
