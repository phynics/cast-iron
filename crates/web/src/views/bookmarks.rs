//! PLACEHOLDER: replaced by the reference-screens task. Use `crate::store` for the data.

use leptos::prelude::*;

#[component]
pub fn Bookmarks() -> impl IntoView {
    view! { <h1>"Bookmarks"</h1><p>"Placeholder."</p> }
}

#[component]
pub fn CollectionView(id: String) -> impl IntoView {
    view! { <h1>"Collection"</h1><p>{format!("Placeholder for {id}.")}</p> }
}
