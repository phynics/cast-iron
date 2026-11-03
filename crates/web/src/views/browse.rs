//! Category browsing and the separate publication directory, with category lenses,
//! A to Z lists, the rules hierarchy, and the old publication page for libraries without books.

use crate::js;
use crate::library::{Library, use_library};
use crate::route::{Route, scroll_to_anchor};
use crate::views::categories::{self, Lens};
use crate::views::search::{facet_label, sort_facet_pairs};
use crate::views::source_variants::{SourceVariant, SourceVariants, group_variants};
use crate::views::util::{Shared, group_letter, local_resource, set_title, sort_key};
use cast_iron_schema::{EntryId, EntryKind, EntryStub};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Rows built before the first paint of an A to Z list, and rows added per step after it.
/// A step is one task, so the page stays responsive while the rest of a long list arrives.
const FIRST_ROWS: usize = 240;
const STEP_ROWS: usize = 480;

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

/// How many letter groups to show now so that at least `rows` rows are built, starting after
/// `from` groups. Always makes progress while groups remain.
pub fn groups_for_rows(groups: &[(char, Vec<usize>)], from: usize, rows: usize) -> usize {
    let mut shown = from;
    let mut built = 0;
    while shown < groups.len() && (built < rows || shown == from) {
        built += groups[shown].1.len();
        shown += 1;
    }
    shown
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

fn category_card(kind: EntryKind, n: usize) -> impl IntoView + use<> {
    view! {
        <li>
            <a class="cat" href=Route::BrowseKind(kind).href()>
                <span class="cat-name">{kind.label()}</span>
                <span class="cat-count">{n}</span>
                <span class="cat-blurb">{categories::blurb(kind)}</span>
            </a>
        </li>
    }
}

#[component]
pub fn Browse() -> impl IntoView {
    set_title("Browse");
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
                <ul class="cats cats-quiet">{rows}</ul>
            </section>
        }
    });
    let shortcuts = library.has_spell_lists().then(|| {
        view! {
            <section aria-labelledby="shortcuts">
                <h2 id="shortcuts" class="sub-head">"Shortcuts"</h2>
                <ul class="pill-list">
                    <li><a class="pill" href=Route::Spells.href()>"Spell lists by class and level"</a></li>
                    <li><a class="pill" href=Route::FeatChains.href()>"Feat chains"</a></li>
                </ul>
            </section>
        }
    });
    view! {
        <div class="wide">
            <h1>"Browse"</h1>
            <Show when=move || library.discovery().entries().next().is_none()>
                <p class="notice">"No books are active. "<a href=Route::Books.href()>"Open Books"</a>" to choose your sources."</p>
            </Show>
            <section aria-labelledby="by-kind">
                <h2 id="by-kind" class="sub-head">"Categories"</h2>
                <ul class="cats">{primary}</ul>
            </section>
            {more_section}
            {shortcuts}
        </div>
    }
}

