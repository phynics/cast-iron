//! The muted `Sources:` line of an entry and of an attached add-on.
//!
//! The formatting is pure: [`rows`] turns the typed citations into display rows, and
//! [`line_text`] gives the whole line for tests. The view only adds the book links.

use crate::library::Library;
use crate::route::Route;
use cast_iron_schema::Citation;
use leptos::prelude::*;

/// One citation as the line renders it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    pub title: String,
    /// Printed page, when the citation has one.
    pub page: Option<String>,
    /// The publication's book page. `None` when the catalog does not know the publication or
    /// the library has no book index.
    pub href: Option<String>,
}

/// One row per citation, in printed order. A publication the catalog does not know keeps its
/// id as the title and gets no link, so no citation is dropped.
pub fn rows(
    sources: &[Citation],
    lookup: impl Fn(&str) -> Option<(String, Option<String>)>,
) -> Vec<SourceRow> {
    sources
        .iter()
        .map(|citation| {
            let (title, href) = lookup(&citation.publication)
                .unwrap_or_else(|| (citation.publication.clone(), None));
            SourceRow {
                title,
                page: citation.page.clone(),
                href,
            }
        })
        .collect()
}

/// `Title p. 221`, or `Title` when the citation has no page.
pub fn citation_text(row: &SourceRow) -> String {
    match row.page.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(page) => format!("{} p. {page}", row.title),
        None => row.title.clone(),
    }
}

/// The whole line, for tests and for callers that only need its text.
pub fn line_text(rows: &[SourceRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    format!(
        "Sources: {}",
        rows.iter()
            .map(citation_text)
            .collect::<Vec<_>>()
            .join("; ")
    )
}

/// A publication's display title and book page. A library without `books.json` has no book
/// page, so every title stays plain text, as the metadata line does.
fn lookup(library: &Library, publication: &str) -> Option<(String, Option<String>)> {
    let source = library.source(publication)?;
    let href = library
        .has_books()
        .then(|| Route::Book {
            source: publication.to_string(),
            chapter: None,
        })
        .map(|route| route.href());
    Some((source.title.clone(), href))
}

/// The muted `Sources:` line. Nothing when the entry has no rows and no fallback.
///
/// `fallback` keeps the pre-field behaviour of an add-on section that never carried a printed
/// citation: a mythic version names its book. A converted add-on uses its own typed rows, and
/// an unconverted body keeps its printed paragraph untouched.
pub fn source_line(citations: &[Citation], library: &Library, fallback: Option<&str>) -> AnyView {
    let rows = rows(citations, |publication| lookup(library, publication));
    if rows.is_empty() {
        return match fallback {
            Some(book) => {
                view! { <p class="sources-line">{format!("Sources: {book}")}</p> }.into_any()
            }
            None => ().into_any(),
        };
    }
    let parts = rows
        .iter()
        .enumerate()
        .map(|(at, row)| {
            let separator = (at > 0).then_some(view! { "; " });
            let title = match &row.href {
                Some(href) => view! { <a href=href.clone()>{row.title.clone()}</a> }.into_any(),
                None => view! { {row.title.clone()} }.into_any(),
            };
            let page = row
                .page
                .as_deref()
                .map(str::trim)
                .filter(|page| !page.is_empty())
                .map(|page| view! { " p. "{page.to_string()} });
            view! { {separator}{title}{page} }.into_any()
        })
        .collect_view();
    view! { <p class="sources-line">"Sources: "{parts}</p> }.into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn citation(publication: &str, page: Option<&str>) -> Citation {
        Citation {
            publication: publication.into(),
            page: page.map(String::from),
        }
    }

    fn lookup(publication: &str) -> Option<(String, Option<String>)> {
        match publication {
            "ultimate-combat" => Some((
                "Ultimate Combat".into(),
                Some("#/browse/book/ultimate-combat".into()),
            )),
            "advanced-players-guide" => Some((
                "Advanced Player's Guide".into(),
                Some("#/browse/book/advanced-players-guide".into()),
            )),
            _ => None,
        }
    }

    #[test]
    fn rows_follow_printed_order_and_keep_every_citation() {
        let sources = [
            citation("ultimate-combat", Some("221")),
            citation("advanced-players-guide", Some("12")),
        ];
        let rows = rows(&sources, lookup);
        assert_eq!(
            line_text(&rows),
            "Sources: Ultimate Combat p. 221; Advanced Player's Guide p. 12"
        );
        assert_eq!(
            rows[0].href.as_deref(),
            Some("#/browse/book/ultimate-combat")
        );
        assert_eq!(
            rows[1].href.as_deref(),
            Some("#/browse/book/advanced-players-guide")
        );
    }

    #[test]
    fn a_missing_page_is_omitted_and_an_unknown_book_keeps_its_id() {
        let unknown = Citation {
            publication: "unknown-book".into(),
            page: None,
        };
        let blank = citation("ultimate-combat", Some("  "));
        let rows = rows(&[unknown, blank], lookup);
        assert_eq!(rows[0].title, "unknown-book");
        assert_eq!(rows[0].href, None);
        assert_eq!(line_text(&rows), "Sources: unknown-book; Ultimate Combat");
    }

    #[test]
    fn no_rows_means_no_line() {
        assert_eq!(line_text(&rows(&[], lookup)), "");
    }
}
