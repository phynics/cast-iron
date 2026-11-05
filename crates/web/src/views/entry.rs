//! The entry screen: title, chips, breadcrumb, rich body, references, copyright, provenance.

use crate::js;
use crate::library::{Library, Resolution, use_library};
use crate::route::{self, Route};
use crate::store::{self, Bookmark};
use crate::views::book::BookPosition;
use crate::views::entry_contents::ArticleContents;
use crate::views::recent;
use crate::views::related::{Indexes, Related, needs};
use crate::views::rich::{RELATED_ANCHOR, RichBody};
use crate::views::util::{kind_name, local_resource, set_title};
use cast_iron_schema::{EntryId, EntryKind, EntryReference, RetirementReason, RuleEntry};
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

/// The design 6.2 retirement reason, in reader-facing words. The category stays explicit so
/// that a dead link never looks like a generic missing page.
pub fn retirement_reason_label(reason: RetirementReason) -> &'static str {
    match reason {
        RetirementReason::Rights => "removed for licensing reasons",
        RetirementReason::OutOfScope => "outside the scope of this library",
        RetirementReason::NoSource => "no matching entry exists in this library",
        RetirementReason::Unrepresentable => "cannot be represented in this format",
    }
}

#[component]
pub fn Entry(id: EntryId, anchor: Option<String>) -> impl IntoView {
    let library = use_library();
    // An old bookmark or link resolves in one step. The address bar becomes canonical without
    // a Back entry; the fetch below uses the canonical id immediately.
    let resolution = library.resolve(&id);
    let retired_reason = match &resolution {
        Resolution::Retired { reason, .. } => Some(*reason),
        _ => None,
    };
    let (id, aliased) = match resolution {
        Resolution::Aliased { canonical, .. } => (canonical, true),
        _ => (id, false),
    };
    if aliased {
        let route = Route::Entry {
            id: id.clone(),
            anchor: anchor.clone(),
        };
        Effect::new(move |_| route::replace(&route));
    }
    // A retired id has no canonical record and is never redirected to a guessed article.
    // Show the standard "not in this library" state with its reason category.
    if let Some(reason) = retired_reason {
        set_title("Not in this library");
        return view! {
            <article class="entry">
                <h1 data-testid="entry-title">"Not in this library"</h1>
                <p class="notice">"This saved entry is not part of the current library."</p>
                <p class="muted">"Reason: " {retirement_reason_label(reason)}"."</p>
                <p class="muted"><code>{id.to_string()}</code></p>
                <p>
                    <a class="btn" href=Route::Home.href()>"Go to Home"</a>
                </p>
            </article>
        }
        .into_any();
    }
    let stub_title = library.stub(&id).map(|s| s.title.clone());
    set_title(stub_title.as_deref().unwrap_or("Entry"));
    let fetch_id = id.clone();
    let fetch_library = library.clone();
    // The index files that this kind of entry shows panels from load with it, so that the panels
    // arrive with the text and nothing above the body moves afterwards. A file that is missing
    // or damaged only costs the panels.
    let entry = local_resource(async move {
        let entry = fetch_library.entry(&fetch_id).await?;
        let wanted = needs(entry.kind);
        let mut indexes = Indexes::default();
        if wanted.relations {
            match fetch_library.discovery().relations().await {
                Ok(found) => indexes.relations = found,
                Err(error) => leptos::logging::warn!("relation panels are off: {error}"),
            }
        }
        if wanted.spell_lists {
            match fetch_library.discovery().spell_lists().await {
                Ok(found) => indexes.spell_lists = found,
                Err(error) => leptos::logging::warn!("spell list panel is off: {error}"),
            }
        }
        Ok::<_, crate::library::LibraryError>((entry, indexes))
    });
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
                Some(Ok((entry, indexes))) => view! {
                    <EntryView
                        library=library.clone()
                        entry=entry.clone()
                        indexes=indexes.clone()
                        anchor=anchor.clone()
                    />
                }.into_any(),
            })}
        </article>
    }
    .into_any()
}

