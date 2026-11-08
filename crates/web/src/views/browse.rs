//! Category browsing and the separate publication directory, with category lenses,
//! A to Z lists, the rules hierarchy, and the old publication page for libraries without books.

use crate::book_presets;
use crate::class_options;
use crate::fmt;
use crate::library::{Library, LibraryError, use_library};
use crate::route::{Route, navigate, replace};
use crate::views::active_books::{
    ACTIVE_LINK_ID, focus_after_frame, note_opened_from_books, take_link_focus,
};
use crate::views::book_tier::{Tier, group_is_open, remember_group};
use crate::views::categories::{self, Lens, applied_facets};
use crate::views::filter_menu::{
    FilterMenuButton, FilterMenuPanel, MenuOption, MenuSpec, MenuState, current_option,
    take_focus_request, trigger_text,
};
use crate::views::icons;
use crate::views::scope::{FilterPill, Scope};
use crate::views::search::{facet_label, sort_facet_pairs};
use crate::views::source_variants::{SourceVariant, SourceVariants, group_variants};
use crate::views::util::{
    ACTIVE_BOOKS_SCOPE, Shared, active_count_line, addon_name, group_letter, kind_name,
    local_resource, set_title, sort_key,
};
use cast_iron_schema::{EntryId, EntryKind, EntryStub, OptionGroupIndex};
use leptos::prelude::*;
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
    sort_rows(&mut list, kind);
    list
}

/// A book's entries of one kind, sorted by title. It reads the catalog, not the active
/// snapshot, so a disabled book still lists what it holds.
fn sorted_of_kind_in_source(library: &Library, kind: EntryKind, source: &str) -> Vec<EntryStub> {
    let mut list: Vec<EntryStub> = library
        .entries_of_kind(kind)
        .filter(|stub| library.stub_is_in_source(stub, source))
        .cloned()
        .collect();
    sort_rows(&mut list, kind);
    list
}

/// Order key for a feat row. A mythic version (`Dodge, Mythic`) sits directly after its base
/// (`Dodge`), before titles such as `Dodge Anything` that sort between the two strings. The
/// corpus titles every mythic version `{base}, Mythic`; the relation graph supplies the row's
/// label, and this supplies only its place.
fn feat_sort_key(title: &str) -> (String, u8) {
    const MYTHIC: &str = ", mythic";
    let lower = sort_key(title);
    match lower.strip_suffix(MYTHIC) {
        Some(base) if !base.is_empty() => (base.to_string(), 1),
        _ => (lower, 0),
    }
}

