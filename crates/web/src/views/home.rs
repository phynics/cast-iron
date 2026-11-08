//! Home: saved entries, recent reading, and the category directory.

use crate::library::use_library;
use crate::route::Route;
use crate::store::{self, Bookmark};
use crate::views::browse::CategoryDirectory;
use crate::views::icons;
use crate::views::recent;
use crate::views::util::{kind_name, set_title};
use cast_iron_schema::{EntryId, EntryKind};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::BTreeSet;

/// Starred rows shown before the reader asks for the rest.
const STARRED_PREVIEW: usize = 5;
/// Recent rows shown before the reader asks for the rest of the stored history.
const RECENT_PREVIEW: usize = 5;

/// What one Home row shows. Starred and Recently viewed share it.
struct RowData {
    id: EntryId,
    title: String,
    kind: Option<EntryKind>,
    book: Option<String>,
    /// False for a starred entry the installed library no longer holds.
    in_library: bool,
}

/// Lower-cased titles that occur more than once, so those rows can also name their book.
fn duplicate_titles<'a>(titles: impl Iterator<Item = &'a str>) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for title in titles {
        let key = title.to_lowercase();
        if !seen.insert(key.clone()) {
            duplicates.insert(key);
        }
    }
    duplicates
}

/// One line: a link (title that truncates, then a muted kind and, when titles collide, the
/// book), and an optional trailing action.
fn entry_row(row: &RowData, show_book: bool, action: Option<AnyView>) -> AnyView {
    let meta = if row.in_library {
        let kind = row.kind.map(kind_name);
        let book = show_book.then(|| row.book.clone()).flatten();
        match (kind, book) {
            (Some(kind), Some(book)) => format!("{kind} · {book}"),
            (Some(kind), None) => kind.to_string(),
            (None, Some(book)) => book,
            (None, None) => String::new(),
        }
    } else {
        "Not in this library".to_string()
    };
    // The name is the bare title, except when titles collide: then it carries the kind and
    // book, so a screen reader can tell the rows apart as a sighted reader does.
    let name = if show_book && row.in_library {
        format!("{}, {}", row.title, meta.replace(" · ", ", "))
    } else {
        row.title.clone()
    };
    let body = view! {
        <span class="home-row-title">{row.title.clone()}</span>
        <span class="home-row-meta">{meta}</span>
    };
    let main = if row.in_library {
        view! { <a class="home-row-link" aria-label=name href=Route::entry(row.id.clone()).href()>{body}</a> }.into_any()
    } else {
        view! { <span class="home-row-link home-bookmark-missing">{body}</span> }.into_any()
    };
    view! { <li class="home-row">{main}{action}</li> }.into_any()
}

