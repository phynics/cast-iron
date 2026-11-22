use crate::route::Route;
use leptos::prelude::*;

/// An address that no page owns. The address itself is not printed: it may carry text that
/// means nothing to the reader.
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <h1>"Page not found"</h1>
        <p class="error" role="alert">"There is nothing at this address. The link may be old or mistyped."</p>
        <p>
            <a href=Route::search("").href()>"Search the rules"</a>" or "
            <a href=Route::Home.href()>"go to Home"</a>"."
        </p>
    }
}