#[component]
fn EntryView(
    library: Library,
    entry: RuleEntry,
    indexes: Indexes,
    anchor: Option<String>,
) -> impl IntoView {
    set_title(&entry.title);
    {
        let (id, title) = (entry.id.clone(), entry.title.clone());
        spawn_local(async move { recent::record(id, title).await });
    }

    let chain = parent_chain(&library, &entry.id);
    let mut chain_titles: Vec<&str> = chain.iter().map(|(_, t)| t.as_str()).collect();
    chain_titles.push(entry.kind.label());
    if entry.kind == EntryKind::PrestigeClass {
        chain_titles.push("Classes");
    }
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

    let has_books = library.has_books();
    let sources = entry
        .source_ids
        .iter()
        .map(|id| {
            let title = library
                .source(id)
                .map_or_else(|| id.clone(), |s| s.title.clone());
            // A library with a book order opens the book; an older one has only the old page.
            let route = if has_books {
                Route::Book {
                    source: id.clone(),
                    chapter: None,
                }
            } else {
                Route::BrowseSource(id.clone())
            };
            view! { <a class="chip-link" href=route.href()>{title}</a> }
        })
        .collect_view();

    // Relationship panels already expose these targets. Avoid a second adjacent link list.
    let related_targets: std::collections::BTreeSet<EntryId> = indexes
        .relations
        .as_ref()
        .map(|relations| {
            [
                cast_iron_schema::RelationKind::Part,
                cast_iron_schema::RelationKind::Archetype,
            ]
            .into_iter()
            .flat_map(|kind| relations.sources_of(&entry.id, kind))
            .cloned()
            .collect()
        })
        .unwrap_or_default();
    let references: Vec<_> = unique_targets(&entry.links)
        .into_iter()
        .filter(|reference| !related_targets.contains(&reference.target))
        .collect();
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
    let has_related = indexes.relations.is_some()
        || !entry.relations.is_empty()
        || matches!(
            kind,
            EntryKind::Class | EntryKind::Spell | EntryKind::Trait | EntryKind::PrestigeClass
        );
    let related_href = Route::Entry {
        id: entry.id.clone(),
        anchor: Some(RELATED_ANCHOR.into()),
    }
    .href();
    let category_route = if kind == EntryKind::Rule {
        Route::Books
    } else {
        Route::browse_kind(kind)
    };
    let disabled_book = !library.is_entry_active(&entry.id);
    view! {
        <nav class="crumbs" aria-label="Breadcrumb">
            <ol>
                <li><a href=if kind == EntryKind::Rule { Route::Books.href() } else { Route::Home.href() }>{if kind == EntryKind::Rule { "Books" } else { "Home" }}</a></li>
                {(kind != EntryKind::Rule).then(|| view! { <li><a href=Route::browse_kind(kind).href()>{kind.label()}</a></li> })}
                {crumbs_chain}
                {crumbs_sections}
            </ol>
        </nav>
        <header class="entry-head">
            <h1 data-testid="entry-title">{entry.title.clone()}</h1>
            <div class="entry-meta">
                <a class="chip-link chip-kind" href=category_route.href()>{kind_name(kind)}</a>
                {sources}
            </div>
            <EntryStar id=entry.id.clone() title=plain_title kind />
        </header>
        {disabled_book.then(|| view! { <p class="notice disabled-book-notice">"This book is disabled for search and browsing. "<a href=Route::Books.href()>"Open Books"</a>" to reactivate it. Your starred entry is still available."</p> })}
        {has_related.then(|| view! { <p class="reader-jumps"><a href=related_href>"Related entries"</a></p> })}
        <ArticleContents body=entry.body.clone() id=entry.id.clone() />
        <RichBody body=entry.body.clone() anchor=anchor />
        <div id=RELATED_ANCHOR><Related library=library.clone() entry=entry.clone() indexes /></div>
        <BookPosition library=library.clone() id=entry.id.clone() />
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

#[component]
fn EntryStar(id: EntryId, title: String, kind: EntryKind) -> impl IntoView {
    let library = use_library();
    let bookmark = RwSignal::new(None::<Bookmark>);
    let loaded = RwSignal::new(false);
    let writing = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    {
        let id = id.clone();
        let library = library.clone();
        spawn_local(async move {
            // Pressed means "some stored row resolves to this id", not just the exact key.
            let matches = |stored: &EntryId| library.resolves_to(stored, &id);
            match store::bookmarks_matching(matches).await {
                Ok(rows) => bookmark.set(store::combine_saved_rows(&rows)),
                Err(e) => error.set(Some(e.to_string())),
            }
            loaded.set(true);
        });
    }
    let toggle_title = title.clone();
    let toggle = move |_| {
        if writing.get_untracked() {
            return;
        }
        writing.set(true);
        let id = id.clone();
        let title = toggle_title.clone();
        let library = library.clone();
        spawn_local(async move {
            error.set(None);
            let pressed = bookmark.get_untracked().is_some();
            let result = if pressed {
                // Unstar removes every row that resolves here, so a legacy row cannot return.
                let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                store::delete_bookmarks_matching(matches).await.map(|_| ())
            } else {
                // Star adds nothing when a resolving row exists, so it cannot duplicate.
                let fresh = Bookmark::new(id.clone(), title, kind, js::now_ms());
                let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                store::put_bookmark_unless_matching(&fresh, matches)
                    .await
                    .map(|_| ())
            };
            match result {
                Ok(()) => {
                    let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                    match store::bookmarks_matching(matches).await {
                        Ok(rows) => bookmark.set(store::combine_saved_rows(&rows)),
                        Err(e) => error.set(Some(e.to_string())),
                    }
                }
                Err(e) => error.set(Some(format!("Could not update starred status: {e}"))),
            }
            writing.set(false);
        });
    };
    let label = title.clone();
    view! {
        <div class="bookmark-control">
            <button class="btn bookmark-toggle" type="button"
                aria-label=move || format!("{} {label}", if bookmark.with(|b| b.is_some()) { "Unstar" } else { "Star" })
                aria-pressed=move || bookmark.with(|b| b.is_some()).to_string()
                disabled=move || !loaded.get() || writing.get() on:click=toggle>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9z"/></svg>
                {move || if bookmark.with(|b| b.is_some()) { "Starred" } else { "Star" }}
            </button>
            {move || error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })}
        </div>
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
    fn every_retirement_reason_has_its_own_reader_facing_label() {
        let reasons = [
            RetirementReason::Rights,
            RetirementReason::OutOfScope,
            RetirementReason::NoSource,
            RetirementReason::Unrepresentable,
        ];
        let mut labels: Vec<&str> = reasons.into_iter().map(retirement_reason_label).collect();
        assert!(labels.iter().all(|label| !label.is_empty()));
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), reasons.len());
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