#[component]
pub fn Home() -> impl IntoView {
    set_title("");
    let library = use_library();
    let recents = crate::app::use_app().recents;
    let bookmarks = RwSignal::new(None::<Result<Vec<Bookmark>, String>>);
    spawn_local(async move {
        bookmarks.set(Some(
            store::list_bookmarks().await.map_err(|e| e.to_string()),
        ));
    });
    // Load the index in the background so that the first search is instant. `data-index-ready`
    // marks the end of that read, so a test can wait for it before it reloads the page.
    let index_ready = RwSignal::new(false);
    {
        let library = library.clone();
        spawn_local(async move {
            let _ = library.search_index().await;
            index_ready.try_set(true);
        });
    }
    let star_error = RwSignal::new(None::<String>);
    let unstarring = RwSignal::new(false);
    let expanded = RwSignal::new(false);
    let recent_expanded = RwSignal::new(false);
    let starred_library = library.clone();
    let title_ref = NodeRef::<leptos::html::H1>::new();

    view! {
        <div class="wide home-page" data-index-ready=move || index_ready.get().then_some("true")>
            <h1 id="home-title" tabindex="-1" node_ref=title_ref>"Your rules reference"</h1>
            <Show when=move || use_library().discovery().entries().next().is_none()>
                <p class="notice">"No books are active. "<a href=Route::BooksManage.href()>"Open Manage books"</a>" to turn some on."</p>
            </Show>
            {move || star_error.get().map(|error| view! { <p class="error" role="alert">{error}</p> })}
            {move || match bookmarks.get() {
                // Nothing while loading and nothing when empty: the section appears with the first star.
                None => ().into_any(),
                Some(Err(_)) => view! { <p class="error" role="alert">"Could not load starred entries."</p> }.into_any(),
                Some(Ok(items)) if items.is_empty() => ().into_any(),
                Some(Ok(items)) => {
                    let library = starred_library.clone();
                    // One row per saved item: legacy rows that resolve to a canonical id
                    // join it instead of appearing twice after a refused migration.
                    let shown = crate::views::bookmarks::shown_items(&items, |id| {
                        library.canonical_target(id)
                    });
                    let total = shown.len();
                    let rows: Vec<RowData> = shown.iter().map(|item| {
                        let stub = library.stub(&item.entry_id);
                        RowData {
                            id: item.entry_id.clone(),
                            title: stub.map_or_else(|| item.title_snapshot.clone(), |s| s.title.clone()),
                            kind: Some(stub.map_or(item.kind, |s| s.kind)),
                            book: stub.and_then(|s| library.source(&s.source)).map(|s| s.title.clone()),
                            in_library: stub.is_some(),
                        }
                    }).collect();
                    let ambiguous = duplicate_titles(rows.iter().map(|r| r.title.as_str()));
                    let visible = move || {
                        let take = if expanded.get() { usize::MAX } else { STARRED_PREVIEW };
                        rows.iter().take(take).map(|row| {
                            let id = row.id.clone();
                            let unstar_name = format!("Unstar {}", row.title);
                            let library = library.clone();
                            let button = view! {
                                <button class="btn star-btn is-on" type="button" aria-label=unstar_name disabled=move || unstarring.get() on:click=move |_| {
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
                                                let list = store::list_bookmarks().await.map_err(|e| e.to_string());
                                                // The section goes with the last star; keep focus on the page.
                                                let emptied = matches!(&list, Ok(items) if items.is_empty());
                                                bookmarks.set(Some(list));
                                                if emptied && let Some(title) = title_ref.get_untracked() {
                                                    let _ = title.focus();
                                                }
                                            }
                                            Err(error) => star_error.set(Some(format!("Could not unstar this entry: {error}"))),
                                        }
                                        unstarring.set(false);
                                    });
                                }>
                                    {icons::star()}
                                </button>
                            }.into_any();
                            entry_row(row, ambiguous.contains(&row.title.to_lowercase()), Some(button))
                        }).collect_view()
                    };
                    let more = (total > STARRED_PREVIEW).then(|| view! {
                        <button
                            class="btn btn-quiet home-list-more"
                            type="button"
                            aria-controls="home-bookmarks-list"
                            aria-expanded=move || if expanded.get() { "true" } else { "false" }
                            on:click=move |_| expanded.update(|open| *open = !*open)
                        >
                            {move || if expanded.get() { "Show fewer".to_string() } else { format!("See all ({})", crate::fmt::count(total)) }}
                        </button>
                    });
                    view! {
                        <section aria-labelledby="home-bookmarks">
                            <h2 id="home-bookmarks">"Starred"</h2>
                            <ul class="home-list home-bookmarks" id="home-bookmarks-list">{visible}</ul>
                            <p class="sr-only" role="status">{move || if expanded.get() { format!("Showing all {} starred entries", crate::fmt::count(total)) } else { String::new() }}</p>
                            {more}
                        </section>
                    }.into_any()
                }
            }}
            <Show when=move || !recents.with(|r| r.is_empty())>
                <section aria-labelledby="home-recent">
                    <div class="home-section-heading">
                        <h2 id="home-recent">"Recently viewed"</h2>
                        <button class="btn btn-quiet" type="button" on:click=move |_| {
                            // Empty the screen now; the write below is the durable copy.
                            recents.set(Vec::new());
                            recent_expanded.set(false);
                            if let Some(title) = title_ref.get_untracked() {
                                let _ = title.focus();
                            }
                            spawn_local(async move { recent::clear(recents).await; });
                        }>"Clear"</button>
                    </div>
                    {move || {
                        let library = use_library();
                        let all = recents.get();
                        let total = all.len();
                        // Resolve every title first: collisions are judged on what is shown, over the
                        // whole history, so a row does not change when "Show more" opens.
                        let resolved: Vec<RowData> = all.iter().map(|item| {
                            let stub = library.stub(&item.id);
                            RowData {
                                id: item.id.clone(),
                                title: stub.map_or_else(|| item.title.clone(), |s| s.title.clone()),
                                kind: stub.map(|s| s.kind),
                                book: stub.and_then(|s| library.source(&s.source)).map(|s| s.title.clone()),
                                in_library: stub.is_some(),
                            }
                        }).collect();
                        let ambiguous = duplicate_titles(resolved.iter().map(|r| r.title.as_str()));
                        let take = if recent_expanded.get() { total } else { RECENT_PREVIEW };
                        let rows = resolved.iter().take(take).map(|row| {
                            entry_row(row, ambiguous.contains(&row.title.to_lowercase()), None)
                        }).collect_view();
                        let more = (total > RECENT_PREVIEW).then(|| view! {
                            <button
                                class="btn btn-quiet home-list-more"
                                type="button"
                                aria-controls="home-recent-list"
                                aria-expanded=move || if recent_expanded.get() { "true" } else { "false" }
                                on:click=move |_| recent_expanded.update(|open| *open = !*open)
                            >
                                {move || if recent_expanded.get() { "Show fewer".to_string() } else { format!("Show more ({})", crate::fmt::count(total - RECENT_PREVIEW)) }}
                            </button>
                        });
                        view! {
                            <ul class="home-list home-recent" id="home-recent-list">{rows}</ul>
                            <p class="sr-only" role="status">{if recent_expanded.get() { format!("Showing all {} recent entries", crate::fmt::count(total)) } else { String::new() }}</p>
                            {more}
                        }
                    }}
                </section>
            </Show>
            // Teaches the star until the first one exists.
            <Show when=move || matches!(bookmarks.get(), Some(Ok(ref items)) if items.is_empty())>
                <p class="muted home-hint">"Tap "{icons::star()}<span class="sr-only">"the star"</span>" on an entry to keep it here."</p>
            </Show>
            <CategoryDirectory note=true />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::duplicate_titles;

    #[test]
    fn equal_titles_are_flagged_regardless_of_case() {
        let found = duplicate_titles(["Dragon, Red", "Fireball", "dragon, red"].into_iter());
        assert_eq!(found.into_iter().collect::<Vec<_>>(), ["dragon, red"]);
        assert!(duplicate_titles(["A", "B"].into_iter()).is_empty());
    }
}
