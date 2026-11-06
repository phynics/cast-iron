//! Hash routes. Pure parsing and formatting live here and are tested natively.
//! `use_route`, `navigate`, and the scroll helpers wrap the browser.
//!
//! Grammar (the hash always starts with `#/`):
//!
//! ```text
//! #/                                 Home
//! #/search?q=..&kind=spell&source=core-rulebook&facet=school:evocation
//! #/browse (Home)                    #/browse/kind/<slug>     #/browse/source/<id>
//! #/books                            the books in priority tiers, titles in order (the default)
//! #/books?sort=category|date         the same tiers, ordered inside each by product line or date
//! #/books?sort=library               one ungrouped list, in catalog order
//! #/browse/books                     the same as #/books, and `#/browse/books?sort=..` too
//! #/browse/book/<source>?c=3.2       a book's contents; `c` is the 1-based chapter path
//! #/spells                           #/spells/<class>         #/spells/<class>/<level>
//! #/feats/chains                     feats that unlock others
//! #/entry/<entry id>?a=<anchor>
//! #/bookmarks                        #/collection/<id>
//! #/settings                         #/legal
//! ```

use cast_iron_schema::{EntryId, EntryKind};
use leptos::prelude::*;

/// A displayed order for the publication directory. [`Route::Books`] is the default: priority
/// tier groups with titles in order inside each. These orders change the order inside each
/// tier, except `Library`, which shows one ungrouped list in catalog order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BookSort {
    /// Inside each tier, by reviewed product line and then title.
    Category,
    /// Inside each tier, by publication date newest first, with unknown dates last.
    Date,
    /// One ungrouped list in catalog order.
    Library,
}

impl BookSort {
    pub const ALL: [BookSort; 3] = [BookSort::Category, BookSort::Date, BookSort::Library];

    pub fn slug(self) -> &'static str {
        match self {
            BookSort::Category => "category",
            BookSort::Date => "date",
            BookSort::Library => "library",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|sort| sort.slug() == slug)
    }

    pub fn label(self) -> &'static str {
        match self {
            BookSort::Category => "Category",
            BookSort::Date => "Publication date",
            BookSort::Library => "Library order",
        }
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
    },
    /// A kind's A to Z list. `letter` selects one letter group; `None` shows the first group.
    BrowseKind {
        kind: EntryKind,
        letter: Option<char>,
    },
    /// Kept for old links. A library that has `books.json` shows the book view here.
    BrowseSource(String),
    /// The publication directory, each a doorway into its contents, in priority tiers.
    Books,
    /// The publication directory grouped by one reviewed catalogue field.
    BooksSorted(BookSort),
    /// One book's contents in original chapter order. `chapter` is the 0-based path of the
    /// open or highlighted node; the URL holds it 1-based, as `c=3.2`.
    Book {
        source: String,
        chapter: Option<Vec<usize>>,
    },
    /// Spell lists by class and level.
    Spells,
    /// One class's spell list. `class` is the name that the spells carry, such as `Wizard`.
    SpellList {
        class: String,
        level: Option<u8>,
    },
    /// Feats that other feats require, with their chains.
    FeatChains,
    Entry {
        id: EntryId,
        anchor: Option<String>,
    },
    /// Retained for internal compatibility; parser redirects legacy URLs to Home.
    Bookmarks,
    Collection(String),
    Settings,
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
        }
    }

    pub fn entry(id: EntryId) -> Self {
        Route::Entry { id, anchor: None }
    }

    /// A kind's A to Z list, opening its first letter group.
    pub fn browse_kind(kind: EntryKind) -> Self {
        Route::BrowseKind { kind, letter: None }
    }

    /// A kind's A to Z list with one letter group open.
    pub fn browse_kind_letter(kind: EntryKind, letter: char) -> Self {
        Route::BrowseKind {
            kind,
            letter: Some(letter),
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
                        Route::Books
                    } else {
                        Route::BrowseKind {
                            kind,
                            letter: query
                                .and_then(|q| query_pairs(q).find(|(key, _)| key == "l"))
                                .and_then(|(_, value)| parse_letter(&value)),
                        }
                    }
                }),
            ["browse", "source", id] => match percent_decode(id) {
                Some(id) if !id.is_empty() => Route::BrowseSource(id),
                _ => not_found(),
            },
            ["books"] | ["browse", "books"] => query
                .and_then(|q| query_pairs(q).find(|(k, _)| k == "sort"))
                .and_then(|(_, v)| BookSort::from_slug(&v))
                .map_or(Route::Books, Route::BooksSorted),
            ["browse", "book", id] => match percent_decode(id) {
                Some(source) if !source.is_empty() => Route::Book {
                    source,
                    chapter: query
                        .and_then(|q| query_pairs(q).find(|(k, _)| k == "c"))
                        .and_then(|(_, v)| parse_chapter_path(&v)),
                },
                _ => not_found(),
            },
            ["spells"] => Route::Spells,
            ["spells", class] => match percent_decode(class) {
                Some(class) if !class.is_empty() => Route::SpellList { class, level: None },
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
                Route::Entry { id, anchor }
            }
            // Legacy saved-entry URLs now land on Home, where all stars are listed.
            ["bookmarks"] | ["collection", _] => Route::Home,
            ["settings"] => Route::Settings,
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
                for (field, value) in facets {
                    pairs.push(format!(
                        "facet={}:{}",
                        percent_encode(field),
                        percent_encode(value)
                    ));
                }
                if pairs.is_empty() {
                    "#/search".into()
                } else {
                    format!("#/search?{}", pairs.join("&"))
                }
            }
            Route::BrowseKind { kind, letter } => match letter {
                Some(letter) => format!(
                    "#/browse/kind/{}?l={}",
                    kind.slug(),
                    percent_encode(&letter.to_string())
                ),
                None => format!("#/browse/kind/{}", kind.slug()),
            },
            Route::BrowseSource(id) => format!("#/browse/source/{}", percent_encode(id)),
            Route::Books => "#/books".into(),
            Route::BooksSorted(sort) => format!("#/books?sort={}", sort.slug()),
            Route::Book { source, chapter } => match chapter {
                Some(path) if !path.is_empty() => format!(
                    "#/browse/book/{}?c={}",
                    percent_encode(source),
                    format_chapter_path(path)
                ),
                _ => format!("#/browse/book/{}", percent_encode(source)),
            },
            Route::Spells => "#/spells".into(),
            Route::SpellList { class, level } => match level {
                Some(level) => format!("#/spells/{}/{level}", percent_encode(class)),
                None => format!("#/spells/{}", percent_encode(class)),
            },
            Route::FeatChains => "#/feats/chains".into(),
            Route::Entry { id, anchor } => match anchor {
                Some(anchor) => format!("#/entry/{id}?a={}", percent_encode(anchor)),
                None => format!("#/entry/{id}"),
            },
            Route::Bookmarks => "#/bookmarks".into(),
            Route::Collection(id) => format!("#/collection/{}", percent_encode(id)),
            Route::Settings => "#/settings".into(),
            Route::Legal => "#/legal".into(),
            Route::NotFound(hash) if hash.starts_with("#/") => hash.clone(),
            Route::NotFound(_) => "#/".into(),
        }
    }

    /// The anchor to scroll to after the route renders.
    pub fn anchor(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Route::Entry { anchor, .. } => anchor.as_deref().map(std::borrow::Cow::Borrowed),
            Route::Book {
                chapter: Some(path),
                ..
            } if !path.is_empty() => Some(std::borrow::Cow::Owned(toc_id(path))),
            _ => None,
        }
    }
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

