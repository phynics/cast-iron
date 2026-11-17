//! The muted `Sources:` line of an entry and of an attached add-on, and the optional
//! Archives of Nethys link that follows it.
//!
//! The formatting is pure: [`rows`] turns the typed citations into display rows, and
//! [`line_text`] gives the whole line for tests. [`aon_href`] turns a stored `aon` path into
//! its full URL. The view only adds the book and Archives of Nethys links.

use crate::library::Library;
use crate::route::Route;
use crate::views::icons;
use cast_iron_schema::{Citation, EntryId, Expansion};
use leptos::prelude::*;

/// The Archives of Nethys base. Every stored `aon` value is a path below it.
pub const AON_BASE: &str = "https://aonprd.com/";

/// The full Archives of Nethys URL for a stored path, or `None` when the entry or publication
/// has none, which is the only case that renders no link. An empty stored value is treated as
/// absent: a link to the bare base would be the AoN homepage, not the record.
///
/// The corpus stores the path already percent-encoded, so this prefixes it and never
/// re-encodes: encoding again would turn `%20` into `%2520`.
pub fn aon_href(path: Option<&str>) -> Option<String> {
    path.filter(|path| !path.is_empty())
        .map(|path| format!("{AON_BASE}{path}"))
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

/// One `Expanded in` credit as the line renders it (#460).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpansionRow {
    pub title: String,
    /// Printed page, when the credit has one.
    pub page: Option<String>,
    /// The expanding book's page. `None` when the catalog does not know the publication or the
    /// library has no book index.
    pub href: Option<String>,
    /// The expanding book is turned off. The credit stays, dimmed, and says so.
    pub inactive: bool,
    /// The `via` entry's address, only while that entry is visible. A hidden `via` entry never
    /// gets a link, so the page number stays plain text.
    pub via_href: Option<String>,
}

/// One row per credit, in corpus order. A publication the catalog does not know keeps its id
/// as the title and gets no link, so no credit is dropped. `via_href` answers for a `via`
/// entry id and returns `None` while the entry is hidden or unknown.
pub fn expansion_rows(
    expanded: &[Expansion],
    lookup: impl Fn(&str) -> Option<(String, Option<String>)>,
    book_active: impl Fn(&str) -> bool,
    via_href: impl Fn(&EntryId) -> Option<String>,
) -> Vec<ExpansionRow> {
    expanded
        .iter()
        .map(|credit| {
            let (title, href) =
                lookup(&credit.publication).unwrap_or_else(|| (credit.publication.clone(), None));
            ExpansionRow {
                title,
                page: credit.page.clone(),
                href,
                inactive: !book_active(&credit.publication),
                via_href: credit.via.as_ref().and_then(&via_href),
            }
        })
        .collect()
}

/// The page number as printed, `None` for a blank or missing page.
fn printed_page(page: Option<&str>) -> Option<&str> {
    page.map(str::trim).filter(|page| !page.is_empty())
}