fn sort_rows(list: &mut [EntryStub], kind: EntryKind) {
    if kind == EntryKind::Feat {
        list.sort_by(|a, b| {
            feat_sort_key(&a.title)
                .cmp(&feat_sort_key(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });
    } else {
        list.sort_by_key(|s| (sort_key(&s.title), s.id.clone()));
    }
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
///
/// The add-on labels come from the same load: a mythic row reads `Mythic · {book}` and keeps
/// its label even while its base's book is inactive.
#[derive(Debug, Clone, Default)]
pub struct FeatRowInfo {
    pub marks: HashMap<EntryId, usize>,
    pub addons: HashMap<EntryId, String>,
}

pub type ChainMarks = ReadSignal<Option<Option<Shared<FeatRowInfo>>>, LocalStorage>;

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
    let icon = icons::chain_mark(&suffix);
    view! {
        <span class="sr-only">{suffix.clone()}</span>
        {icon}
    }
}

/// A feat row. The marker follows the title inside the link, so the row stays one target. A
/// mythic version's muted line names its label before the book (`Mythic · Mythic Adventures`),
/// whatever the list's rule for showing a book.
fn feat_row(
    stub: &EntryStub,
    source: Option<String>,
    show_source: bool,
    info: ChainMarks,
) -> impl IntoView + use<> {
    let id = stub.id.clone();
    let mark_id = id.clone();
    let mark = move || {
        info.with(|loaded| {
            loaded
                .as_ref()
                .and_then(Option::as_ref)
                .and_then(|info| info.marks.get(&mark_id).copied())
                .map(chain_mark)
        })
    };
    let detail = move || {
        info.with(|loaded| {
            let loaded = loaded.as_ref().and_then(Option::as_ref)?;
            if let Some(label) = loaded.addons.get(&id) {
                let book = source.clone().unwrap_or_else(|| "Other".to_string());
                return Some(format!("{} · {book}", addon_name(label)));
            }
            show_source.then(|| source.clone()).flatten()
        })
    };
    view! {
        <li>
            <a href=Route::entry(stub.id.clone()).href()>{stub.title.clone()}{mark}</a>
            {move || detail().map(|text| view! { <span class="muted row-source">{text}</span> })}
        </li>
    }
}

/// The class-option group index while it loads. The outer `Option` is "not loaded yet"; the
/// inner one is "this library has no `option-groups.json`".
type OptionGroups = ReadSignal<Option<Option<Rc<OptionGroupIndex>>>, LocalStorage>;

/// A class-option row: the title, then `{book}` while the group filter is on, or
/// `{Group singular} · {book}` in the whole list.
fn class_option_row(
    stub: &EntryStub,
    book: Option<String>,
    groups: OptionGroups,
    group_selected: bool,
) -> impl IntoView + use<> {
    let id = stub.id.clone();
    let detail_id = id.clone();
    let detail = move || {
        let book = book.clone().unwrap_or_else(|| "Other".to_string());
        if group_selected {
            return book;
        }
        groups.with(|loaded| {
            let singular = loaded
                .as_ref()
                .and_then(|index| index.as_ref())
                .and_then(|index| class_options::group_of(index, &detail_id))
                .map(|group| group.singular.clone());
            match singular {
                Some(singular) => format!("{singular} · {book}"),
                None => book,
            }
        })
    };
    view! {
        <li>
            <a href=Route::entry(id).href()>{stub.title.clone()}</a>
            <span class="muted row-source">{detail}</span>
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

/// One displayed group of publications: a priority tier and the books in it, in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookGroup {
    pub tier: Tier,
    pub books: Vec<usize>,
}

/// The date of a book when it starts with a four-digit year, else `None`.
fn dated(date: Option<&str>) -> Option<&str> {
    let date = date?;
    (date.len() >= 4 && date.as_bytes()[..4].iter().all(u8::is_ascii_digit)).then_some(date)
}

fn title_order(books: &[BookListEntry], a: usize, b: usize) -> std::cmp::Ordering {
    sort_key(&books[a].title)
        .cmp(&sort_key(&books[b].title))
        .then_with(|| books[a].id.cmp(&books[b].id))
}

/// Groups publications for the one order of Books and Active books: tiers in priority order
/// (empty tiers are left out), and inside a tier the oldest book first. A book with no usable
/// date comes last; ties are alphabetical.
pub fn book_groups(books: &[BookListEntry]) -> Vec<BookGroup> {
    Tier::ALL
        .into_iter()
        .filter_map(|tier| {
            let mut indices: Vec<usize> = (0..books.len())
                .filter(|&index| Tier::from_category(books[index].category.as_deref()) == tier)
                .collect();
            if indices.is_empty() {
                return None;
            }
            indices.sort_by(|&a, &b| {
                let left = dated(books[a].publication_date.as_deref());
                let right = dated(books[b].publication_date.as_deref());
                // ISO dates compare as text. A missing or malformed date sorts after all others.
                left.is_none()
                    .cmp(&right.is_none())
                    .then_with(|| left.cmp(&right))
                    .then_with(|| title_order(books, a, b))
            });
            Some(BookGroup {
                tier,
                books: indices,
            })
        })
        .collect()
}

/// One book row: a single link to the book page, where the contents and the Active checkbox
/// live. The row carries no checkbox; Active books and the book page change the choice. The
/// meta line reads `Aug 2009 · 2,994 entries`, and a book that is not active ends it with the
/// plain label `Inactive`.
fn book_row(row: BookListEntry, library: Library) -> AnyView {
    let meta = fmt::book_meta(row.publication_date.as_deref(), row.count);
    let href = Route::Book {
        source: row.id.clone(),
        chapter: None,
    }
    .href();
    let id = row.id;
    let title = row.title;
    let revision = library.selection_revision();
    let state_id = id.clone();
    let is_active = move || {
        revision.get();
        library.book_active(&state_id)
    };
    view! {
        <li class="book-directory-item" data-book-id=id>
            <a class="book-directory-row book-card" href=href>
                <span class="book-card-text">
                    <span class="cat-name">{title}</span>
                    <span class="cat-blurb">
                        {meta}
                        {move || (!is_active()).then(|| view! {
                            <span class="book-meta-sep" aria-hidden="true">" · "</span>
                            <span class="tag book-inactive">"Inactive"</span>
                        })}
                    </span>
                </span>
                {icons::book_chevron()}
            </a>
        </li>
    }
    .into_any()
}

/// The publication directory: a heading, one live count line, a link to Active books, and one
/// link row per book in the tier groups. There is one order (tier order, oldest first) and no
/// sort control. The checkboxes and the presets live on Active books.
#[component]
pub fn Books() -> impl IntoView {
    set_title("Books");
    let library = use_library();
    // Back from Active books lands here: Done asks for focus on the link it was left from.
    if take_link_focus() {
        focus_after_frame(ACTIVE_LINK_ID);
    }
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
    let total = rows.len();
    // The count line names the preset that the active set matches, or Custom.
    let revision = library.selection_revision();
    let count_library = library.clone();
    let count_line = move || {
        revision.get();
        let sources = count_library.sources();
        let active = sources
            .iter()
            .filter(|source| count_library.book_active(&source.id))
            .count();
        let books = book_presets::books_of(sources);
        let preset = book_presets::matching(&books, |id| count_library.book_active(id));
        book_presets::count_line(active, total, Some(book_presets::name_or_custom(preset)))
    };
    // The rows of a closed group are built when it opens, so a mount builds one group's rows.
    let rows = Shared::new(rows);
    let groups = book_groups(&rows)
        .into_iter()
        .map(|group| {
            let tier = group.tier;
            let ids: Vec<String> = group
                .books
                .iter()
                .map(|&index| rows[index].id.clone())
                .collect();
            let lazy_rows = rows.clone();
            let lazy_library = library.clone();
            let books = group.books;
            let items = move || {
                let rows = &*lazy_rows;
                books
                    .iter()
                    .map(|&index| book_row(rows[index].clone(), lazy_library.clone()))
                    .collect_view()
                    .into_any()
            };
            tier_section(tier, ids, items, library.clone())
        })
        .collect_view();
    view! {
        <div class="wide">
            <h1>"Books"</h1>
            <p class="muted books-count" aria-live="polite">{count_line}</p>
            <div class="filter-row">
                <a class="page-link" id=ACTIVE_LINK_ID href=Route::BooksActive.href()
                    on:click=|_| note_opened_from_books()>
                    <span>"Active books"</span>
                    {icons::chevron_right_large()}
                </a>
            </div>
            {groups}
        </div>
    }
}

/// The count at the right of a group heading: `31 books` when every book in it is active,
/// otherwise `28 of 31 active` (including `0 of 151 active`).
pub fn group_count_text(active: usize, total: usize) -> String {
    if active == total {
        format!(
            "{} {}",
            fmt::count(total),
            if total == 1 { "book" } else { "books" }
        )
    } else {
        format!("{} of {} active", fmt::count(active), fmt::count(total))
    }
}

/// One tier: an `h2` that holds a disclosure button (a chevron, the name, and a muted count of
/// active books), then the rows. The button is the only control; there is no group action of any
/// kind, and the presets on Active books are the only bulk change. Rulebooks start open and the
/// other groups closed; the reader's choice is kept for the session ([`remember_group`]).
///
/// A closed group holds an empty list: `items` runs when the group first opens, so a mount builds
/// the rows of the open groups only. The rows stay after the group closes again, so reopening
/// costs nothing, and a group that starts open still has its rows on the first paint (Back to
/// Books restores the scroll position of a group the reader left open).
pub(crate) fn tier_section(
    tier: Tier,
    ids: Vec<String>,
    items: impl Fn() -> AnyView + Send + 'static,
    library: Library,
) -> AnyView {
    let label = tier.label();
    let total = ids.len();
    let slug = label.to_ascii_lowercase().replace(' ', "-");
    let heading_id = format!("book-tier-{slug}");
    let list_id = format!("book-tier-list-{slug}");
    let labelled_by = heading_id.clone();
    let controls = list_id.clone();
    let starts_open = group_is_open(tier);
    let open = RwSignal::new(starts_open);
    let built = RwSignal::new(starts_open);
    let revision = library.selection_revision();
    let active_ids = ids;
    let count_text = move || {
        revision.get();
        let active = active_ids
            .iter()
            .filter(|id| library.book_active(id))
            .count();
        group_count_text(active, total)
    };
    view! {
        <section class="book-tier" aria-labelledby=labelled_by data-tier=label>
            <h2 class="book-group-head" id=heading_id>
                <button type="button" class="tier-toggle" aria-controls=controls
                    aria-expanded=move || if open.get() { "true" } else { "false" }
                    on:click=move |_| {
                        let next = !open.get_untracked();
                        open.set(next);
                        if next {
                            built.set(true);
                        }
                        remember_group(tier, next);
                    }>
                    {icons::tier_chevron()}
                    <span class="tier-name">{label}</span>
                    <span class="tier-active">{count_text}</span>
                </button>
            </h2>
            <ul class="book-directory" id=list_id hidden=move || !open.get()>
                {move || built.get().then(&items)}
            </ul>
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
                                <summary>{icons::chevron_right("disclosure-chevron")}{format!("{} more", fmt::count(n))}</summary>
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
    source: Option<String>,
    values: &[(String, u32)],
) -> Vec<MenuOption> {
    values
        .iter()
        .map(|(value, count)| MenuOption {
            value: value.clone(),
            label: facet_label(spec.field, value),
            count: *count,
            href: Route::browse_kind_facets(kind, facets_with(current, spec.field, Some(value)))
                .with_source(source.clone())
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
    source: Option<String>,
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
    let source_for_text = source.clone();
    let text = Signal::derive(move || {
        values.with(|loaded| match loaded {
            None => None,
            Some(Err(_)) => Some(trigger_text(&spec, None)),
            Some(Ok(values)) => {
                let options = menu_options(
                    kind,
                    &spec,
                    &applied_for_text,
                    source_for_text.clone(),
                    values,
                );
                Some(trigger_text(
                    &spec,
                    current_option(&options, value_for_text.as_deref()),
                ))
            }
        })
    });
    let value_for_narrowed = value.clone();
    let applied_for_narrowed = applied.to_vec();
    let source_for_narrowed = source.clone();
    let narrowed = Signal::derive(move || {
        values.with(|loaded| match loaded {
            Some(Ok(values)) => {
                let options = menu_options(
                    kind,
                    &spec,
                    &applied_for_narrowed,
                    source_for_narrowed.clone(),
                    values,
                );
                current_option(&options, value_for_narrowed.as_deref()).is_some()
            }
            _ => false,
        })
    });
    let applied_for_panel = applied.to_vec();
    let source_for_panel = source.clone();
    let all_href = Route::browse_kind_facets(kind, facets_with(applied, spec.field, None))
        .with_source(source.clone())
        .href();
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
                let options = menu_options(
                    kind,
                    &spec,
                    &applied_for_panel,
                    source_for_panel.clone(),
                    values,
                );
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

/// The class-option group menu. Its choices and labels come from `option-groups.json`, because
/// the search index has no `group` field: a slug such as `arcane-discovery` does not print as
/// `Arcane discoveries`. A group with no option in the active books is left out, except the one
/// the URL selects, so the button can still name it.
fn class_option_menu(
    library: &Library,
    kind: EntryKind,
    spec: MenuSpec,
    applied: &[(String, String)],
    source: Option<String>,
    focus: bool,
) -> (AnyView, AnyView) {
    let wanted = applied
        .iter()
        .find(|(field, _)| field == spec.field)
        .map(|(_, value)| value.clone());
    let applied_for_values = applied.to_vec();
    let source_for_values = source.clone();
    let wanted_for_values = wanted.clone();
    let values: ReadSignal<Option<Result<Vec<MenuOption>, LibraryError>>, LocalStorage> = {
        let library = library.clone();
        local_resource(async move {
            let Some(index) = library.option_groups().await? else {
                return Ok(Vec::new());
            };
            let discovery = library.discovery();
            let mut options: Vec<MenuOption> = index
                .groups
                .iter()
                .filter_map(|group| {
                    let count = group
                        .entries
                        .iter()
                        .filter(|id| discovery.is_eligible(id.as_str()))
                        .count();
                    if count == 0 && wanted_for_values.as_deref() != Some(group.slug.as_str()) {
                        return None;
                    }
                    Some(MenuOption {
                        value: group.slug.clone(),
                        label: group.label.clone(),
                        count: u32::try_from(count).unwrap_or(u32::MAX),
                        href: Route::browse_kind_facets(
                            kind,
                            facets_with(&applied_for_values, spec.field, Some(group.slug.as_str())),
                        )
                        .with_source(source_for_values.clone())
                        .href(),
                    })
                })
                .collect();
            options.sort_by_key(|option| option.label.to_lowercase());
            Ok(options)
        })
    };
    let state = MenuState::new();
    let value_for_text = wanted.clone();
    let text = Signal::derive(move || {
        values.with(|loaded| match loaded {
            None => None,
            Some(Err(_)) => Some(trigger_text(&spec, None)),
            Some(Ok(options)) => Some(trigger_text(
                &spec,
                current_option(options, value_for_text.as_deref()),
            )),
        })
    });
    let value_for_narrowed = wanted.clone();
    let narrowed = Signal::derive(move || {
        values.with(|loaded| match loaded {
            Some(Ok(options)) => current_option(options, value_for_narrowed.as_deref()).is_some(),
            _ => false,
        })
    });
    let all_href = Route::browse_kind_facets(kind, facets_with(applied, spec.field, None))
        .with_source(source)
        .href();
    let button = move || {
        values.with(|loaded| match loaded {
            Some(Err(_)) => None,
            Some(Ok(options)) if options.is_empty() => None,
            _ => Some(view! { <FilterMenuButton spec state text narrowed focus /> }),
        })
    };
    let panel = move || {
        values.with(|loaded| {
            match loaded {
            Some(Ok(options)) if !options.is_empty() => Some(
                view! {
                    <FilterMenuPanel spec state options=options.clone() current=wanted.clone()
                        all_href=all_href.clone() />
                }
                .into_any(),
            ),
            Some(Err(_)) => Some(
                view! { <p class="hint">"The filters could not be loaded. Search still works."</p> }
                    .into_any(),
            ),
            _ => None,
        }
        })
    };
    (
        view! { <div class="filter-menu">{button}</div> }.into_any(),
        panel.into_any(),
    )
}

/// The filter row of a list page: the filter menus on the left, the related pages as plain
/// links on the right, and each menu's panel under the row. Races keep their pills (the old
/// lens) until the library has group data (#196).
#[component]
fn Lenses(
    kind: EntryKind,
    /// The facets that narrow the list now.
    applied: Vec<(String, String)>,
    /// The book the list is limited to, kept by every filter link.
    source: Option<String>,
    /// The field of the menu whose choice just opened this page, if any.
    focus: Option<&'static str>,
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
                let menu_focus = focus == Some(spec.field);
                let (button, panel) = if kind == EntryKind::ClassOption
                    && spec.field == categories::CLASS_OPTION_GROUP_MENU.field
                {
                    class_option_menu(&library, kind, spec, &applied, source.clone(), menu_focus)
                } else {
                    filter_menu(&library, kind, spec, &applied, source.clone(), menu_focus)
                };
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
                        {icons::chevron_right_large()}
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

thread_local! {
    /// Set when the book pill is removed, so the rebuilt list puts focus on its first filter
    /// control. The page is rebuilt for the new route, so the request has to outlive it.
    static FOCUS_FILTER_NEXT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn request_filter_focus() {
    FOCUS_FILTER_NEXT.with(|flag| flag.set(true));
}

/// Reads and clears the request left by the book pill. Called once while a list page is built.
fn take_filter_focus() -> bool {
    FOCUS_FILTER_NEXT.with(|flag| flag.replace(false))
}

fn focus_element(element: &web_sys::Element) {
    let Some(element) = element.clone().dyn_into::<web_sys::HtmlElement>().ok() else {
        return;
    };
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    let _ = element.focus_with_options(&options);
}

/// Focuses the first filter control, or the H1 when the page has none. A filter button is
/// disabled until its choices load, so wait while one is present, then fall back to the H1.
fn focus_first_filter(tries: u32) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if let Some(control) = document
        .query_selector(".filter-btn:not([disabled]), .lens-chip")
        .ok()
        .flatten()
    {
        focus_element(&control);
        return;
    }
    let pending = document
        .query_selector(".filter-btn")
        .ok()
        .flatten()
        .is_some();
    if pending && tries > 0 {
        request_animation_frame(move || focus_first_filter(tries - 1));
        return;
    }
    // The shell focuses <main> one frame after the route change. Wait it out, as the Active
    // books H1 does, so the heading keeps focus.
    focus_heading_later();
}

fn focus_heading_later() {
    request_animation_frame(|| {
        request_animation_frame(|| {
            let Some(heading) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.query_selector("main h1").ok().flatten())
            else {
                return;
            };
            focus_element(&heading);
        });
    });
}

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
    source: Option<String>,
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
                .with_source(source.clone())
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
    source: Option<String>,
    marks: Option<ChainMarks>,
    groups: Vec<(char, Vec<usize>)>,
    stubs: Vec<EntryStub>,
    source_titles: Vec<Option<String>>,
    show_source: bool,
    /// The class-option groups, so a row can name its group when the list is not filtered.
    /// `None` for every other kind.
    class_option_groups: Option<OptionGroups>,
    /// True while a group filter is on: each option row then shows only its book.
    class_option_group_selected: bool,
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
                    let source = source_titles[i].clone();
                    rendered.push(match (marks, class_option_groups) {
                        (Some(marks), _) => feat_row(stub, source, show_source, marks).into_any(),
                        (None, Some(groups)) => {
                            class_option_row(stub, source, groups, class_option_group_selected)
                                .into_any()
                        }
                        (None, None) => {
                            entry_row(stub, show_source.then_some(source).flatten()).into_any()
                        }
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
                    .with_source(source.clone())
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
        source: None,
        source_title: None,
    };
    format!("No {} in your active books.", scope.label())
}

/// The count line of a book-filtered list. It counts the book's set and never repeats the book
/// name: `626 spells` for the book alone, `48 of 626 spells` once a facet narrows that set.
pub fn book_count_line(
    kind: EntryKind,
    shown: usize,
    total: usize,
    facet_narrowed: bool,
) -> String {
    if facet_narrowed {
        narrowed_count_line(kind, shown, total)
    } else {
        kind_count_line(kind, total)
    }
}

/// What a book-filtered list says when the book has none of the kind: `No spells in Core
/// Rulebook.`
pub fn book_empty_line(kind: EntryKind, book: &str) -> String {
    format!("No {} in {book}.", kind.label().to_lowercase())
}

/// The way out of a book's empty kind list: `Browse all spells`.
pub fn browse_all_label(kind: EntryKind) -> String {
    format!("Browse all {}", kind.label().to_lowercase())
}

/// The notice for a source id that the library does not publish. It is never a missing page, so
/// a bookmark survives a renamed book.
pub fn unknown_source_notice(kind: EntryKind, source: &str) -> String {
    format!(
        "Unknown book \"{source}\". Showing all {}.",
        kind.label().to_lowercase()
    )
}

/// A kind's A to Z list. The URL may narrow it by a facet (`?facet=feat_type:Combat`) and by a
/// publication (`?source=core-rulebook`); the index then names the entries that carry the value,
/// and the list, its letters and its count all follow. Without a facet the list is built at
/// once. An unknown source is the plain list with a notice, never a missing page.
#[component]
pub fn BrowseKind(
    kind: EntryKind,
    letter: Option<char>,
    #[prop(optional)] facets: Vec<(String, String)>,
    source: Option<String>,
) -> impl IntoView {
    set_title(kind.label());
    let library = use_library();
    // A choice in a filter menu asks the page it opens to focus the menu's button. Removing the
    // book pill asks the same page to focus its first filter control.
    let focus_menu = take_focus_request();
    let focus_first = take_filter_focus();
    let applied = applied_facets(kind, &facets);
    let known = source.as_deref().and_then(|id| {
        library
            .source(id)
            .map(|source| (id.to_string(), source.title.clone()))
    });
    let unknown = source
        .as_deref()
        .filter(|id| library.source(id).is_none())
        .map(str::to_owned);
    let source_id = known.as_ref().map(|(id, _)| id.clone());
    let source_title = known.as_ref().map(|(_, title)| title.clone());
    // A class page names itself when it opens a group that serves several classes. The value
    // is a scope only: the group's options already apply to every class it serves.
    let class_scope = (kind == EntryKind::ClassOption)
        .then(|| {
            facets
                .iter()
                .find(|(field, _)| field == "class_scope")
                .map(|(_, value)| value.clone())
        })
        .flatten();
    // A book's set comes from the whole catalog, so a disabled book still lists what it holds.
    let all = Shared::new(match &source_id {
        Some(source) => sorted_of_kind_in_source(&library, kind, source),
        None => sorted_of_kind(&library, kind),
    });
    if source_id.is_none()
        && let Some(field) = categories::grouped_by(kind)
    {
        if focus_first {
            focus_heading_later();
        }
        return view! { <GroupedKind kind field all /> }.into_any();
    }
    if applied.is_empty() {
        let stubs = all.to_vec();
        return view! {
            <BrowseList kind letter applied=applied source=source_id source_title
                unknown stubs total=all.len() focus_menu focus_first
                class_scope=class_scope.clone().unwrap_or_default() />
        }
        .into_any();
    }

    let ids = {
        let (library, applied, source) = (library.clone(), applied.clone(), source_id.clone());
        local_resource(async move {
            // Several facets narrow together: a feat has to carry every value. A book filter
            // reads the whole index, because the book may be disabled and its entries are then
            // not eligible; a plain list keeps the active-book snapshot.
            let index = if source.is_some() {
                Some(library.search_index().await?)
            } else {
                None
            };
            let discovery = library.discovery();
            // A class option's group comes from `option-groups.json`, not the search index.
            // A book filter keeps the book's whole set, so it reads the catalog, not the
            // active-book snapshot.
            if kind == EntryKind::ClassOption {
                let groups = library.option_groups().await?.unwrap_or_default();
                let mut ids: Option<HashSet<String>> = None;
                for (field, value) in &applied {
                    if field != "group" {
                        continue;
                    }
                    let found = class_options::group_ids(&groups, value, |id| match &source {
                        Some(source) => library
                            .stub(id)
                            .is_some_and(|stub| library.stub_is_in_source(stub, source)),
                        None => discovery.is_eligible(id.as_str()),
                    });
                    ids = Some(match ids {
                        None => found,
                        Some(earlier) => earlier.intersection(&found).cloned().collect(),
                    });
                }
                return Ok(ids.unwrap_or_default());
            }
            let mut ids: Option<HashSet<String>> = None;
            for (field, value) in &applied {
                let found: HashSet<String> = match &index {
                    Some(index) => index
                        .facet_ids_eligible(field, value, |_| true)
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                    None => discovery.facet_entry_ids(field, value).await?,
                };
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
                    <h1 tabindex="-1">{kind.label()}</h1>
                    <p class="muted list-count" role="status" aria-live="polite">"Loading…"</p>
                </div>
            }
            .into_any(),
            Some(Err(error)) => view! {
                <div class="wide list-page">
                    <h1 tabindex="-1">{kind.label()}</h1>
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
                    <BrowseList kind letter applied=applied.clone() source=source_id.clone()
                        source_title=source_title.clone() unknown=unknown.clone() stubs
                        total=all.len() focus_menu focus_first
                        class_scope=class_scope.clone().unwrap_or_default() />
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
    let focus_menu = None;
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
                    <h1 tabindex="-1">{kind.label()}</h1>
                    <p class="muted list-count" role="status" aria-live="polite">"Loading…"</p>
                </div>
            }
            .into_any(),
            Some(Err(_)) => {
                let stubs = all.to_vec();
                view! {
                    <BrowseList kind letter=None applied=Vec::new() source=None source_title=None
                        unknown=None stubs total focus_menu focus_first=false />
                }
                .into_any()
            }
            Some(Ok(found)) => {
                let sections = sections_by_group(field, &all, found);
                let stubs = all.to_vec();
                view! {
                    <BrowseList kind letter=None applied=Vec::new() source=None source_title=None
                        unknown=None stubs total focus_menu focus_first=false sections />
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
    /// The book the list is limited to, if any.
    source: Option<String>,
    /// The book's display name, so the pill can name it.
    source_title: Option<String>,
    /// A source id the library does not publish: the list stays whole and says so.
    unknown: Option<String>,
    stubs: Vec<EntryStub>,
    /// The size of the whole kind before a facet filter. With a book, the book's size.
    total: usize,
    /// The field of the menu whose choice opened this page, if any.
    focus_menu: Option<&'static str>,
    /// True when the book pill was just removed: focus the first filter control.
    #[prop(optional)]
    focus_first: bool,
    /// The class that opened a shared option group, named by a scope pill. It does not narrow
    /// the group's options, which are the same for every class the group serves. Empty when the
    /// list has no class context.
    #[prop(optional)]
    class_scope: String,
    /// A short list under headings, in order: `(heading, entries)`. Without it the list is one
    /// A to Z list.
    #[prop(optional)]
    sections: Option<Vec<Section>>,
) -> impl IntoView {
    let library = use_library();
    let count = stubs.len();
    let narrowed = !applied.is_empty();
    let book = source_title.is_some();
    let multi_source = stubs.iter().any(|s| s.source != stubs[0].source);
    let titles: Vec<&str> = stubs.iter().map(|s| s.title.as_str()).collect();
    let groups = group_by_letter(&titles);
    let requested = letter.filter(|letter| groups.iter().any(|(group, _)| group == letter));
    let selected = requested.or_else(|| groups.first().map(|(letter, _)| *letter));
    // A letter the list does not have still renders the first group; make the URL match it.
    if letter.is_some() && requested.is_none() {
        let route = Route::browse_kind_facets(kind, applied.clone()).with_source(source.clone());
        Effect::new(move |_| replace(&route));
    }
    // Removing the book pill rebuilds this page; take focus after the route frame has moved it.
    if focus_first {
        request_animation_frame(|| focus_first_filter(120));
    }
    let letters: Vec<char> = groups.iter().map(|(letter, _)| *letter).collect();
    let jump = (count > GROUP_THRESHOLD).then(|| {
        view! {
            <LetterNav kind facets=applied.clone() source=source.clone() letters=letters
                selected=selected />
        }
    });

    // Feats that open a long chain carry a marker. The numbers come from the relations graph,
    // read once for the page; the rows do not wait for them.
    let marks: Option<ChainMarks> =
        (kind == EntryKind::Feat && library.has_relations()).then(|| {
            let library = library.clone();
            local_resource(async move {
                // The labels read the whole graph: an add-on's row names its own label even
                // while its base's book is inactive. The markers read the discovery projection,
                // so a disabled feat never inflates a visible row's count.
                let all = library.relations().await.ok().flatten()?;
                let discovery = library.discovery();
                let scoped = all.filtered(|id| discovery.is_eligible(id.as_str()));
                Some(Shared::new(FeatRowInfo {
                    marks: scoped.long_chains(CHAIN_MARK_MIN),
                    addons: all.addon_labels(),
                }))
            })
        });

    // Class options carry their group on the entry, but the list page reads the small derived
    // index so a row can name the group without opening a shard.
    let class_option_groups: Option<OptionGroups> = (kind == EntryKind::ClassOption).then(|| {
        let library = library.clone();
        local_resource(async move { library.option_groups().await.ok().flatten() })
    });
    let class_option_group_selected = applied.iter().any(|(field, _)| field == "group");

    let sectioned = sections.is_some();
    let body = if count == 0
        && total == 0
        && let Some(title) = &source_title
    {
        // The book holds none of this kind.
        view! {
            <p>{book_empty_line(kind, title)}</p>
            <p class="empty-actions">
                <a class="count-action" href=Route::browse_kind(kind).href()>
                    {browse_all_label(kind)}
                </a>
            </p>
        }
        .into_any()
    } else if count == 0 {
        // The value is not in the active books. Say so, and offer the two ways out.
        view! {
            <p>{empty_filter_line(kind, &applied)}</p>
            <p class="empty-actions">
                <a class="count-action" href=Route::browse_kind(kind).href()>
                    {format!("Show all {}", kind.label().to_lowercase())}
                </a>
                " · "
                <a class="count-action" href=Route::BooksActive.href()>"Active books"</a>
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
                        <AzList kind facets=Vec::new() source=None marks=None groups=Vec::new()
                            stubs=group source_titles show_source=multi_source
                            class_option_groups=None class_option_group_selected=false
                            selected=None focus=false />
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
            <AzList kind facets=applied.clone() source=source.clone() marks groups
                stubs=stubs.clone() source_titles show_source=multi_source
                class_option_groups=class_option_groups
                class_option_group_selected=class_option_group_selected
                selected=selected
                focus=requested.is_some() && focus_menu.is_none() && !focus_first />
        }
        .into_any()
    };

    let count_line = if book {
        book_count_line(kind, count, total, narrowed)
    } else if narrowed {
        narrowed_count_line(kind, count, total)
    } else {
        kind_count_line(kind, total)
    };
    // A facet widens back to the book's whole kind when a book filter is on. Without a book it
    // widens to the whole kind, as before.
    let show_all_route = Route::browse_kind(kind).with_source(source.clone());
    let show_all = narrowed.then(|| {
        view! {
            " · "
            <a class="count-action" href=show_all_route.href()
                aria-label=format!("Show all {}", kind.label().to_lowercase())>"Show all"</a>
        }
    });
    let remove_route = Route::BrowseKind {
        kind,
        letter,
        facets: applied.clone(),
        source: None,
    };
    let pill = source_title.clone().map(|title| {
        let route = remove_route.clone();
        view! {
            <div class="book-filter">
                <FilterPill label=title.clone() name=format!("Remove filter: {title}")
                    on_remove=Callback::new(move |()| {
                        request_filter_focus();
                        navigate(&route);
                    }) />
            </div>
        }
    });
    // A shared class-option group opened from a class page names that class. Removing the pill
    // keeps the group and drops the class context.
    let class_pill = EntryId::parse(&class_scope).ok().map(|class| {
        let title = library
            .stub(&class)
            .map_or_else(|| class.to_string(), |stub| stub.title.clone());
        let route = Route::BrowseKind {
            kind,
            letter,
            facets: applied.clone(),
            source: source.clone(),
        };
        view! {
            <div class="book-filter">
                <FilterPill label=title.clone() name=format!("Remove filter: {title}")
                    on_remove=Callback::new(move |()| {
                        request_filter_focus();
                        navigate(&route);
                    }) />
            </div>
        }
    });
    let notice = unknown
        .as_deref()
        .map(|id| view! { <p class="notice" role="status">{unknown_source_notice(kind, id)}</p> });

    view! {
        <div class="wide list-page">
        <h1 tabindex="-1">{kind.label()}</h1>
        {notice}
        {pill}
        {class_pill}
        <p class="muted list-count" role="status" aria-live="polite">{count_line}{show_all}</p>
        <Lenses kind applied=applied.clone() source=source.clone() focus=focus_menu />
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
                        {icons::chevron_right("disclosure-chevron")}
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
        assert_eq!(
            empty_filter_line(
                EntryKind::Creature,
                &[facet("creature_type", "dragon"), facet("cr", "1/8")]
            ),
            "No CR 1/8 Dragon creatures in your active books."
        );
        assert_eq!(
            empty_filter_line(EntryKind::Spell, &[facet("spell_level", "5")]),
            "No Level 5 spells in your active books."
        );
    }

    #[test]
    fn a_book_count_line_counts_the_book_and_never_repeats_its_name() {
        assert_eq!(
            book_count_line(EntryKind::Spell, 626, 626, false),
            "626 spells"
        );
        assert_eq!(
            book_count_line(EntryKind::Spell, 48, 626, true),
            "48 of 626 spells"
        );
        assert_eq!(book_count_line(EntryKind::Spell, 1, 1, false), "1 spell");
        assert_eq!(book_count_line(EntryKind::Spell, 0, 0, false), "0 spells");
        assert_eq!(
            book_empty_line(EntryKind::Spell, "Core Rulebook"),
            "No spells in Core Rulebook."
        );
        assert_eq!(
            book_empty_line(EntryKind::PrestigeClass, "Core Rulebook"),
            "No prestige classes in Core Rulebook."
        );
        assert_eq!(browse_all_label(EntryKind::Spell), "Browse all spells");
        assert_eq!(
            unknown_source_notice(EntryKind::Spell, "no-such-book"),
            "Unknown book \"no-such-book\". Showing all spells."
        );
        assert_eq!(
            unknown_source_notice(EntryKind::PrestigeClass, "gone"),
            "Unknown book \"gone\". Showing all prestige classes."
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
            None,
            &values,
        );
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Combat");
        assert_eq!(options[0].count, 1324);
        // No `l=`: choosing a value resets the letter.
        assert_eq!(options[0].href, "#/browse/kind/feat?facet=feat_type:Combat");
        // A book filter survives a choice.
        let in_book = menu_options(
            EntryKind::Feat,
            &crate::views::categories::FEAT_TYPE_MENU,
            &[],
            Some("core-rulebook".into()),
            &values,
        );
        assert_eq!(
            in_book[0].href,
            "#/browse/kind/feat?source=core-rulebook&facet=feat_type:Combat"
        );
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

    #[test]
    fn a_mythic_version_sits_directly_after_its_base_in_a_feat_list() {
        let mut titles = ["Acrobatic Spellcaster", "Acrobatic, Mythic", "Acrobatic"];
        titles.sort_by_key(|title| feat_sort_key(title));
        assert_eq!(
            titles,
            ["Acrobatic", "Acrobatic, Mythic", "Acrobatic Spellcaster"]
        );
        assert_eq!(feat_sort_key("Dodge"), ("dodge".to_string(), 0));
        assert_eq!(feat_sort_key("Dodge, Mythic"), ("dodge".to_string(), 1));
        // A title that only looks like a suffix keeps its own place.
        assert_eq!(feat_sort_key(", Mythic"), (", mythic".to_string(), 0));
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
        groups.iter().map(|group| group.tier).collect()
    }

    #[test]
    fn groups_follow_the_priority_tiers_and_omit_empty_ones() {
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
        let groups = book_groups(&books);
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
        assert_eq!(
            groups[0].books,
            vec![3, 1],
            "undated books are alphabetical inside a tier"
        );
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
    fn inside_a_tier_the_oldest_book_comes_first_undated_last_and_ties_are_alphabetical() {
        let books = vec![
            book("a", "Alpha", Some("RPG"), Some("2009-08-01")),
            book("b", "Beta", Some("RPG"), Some("2014-08-14")),
            book("c", "Aardvark", Some("RPG"), Some("2014-08-14")),
            book("d", "Dated", Some("RPG"), Some("2014-07-01")),
            book("u", "Unknown", Some("RPG"), None),
            book("bad", "Bad", Some("RPG"), Some("not-a-date")),
            book("m", "Module", Some("Module"), Some("2020-01-01")),
            book("e", "Early module", Some("Module"), Some("2008-03-01")),
        ];
        let groups = book_groups(&books);
        assert_eq!(tiers_of(&groups), [Tier::Rulebooks, Tier::Adventures]);
        assert_eq!(
            groups[0].books,
            vec![0, 3, 2, 1, 5, 4],
            "oldest date first, the shared date by title, then the undated by title"
        );
        assert_eq!(groups[1].books, vec![7, 6], "the order holds in every tier");
    }

    #[test]
    fn a_group_counts_every_book_only_when_all_are_active() {
        assert_eq!(group_count_text(31, 31), "31 books");
        assert_eq!(group_count_text(1, 1), "1 book");
        assert_eq!(group_count_text(28, 31), "28 of 31 active");
        assert_eq!(group_count_text(0, 151), "0 of 151 active");
        assert_eq!(group_count_text(1_000, 1_200), "1,000 of 1,200 active");
        assert_eq!(group_count_text(0, 0), "0 books");
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
