//! PLACEHOLDER: replaced by the reference-screens task. Runs the query only to prove the seam.

use crate::library::use_library;
use crate::route::Route;
use crate::views::util::local_resource;
use cast_iron_schema::EntryKind;
use cast_iron_search::{Filters, Query};
use leptos::prelude::*;

#[component]
pub fn Search(
    q: String,
    kinds: Vec<EntryKind>,
    sources: Vec<String>,
    facets: Vec<(String, String)>,
) -> impl IntoView {
    let library = use_library();
    let query = Query {
        text: q.clone(),
        filters: Filters {
            kinds: kinds.iter().map(|k| k.slug().to_string()).collect(),
            sources,
            facets,
        },
        limit: 10,
    };
    let hits = local_resource(async move { library.search(&query).await });
    view! {
        <h1>"Search"</h1>
        <p>{format!("Placeholder. Query: {q:?}")}</p>
        {move || hits.with(|state| match state {
            None => view! { <p>"Searching..."</p> }.into_any(),
            Some(Err(error)) => view! { <p class="error" role="alert">{error.to_string()}</p> }.into_any(),
            Some(Ok(list)) => view! {
                <ol>
                    {list.iter().map(|hit| {
                        let href = format!("#/entry/{}", hit.id);
                        view! { <li><a href=href>{hit.title.clone()}</a></li> }
                    }).collect_view()}
                </ol>
            }.into_any(),
        })}
        <p><a href=Route::Home.href()>"Home"</a></p>
    }
}
