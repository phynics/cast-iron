//! Category browsing and the separate publication directory, with category lenses,
//! A to Z lists, the rules hierarchy, and the old publication page for libraries without books.

use crate::fmt;
use crate::graph::{BookLookup, entries_under, node_at};
use crate::library::{Library, LibraryError, use_library};
use crate::route::{BookSort, Route, replace, use_route};
use crate::views::book_tier::Tier;
use crate::views::categories::{self, Lens, applied_facets};
use crate::views::filter_menu::{
    FilterMenuButton, FilterMenuPanel, MenuOption, MenuSpec, MenuState, current_option,
    take_focus_request, trigger_text,
};
use crate::views::scope::Scope;
use crate::views::search::{facet_label, sort_facet_pairs};
use crate::views::source_variants::{SourceVariant, SourceVariants, group_variants};
use crate::views::util::{
    ACTIVE_BOOKS_SCOPE, BOOK_TOTALS_SCOPE, Shared, active_count_line, group_letter, kind_name,
    local_resource, section_label, set_title, sort_key,
};
use cast_iron_schema::{EntryId, EntryKind, EntryStub};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasm_bindgen::JsCast;

// ---- pure helpers -----------------------------------------------------------------------

/// Indices into `titles` grouped by first letter, in A to Z order with `#` last. Within a
/// group, entries keep their order in `titles`, so pass them sorted.
pub fn group_by_letter(titles: &[&str]) -> Vec<(char, Vec<usize>)> {
    let mut groups: Vec<(char, Vec<usize>)> = Vec::new();
    for (i, title) in titles.iter().enumerate() {
        let letter = group_letter(title);
        match groups.iter_mut().find(|(l, _)| *l == letter) {
            Some((_, list)) => list.push(i),
            None => groups.push((letter, vec![i])),
        }
    }
    groups.sort_by_key(|(l, _)| (*l == '#', *l));
    groups
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub index: usize,
    pub children: Vec<TreeNode>,
}

/// Nests entries under their `parent`. Entries whose parent is not in the list become roots.
/// Roots and siblings are ordered by title. `items` is `(id, parent, title)`.
pub fn build_tree(items: &[(EntryId, Option<EntryId>, String)]) -> Vec<TreeNode> {
    fn grow(
        items: &[(EntryId, Option<EntryId>, String)],
        order: &[usize],
        parent: usize,
        depth: usize,
    ) -> Vec<TreeNode> {
        if depth > 6 {
            return Vec::new();
        }
        order
            .iter()
            .copied()
            .filter(|&i| i != parent && items[i].1.as_ref() == Some(&items[parent].0))
            .map(|index| TreeNode {
                index,
                children: grow(items, order, index, depth + 1),
            })
            .collect()
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (sort_key(&items[i].2), items[i].0.clone()));
    order
        .iter()
        .copied()
        .filter(|&i| {
            items[i]
                .1
                .as_ref()
                .is_none_or(|p| !items.iter().any(|(id, _, _)| id == p))
        })
        .map(|index| TreeNode {
            index,
            children: grow(items, &order, index, 0),
        })
        .collect()
}

/// Entries of a kind as `(stub)` sorted by title.
fn sorted_of_kind(library: &Library, kind: EntryKind) -> Vec<EntryStub> {
    let mut list: Vec<EntryStub> = library.discovery().entries_of_kind(kind).cloned().collect();
    list.sort_by_key(|s| (sort_key(&s.title), s.id.clone()));
    list
}

// ---- views ------------------------------------------------------------------------------

fn entry_row(stub: &EntryStub, source: Option<String>) -> impl IntoView + use<> {
    view! {
        <li>
            <a href=Route::entry(stub.id.clone()).href()>{stub.title.clone()}</a>
            {source.map(|s| view! { <span class="muted row-source">{s}</span> })}
        </li>
    }
}

/// A feat leads to more than this many feats down its chain before its row carries the chain
/// marker. Of the 3,583 feats in the pinned library, 75 pass it (2.1 %).
pub const CHAIN_MARK_MIN: usize = 5;

/// How many feats each marked feat leads to, once the relations have loaded. The outer `None`
/// is "loading"; the inner `None` is a library with no relations. Rows read it, so the list
/// shows at once and the markers appear when the numbers are ready.
pub type ChainMarks = ReadSignal<Option<Option<Shared<HashMap<EntryId, usize>>>>, LocalStorage>;

/// The accessible suffix of a marked row: `, leads to 93 feats`.
pub fn chain_mark_suffix(leads_to: usize) -> String {
    format!(
        ", leads to {} {}",
        fmt::count(leads_to),
        if leads_to == 1 { "feat" } else { "feats" }
    )
}

/// The marker: a root joined to two feats. The text is for screen readers, the icon for the rest.
fn chain_mark(leads_to: usize) -> impl IntoView {
    let suffix = chain_mark_suffix(leads_to);
    view! {
        <span class="sr-only">{suffix.clone()}</span>
        <svg class="chain-mark" viewBox="0 0 24 24" width="16" height="16" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <title>{suffix.trim_start_matches(", ").to_string()}</title>
            <circle cx="5.5" cy="12" r="2.2"/><circle cx="18.5" cy="6" r="2.2"/>
            <circle cx="18.5" cy="18" r="2.2"/><path d="M7.7 12H11l3.4-4.5M11 12l3.4 4.5"/>
        </svg>
    }
}

/// A feat row. The marker follows the title inside the link, so the row stays one target.
fn feat_row(stub: &EntryStub, source: Option<String>, marks: ChainMarks) -> impl IntoView + use<> {
    let id = stub.id.clone();
    let mark = move || {
        marks.with(|loaded| {
            loaded
                .as_ref()
                .and_then(Option::as_ref)
                .and_then(|marks| marks.get(&id).copied())
                .map(chain_mark)
        })
    };
    view! {
        <li>
            <a href=Route::entry(stub.id.clone()).href()>{stub.title.clone()}{mark}</a>
            {source.map(|s| view! { <span class="muted row-source">{s}</span> })}
        </li>
    }
}

fn category_card(kind: EntryKind, n: usize) -> impl IntoView + use<> {
    view! {
        <li>
            <a class="cat" href=Route::browse_kind(kind).href()>
                <span class="cat-name">{kind.label()}</span>
                <span class="cat-count">{fmt::count(n)}</span>
                <span class="cat-blurb">{categories::blurb(kind)}</span>
            </a>
        </li>
    }
}

/// The category directory shared by Home and any other screen that lists every category.
/// `note` adds the line about what the counts mean, which only Home shows.
#[component]
pub fn CategoryDirectory(#[prop(optional)] note: bool) -> impl IntoView {
    let library = use_library();
    let (primary, more) = categories::arrange(&library.discovery().kind_counts());
    let primary = primary
        .into_iter()
        .map(|(kind, n)| category_card(kind, n))
        .collect_view();
    let more_section = (!more.is_empty()).then(|| {
        let rows = more
            .into_iter()
            .map(|(kind, n)| category_card(kind, n))
            .collect_view();
        view! {
            <section aria-labelledby="by-kind-more">
                <h2 id="by-kind-more" class="sub-head">"More categories"</h2>
                <ul class="cats cats-quiet" aria-describedby=note.then_some("by-kind-scope")>{rows}</ul>
            </section>
        }
    });
    let shortcuts = library.has_spell_lists().then(|| {
        view! {
            <section aria-labelledby="shortcuts">
                <h2 id="shortcuts" class="sub-head">"Shortcuts"</h2>
                <ul class="jump-list">
                    <li><a class="jump" href=Route::Spells.href()>"Spell lists by class and level"</a></li>
                    <li><a class="jump" href=Route::FeatChains.href()>"Feat chains"</a></li>
                </ul>
            </section>
        }
    });
    view! {
        <section aria-labelledby="by-kind">
            <h2 id="by-kind">"Categories"</h2>
            {note.then(|| view! { <p id="by-kind-scope" class="muted cats-note">{ACTIVE_BOOKS_SCOPE}"."</p> })}
            <ul class="cats" aria-describedby=note.then_some("by-kind-scope")>{primary}</ul>
        </section>
        {more_section}
        {shortcuts}
    }
}

/// One publication row. Grouping is a pure function over these so the order can be tested
/// without a browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookListEntry {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
    pub publication_date: Option<String>,
    pub count: usize,
}

