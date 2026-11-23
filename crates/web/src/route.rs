//! Hash routes. Pure parsing and formatting live here and are tested natively.
//! `use_route`, `navigate`, and the scroll helpers wrap the browser.
//!
//! Grammar (the hash always starts with `#/`):
//!
//! ```text
//! #/                                 Home
//! #/search?q=..&kind=spell&source=core-rulebook&facet=school:evocation
//! #/browse (Home)                    #/browse/kind/<slug>     #/browse/source/<id>
//! #/books                            the books in priority tiers, oldest first inside each tier
//!                                    (one order; `#/books?sort=..` and `#/browse/books` are
//!                                    legacy addresses that the router replaces with `#/books`)
//! #/books/manage                     Manage books: the presets and one checkbox per book
//!                                    (`#/books/active` is the old address and still parses)
//! #/browse/book/<source>?c=~<chapter>&a=<row anchor>
//!                                    a book's contents, with one row marked: the chapter page's
//!                                    id and the row's stable anchor (`a` is absent for the chapter
//!                                    row itself). `?c=3.2`, the 1-based position in the tree, is
//!                                    the old form and still opens the row it named
//! #/browse/book/<source>/<chapter>    one chapter page; the id is the anchor or title slug
//! #/spells                           #/spells/<class>         #/spells/<class>/<level>
//! #/feats/chains                     feats that unlock others
//! #/entry/<entry id>?a=<anchor>&book=<source>&s=<section>   an entry, with the book it was
//!                                    read from; `s` is a class page's section (`features`,
//!                                    `archetypes`, `spells`), which jumps to it (#170)
//! #/bookmarks                        #/collection/<id>   old addresses: they open Home
//! #/quick                            Quick reference: the rules a table looks up mid-play
//! #/settings[?section=updates]       #/settings/diagnostics       #/legal
//! ```

use cast_iron_schema::books::ContentsRow;
use cast_iron_schema::{EntryId, EntryKind};
use leptos::prelude::*;

use crate::book_source::{Retirement, aliased_book_source, retirement};
use crate::class_sections::Section;

/// The archetype list's choices on a class page (`?keep=arcane-bond&with=ref/arc-wizard/x`): the
/// features to keep, in the order chosen, and the one archetype to combine with. They are the
/// route's, like the schools of a spell list: a choice is a history entry that the page reads
/// without being rebuilt (#170, part D).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchFilter {
    /// Heading anchors of the class's features, as the corpus writes them.
    pub keep: Vec<String>,
    pub with: Option<EntryId>,
}

impl ArchFilter {
    pub fn is_empty(&self) -> bool {
        self.keep.is_empty() && self.with.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Home,
    Search {
        q: String,
        kinds: Vec<EntryKind>,
        sources: Vec<String>,
        facets: Vec<(String, String)>,
        /// The folded Book sections row is open (`sections=1`). It rides on the history entry so
        /// that Back from a section returns with the row open. Any new query or filter writes a
        /// route without it, so the row folds again.
        sections: bool,
    },
    /// A kind's A to Z list. `letter` selects one letter group; `None` shows the first group.
    /// `facets` narrow the list in place, with the same `facet=field:value` pairs as search.
    /// `source` limits the list to one publication, so a book page can link its kinds.
    BrowseKind {
        kind: EntryKind,
        letter: Option<char>,
        facets: Vec<(String, String)>,
        source: Option<String>,
    },
    /// Kept for old links. A library that has `books.json` shows the book view here.
    BrowseSource(String),
    /// The publication directory, each a doorway into its contents, in priority tiers.
    Books {
        /// The text of the `Find a book` field, as typed (`?q=`). Empty is no search.
        find: String,
    },
    /// Manage books: the presets and one checkbox per book. It is a route, so a reload and
    /// Back stay in it and Settings can link to it. The Books tab stays highlighted.
    BooksManage {
        /// The text of the `Find a book` field, as typed (`?q=`). Empty is no search.
        find: String,
    },
    /// A shared book set (#408): `#/books/manage?set=<code>`. The page is Manage books, which
    /// asks before it changes anything; the route only carries the code. The text is the code as
    /// written, so a typo is the page's to report, not the router's. Links made while the section
    /// sat on Books (`#/books?set=<code>`) still parse to this route, and the router rewrites
    /// their address to the canonical one.
    BooksSet(String),
    /// An old address of a book that the corpus deleted and that has no replacement (a blog
    /// post, a comic issue). The page is Books with a plain line on top that says so, and the
    /// route only carries the id the address named.
    BookRemoved(String),
    /// One book's contents in original chapter order. `chapter` is the row that the address
    /// marks. New addresses name it by identity, which a pin does not move; an old one holds its
    /// position in the tree (`c=3.2`) and the page resolves it.
    Book {
        source: String,
        chapter: Option<ContentsRow>,
    },
    /// One chapter page of a book. `chapter` is the permanent id: the chapter's anchor or a
    /// slug of its printed title, never its row number. `#/books/<source>/<chapter>` parses
    /// here too, so a hand-written address in the issue's shape still opens the page.
    BookChapter {
        source: String,
        chapter: String,
        /// A section row on the chapter page, when the address came from a book-section result.
        anchor: Option<String>,
    },
    /// Spell lists by class and level.
    Spells,
    /// One class's spell list. `class` is the name that the spells carry, such as `Wizard`.
    SpellList {
        class: String,
        level: Option<u8>,
        /// The schools the list is narrowed to, in the order they were chosen: a spell of any of
        /// them stays. Lower case, as the search index holds them. Empty is every school.
        schools: Vec<String>,
        /// The text of the name field, as typed. Empty is no name filter.
        name: String,
    },
    /// Feats that other feats require, with their chains.
    FeatChains,
    Entry {
        id: EntryId,
        anchor: Option<String>,
        /// The book the reader came from. It survives reload, so previous/next follows the same
        /// book after the page is opened again.
        book: Option<String>,
        /// The part of a class page to open at (`?s=archetypes`). The entry id is the same:
        /// a section is a place on the page, not another entry. Overview has no value.
        section: Option<Section>,
        /// The archetype list's choices (`?keep=` and `?with=`). Empty on every other page.
        arch: ArchFilter,
    },
    /// Quick reference (#437): chips to the rules a table looks up mid-play. The Table tab.
    Quick,
    Settings,
    /// Timings and storage that this device recorded, for a bug report (#423). It belongs to
    /// Settings in the navigation.
    Diagnostics,
    Legal,
    /// Holds the hash that did not match any route.
    NotFound(String),
}

impl Route {
    /// A search route with only query text.
    pub fn search(q: impl Into<String>) -> Self {
        Route::Search {
            q: q.into(),
            kinds: Vec::new(),
            sources: Vec::new(),
            facets: Vec::new(),
            sections: false,
        }
    }

    /// Books with no search.
    pub fn books() -> Self {
        Route::Books {
            find: String::new(),
        }
    }

    /// Manage books with no search.
    pub fn books_manage() -> Self {
        Route::BooksManage {
            find: String::new(),
        }
    }

    /// A class's spell list, whole or at one level, with every school.
    pub fn spell_list(class: impl Into<String>, level: Option<u8>) -> Self {
        Route::SpellList {
            class: class.into(),
            level,
            schools: Vec::new(),
            name: String::new(),
        }
    }

    pub fn entry(id: EntryId) -> Self {
        Route::Entry {
            id,
            anchor: None,
            book: None,
            section: None,
            arch: Default::default(),
        }
    }

    /// An entry opened from a book or a chapter page, carrying the book into the URL.
    pub fn entry_in_book(id: EntryId, source: impl Into<String>) -> Self {
        Route::Entry {
            id,
            anchor: None,
            book: Some(source.into()),
            section: None,
            arch: Default::default(),
        }
    }

    /// A kind's A to Z list, opening its first letter group.
    pub fn browse_kind(kind: EntryKind) -> Self {
        Route::BrowseKind {
            kind,
            letter: None,
            facets: Vec::new(),
            source: None,
        }
    }

    /// A kind's A to Z list with one letter group open.
    pub fn browse_kind_letter(kind: EntryKind, letter: char) -> Self {
        Route::BrowseKind {
            kind,
            letter: Some(letter),
            facets: Vec::new(),
            source: None,
        }
    }

    /// A kind's A to Z list narrowed by `facets`, opening its first letter group.
    pub fn browse_kind_facets(kind: EntryKind, facets: Vec<(String, String)>) -> Self {
        Route::BrowseKind {
            kind,
            letter: None,
            facets,
            source: None,
        }
    }

    /// The same list with a letter group open, keeping the facets and the source.
    pub fn with_letter(&self, letter: char) -> Self {
        match self {
            Route::BrowseKind {
                kind,
                facets,
                source,
                ..
            } => Route::BrowseKind {
                kind: *kind,
                letter: Some(letter),
                facets: facets.clone(),
                source: source.clone(),
            },
            other => other.clone(),
        }
    }

    /// The same list limited to `source`, keeping the letter group and the facets. `None` removes
    /// the book filter and keeps the rest, which is what the pill on the list does.
    pub fn with_source(&self, source: Option<String>) -> Self {
        match self {
            Route::BrowseKind {
                kind,
                letter,
                facets,
                ..
            } => Route::BrowseKind {
                kind: *kind,
                letter: *letter,
                facets: facets.clone(),
                source,
            },
            other => other.clone(),
        }
    }

