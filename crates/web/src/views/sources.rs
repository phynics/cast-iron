//! The muted `Sources:` line of an entry and of an attached add-on, and the optional
//! Archives of Nethys link that follows it.
//!
//! The formatting is pure: [`rows`] turns the typed citations into display rows, and
//! [`line_text`] gives the whole line for tests. [`aon_href`] turns a stored `aon` path into
//! its full URL. The view only adds the book and Archives of Nethys links.

use crate::library::Library;
use crate::route::Route;
use crate::views::icons;
use cast_iron_schema::Citation;
use leptos::prelude::*;

/// The Archives of Nethys base. Every stored `aon` value is a path below it.
pub const AON_BASE: &str = "https://aonprd.com/";

/// The full Archives of Nethys URL for a stored path, or `None` when the entry or publication
/// has none, which is the only case that renders no link.
///
/// The corpus stores the path already percent-encoded, so this prefixes it and never
/// re-encodes: encoding again would turn `%20` into `%2520`.
pub fn aon_href(path: Option<&str>) -> Option<String> {
    path.map(|path| format!("{AON_BASE}{path}"))
}

/// The quiet `View on Archives of Nethys` line, in the Metadata role, directly under the
/// Sources line. Nothing when the record has no stored path.
///
/// The link opens in the device browser and fetches nothing itself, so reading stays offline.
/// The wording names the reference site and never implies official status (AGENTS.md rule 11).
pub fn aon_line(path: Option<&str>) -> AnyView {
    let Some(href) = aon_href(path) else {
        return ().into_any();
    };
    view! {
        <p class="aon-line">
            <a class="meta-link aon-link" href=href target="_blank" rel="noopener noreferrer"
                data-testid="aon-link">
                "View on Archives of Nethys"
                {icons::external_link()}
                <span class="sr-only">" (opens in a new tab)"</span>
            </a>
        </p>
    }
    .into_any()
}

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

    #[test]
    fn a_stored_path_becomes_a_full_url_without_re_encoding() {
        assert_eq!(
            aon_href(Some("FeatDisplay.aspx?ItemName=Power%20Attack")).as_deref(),
            Some("https://aonprd.com/FeatDisplay.aspx?ItemName=Power%20Attack")
        );
        assert_eq!(
            aon_href(Some("SourceDisplay.aspx?FixedSource=Core%20Rulebook")).as_deref(),
            Some("https://aonprd.com/SourceDisplay.aspx?FixedSource=Core%20Rulebook")
        );
    }

    #[test]
    fn no_stored_path_means_no_link() {
        assert_eq!(aon_href(None), None);
    }
}