/// One displayed group of publications. `tier` is `None` for library order, which is one
/// ungrouped list. Every other order groups by priority tier and omits empty tiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookGroup {
    pub tier: Option<Tier>,
    pub books: Vec<usize>,
}

fn year_of(date: Option<&str>) -> Option<&str> {
    let date = date?;
    (date.len() >= 4 && date.as_bytes()[..4].iter().all(u8::is_ascii_digit)).then(|| &date[..4])
}

fn title_order(books: &[BookListEntry], a: usize, b: usize) -> std::cmp::Ordering {
    sort_key(&books[a].title)
        .cmp(&sort_key(&books[b].title))
        .then_with(|| books[a].id.cmp(&books[b].id))
}

/// Groups publications for the selected order. Library order is one ungrouped list in catalog
/// order. Every other order groups by [`Tier`] in priority order and omits empty tiers. Inside
/// a tier the default sorts by title, `Category` by reviewed product line and then title, and
/// `Date` by publication date newest first with unknown dates last. Ties stay alphabetical.
pub fn book_groups(books: &[BookListEntry], sort: Option<BookSort>) -> Vec<BookGroup> {
    if sort == Some(BookSort::Library) {
        return vec![BookGroup {
            tier: None,
            books: (0..books.len()).collect(),
        }];
    }
    Tier::ALL
        .into_iter()
        .filter_map(|tier| {
            let mut indices: Vec<usize> = (0..books.len())
                .filter(|&index| Tier::from_category(books[index].category.as_deref()) == tier)
                .collect();
            if indices.is_empty() {
                return None;
            }
            indices.sort_by(|&a, &b| match sort {
                Some(BookSort::Category) => Tier::category_rank(books[a].category.as_deref())
                    .cmp(&Tier::category_rank(books[b].category.as_deref()))
                    .then_with(|| title_order(books, a, b)),
                Some(BookSort::Date) => {
                    let left = year_of(books[a].publication_date.as_deref())
                        .and(books[a].publication_date.as_deref());
                    let right = year_of(books[b].publication_date.as_deref())
                        .and(books[b].publication_date.as_deref());
                    // Newest first; a missing or malformed date sorts last.
                    right
                        .is_some()
                        .cmp(&left.is_some())
                        .then_with(|| right.cmp(&left))
                        .then_with(|| title_order(books, a, b))
                }
                _ => title_order(books, a, b),
            });
            Some(BookGroup {
                tier: Some(tier),
                books: indices,
            })
        })
        .collect()
}

fn book_meta(row: &BookListEntry) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(category) = &row.category {
        parts.push(category.clone());
    }
    if let Some(date) = &row.publication_date {
        parts.push(date.clone());
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

type BookLookupSignal =
    ReadSignal<Option<Result<Option<Rc<BookLookup>>, LibraryError>>, LocalStorage>;

fn book_row(
    row: BookListEntry,
    library: Library,
    lookup: BookLookupSignal,
    saving: RwSignal<bool>,
    save_error: RwSignal<Option<String>>,
    status: RwSignal<Option<String>>,
) -> AnyView {
    let meta = book_meta(&row);
    let id = row.id;
    let title = row.title;
    let count = row.count;
    let count_label = format!("{count} {}", if count == 1 { "entry" } else { "entries" });
    let revision = library.selection_revision();
    let state_library = library.clone();
    let save_library = library.clone();
    let state_id = id.clone();
    let save_id = id.clone();
    let label = format!("Active {title}");
    let is_active = move || {
        revision.get();
        state_library.book_active(&state_id)
    };
    let badge_active = is_active.clone();
    let id_for_tree = id.clone();
    let title_for_tree = title.clone();
    let expanded = RwSignal::new(false);
    let on_toggle = move |event: leptos::ev::Event| {
        expanded.set(event_target::<web_sys::HtmlDetailsElement>(&event).open());
    };
    view! {
        <li class="book-directory-item" data-book-id=id>
            <details class="book-directory-row" on:toggle=on_toggle>
                <summary class="book-card">
                    <span class="cat-name">{title.clone()}</span>
                    <span class="cat-count">{count}</span>
                    <span class="cat-blurb">
                        {count_label}
                        {meta.map(|line| view! { <span class="book-meta">" · "{line}</span> })}
                        {move || (!badge_active()).then_some(" · Disabled")}
                    </span>
                </summary>
                <div class="book-directory-contents">
                    {move || expanded.get().then(|| lookup.with(|state| match state {
                        Some(Ok(Some(books))) => match books.books().iter().position(|book| book.source == id_for_tree) {
                            Some(index) => view! {
                                <BookTree lookup=Shared::from_rc(books.clone()) book=index title=title_for_tree.clone() />
                            }.into_any(),
                            None => view! { <p class="muted">"No ordered contents are available for this book."</p> }.into_any(),
                        },
                        Some(Err(error)) => view! { <p class="error" role="alert">{error.to_string()}</p> }.into_any(),
                        _ => view! { <p class="muted" role="status">"Loading chapter list…"</p> }.into_any(),
                    }))}
                </div>
            </details>
            <label class="book-activation">
                <input type="checkbox" aria-label=label prop:checked=is_active disabled=move || saving.get()
                    on:change=move |event| {
                        let input = event_target::<web_sys::HtmlInputElement>(&event);
                        if saving.get_untracked() {
                            // Defensive guard: the checkbox is disabled during a save, so this
                            // runs only for a programmatic event. Undo the change and say so.
                            input.set_checked(save_library.book_active(&save_id));
                            status.set(None);
                            save_error.set(Some(SAVE_BUSY.to_string()));
                            return;
                        }
                        let active = input.checked();
                        // Discovery remains at the saved value while this control is busy.
                        saving.set(true);
                        save_error.set(None);
                        // The bulk message no longer describes the selection.
                        status.set(None);
                        let library = save_library.clone();
                        let id = save_id.clone();
                        spawn_local(async move {
                            if let Err(error) = library.set_book_active(&id, active).await {
                                input.set_checked(library.book_active(&id));
                                save_error.set(Some(format!("Could not save the book selection: {error}")));
                            }
                            saving.set(false);
                        });
                    } />
                <span>"Active"</span>
            </label>
        </li>
    }
    .into_any()
}

/// The expanded book, rooted at the same `Read {book}` label as the full contents page.
/// Chapters and sections are native disclosures; children are built only when a branch opens,
/// so a two-thousand-entry book costs a few rows until the reader asks for more.
#[component]
fn BookTree(lookup: Shared<BookLookup>, book: usize, title: String) -> impl IntoView {
    let Some(tree) = lookup.book_at(book) else {
        return ().into_any();
    };
    let href = Route::Book {
        source: tree.source.clone(),
        chapter: None,
    }
    .href();
    let nav_label = format!("Contents of {title}");
    let read_label = format!("Read {title}");
    let chapters = (0..tree.chapters.len())
        .map(|index| {
            view! {
                <DirectoryNode lookup=lookup.clone() book path=vec![index] />
            }
        })
        .collect_view();
    view! {
        <nav class="toc book-tree" aria-label=nav_label data-testid="book-tree">
            <a class="book-tree-root" href=href>{read_label}</a>
            <ul class="toc-list toc-top">{chapters}</ul>
        </nav>
    }
    .into_any()
}

#[component]
fn DirectoryNode(lookup: Shared<BookLookup>, book: usize, path: Vec<usize>) -> impl IntoView {
    let Some(tree) = lookup.book_at(book) else {
        return ().into_any();
    };
    let Some(node) = node_at(tree, &path) else {
        return ().into_any();
    };
    let title = section_label(&node.title);
    let under = entries_under(node);
    let child_count = node.children.len();
    let own_link = node.entry.as_ref().map(|entry| {
        Route::Entry {
            id: entry.clone(),
            anchor: node.anchor.clone(),
        }
        .href()
    });

    if child_count == 0 {
        return match own_link {
            Some(href) => view! { <li class="toc-leaf"><a href=href>{title}</a></li> }.into_any(),
            None => view! { <li class="toc-leaf"><span>{title}</span></li> }.into_any(),
        };
    }

    let opened = RwSignal::new(false);
    let on_toggle = move |event: leptos::ev::Event| {
        opened.set(event_target::<web_sys::HtmlDetailsElement>(&event).open());
    };
    let child_title = title.clone();
    let children = {
        let lookup = lookup.clone();
        let path = path.clone();
        move || {
            opened.get().then(|| {
                let items = (0..child_count)
                    .map(|index| {
                        let mut child = path.clone();
                        child.push(index);
                        view! {
                            <DirectoryNode lookup=lookup.clone() book path=child />
                        }
                    })
                    .collect_view();
                let own = own_link.clone().map(|href| {
                    view! {
                        <li class="toc-leaf toc-self">
                            <a href=href>{format!("Read {child_title}")}</a>
                        </li>
                    }
                });
                view! { <ul class="toc-list">{own}{items}</ul> }
            })
        }
    };
    view! {
        <li class="toc-branch">
            <details class="toc-details" on:toggle=on_toggle>
                <summary>
                    <span class="toc-title">{title}</span>
                    <span class="toc-count" aria-label=format!("{under} entries")>{under}</span>
                </summary>
                {children}
            </details>
        </li>
    }
    .into_any()
}

/// The publication directory. The same `books.json` tree as the book page, built lazily.
/// Books are grouped by priority tier; the sort in the URL orders books inside each tier, and
/// library order shows one ungrouped list. Group and preset controls write the stored active
/// set once through [`Library::update_active_books`].
#[component]
pub fn Books() -> impl IntoView {
    set_title("Books");
    let library = use_library();
    let lookup = {
        let library = library.clone();
        local_resource(async move { library.books().await })
    };
    let saving = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let status = RwSignal::new(None::<String>);
    let current = use_route();
    let sort = move || match current.get() {
        Route::BooksSorted(sort) => Some(sort),
        _ => None,
    };
    // One cached pass over the catalog gives every row its count. Counting per row rescanned
    // the whole catalog for each publication and stalled the page for seconds.
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
    let apply = BulkApply {
        library: library.clone(),
        saving,
        save_error,
        status,
    };
    let tier_ids = |tier: Tier| -> Vec<String> {
        rows.iter()
            .filter(|row| Tier::from_category(row.category.as_deref()) == tier)
            .map(|row| row.id.clone())
            .collect()
    };
    let rulebook_ids = tier_ids(Tier::Rulebooks);
    let other_ids: Vec<String> = rows
        .iter()
        .filter(|row| Tier::from_category(row.category.as_deref()) != Tier::Rulebooks)
        .map(|row| row.id.clone())
        .collect();
    let no_rulebooks = rulebook_ids.is_empty();
    let only_rulebooks = {
        let apply = apply.clone();
        move |_| {
            apply.run(
                rulebook_ids.clone(),
                other_ids.clone(),
                "Only Rulebooks are active. Other books keep their downloads and your stars."
                    .to_string(),
            )
        }
    };
    let sort_link = move |value: Option<BookSort>, label: &'static str| {
        let href = value.map_or_else(
            || Route::Books.href(),
            |sort| Route::BooksSorted(sort).href(),
        );
        view! {
            <li>
                <a class="jump" href=href aria-current=move || (sort() == value).then_some("true")>{label}</a>
            </li>
        }
    };
    view! {
        <div class="wide">
            <h1>"Books"</h1>
            <section>
                <p class="muted measure">
                    "Browse each book through its sections and contents. "
                    "Each book explains which parts of its order still need checking against the printed contents."
                </p>
                <p class="hint">"Active books appear in search and category browsing. Disabling a book keeps its download and your stars."</p>
                <div class="books-preset">
                    <button type="button" class="btn" disabled=move || saving.get() || no_rulebooks
                        on:click=only_rulebooks>"Only Rulebooks"</button>
                    <span class="hint">
                        {if no_rulebooks {
                            "This library has no books that are marked as Rulebooks."
                        } else {
                            "Keep Rulebooks active and deactivate every other book."
                        }}
                    </span>
                </div>
                <p id="books-scope" class="count-scope">{BOOK_TOTALS_SCOPE}</p>
                <nav class="book-sort" aria-label="Sort books">
                    <span class="sort-label">"Sort"</span>
                    <ul class="sort-options">
                        {sort_link(None, "Title")}
                        {sort_link(Some(BookSort::Category), BookSort::Category.label())}
                        {sort_link(Some(BookSort::Date), BookSort::Date.label())}
                        {sort_link(Some(BookSort::Library), BookSort::Library.label())}
                    </ul>
                </nav>
                {move || save_error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })}
                <p class="hint books-status" role="status">{move || status.get()}</p>
                {move || {
                    book_groups(&rows, sort())
                        .into_iter()
                        .map(|group| {
                            let ids: Vec<String> = group
                                .books
                                .iter()
                                .map(|&index| rows[index].id.clone())
                                .collect();
                            let items = group
                                .books
                                .into_iter()
                                .map(|index| book_row(rows[index].clone(), library.clone(), lookup, saving, save_error, status))
                                .collect_view();
                            match group.tier {
                                Some(tier) => tier_section(tier, ids, items, library.clone(), apply.clone(), saving),
                                None => view! {
                                    <ul class="book-directory" aria-describedby="books-scope">{items}</ul>
                                }.into_any(),
                            }
                        })
                        .collect_view()
                }}
            </section>
        </div>
    }
}

