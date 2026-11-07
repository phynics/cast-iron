//! The bookmark toggle on an entry, with a collection picker once it is bookmarked.

use crate::js;
use crate::library::use_library;
use crate::store::{self, Bookmark, Collection};
use crate::views::icons;
use cast_iron_schema::{EntryId, EntryKind};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn BookmarkControl(id: EntryId, title: String, kind: EntryKind) -> impl IntoView {
    let library = use_library();
    let bookmark = RwSignal::new(None::<Bookmark>);
    let collections = RwSignal::new(Vec::<Collection>::new());
    let loaded = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let new_name = RwSignal::new(String::new());

    {
        let id = id.clone();
        let library = library.clone();
        spawn_local(async move {
            // Pressed means "some stored row resolves to this id", so a legacy row that a
            // refused migration left behind still reads as saved.
            let matches = |stored: &EntryId| library.resolves_to(stored, &id);
            match store::bookmarks_matching(matches).await {
                Ok(rows) => bookmark.set(store::combine_saved_rows(&rows)),
                Err(e) => error.set(Some(e.to_string())),
            }
            if let Ok(list) = store::list_collections().await {
                collections.set(list);
            }
            loaded.set(true);
        });
    }

    let toggle = {
        let id = id.clone();
        let title = title.clone();
        let library = library.clone();
        move |_| {
            let id = id.clone();
            let title = title.clone();
            let library = library.clone();
            spawn_local(async move {
                error.set(None);
                let pressed = bookmark.get_untracked().is_some();
                let result = if pressed {
                    // Unstar removes every row that resolves to the id, so a not-yet-migrated
                    // legacy row cannot resurrect.
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
                    Err(e) => error.set(Some(format!("Could not save the bookmark: {e}"))),
                }
            });
        }
    };

    let member = Callback::new({
        let id = id.clone();
        let library = library.clone();
        move |(collection_id, on): (String, bool)| {
            let id = id.clone();
            let library = library.clone();
            spawn_local(async move {
                let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                let result = if on {
                    store::add_to_collection_matching(&collection_id, matches).await
                } else {
                    store::remove_from_collection_matching(&collection_id, matches).await
                };
                match result {
                    Ok(()) => {
                        let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                        if let Ok(rows) = store::bookmarks_matching(matches).await {
                            bookmark.set(store::combine_saved_rows(&rows));
                        }
                    }
                    Err(e) => error.set(Some(format!("Could not update the collection: {e}"))),
                }
            });
        }
    });

    let create = Callback::new({
        let id = id.clone();
        let library = library.clone();
        move |()| {
            let name = new_name.get_untracked().trim().to_string();
            if name.is_empty() {
                return;
            }
            let id = id.clone();
            let library = library.clone();
            spawn_local(async move {
                match store::create_collection(&name).await {
                    Ok(created) => {
                        new_name.set(String::new());
                        let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                        let added = store::add_to_collection_matching(&created.id, matches).await;
                        if let Err(e) = added {
                            error.set(Some(format!("Could not add to the collection: {e}")));
                        }
                        if let Ok(list) = store::list_collections().await {
                            collections.set(list);
                        }
                        let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                        if let Ok(rows) = store::bookmarks_matching(matches).await {
                            bookmark.set(store::combine_saved_rows(&rows));
                        }
                    }
                    Err(e) => error.set(Some(format!("Could not create the collection: {e}"))),
                }
            });
        }
    });

    view! {
        <div class="bookmark-control">
            <button
                class="btn bookmark-toggle"
                type="button"
                aria-pressed=move || bookmark.with(|b| b.is_some()).to_string()
                disabled=move || !loaded.get()
                on:click=toggle
            >
                {move || if bookmark.with(|b| b.is_some()) {
                    icons::star_filled().into_any()
                } else {
                    icons::star().into_any()
                }}
                {move || if bookmark.with(|b| b.is_some()) { "Bookmarked" } else { "Bookmark" }}
            </button>
            <Show when=move || bookmark.with(|b| b.is_some())>
                <details class="picker">
                    <summary>"Collections"</summary>
                    <div class="picker-body">
                        <Show
                            when=move || !collections.with(|c| c.is_empty())
                            fallback=|| view! { <p class="muted">"No collections yet. Name one below to group bookmarks."</p> }
                        >
                            <ul class="plain">
                                <For
                                    each=move || collections.get()
                                    key=|c| (c.id.clone(), c.name.clone())
                                    let:collection
                                >
                                    {
                                        let cid = collection.id.clone();
                                        let cid2 = collection.id.clone();
                                        view! {
                                            <li>
                                                <label class="check">
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || bookmark.with(|b| b.as_ref().is_some_and(|b| b.in_collection(&cid)))
                                                        on:change=move |ev| member.run((cid2.clone(), event_target_checked(&ev)))
                                                    />
                                                    <span>{collection.name.clone()}</span>
                                                </label>
                                            </li>
                                        }
                                    }
                                </For>
                            </ul>
                        </Show>
                        <form class="inline-form" on:submit=move |ev| { ev.prevent_default(); create.run(()); }>
                            <label class="sr-only" for="new-collection-entry">"New collection name"</label>
                            <input
                                id="new-collection-entry"
                                type="text"
                                placeholder="New collection"
                                maxlength="60"
                                autocomplete="off"
                                prop:value=move || new_name.get()
                                on:input=move |ev| new_name.set(event_target_value(&ev))
                            />
                            <button class="btn" type="submit">"Create and add"</button>
                        </form>
                    </div>
                </details>
            </Show>
            {move || error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })}
        </div>
    }
}
