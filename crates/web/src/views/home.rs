//! Home: saved entries, recent reading, and the category directory.

use crate::library::use_library;
use crate::route::Route;
use crate::store::{self, Bookmark};
use crate::views::browse::CategoryDirectory;
use crate::views::recent;
use crate::views::util::set_title;
use cast_iron_schema::EntryId;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Starred rows shown before the reader asks for the rest.
const STARRED_PREVIEW: usize = 5;

#[component]
pub fn Home() -> impl IntoView {
    set_title("");
    let library = use_library();
    let recents = RwSignal::new(Vec::<recent::Recent>::new());
    let bookmarks = RwSignal::new(None::<Result<Vec<Bookmark>, String>>);
    spawn_local(async move {
        recents.set(recent::load().await);
        bookmarks.set(Some(
            store::list_bookmarks().await.map_err(|e| e.to_string()),
        ));
    });
    // Load the index in the background so that the first search is instant.
    {
        let library = library.clone();
        spawn_local(async move {
            let _ = library.search_index().await;
        });
    }
    let star_error = RwSignal::new(None::<String>);
    let unstarring = RwSignal::new(false);
    let expanded = RwSignal::new(false);

    view! {
        <div class="wide home-page">
            <h1 id="home-title">"Your rules reference"</h1>
            <Show when=move || use_library().discovery().entries().next().is_none()>
                <p class="notice">"No books are active. "<a href=Route::Books.href()>"Open Books"</a>" to choose your sources."</p>
            </Show>
            <section aria-labelledby="home-bookmarks">
                <div class="home-section-heading"><h2 id="home-bookmarks">"Starred"</h2></div>
                {move || star_error.get().map(|error| view! { <p class="error" role="alert">{error}</p> })}
                {move || match bookmarks.get() {
                    None => view! { <p class="muted" role="status">"Loading starred entries…"</p> }.into_any(),
                    Some(Err(_)) => view! { <p class="error" role="alert">"Could not load starred entries."</p> }.into_any(),
                    Some(Ok(items)) if items.is_empty() => view! {
                        <p class="muted">"No starred entries yet. Open an entry and choose Star to save it here."</p>
                    }.into_any(),
                    Some(Ok(items)) => {
                        // One row per saved item: legacy rows that resolve to a canonical id
                        // join it instead of appearing twice after a refused migration.
                        let shown = crate::views::bookmarks::shown_items(&items, |id| {
                            library.canonical_target(id)
                        });
                        let total = shown.len();
                        let visible: Vec<Bookmark> = if expanded.get() {
                            shown
                        } else {
                            shown.into_iter().take(STARRED_PREVIEW).collect()
                        };
                        let rows = visible.into_iter().map(|item| {
                            let stub = library.stub(&item.entry_id);
                            let title = stub.map_or_else(|| item.title_snapshot.clone(), |s| s.title.clone());
                            let kind = stub.map_or(item.kind, |s| s.kind);
                            let id = item.entry_id.clone();
                            let title_for_error = title.clone();
                            let library = library.clone();
                            view! { <li class="home-bookmark-row">
                                {if stub.is_some() {
                                    view! { <a aria-label=title.clone() href=Route::entry(id.clone()).href()><span>{title.clone()}</span><span class="muted">{kind.label()}</span></a> }.into_any()
                                } else {
                                    view! { <span class="home-bookmark-missing"><span>{title}</span><span class="muted">"Not in this library"</span></span> }.into_any()
                                }}
                                <button class="btn star-btn is-on" type="button" aria-label=format!("Unstar {title_for_error}") disabled=move || unstarring.get() on:click=move |_| {
                                    if unstarring.get_untracked() { return; }
                                    unstarring.set(true);
                                    let id = id.clone();
                                    let library = library.clone();
                                    spawn_local(async move {
                                        // Every row that resolves to this item goes, so a
                                        // legacy row cannot resurrect after unstar.
                                        let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                                        match store::delete_bookmarks_matching(matches).await {
                                            Ok(_) => {
                                                star_error.set(None);
                                                bookmarks.set(Some(store::list_bookmarks().await.map_err(|e| e.to_string())));
                                            }
                                            Err(error) => star_error.set(Some(format!("Could not unstar this entry: {error}"))),
                                        }
                                        unstarring.set(false);
                                    });
                                }>
                                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9z"/></svg>
                                </button>
                            </li> }.into_any()
                        }).collect_view();
                        let more = (total > STARRED_PREVIEW).then(|| view! {
                            <button
                                class="btn home-bookmarks-more"
                                type="button"
                                aria-controls="home-bookmarks-list"
                                aria-expanded=move || if expanded.get() { "true" } else { "false" }
                                on:click=move |_| expanded.update(|open| *open = !*open)
                            >
                                {move || if expanded.get() {
                                    "Show fewer".to_string()
                                } else {
                                    format!("Show all {total} starred entries")
                                }}
                            </button>
                        });
                        view! {
                            <ul class="home-bookmarks" id="home-bookmarks-list">{rows}</ul>
                            {more}
                        }.into_any()
                    }
                }}
            </section>
            <Show when=move || !recents.with(|r| r.is_empty())>
                <section aria-labelledby="home-recent">
                    <h2 id="home-recent">"Recently viewed"</h2>
                    <ul class="jump-list">
                        <For each=move || recents.get() key=|r| r.id.clone() let:item>
                            <li><a class="jump" href=Route::entry(item.id.clone()).href()>{item.title.clone()}</a></li>
                        </For>
                    </ul>
                </section>
            </Show>
            <CategoryDirectory note=true />
        </div>
    }
}