/// Shown when a click arrives while an earlier selection write is still being saved.
const SAVE_BUSY: &str = "A book selection update is already being saved. Try again shortly.";

/// The write path for the group and preset controls. It reuses the busy flag and the error line
/// of the per-book checkboxes, so only one selection write runs at a time.
#[derive(Clone)]
struct BulkApply {
    library: Library,
    saving: RwSignal<bool>,
    save_error: RwSignal<Option<String>>,
    status: RwSignal<Option<String>>,
}

impl BulkApply {
    fn run(&self, activate: Vec<String>, deactivate: Vec<String>, message: String) {
        // Defensive guard: every control that calls this is disabled during a save, so this
        // runs only for a programmatic event.
        if self.saving.get_untracked() {
            self.status.set(None);
            self.save_error.set(Some(SAVE_BUSY.to_string()));
            return;
        }
        self.saving.set(true);
        self.save_error.set(None);
        self.status.set(None);
        let this = self.clone();
        spawn_local(async move {
            match this
                .library
                .update_active_books(&activate, &deactivate)
                .await
            {
                Ok(()) => this.status.set(Some(message)),
                Err(error) => this
                    .save_error
                    .set(Some(format!("Could not save the book selection: {error}"))),
            }
            this.saving.set(false);
        });
    }
}

/// One tier: a heading inside a native disclosure, and the group's two bulk controls. The
/// controls sit outside the disclosure, so a collapsed group still deactivates in one click.
/// Their names carry the group, such as `Deactivate all Adventures`.
fn tier_section(
    tier: Tier,
    ids: Vec<String>,
    items: impl IntoView + 'static,
    library: Library,
    apply: BulkApply,
    saving: RwSignal<bool>,
) -> AnyView {
    let label = tier.label();
    let count = ids.len();
    let heading_id = format!("book-tier-{}", label.to_ascii_lowercase().replace(' ', "-"));
    let labelled_by = heading_id.clone();
    let revision = library.selection_revision();
    let active_ids = ids.clone();
    let active_count = move || {
        revision.get();
        active_ids
            .iter()
            .filter(|id| library.book_active(id))
            .count()
    };
    let activate = {
        let apply = apply.clone();
        let ids = ids.clone();
        move |_| {
            apply.run(
                ids.clone(),
                Vec::new(),
                format!("{label}: all {count} books are active."),
            )
        }
    };
    let deactivate = move |_| {
        apply.run(
            Vec::new(),
            ids.clone(),
            format!("{label}: all {count} books are inactive. Downloads and your stars are kept."),
        )
    };
    view! {
        <section class="book-tier" aria-labelledby=labelled_by data-tier=label>
            <div class="tier-actions" role="group" aria-label=format!("{label} activation")>
                <button type="button" class="btn" aria-label=format!("Activate all {label}")
                    disabled=move || saving.get() on:click=activate>"Activate all"</button>
                <button type="button" class="btn" aria-label=format!("Deactivate all {label}")
                    disabled=move || saving.get() on:click=deactivate>"Deactivate all"</button>
            </div>
            <details class="tier-details" open=!tier.collapsed_by_default()>
                <summary class="tier-summary">
                    <h2 class="sub-head book-group-head" id=heading_id.clone()>{format!("{label} ({count})")}</h2>
                    <span class="tier-active">{move || format!("{} of {count} active", active_count())}</span>
                </summary>
                <ul class="book-directory" aria-describedby="books-scope">{items}</ul>
            </details>
        </section>
    }
    .into_any()
}