    /// Parses `location.hash`, with or without the leading `#`.
    pub fn parse(hash: &str) -> Route {
        let raw = hash.strip_prefix('#').unwrap_or(hash);
        let not_found = || Route::NotFound(hash.to_string());
        if raw.is_empty() || raw == "/" {
            return Route::Home;
        }
        let Some(rest) = raw.strip_prefix('/') else {
            return not_found();
        };
        let (path, query) = match rest.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (rest, None),
        };
        let path = path.trim_end_matches('/');
        let segments: Vec<&str> = path.split('/').collect();
        match segments.as_slice() {
            [""] => Route::Home,
            ["search"] => parse_search(query),
            ["browse"] => Route::Home,
            ["browse", "kind", slug] => percent_decode(slug)
                .and_then(|slug| EntryKind::from_slug(&slug))
                .map_or_else(not_found, |kind| {
                    if kind == EntryKind::Rule {
                        Route::books()
                    } else {
                        Route::BrowseKind {
                            kind,
                            letter: query
                                .and_then(|q| query_pairs(q).find(|(key, _)| key == "l"))
                                .and_then(|(_, value)| parse_letter(&value)),
                            facets: parse_facets(query),
                            source: parse_source(query),
                        }
                    }
                }),
            ["browse", "source", id] => match percent_decode(id) {
                Some(id) if retirement(&id) == Some(Retirement::Books) => Route::BookRemoved(id),
                Some(id) if !id.is_empty() => {
                    Route::BrowseSource(aliased_book_source(&id).to_string())
                }
                _ => not_found(),
            },
            // `active` is the old address of the page; both parse to the same route. A `set`
            // query is a shared book set (#408): the canonical address is under `manage`, and
            // `#/books?set=` (where the link pointed first) means the same.
            ["books", "manage"] | ["books", "active"] => match shared_set(query) {
                Some(code) if segments == ["books", "manage"] => Route::BooksSet(code),
                _ => Route::BooksManage {
                    find: parse_text(query, "q"),
                },
            },
            // There is one order. A link from when Books had sort options opens the list.
            ["books"] | ["browse", "books"] => match shared_set(query) {
                Some(code) if segments == ["books"] => Route::BooksSet(code),
                _ => Route::Books {
                    find: parse_text(query, "q"),
                },
            },
            ["browse", "book", id] => match percent_decode(id) {
                Some(source) if retirement(&source).is_some() => retired_book_route(&source),
                Some(source) if !source.is_empty() => Route::Book {
                    source: aliased_book_source(&source).to_string(),
                    chapter: query.and_then(parse_contents_row),
                },
                _ => not_found(),
            },
            // A chapter page. The id is one path segment, so it is percent-encoded as a whole;
            // the issue's `#/books/<source>/<chapter>` shape opens the same page.
            ["browse", "book", source, chapter] | ["books", source, chapter] => {
                match (percent_decode(source), percent_decode(chapter)) {
                    (Some(source), Some(_)) if retirement(&source).is_some() => {
                        retired_book_route(&source)
                    }
                    (Some(source), Some(chapter)) if !source.is_empty() && !chapter.is_empty() => {
                        Route::BookChapter {
                            source: aliased_book_source(&source).to_string(),
                            chapter,
                            anchor: query
                                .and_then(|q| query_pairs(q).find(|(k, _)| k == "a"))
                                .map(|(_, v)| v)
                                .filter(|v| !v.is_empty()),
                        }
                    }
                    _ => not_found(),
                }
            }
            ["spells"] => Route::Spells,
            ["spells", class] => match percent_decode(class) {
                Some(class) if !class.is_empty() => Route::SpellList {
                    class,
                    level: None,
                    schools: parse_schools(query),
                    name: parse_text(query, "name"),
                },
                _ => not_found(),
            },
            ["spells", class, level] => {
                let level = (!level.is_empty() && level.bytes().all(|b| b.is_ascii_digit()))
                    .then(|| level.parse::<u8>().ok())
                    .flatten();
                match (percent_decode(class), level) {
                    (Some(class), Some(level)) if !class.is_empty() => Route::SpellList {
                        class,
                        level: Some(level),
                        schools: parse_schools(query),
                        name: parse_text(query, "name"),
                    },
                    _ => not_found(),
                }
            }
            ["feats", "chains"] => Route::FeatChains,
            ["entry", id @ ..] if !id.is_empty() => {
                let Some(id) = percent_decode(&id.join("/")).and_then(|s| EntryId::parse(s).ok())
                else {
                    return not_found();
                };
                let anchor = query
                    .and_then(|q| query_pairs(q).find(|(k, _)| k == "a"))
                    .map(|(_, v)| v)
                    .filter(|v| !v.is_empty());
                let book = query
                    .and_then(|q| query_pairs(q).find(|(k, _)| k == "book"))
                    .map(|(_, v)| aliased_book_source(&v).to_string())
                    .filter(|v| !v.is_empty());
                let section = query
                    .and_then(|q| query_pairs(q).find(|(k, _)| k == "s"))
                    .and_then(|(_, v)| Section::from_slug(&v));
                let mut keep: Vec<String> = Vec::new();
                for (_, value) in query
                    .into_iter()
                    .flat_map(query_pairs)
                    .filter(|(k, _)| k == "keep")
                {
                    if !value.is_empty() && !keep.contains(&value) {
                        keep.push(value);
                    }
                }
                let with = query
                    .and_then(|q| query_pairs(q).find(|(k, _)| k == "with"))
                    .and_then(|(_, v)| EntryId::parse(v).ok());
                Route::Entry {
                    id,
                    anchor,
                    book,
                    section,
                    arch: ArchFilter { keep, with },
                }
            }
            // Legacy saved-entry URLs now land on Home, where all stars are listed.
            ["bookmarks"] | ["collection", _] => Route::Home,
            ["quick"] => Route::Quick,
            ["settings"] => Route::Settings,
            ["settings", "diagnostics"] => Route::Diagnostics,
            ["legal"] => Route::Legal,
            _ => not_found(),
        }
    }

    /// The hash for an anchor `href`, starting with `#/`.
    pub fn href(&self) -> String {
        match self {
            Route::Home => "#/".into(),
            Route::Search {
                q,
                kinds,
                sources,
                facets,
                sections,
            } => {
                let mut pairs: Vec<String> = Vec::new();
                if !q.is_empty() {
                    pairs.push(format!("q={}", percent_encode(q)));
                }
                for kind in kinds {
                    pairs.push(format!("kind={}", kind.slug()));
                }
                for source in sources {
                    pairs.push(format!("source={}", percent_encode(source)));
                }
                pairs.extend(facets.iter().map(facet_pair));
                if *sections {
                    pairs.push("sections=1".into());
                }
                if pairs.is_empty() {
                    "#/search".into()
                } else {
                    format!("#/search?{}", pairs.join("&"))
                }
            }
            Route::BrowseKind {
                kind,
                letter,
                facets,
                source,
            } => {
                let mut pairs: Vec<String> = Vec::new();
                if let Some(source) = source {
                    pairs.push(format!("source={}", percent_encode(source)));
                }
                pairs.extend(facets.iter().map(facet_pair));
                if let Some(letter) = letter {
                    pairs.push(format!("l={}", percent_encode(&letter.to_string())));
                }
                if pairs.is_empty() {
                    format!("#/browse/kind/{}", kind.slug())
                } else {
                    format!("#/browse/kind/{}?{}", kind.slug(), pairs.join("&"))
                }
            }
            Route::BrowseSource(id) => format!("#/browse/source/{}", percent_encode(id)),
            Route::Books { find } => with_text("#/books", "q", find),
            Route::BooksManage { find } => with_text("#/books/manage", "q", find),
            Route::BooksSet(code) => format!("#/books/manage?set={}", percent_encode(code)),
            Route::BookRemoved(source) => format!("#/browse/book/{}", percent_encode(source)),
            Route::Book { source, chapter } => {
                let base = format!("#/browse/book/{}", percent_encode(source));
                match chapter {
                    Some(ContentsRow::Position(path)) if !path.is_empty() => {
                        format!("{base}?c={}", format_chapter_path(path))
                    }
                    Some(ContentsRow::Stable {
                        chapter,
                        anchor: None,
                    }) => format!("{base}?c=~{}", percent_encode(chapter)),
                    Some(ContentsRow::Stable {
                        chapter,
                        anchor: Some(anchor),
                    }) => format!(
                        "{base}?c=~{}&a={}",
                        percent_encode(chapter),
                        percent_encode(anchor)
                    ),
                    _ => base,
                }
            }
            Route::BookChapter {
                source,
                chapter,
                anchor,
            } => match anchor {
                Some(anchor) => format!(
                    "#/browse/book/{}/{}?a={}",
                    percent_encode(source),
                    percent_encode(chapter),
                    percent_encode(anchor)
                ),
                None => format!(
                    "#/browse/book/{}/{}",
                    percent_encode(source),
                    percent_encode(chapter)
                ),
            },
            Route::Spells => "#/spells".into(),
            Route::SpellList {
                class,
                level,
                schools,
                name,
            } => {
                let mut href = match level {
                    Some(level) => format!("#/spells/{}/{level}", percent_encode(class)),
                    None => format!("#/spells/{}", percent_encode(class)),
                };
                for (i, school) in schools.iter().enumerate() {
                    href.push(if i == 0 { '?' } else { '&' });
                    href.push_str(&format!("school={}", percent_encode(school)));
                }
                if !name.trim().is_empty() {
                    href.push(if schools.is_empty() { '?' } else { '&' });
                    href.push_str(&format!("name={}", percent_encode(name)));
                }
                href
            }
            Route::FeatChains => "#/feats/chains".into(),
            Route::Entry {
                id,
                anchor,
                book,
                section,
                arch,
            } => {
                let mut pairs: Vec<String> = Vec::new();
                if let Some(anchor) = anchor {
                    pairs.push(format!("a={}", percent_encode(anchor)));
                }
                if let Some(book) = book {
                    pairs.push(format!("book={}", percent_encode(book)));
                }
                if let Some(section) = section {
                    pairs.push(format!("s={}", section.slug()));
                }
                for anchor in &arch.keep {
                    pairs.push(format!("keep={}", percent_encode(anchor)));
                }
                if let Some(with) = &arch.with {
                    pairs.push(format!("with={}", percent_encode(with.as_str())));
                }
                if pairs.is_empty() {
                    format!("#/entry/{id}")
                } else {
                    format!("#/entry/{id}?{}", pairs.join("&"))
                }
            }
            Route::Quick => "#/quick".into(),
            Route::Settings => "#/settings".into(),
            Route::Diagnostics => "#/settings/diagnostics".into(),
            Route::Legal => "#/legal".into(),
            Route::NotFound(hash) if hash.starts_with("#/") => hash.clone(),
            Route::NotFound(_) => "#/".into(),
        }
    }

    /// The archetype list's choices of this route; empty for every other route.
    pub fn arch(&self) -> ArchFilter {
        match self {
            Route::Entry { arch, .. } => arch.clone(),
            _ => ArchFilter::default(),
        }
    }

    /// Whether `other` is a choice of the archetype list on the entry page that `self` shows: the
    /// same entry and book, other choices, no heading anchor, and the section the page is at or
    /// the Archetypes stop that the list's own links name. The page reads the choices from the route
    /// and is not rebuilt, so the frame leaves scroll and focus alone (the reader is looking at the
    /// list). The first choice from a bare address or from an `?a=` address is one too: the list's
    /// links add the stop and drop the anchor, and that must not read as a jump to the section.
    /// Back from that first choice to the bare address is one as well.
    pub fn only_arch_differs(&self, other: &Route) -> bool {
        match (self, other) {
            (
                Route::Entry {
                    id,
                    book,
                    section,
                    arch,
                    ..
                },
                Route::Entry {
                    id: other_id,
                    anchor: None,
                    book: other_book,
                    section: other_section,
                    arch: other_arch,
                },
            ) => {
                id == other_id
                    && book == other_book
                    && arch != other_arch
                    && (section == other_section
                        || *other_section == Some(Section::Archetypes)
                        || (*section == Some(Section::Archetypes) && other_section.is_none()))
            }
            _ => false,
        }
    }

    /// The section of a class page that this route opens at.
    pub fn section(&self) -> Option<Section> {
        match self {
            Route::Entry { section, .. } => *section,
            _ => None,
        }
    }

    /// Whether moving from `self` to `other` is a jump inside the entry page that is on screen:
    /// the same entry and book, `other` names no heading anchor, and the address differs, which
    /// is another section or an `?a=` that the jump clears. The page stays, so the move scrolls it
    /// and does not rebuild it, and focus stays where the reader put it or moves to the section's
    /// heading. A move to an `?a=` address is an ordinary navigation.
    pub fn is_section_jump(&self, other: &Route) -> bool {
        match (self, other) {
            (
                Route::Entry { id, book, .. },
                Route::Entry {
                    id: other_id,
                    anchor: None,
                    book: other_book,
                    ..
                },
            ) => id == other_id && book == other_book && self != other,
            _ => false,
        }
    }

