//! PLACEHOLDER: replaced by the reference-screens task. Loads the entry only to prove the seam.

use crate::library::use_library;
use crate::views::util::local_resource;
use cast_iron_schema::EntryId;
use leptos::prelude::*;

#[component]
pub fn Entry(id: EntryId, anchor: Option<String>) -> impl IntoView {
    let library = use_library();
    let shown = id.to_string();
    let entry = local_resource(async move { library.entry(&id).await });
    view! {
        <h1>"Entry"</h1>
        <p>{format!("Placeholder for {shown} (anchor {anchor:?}).")}</p>
        {move || entry.with(|state| match state {
            None => view! { <p>"Loading..."</p> }.into_any(),
            Some(Err(error)) => view! { <p class="error" role="alert">{error.to_string()}</p> }.into_any(),
            Some(Ok(entry)) => view! { <p data-testid="entry-title">{entry.title.clone()}</p> }.into_any(),
        })}
    }
}