/// The DOM id of a contents node, so that a chapter link can scroll to it.
pub fn toc_id(path: &[usize]) -> String {
    format!(
        "toc-{}",
        path.iter()
            .map(|i| (i + 1).to_string())
            .collect::<Vec<_>>()
            .join("-")
    )
}

fn parse_search(query: Option<&str>) -> Route {
    let (mut q, mut kinds, mut sources, mut facets) =
        (String::new(), Vec::new(), Vec::new(), Vec::new());
    for (key, value) in query.into_iter().flat_map(query_pairs) {
        match key.as_str() {
            "q" => q = value,
            "kind" => kinds.extend(EntryKind::from_slug(&value)),
            "source" => sources.push(value),
            _ => {}
        }
    }
    // The field and value are encoded separately, so split on the first raw ':' before decoding.
    for raw in query.into_iter().flat_map(|q| q.split('&')) {
        if let Some(raw) = raw.strip_prefix("facet=")
            && let Some((field, value)) = raw.split_once(':')
            && let (Some(field), Some(value)) = (percent_decode(field), percent_decode(value))
        {
            let value = if field == "school" {
                legacy_school(&value)
            } else {
                value
            };
            facets.push((field, value));
        }
    }
    Route::Search {
        q,
        kinds,
        sources,
        facets,
    }
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
        || hash == "#/browse/books";
    if legacy && let Some(history) = web_sys::window().and_then(|window| window.history().ok()) {
        let _ =
            history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&route.href()));
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

/// Pushes a history entry, like clicking an anchor with `route.href()`.
pub fn navigate(route: &Route) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_hash(&route.href());
    }
}

/// Replaces the current history entry, such as while a search box types.
/// `replaceState` fires no `hashchange`, so this updates the route signal itself.
pub fn replace(route: &Route) {
    let Some(history) = web_sys::window().and_then(|w| w.history().ok()) else {
        return;
    };
    // Keep the entry's state, which carries its scroll-memory id.
    let state = history.state().unwrap_or(wasm_bindgen::JsValue::NULL);
    let _ = history.replace_state_with_url(&state, "", Some(&route.href()));
    if let Some(write) = ROUTE_WRITE.with(|cell| cell.get()) {
        write.set(read_location_route());
    }
}

