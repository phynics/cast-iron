//! PLACEHOLDER: replaced by the reference-screens task. Search box, categories, recent entries.

use crate::library::use_library;
use leptos::prelude::*;

#[component]
pub fn Home() -> impl IntoView {
    let library = use_library();
    view! {
        <h1>"Home"</h1>
        <p>{format!("Placeholder. {} entries are available offline.", library.catalog().entries.len())}</p>
    }
}
