use crate::{js, star_transfer as archive, store};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

#[component]
pub fn StarTransfer(on_imported: Callback<()>) -> impl IntoView {
    let status = RwSignal::new(None::<(bool, String)>);
    let busy = RwSignal::new(false);
    let export = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        let status = status;
        spawn_local(async move {
            let result = async {
                let bookmarks = store::list_bookmarks().await.map_err(|e| e.to_string())?;
                let json = archive::export_archive(&bookmarks).map_err(|e| e.to_string())?;
                let parts = js_sys::Array::new();
                parts.push(&wasm_bindgen::JsValue::from_str(&json));
                let blob =
                    web_sys::Blob::new_with_str_sequence(&parts).map_err(|e| js::describe(&e))?;
                let url = web_sys::Url::create_object_url_with_blob(&blob)
                    .map_err(|e| js::describe(&e))?;
                let window = web_sys::window().ok_or("Browser window is unavailable")?;
                let document = window.document().ok_or("Page document is unavailable")?;
                let anchor: web_sys::HtmlAnchorElement = document
                    .create_element("a")
                    .map_err(|e| js::describe(&e))?
                    .dyn_into()
                    .map_err(|e| js::describe(&e))?;
                anchor.set_href(&url);
                anchor.set_download("cast-iron-stars.json");
                anchor.click();
                web_sys::Url::revoke_object_url(&url).ok();
                Ok::<_, String>(format!(
                    "Exported {} {}.",
                    bookmarks.len(),
                    if bookmarks.len() == 1 {
                        "star"
                    } else {
                        "stars"
                    }
                ))
            }
            .await;
            status.set(Some(match result {
                Ok(s) => (true, s),
                Err(e) => (false, format!("Could not export stars: {e}")),
            }));
            busy.set(false);
        });
    };
    let import = move |ev: web_sys::Event| {
        if busy.get_untracked() {
            return;
        }
        let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(file) = input.files().and_then(|files| files.item(0)) else {
            return;
        };
        input.set_value("");
        busy.set(true);
        let status = status;
        spawn_local(async move {
            let result = async {
                if file.size() > archive::MAX_ARCHIVE_BYTES as f64 {
                    return Err("This file is larger than the 2 MB limit.".to_string());
                }
                let text = wasm_bindgen_futures::JsFuture::from(file.text())
                    .await
                    .map_err(|e| js::describe(&e))?
                    .as_string()
                    .ok_or("The selected file could not be read".to_string())?;
                let records = archive::parse_archive(&text).map_err(|e| e.to_string())?;
                let incoming: Vec<_> = records
                    .into_iter()
                    .map(|s| store::Bookmark::new(s.entry_id, s.title, s.kind, s.added_at))
                    .collect();
                let added = store::merge_bookmarks(&incoming)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>((added, incoming.len() - added))
            }
            .await;
            status.set(Some(match result {
                Ok((added, existing)) => {
                    on_imported.run(());
                    (
                        true,
                        format!(
                            "Imported {added} new {}; {existing} already present.",
                            if added == 1 { "star" } else { "stars" },
                        ),
                    )
                }
                Err(e) => (false, format!("Could not import stars: {e}")),
            }));
            busy.set(false);
        });
    };
    view! {
        <details class="star-transfer">
            <summary>"Move stars"</summary>
            <p>"Export a file here, then import that file in the other browser or Home Screen app. Stars are not synced automatically."</p>
            <div class="star-transfer-actions">
                <button class="btn" type="button" disabled=move || busy.get() on:click=export>"Export stars"</button>
                <input id="star-archive-file" class="star-transfer-file" type="file" accept="application/json,.json" aria-label="Import stars file" on:change=import disabled=move || busy.get() />
            </div>
            {move || status.get().map(|(ok, message)| view! { <p class=if ok { "muted" } else { "error" } role=if ok { "status" } else { "alert" }>{message}</p> })}
        </details>
    }
}