/// One group of lens chips, with the first few shown and the rest behind a disclosure.
#[component]
fn FacetLens(kind: EntryKind, field: &'static str, label: &'static str) -> impl IntoView {
    const VISIBLE: usize = 16;
    let library = use_library();
    let values = local_resource(async move {
        library.discovery().facet_values(field).await.map(|values| {
            let mut values: Vec<(String, u32)> = values
                .into_iter()
                .filter(|(v, _)| !categories::lens_skips(kind, field, v))
                .collect();
            sort_facet_pairs(field, &mut values);
            values
        })
    });
    let chip = move |(value, n): (String, u32)| {
        let route = Route::Search {
            q: String::new(),
            kinds: vec![kind],
            sources: Vec::new(),
            facets: vec![(field.to_string(), value.clone())],
        };
        view! {
            <li>
                <a class="lens-chip" href=route.href()>
                    {facet_label(field, &value)}<small>{fmt::count(n)}</small>
                </a>
            </li>
        }
    };
    view! {
        <section class="lens" aria-labelledby=format!("lens-{field}")>
            <h2 id=format!("lens-{field}") class="sub-head">{label}</h2>
            {move || values.with(|state| match state {
                None => view! { <p class="hint lens-wait" role="status">"Loading filters…"</p> }.into_any(),
                Some(Err(_)) => view! {
                    <p class="hint">"The filters could not be loaded. Search still works."</p>
                }.into_any(),
                Some(Ok(values)) if values.is_empty() => ().into_any(),
                Some(Ok(values)) => {
                    let (first, rest) = values.split_at(values.len().min(VISIBLE));
                    let first = first.iter().cloned().map(chip).collect_view();
                    let rest_view = (!rest.is_empty()).then(|| {
                        let n = rest.len();
                        let rest = rest.iter().cloned().map(chip).collect_view();
                        view! {
                            <details class="lens-more">
                                <summary>{format!("{} more", fmt::count(n))}</summary>
                                <ul class="lens-chips">{rest}</ul>
                            </details>
                        }
                    });
                    view! {
                        <ul class="lens-chips">{first}</ul>
                        {rest_view}
                    }.into_any()
                }
            })}
        </section>
    }
}

/// The facets with one value changed: `value` replaces the field's value, and `None` removes the
/// field. The other fields keep theirs, so two menus on one page do not undo each other.
pub fn facets_with(
    current: &[(String, String)],
    field: &str,
    value: Option<&str>,
) -> Vec<(String, String)> {
    let mut next: Vec<(String, String)> = current
        .iter()
        .filter(|(f, _)| f != field)
        .cloned()
        .collect();
    if let Some(value) = value {
        next.push((field.to_string(), value.to_string()));
    }
    next
}

/// The choices of a menu from the index's values: the label as the page says it, the count,
/// and the filtered list each one opens.
fn menu_options(
    kind: EntryKind,
    spec: &MenuSpec,
    current: &[(String, String)],
    values: &[(String, u32)],
) -> Vec<MenuOption> {
    values
        .iter()
        .map(|(value, count)| MenuOption {
            value: value.clone(),
            label: facet_label(spec.field, value),
            count: *count,
            href: Route::browse_kind_facets(kind, facets_with(current, spec.field, Some(value)))
                .href(),
        })
        .collect()
}

type FacetValues = ReadSignal<Option<Result<Vec<(String, u32)>, LibraryError>>, LocalStorage>;

/// One filter menu, as the button for the filter row and the panel that goes under the row.
/// Both read the values of the facet from the search index, which the page loads once.
fn filter_menu(
    library: &Library,
    kind: EntryKind,
    spec: MenuSpec,
    applied: &[(String, String)],
    focus: bool,
) -> (AnyView, AnyView) {
    let values: FacetValues = {
        let library = library.clone();
        local_resource(async move {
            library
                .discovery()
                .facet_values(spec.field)
                .await
                .map(|mut values| {
                    sort_facet_pairs(spec.field, &mut values);
                    values
                })
        })
    };
    let state = MenuState::new();
    let value = applied
        .iter()
        .find(|(field, _)| field == spec.field)
        .map(|(_, value)| value.clone());
    let applied_for_text = applied.to_vec();
    let value_for_text = value.clone();
    let text = Signal::derive(move || {
        values.with(|loaded| match loaded {
            None => None,
            Some(Err(_)) => Some(trigger_text(&spec, None)),
            Some(Ok(values)) => {
                let options = menu_options(kind, &spec, &applied_for_text, values);
                Some(trigger_text(
                    &spec,
                    current_option(&options, value_for_text.as_deref()),
                ))
            }
        })
    });
    let value_for_narrowed = value.clone();
    let applied_for_narrowed = applied.to_vec();
    let narrowed = Signal::derive(move || {
        values.with(|loaded| match loaded {
            Some(Ok(values)) => {
                let options = menu_options(kind, &spec, &applied_for_narrowed, values);
                current_option(&options, value_for_narrowed.as_deref()).is_some()
            }
            _ => false,
        })
    });
    let applied_for_panel = applied.to_vec();
    let all_href = Route::browse_kind_facets(kind, facets_with(applied, spec.field, None)).href();
    let button = move || {
        values.with(|loaded| match loaded {
            // The filters could not be loaded: the hint under the row says so.
            Some(Err(_)) => None,
            Some(Ok(values)) if values.is_empty() => None,
            _ => Some(view! { <FilterMenuButton spec state text narrowed focus /> }),
        })
    };
    let panel = move || {
        values.with(|loaded| match loaded {
            Some(Ok(values)) if !values.is_empty() => {
                let options = menu_options(kind, &spec, &applied_for_panel, values);
                Some(view! {
                    <FilterMenuPanel spec state options current=value.clone() all_href=all_href.clone() />
                }.into_any())
            }
            Some(Err(_)) => Some(view! {
                <p class="hint">"The filters could not be loaded. Search still works."</p>
            }
            .into_any()),
            _ => None,
        })
    };
    (
        view! { <div class="filter-menu">{button}</div> }.into_any(),
        panel.into_any(),
    )
}

/// The filter row of a list page: the filter menus on the left, the related pages as plain
/// links on the right, and each menu's panel under the row. Races and creatures keep their pills
/// (the old lens) until the library has the data or their pull request moves them to this row.
#[component]
fn Lenses(
    kind: EntryKind,
    /// The facets that narrow the list now.
    applied: Vec<(String, String)>,
    /// True when a choice just opened this page.
    focus: bool,
) -> impl IntoView {
    let all = categories::lenses(kind);
    if all.is_empty() {
        return ().into_any();
    }
    let library = use_library();
    let mut facets = Vec::new();
    let mut buttons = Vec::new();
    let mut panels = Vec::new();
    let mut links = Vec::new();
    for lens in all {
        match lens {
            Lens::Facet { field, label } => facets.push((field, label)),
            Lens::Menu(spec) => {
                let (button, panel) = filter_menu(&library, kind, spec, &applied, focus);
                buttons.push(button);
                panels.push(panel);
            }
            Lens::Link { label, route } => {
                // The spell and feat pages need the schema-3 files; an old library has none.
                let usable = match route {
                    Route::Spells => library.has_spell_lists(),
                    Route::FeatChains => library.has_relations(),
                    _ => true,
                };
                if usable {
                    links.push((label, route));
                }
            }
        }
    }
    let facets = facets
        .into_iter()
        .map(|(field, label)| view! { <FacetLens kind field label /> })
        .collect_view();
    let row = (!buttons.is_empty() || !links.is_empty()).then(|| {
        let links = links
            .into_iter()
            .map(|(label, route)| {
                view! {
                    <a class="page-link" href=route.href()>
                        <span>{label}</span>
                        <svg viewBox="0 0 24 24" width="20" height="20" fill="none"
                            stroke="currentColor" stroke-width="1.8" stroke-linecap="round"
                            stroke-linejoin="round" aria-hidden="true">
                            <path d="m9 6 6 6-6 6"/>
                        </svg>
                    </a>
                }
            })
            .collect_view();
        view! { <div class="filter-row">{buttons}{links}</div> }
    });
    view! {
        <div class="lenses">
            {row}
            {panels}
            {facets}
        </div>
    }
    .into_any()
}

