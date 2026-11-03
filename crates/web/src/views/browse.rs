//! Browse: categories and publications, A to Z lists, the rules hierarchy, and publication pages.

use crate::library::{Library, use_library};
use crate::route::{Route, scroll_to_anchor};
use crate::views::util::{group_letter, set_title, sort_key};
use cast_iron_schema::{EntryId, EntryKind, EntryStub};
use leptos::prelude::*;

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
    let mut list: Vec<EntryStub> = library.entries_of_kind(kind).cloned().collect();
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

#[component]
pub fn Browse() -> impl IntoView {
    set_title("Browse");
    let library = use_library();
    let kinds = library
        .kind_counts()
        .into_iter()
        .map(|(kind, n)| {
            view! {
                <li>
                    <a class="tile" href=Route::BrowseKind(kind).href()>
                        <span class="tile-name">{kind.label()}</span>
                        <span class="tile-count">{n}</span>
                    </a>
                </li>
            }
        })
        .collect_view();
    let sources = library
        .sources()
        .iter()
        .map(|s| {
            view! {
                <li>
                    <a class="tile tile-wide" href=Route::BrowseSource(s.id.clone()).href()>
                        <span class="tile-name">{s.title.clone()}</span>
                        <span class="tile-count">{s.entry_count}</span>
                    </a>
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="wide">
        <h1>"Browse"</h1>
        <section aria-labelledby="by-kind">
            <h2 id="by-kind">"By category"</h2>
            <ul class="tiles">{kinds}</ul>
        </section>
        <section aria-labelledby="by-source">
            <h2 id="by-source">"By publication"</h2>
            <ul class="tiles">{sources}</ul>
        </section>
        </div>
    }
}

#[component]
pub fn BrowseKind(kind: EntryKind) -> impl IntoView {
    set_title(kind.label());
    let library = use_library();
    let stubs = sorted_of_kind(&library, kind);
    let count = stubs.len();
    let multi_source = stubs.iter().any(|s| s.source != stubs[0].source);
    let source_of = {
        let library = library.clone();
        move |stub: &EntryStub| {
            multi_source
                .then(|| library.source(&stub.source).map(|s| s.title.clone()))
                .flatten()
        }
    };

    let body = if kind == EntryKind::Rule {
        let items: Vec<(EntryId, Option<EntryId>, String)> = stubs
            .iter()
            .map(|s| (s.id.clone(), s.parent.clone(), s.title.clone()))
            .collect();
        let tree = build_tree(&items);
        view! { <ul class="tree">{tree_view(&tree, &stubs)}</ul> }.into_any()
    } else {
        let titles: Vec<&str> = stubs.iter().map(|s| s.title.as_str()).collect();
        let groups = group_by_letter(&titles);
        let jump = groups
            .iter()
            .map(|(letter, _)| {
                let id = letter_id(*letter);
                let label = letter.to_string();
                view! {
                    <li>
                        <button
                            type="button"
                            class="letter-btn"
                            aria-label=format!("Jump to {label}")
                            on:click=move |_| { scroll_to_anchor(&id); }
                        >{label.clone()}</button>
                    </li>
                }
            })
            .collect_view();
        let sections = groups
            .iter()
            .map(|(letter, indices)| {
                let rows = indices
                    .iter()
                    .map(|&i| entry_row(&stubs[i], source_of(&stubs[i])))
                    .collect_view();
                view! {
                    <section class="letter-group" id=letter_id(*letter)>
                        <h2 class="letter-head">{letter.to_string()}</h2>
                        <ul class="entry-list">{rows}</ul>
                    </section>
                }
            })
            .collect_view();
        view! {
            <nav class="letter-nav" aria-label="Jump to letter"><ul>{jump}</ul></nav>
            {sections}
        }
        .into_any()
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
        <p class="muted">
            {format!("{count} entries")}" · "
            <a href=search_href>
                {format!("Search {}", kind.label().to_lowercase())}
            </a>
        </p>
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
pub fn BrowseSource(id: String) -> impl IntoView {
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
