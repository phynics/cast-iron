//! The breadcrumb (#571): the linked pages between a tab's destination and the current page.
//!
//! A trail never names the tab's own destination (Home, Books, Table, Settings) and never names
//! the page the reader is on. A page with nothing between the two has no trail at all, and
//! [`Crumbs`] then renders no `<nav>`. Every screen builds a `Vec<Crumb>` and hands it here, so
//! the markup, the separator and the empty case live in one place.

use crate::library::Library;
use crate::route::Route;
use leptos::prelude::*;

/// One link of a breadcrumb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    pub route: Route,
}

impl Crumb {
    pub fn new(label: impl Into<String>, route: Route) -> Self {
        Self {
            label: label.into(),
            route,
        }
    }
}

/// The page of a book: its book page, or the old publication page in a library without
/// `books.json` (which has no book pages).
pub fn book_route(library: &Library, source: &str) -> Route {
    if library.has_books() {
        Route::Book {
            source: source.to_string(),
            chapter: None,
        }
    } else {
        Route::BrowseSource(source.to_string())
    }
}

/// The breadcrumb `<nav>`, or nothing for an empty list.
#[component]
pub fn Crumbs(items: Vec<Crumb>) -> impl IntoView {
    (!items.is_empty()).then(|| {
        let items = items
            .into_iter()
            .map(|crumb| view! { <li><a href=crumb.route.href()>{crumb.label}</a></li> })
            .collect_view();
        view! {
            <nav class="crumbs" aria-label="Breadcrumb">
                <ol>{items}</ol>
            </nav>
        }
    })
}
