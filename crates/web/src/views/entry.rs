//! The entry screen: title, chips, breadcrumb, rich body, references, copyright, provenance.

use crate::library::{Library, use_library};
use crate::route::Route;
use crate::views::bookmark_control::BookmarkControl;
use crate::views::recent;
use crate::views::rich::RichBody;
use crate::views::util::{kind_name, local_resource, set_title};
use cast_iron_schema::{EntryId, EntryKind, EntryReference, RuleEntry};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// The parent chain of a stub, root first, stopping at a missing stub or a cycle.
pub fn parent_chain(library: &Library, id: &EntryId) -> Vec<(EntryId, String)> {
    let mut chain = Vec::new();
    let mut next = library.stub(id).and_then(|s| s.parent.clone());
    while let Some(parent) = next {
        if chain.len() >= 8 || chain.iter().any(|(p, _)| *p == parent) || parent == *id {
            break;
        }
        let Some(stub) = library.stub(&parent) else {
            break;
        };
        chain.push((stub.id.clone(), stub.title.clone()));
        next = stub.parent.clone();
    }
    chain.reverse();
    chain
}

/// Section names that the parent chain does not already show.
pub fn extra_sections(section: &[String], chain_titles: &[&str]) -> Vec<String> {
    section
        .iter()
        .filter(|s| !chain_titles.iter().any(|t| t.eq_ignore_ascii_case(s)))
        .cloned()
        .collect()
}

/// One reference per target entry; anchors are kept only when the target appears once.
pub fn unique_targets(links: &[EntryReference]) -> Vec<EntryReference> {
    let mut out: Vec<EntryReference> = Vec::new();
    for link in links {
        match out.iter_mut().find(|l| l.target == link.target) {
            Some(existing) => existing.anchor = None,
            None => out.push(link.clone()),
        }
    }
    out
}

#[component]
pub fn Entry(id: EntryId, anchor: Option<String>) -> impl IntoView {
    let library = use_library();
    let stub_title = library.stub(&id).map(|s| s.title.clone());
    set_title(stub_title.as_deref().unwrap_or("Entry"));
    let fetch_id = id.clone();
    let fetch_library = library.clone();
    let entry = local_resource(async move { fetch_library.entry(&fetch_id).await });
    let shown_id = id.clone();
    view! {
        <article class="entry">
            {move || entry.with(|state| match state {
                None => view! {
                    <h1>{stub_title.clone().unwrap_or_else(|| "Entry".to_string())}</h1>
                    <p class="status" role="status">"Loading entry…"</p>
                }.into_any(),
                Some(Err(error)) => view! {
                    <h1>{stub_title.clone().unwrap_or_else(|| "Entry unavailable".to_string())}</h1>
                    <div class="error" role="alert">
                        <p>{error.to_string()}</p>
                        <p>
                            <a href=Route::Settings.href()>"Open Settings"</a>" to check or reinstall the library, or "
                            <a href=Route::Home.href()>"go to Home"</a>"."
                        </p>
                        <p class="muted"><code>{shown_id.to_string()}</code></p>
                    </div>
                }.into_any(),
                Some(Ok(entry)) => view! {
                    <EntryView library=library.clone() entry=entry.clone() anchor=anchor.clone() />
                }.into_any(),
            })}
        </article>
    }
}