    /// The anchor to scroll to after the route renders.
    pub fn anchor(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Route::Entry { anchor, .. } | Route::BookChapter { anchor, .. } => {
                anchor.as_deref().map(std::borrow::Cow::Borrowed)
            }
            _ => None,
        }
    }
}

/// The Contents row that a book address marks. A `c` that starts with `~` names a chapter page by
/// id and, with `a`, a row on it. Any other `c` is the old position, `3.2`. A chapter id that is
/// digits and dots is still unambiguous, because the id always follows the `~`.
fn parse_contents_row(query: &str) -> Option<ContentsRow> {
    let (_, value) = query_pairs(query).find(|(k, _)| k == "c")?;
    let Some(chapter) = value.strip_prefix('~') else {
        return parse_chapter_path(&value).map(ContentsRow::Position);
    };
    if chapter.is_empty() {
        return None;
    }
    let anchor = query_pairs(query)
        .find(|(k, _)| k == "a")
        .map(|(_, v)| v)
        .filter(|v| !v.is_empty());
    Some(ContentsRow::Stable {
        chapter: chapter.to_string(),
        anchor,
    })
}

/// `3.2` for the 0-based path `[2, 1]`: the URL numbers chapters from 1, as a reader counts.
pub fn format_chapter_path(path: &[usize]) -> String {
    path.iter()
        .map(|i| (i + 1).to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// The inverse of [`format_chapter_path`]. `None` for an empty or malformed path.
pub fn parse_chapter_path(text: &str) -> Option<Vec<usize>> {
    let path: Option<Vec<usize>> = text
        .split('.')
        .map(|part| {
            let ok =
                !part.is_empty() && part.len() <= 5 && part.bytes().all(|b| b.is_ascii_digit());
            ok.then(|| part.parse::<usize>().ok())
                .flatten()
                .and_then(|n| n.checked_sub(1))
        })
        .collect();
    path.filter(|p| !p.is_empty())
}

/// A letter group selector: one ASCII letter in either case, or `#` for digits and symbols.
fn parse_letter(value: &str) -> Option<char> {
    let mut chars = value.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    if chars.next().is_some() {
        return None;
    }
    (letter.is_ascii_uppercase() || letter == '#').then_some(letter)
}

pub use cast_iron_schema::books::toc_id;

fn parse_search(query: Option<&str>) -> Route {
    let (mut q, mut kinds, mut sources) = (String::new(), Vec::new(), Vec::new());
    let mut sections = false;
    for (key, value) in query.into_iter().flat_map(query_pairs) {
        match key.as_str() {
            "q" => q = value,
            "sections" => sections = value == "1",
            "kind" => kinds.extend(EntryKind::from_slug(&value)),
            // A retired book id filters by the book that holds its entries now, so an old link
            // still matches entries by membership.
            "source" => sources.push(aliased_book_source(&value).to_string()),
            _ => {}
        }
    }
    Route::Search {
        q,
        kinds,
        sources,
        facets: parse_facets(query),
        sections,
    }
}

/// `facet=field:value` as it appears in a URL.
fn facet_pair((field, value): &(String, String)) -> String {
    format!("facet={}:{}", percent_encode(field), percent_encode(value))
}

/// The `school=<name>` pairs of a spell list's query: lower case, in order, each school once.
fn parse_schools(query: Option<&str>) -> Vec<String> {
    let mut schools: Vec<String> = Vec::new();
    for (key, value) in query.into_iter().flat_map(query_pairs) {
        let value = value.trim().to_lowercase();
        if key == "school" && !value.is_empty() && !schools.contains(&value) {
            schools.push(value);
        }
    }
    schools
}

/// The `<key>=<text>` of a query: the first one, empty when it has none or holds only blanks.
/// Spell lists keep their name filter in `name`, and Books and Manage books their find in `q`.
fn parse_text(query: Option<&str>, key: &str) -> String {
    query
        .into_iter()
        .flat_map(query_pairs)
        .find(|(found, _)| found == key)
        .map(|(_, value)| value)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_default()
}

/// `base` with `?<key>=<text>` after it, or `base` alone when the text is blank.
fn with_text(base: &str, key: &str, text: &str) -> String {
    if text.trim().is_empty() {
        base.to_string()
    } else {
        format!("{base}?{key}={}", percent_encode(text))
    }
}

/// The `source=<id>` of a list page's query, when it has one.
/// The route for an address of a book tree that the corpus deleted. A chapter address uses the
/// book's destination too, because the old chapters did not carry over.
fn retired_book_route(source: &str) -> Route {
    match retirement(source) {
        Some(Retirement::Book(book)) => Route::Book {
            source: book.to_string(),
            chapter: None,
        },
        _ => Route::BookRemoved(source.to_string()),
    }
}

fn parse_source(query: Option<&str>) -> Option<String> {
    query
        .and_then(|q| query_pairs(q).find(|(key, _)| key == "source"))
        .map(|(_, value)| aliased_book_source(&value).to_string())
        .filter(|value| !value.is_empty())
}

/// The `facet=field:value` pairs of a query string, in order. Search and a list page read them
/// the same way.
fn parse_facets(query: Option<&str>) -> Vec<(String, String)> {
    let mut facets = Vec::new();
    // The field and value are encoded separately, so split on the first raw ':' before decoding.
    for raw in query.into_iter().flat_map(|q| q.split('&')) {
        if let Some(raw) = raw.strip_prefix("facet=")
            && let Some((field, value)) = raw.split_once(':')
            && let (Some(field), Some(value)) = (percent_decode(field), percent_decode(value))
            && !value.trim().is_empty()
        {
            let value = match field.as_str() {
                "school" => legacy_school(&value),
                // The index holds sizes in lower case; an older link may say `Huge`.
                "size" => value.to_lowercase(),
                _ => value,
            };
            facets.push((field, value));
        }
    }
    facets
}

/// A `school` facet value as the index holds it: the school word alone. Links shared before
/// the facet dropped subschools, such as `school:conjuration (calling)`, land on `conjuration`.
fn legacy_school(value: &str) -> String {
    let end = value.find(['(', '[']).unwrap_or(value.len());
    let school = value[..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if school.is_empty() {
        value.to_string()
    } else {
        school.to_lowercase()
    }
}

/// The code of a share link (`set=<code>`), unless it is empty.
fn shared_set(query: Option<&str>) -> Option<String> {
    query
        .and_then(|q| query_pairs(q).find(|(key, _)| key == "set"))
        .map(|(_, code)| code)
        .filter(|code| !code.trim().is_empty())
}

/// Decoded `key=value` pairs. Pairs with undecodable escapes are skipped.
fn query_pairs(query: &str) -> impl Iterator<Item = (String, String)> + '_ {
    query
        .split('&')
        .filter(|p| !p.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((percent_decode(key)?, percent_decode(value)?))
        })
}

/// Encodes everything except RFC 3986 unreserved characters.
pub fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Decodes `%XX` escapes and `+` as a space. Returns `None` for a bad escape or invalid UTF-8.
pub fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = value.get(i + 1..i + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// The current route, provided once by [`provide_route`].
pub fn use_route() -> ReadSignal<Route> {
    expect_context::<RouteContext>().read
}

#[derive(Clone, Copy)]
struct RouteContext {
    read: ReadSignal<Route>,
}

/// Creates the route signal from `location.hash` and keeps it in step with `hashchange`.
/// Call once, in the root component.
pub fn provide_route() {
    let (read, write) = signal(read_location_route());
    // Back and forward fire `hashchange` too, so the browser's history stays the only history.
    let handle = window_event_listener(leptos::ev::hashchange, move |_| {
        crate::focus_guard::begin_navigation();
        write.set(read_location_route());
    });
    on_cleanup(move || handle.remove());
    provide_context(RouteContext { read });
    // `replace` runs from async tasks that have no reactive owner, where `use_context` fails.
    ROUTE_WRITE.with(|cell| cell.set(Some(write)));
}

/// Retired URLs recover through the canonical destination without another history step.
fn read_location_route() -> Route {
    let hash = current_hash();
    let route = Route::parse(&hash);
    let legacy = hash.starts_with("#/bookmarks")
        || hash.starts_with("#/collection/")
        || hash.starts_with("#/browse/kind/rule")
        || hash.starts_with("#/browse/books")
        // The page that manages books used to live at `#/books/active`.
        || hash.starts_with("#/books/active")
        // Books had sort links; the one order is the list, and the address says so. A share link
        // from when the section sat on Books opens Manage books, and the address says so.
        || hash.starts_with("#/books?");
    if legacy
        && hash != route.href()
        && let Some(history) = web_sys::window().and_then(|window| window.history().ok())
    {
        replace_url_keeping_state(&history, &route.href());
    }
    route
}

thread_local! {
    static ROUTE_WRITE: std::cell::Cell<Option<WriteSignal<Route>>> = const { std::cell::Cell::new(None) };
}

pub fn current_hash() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
}

/// The DOM id of the Updates section on Settings.
pub const UPDATES_SECTION_ID: &str = "set-updates";

/// Settings, opened on its Updates section. The section is not a route of its own, so the query
/// names the place on the screen that the header action opens. [`Route::parse`] reads it as
/// [`Route::Settings`], which keeps every link, the history, and the route signal the same.
pub fn settings_updates_href() -> &'static str {
    "#/settings?section=updates"
}

/// Whether a Settings hash asks for the Updates section.
pub fn wants_updates_section(hash: &str) -> bool {
    Route::parse(hash) == Route::Settings
        && hash
            .split_once('?')
            .is_some_and(|(_, query)| query.split('&').any(|pair| pair == "section=updates"))
}

/// Pushes a history entry for a hash that [`Route`] does not fully describe, such as
/// [`settings_updates_href`].
pub fn navigate_hash(hash: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(hash);
    }
}

/// Pushes a history entry, like clicking an anchor with `route.href()`.
pub fn navigate(route: &Route) {
    navigate_hash(&route.href());
}

/// Pushes a history entry for `route` and updates the route signal at once, so the page that
/// asked for it reads the new state in the same turn. A plain hash change reaches the signal one
/// event later, which a control that shows the route's state would miss. Back and Forward fire
/// `hashchange` as usual.
///
/// The new entry starts with no history state, on purpose. Its fields describe one entry:
/// `scrollId` is that entry's scroll offset, and `quickSearch` marks the entry that Quick search
/// pushed. Copying them (as [`push_flagged_entry`] does for the page that is on screen) would
/// give two entries one scroll offset and mark this one as a Quick search entry.
pub fn push(route: &Route) {
    push_with_state(route, &wasm_bindgen::JsValue::NULL);
}

/// [`push`], with `field` set to true in the new entry's history state. The flag says how the
/// reader came to this one entry; it is on the entry the moment it exists, so a click that
/// pushes nothing (one that asks for another tab) sets nothing, and no later page can read a
/// request that this entry never made.
pub fn push_flagged(route: &Route, field: &str) {
    let state = state_with(
        &wasm_bindgen::JsValue::NULL,
        field,
        &wasm_bindgen::JsValue::TRUE,
    );
    push_with_state(route, &state);
}

