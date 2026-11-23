//! The breadcrumb (#571): the linked pages between a tab's destination and the current page.
//!
//! A trail never names the tab's own destination (Home, Books, Table, Settings) and never names
//! the page the reader is on. A page with nothing between the two has no trail at all, and
//! [`Crumbs`] then renders no `<nav>`. Every screen builds a `Vec<Crumb>` and hands it here, so
//! the markup, the separator and the empty case live in one place.

use crate::library::Library;
use crate::route::Route;
use crate::views::util::local_resource;
use cast_iron_schema::{EntryId, EntryKind};
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

/// What an entry's trail depends on besides the book it was opened from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFacts {
    pub kind: EntryKind,
    /// A variant, mythic version, revision or other add-on of a base entry.
    pub addon: bool,
    /// The entry's kind list.
    pub kind_crumb: Crumb,
    /// The parent entries, root first.
    pub parents: Vec<Crumb>,
}

/// Whether the book of `?book=` places the entry, as far as the page knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// `books.json` is still loading.
    Pending,
    /// The book places the entry. `chapter` is the chapter page above it, when there is one.
    Placed { chapter: Option<Crumb> },
    /// The book does not place the entry, or its order cannot be read.
    Missing,
}

/// The trail of an entry page (#571), from the page after the tab's destination down to the
/// parent of the entry. `book` is the book that the address names and the library holds.
///
/// - A book that places the entry: `{Book} › {Chapter}`, also for an add-on, so the trail stays
///   steady while the reader steps with Next. The chapter is left out when the entry is that
///   chapter or sits outside every chapter.
/// - Otherwise an add-on has none, a rule has its parent chain, and any other entry has its kind
///   list and then its parent chain.
///
/// Printed sections and a creature's taxonomy are never crumbs.
pub fn entry_trail(facts: &EntryFacts, book: Option<(&Crumb, &Placement)>) -> Vec<Crumb> {
    match book {
        Some((book, Placement::Pending)) => vec![book.clone()],
        Some((book, Placement::Placed { chapter })) => std::iter::once(book.clone())
            .chain(chapter.clone())
            .collect(),
        Some((_, Placement::Missing)) | None => {
            if facts.addon {
                Vec::new()
            } else if facts.kind == EntryKind::Rule {
                facts.parents.clone()
            } else {
                std::iter::once(facts.kind_crumb.clone())
                    .chain(facts.parents.iter().cloned())
                    .collect()
            }
        }
    }
}

/// The breadcrumb of an entry page. With `?book=` that the library holds, `{Book}` shows at
/// once and `› {Chapter}` joins it when `books.json` has loaded, so the line does not jump; a
/// book that turns out not to place the entry gives way to the trail of an entry opened any
/// other way.
#[component]
pub fn EntryCrumbs(
    library: Library,
    id: EntryId,
    book: Option<String>,
    facts: EntryFacts,
) -> impl IntoView {
    let named = book.filter(|_| library.has_books()).and_then(|source| {
        let title = library.source(&source)?.title.clone();
        Some((source, title))
    });
    let Some((source, title)) = named else {
        return view! { <Crumbs items=entry_trail(&facts, None) /> }.into_any();
    };
    let book_crumb = Crumb::new(
        title,
        Route::Book {
            source: source.clone(),
            chapter: None,
        },
    );
    let lookup = {
        let library = library.clone();
        local_resource(async move { library.books().await })
    };
    let trail = move || {
        let placement = lookup.with(|state| match state {
            None => Placement::Pending,
            Some(Ok(Some(books))) => books
                .books()
                .iter()
                .position(|tree| tree.source == source)
                .and_then(|index| library.reading(index))
                .filter(|reading| reading.contains(&id))
                .map_or(Placement::Missing, |reading| Placement::Placed {
                    chapter: reading.chapter(&id).map(|row| {
                        Crumb::new(
                            row.title.clone(),
                            Route::BookChapter {
                                source: source.clone(),
                                chapter: row.id.clone(),
                                anchor: None,
                            },
                        )
                    }),
                }),
            Some(_) => Placement::Missing,
        });
        let items = entry_trail(&facts, Some((&book_crumb, &placement)));
        view! { <Crumbs items /> }
    };
    trail.into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crumb(label: &str) -> Crumb {
        Crumb::new(label, Route::Home)
    }

    fn labels(items: &[Crumb]) -> Vec<&str> {
        items.iter().map(|c| c.label.as_str()).collect()
    }

    fn facts(kind: EntryKind, addon: bool, parents: &[&str]) -> EntryFacts {
        EntryFacts {
            kind,
            addon,
            kind_crumb: crumb(kind.label()),
            parents: parents.iter().map(|p| crumb(p)).collect(),
        }
    }

    fn placed(chapter: Option<&str>) -> Placement {
        Placement::Placed {
            chapter: chapter.map(crumb),
        }
    }

    #[test]
    fn an_entry_opened_from_a_book_shows_the_book_and_its_chapter() {
        let book = crumb("Core Rulebook");
        for f in [
            facts(EntryKind::Spell, false, &[]),
            facts(EntryKind::Feat, true, &[]),
            facts(EntryKind::Rule, false, &["Combat"]),
        ] {
            let trail = entry_trail(&f, Some((&book, &placed(Some("Chapter 9")))));
            assert_eq!(labels(&trail), ["Core Rulebook", "Chapter 9"]);
        }
    }

    #[test]
    fn the_book_stands_alone_while_loading_and_for_the_chapter_row_itself() {
        let book = crumb("Core Rulebook");
        let f = facts(EntryKind::Rule, false, &[]);
        let pending = entry_trail(&f, Some((&book, &Placement::Pending)));
        assert_eq!(labels(&pending), ["Core Rulebook"]);
        let chapter_row = entry_trail(&f, Some((&book, &placed(None))));
        assert_eq!(labels(&chapter_row), ["Core Rulebook"]);
    }

    #[test]
    fn a_book_that_does_not_place_the_entry_gives_the_trail_of_a_direct_visit() {
        let book = crumb("Core Rulebook");
        let f = facts(EntryKind::Feat, false, &[]);
        let missing = entry_trail(&f, Some((&book, &Placement::Missing)));
        assert_eq!(labels(&missing), ["Feats"]);
        assert_eq!(missing, entry_trail(&f, None));
    }

    #[test]
    fn an_add_on_opened_any_other_way_has_no_trail() {
        let f = facts(EntryKind::Feat, true, &["Power Attack"]);
        assert!(entry_trail(&f, None).is_empty());
        let book = crumb("Mythic Adventures");
        assert!(entry_trail(&f, Some((&book, &Placement::Missing))).is_empty());
    }

    #[test]
    fn a_rule_has_its_parent_chain_or_nothing() {
        let with_parent = facts(EntryKind::Rule, false, &["Barbarian"]);
        assert_eq!(labels(&entry_trail(&with_parent, None)), ["Barbarian"]);
        let alone = facts(EntryKind::Rule, false, &[]);
        assert!(entry_trail(&alone, None).is_empty());
    }

    #[test]
    fn any_other_entry_has_its_kind_list_and_then_its_parents() {
        let spell = facts(EntryKind::Spell, false, &[]);
        assert_eq!(labels(&entry_trail(&spell, None)), ["Spells"]);
        let option = facts(EntryKind::ClassOption, false, &["Rogue"]);
        assert_eq!(
            labels(&entry_trail(&option, None)),
            ["Class options", "Rogue"]
        );
    }
}