/// The publication directory. Chapter rows reuse `books.json` and link into the existing
/// lazy contents tree, preserving its draft notes and original order.
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
    let revision = library.selection_revision();
    let rows = library
        .sources()
        .iter()
        .map(|source| {
            let id = source.id.clone();
            let title = source.title.clone();
            let count = library.catalog().entries.iter().filter(|stub| {
                library.canonical_source_for_entry(&stub.id).as_deref() == Some(id.as_str())
            }).count();
            let state_library = library.clone();
            let save_library = library.clone();
            let state_id = id.clone();
            let save_id = id.clone();
            let label = format!("Active {title}");
            let is_active = move || { revision.get(); state_library.book_active(&state_id) };
            let badge_active = is_active.clone();
            let href = Route::Book {
                source: id.clone(),
                chapter: None,
            }
            .href();
            let id_for_rows = id.clone();
            let title_for_summary = title.clone();
            let expanded = RwSignal::new(false);
            let on_toggle = move |event: leptos::ev::Event| {
                expanded.set(event_target::<web_sys::HtmlDetailsElement>(&event).open());
            };
            view! {
                <li class="book-directory-item" data-book-id=id>
                    <details class="book-directory-row" on:toggle=on_toggle>
                        <summary class="book-card">
                            <span class="cat-name">{title_for_summary}</span>
                            <span class="cat-count">{count}</span>
                            <span class="cat-blurb">{format!("{count} entries")}{move || (!badge_active()).then_some(" · Disabled")}</span>
                        </summary>
                        <div class="book-directory-contents">
                            <a href=href>{format!("Open {title} contents")}</a>
                            {move || expanded.get().then(|| lookup.with(|state| match state {
                                Some(Ok(Some(books))) => books.book(&id_for_rows).map(|book| {
                                    let chapters = book.chapters.iter().enumerate().map(|(i, chapter)| {
                                        let href = Route::Book { source: id_for_rows.clone(), chapter: Some(vec![i]) }.href();
                                        view! { <li><a href=href>{crate::views::util::section_label(&chapter.title)}</a></li> }
                                    }).collect_view();
                                    view! { <ul class="book-directory-chapters">{chapters}</ul> }.into_any()
                                }).unwrap_or_else(|| view! { <p class="muted">"No ordered contents are available for this book."</p> }.into_any()),
                                Some(Err(error)) => view! { <p class="error" role="alert">{error.to_string()}</p> }.into_any(),
                                _ => view! { <p class="muted" role="status">"Loading chapter list…"</p> }.into_any(),
                            }))}
                        </div>
                    </details>
                    <label class="book-activation">
                        <input type="checkbox" aria-label=label prop:checked=is_active disabled=move || saving.get()
                            on:change=move |event| {
                                if saving.get_untracked() { return; }
                                let input = event_target::<web_sys::HtmlInputElement>(&event);
                                let active = input.checked();
                                // Discovery remains at the saved value while this control is busy.
                                saving.set(true);
                                save_error.set(None);
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
        })
        .collect_view();
    view! {
        <div class="wide">
            <h1>"Books"</h1>
            <section>
                <p class="muted measure">
                    "Browse each book through its sections and contents. "
                    "Each book explains which parts of its order still need checking against the printed contents."
                </p>
                <p class="hint">"Active books appear in search and category browsing. Disabling a book keeps its download and your stars."</p>
                {move || save_error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })}
                <ul class="book-directory">{rows}</ul>
            </section>
        </div>
    }
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
                <a class="chip-link lens-chip" href=route.href()>
                    {facet_label(field, &value)}<small>{n}</small>
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
                                <summary>{format!("{n} more")}</summary>
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

#[component]
fn Lenses(kind: EntryKind) -> impl IntoView {
    let all = categories::lenses(kind);
    if all.is_empty() {
        return ().into_any();
    }
    let library = use_library();
    let mut pages = Vec::new();
    let mut facets = Vec::new();
    for lens in all {
        match lens {
            Lens::Page {
                label,
                blurb,
                route,
            } => {
                // The spell and feat pages need the schema-3 files; an old library has none.
                let usable = match route {
                    Route::Spells => library.has_spell_lists(),
                    Route::FeatChains => library.has_relations(),
                    _ => true,
                };
                if usable {
                    pages.push((label, blurb, route));
                }
            }
            Lens::Facet { field, label } => facets.push((field, label)),
        }
    }
    let pages = pages
        .into_iter()
        .map(|(label, blurb, route)| {
            view! {
                <li>
                    <a class="lens-page" href=route.href()>
                        <span class="lens-page-name">{label}</span>
                        <span class="lens-page-note">{blurb}</span>
                    </a>
                </li>
            }
        })
        .collect_view();
    let facets = facets
        .into_iter()
        .map(|(field, label)| view! { <FacetLens kind field label /> })
        .collect_view();
    view! {
        <div class="lenses">
            <ul class="lens-pages">{pages}</ul>
            {facets}
        </div>
    }
    .into_any()
}