fn push_with_state(route: &Route, state: &wasm_bindgen::JsValue) {
    let Some(history) = web_sys::window().and_then(|w| w.history().ok()) else {
        return;
    };
    if history
        .push_state_with_url(state, "", Some(&route.href()))
        .is_err()
    {
        navigate(route);
        return;
    }
    if let Some(write) = ROUTE_WRITE.with(|cell| cell.get()) {
        crate::focus_guard::begin_navigation();
        write.set(read_location_route());
    }
}

/// Replaces the current history entry, such as while a search box types.
/// `replaceState` fires no `hashchange`, so this updates the route signal itself.
pub fn replace(route: &Route) {
    replace_with_state(route, true);
}

/// Replaces the current history entry with a different page. The entry starts clean, so the
/// page gets its own scroll offset instead of the one of the page it replaces. Quick search
/// uses this when it leaves its own entry for a result.
pub fn replace_as_new_page(route: &Route) {
    replace_with_state(route, false);
}

fn replace_with_state(route: &Route, keep_state: bool) {
    let Some(history) = web_sys::window().and_then(|w| w.history().ok()) else {
        return;
    };
    let state = if keep_state {
        history.state().unwrap_or(wasm_bindgen::JsValue::NULL)
    } else {
        wasm_bindgen::JsValue::NULL
    };
    let _ = history.replace_state_with_url(&state, "", Some(&route.href()));
    if let Some(write) = ROUTE_WRITE.with(|cell| cell.get()) {
        crate::focus_guard::begin_navigation();
        write.set(read_location_route());
    }
}

/// Rewrites the URL of the current entry and keeps its history state.
pub fn replace_url_keeping_state(history: &web_sys::History, url: &str) {
    let state = history.state().unwrap_or(wasm_bindgen::JsValue::NULL);
    let _ = history.replace_state_with_url(&state, "", Some(url));
}

// ---- history state ---------------------------------------------------------------------
//
// One object describes every history entry the app touches. Each owner reads and writes only its
// own field and copies the others: `scrollId` belongs to the scroll memory below, and
// `quickSearch` marks the entry that Quick search pushes. `{ scrollId: 7, quickSearch: true }`
// is one entry that has both.

const SCROLL_ID_FIELD: &str = "scrollId";
/// The field that marks the history entry that Quick search pushed.
pub const QUICK_SEARCH_FIELD: &str = "quickSearch";
/// The field that marks the history entry that holds a share link (#504): a number drawn once
/// for the entry (see [`mark_current_entry`]). Only that entry has it.
pub const SHARE_LINK_FIELD: &str = "shareLink";
/// The field of an entry that Quick search pushed over a share link's entry: the link entry's
/// mark. The layer keeps the link's address, so Forward into it after the question was answered
/// must not read that address as a new link. It is a separate field, so a task that finds
/// [`SHARE_LINK_FIELD`] on the entry on screen still knows that entry is the link's own.
pub const SHARE_LINK_LAYER_FIELD: &str = "shareLinkLayer";
/// How many history entries keep a scroll offset in memory.
const SCROLL_MEMORY_LIMIT: usize = 100;

fn state_field(state: &wasm_bindgen::JsValue, field: &str) -> Option<wasm_bindgen::JsValue> {
    if !state.is_object() {
        return None;
    }
    js_sys::Reflect::get(state, &field.into())
        .ok()
        .filter(|value| !value.is_undefined())
}

/// A copy of `state` with one field set. Any other field is kept; a state that is not an
/// object (none, or a string from an older build) starts an empty object.
fn state_with(
    state: &wasm_bindgen::JsValue,
    field: &str,
    value: &wasm_bindgen::JsValue,
) -> js_sys::Object {
    let copy = js_sys::Object::new();
    if let Some(source) = wasm_bindgen::JsCast::dyn_ref::<js_sys::Object>(state) {
        let _ = js_sys::Object::assign(&copy, source);
    }
    let _ = js_sys::Reflect::set(&copy, &field.into(), value);
    copy
}

fn current_state() -> wasm_bindgen::JsValue {
    web_sys::window()
        .and_then(|w| w.history().ok())
        .and_then(|history| history.state().ok())
        .unwrap_or(wasm_bindgen::JsValue::NULL)
}

/// True when the current history entry carries `field` set to true.
pub fn current_entry_has_flag(field: &str) -> bool {
    state_field(&current_state(), field).is_some_and(|value| value.is_truthy())
}

/// The mark that `field` holds on the current history entry, if the entry has one.
pub fn current_entry_mark(field: &str) -> Option<u32> {
    state_field(&current_state(), field)
        .and_then(|value| value.as_f64())
        .map(|value| value as u32)
}

/// Gives the current history entry a mark in `field`, or returns the one it already has. A mark
/// tells two entries apart that have the same route, which the address cannot: Back from a share
/// link lands on an entry whose address is the one the link was rewritten to. It lives in the
/// entry's history state, so Back, Forward and a reload keep it, and it is never zero.
pub fn mark_current_entry(field: &str) -> u32 {
    if let Some(mark) = current_entry_mark(field) {
        return mark;
    }
    let mark = (js_sys::Math::random() * f64::from(u32::MAX - 1)) as u32 + 1;
    if let Some(history) = web_sys::window().and_then(|w| w.history().ok()) {
        let state = state_with(&current_state(), field, &f64::from(mark).into());
        let _ = history.replace_state(&state, "");
    }
    mark
}

/// Pushes a history entry for the page that is on screen, flagged with `field`. The entry keeps
/// the scroll id of the page, so Back to it restores the same offset, and nothing else: every
/// other field of the page's entry (a share link's mark, how Manage books was opened) says
/// something about that one entry, and a copy on this one would let a task that finishes later
/// take the new entry for the old one. The one thing it records about a share link's mark is
/// [`SHARE_LINK_LAYER_FIELD`], under another name. Returns false when the browser refuses the push.
pub fn push_flagged_entry(field: &str) -> bool {
    let Some(history) = web_sys::window().and_then(|w| w.history().ok()) else {
        return false;
    };
    let mut state = js_sys::Object::new();
    if let Some(id) = state_field(&current_state(), SCROLL_ID_FIELD) {
        state = state_with(&state, SCROLL_ID_FIELD, &id);
    }
    let state = state_with(&state, field, &wasm_bindgen::JsValue::TRUE);
    let state = match current_entry_mark(SHARE_LINK_FIELD) {
        Some(mark) => state_with(&state, SHARE_LINK_LAYER_FIELD, &f64::from(mark).into()),
        None => state,
    };
    history.push_state_with_url(&state, "", None).is_ok()
}

thread_local! {
    static SCROLL_MEMORY: std::cell::RefCell<ScrollMemory> = std::cell::RefCell::new(ScrollMemory::default());
}

/// The scroll offset of the most recent history entries visited in this session. It lives in
/// memory only: a reload starts clean, and an entry that was never visited here has no offset.
#[derive(Default)]
struct ScrollMemory {
    current: Option<u32>,
    saved: std::collections::HashMap<u32, f64>,
    /// Ids from least to most recently reached; the oldest is dropped past the limit.
    order: std::collections::VecDeque<u32>,
}

impl ScrollMemory {
    fn reach(&mut self, id: u32) {
        self.current = Some(id);
        self.order.retain(|&known| known != id);
        self.order.push_back(id);
        while self.order.len() > SCROLL_MEMORY_LIMIT {
            if let Some(oldest) = self.order.pop_front() {
                self.saved.remove(&oldest);
            }
        }
    }
}

/// Records the offset of the entry on screen. Call it from a window `scroll` listener.
pub fn remember_scroll() {
    let Some(y) = web_sys::window().and_then(|w| w.scroll_y().ok()) else {
        return;
    };
    SCROLL_MEMORY.with(|memory| {
        let mut memory = memory.borrow_mut();
        if let Some(id) = memory.current {
            memory.saved.insert(id, y);
        }
    });
}

/// Marks the history entry that a route change has just reached. An entry that this session
/// visited before (Back or Forward) returns the offset it was left at. A new entry gets a
/// scroll id in its history state, next to any other field, and returns `None`.
pub fn arrive_at_entry() -> Option<f64> {
    let history = web_sys::window()?.history().ok()?;
    let state = history.state().unwrap_or(wasm_bindgen::JsValue::NULL);
    if let Some(id) = state_field(&state, SCROLL_ID_FIELD).and_then(|value| value.as_f64()) {
        return SCROLL_MEMORY.with(|memory| {
            let mut memory = memory.borrow_mut();
            memory.reach(id as u32);
            memory.saved.get(&(id as u32)).copied()
        });
    }
    let id = (js_sys::Math::random() * f64::from(u32::MAX)) as u32;
    let state = state_with(&state, SCROLL_ID_FIELD, &f64::from(id).into());
    let _ = history.replace_state(&state, "");
    SCROLL_MEMORY.with(|memory| memory.borrow_mut().reach(id));
    None
}

pub fn scroll_to_offset(y: f64) {
    if let Some(window) = web_sys::window() {
        window.scroll_to_with_x_and_y(0.0, y);
    }
}

pub fn go_back() {
    if let Some(history) = web_sys::window().and_then(|w| w.history().ok()) {
        let _ = history.back();
    }
}

/// Scrolls the element for this anchor into view. Content headings use `h-<key>`; the raw id is
/// the fallback for app elements such as `entry-related` and the browse letter groups.
/// Returns false when neither element is in the document yet.
pub fn scroll_to_anchor(anchor: &str) -> bool {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return false;
    };
    let element = document
        .get_element_by_id(&format!("h-{anchor}"))
        .or_else(|| document.get_element_by_id(anchor));
    let Some(element) = element else {
        return false;
    };
    open_enclosing_disclosure(&element);
    element.scroll_into_view();
    true
}

/// Opens the closed `<details>` that holds an element, so that scrolling to it lands on
/// something visible. A footnote marker in the spell columns of a class progression sits in
/// such a disclosure, and the way back from its note must reach it (#534). A target in no
/// disclosure is left alone. The `closest` call includes the element itself, so start at its
/// parent.
fn open_enclosing_disclosure(element: &web_sys::Element) {
    let Some(parent) = element.parent_element() else {
        return;
    };
    if let Ok(Some(details)) = parent.closest("details")
        && !details.has_attribute("open")
    {
        let _ = details.set_attribute("open", "");
    }
}

/// Scrolls to the start of a class page's section and puts focus on its heading, so that the
/// next Tab enters the section and a screen reader reads it. The target is the element that
/// carries `data-class-section`. Returns false when the page has no such element, which is the
/// case before the entry has rendered and on a class that lacks the section.
pub fn scroll_to_section(section: Section) -> bool {
    scroll_to_section_with(section, true)
}

