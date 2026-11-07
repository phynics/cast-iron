//! Bookmarks and collections. A bookmark whose entry is gone from the current library still
//! shows, from its saved title, and is never deleted by a content change.

use crate::library::{Library, use_library};
use crate::route::{Route, navigate};
use crate::store::{self, Bookmark, Collection};
use crate::views::util::{kind_name, set_title};
use cast_iron_schema::EntryId;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, Default, PartialEq)]
struct Data {
    bookmarks: Vec<Bookmark>,
    collections: Vec<Collection>,
}

/// Bookmarks that belong to a collection, or all of them, in the stored (newest first) order.
pub fn in_scope<'a>(bookmarks: &'a [Bookmark], collection: Option<&str>) -> Vec<&'a Bookmark> {
    bookmarks
        .iter()
        .filter(|b| collection.is_none_or(|c| b.in_collection(c)))
        .collect()
}

/// How many bookmarks each collection holds.
pub fn collection_size(bookmarks: &[Bookmark], collection_id: &str) -> usize {
    bookmarks
        .iter()
        .filter(|b| b.in_collection(collection_id))
        .count()
}

/// One row per saved item, newest first. An aliased row collapses onto its canonical target
/// and shows the target's id, while a retired or unknown row keeps its own id and snapshot.
/// Collection ids are the union across the rows that collapse together.
pub fn shown_items(
    bookmarks: &[Bookmark],
    target_of: impl Fn(&EntryId) -> EntryId,
) -> Vec<Bookmark> {
    let mut out: Vec<Bookmark> = Vec::new();
    for bookmark in bookmarks {
        let target = target_of(&bookmark.entry_id);
        match out.iter_mut().find(|shown| shown.entry_id == target) {
            Some(shown) => {
                for collection_id in &bookmark.collection_ids {
                    if !shown.collection_ids.contains(collection_id) {
                        shown.collection_ids.push(collection_id.clone());
                    }
                }
            }
            None => out.push(Bookmark {
                entry_id: target,
                ..bookmark.clone()
            }),
        }
    }
    for shown in &mut out {
        shown.collection_ids.sort();
        shown.collection_ids.dedup();
    }
    out
}

fn reload(data: RwSignal<Option<Result<Data, String>>>) {
    spawn_local(async move {
        let result = async {
            let bookmarks = store::list_bookmarks().await.map_err(|e| e.to_string())?;
            let collections = store::list_collections().await.map_err(|e| e.to_string())?;
            Ok(Data {
                bookmarks,
                collections,
            })
        }
        .await;
        data.set(Some(result));
    });
}

#[component]
pub fn Bookmarks() -> impl IntoView {
    set_title("Bookmarks");
    view! { <BookmarksScreen collection=None /> }
}

#[component]
pub fn CollectionView(id: String) -> impl IntoView {
    view! { <BookmarksScreen collection=Some(id) /> }
}