/// Kinds with more entries than this open one letter group at a time. Shorter kinds stay a
/// plain list.
const GROUP_THRESHOLD: usize = 200;

/// The count line under a list page's title: `3,583 feats`, `1 feat`. It names the kind, so it
/// reads as the size of this list. The counts are totals for the active books.
pub fn kind_count_line(kind: EntryKind, n: usize) -> String {
    let noun = if n == 1 {
        kind_name(kind).to_lowercase()
    } else {
        kind.label().to_lowercase()
    };
    format!("{} {noun}", fmt::count(n))
}

fn entry_count(n: usize) -> String {
    if n == 1 {
        "1 entry".to_string()
    } else {
        format!("{} entries", fmt::count(n))
    }
}

/// The jump bar for a long kind. It is one wrapping row on desktop and one horizontally
/// scrolling row on phones, where a wrapped sticky bar would cover too much of the viewport.
#[component]
fn LetterNav(
    kind: EntryKind,
    facets: Vec<(String, String)>,
    letters: Vec<char>,
    selected: Option<char>,
) -> impl IntoView {
    Effect::new(move |_| {
        // Keep the open letter visible in the phone's single scrolling row. Read the DOM
        // directly: a pending frame against a disposed NodeRef would panic after remount.
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Some(nav) = document
            .query_selector(".letter-nav")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return;
        };
        let Some(link) = document
            .query_selector(".letter-btn[aria-current='true']")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return;
        };
        let target = link.offset_left() - (nav.client_width() - link.client_width()) / 2;
        nav.set_scroll_left(target.max(0));
    });
    let links = letters
        .iter()
        .map(|letter| {
            let label = letter.to_string();
            let jump_label = label.clone();
            let href = Route::browse_kind_facets(kind, facets.clone())
                .with_letter(*letter)
                .href();
            let current = Some(*letter) == selected;
            if current {
                view! {
                    <li>
                        <a
                            class="letter-btn"
                            href=href
                            aria-current="true"
                            aria-label=format!("Jump to {jump_label}")
                        >{label}</a>
                    </li>
                }
                .into_any()
            } else {
                view! {
                    <li>
                        <a class="letter-btn" href=href aria-label=format!("Jump to {jump_label}")>
                            {label}
                        </a>
                    </li>
                }
                .into_any()
            }
        })
        .collect_view();
    view! {
        <nav class="letter-nav" aria-label="Jump to letter">
            <ul>{links}</ul>
        </nav>
    }
}

