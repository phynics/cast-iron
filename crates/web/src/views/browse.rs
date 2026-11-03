//! PLACEHOLDER: replaced by the reference-screens task. Categories, sourcebooks, listings.

use crate::library::use_library;
use crate::route::Route;
use cast_iron_schema::EntryKind;
use leptos::prelude::*;

#[component]
pub fn Browse() -> impl IntoView {
    let library = use_library();
    let kinds = library
        .kind_counts()
        .into_iter()
        .map(|(kind, n)| {
            view! { <li><a href=Route::BrowseKind(kind).href()>{format!("{} ({n})", kind.label())}</a></li> }
        })
        .collect_view();
    view! {
        <h1>"Browse"</h1>
        <p>"Placeholder."</p>
        <ul>{kinds}</ul>
    }
}

#[component]
pub fn BrowseKind(kind: EntryKind) -> impl IntoView {
    view! { <h1>{kind.label()}</h1><p>"Placeholder."</p> }
}

#[component]
pub fn BrowseSource(id: String) -> impl IntoView {
    view! { <h1>{id}</h1><p>"Placeholder."</p> }
}
