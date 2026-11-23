//! `Find a book` (#587): one field on Books and on Manage books that narrows the book groups by
//! name.
//!
//! The text is route state (`?q=`), so Back from a book page finds the same rows. Typing replaces
//! the history entry, so no key makes an entry of its own, and the write waits for the next
//! animation frame, so a fast typist rebuilds the groups once per frame, not once per key.
//!
//! A book matches when the text is part of its full title or of its short display title
//! (`#1 Burnt Offerings`), without regard to case or accents. The find never re-sorts: matches
//! keep book order inside each group. The matcher and the lines are pure and native-tested; the
//! groups are drawn by `browse::groups_view`.

use crate::fmt;
use crate::route::{Route, replace, use_route};
use crate::views::book_series::display_title;
use crate::views::browse::BookListEntry;
use crate::views::filter_menu::focus_by_id;
use crate::views::util::Shared;
use cast_iron_search::normalize;
use cast_iron_sort::set_of;
use leptos::prelude::*;
use std::collections::BTreeSet;

/// The DOM id of the field on Books and on Manage books.
pub const FIELD_ID: &str = "book-find";

/// The text to look for, ready to compare: lower case, without accents or punctuation. Empty
/// when the field holds nothing that can match, which is no search.
pub fn needle(text: &str) -> String {
    normalize(text)
}

/// Whether `book` matches the needle: its title or its display title holds it.
pub fn matches(book: &BookListEntry, needle: &str) -> bool {
    normalize(&book.title).contains(needle) || normalize(&display_title(book)).contains(needle)
}

/// The ids of the books that match, for the group views.
pub fn matching_ids(books: &[BookListEntry], needle: &str) -> BTreeSet<String> {
    set_of(
        books
            .iter()
            .filter(|book| matches(book, needle))
            .map(|book| book.id.clone()),
    )
}

/// The status line: `12 books match "bestiary"`, or `No books match "xyz".`
pub fn status_line(found: usize, text: &str) -> String {
    let text = text.trim();
    match found {
        0 => format!("No books match \"{text}\"."),
        1 => format!("1 book matches \"{text}\""),
        n => format!("{} books match \"{text}\"", fmt::count(n)),
    }
}

/// The count at the right of a group heading while a search is on: `3 matches`.
pub fn match_count(found: usize) -> String {
    format!(
        "{} {}",
        fmt::count(found),
        if found == 1 { "match" } else { "matches" }
    )
}

/// Whether `next` is the same page as `previous` with another search text: typing writes the
/// text into the address of the page that is on screen, and the frame must not scroll that page
/// to the top or move its focus. The removed-book line is the same page as Books, and a share
/// link's question stays when the reader types (the address then loses the code).
pub fn same_page(previous: &Route, next: &Route) -> bool {
    matches!(
        (previous, next),
        (
            Route::Books { .. } | Route::BookRemoved(_),
            Route::Books { .. }
        ) | (Route::BooksManage { .. }, Route::BooksManage { .. })
    ) || matches!((previous, next), (Route::BooksSet(_), Route::BooksManage { find }) if !find.is_empty())
}

/// Which page the field belongs to. A search on Books never writes the Manage books address.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Books,
    Manage,
}

/// The state of the field: the text and the needle read from the route, and the way back into
/// the route. It is `Copy`, so each handler takes its own.
#[derive(Clone, Copy)]
pub struct Find {
    page: Page,
    route: ReadSignal<Route>,
    /// The text of the field, as the route holds it.
    pub text: Memo<String>,
    /// [`needle`] of the text. A key that changes only punctuation leaves it, and the groups,
    /// alone.
    pub needle: Memo<String>,
    /// The text that waits for the next frame, if a key has been typed since the last write.
    typed: StoredValue<Option<String>>,
}

impl Find {
    pub fn new(page: Page) -> Find {
        let route = use_route();
        let text = Memo::new(move |_| match (page, route.get()) {
            (Page::Books, Route::Books { find }) | (Page::Manage, Route::BooksManage { find }) => {
                find
            }
            _ => String::new(),
        });
        let needle = Memo::new(move |_| needle(&text.get()));
        Find {
            page,
            route,
            text,
            needle,
            typed: StoredValue::new(None),
        }
    }

    /// Whether the groups are narrowed now.
    pub fn active(&self) -> bool {
        !self.needle.get().is_empty()
    }

    /// Writes the text into the address: the history entry is replaced.
    fn write(&self, text: String) {
        // Read the route as it is now, not as it was when this field rendered: a link pushes the
        // address before the page leaves, so a stale field could type into the next page. The
        // reader may also have left the page in this frame. The removed-book line and a share
        // link are the same page as Books and Manage books; the text replaces their address.
        let target = match (self.page, self.route.get_untracked()) {
            (Page::Books, Route::Books { .. } | Route::BookRemoved(_)) => {
                Route::Books { find: text }
            }
            (Page::Manage, Route::BooksManage { .. } | Route::BooksSet(_)) => {
                Route::BooksManage { find: text }
            }
            _ => return,
        };
        replace(&target);
    }

    /// A key was typed: write the text on the next animation frame.
    fn type_text(&self, text: String) {
        let waiting = self.typed.with_value(Option::is_some);
        self.typed.set_value(Some(text));
        if waiting {
            return;
        }
        let find = *self;
        request_animation_frame(move || {
            if let Some(text) = find.typed.try_update_value(Option::take).flatten() {
                find.write(text);
            }
        });
    }

    /// Empties the field now, and drops a key that still waits for its frame.
    pub fn clear(&self) {
        self.typed.set_value(None);
        self.write(String::new());
    }