#[component]
fn BookmarksScreen(collection: Option<String>) -> impl IntoView {
    let library = use_library();
    let data = RwSignal::new(None::<Result<Data, String>>);
    let notice = RwSignal::new(None::<String>);
    let editing = RwSignal::new(None::<String>);
    let confirming = RwSignal::new(None::<String>);
    let rename_to = RwSignal::new(String::new());
    let new_name = RwSignal::new(String::new());
    reload(data);

    let scope = collection.clone();
    let title_scope = collection.clone();
    Effect::new(move |_| {
        if let Some(id) = &title_scope {
            let name = data.with(|d| {
                d.as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .and_then(|d| d.collections.iter().find(|c| c.id == *id))
                    .map(|c| c.name.clone())
            });
            set_title(&name.unwrap_or_else(|| "Collection".to_string()));
        }
    });

    let fail = move |what: &str, e: store::StoreError| {
        notice.set(Some(format!("{what}: {e}")));
    };

    let create = move || {
        let name = new_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        spawn_local(async move {
            match store::create_collection(&name).await {
                Ok(_) => {
                    new_name.set(String::new());
                    notice.set(None);
                    reload(data);
                }
                Err(e) => fail("Could not create the collection", e),
            }
        });
    };
    let save_rename = Callback::new(move |id: String| {
        let name = rename_to.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let existing = data.with_untracked(|d| {
            d.as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(|d| d.collections.iter().find(|c| c.id == id).cloned())
        });
        let Some(mut existing) = existing else { return };
        existing.name = name;
        spawn_local(async move {
            match store::put_collection(&existing).await {
                Ok(()) => {
                    editing.set(None);
                    notice.set(None);
                    reload(data);
                }
                Err(e) => fail("Could not rename the collection", e),
            }
        });
    });
    let delete = Callback::new(move |id: String| {
        let viewing_it = scope.as_deref() == Some(id.as_str());
        spawn_local(async move {
            match store::delete_collection(&id).await {
                Ok(()) => {
                    confirming.set(None);
                    notice.set(None);
                    if viewing_it {
                        navigate(&Route::Bookmarks);
                    } else {
                        reload(data);
                    }
                }
                Err(e) => fail("Could not delete the collection", e),
            }
        });
    });
    let remove = Callback::new({
        let library = library.clone();
        move |id: EntryId| {
            let library = library.clone();
            spawn_local(async move {
                // The list shows one row per saved item, so Remove takes every stored row
                // that resolves to it, including a legacy row left by a refused migration.
                let matches = |stored: &EntryId| library.resolves_to(stored, &id);
                match store::delete_bookmarks_matching(matches).await {
                    Ok(_) => reload(data),
                    Err(e) => fail("Could not remove the bookmark", e),
                }
            });
        }
    });
    let set_member = Callback::new({
        let library = library.clone();
        move |(entry, cid, on): (EntryId, String, bool)| {
            let library = library.clone();
            spawn_local(async move {
                let matches = |stored: &EntryId| library.resolves_to(stored, &entry);
                let result = if on {
                    store::add_to_collection_matching(&cid, matches).await
                } else {
                    store::remove_from_collection_matching(&cid, matches).await
                };
                match result {
                    Ok(()) => reload(data),
                    Err(e) => fail("Could not update the collection", e),
                }
            });
        }
    });

    let body_library = library.clone();
    let body_scope = collection.clone();
    let body = move || match data.get() {
        None => view! { <p class="status" role="status">"Loading bookmarks…"</p> }.into_any(),
        Some(Err(message)) => view! {
            <p class="error" role="alert">{format!("Cannot read your bookmarks: {message}")}</p>
        }
        .into_any(),
        Some(Ok(d)) => {
            let current = body_scope.clone().and_then(|id| {
                d.collections
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .or_else(|| {
                        Some(Collection {
                            id,
                            name: String::new(),
                            created_at: 0.0,
                        })
                    })
            });
            if let Some(c) = &current
                && c.name.is_empty()
            {
                return view! {
                    <h1>"Collection not found"</h1>
                    <p>"This collection no longer exists. Your bookmarks are still saved."</p>
                    <p><a href=Route::Bookmarks.href()>"All bookmarks"</a></p>
                }
                .into_any();
            }
            let shown = shown_items(&d.bookmarks, |id| body_library.canonical_target(id));
            let scoped = in_scope(&shown, body_scope.as_deref());
            let heading = current
                .as_ref()
                .map_or_else(|| "Bookmarks".to_string(), |c| c.name.clone());
            let list = bookmark_rows(&body_library, &scoped, &d.collections, remove, set_member);
            let empty = scoped.is_empty().then(|| {
                if current.is_some() {
                    view! {
                        <div class="empty">
                            <p>"This collection is empty."</p>
                            <p class="muted">"Open Bookmarks and tick this collection under a bookmark's Collections, or do it from the entry page."</p>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="empty">
                            <p>"No bookmarks yet."</p>
                            <p class="muted">"Open any entry and choose Bookmark. It is saved on this device and works offline."</p>
                            <p>
                                <a class="btn" href=Route::search("").href()>"Search"</a>" "
                                <a class="btn" href=Route::Home.href()>"Home"</a>
                            </p>
                        </div>
                    }
                    .into_any()
                }
            });
            let collection_items = d
                .collections
                .iter()
                .map(|c| {
                    collection_item(
                        c,
                        &shown,
                        editing,
                        confirming,
                        rename_to,
                        save_rename,
                        delete,
                    )
                })
                .collect_view();
            let back = current.is_some().then(|| {
                view! {
                    <p><a href=Route::Bookmarks.href()>"← All bookmarks"</a></p>
                }
            });
            view! {
                {back}
                <h1>{heading}</h1>
                <p class="muted">{format!("{} saved", scoped.len())}</p>
                {empty}
                <ul class="bm-list">{list}</ul>
                <section class="collections" aria-labelledby="collections-heading">
                    <h2 id="collections-heading">"Collections"</h2>
                    {d.collections.is_empty().then(|| view! {
                        <p class="muted">"Collections group bookmarks, such as one per character or campaign. Create the first one here."</p>
                    })}
                    <ul class="collection-list">{collection_items}</ul>
                    <form class="inline-form" on:submit=move |ev| { ev.prevent_default(); create(); }>
                        <label class="sr-only" for="new-collection">"New collection name"</label>
                        <input
                            id="new-collection"
                            type="text"
                            placeholder="New collection name"
                            maxlength="60"
                            autocomplete="off"
                            prop:value=move || new_name.get()
                            on:input=move |ev| new_name.set(event_target_value(&ev))
                        />
                        <button class="btn" type="submit">"Create collection"</button>
                    </form>
                </section>
            }
            .into_any()
        }
    };

    view! {
        <div class="wide">
            {move || notice.get().map(|m| view! { <p class="error" role="alert">{m}</p> })}
            {body}
        </div>
    }
}

