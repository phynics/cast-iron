//! Feat chains: the feats that start a chain, and the feats that follow from each. Also the
//! tree renderer that the feat entry screen uses for its own chain.

use crate::fmt;
use crate::graph::{ChainHead, ChainNode, Relations};
use crate::library::{Library, use_library};
use crate::route::{self, Route, use_route};
use crate::views::crumbs::{Crumb, Crumbs};
use crate::views::icons;
use crate::views::util::{Shared, local_resource, set_title, sort_key};
use cast_iron_schema::{EntryId, EntryKind};
use leptos::prelude::*;
use std::collections::HashMap;

/// The title of a feat for display, from the catalog; the id when the catalog lacks it.
fn title_of(library: &Library, id: &EntryId) -> String {
    library
        .stub(id)
        .map_or_else(|| id.to_string(), |s| s.title.clone())
}

/// A chain as nested lists. The root is the feat that the walk started from and is not repeated
/// by the caller.
pub fn chain_tree(library: &Library, node: &ChainNode, show_root: bool) -> AnyView {
    let children = (!node.next.is_empty()).then(|| {
        let items = node
            .next
            .iter()
            .map(|child| chain_tree(library, child, true))
            .collect_view();
        view! { <ul>{items}</ul> }
    });
    let cut = node.cut.then(|| {
        view! { <p class="chain-cut muted">"The chain goes on beyond this point."</p> }
    });
    if !show_root {
        return view! { {children}{cut} }.into_any();
    }
    let href = Route::entry(node.id.clone()).href();
    let title = title_of(library, &node.id);
    let cycle = node
        .cycle
        .then(|| view! { <span class="tag">"repeats"</span> });
    view! {
        <li>
            <a href=href>{title}</a>
            {cycle}
            {children}
            {cut}
        </li>
    }
    .into_any()
}

/// `#/feats/chains`.
#[component]
pub fn FeatChains() -> impl IntoView {
    set_title("Feat chains");
    let library = use_library();
    let revisions = {
        let library = library.clone();
        let revision = library.selection_revision();
        local_resource(async move {
            let _ = revision.get();
            library.revisions().await.ok().flatten()
        })
    };
    let body = move || {
        let library = library.clone();
        revisions.with(|state| match state {
            None => view! { <p class="status" role="status">"Loading feat chains…"</p> }.into_any(),
            Some(None) => view! {
                <p class="notice">
                    "This library has no feat chains. It was installed before they existed. "
                    "Open Settings and check for a library update."
                </p>
            }
            .into_any(),
            Some(Some(revisions)) => {
                // The effective graph already hides revision rows and removed or merged
                // feats, and walks a revised feat's own prerequisites.
                view! {
                    <HeadList library relations=Shared::from_rc(revisions.graph()) />
                }
                .into_any()
            }
        })
    };
    view! {
        <div class="wide">
            <Crumbs items=vec![Crumb::new("Feats", Route::browse_kind(EntryKind::Feat))] />
            <h1>"Feat chains"</h1>
            <p class="muted measure">
                "These feats have no feat prerequisite, and other feats build on them. "
                "Open one to see everything that follows from it. The links come from the "
                "prerequisite text of each feat; a feat with an either-or list is linked to every feat in it."
            </p>
            {body}
        </div>
    }
}

/// A chain folds when its head has more than this many feats below it. A shorter chain stays
/// fully open, as before.
const FOLD_MORE_THAN: usize = 20;

/// What a folding chain needs to draw a branch: the library for titles, and the number of distinct
/// feats below each feat that has dependents (the number of the head summary and the marker).
#[derive(Clone)]
struct Fold {
    library: Library,
    below: Shared<HashMap<EntryId, usize>>,
}

/// The feats below each feat of `node` that has dependents, once each.
fn count_below(relations: &Relations, node: &ChainNode, out: &mut HashMap<EntryId, usize>) {
    if node.next.is_empty() {
        return;
    }
    if !out.contains_key(&node.id) {
        out.insert(node.id.clone(), relations.descendants(&node.id).len());
    }
    for child in &node.next {
        count_below(relations, child, out);
    }
}

/// The rows of `nodes` for a folding chain: a feat with dependents is a closed disclosure and a
/// feat without is a link.
fn fold_rows(fold: &Fold, nodes: &[ChainNode]) -> AnyView {
    nodes
        .iter()
        .map(|node| fold_row(fold, node))
        .collect_view()
        .into_any()
}

fn fold_row(fold: &Fold, node: &ChainNode) -> AnyView {
    let library = &fold.library;
    let href = Route::entry(node.id.clone()).href();
    let title = title_of(library, &node.id);
    let cycle = node
        .cycle
        .then(|| view! { <span class="tag">"repeats"</span> });
    let cut = node.cut.then(|| {
        view! { <p class="chain-cut muted">"The chain goes on beyond this point."</p> }
    });
    let Some(count) = fold
        .below
        .get(&node.id)
        .copied()
        .filter(|_| !node.next.is_empty())
    else {
        return view! {
            <li>
                <a href=href>{title}</a>
                {cycle}
                {cut}
            </li>
        }
        .into_any();
    };
    // The children are built when the branch first opens, so a closed chain costs one row for
    // each branch that is shown.
    let opened = RwSignal::new(false);
    let next = Shared::new(node.next.clone());
    let children = {
        let fold = fold.clone();
        move || {
            opened.get().then(|| {
                let rows = fold_rows(&fold, &next);
                view! { <ul>{rows}</ul> }
            })
        }
    };
    view! {
        <li class="chain-branch">
            <details
                on:toggle=move |ev| {
                    if event_target::<web_sys::HtmlDetailsElement>(&ev).open() {
                        opened.set(true);
                    }
                }
            >
                <summary>
                    {icons::chevron_right("disclosure-chevron")}
                    <span class="chain-name">{title}</span>
                    <span class="muted chain-count">{builds_on_it(count)}</span>
                </summary>
                <p class="chain-go"><a href=href>"Open the feat"</a></p>
                {children}
                {cut}
            </details>
        </li>
    }
    .into_any()
}

