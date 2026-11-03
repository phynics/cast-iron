use crate::route::Route;
use leptos::prelude::*;

#[component]
pub fn NotFound(hash: String) -> impl IntoView {
    view! {
        <h1>"Page not found"</h1>
        <p>{format!("Nothing is at {hash:?}.")}</p>
        <p><a href=Route::Home.href()>"Go to the home page"</a></p>
    }
}
