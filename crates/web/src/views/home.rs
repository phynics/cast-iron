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
    let (primary, _) = crate::views::categories::arrange(&library.kind_counts());
    let kinds = primary
        .into_iter()
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
                <div class="home-section-heading">
                    <h2 id="home-bookmarks">"Bookmarks"</h2>
                    <a href=Route::Bookmarks.href()>"Open bookmarks"</a>
                </div>
                {move || match bookmarks.get() {
                    None => view! { <p class="muted" role="status">"Loading bookmarks..."</p> }.into_any(),
                    Some(Err(_)) => view! { <p class="error" role="alert">"Could not load bookmarks. Open bookmarks to try again."</p> }.into_any(),
                    Some(Ok(items)) if items.is_empty() => view! {
                        <p class="muted">"No bookmarks yet. Open an entry and choose Bookmark to save it here."</p>
                    }.into_any(),
                    Some(Ok(items)) => view! {
                        <ul class="home-bookmarks">
                            {items.into_iter().take(6).map(|item| {
                                let stub = library.stub(&item.entry_id);
                                let title = stub.map_or_else(|| item.title_snapshot.clone(), |s| s.title.clone());
                                let kind = stub.map_or(item.kind, |s| s.kind);
                                if stub.is_some() {
                                    view! { <li><a href=Route::entry(item.entry_id).href()>
                                        <span>{title}</span><span class="muted">{kind.label()}</span>
                                    </a></li> }.into_any()
                                } else {
                                    view! { <li class="home-bookmark-missing"><span>{title}</span><span class="muted">"Not in this library"</span></li> }.into_any()
                                }
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