/// An A to Z list. A long kind groups its rows by first letter and renders only the selected
/// group, so a 3,000-entry category no longer puts every link in the document. Short kinds
/// render as one flat list.
#[component]
fn AzList(
    kind: EntryKind,
    facets: Vec<(String, String)>,
    marks: Option<ChainMarks>,
    groups: Vec<(char, Vec<usize>)>,
    stubs: Vec<EntryStub>,
    source_titles: Vec<Option<String>>,
    show_source: bool,
    selected: Option<char>,
    focus: bool,
) -> impl IntoView {
    let groups = Shared::new(groups);
    let stubs = Shared::new(stubs);
    let source_titles = Shared::new(source_titles);
    // Group only for presentation, after the caller has selected active catalog stubs. The
    // letter index still counts source records, so the counts match the category total.
    let mut grouped_by_entry = Vec::with_capacity(stubs.len());
    for (i, stub) in stubs.iter().enumerate() {
        let source = source_titles[i]
            .clone()
            .unwrap_or_else(|| "Source".to_string());
        grouped_by_entry.push(SourceVariant {
            id: stub.id.clone(),
            title: stub.title.clone(),
            source,
            kind: stub.kind,
        });
    }
    let presentation_groups = group_variants(grouped_by_entry);
    let variant_group_by_id = presentation_groups
        .iter()
        .enumerate()
        .flat_map(|(group_index, group)| {
            group
                .variants
                .iter()
                .map(move |v| (v.id.clone(), group_index))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let entry_index_by_id = stubs
        .iter()
        .enumerate()
        .map(|(index, stub)| (stub.id.clone(), index))
        .collect::<std::collections::HashMap<_, _>>();

    // One list item per entry, with same-title records folded into one disclosure. The indices
    // belong to one letter group, or to the whole list when the kind is short.
    let rows_for = {
        let stubs = stubs.clone();
        let source_titles = source_titles.clone();
        let presentation_groups = presentation_groups.clone();
        let variant_group_by_id = variant_group_by_id.clone();
        let entry_index_by_id = entry_index_by_id.clone();
        move |indices: &[usize]| {
            let mut rendered = Vec::<AnyView>::new();
            let mut consumed = std::collections::HashSet::new();
            for &i in indices {
                if !consumed.insert(i) {
                    continue;
                }
                let stub = &stubs[i];
                let group = variant_group_by_id
                    .get(&stub.id)
                    .and_then(|group_index| presentation_groups.get(*group_index));
                if let Some(group) = group.filter(|g| g.variants.len() > 1) {
                    for variant in &group.variants {
                        if let Some(index) = entry_index_by_id.get(&variant.id) {
                            consumed.insert(*index);
                        }
                    }
                    rendered
                        .push(view! { <li><SourceVariants group=group.clone() /></li> }.into_any());
                } else {
                    let source = show_source.then(|| source_titles[i].clone()).flatten();
                    rendered.push(match marks {
                        Some(marks) => feat_row(stub, source, marks).into_any(),
                        None => entry_row(stub, source).into_any(),
                    });
                }
            }
            rendered
        }
    };

    // Short lists keep the plain, ungrouped list.
    if stubs.len() <= GROUP_THRESHOLD {
        let indices: Vec<usize> = (0..stubs.len()).collect();
        let rows = rows_for(&indices);
        return view! { <ul class="entry-list">{rows}</ul> }.into_any();
    }

    if focus && let Some(letter) = selected {
        // The route frame scrolls to the top and focuses <main> in the next frame, so wait one
        // more frame before moving focus to the group heading. Query the DOM by id: a pending
        // frame against a disposed NodeRef would panic when the route changes again.
        let heading_id = format!("{}-heading", letter_id(letter));
        Effect::new(move |_| {
            let heading_id = heading_id.clone();
            request_animation_frame(move || {
                let heading_id = heading_id.clone();
                request_animation_frame(move || {
                    let Some(heading) = web_sys::window()
                        .and_then(|window| window.document())
                        .and_then(|document| document.get_element_by_id(&heading_id))
                        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                    else {
                        return;
                    };
                    let _ = heading.focus();
                });
            });
        });
    }

    let mut selected_rows = selected
        .and_then(|letter| groups.iter().position(|(group, _)| *group == letter))
        .map(|position| rows_for(&groups[position].1));
    let sections = groups
        .iter()
        .map(|(group_letter, indices)| {
            let n = indices.len();
            let count = entry_count(n);
            let label = group_letter.to_string();
            let dom_id = letter_id(*group_letter);
            let is_selected = Some(*group_letter) == selected;
            if is_selected {
                let rows = selected_rows.take().unwrap_or_default();
                view! {
                    <section class="letter-group" id=dom_id data-letter=label.clone()>
                        <h2 class="letter-head" id=format!("{dom_id}-heading") tabindex="-1">
                            {label.clone()}<span class="letter-count">{format!(" {count}")}</span>
                        </h2>
                        <ul class="entry-list">{rows}</ul>
                    </section>
                }
                .into_any()
            } else {
                let href = Route::browse_kind_facets(kind, facets.clone())
                    .with_letter(*group_letter)
                    .href();
                let group_name = format!("{label}, {count}");
                view! {
                    <section class="letter-group" id=dom_id data-letter=label.clone()>
                        <h2 class="letter-head">
                            // The whole collapsed row is the one link to this letter.
                            <a class="letter-head-link" href=href aria-label=group_name>
                                {label.clone()}<span class="letter-count">{format!(" {count}")}</span>
                            </a>
                        </h2>
                    </section>
                }
                .into_any()
            }
        })
        .collect_view();

    view! { {sections} }.into_any()
}

/// The count line of a narrowed list: `1,324 of 3,583 feats`. The value that narrows it is in
/// the filter button, so the line does not repeat it.
pub fn narrowed_count_line(kind: EntryKind, shown: usize, total: usize) -> String {
    format!("{} of {}", fmt::count(shown), kind_count_line(kind, total))
}

/// What a narrowed list says when nothing is left: `No Combat feats in your active books.`
pub fn empty_filter_line(kind: EntryKind, applied: &[(String, String)]) -> String {
    let scope = Scope {
        kind,
        facets: applied.to_vec(),
    };
    format!("No {} in your active books.", scope.label())
}

/// A kind's A to Z list. The URL may narrow it by a facet (`?facet=feat_type:Combat`); the
/// index then names the entries that carry the value, and the list, its letters and its count
/// all follow. Without a facet the list is built at once.
#[component]
pub fn BrowseKind(
    kind: EntryKind,
    letter: Option<char>,
    #[prop(optional)] facets: Vec<(String, String)>,
) -> impl IntoView {
    set_title(kind.label());
    let library = use_library();
    // A choice in a filter menu asks the page it opens to focus the menu's button.
    let focus_menu = take_focus_request();
    let applied = applied_facets(kind, &facets);
    let all = Shared::new(sorted_of_kind(&library, kind));
    if let Some(field) = categories::grouped_by(kind) {
        return view! { <GroupedKind kind field all /> }.into_any();
    }
    if applied.is_empty() {
        let stubs = all.to_vec();
        return view! {
            <BrowseList kind letter applied=applied stubs total=all.len() focus_menu />
        }
        .into_any();
    }

    let ids = {
        let (library, applied) = (library.clone(), applied.clone());
        local_resource(async move {
            // Several facets narrow together: a feat has to carry every value.
            let discovery = library.discovery();
            let mut ids: Option<HashSet<String>> = None;
            for (field, value) in &applied {
                let found = discovery.facet_entry_ids(field, value).await?;
                ids = Some(match ids {
                    None => found,
                    Some(earlier) => earlier.intersection(&found).cloned().collect(),
                });
            }
            Ok::<_, LibraryError>(ids.unwrap_or_default())
        })
    };
    let page = move || {
        ids.with(|loaded| match loaded {
            None => view! {
                <div class="wide list-page">
                    <h1>{kind.label()}</h1>
                    <p class="muted list-count" role="status" aria-live="polite">"Loading…"</p>
                </div>
            }
            .into_any(),
            Some(Err(error)) => view! {
                <div class="wide list-page">
                    <h1>{kind.label()}</h1>
                    <p class="error" role="alert">{error.to_string()}</p>
                </div>
            }
            .into_any(),
            Some(Ok(ids)) => {
                let stubs: Vec<EntryStub> = all
                    .iter()
                    .filter(|stub| ids.contains(stub.id.as_str()))
                    .cloned()
                    .collect();
                view! {
                    <BrowseList kind letter applied=applied.clone() stubs total=all.len() focus_menu />
                }
                .into_any()
            }
        })
    };
    view! { {page} }.into_any()
}

/// Entries under headings by a facet, for a short kind whose few groups are better seen than
/// chosen: classes by type. The groups come from the search index, so they follow the active
/// books and show a group only while one of its entries is active. Until the index answers the
/// page shows its head alone, and if it fails the list falls back to plain A to Z.
#[component]
fn GroupedKind(kind: EntryKind, field: &'static str, all: Shared<Vec<EntryStub>>) -> impl IntoView {
    let library = use_library();
    let focus_menu = false;
    let total = all.len();
    let groups = {
        let library = library.clone();
        local_resource(async move {
            let discovery = library.discovery();
            let values: Vec<String> = discovery
                .facet_values(field)
                .await?
                .into_iter()
                .map(|(value, _)| value)
                .collect();
            let mut found = Vec::new();
            for value in categories::order_class_groups(&values) {
                let ids = discovery.facet_entry_ids(field, &value).await?;
                found.push((value, ids));
            }
            Ok::<_, LibraryError>(found)
        })
    };
    let page = move || {
        groups.with(|loaded| match loaded {
            None => view! {
                <div class="wide list-page">
                    <h1>{kind.label()}</h1>
                    <p class="muted list-count" role="status" aria-live="polite">"Loading…"</p>
                </div>
            }
            .into_any(),
            Some(Err(_)) => {
                let stubs = all.to_vec();
                view! {
                    <BrowseList kind letter=None applied=Vec::new() stubs total focus_menu />
                }
                .into_any()
            }
            Some(Ok(found)) => {
                let sections = sections_by_group(field, &all, found);
                let stubs = all.to_vec();
                view! {
                    <BrowseList kind letter=None applied=Vec::new() stubs total focus_menu
                        sections />
                }
                .into_any()
            }
        })
    };
    view! { {page} }
}

/// A group heading and the entries under it.
type Section = (String, Vec<EntryStub>);

/// The entries under their group headings, in the order given, then the entries no group holds
/// under a final `Other`. A group with no entry is left out, so the page never shows an empty
/// heading. The heading is the page's label for the value; the value itself is not changed.
fn sections_by_group(
    field: &str,
    all: &[EntryStub],
    groups: &[(String, HashSet<String>)],
) -> Vec<Section> {
    let mut sections = Vec::new();
    for (value, ids) in groups {
        let members: Vec<EntryStub> = all
            .iter()
            .filter(|stub| ids.contains(stub.id.as_str()))
            .cloned()
            .collect();
        if !members.is_empty() {
            sections.push((facet_label(field, value), members));
        }
    }
    let rest: Vec<EntryStub> = all
        .iter()
        .filter(|stub| !groups.iter().any(|(_, ids)| ids.contains(stub.id.as_str())))
        .cloned()
        .collect();
    if !rest.is_empty() {
        sections.push(("Other".to_string(), rest));
    }
    sections
}

/// The list page itself, over the entries that are left after the filter. The list-page
/// template (DESIGN.md): title, count, the filter row, the letter bar, the rows. No breadcrumb,
/// lede or search link: the tab bar and the brand lead home, the category cards on Home carry
/// the blurb, and the header search icon opens Quick search scoped to this list.
#[component]
fn BrowseList(
    kind: EntryKind,
    letter: Option<char>,
    applied: Vec<(String, String)>,
    stubs: Vec<EntryStub>,
    /// The size of the whole kind, before the filter.
    total: usize,
    focus_menu: bool,
    /// A short list under headings, in order: `(heading, entries)`. Without it the list is one
    /// A to Z list.
    #[prop(optional)]
    sections: Option<Vec<Section>>,
) -> impl IntoView {
    let library = use_library();
    let count = stubs.len();
    let narrowed = !applied.is_empty();
    let multi_source = stubs.iter().any(|s| s.source != stubs[0].source);
    let titles: Vec<&str> = stubs.iter().map(|s| s.title.as_str()).collect();
    let groups = group_by_letter(&titles);
    let requested = letter.filter(|letter| groups.iter().any(|(group, _)| group == letter));
    let selected = requested.or_else(|| groups.first().map(|(letter, _)| *letter));
    // A letter the list does not have still renders the first group; make the URL match it.
    if letter.is_some() && requested.is_none() {
        let route = Route::browse_kind_facets(kind, applied.clone());
        Effect::new(move |_| replace(&route));
    }
    let letters: Vec<char> = groups.iter().map(|(letter, _)| *letter).collect();
    let jump = (count > GROUP_THRESHOLD).then(|| {
        view! { <LetterNav kind facets=applied.clone() letters=letters selected=selected /> }
    });

    // Feats that open a long chain carry a marker. The numbers come from the relations graph,
    // read once for the page; the rows do not wait for them.
    let marks: Option<ChainMarks> =
        (kind == EntryKind::Feat && library.has_relations()).then(|| {
            let library = library.clone();
            local_resource(async move {
                let relations = library.discovery().relations().await.ok().flatten()?;
                Some(Shared::new(relations.long_chains(CHAIN_MARK_MIN)))
            })
        });

    let sectioned = sections.is_some();
    let body = if count == 0 {
        // The value is not in the active books. Say so, and offer the two ways out.
        view! {
            <p>{empty_filter_line(kind, &applied)}</p>
            <p class="empty-actions">
                <a class="count-action" href=Route::browse_kind(kind).href()>
                    {format!("Show all {}", kind.label().to_lowercase())}
                </a>
                " · "
                <a class="count-action" href=Route::Books.href()>"Choose books"</a>
            </p>
        }
        .into_any()
    } else if let Some(sections) = sections {
        let sections = sections
            .into_iter()
            .enumerate()
            .map(|(i, (label, group))| {
                let heading_id = format!("group-{i}");
                let labelled_by = heading_id.clone();
                let n = group.len();
                let source_titles: Vec<Option<String>> = group
                    .iter()
                    .map(|s| library.source(&s.source).map(|src| src.title.clone()))
                    .collect();
                view! {
                    <section class="letter-group" aria-labelledby=labelled_by>
                        <h2 class="letter-head" id=heading_id>
                            {label}<span class="letter-count">{fmt::count(n)}</span>
                        </h2>
                        <AzList kind facets=Vec::new() marks=None groups=Vec::new() stubs=group
                            source_titles show_source=multi_source selected=None focus=false />
                    </section>
                }
            })
            .collect_view();
        view! { {sections} }.into_any()
    } else if kind == EntryKind::Rule {
        let items: Vec<(EntryId, Option<EntryId>, String)> = stubs
            .iter()
            .map(|s| (s.id.clone(), s.parent.clone(), s.title.clone()))
            .collect();
        let tree = build_tree(&items);
        view! { <ul class="tree">{tree_view(&tree, &stubs)}</ul> }.into_any()
    } else {
        let source_titles: Vec<Option<String>> = stubs
            .iter()
            .map(|s| library.source(&s.source).map(|src| src.title.clone()))
            .collect();
        view! {
            <AzList kind facets=applied.clone() marks groups stubs=stubs.clone() source_titles
                show_source=multi_source selected=selected
                focus=requested.is_some() && !focus_menu />
        }
        .into_any()
    };

    let count_line = if narrowed {
        narrowed_count_line(kind, count, total)
    } else {
        kind_count_line(kind, total)
    };
    let show_all = narrowed.then(|| {
        view! {
            " · "
            <a class="count-action" href=Route::browse_kind(kind).href()
                aria-label=format!("Show all {}", kind.label().to_lowercase())>"Show all"</a>
        }
    });

    view! {
        <div class="wide list-page">
        <h1>{kind.label()}</h1>
        <p class="muted list-count" role="status" aria-live="polite">{count_line}{show_all}</p>
        <Lenses kind applied=applied.clone() focus=focus_menu />
        {jump}
        {(!sectioned).then(|| view! { <h2 class="sr-only">"All, A to Z"</h2> })}
        {body}
        </div>
    }
}

fn letter_id(letter: char) -> String {
    if letter == '#' {
        "letter-other".to_string()
    } else {
        format!("letter-{letter}")
    }
}

fn tree_view(nodes: &[TreeNode], stubs: &[EntryStub]) -> Vec<AnyView> {
    nodes
        .iter()
        .map(|node| {
            let stub = &stubs[node.index];
            let children = (!node.children.is_empty())
                .then(|| view! { <ul>{tree_view(&node.children, stubs)}</ul> });
            let class = if node.children.is_empty() {
                "leaf"
            } else {
                "branch"
            };
            view! {
                <li class=class>
                    <a href=Route::entry(stub.id.clone()).href()>{stub.title.clone()}</a>
                    {children}
                </li>
            }
            .into_any()
        })
        .collect()
}

#[component]
pub fn BrowseSource(id: String, #[prop(optional)] bare: bool) -> impl IntoView {
    let library = use_library();
    let Some(source) = library.source(&id).cloned() else {
        set_title("Publication not found");
        return view! {
            <div class="wide">
            <h1>"Publication not found"</h1>
            <p>{format!("This library has no publication {id:?}.")}</p>
            <p><a href=Route::Home.href()>"Back to Home"</a></p>
            </div>
        }
        .into_any();
    };
    set_title(&source.title);
    let groups = EntryKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let mut stubs: Vec<EntryStub> = library
                .discovery()
                .entries_of_source(&id)
                .filter(|s| s.kind == kind)
                .cloned()
                .collect();
            if stubs.is_empty() {
                return None;
            }
            stubs.sort_by_key(|s| (sort_key(&s.title), s.id.clone()));
            Some((kind, stubs))
        })
        .map(|(kind, stubs)| {
            let n = stubs.len();
            let opened = RwSignal::new(n <= 30);
            let stubs = StoredValue::new(stubs);
            view! {
                <details
                    class="kind-group"
                    open=n <= 30
                    on:toggle=move |ev| {
                        let open = event_target::<web_sys::HtmlDetailsElement>(&ev).open();
                        if open { opened.set(true); }
                    }
                >
                    <summary>
                        <span class="tile-name">{kind.label()}</span>
                        <span class="tile-count">{fmt::count(n)}</span>
                    </summary>
                    {move || opened.get().then(|| {
                        let rows = stubs.with_value(|list| {
                            list.iter().map(|s| entry_row(s, None).into_any()).collect::<Vec<_>>()
                        });
                        view! { <ul class="entry-list">{rows}</ul> }
                    })}
                </details>
            }
        })
        .collect_view();
    if bare {
        // Inside the book view, which already has the title, copyright line, and legal link.
        return view! { <div class="kind-groups">{groups}</div> }.into_any();
    }
    let copyright = source.copyright.clone();
    let count = active_count_line(library.discovery().entries_of_source(&id).count());
    view! {
        <div class="wide">
        <nav class="crumbs" aria-label="Breadcrumb">
            <ol><li><a href=Route::Home.href()>"Home"</a></li></ol>
        </nav>
        <h1>{source.title.clone()}</h1>
        <p class="muted" data-testid="source-count">{count}</p>
        {copyright.map(|line| view! { <p class="copyright">{line}</p> })}
        <p class="muted"><a href=Route::Legal.href()>"Legal and attribution"</a></p>
        <div class="kind-groups">{groups}</div>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> EntryId {
        EntryId::parse(s).unwrap()
    }

    #[test]
    fn the_count_line_names_the_kind() {
        assert_eq!(kind_count_line(EntryKind::Feat, 3583), "3,583 feats");
        assert_eq!(kind_count_line(EntryKind::Feat, 999), "999 feats");
        assert_eq!(kind_count_line(EntryKind::Feat, 1), "1 feat");
        assert_eq!(kind_count_line(EntryKind::Feat, 0), "0 feats");
        assert_eq!(
            kind_count_line(EntryKind::PrestigeClass, 40),
            "40 prestige classes"
        );
        assert_eq!(kind_count_line(EntryKind::Class, 1), "1 class");
    }

    #[test]
    fn letters_group_in_order_with_other_last() {
        let titles = ["1st Strike", "Acid", "Alarm", "Bless", "(Zone)"];
        let groups = group_by_letter(&titles);
        assert_eq!(
            groups,
            vec![
                ('A', vec![1, 2]),
                ('B', vec![3]),
                ('Z', vec![4]),
                ('#', vec![0])
            ]
        );
    }

    fn facet(field: &str, value: &str) -> (String, String) {
        (field.to_string(), value.to_string())
    }

    #[test]
    fn a_narrowed_count_line_gives_the_part_of_the_whole_without_the_value() {
        assert_eq!(
            narrowed_count_line(EntryKind::Feat, 1324, 3583),
            "1,324 of 3,583 feats"
        );
        assert_eq!(
            narrowed_count_line(EntryKind::Feat, 0, 3583),
            "0 of 3,583 feats"
        );
        assert_eq!(narrowed_count_line(EntryKind::Feat, 1, 1), "1 of 1 feat");
        assert_eq!(
            empty_filter_line(EntryKind::Feat, &[facet("feat_type", "Combat")]),
            "No Combat feats in your active books."
        );
        assert_eq!(
            empty_filter_line(EntryKind::Feat, &[facet("feat_type", "armor mastery")]),
            "No Armor mastery feats in your active books."
        );
    }

    #[test]
    fn choosing_a_value_replaces_that_field_and_keeps_the_others() {
        let current = [facet("feat_type", "Combat"), facet("cr", "5")];
        assert_eq!(
            facets_with(&current, "feat_type", Some("General")),
            [facet("cr", "5"), facet("feat_type", "General")]
        );
        assert_eq!(facets_with(&current, "feat_type", None), [facet("cr", "5")]);
        assert_eq!(
            facets_with(&[], "feat_type", Some("Combat")),
            [facet("feat_type", "Combat")]
        );
    }

    #[test]
    fn every_menu_option_opens_the_list_filtered_at_its_first_letter() {
        let values = vec![("Combat".to_string(), 1324), ("General".to_string(), 600)];
        let options = menu_options(
            EntryKind::Feat,
            &crate::views::categories::FEAT_TYPE_MENU,
            &[facet("feat_type", "General")],
            &values,
        );
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Combat");
        assert_eq!(options[0].count, 1324);
        // No `l=`: choosing a value resets the letter.
        assert_eq!(options[0].href, "#/browse/kind/feat?facet=feat_type:Combat");
        // The href parses back to the route the page then renders.
        assert_eq!(
            Route::parse(&options[1].href),
            Route::browse_kind_facets(EntryKind::Feat, vec![facet("feat_type", "General")])
        );
    }

    fn stub(id: &str, title: &str) -> EntryStub {
        EntryStub {
            id: self::id(id),
            kind: EntryKind::Class,
            title: title.into(),
            source: "core-rulebook".into(),
            parent: None,
            shard: "entries/class-000.json".into(),
        }
    }

    fn ids(list: &[&str]) -> HashSet<String> {
        list.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn classes_sit_under_their_group_headings_in_the_given_order_and_none_is_lost() {
        let all = [
            stub("ref/class/alchemist", "Alchemist"),
            stub("ref/class/bard", "Bard"),
            stub("ref/class/commoner", "Commoner"),
            stub("ref/class/mystery", "Mystery"),
        ];
        let groups = [
            ("core".to_string(), ids(&["ref/class/bard"])),
            ("base".to_string(), ids(&["ref/class/alchemist"])),
            // A group none of the active books holds shows no heading.
            ("hybrid".to_string(), ids(&[])),
            ("npc".to_string(), ids(&["ref/class/commoner"])),
        ];
        let sections = sections_by_group("class_type", &all, &groups);
        let names: Vec<&str> = sections.iter().map(|(label, _)| label.as_str()).collect();
        // An entry with no group is shown last, not dropped.
        assert_eq!(names, ["Core", "Base", "NPC", "Other"]);
        assert_eq!(sections[0].1[0].title, "Bard");
        assert_eq!(sections[3].1[0].title, "Mystery");
        let shown: usize = sections.iter().map(|(_, list)| list.len()).sum();
        assert_eq!(shown, all.len());

        // With every class in a group there is no `Other`.
        let none = sections_by_group("class_type", &all[..2], &groups[..2]);
        assert_eq!(none.len(), 2);
    }

    #[test]
    fn an_item_category_count_line_and_empty_state_name_the_kind() {
        assert_eq!(
            narrowed_count_line(EntryKind::Item, 1324, 8151),
            "1,324 of 8,151 items"
        );
        assert_eq!(kind_count_line(EntryKind::Item, 8151), "8,151 items");
        assert_eq!(
            empty_filter_line(EntryKind::Item, &[facet("item_category", "Rods")]),
            "No Rods in your active books."
        );
        assert_eq!(
            empty_filter_line(EntryKind::Spell, &[facet("school", "evocation")]),
            "No Evocation spells in your active books."
        );
    }

    #[test]
    fn the_chain_marker_names_how_many_feats_it_leads_to() {
        assert_eq!(chain_mark_suffix(93), ", leads to 93 feats");
        assert_eq!(chain_mark_suffix(1234), ", leads to 1,234 feats");
        assert_eq!(chain_mark_suffix(1), ", leads to 1 feat");
        assert_eq!(CHAIN_MARK_MIN, 5);
    }

    fn book(id: &str, title: &str, category: Option<&str>, date: Option<&str>) -> BookListEntry {
        BookListEntry {
            id: id.into(),
            title: title.into(),
            category: category.map(str::to_string),
            publication_date: date.map(str::to_string),
            count: 0,
        }
    }

    fn tiers_of(groups: &[BookGroup]) -> Vec<Tier> {
        groups.iter().map(|group| group.tier.unwrap()).collect()
    }

    #[test]
    fn library_order_is_one_ungrouped_list_in_catalog_order() {
        let books = vec![
            book("b", "Beta", Some("RPG"), Some("2010-01-01")),
            book("a", "Alpha", Some("Module"), None),
        ];
        let groups = book_groups(&books, Some(BookSort::Library));
        assert_eq!(
            groups,
            vec![BookGroup {
                tier: None,
                books: vec![0, 1]
            }]
        );
    }

    #[test]
    fn default_groups_follow_the_priority_tiers_and_omit_empty_ones() {
        let books = vec![
            book("m", "Mid", Some("Module"), None),
            book("z", "Zeta", Some("RPG"), None),
            book("u", "Unknown", None, None),
            book("a", "Alpha", Some("RPG"), None),
            book("c", "Companion", Some("Player Companion"), None),
            book("s", "Society", Some("Society"), None),
            book("o", "Other line", Some("Something new"), None),
            book("x", "Misc", Some("Miscellaneous"), None),
        ];
        let groups = book_groups(&books, None);
        assert_eq!(
            tiers_of(&groups),
            [
                Tier::Rulebooks,
                Tier::PlayerCompanions,
                Tier::Adventures,
                Tier::Other
            ],
            "no Campaign Setting book, so no Campaign Setting group"
        );
        assert_eq!(groups[0].books, vec![3, 1], "alphabetical inside a tier");
        assert_eq!(
            groups[2].books,
            vec![0, 5],
            "Module and Society in one tier"
        );
        assert_eq!(
            groups[3].books,
            vec![7, 6, 2],
            "Miscellaneous, an unrecognized value and Unknown share Other"
        );
        let all: usize = groups.iter().map(|group| group.books.len()).sum();
        assert_eq!(all, books.len(), "no book is dropped");
    }

    #[test]
    fn category_order_orders_product_lines_inside_a_tier() {
        let books = vec![
            book("a", "Alpha Module", Some("Module"), None),
            book("b", "Beta Path", Some("Adventure Path"), None),
            book("c", "Cast Path", Some("Adventure Path"), None),
            book("d", "Dawn Society", Some("Society"), None),
        ];
        let groups = book_groups(&books, Some(BookSort::Category));
        assert_eq!(tiers_of(&groups), [Tier::Adventures]);
        assert_eq!(groups[0].books, vec![1, 2, 0, 3]);
    }

    #[test]
    fn date_order_is_newest_first_unknown_last_and_ties_are_alphabetical() {
        let books = vec![
            book("a", "Alpha", Some("RPG"), Some("2009-08-01")),
            book("b", "Beta", Some("RPG"), Some("2014-08-14")),
            book("c", "Aardvark", Some("RPG"), Some("2014-08-14")),
            book("d", "Dated", Some("RPG"), Some("2014-07-01")),
            book("u", "Unknown", Some("RPG"), None),
            book("bad", "Bad", Some("RPG"), Some("not-a-date")),
            book("m", "Module", Some("Module"), Some("2020-01-01")),
        ];
        let groups = book_groups(&books, Some(BookSort::Date));
        assert_eq!(tiers_of(&groups), [Tier::Rulebooks, Tier::Adventures]);
        assert_eq!(
            groups[0].books,
            vec![2, 1, 3, 0, 5, 4],
            "newest date, then title for the shared date, then undated by title"
        );
        assert_eq!(groups[1].books, vec![6]);
    }

    #[test]
    fn tree_nests_children_and_orders_by_title() {
        let items = vec![
            (id("a/combat"), None, "Combat".to_string()),
            (
                id("a/combat/zeta"),
                Some(id("a/combat")),
                "Zeta".to_string(),
            ),
            (
                id("a/combat/alpha"),
                Some(id("a/combat")),
                "Alpha".to_string(),
            ),
            (id("a/alpha-root"), None, "Alpha Root".to_string()),
            (id("a/orphan"), Some(id("a/gone")), "Orphan".to_string()),
        ];
        let tree = build_tree(&items);
        let roots: Vec<usize> = tree.iter().map(|n| n.index).collect();
        assert_eq!(roots, vec![3, 0, 4]);
        let combat = &tree[1];
        assert_eq!(
            combat.children.iter().map(|n| n.index).collect::<Vec<_>>(),
            vec![2, 1]
        );
    }
}