/// An A to Z list that builds its rows in steps. The first screenful is there at once; the rest
/// follows task by task, so a 3,500-row category never blocks input. Every row ends up in the
/// document, which keeps find-in-page working.
#[component]
fn AzList(
    stubs: Vec<EntryStub>,
    source_titles: Vec<Option<String>>,
    show_source: bool,
) -> impl IntoView {
    let titles: Vec<&str> = stubs.iter().map(|s| s.title.as_str()).collect();
    let groups = Shared::new(group_by_letter(&titles));
    let stubs = Shared::new(stubs);
    let source_titles = Shared::new(source_titles);
    // Group only for presentation, after the caller has selected active catalog stubs. The
    // letter index still counts source records, so incremental loading thresholds stay stable.
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
    let shown = RwSignal::new(groups_for_rows(&groups, 0, FIRST_ROWS));

    {
        let groups = groups.clone();
        spawn_local(async move {
            loop {
                js::sleep(0).await;
                let Some(current) = shown.try_get_untracked() else {
                    return;
                };
                if current >= groups.len() {
                    return;
                }
                shown.set(groups_for_rows(&groups, current, STEP_ROWS));
            }
        });
    }

    let jump = groups
        .iter()
        .enumerate()
        .map(|(position, (letter, _))| {
            let id = letter_id(*letter);
            let label = letter.to_string();
            view! {
                <li>
                    <button
                        type="button"
                        class="letter-btn"
                        aria-label=format!("Jump to {label}")
                        on:click=move |_| {
                            // The group may not be built yet; build up to it, then scroll.
                            if shown.get_untracked() <= position {
                                shown.set(position + 1);
                            }
                            let id = id.clone();
                            request_animation_frame(move || { scroll_to_anchor(&id); });
                        }
                    >{label.clone()}</button>
                </li>
            }
        })
        .collect_view();

    let group_rows = {
        let groups = groups.clone();
        let stubs = stubs.clone();
        let source_titles = source_titles.clone();
        let presentation_groups = presentation_groups.clone();
        let variant_group_by_id = variant_group_by_id.clone();
        let entry_index_by_id = entry_index_by_id.clone();
        move |position: usize| {
            let (letter, indices) = &groups[position];
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
                    rendered.push(
                        entry_row(
                            stub,
                            show_source.then(|| source_titles[i].clone()).flatten(),
                        )
                        .into_any(),
                    );
                }
            }
            let rows = rendered;
            view! {
                <section class="letter-group" id=letter_id(*letter)>
                    <h2 class="letter-head">{letter.to_string()}</h2>
                    <ul class="entry-list">{rows}</ul>
                </section>
            }
        }
    };
    view! {
        <nav class="letter-nav" aria-label="Jump to letter"><ul>{jump}</ul></nav>
        <For each=move || 0..shown.get() key=|position| *position children=group_rows />
    }
}

#[component]
pub fn BrowseKind(kind: EntryKind) -> impl IntoView {
    set_title(kind.label());
    let library = use_library();
    let stubs = sorted_of_kind(&library, kind);
    let count = stubs.len();
    let multi_source = stubs.iter().any(|s| s.source != stubs[0].source);

    let body = if kind == EntryKind::Rule {
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
        view! { <AzList stubs=stubs.clone() source_titles show_source=multi_source /> }.into_any()
    };

    let search_href = Route::Search {
        q: String::new(),
        kinds: vec![kind],
        sources: vec![],
        facets: vec![],
    }
    .href();
    view! {
        <div class="wide">
        <nav class="crumbs" aria-label="Breadcrumb">
            <ol><li><a href=Route::Browse.href()>"Browse"</a></li></ol>
        </nav>
        <h1>{kind.label()}</h1>
        <p class="lede-line">{categories::blurb(kind)}</p>
        <p class="muted">
            {format!("{count} entries")}" · "
            <a href=search_href>
                {format!("Search {}", kind.label().to_lowercase())}
            </a>
        </p>
        <Lenses kind />
        <h2 class="sub-head az-head">"All, A to Z"</h2>
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
            <p><a href=Route::Browse.href()>"Back to Browse"</a></p>
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
                        <span class="tile-count">{n}</span>
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
    view! {
        <div class="wide">
        <nav class="crumbs" aria-label="Breadcrumb">
            <ol><li><a href=Route::Browse.href()>"Browse"</a></li></ol>
        </nav>
        <h1>{source.title.clone()}</h1>
        <p class="muted">{format!("{} entries in this library", source.entry_count)}</p>
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

    #[test]
    fn rows_are_built_in_whole_letter_groups_and_always_progress() {
        let groups = vec![('A', vec![0; 300]), ('B', vec![0; 300]), ('C', vec![0; 10])];
        // The first call builds at least the rows asked for, in whole groups.
        assert_eq!(groups_for_rows(&groups, 0, 240), 1);
        assert_eq!(groups_for_rows(&groups, 0, 301), 2);
        // A step builds one group even when it alone is larger than the step.
        assert_eq!(groups_for_rows(&groups, 1, 10), 2);
        assert_eq!(groups_for_rows(&groups, 1, 1000), 3);
        // Past the end it stays at the end.
        assert_eq!(groups_for_rows(&groups, 3, 100), 3);
        assert_eq!(groups_for_rows(&[], 0, 100), 0);
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
