//! Home: saved entries, recent reading, and paths into the library.

use crate::library::use_library;
use crate::route::Route;
use crate::store::{self, Bookmark};
use crate::views::recent;
use crate::views::util::set_title;
use leptos::prelude::*;
use leptos::task::spawn_local;

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
    let refresh_bookmarks = Callback::new(move |_| {
        spawn_local(async move {
            bookmarks.set(Some(
                store::list_bookmarks().await.map_err(|e| e.to_string()),
            ));
        });
    });
    let (primary, _) = crate::views::categories::arrange(&library.discovery().kind_counts());
    let kinds = primary
        .into_iter()
        .filter(|(kind, _)| *kind != cast_iron_schema::EntryKind::Rule)
        .take(6)
        .map(|(kind, n)| {
            view! {
                <li><a class="tile" href=Route::BrowseKind(kind).href()>
                    <span class="tile-name">{kind.label()}</span><span class="tile-count">{n}</span>
                </a></li>
            }
        })
        .collect_view();
    let has_books = library.has_books();

    view! {
        <div class="wide home-page">
            <h1 id="home-title">"Your rules reference"</h1>
            <section aria-labelledby="home-bookmarks">
                <div class="home-section-heading"><h2 id="home-bookmarks">"Starred"</h2></div>
                <crate::views::star_transfer::StarTransfer on_imported=refresh_bookmarks />
                {move || star_error.get().map(|error| view! { <p class="error" role="alert">{error}</p> })}
                {move || match bookmarks.get() {
                    None => view! { <p class="muted" role="status">"Loading starred entries…"</p> }.into_any(),
                    Some(Err(_)) => view! { <p class="error" role="alert">"Could not load starred entries."</p> }.into_any(),
                    Some(Ok(items)) if items.is_empty() => view! {
                        <p class="muted">"No starred entries yet. Open an entry and choose Star to save it here."</p>
                    }.into_any(),
                    Some(Ok(items)) => view! {
                        <ul class="home-bookmarks">
                            {items.into_iter().map(|item| {
                                let stub = library.stub(&item.entry_id);
                                let title = stub.map_or_else(|| item.title_snapshot.clone(), |s| s.title.clone());
                                let kind = stub.map_or(item.kind, |s| s.kind);
                                let id = item.entry_id.clone();
                                let title_for_error = title.clone();
                                view! { <li class="home-bookmark-row">
                                    {if stub.is_some() {
                                        view! { <a aria-label=title.clone() href=Route::entry(id.clone()).href()><span>{title.clone()}</span><span class="muted">{kind.label()}</span></a> }.into_any()
                                    } else {
                                        view! { <span class="home-bookmark-missing"><span>{title}</span><span class="muted">"Not in this library"</span></span> }.into_any()
                                    }}
                                    <button class="btn" type="button" aria-label=format!("Unstar {title_for_error}") disabled=move || unstarring.get() on:click=move |_| {
                                        if unstarring.get_untracked() { return; }
                                        unstarring.set(true);
                                        let id = id.clone();
                                        spawn_local(async move {
                                            match store::delete_bookmark(&id).await {
                                                Ok(()) => {
                                                    star_error.set(None);
                                                    bookmarks.set(Some(store::list_bookmarks().await.map_err(|e| e.to_string())));
                                                }
                                                Err(error) => star_error.set(Some(format!("Could not unstar this entry: {error}"))),
                                            }
                                            unstarring.set(false);
                                        });
                                    }>
                                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9z"/></svg>
                                        "Unstar"
                                    </button>
                                </li> }.into_any()
                            }).collect_view()}
                        </ul>
                    }.into_any(),
                }}
            </section>
            <Show when=move || !recents.with(|r| r.is_empty())>
                <section aria-labelledby="home-recent">
                    <h2 id="home-recent">"Recently viewed"</h2>
                    <ul class="pill-list">
                        <For each=move || recents.get() key=|r| r.id.clone() let:item>
                            <li><a class="pill" href=Route::entry(item.id.clone()).href()>{item.title.clone()}</a></li>
                        </For>
                    </ul>
                </section>
            </Show>
            <section aria-labelledby="home-kinds">
                <Show when=move || use_library().discovery().entries().next().is_none()>
                    <p class="notice">"No books are active. "<a href=Route::Books.href()>"Open Books"</a>" to choose your sources."</p>
                </Show>
                <div class="home-section-heading"><h2 id="home-kinds">"Browse by category"</h2><a href=Route::Browse.href()>"All categories"</a></div>
                <ul class="tiles">{kinds}</ul>
            </section>
            <section class="home-books" aria-labelledby="home-sources">
                <h2 id="home-sources">{if has_books { "Read by book" } else { "Publications" }}</h2>
                <p class="muted">{if has_books { "Explore rules in book and chapter order." } else { "Explore rules by publication." }}</p>
                <a href=Route::Books.href()>"Browse books"</a>
            </section>
        </div>
    }
}