#[component]
fn EntryView(library: Library, entry: RuleEntry, anchor: Option<String>) -> impl IntoView {
    set_title(&entry.title);
    {
        let (id, title) = (entry.id.clone(), entry.title.clone());
        spawn_local(async move { recent::record(id, title).await });
    }

    let chain = parent_chain(&library, &entry.id);
    let mut chain_titles: Vec<&str> = chain.iter().map(|(_, t)| t.as_str()).collect();
    chain_titles.push(entry.kind.label());
    let sections = extra_sections(&entry.section, &chain_titles);
    let crumbs_chain = chain
        .iter()
        .map(|(id, title)| {
            view! { <li><a href=Route::entry(id.clone()).href()>{title.clone()}</a></li> }
        })
        .collect_view();
    let crumbs_sections = sections
        .into_iter()
        .map(|s| view! { <li><span>{s}</span></li> })
        .collect_view();

    let sources = entry
        .source_ids
        .iter()
        .map(|id| {
            let title = library
                .source(id)
                .map_or_else(|| id.clone(), |s| s.title.clone());
            view! { <a class="chip-link" href=Route::BrowseSource(id.clone()).href()>{title}</a> }
        })
        .collect_view();

    let references = unique_targets(&entry.links);
    let reference_items = references
        .iter()
        .map(|reference| {
            let stub = library.stub(&reference.target);
            let title = stub.map_or_else(|| reference.target.to_string(), |s| s.title.clone());
            let kind = stub.map(|s| kind_name(s.kind));
            let href = Route::Entry {
                id: reference.target.clone(),
                anchor: reference.anchor.clone(),
            }
            .href();
            view! {
                <li>
                    <a href=href>{title}</a>
                    {kind.map(|k| view! { <span class="muted">" · "{k}</span> })}
                </li>
            }
        })
        .collect_view();
    let has_references = !references.is_empty();

    let copyrights = entry
        .source_ids
        .iter()
        .filter_map(|id| library.source(id).and_then(|s| s.copyright.clone()))
        .map(|line| view! { <p class="copyright">{line}</p> })
        .collect_view();

    let provenance = entry
        .provenance
        .iter()
        .map(|p| {
            view! {
                <div>
                    <dt>"Dataset"</dt><dd>{p.dataset.clone()}</dd>
                    <dt>"Upstream record"</dt><dd><code>{p.record.clone()}</code></dd>
                    <dt>"Upstream file"</dt><dd><code>{p.path.clone()}</code></dd>
                </div>
            }
        })
        .collect_view();

    let kind: EntryKind = entry.kind;
    let plain_title = entry.title.clone();
    view! {
        <nav class="crumbs" aria-label="Breadcrumb">
            <ol>
                <li><a href=Route::Browse.href()>"Browse"</a></li>
                <li><a href=Route::BrowseKind(kind).href()>{kind.label()}</a></li>
                {crumbs_chain}
                {crumbs_sections}
            </ol>
        </nav>
        <header class="entry-head">
            <h1 data-testid="entry-title">{entry.title.clone()}</h1>
            <div class="entry-meta">
                <a class="chip-link chip-kind" href=Route::BrowseKind(kind).href()>{kind_name(kind)}</a>
                {sources}
            </div>
            <BookmarkControl id=entry.id.clone() title=plain_title kind />
        </header>
        <RichBody body=entry.body.clone() anchor=anchor />
        {has_references.then(|| view! {
            <section class="entry-refs" aria-labelledby="references-heading">
                <h2 id="references-heading">"References"</h2>
                <ul class="ref-list">{reference_items}</ul>
            </section>
        })}
        <footer class="entry-foot">
            {copyrights}
            <p class="muted">
                "Open Game License content. "<a href=Route::Legal.href()>"Legal and attribution"</a>
            </p>
            <details class="provenance">
                <summary>"Provenance"</summary>
                <dl class="facts">{provenance}</dl>
            </details>
        </footer>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(target: &str, anchor: Option<&str>) -> EntryReference {
        EntryReference {
            target: EntryId::parse(target).unwrap(),
            anchor: anchor.map(String::from),
        }
    }

    #[test]
    fn references_dedupe_by_target() {
        let out = unique_targets(&[r("a/x", Some("one")), r("a/y", None), r("a/x", Some("two"))]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].anchor, None);
        let single = unique_targets(&[r("a/x", Some("one"))]);
        assert_eq!(single[0].anchor.as_deref(), Some("one"));
    }

    #[test]
    fn sections_skip_chain_titles() {
        let section = vec!["Combat".to_string(), "Actions".to_string()];
        assert_eq!(
            extra_sections(&section, &["combat"]),
            vec!["Actions".to_string()]
        );
        assert_eq!(extra_sections(&section, &[]).len(), 2);
    }
}