    /// The labelled field.
    pub fn field(self) -> AnyView {
        view! {
            <div class="book-find">
                <label for=FIELD_ID>"Find a book"</label>
                <input
                    id=FIELD_ID
                    type="search"
                    placeholder="Find a book"
                    autocomplete="off"
                    autocapitalize="off"
                    spellcheck="false"
                    enterkeyhint="search"
                    prop:value=move || self.text.get()
                    on:input=move |event| self.type_text(event_target_value(&event))
                    on:keydown=move |event| {
                        // The field itself may hold a key that the route has not seen yet.
                        if event.key() == "Escape" && !event_target_value(&event).is_empty() {
                            event.prevent_default();
                            self.clear();
                        }
                    }
                />
            </div>
        }
        .into_any()
    }

    /// The status line under the field. It is always in the page, so a screen reader announces
    /// the text that appears in it; `found` is the number of matching books.
    pub fn status(self, found: impl Fn() -> usize + Send + Sync + 'static) -> AnyView {
        view! {
            <p class="status book-find-status" role="status" class:book-find-on=move || self.active()>
                {move || self.active().then(|| status_line(found(), &self.text.get()))}
                {move || self.active().then(|| view! {
                    " "
                    <button type="button" class="count-action"
                        on:click=move |_| {
                            self.clear();
                            focus_by_id(FIELD_ID);
                        }>"Clear"</button>
                })}
            </p>
        }
        .into_any()
    }
}

/// Runs `run` with the matching ids whenever the needle changes: `None` while no search is on.
/// The page uses it to draw the groups again.
pub fn matched(
    rows: Shared<Vec<BookListEntry>>,
    needle: Memo<String>,
) -> impl Fn() -> Option<Shared<BTreeSet<String>>> + Clone + Send + Sync + 'static {
    move || {
        let needle = needle.get();
        (!needle.is_empty()).then(|| Shared::new(matching_ids(&rows, &needle)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::SeriesKind;

    fn book(
        id: &str,
        title: &str,
        kind: Option<SeriesKind>,
        number: Option<&str>,
    ) -> BookListEntry {
        BookListEntry {
            id: id.into(),
            title: title.into(),
            category: None,
            publication_date: None,
            count: 0,
            third_party: false,
            series_kind: kind,
            series: None,
            series_number: number.map(String::from),
            season: None,
        }
    }

    fn ids(books: &[BookListEntry], text: &str) -> Vec<String> {
        matching_ids(books, &needle(text)).into_iter().collect()
    }

    #[test]
    fn a_title_matches_without_regard_to_case_or_accents() {
        let books = [
            book("bestiary-1", "Pathfinder RPG Bestiary", None, None),
            book("ex", "Ultimate Équipment", None, None),
        ];
        assert_eq!(ids(&books, "BESTIARY"), ["bestiary-1"]);
        assert_eq!(ids(&books, "equipment"), ["ex"]);
        assert_eq!(ids(&books, "équip"), ["ex"]);
        // A word in the middle, and two words, match as part of the title.
        assert_eq!(ids(&books, "rpg best"), ["bestiary-1"]);
        assert!(ids(&books, "bestiary 2").is_empty());
    }

    #[test]
    fn the_short_title_of_an_adventure_matches_too() {
        let path = book(
            "burnt",
            "Pathfinder #1: Burnt Offerings",
            Some(SeriesKind::AdventurePath),
            Some("1"),
        );
        let module = book(
            "academy",
            "Pathfinder Module: Academy of Secrets",
            Some(SeriesKind::Module),
            None,
        );
        let books = [path, module];
        // `#1 Burnt Offerings` is the row's title on Books; the full title has a colon in it.
        assert_eq!(ids(&books, "#1 Burnt"), ["burnt"]);
        assert_eq!(ids(&books, "Burnt Offerings"), ["burnt"]);
        assert_eq!(ids(&books, "Academy of Secrets"), ["academy"]);
        // The full title still matches the prefix that the row drops.
        assert_eq!(ids(&books, "pathfinder module"), ["academy"]);
    }

    #[test]
    fn a_text_with_nothing_to_match_is_no_search() {
        assert_eq!(needle(""), "");
        assert_eq!(needle("   "), "");
        assert_eq!(needle("!?"), "");
        assert_eq!(needle(" Bestiary "), "bestiary");
    }

    #[test]
    fn a_new_text_is_the_same_page_and_a_new_page_is_not() {
        let books = |find: &str| Route::Books { find: find.into() };
        let manage = |find: &str| Route::BooksManage { find: find.into() };
        assert!(same_page(&books(""), &books("a")));
        assert!(same_page(&books("a"), &books("")));
        assert!(same_page(&manage(""), &manage("a")));
        assert!(same_page(&Route::BookRemoved("x".into()), &books("a")));
        // Typing beside a link's question keeps the page; answering it (an empty text) is not typing.
        assert!(same_page(&Route::BooksSet("C".into()), &manage("a")));
        assert!(!same_page(&Route::BooksSet("C".into()), &manage("")));
        assert!(!same_page(&books("a"), &manage("a")));
        assert!(!same_page(&manage("a"), &books("a")));
        assert!(!same_page(&Route::Settings, &books("a")));
    }

    #[test]
    fn the_lines_name_the_count_and_the_text() {
        assert_eq!(status_line(12, " bestiary "), "12 books match \"bestiary\"");
        assert_eq!(status_line(1, "core"), "1 book matches \"core\"");
        assert_eq!(status_line(0, "xyz123"), "No books match \"xyz123\".");
        assert_eq!(status_line(1_234, "a"), "1,234 books match \"a\"");
        assert_eq!(match_count(3), "3 matches");
        assert_eq!(match_count(1), "1 match");
    }
}