/// [`scroll_to_section`], with the focus move optional: a scheduled frame passes `false` when
/// the reader has focused a control since (#419), and the page still scrolls.
pub fn scroll_to_section_with(section: Section, focus: bool) -> bool {
    use wasm_bindgen::JsCast;
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return false;
    };
    let selector = format!("[data-class-section=\"{}\"]", section.slug());
    let Some(target) = document.query_selector(&selector).ok().flatten() else {
        return false;
    };
    target.scroll_into_view();
    if focus && let Ok(target) = target.dyn_into::<web_sys::HtmlElement>() {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = target.focus_with_options(&options);
    }
    true
}

pub fn scroll_to_top() {
    if let Some(window) = web_sys::window() {
        window.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> EntryId {
        EntryId::parse(s).unwrap()
    }

    fn round_trip(route: Route) {
        assert_eq!(Route::parse(&route.href()), route, "{}", route.href());
    }

    #[test]
    fn simple_routes_round_trip() {
        for route in [
            Route::Home,
            Route::Quick,
            Route::Settings,
            Route::Diagnostics,
            Route::Legal,
            Route::browse_kind(EntryKind::Spell),
            Route::BrowseSource("core-rulebook".into()),
            Route::books(),
            Route::books_manage(),
            Route::BooksSet("1ABC-DEF2".into()),
            Route::Spells,
            Route::FeatChains,
        ] {
            round_trip(route);
        }
    }

    #[test]
    fn kind_routes_carry_the_selected_letter() {
        let plain = Route::browse_kind(EntryKind::Spell);
        assert_eq!(plain.href(), "#/browse/kind/spell");
        round_trip(plain.clone());
        let letter = Route::browse_kind_letter(EntryKind::Spell, 'F');
        assert_eq!(letter.href(), "#/browse/kind/spell?l=F");
        round_trip(letter);
        let other = Route::browse_kind_letter(EntryKind::Feat, '#');
        assert_eq!(other.href(), "#/browse/kind/feat?l=%23");
        round_trip(other);
        // A lower-case selector and unknown parameters are accepted; a bad selector is ignored.
        assert_eq!(
            Route::parse("#/browse/kind/spell?l=f&zz=1"),
            Route::browse_kind_letter(EntryKind::Spell, 'F')
        );
        for bad in ["", "ff", "1", "-"] {
            assert_eq!(
                Route::parse(&format!("#/browse/kind/spell?l={bad}")),
                plain,
                "{bad:?}"
            );
        }
    }

    #[test]
    fn kind_routes_carry_a_source_with_letter_and_facets_in_every_combination() {
        let facets = vec![("feat_type".to_string(), "Combat".to_string())];
        for source in [None, Some("core-rulebook".to_string())] {
            for letter in [None, Some('B')] {
                for facets in [Vec::new(), facets.clone()] {
                    let route = Route::BrowseKind {
                        kind: EntryKind::Feat,
                        letter,
                        facets,
                        source: source.clone(),
                    };
                    round_trip(route);
                }
            }
        }
        let source = Route::browse_kind(EntryKind::Spell).with_source(Some("core-rulebook".into()));
        assert_eq!(source.href(), "#/browse/kind/spell?source=core-rulebook");
        assert_eq!(
            Route::parse("#/browse/kind/spell?source=core-rulebook"),
            source
        );
        // A source and a facet keep each other, in either parameter order.
        let both = Route::browse_kind_facets(
            EntryKind::Spell,
            vec![("school".into(), "evocation".into())],
        )
        .with_source(Some("core-rulebook".into()));
        assert_eq!(
            both.href(),
            "#/browse/kind/spell?source=core-rulebook&facet=school:evocation"
        );
        round_trip(both.clone());
        assert_eq!(
            Route::parse("#/browse/kind/spell?facet=school:evocation&source=core-rulebook"),
            both
        );
        // Removing the book filter keeps the letter and the facets.
        assert_eq!(
            both.with_letter('C').with_source(None).href(),
            "#/browse/kind/spell?facet=school:evocation&l=C"
        );
        // An empty or undecodable source is no filter, never an error.
        assert_eq!(
            Route::parse("#/browse/kind/spell?source="),
            Route::browse_kind(EntryKind::Spell)
        );
        assert_eq!(
            Route::parse("#/browse/kind/spell?source=%FF"),
            Route::browse_kind(EntryKind::Spell)
        );
        // An unknown book is still the kind list, not a missing page.
        assert_eq!(
            Route::parse("#/browse/kind/spell?source=no-such-book"),
            Route::BrowseKind {
                kind: EntryKind::Spell,
                letter: None,
                facets: Vec::new(),
                source: Some("no-such-book".into()),
            }
        );
    }

    #[test]
    fn kind_routes_carry_facets_in_the_url_and_keep_them_with_a_letter() {
        let combat =
            Route::browse_kind_facets(EntryKind::Feat, vec![("feat_type".into(), "Combat".into())]);
        assert_eq!(combat.href(), "#/browse/kind/feat?facet=feat_type:Combat");
        round_trip(combat.clone());
        let lettered = combat.with_letter('B');
        assert_eq!(
            lettered.href(),
            "#/browse/kind/feat?facet=feat_type:Combat&l=B"
        );
        round_trip(lettered);
        // A value with a space or a symbol is encoded, and the first ':' splits the pair.
        let spaced = Route::browse_kind_facets(
            EntryKind::Feat,
            vec![("feat_type".into(), "Armor Mastery: Heavy".into())],
        );
        assert_eq!(
            spaced.href(),
            "#/browse/kind/feat?facet=feat_type:Armor%20Mastery%3A%20Heavy"
        );
        round_trip(spaced);
        // The parameters may come in either order, and a pair with no ':' is ignored.
        assert_eq!(
            Route::parse("#/browse/kind/feat?l=b&facet=feat_type:combat&facet=broken"),
            Route::BrowseKind {
                kind: EntryKind::Feat,
                letter: Some('B'),
                facets: vec![("feat_type".into(), "combat".into())],
                source: None,
            }
        );
        // Two fields narrow one list together, in the order they were chosen.
        let two = Route::browse_kind_facets(
            EntryKind::Creature,
            vec![
                ("creature_type".into(), "dragon".into()),
                ("cr".into(), "10".into()),
            ],
        );
        assert_eq!(
            two.href(),
            "#/browse/kind/creature?facet=creature_type:dragon&facet=cr:10"
        );
        round_trip(two);
        // Search keeps its own href with the same pairs.
        assert_eq!(
            Route::Search {
                q: "grapple".into(),
                kinds: vec![EntryKind::Feat],
                sources: vec!["core".into()],
                facets: vec![("feat_type".into(), "Combat".into())],
                sections: false,
            }
            .href(),
            "#/search?q=grapple&kind=feat&source=core&facet=feat_type:Combat"
        );
    }

    #[test]
    fn books_has_one_order_and_old_sort_links_open_it() {
        assert_eq!(Route::books().href(), "#/books");
        assert_eq!(Route::parse("#/books"), Route::books());
        // `?sort=` belonged to sort links that no longer exist. A bookmark must not break, and
        // it must not become a second route or a second history entry.
        for hash in [
            "#/books?sort=category",
            "#/books?sort=date",
            "#/books?sort=library",
            "#/books?sort=title",
            "#/books?sort=",
            "#/books?other=date",
            "#/browse/books",
            "#/browse/books?sort=date",
        ] {
            assert_eq!(Route::parse(hash), Route::books(), "{hash}");
        }
        assert_eq!(Route::parse("#/books?sort=date").href(), "#/books");
    }

    #[test]
    fn books_and_manage_books_keep_the_find_text_in_q() {
        let books = |find: &str| Route::Books { find: find.into() };
        let manage = |find: &str| Route::BooksManage { find: find.into() };
        assert_eq!(books("bestiary").href(), "#/books?q=bestiary");
        assert_eq!(manage("bestiary").href(), "#/books/manage?q=bestiary");
        assert_eq!(Route::parse("#/books?q=bestiary"), books("bestiary"));
        assert_eq!(
            Route::parse("#/books/manage?q=bestiary"),
            manage("bestiary")
        );
        // The old address of Manage books takes the text too, and the old sort is ignored beside it.
        assert_eq!(Route::parse("#/books/active?q=a"), manage("a"));
        assert_eq!(Route::parse("#/books?sort=date&q=a"), books("a"));
        // Characters that mean something in an address survive the round trip.
        for text in ["a&b#c+d é", "x=y", "100%", "  spaced  ", "Pathfinder #1"] {
            round_trip(books(text));
            round_trip(manage(text));
            assert!(!books(text).href()[1..].contains(['#', ' ', 'é']), "{text}");
        }
        // A blank or empty text is no search, and the address drops it.
        for hash in ["#/books?q=", "#/books?q=%20%20"] {
            assert_eq!(Route::parse(hash), Route::books(), "{hash}");
        }
        assert_eq!(books("  ").href(), "#/books");
        assert_eq!(Route::parse("#/books/manage?q="), Route::books_manage());
        // Only the first text counts.
        assert_eq!(Route::parse("#/books?q=a&q=b"), books("a"));
        // A share link is Manage books with a question and keeps its code, not a search.
        assert_eq!(
            Route::parse("#/books/manage?set=1ABC&q=a"),
            Route::BooksSet("1ABC".into())
        );
    }

    #[test]
    fn manage_books_has_its_own_route_and_keeps_the_old_address() {
        assert_eq!(Route::books_manage().href(), "#/books/manage");
        round_trip(Route::books_manage());
        assert_eq!(Route::parse("#/books/manage"), Route::books_manage());
        assert_eq!(Route::parse("#/books/manage/"), Route::books_manage());
        // The old address still opens the same page.
        assert_eq!(Route::parse("#/books/active"), Route::books_manage());
        assert_eq!(Route::parse("#/books/active/"), Route::books_manage());
        assert_eq!(Route::parse("#/books/active").href(), "#/books/manage");
        // The page takes no sort; a stray query does not change the route.
        assert_eq!(
            Route::parse("#/books/manage?sort=date"),
            Route::books_manage()
        );
        // A share link opens Manage books with the code (#408); a sort link or an empty code is
        // Books or Manage books. A link made while the section sat on Books means the same.
        assert_eq!(
            Route::parse("#/books/manage?set=1ABC-DEF2"),
            Route::BooksSet("1ABC-DEF2".into())
        );
        assert_eq!(
            Route::BooksSet("1ABC-DEF2".into()).href(),
            "#/books/manage?set=1ABC-DEF2"
        );
        assert_eq!(
            Route::parse("#/books?set=1ABC-DEF2"),
            Route::BooksSet("1ABC-DEF2".into())
        );
        assert_eq!(Route::parse("#/books?set="), Route::books());
        assert_eq!(Route::parse("#/books/manage?set="), Route::books_manage());
        assert_eq!(Route::parse("#/books?sort=date"), Route::books());
        assert_eq!(
            Route::parse("#/books?sort=date&set=1ABC"),
            Route::BooksSet("1ABC".into())
        );
        assert_eq!(
            Route::parse("#/books/active?sort=date"),
            Route::books_manage()
        );
        // Other paths under /books are not this page.
        assert!(matches!(Route::parse("#/books/other"), Route::NotFound(_)));
    }

    #[test]
    fn legacy_bookmark_and_collection_urls_open_home() {
        assert_eq!(Route::parse("#/bookmarks"), Route::Home);
        assert_eq!(Route::parse("#/bookmarks/"), Route::Home);
        assert_eq!(Route::parse("#/collection/old%20collection"), Route::Home);
    }

    #[test]
    fn the_browse_hub_url_opens_home() {
        for hash in ["#/browse", "#/browse/", "#/browse?x=1"] {
            assert_eq!(Route::parse(hash), Route::Home, "{hash}");
        }
    }

    #[test]
    fn book_routes_carry_the_chapter_path_one_based() {
        let plain = Route::Book {
            source: "core-rulebook".into(),
            chapter: None,
        };
        assert_eq!(plain.href(), "#/browse/book/core-rulebook");
        round_trip(plain);
        let deep = Route::Book {
            source: "core-rulebook".into(),
            chapter: Some(ContentsRow::Position(vec![6, 0, 11])),
        };
        assert_eq!(deep.href(), "#/browse/book/core-rulebook?c=7.1.12");
        round_trip(deep.clone());
        // The page scrolls to the row itself once it has resolved the address.
        assert_eq!(deep.anchor(), None);
        // Old links to a publication and links without a chapter still parse.
        assert_eq!(
            Route::parse("#/browse/book/core-rulebook?c="),
            Route::Book {
                source: "core-rulebook".into(),
                chapter: None
            }
        );
        // A malformed or zero-based path is ignored, not an error.
        for bad in ["0", "1..2", "a", "1.0", "-1", "99999999"] {
            assert_eq!(
                Route::parse(&format!("#/browse/book/x?c={bad}")),
                Route::Book {
                    source: "x".into(),
                    chapter: None
                },
                "{bad}"
            );
        }
        assert_eq!(Route::books().href(), "#/books");
        assert_eq!(Route::parse("#/browse/books"), Route::books());
        assert_eq!(Route::parse("#/browse/kind/rule"), Route::books());
        assert_eq!(
            Route::parse("#/browse/book"),
            Route::NotFound("#/browse/book".into())
        );
    }

    #[test]
    fn a_contents_row_is_named_by_identity() {
        let stable = |chapter: &str, anchor: Option<&str>| Route::Book {
            source: "core-rulebook".into(),
            chapter: Some(ContentsRow::Stable {
                chapter: chapter.into(),
                anchor: anchor.map(str::to_string),
            }),
        };
        let row = stable("combat", Some("row-ref-rule-actions--h-full-attack"));
        assert_eq!(
            row.href(),
            "#/browse/book/core-rulebook?c=~combat&a=row-ref-rule-actions--h-full-attack"
        );
        round_trip(row);
        let chapter = stable("combat", None);
        assert_eq!(chapter.href(), "#/browse/book/core-rulebook?c=~combat");
        round_trip(chapter);
        // A chapter id that reads as a position stays an id after the marker.
        let numeric = stable("2", None);
        assert_eq!(numeric.href(), "#/browse/book/core-rulebook?c=~2");
        round_trip(numeric);
        // Characters that a query reserves survive.
        round_trip(stable("a&b/c", Some("row-x y")));
        // A bare marker, or an anchor without a chapter, names no row.
        for bad in ["~", "", "a=row-x"] {
            let Route::Book { chapter, .. } =
                Route::parse(&format!("#/browse/book/core-rulebook?c={bad}"))
            else {
                panic!("a book route");
            };
            assert_eq!(chapter, None, "{bad}");
        }
        let Route::Book { chapter, .. } =
            Route::parse("#/browse/book/core-rulebook?a=row-ref-class-ranger")
        else {
            panic!("a book route");
        };
        assert_eq!(chapter, None);
    }

    #[test]
    fn a_retired_book_id_opens_the_surviving_book() {
        // The corpus once exported the GameMastery Guide under two book ids. The old
        // address keeps opening the one book record.
        assert_eq!(
            Route::parse("#/browse/book/gamemastery-guide"),
            Route::Book {
                source: "game-mastery-guide".into(),
                chapter: None
            }
        );
        assert_eq!(
            Route::parse("#/browse/book/gamemastery-guide?c=7.1"),
            Route::Book {
                source: "game-mastery-guide".into(),
                chapter: Some(ContentsRow::Position(vec![6, 0]))
            }
        );
        assert_eq!(
            Route::parse("#/books/gamemastery-guide/combat"),
            Route::BookChapter {
                source: "game-mastery-guide".into(),
                chapter: "combat".into(),
                anchor: None
            }
        );
        assert_eq!(
            Route::parse("#/browse/source/gamemastery-guide"),
            Route::BrowseSource("game-mastery-guide".into())
        );
        assert_eq!(
            Route::parse("#/entry/ref/rule/getting-started?book=gamemastery-guide"),
            Route::Entry {
                id: EntryId::parse("ref/rule/getting-started").unwrap(),
                anchor: None,
                book: Some("game-mastery-guide".into()),
                section: None,
                arch: Default::default(),
            }
        );
        // The surviving id and every other book id are unchanged.
        for source in ["game-mastery-guide", "core-rulebook"] {
            assert_eq!(
                Route::parse(&format!("#/browse/book/{source}")),
                Route::Book {
                    source: source.into(),
                    chapter: None
                },
                "{source}"
            );
        }
    }

    #[test]
    fn a_book_filter_of_a_retired_book_id_names_the_book_that_holds_its_entries() {
        // Membership matches the book ids in `source_ids`, so an old link's filter has to carry
        // the current id or it finds nothing.
        assert_eq!(
            Route::parse("#/browse/kind/feat?source=gamemastery-guide"),
            Route::BrowseKind {
                kind: EntryKind::Feat,
                letter: None,
                facets: vec![],
                source: Some("game-mastery-guide".into()),
            }
        );
        assert_eq!(
            Route::parse("#/browse/kind/creature?source=pathfinder-rpg-bestiary"),
            Route::BrowseKind {
                kind: EntryKind::Creature,
                letter: None,
                facets: vec![],
                source: Some("bestiary".into()),
            }
        );
        let Route::Search { sources, .. } = Route::parse("#/search?q=x&source=prpg-core-rulebook")
        else {
            panic!("a search route");
        };
        assert_eq!(sources, ["core-rulebook"]);
        // A current id and a one-off tree (no replacement) pass through unchanged.
        let Route::Search { sources, .. } =
            Route::parse("#/search?q=x&source=core-rulebook&source=pathfinder-comics-3")
        else {
            panic!("a search route");
        };
        assert_eq!(sources, ["core-rulebook", "pathfinder-comics-3"]);
    }

    #[test]
    fn an_address_of_a_deleted_book_tree_opens_its_destination() {
        let book = |source: &str| Route::Book {
            source: source.into(),
            chapter: None,
        };
        // A listing tree opens the printed book. Its chapter and path addresses have no
        // equivalent, so they open the book's page as well.
        for (old, new) in [
            ("prpg-core-rulebook", "core-rulebook"),
            ("pathfinder-rpg-bestiary", "bestiary"),
        ] {
            assert_eq!(Route::parse(&format!("#/browse/book/{old}")), book(new));
            assert_eq!(Route::parse(&format!("#/browse/book/{old}?c=1")), book(new));
            for chapter in ["page-order", "unpaged"] {
                assert_eq!(
                    Route::parse(&format!("#/browse/book/{old}/{chapter}")),
                    book(new),
                    "{old}/{chapter}"
                );
                assert_eq!(
                    Route::parse(&format!("#/books/{old}/{chapter}?a=toc-1")),
                    book(new)
                );
            }
            assert_eq!(
                Route::parse(&format!("#/browse/source/{old}")),
                Route::BrowseSource(new.into())
            );
            assert_eq!(
                Route::parse(&format!("#/entry/ref/rule/getting-started?book={old}")),
                Route::Entry {
                    id: EntryId::parse("ref/rule/getting-started").unwrap(),
                    anchor: None,
                    book: Some(new.into()),
                    section: None,
                    arch: Default::default(),
                }
            );
        }
        // A one-off tree has no replacement: its book, chapter and source addresses open Books
        // with a line that says the book was removed.
        for old in ["paizo-blog-sable-company-marine", "pathfinder-comics-3"] {
            let removed = Route::BookRemoved(old.into());
            assert_eq!(Route::parse(&format!("#/browse/book/{old}")), removed);
            assert_eq!(
                Route::parse(&format!("#/browse/book/{old}/page-order")),
                removed
            );
            assert_eq!(
                Route::parse(&format!("#/browse/book/{old}/unpaged")),
                removed
            );
            assert_eq!(Route::parse(&format!("#/browse/source/{old}")), removed);
            round_trip(removed);
        }
    }

    #[test]
    fn a_class_section_is_a_query_and_never_part_of_the_entry_id() {
        let id = EntryId::parse("ref/class/wizard").unwrap();
        let route = Route::Entry {
            id: id.clone(),
            anchor: None,
            book: None,
            section: Some(Section::Archetypes),
            arch: Default::default(),
        };
        assert_eq!(route.href(), "#/entry/ref/class/wizard?s=archetypes");
        round_trip(route.clone());
        assert_eq!(route.section(), Some(Section::Archetypes));
        // The id, the anchor and the book keep their own places.
        let all = Route::Entry {
            id: id.clone(),
            anchor: Some("arcane-bond".into()),
            book: Some("core-rulebook".into()),
            section: Some(Section::Spells),
            arch: Default::default(),
        };
        assert_eq!(
            all.href(),
            "#/entry/ref/class/wizard?a=arcane-bond&book=core-rulebook&s=spells"
        );
        round_trip(all);
        // Overview is the page, so it has no value; an unknown or empty value is ignored.
        for hash in [
            "#/entry/ref/class/wizard?s=overview",
            "#/entry/ref/class/wizard?s=",
            "#/entry/ref/class/wizard?s=nope",
            "#/entry/ref/class/wizard",
        ] {
            assert_eq!(Route::parse(hash), Route::entry(id.clone()), "{hash}");
        }
        assert_eq!(
            Route::parse("#/entry/ref/class/wizard?s=FEATURES").section(),
            Some(Section::Features)
        );
        // A prestige class's Requirements stop is a query of the same kind.
        let archer = EntryId::parse("ref/pclass/arcane-archer").unwrap();
        let requirements = Route::Entry {
            id: archer,
            anchor: None,
            book: None,
            section: Some(Section::Requirements),
            arch: Default::default(),
        };
        assert_eq!(
            requirements.href(),
            "#/entry/ref/pclass/arcane-archer?s=requirements"
        );
        round_trip(requirements);
        // Another route has no section.
        assert_eq!(Route::Home.section(), None);
    }

    #[test]
    fn the_archetype_choices_are_a_query_that_round_trips() {
        let wizard = EntryId::parse("ref/class/wizard").unwrap();
        let physician = EntryId::parse("ref/arc-wizard/arcane-physician").unwrap();
        let route = Route::Entry {
            id: wizard.clone(),
            anchor: None,
            book: None,
            section: Some(Section::Archetypes),
            arch: ArchFilter {
                keep: vec!["arcane-bond".into(), "scribe-scroll".into()],
                with: Some(physician.clone()),
            },
        };
        assert_eq!(
            route.href(),
            "#/entry/ref/class/wizard?s=archetypes&keep=arcane-bond&keep=scribe-scroll&with=ref%2Farc-wizard%2Farcane-physician"
        );
        round_trip(route.clone());
        assert_eq!(route.arch().keep, ["arcane-bond", "scribe-scroll"]);
        assert_eq!(route.arch().with, Some(physician.clone()));
        // The entry id stays the path (rule 3); the choices ride beside the other parts.
        let all = Route::Entry {
            id: wizard.clone(),
            anchor: Some("arcane-bond".into()),
            book: Some("core-rulebook".into()),
            section: Some(Section::Archetypes),
            arch: ArchFilter {
                keep: vec!["cantrips".into()],
                with: None,
            },
        };
        assert_eq!(
            all.href(),
            "#/entry/ref/class/wizard?a=arcane-bond&book=core-rulebook&s=archetypes&keep=cantrips"
        );
        round_trip(all);
        // One choice alone.
        let with_only = Route::Entry {
            id: wizard.clone(),
            anchor: None,
            book: None,
            section: None,
            arch: ArchFilter {
                keep: Vec::new(),
                with: Some(physician),
            },
        };
        round_trip(with_only);
        // Another route has no choices.
        assert!(Route::Home.arch().is_empty());
    }

    #[test]
    fn old_entry_links_parse_with_no_archetype_choices() {
        let wizard = EntryId::parse("ref/class/wizard").unwrap();
        for hash in [
            "#/entry/ref/class/wizard",
            "#/entry/ref/class/wizard?s=archetypes",
            "#/entry/ref/class/wizard?a=arcane-bond",
            "#/entry/ref/class/wizard?a=arcane-bond&book=core-rulebook&s=features",
            "#/entry/ref/pclass/arcane-archer?s=requirements",
        ] {
            let route = Route::parse(hash);
            assert!(route.arch().is_empty(), "{hash}");
            assert_eq!(route.href(), hash, "{hash} serializes back unchanged");
        }
        assert_eq!(
            Route::parse("#/entry/ref/class/wizard?s=archetypes"),
            Route::Entry {
                id: wizard,
                anchor: None,
                book: None,
                section: Some(Section::Archetypes),
                arch: ArchFilter::default(),
            }
        );
    }

    #[test]
    fn a_bad_archetype_choice_is_dropped_and_a_repeated_one_counts_once() {
        let route = Route::parse(
            "#/entry/ref/class/wizard?keep=arcane-bond&keep=&keep=arcane-bond&keep=cantrips&with=not%20an%20id",
        );
        assert_eq!(route.arch().keep, ["arcane-bond", "cantrips"]);
        assert_eq!(route.arch().with, None);
        // An id that is well formed stays, whether or not the page lists it.
        let route = Route::parse("#/entry/ref/class/wizard?with=ref%2Farc-wizard%2Fgone");
        assert_eq!(
            route.arch().with,
            Some(EntryId::parse("ref/arc-wizard/gone").unwrap())
        );
    }

    #[test]
    fn a_change_of_archetype_choices_alone_is_not_a_navigation_of_the_page() {
        let id = EntryId::parse("ref/class/wizard").unwrap();
        let at = |keep: &[&str], section: Option<Section>, anchor: Option<&str>| Route::Entry {
            id: id.clone(),
            anchor: anchor.map(str::to_owned),
            book: None,
            section,
            arch: ArchFilter {
                keep: keep.iter().map(|k| k.to_string()).collect(),
                with: None,
            },
        };
        let archetypes = Some(Section::Archetypes);
        assert!(at(&[], archetypes, None).only_arch_differs(&at(&["cantrips"], archetypes, None)));
        assert!(at(&["cantrips"], archetypes, None).only_arch_differs(&at(&[], archetypes, None)));
        // The first choice from a bare address, from another stop, and from an `?a=` address: the
        // list's own links name the Archetypes stop and no anchor.
        assert!(at(&[], None, None).only_arch_differs(&at(&["cantrips"], archetypes, None)));
        assert!(
            at(&[], Some(Section::Features), None).only_arch_differs(&at(
                &["cantrips"],
                archetypes,
                None
            ))
        );
        assert!(at(&[], None, Some("arcane-bond")).only_arch_differs(&at(
            &["cantrips"],
            archetypes,
            None
        )));
        // A class without the stop keeps its section: none stays none.
        assert!(at(&[], None, None).only_arch_differs(&at(&["cantrips"], None, None)));
        // Back from that first choice to the bare address undoes the choice in place too.
        assert!(at(&["cantrips"], archetypes, None).only_arch_differs(&at(&[], None, None)));
        // Leaving the stop for another one with other choices is a move.
        assert!(!at(&["cantrips"], archetypes, None).only_arch_differs(&at(
            &[],
            Some(Section::Features),
            None
        )));
        // Back to an `?a=` address is a move to that heading.
        assert!(!at(&["cantrips"], archetypes, None).only_arch_differs(&at(
            &[],
            None,
            Some("arcane-bond")
        )));
        // The same choices, or a move to another stop, another anchor or another entry, is a
        // different move.
        assert!(!at(&[], archetypes, None).only_arch_differs(&at(&[], archetypes, None)));
        assert!(!at(&[], archetypes, None).only_arch_differs(&at(
            &["cantrips"],
            Some(Section::Features),
            None
        )));
        assert!(!at(&[], None, None).only_arch_differs(&at(
            &["cantrips"],
            archetypes,
            Some("arcane-bond")
        )));
        assert!(!at(&[], None, None).only_arch_differs(&Route::Home));
        let other = Route::entry(EntryId::parse("ref/class/fighter").unwrap());
        assert!(!at(&[], archetypes, None).only_arch_differs(&other));
        // Another book is another page.
        let in_book = Route::Entry {
            id: id.clone(),
            anchor: None,
            book: Some("core-rulebook".into()),
            section: archetypes,
            arch: ArchFilter {
                keep: vec!["cantrips".into()],
                with: None,
            },
        };
        assert!(!at(&[], archetypes, None).only_arch_differs(&in_book));
    }

    #[test]
    fn a_jump_inside_the_entry_on_screen_is_not_a_navigation() {
        let id = EntryId::parse("ref/class/wizard").unwrap();
        let at = |anchor: Option<&str>, section| Route::Entry {
            id: id.clone(),
            anchor: anchor.map(String::from),
            book: None,
            section,
            arch: Default::default(),
        };
        assert!(at(None, None).is_section_jump(&at(None, Some(Section::Features))));
        assert!(
            at(None, Some(Section::Spells)).is_section_jump(&at(None, Some(Section::Features)))
        );
        // Back to the plain address from a section is the same kind of move.
        assert!(at(None, Some(Section::Spells)).is_section_jump(&at(None, None)));
        // A stop chosen from an `?a=` address clears the anchor and keeps the page.
        assert!(
            at(Some("arcane-bond"), None).is_section_jump(&at(None, Some(Section::Archetypes)))
        );
        assert!(at(Some("arcane-bond"), None).is_section_jump(&at(None, None)));
        // The same address is no move.
        assert!(!at(None, Some(Section::Spells)).is_section_jump(&at(None, Some(Section::Spells))));
        // A move to an anchor, to another entry, another book or another route is a navigation.
        assert!(!at(None, None).is_section_jump(&at(Some("arcane-bond"), None)));
        assert!(!at(None, Some(Section::Spells)).is_section_jump(&at(Some("arcane-bond"), None)));
        let other = Route::Entry {
            id: EntryId::parse("ref/class/fighter").unwrap(),
            anchor: None,
            book: None,
            section: Some(Section::Features),
            arch: Default::default(),
        };
        assert!(!at(None, None).is_section_jump(&other));
        let in_book = Route::Entry {
            id: id.clone(),
            anchor: None,
            book: Some("core-rulebook".into()),
            section: Some(Section::Features),
            arch: Default::default(),
        };
        assert!(!at(None, None).is_section_jump(&in_book));
        assert!(!at(None, None).is_section_jump(&Route::Home));
        assert!(!Route::Home.is_section_jump(&at(None, Some(Section::Features))));
    }

    #[test]
    fn chapter_pages_have_their_own_route_and_accept_the_issue_shape() {
        let route = Route::BookChapter {
            source: "core-rulebook".into(),
            chapter: "combat".into(),
            anchor: None,
        };
        assert_eq!(route.href(), "#/browse/book/core-rulebook/combat");
        round_trip(route.clone());
        // The compact `#/books/<source>/<chapter>` address opens the same page.
        assert_eq!(Route::parse("#/books/core-rulebook/combat"), route);
        // A chapter id with a space or a slash is one encoded segment.
        let odd = Route::BookChapter {
            source: "odd source".into(),
            chapter: "a/b & c".into(),
            anchor: None,
        };
        assert_eq!(odd.href(), "#/browse/book/odd%20source/a%2Fb%20%26%20c");
        round_trip(odd);
        // A section result carries the anchor of its row on the page.
        let section = Route::BookChapter {
            source: "core-rulebook".into(),
            chapter: "combat".into(),
            anchor: Some("toc-3-2".into()),
        };
        assert_eq!(
            section.href(),
            "#/browse/book/core-rulebook/combat?a=toc-3-2"
        );
        round_trip(section.clone());
        assert_eq!(section.anchor().as_deref(), Some("toc-3-2"));
        // A malformed address is not found.
        for hash in ["#/browse/book//combat", "#/books/core-rulebook/%FF"] {
            assert_eq!(Route::parse(hash), Route::NotFound(hash.into()), "{hash}");
        }
    }

    #[test]
    fn a_spell_list_keeps_its_chosen_schools_in_the_url_in_order() {
        let list = Route::SpellList {
            class: "Wizard".into(),
            level: Some(3),
            schools: vec!["evocation".into(), "conjuration".into()],
            name: String::new(),
        };
        assert_eq!(
            list.href(),
            "#/spells/Wizard/3?school=evocation&school=conjuration"
        );
        round_trip(list);
        round_trip(Route::SpellList {
            class: "Wizard".into(),
            level: None,
            schools: vec!["illusion".into()],
            name: String::new(),
        });
        // Case and repeats do not make a second choice, and an empty value is none.
        assert_eq!(
            Route::parse("#/spells/Wizard?school=Evocation&school=evocation&school="),
            Route::SpellList {
                class: "Wizard".into(),
                level: None,
                schools: vec!["evocation".into()],
                name: String::new(),
            }
        );
    }

    #[test]
    fn a_spell_list_keeps_its_name_filter_after_the_schools() {
        let list = Route::SpellList {
            class: "Wizard".into(),
            level: Some(1),
            schools: vec!["evocation".into()],
            name: "fire ball".into(),
        };
        assert_eq!(
            list.href(),
            "#/spells/Wizard/1?school=evocation&name=fire%20ball"
        );
        round_trip(list);
        let only_name = Route::SpellList {
            class: "Wizard".into(),
            level: None,
            schools: Vec::new(),
            name: "fire".into(),
        };
        assert_eq!(only_name.href(), "#/spells/Wizard?name=fire");
        round_trip(only_name);
        // Characters that mean something in an address survive the round trip, and the name stays
        // one pair of the query.
        for text in ["a&b#c+d é", "x=y", "100%", "  spaced  "] {
            let route = Route::SpellList {
                class: "Wizard".into(),
                level: Some(2),
                schools: vec!["evocation".into()],
                name: text.into(),
            };
            let href = route.href();
            assert!(
                !href[1..].contains(['#', ' ', 'é']),
                "{href} holds a raw character"
            );
            assert_eq!(href.matches('&').count(), 1, "{href}");
            round_trip(route);
        }
        // An empty or blank name is none, and the address drops it.
        for hash in ["#/spells/Wizard?name=", "#/spells/Wizard?name=%20%20"] {
            assert_eq!(
                Route::parse(hash),
                Route::spell_list("Wizard", None),
                "{hash}"
            );
        }
        // Only the first name counts.
        assert_eq!(
            Route::parse("#/spells/Wizard/1?name=a&name=b"),
            Route::SpellList {
                class: "Wizard".into(),
                level: Some(1),
                schools: Vec::new(),
                name: "a".into(),
            }
        );
    }

    #[test]
    fn spell_list_routes_encode_the_class_name() {
        let list = Route::SpellList {
            class: "Elementalist Wizard".into(),
            level: Some(3),
            schools: Vec::new(),
            name: String::new(),
        };
        assert_eq!(list.href(), "#/spells/Elementalist%20Wizard/3");
        round_trip(list);
        let all = Route::SpellList {
            class: "Wizard".into(),
            level: None,
            schools: Vec::new(),
            name: String::new(),
        };
        assert_eq!(all.href(), "#/spells/Wizard");
        round_trip(all);
        round_trip(Route::SpellList {
            class: "Witch".into(),
            level: Some(0),
            schools: Vec::new(),
            name: String::new(),
        });
        for bad in [
            "#/spells/Wizard/x",
            "#/spells/Wizard/-1",
            "#/spells/Wizard/300",
            "#/spells/Wizard/1/2",
            "#/spells//1",
        ] {
            assert_eq!(Route::parse(bad), Route::NotFound(bad.into()), "{bad}");
        }
    }

    #[test]
    fn feat_chains_route_does_not_swallow_other_feat_paths() {
        assert_eq!(Route::parse("#/feats/chains"), Route::FeatChains);
        assert_eq!(Route::parse("#/feats"), Route::NotFound("#/feats".into()));
    }

    #[test]
    fn empty_and_root_hashes_are_home() {
        for hash in ["", "#", "#/", "/"] {
            assert_eq!(Route::parse(hash), Route::Home, "{hash:?}");
        }
    }

    #[test]
    fn entry_ids_keep_their_slashes() {
        let route = Route::entry(id("core-rulebook/spells/fireball"));
        assert_eq!(route.href(), "#/entry/core-rulebook/spells/fireball");
        round_trip(route);
    }

    #[test]
    fn entry_anchor_is_encoded_in_a_query() {
        let route = Route::Entry {
            id: id("core-rulebook/spells/fireball"),
            anchor: Some("caster level & more/é".into()),
            book: None,
            section: None,
            arch: Default::default(),
        };
        assert_eq!(
            route.href(),
            "#/entry/core-rulebook/spells/fireball?a=caster%20level%20%26%20more%2F%C3%A9"
        );
        round_trip(route);
    }

    #[test]
    fn an_entry_carries_the_book_it_was_read_from() {
        let route = Route::entry_in_book(id("core-rulebook/spells/fireball"), "core-rulebook");
        assert_eq!(
            route.href(),
            "#/entry/core-rulebook/spells/fireball?book=core-rulebook"
        );
        round_trip(route.clone());
        // The anchor and the book keep each other, in either parameter order.
        let both = Route::Entry {
            id: id("core-rulebook/spells/fireball"),
            anchor: Some("casting".into()),
            book: Some("core-rulebook".into()),
            section: None,
            arch: Default::default(),
        };
        assert_eq!(
            both.href(),
            "#/entry/core-rulebook/spells/fireball?a=casting&book=core-rulebook"
        );
        round_trip(both);
        assert_eq!(
            Route::parse("#/entry/core-rulebook/spells/fireball?book=core-rulebook&a=casting"),
            Route::Entry {
                id: id("core-rulebook/spells/fireball"),
                anchor: Some("casting".into()),
                book: Some("core-rulebook".into()),
                section: None,
                arch: Default::default(),
            }
        );
        // An empty context is no context.
        assert_eq!(Route::parse("#/entry/a/b?book="), Route::entry(id("a/b")));
    }

    #[test]
    fn empty_anchor_is_none() {
        assert_eq!(Route::parse("#/entry/a/b?a="), Route::entry(id("a/b")));
    }

    #[test]
    fn malformed_entry_ids_are_not_found() {
        for hash in [
            "#/entry",
            "#/entry/",
            "#/entry//a",
            "#/entry/Bad Id",
            "#/entry/A/b",
            "#/entry/a/-b",
            "#/entry/a%2",
            "#/entry/a/%FF",
        ] {
            assert_eq!(Route::parse(hash), Route::NotFound(hash.into()), "{hash}");
        }
    }

    #[test]
    fn percent_encoded_ids_decode_before_validation() {
        assert_eq!(Route::parse("#/entry/a%2Fb"), Route::entry(id("a/b")));
    }

    #[test]
    fn unknown_hashes_are_not_found() {
        for hash in [
            "#/nope",
            "#nope",
            "#/browse/kind/dragon",
            "#/browse/kind",
            "#/search/x",
        ] {
            assert_eq!(Route::parse(hash), Route::NotFound(hash.into()), "{hash}");
        }
        assert_eq!(Route::NotFound("#/nope".into()).href(), "#/nope");
        assert_eq!(Route::NotFound("junk".into()).href(), "#/");
    }

    #[test]
    fn empty_facet_values_are_dropped() {
        // A hand-made URL must not narrow a list on an empty value and show "No  feats".
        let Route::BrowseKind { facets, .. } =
            Route::parse("#/browse/kind/feat?facet=feat_type:&l=b")
        else {
            panic!("not a kind route");
        };
        assert!(facets.is_empty());
        let Route::Search { facets, .. } =
            Route::parse("#/search?q=x&facet=feat_type:&facet=descriptor:%20")
        else {
            panic!("not a search route");
        };
        assert!(facets.is_empty());
        // A real pair next to an empty one survives.
        let Route::Search { facets, .. } =
            Route::parse("#/search?facet=feat_type:&facet=school:evocation")
        else {
            panic!("not a search route");
        };
        assert_eq!(facets, [("school".to_string(), "evocation".to_string())]);
    }

    #[test]
    fn search_round_trips_with_filters() {
        round_trip(Route::search("magic missile"));
        round_trip(Route::Search {
            q: "fire & ice+é".into(),
            kinds: vec![EntryKind::Spell, EntryKind::Feat],
            sources: vec!["core-rulebook".into(), "odd source".into()],
            facets: vec![
                ("school".into(), "evocation".into()),
                ("level".into(), "a:b c".into()),
            ],
            sections: false,
        });
        round_trip(Route::search(""));
    }

    #[test]
    fn the_open_sections_row_rides_in_the_search_url() {
        let open = Route::Search {
            q: "fire".into(),
            kinds: vec![EntryKind::Spell],
            sources: Vec::new(),
            facets: vec![("school".into(), "evocation".into())],
            sections: true,
        };
        assert_eq!(
            open.href(),
            "#/search?q=fire&kind=spell&facet=school:evocation&sections=1"
        );
        round_trip(open.clone());
        // Only the value `1` opens the row, and a folded search has no marker.
        let Route::Search { sections, .. } = Route::parse("#/search?q=fire&sections=0") else {
            panic!("not a search route");
        };
        assert!(!sections);
        assert_eq!(Route::search("fire").href(), "#/search?q=fire");
        // An open row with no text is still a distinct route.
        round_trip(Route::Search {
            q: String::new(),
            kinds: Vec::new(),
            sources: Vec::new(),
            facets: Vec::new(),
            sections: true,
        });
    }

    #[test]
    fn a_size_facet_link_reads_in_lower_case() {
        for hash in [
            "#/search?q=dragon&kind=creature&facet=size:Huge",
            "#/search?q=dragon&kind=creature&facet=size:huge",
        ] {
            let Route::Search { facets, .. } = Route::parse(hash) else {
                panic!("{hash} is not a search route");
            };
            assert_eq!(facets, [("size".to_string(), "huge".to_string())], "{hash}");
        }
    }

    #[test]
    fn an_old_school_facet_link_lands_on_the_school() {
        for (hash, school) in [
            (
                "#/search?kind=spell&facet=school:conjuration%20(calling)",
                "conjuration",
            ),
            (
                "#/search?facet=school:enchantment%20(compulsion)",
                "enchantment",
            ),
            ("#/search?facet=school:multiple%20(see%20text)", "multiple"),
            ("#/search?facet=school:evocation", "evocation"),
        ] {
            let Route::Search { facets, .. } = Route::parse(hash) else {
                panic!("{hash} is not a search route");
            };
            assert_eq!(
                facets,
                [("school".to_string(), school.to_string())],
                "{hash}"
            );
        }
        // Other fields keep their values.
        let Route::Search { facets, .. } = Route::parse("#/search?facet=descriptor:a%20(b)") else {
            panic!()
        };
        assert_eq!(facets, [("descriptor".to_string(), "a (b)".to_string())]);
    }

    #[test]
    fn search_without_parameters_has_a_short_href() {
        assert_eq!(Route::search("").href(), "#/search");
        assert_eq!(Route::search("a b").href(), "#/search?q=a%20b");
    }

    #[test]
    fn search_ignores_unknown_parameters_and_kinds() {
        assert_eq!(
            Route::parse("#/search?q=x&zz=1&kind=dragon&kind=spell"),
            Route::Search {
                q: "x".into(),
                kinds: vec![EntryKind::Spell],
                sources: vec![],
                facets: vec![],
                sections: false,
            }
        );
    }

    #[test]
    fn trailing_slashes_are_tolerated() {
        assert_eq!(Route::parse("#/settings/"), Route::Settings);
        assert_eq!(Route::parse("#/settings/diagnostics/"), Route::Diagnostics);
    }

    #[test]
    fn the_updates_query_lands_on_settings_and_names_the_section() {
        assert_eq!(Route::parse(settings_updates_href()), Route::Settings);
        assert!(wants_updates_section(settings_updates_href()));
        assert!(wants_updates_section("#/settings?section=updates&x=1"));
        for hash in [
            "#/settings",
            "#/legal?section=updates",
            "#/settings?section=library",
            "#/settings?section=updates#later",
        ] {
            assert!(!wants_updates_section(hash), "{hash}");
        }
    }

    #[test]
    fn percent_codec_round_trips_and_rejects_bad_input() {
        let text = "a b/c?d=e&f#g%é";
        assert_eq!(percent_decode(&percent_encode(text)).as_deref(), Some(text));
        assert_eq!(percent_decode("%zz"), None);
        assert_eq!(percent_decode("%4"), None);
        assert_eq!(percent_decode("%FF"), None);
        assert_eq!(percent_decode("a+b").as_deref(), Some("a b"));
    }
}