thread_local! {
    static SCROLL_MEMORY: std::cell::RefCell<ScrollMemory> = std::cell::RefCell::new(ScrollMemory::default());
}

/// The scroll offset of each history entry visited in this session. It lives in memory only:
/// a reload starts clean, and a pushed entry that was never visited here has no offset.
#[derive(Default)]
struct ScrollMemory {
    current: Option<u32>,
    saved: std::collections::HashMap<u32, f64>,
}

fn history_entry_id(history: &web_sys::History) -> Option<u32> {
    let state = history.state().ok()?;
    js_sys::Reflect::get(&state, &"scrollId".into())
        .ok()?
        .as_f64()
        .map(|id| id as u32)
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
/// visited before (Back or Forward) returns the offset it was left at. A new entry gets an id
/// in its history state and returns `None`.
pub fn arrive_at_entry() -> Option<f64> {
    let history = web_sys::window()?.history().ok()?;
    if let Some(id) = history_entry_id(&history) {
        return SCROLL_MEMORY.with(|memory| {
            let mut memory = memory.borrow_mut();
            memory.current = Some(id);
            memory.saved.get(&id).copied()
        });
    }
    let id = (js_sys::Math::random() * f64::from(u32::MAX)) as u32;
    let state = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&state, &"scrollId".into(), &f64::from(id).into());
    let _ = history.replace_state(&state, "");
    SCROLL_MEMORY.with(|memory| memory.borrow_mut().current = Some(id));
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
    element.scroll_into_view();
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
            Route::Settings,
            Route::Legal,
            Route::browse_kind(EntryKind::Spell),
            Route::BrowseSource("core-rulebook".into()),
            Route::Books,
            Route::BooksSorted(BookSort::Category),
            Route::BooksSorted(BookSort::Date),
            Route::BooksSorted(BookSort::Library),
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
    fn book_sorts_round_trip_and_unknown_values_fall_back_to_the_default_tiers() {
        assert_eq!(Route::Books.href(), "#/books");
        assert_eq!(
            Route::BooksSorted(BookSort::Category).href(),
            "#/books?sort=category"
        );
        assert_eq!(
            Route::BooksSorted(BookSort::Date).href(),
            "#/books?sort=date"
        );
        assert_eq!(
            Route::parse("#/books?sort=category"),
            Route::BooksSorted(BookSort::Category)
        );
        assert_eq!(
            Route::parse("#/books?sort=date"),
            Route::BooksSorted(BookSort::Date)
        );
        assert_eq!(
            Route::BooksSorted(BookSort::Library).href(),
            "#/books?sort=library"
        );
        assert_eq!(
            Route::parse("#/books?sort=library"),
            Route::BooksSorted(BookSort::Library)
        );
        // An unknown value is the default list, not an error or a second history entry.
        for hash in [
            "#/books?sort=title",
            "#/books?sort=",
            "#/books?other=date",
            "#/browse/books?sort=unknown",
        ] {
            assert_eq!(Route::parse(hash), Route::Books, "{hash}");
        }
        assert_eq!(
            Route::parse("#/browse/books?sort=date"),
            Route::BooksSorted(BookSort::Date)
        );
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
            chapter: Some(vec![6, 0, 11]),
        };
        assert_eq!(deep.href(), "#/browse/book/core-rulebook?c=7.1.12");
        round_trip(deep.clone());
        assert_eq!(deep.anchor().as_deref(), Some("toc-7-1-12"));
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
        assert_eq!(Route::Books.href(), "#/books");
        assert_eq!(Route::parse("#/browse/books"), Route::Books);
        assert_eq!(Route::parse("#/browse/kind/rule"), Route::Books);
        assert_eq!(
            Route::parse("#/browse/book"),
            Route::NotFound("#/browse/book".into())
        );
    }

    #[test]
    fn spell_list_routes_encode_the_class_name() {
        let list = Route::SpellList {
            class: "Elementalist Wizard".into(),
            level: Some(3),
        };
        assert_eq!(list.href(), "#/spells/Elementalist%20Wizard/3");
        round_trip(list);
        let all = Route::SpellList {
            class: "Wizard".into(),
            level: None,
        };
        assert_eq!(all.href(), "#/spells/Wizard");
        round_trip(all);
        round_trip(Route::SpellList {
            class: "Witch".into(),
            level: Some(0),
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
        };
        assert_eq!(
            route.href(),
            "#/entry/core-rulebook/spells/fireball?a=caster%20level%20%26%20more%2F%C3%A9"
        );
        round_trip(route);
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
        });
        round_trip(Route::search(""));
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
                facets: vec![]
            }
        );
    }

    #[test]
    fn trailing_slashes_are_tolerated() {
        assert_eq!(Route::parse("#/settings/"), Route::Settings);
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
