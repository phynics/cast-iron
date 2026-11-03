//! Feat chains: the feats that start a chain, and the feats that follow from each. Also the
//! tree renderer that the feat entry screen uses for its own chain.

use crate::graph::{ChainHead, ChainNode, Relations};
use crate::library::{Library, use_library};
use crate::route::Route;
use crate::views::util::{Shared, local_resource, set_title, sort_key};
use cast_iron_schema::{EntryId, EntryKind};
use leptos::prelude::*;

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
    let relations = {
        let library = library.clone();
        let revision = library.selection_revision();
        local_resource(async move {
            let _ = revision.get();
            library.discovery().relations().await
        })
    };
    let body = move || {
        let library = library.clone();
        relations.with(|state| match state {
            None => view! { <p class="status" role="status">"Loading feat chains…"</p> }.into_any(),
            Some(Err(error)) => view! {
                <div class="error" role="alert"><p>{error.to_string()}</p></div>
            }
            .into_any(),
            Some(Ok(None)) => view! {
                <p class="notice">
                    "This library has no feat chains. It was installed before they existed. "
                    "Open Settings and check for a library update."
                </p>
            }
            .into_any(),
            Some(Ok(Some(relations))) => {
                view! { <HeadList library relations=Shared::from_rc(relations.clone()) /> }
                    .into_any()
            }
        })
    };
    view! {
        <div class="wide">
            <nav class="crumbs" aria-label="Breadcrumb">
                <ol>
                    <li><a href=Route::Browse.href()>"Browse"</a></li>
                    <li><a href=Route::BrowseKind(EntryKind::Feat).href()>"Feats"</a></li>
                </ol>
            </nav>
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

#[component]
fn HeadList(library: Library, relations: Shared<Relations>) -> impl IntoView {
    let mut heads: Vec<(String, ChainHead)> = relations
        .chain_heads()
        .into_iter()
        .map(|h| (title_of(&library, &h.id), h))
        .collect();
    // Most built-upon first; equal counts in name order.
    heads.sort_by(|a, b| {
        b.1.total
            .cmp(&a.1.total)
            .then_with(|| sort_key(&a.0).cmp(&sort_key(&b.0)))
    });
    let count = heads.len();
    let rows = heads
        .into_iter()
        .map(|(title, head)| {
            let opened = RwSignal::new(false);
            let href = Route::entry(head.id.clone()).href();
            let tree = {
                let (library, relations, id) =
                    (library.clone(), relations.clone(), head.id.clone());
                move || {
                    opened.get().then(|| {
                        let tree = relations.dependents_tree(&id);
                        view! { <ul class="chain">{chain_tree(&library, &tree, false)}</ul> }
                    })
                }
            };
            let summary = format!(
                "{} {} build on it",
                head.total,
                if head.total == 1 { "feat" } else { "feats" }
            );
            view! {
                <li class="chain-head">
                    <details
                        on:toggle=move |ev| {
                            if event_target::<web_sys::HtmlDetailsElement>(&ev).open() {
                                opened.set(true);
                            }
                        }
                    >
                        <summary>
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
        <p class="muted" role="status">{format!("{count} feats start a chain")}</p>
        <ul class="chain-heads">{rows}</ul>
    }
}