/// `12 feats build on it`.
fn builds_on_it(count: usize) -> String {
    format!(
        "{count} {} build on it",
        if count == 1 { "feat" } else { "feats" }
    )
}

#[component]
fn HeadList(library: Library, relations: Shared<Relations>) -> impl IntoView {
    let mut heads: Vec<(String, ChainHead)> = relations
        .chain_heads()
        .into_iter()
        .map(|h| (title_of(&library, &h.id), h))
        .collect();
    // Most built-upon first; equal counts in name order.
    cast_iron_sort::sort_by(&mut heads, |a, b| {
        b.1.total
            .cmp(&a.1.total)
            .then_with(|| sort_key(&a.0).cmp(&sort_key(&b.0)))
    });
    let count = heads.len();
    // The chain that the address asks for (`?open=`), shown open and scrolled to.
    let address = use_route();
    let wanted = match address.get_untracked() {
        Route::FeatChains { open } => open,
        _ => None,
    };
    let rows = heads
        .into_iter()
        .map(|(title, head)| {
            let is_wanted = wanted.as_ref() == Some(&head.id);
            let opened = RwSignal::new(is_wanted);
            let row = NodeRef::<leptos::html::Li>::new();
            if is_wanted {
                // After the frame that scrolls a new page to its top, so two frames. Back and
                // Forward restore the offset the reader left at, which is inside the chain.
                request_animation_frame(move || {
                    request_animation_frame(move || match route::take_arrival_offset() {
                        Some(y) => route::scroll_to_offset(y),
                        None => {
                            if let Some(row) = row.get_untracked() {
                                row.scroll_into_view();
                            }
                        }
                    });
                });
            }
            let href = Route::entry(head.id.clone()).href();
            let folds = head.total > FOLD_MORE_THAN;
            let tree = {
                let (library, relations, id) =
                    (library.clone(), relations.clone(), head.id.clone());
                move || {
                    opened.get().then(|| {
                        // Every feat below the head, children by title as `Leads to` lists them.
                        let keys: HashMap<EntryId, String> = relations
                            .descendants(&id)
                            .into_iter()
                            .map(|d| {
                                let key = sort_key(&title_of(&library, &d));
                                (d, key)
                            })
                            .collect();
                        let tree = relations.dependents_tree(&id, &|a, b| {
                            keys.get(a).cmp(&keys.get(b)).then_with(|| a.cmp(b))
                        });
                        if folds {
                            let mut below = HashMap::new();
                            count_below(&relations, &tree, &mut below);
                            let fold = Fold {
                                library: library.clone(),
                                below: Shared::new(below),
                            };
                            let rows = fold_rows(&fold, &tree.next);
                            let cut = tree.cut.then(|| {
                                view! { <p class="chain-cut muted">"The chain goes on beyond this point."</p> }
                            });
                            view! { <ul class="chain chain-fold">{rows}</ul>{cut} }.into_any()
                        } else {
                            view! { <ul class="chain">{chain_tree(&library, &tree, false)}</ul> }
                                .into_any()
                        }
                    })
                }
            };
            let summary = builds_on_it(head.total);
            let head_id = head.id.clone();
            view! {
                <li class="chain-head" node_ref=row>
                    <details
                        open=is_wanted
                        on:toggle=move |ev| {
                            let open = event_target::<web_sys::HtmlDetailsElement>(&ev).open();
                            if open {
                                opened.set(true);
                            }
                            remember_open(address, &head_id, open);
                        }
                    >
                        <summary>
                            {icons::chevron_right("disclosure-chevron")}
                            <span class="chain-name">{title}</span>
                            <span class="muted chain-count">{summary}</span>
                        </summary>
                        <p class="chain-open"><a href=href>"Open the feat"</a></p>
                        {tree}
                    </details>
                </li>
            }
        })
        .collect_view();
    view! {
        <p class="muted" role="status">{format!(
            "{} {} a chain",
            fmt::pluralize(count, "feat", "feats"),
            if count == 1 { "starts" } else { "start" }
        )}</p>
        <ul class="chain-heads">{rows}</ul>
    }
}

/// Writes the chain the reader opened into the address, in place of the current history entry,
/// so Back from one of its feats returns to the chain. Closing the chain that the address names
/// clears it; any other chain leaves the address alone.
fn remember_open(address: ReadSignal<Route>, head: &EntryId, open: bool) {
    let current = match address.get_untracked() {
        Route::FeatChains { open } => open,
        _ => return,
    };
    let target = if open {
        Some(head.clone())
    } else if current.as_ref() == Some(head) {
        None
    } else {
        return;
    };
    if target != current {
        route::replace(&Route::FeatChains { open: target });
    }
}