fn bookmark_rows(
    library: &Library,
    bookmarks: &[&Bookmark],
    collections: &[Collection],
    remove: Callback<EntryId>,
    set_member: Callback<(EntryId, String, bool)>,
) -> Vec<AnyView> {
    bookmarks
        .iter()
        .map(|bookmark| {
            let stub = library.stub(&bookmark.entry_id);
            let title = stub.map_or_else(|| bookmark.title_snapshot.clone(), |s| s.title.clone());
            let kind = stub.map_or(bookmark.kind, |s| s.kind);
            let present = stub.is_some();
            let id = bookmark.entry_id.clone();
            let remove_id = id.clone();
            let label = format!("Remove {title}");
            let picker = (!collections.is_empty()).then(|| {
                let boxes = collections
                    .iter()
                    .map(|c| {
                        let (entry, cid) = (id.clone(), c.id.clone());
                        let checked = bookmark.in_collection(&c.id);
                        view! {
                            <li>
                                <label class="check">
                                    <input
                                        type="checkbox"
                                        prop:checked=checked
                                        on:change=move |ev| set_member.run((entry.clone(), cid.clone(), event_target_checked(&ev)))
                                    />
                                    <span>{c.name.clone()}</span>
                                </label>
                            </li>
                        }
                    })
                    .collect_view();
                let n = bookmark.collection_ids.len();
                view! {
                    <details class="picker">
                        <summary>{if n == 0 { "Collections".to_string() } else { format!("Collections ({})", crate::fmt::count(n)) }}</summary>
                        <ul class="plain picker-body">{boxes}</ul>
                    </details>
                }
            });
            let name = if present {
                view! { <a class="bm-title" href=Route::entry(id.clone()).href()>{title.clone()}</a> }
                    .into_any()
            } else {
                view! { <span class="bm-title">{title.clone()}</span> }.into_any()
            };
            view! {
                <li class="bm">
                    <div class="bm-main">
                        {name}
                        <span class="muted">{kind_name(kind)}</span>
                        {(!present).then(|| view! {
                            <span class="tag tag-warn" title="This library has no entry with this id. The bookmark is kept.">
                                "not in this library"
                            </span>
                        })}
                    </div>
                    <div class="bm-actions">
                        {picker}
                        <button class="btn btn-quiet" type="button" aria-label=label on:click=move |_| remove.run(remove_id.clone())>
                            "Remove"
                        </button>
                    </div>
                </li>
            }
            .into_any()
        })
        .collect()
}