/// The whole line, for tests: `Expanded in: Monster Summoner's Handbook p. 18`. An inactive
/// book's credit ends with the same `Inactive` label the screen shows.
pub fn expansion_text(rows: &[ExpansionRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let credits = rows
        .iter()
        .map(|row| {
            let page = printed_page(row.page.as_deref())
                .map(|page| format!(" p. {page}"))
                .unwrap_or_default();
            let label = if row.inactive { " · Inactive" } else { "" };
            format!("{}{page}{label}", row.title)
        })
        .collect::<Vec<_>>();
    format!("Expanded in: {}", credits.join("; "))
}

/// The muted `Expanded in:` line under the `Sources:` line. Nothing when the entry has no
/// credit. The book links to its book page; the page number links to the `via` entry only
/// while that entry is visible. An inactive book's credit is dimmed and labelled, and never
/// links to a hidden entry. Typed nodes only (AGENTS.md rule 2).
pub fn expanded_line(expanded: &[Expansion], library: &Library) -> AnyView {
    let rows = expansion_rows(
        expanded,
        |publication| lookup(library, publication),
        |publication| library.book_active(publication),
        |id| {
            library.is_entry_active(id).then(|| {
                Route::Entry {
                    id: id.clone(),
                    anchor: None,
                    book: None,
                    section: None,
                }
                .href()
            })
        },
    );
    if rows.is_empty() {
        return ().into_any();
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
            let page = printed_page(row.page.as_deref()).map(|page| {
                let page = page.to_string();
                let number = match &row.via_href {
                    Some(href) => {
                        view! { <a href=href.clone() data-testid="expanded-via">{page}</a> }
                            .into_any()
                    }
                    None => view! { {page} }.into_any(),
                };
                view! { " p. "{number} }
            });
            let label = row
                .inactive
                .then(|| view! { " · "<span class="tag">"Inactive"</span> });
            view! {
                {separator}
                <span class="expanded-credit" data-inactive=row.inactive.then_some("true")>
                    {title}{page}{label}
                </span>
            }
            .into_any()
        })
        .collect_view();
    view! { <p class="expanded-line" data-testid="expanded-line">"Expanded in: "{parts}</p> }
        .into_any()
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

    #[test]
    fn an_empty_stored_path_means_no_link() {
        assert_eq!(aon_href(Some("")), None);
    }

    fn expansion(publication: &str, page: Option<&str>, via: Option<&str>) -> Expansion {
        Expansion {
            publication: publication.into(),
            page: page.map(String::from),
            via: via.map(|id| EntryId::parse(id).unwrap()),
        }
    }

    fn handbook_lookup(publication: &str) -> Option<(String, Option<String>)> {
        (publication == "monster-summoners-handbook").then(|| {
            (
                "Monster Summoner's Handbook".to_string(),
                Some("#/browse/book/monster-summoners-handbook".to_string()),
            )
        })
    }

    #[test]
    fn a_credit_reads_book_and_page_and_links_the_book() {
        let credits = [expansion("monster-summoners-handbook", Some("18"), None)];
        let rows = expansion_rows(&credits, handbook_lookup, |_| true, |_| None);
        assert_eq!(
            expansion_text(&rows),
            "Expanded in: Monster Summoner's Handbook p. 18"
        );
        assert_eq!(
            rows[0].href.as_deref(),
            Some("#/browse/book/monster-summoners-handbook")
        );
        assert!(!rows[0].inactive);
        assert_eq!(rows[0].via_href, None);
    }

    #[test]
    fn via_links_only_while_the_via_entry_is_visible() {
        let credits = [expansion(
            "monster-summoners-handbook",
            Some("18"),
            Some("ref/rule/simple-templates"),
        )];
        let visible = expansion_rows(
            &credits,
            handbook_lookup,
            |_| true,
            |id| Some(format!("#/entry/{id}")),
        );
        assert_eq!(
            visible[0].via_href.as_deref(),
            Some("#/entry/ref/rule/simple-templates")
        );
        let hidden = expansion_rows(&credits, handbook_lookup, |_| true, |_| None);
        assert_eq!(hidden[0].via_href, None);
        assert_eq!(expansion_text(&visible), expansion_text(&hidden));
    }

    #[test]
    fn an_inactive_book_is_marked_and_a_hidden_via_never_links() {
        let credits = [expansion(
            "monster-summoners-handbook",
            Some("18"),
            Some("ref/rule/simple-templates"),
        )];
        let rows = expansion_rows(&credits, handbook_lookup, |_| false, |_| None);
        assert!(rows[0].inactive);
        assert_eq!(rows[0].via_href, None);
        assert_eq!(
            expansion_text(&rows),
            "Expanded in: Monster Summoner's Handbook p. 18 · Inactive"
        );
    }

    #[test]
    fn an_unknown_book_keeps_its_id_and_a_blank_page_is_omitted() {
        let credits = [
            expansion("unknown-book", None, None),
            expansion("monster-summoners-handbook", Some("  "), None),
        ];
        let rows = expansion_rows(&credits, handbook_lookup, |_| true, |_| None);
        assert_eq!(rows[0].href, None);
        assert_eq!(
            expansion_text(&rows),
            "Expanded in: unknown-book; Monster Summoner's Handbook"
        );
    }

    #[test]
    fn no_credit_means_no_line() {
        assert_eq!(
            expansion_text(&expansion_rows(&[], handbook_lookup, |_| true, |_| None)),
            ""
        );
    }
}
