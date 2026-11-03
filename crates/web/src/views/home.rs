//! Home: a prominent search box, categories with counts, recent entries, bookmarks, publications.

use crate::library::use_library;
use crate::route::{Route, navigate};
use crate::store;
use crate::views::recent;
use crate::views::util::set_title;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn Home() -> impl IntoView {
    set_title("");
    let library = use_library();
    let query = RwSignal::new(String::new());
    let recents = RwSignal::new(Vec::<recent::Recent>::new());
    let bookmark_count = RwSignal::new(None::<usize>);
    spawn_local(async move {
        recents.set(recent::load().await);
        bookmark_count.set(store::list_bookmarks().await.ok().map(|b| b.len()));
    });
    // Load the index in the background so that the first search is instant.
    {
        let library = library.clone();
        spawn_local(async move {
            let _ = library.search_index().await;
        });
    }

    let total = library.catalog().entries.len();
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
        <section class="hero" aria-labelledby="home-title">
            <h1 id="home-title">"Find any rule, offline"</h1>
            <form class="search-form hero-search" role="search" on:submit=move |ev| {
                ev.prevent_default();
                navigate(&Route::search(query.get_untracked().trim()));
            }>
                <label class="sr-only" for="search-input">"Search the rules"</label>
                <input
                    id="search-input"
                    type="search"
                    placeholder=format!("Search {total} entries")
                    autocomplete="off"
                    autocapitalize="off"
                    spellcheck="false"
                    enterkeyhint="search"
                    prop:value=move || query.get()
                    on:input=move |ev| query.set(event_target_value(&ev))
                />
                <button class="btn btn-primary" type="submit">"Search"</button>
            </form>
            <p class="muted hint-keys">"Press / anywhere to search."</p>
        </section>
        <section aria-labelledby="home-kinds">
            <h2 id="home-kinds">"Browse by category"</h2>
            <ul class="tiles">{kinds}</ul>
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
        <section aria-labelledby="home-bookmarks">
            <h2 id="home-bookmarks">"Bookmarks"</h2>
            <p>
                {move || match bookmark_count.get() {
                    Some(0) => "Nothing saved yet. Open any entry and choose Bookmark to keep it here.".to_string(),
                    Some(1) => "1 saved entry.".to_string(),
                    Some(n) => format!("{n} saved entries."),
                    None => String::new(),
                }}
                " "
                <a href=Route::Bookmarks.href()>"Open bookmarks"</a>
            </p>
        </section>
        <section aria-labelledby="home-sources">
            <h2 id="home-sources">"Publications"</h2>
            <ul class="tiles">{sources}</ul>
        </section>
        </div>
    }
}