fn collection_item(
    c: &Collection,
    bookmarks: &[Bookmark],
    editing: RwSignal<Option<String>>,
    confirming: RwSignal<Option<String>>,
    rename_to: RwSignal<String>,
    save_rename: Callback<String>,
    delete: Callback<String>,
) -> impl IntoView + use<> {
    let size = collection_size(bookmarks, &c.id);
    let (id, name) = (c.id.clone(), c.name.clone());
    let (id_a, id_b, id_c, id_d, id_e, id_f) = (
        id.clone(),
        id.clone(),
        id.clone(),
        id.clone(),
        id.clone(),
        id.clone(),
    );
    let name_a = name.clone();
    let name_b = name.clone();
    let name_c = name.clone();
    view! {
        <li class="collection">
            {move || {
                let is_editing = editing.with(|e| e.as_deref() == Some(id_a.as_str()));
                let is_confirming = confirming.with(|e| e.as_deref() == Some(id_b.as_str()));
                if is_editing {
                    let id_save = id_c.clone();
                    view! {
                        <form class="inline-form" on:submit=move |ev| { ev.prevent_default(); save_rename.run(id_save.clone()); }>
                            <label class="sr-only" for=format!("rename-{id_c}")>{format!("New name for {name_a}")}</label>
                            <input id=format!("rename-{id_c}") type="text" maxlength="60" prop:value=move || rename_to.get()
                                on:input=move |ev| rename_to.set(event_target_value(&ev)) />
                            <button class="btn btn-primary" type="submit">"Save"</button>
                            <button class="btn" type="button" on:click=move |_| editing.set(None)>"Cancel"</button>
                        </form>
                    }.into_any()
                } else if is_confirming {
                    let id_del = id_d.clone();
                    view! {
                        <div class="confirm" role="group" aria-label=format!("Delete {name_b}")>
                            <p>{if size == 1 {
                                format!("Delete the collection “{name_b}”? Its 1 bookmark stays in your list.")
                            } else {
                                format!("Delete the collection “{name_b}”? Its {size} bookmarks stay in your list.")
                            }}</p>
                            <button class="btn btn-danger" type="button" on:click=move |_| delete.run(id_del.clone())>"Delete collection"</button>
                            <button class="btn" type="button" on:click=move |_| confirming.set(None)>"Keep it"</button>
                        </div>
                    }.into_any()
                } else {
                    let (id_edit, name_edit) = (id_e.clone(), name_c.clone());
                    let id_ask = id_f.clone();
                    view! {
                        <a class="collection-link" href=Route::Collection(id_c.clone()).href()>{name_c.clone()}</a>
                        <span class="muted">{format!("{size}")}</span>
                        <span class="collection-actions">
                            <button class="btn btn-quiet" type="button" aria-label=format!("Rename {name_c}")
                                on:click=move |_| { rename_to.set(name_edit.clone()); confirming.set(None); editing.set(Some(id_edit.clone())); }>
                                "Rename"
                            </button>
                            <button class="btn btn-quiet" type="button" aria-label=format!("Delete {name_c}")
                                on:click=move |_| { editing.set(None); confirming.set(Some(id_ask.clone())); }>
                                "Delete"
                            </button>
                        </span>
                    }.into_any()
                }
            }}
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::EntryKind;

    fn bm(n: u32, cols: &[&str]) -> Bookmark {
        let mut b = Bookmark::new(
            EntryId::parse(format!("a/e-{n}")).unwrap(),
            format!("E{n}"),
            EntryKind::Spell,
            n as f64,
        );
        for c in cols {
            b.add_collection(c);
        }
        b
    }

    fn row(entry: &str, title: &str, at: f64, cols: &[&str]) -> Bookmark {
        let mut b = Bookmark::new(EntryId::parse(entry).unwrap(), title, EntryKind::Spell, at);
        for c in cols {
            b.add_collection(c);
        }
        b
    }

    #[test]
    fn scope_filters_by_collection() {
        let all = vec![bm(1, &["x"]), bm(2, &[]), bm(3, &["x", "y"])];
        assert_eq!(in_scope(&all, None).len(), 3);
        assert_eq!(in_scope(&all, Some("x")).len(), 2);
        assert_eq!(in_scope(&all, Some("zzz")).len(), 0);
        assert_eq!(collection_size(&all, "y"), 1);
    }

    #[test]
    fn shown_items_collapse_aliases_and_keep_retired_rows_under_their_own_id() {
        let rows = vec![
            row("legacy/a", "Old", 3.0, &["x"]),
            row("ref/rule/a", "New", 2.0, &["y"]),
            row("legacy/retired", "Gone", 1.0, &[]),
        ];
        let target_of = |stored: &EntryId| {
            if stored.as_str() == "legacy/a" {
                EntryId::parse("ref/rule/a").unwrap()
            } else {
                stored.clone()
            }
        };
        let shown = shown_items(&rows, target_of);
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].entry_id, EntryId::parse("ref/rule/a").unwrap());
        assert_eq!(shown[0].title_snapshot, "Old"); // the newest row of the group
        assert_eq!(shown[0].collection_ids, ["x", "y"]);
        assert_eq!(shown[1].entry_id, EntryId::parse("legacy/retired").unwrap());
        assert_eq!(shown[1].title_snapshot, "Gone");
    }
}
