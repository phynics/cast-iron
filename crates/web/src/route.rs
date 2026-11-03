//! Hash routes. Pure parsing and formatting live here and are tested natively.
//! `use_route`, `navigate`, and the scroll helpers wrap the browser.
//!
//! Grammar (the hash always starts with `#/`):
//!
//! ```text
//! #/                                 Home
//! #/search?q=..&kind=spell&source=core-rulebook&facet=school:evocation
//! #/browse                           #/browse/kind/<slug>     #/browse/source/<id>
//! #/browse/books                     the list of books, in original order of the library
//! #/browse/book/<source>?c=3.2       a book's contents; `c` is the 1-based chapter path
//! #/spells                           #/spells/<class>         #/spells/<class>/<level>
//! #/feats/chains                     feats that unlock others
//! #/entry/<entry id>?a=<anchor>
//! #/bookmarks                        #/collection/<id>
//! #/settings                         #/legal
//! ```

use cast_iron_schema::{EntryId, EntryKind};
use leptos::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Home,
    Search {
        q: String,
        kinds: Vec<EntryKind>,
        sources: Vec<String>,
        facets: Vec<(String, String)>,
    },
    Browse,
    BrowseKind(EntryKind),
    /// Kept for old links. A library that has `books.json` shows the book view here.
    BrowseSource(String),
    /// The 16 books, each a doorway into its contents.
    Books,
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
            ["browse"] => Route::Browse,
            ["browse", "kind", slug] => percent_decode(slug)
                .and_then(|slug| EntryKind::from_slug(&slug))
                .map_or_else(not_found, Route::BrowseKind),
            ["browse", "source", id] => match percent_decode(id) {
                Some(id) if !id.is_empty() => Route::BrowseSource(id),
                _ => not_found(),
            },
            ["browse", "books"] => Route::Books,
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
            ["bookmarks"] => Route::Bookmarks,
            ["collection", id] => match percent_decode(id) {
                Some(id) if !id.is_empty() => Route::Collection(id),
                _ => not_found(),
            },
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
            Route::Browse => "#/browse".into(),
            Route::BrowseKind(kind) => format!("#/browse/kind/{}", kind.slug()),
            Route::BrowseSource(id) => format!("#/browse/source/{}", percent_encode(id)),
            Route::Books => "#/browse/books".into(),
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
    let (read, write) = signal(Route::parse(&current_hash()));
    // Back and forward fire `hashchange` too, so the browser's history stays the only history.
    let handle = window_event_listener(leptos::ev::hashchange, move |_| {
        write.set(Route::parse(&current_hash()));
    });
    on_cleanup(move || handle.remove());
    provide_context(RouteContext { read });
    // `replace` runs from async tasks that have no reactive owner, where `use_context` fails.
    ROUTE_WRITE.with(|cell| cell.set(Some(write)));
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
    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&route.href()));
    if let Some(write) = ROUTE_WRITE.with(|cell| cell.get()) {
        write.set(Route::parse(&current_hash()));
    }
}

pub fn go_back() {
    if let Some(history) = web_sys::window().and_then(|w| w.history().ok()) {
        let _ = history.back();
    }
}

/// Scrolls the element with this id into view. Returns false when it is not in the document yet.
/// An entry view calls this after its content renders.
pub fn scroll_to_anchor(anchor: &str) -> bool {
    let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(anchor))
    else {
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
            Route::Browse,
            Route::Bookmarks,
            Route::Settings,
            Route::Legal,
            Route::BrowseKind(EntryKind::Spell),
            Route::BrowseSource("core-rulebook".into()),
            Route::Collection("3f2a-uuid".into()),
            Route::Books,
            Route::Spells,
            Route::FeatChains,
        ] {
            round_trip(route);
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
        assert_eq!(Route::Books.href(), "#/browse/books");
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
