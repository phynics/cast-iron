//! "Continue reading" per book.
//!
//! A point is written only when an entry route carries `?book=<source>`, which is how a visit
//! from a book page, a chapter page or a previous/next link arrives (#161, #346). Opening the
//! same entry from search leaves the stored point alone.
//!
//! The reader keeps one point per book, in the `meta` store next to `recent`. Quick search shows
//! the most recently read book's point as the first recently viewed row, and the book page shows
//! that book's own point. Stored ids resolve through the alias map; a point whose entry the
//! library no longer holds is hidden. Resolution happens on read, so a stale row never reaches
//! the screen.
//!
//! A point also needs its book to still hold its entry (#543). A library update can remove an
//! entry from a book it once sat in; the entry still opens by id, but "where I stopped in that
//! book" no longer names a place in it. Such a point is hidden on read, and a visit from an old
//! book-qualified address records no point for the book that dropped the entry. The stored row
//! is left as it is, so a library that restores the membership shows the point again, and the
//! entry itself stays in Recently viewed, which has no book. Falling back to the entry without
//! a book was rejected: the book page and the Continue row both say "Continue <book>", which
//! would then open outside the book and lose the reading footer under a label that claims it.

use crate::library::{Library, Resolution};
use crate::store::{self, ContinuePoint};
use cast_iron_schema::EntryId;

/// The id a stored point shows and opens under: the entry itself when the catalog still has it,
/// the alias target when the map sends it there, and `None` when the entry is gone or retired.
pub fn live_target(library: &Library, entry_id: &EntryId) -> Option<EntryId> {
    match library.resolve(entry_id) {
        Resolution::Canonical(id) => Some(id),
        Resolution::Aliased { canonical, .. } if library.stub(&canonical).is_some() => {
            Some(canonical)
        }
        _ => None,
    }
}

/// The first content schema whose catalog stubs list their member books (#394). An older
/// stub has none, so the catalog cannot say that a book dropped an entry.
const MEMBERSHIP_SCHEMA: u32 = 5;

/// Whether a book still holds an entry, given the library's content schema and whether the
/// catalog lists the entry among the book's members. A library older than schema 5 cannot say
/// a book dropped the entry (a reader can keep one, because updates are never automatic), so it
/// keeps every point and records every visit as before. Stale memberships only arise from a
/// library that lists them.
fn book_holds(schema_version: u32, listed: bool) -> bool {
    schema_version < MEMBERSHIP_SCHEMA || listed
}

/// Whether the library's catalog lists the entry among the member books of `source`, active or
/// not. An entry the catalog lacks is in no book. A library without member books (schema 4 and
/// older) says nothing against any book, so this holds there.
pub fn entry_in_book(library: &Library, entry_id: &EntryId, source: &str) -> bool {
    let listed = library
        .stub(entry_id)
        .is_some_and(|stub| library.stub_is_in_source(stub, source));
    book_holds(library.manifest().schema_version, listed)
}

/// The book a visit may record a point under: the route's book while it still holds the entry,
/// otherwise `None`, so the visit leaves every point alone.
pub fn recordable_book<'a>(
    book: Option<&'a str>,
    entry_id: &EntryId,
    in_book: impl Fn(&EntryId, &str) -> bool,
) -> Option<&'a str> {
    book.filter(|source| in_book(entry_id, source))
}

/// Every stored point that still resolves to a known book and entry that the book still holds,
/// newest first. A point for a book the library no longer installs is hidden the same way as a
/// missing entry.
pub async fn load(library: &Library) -> Vec<ContinuePoint> {
    let stored = store::continue_points().await.unwrap_or_default();
    // The book and entry checks run inside the resolution, before a book's newest point shadows
    // an older one stored under a retired id of the same book.
    store::resolve_continue_points(
        &stored,
        |entry_id| live_target(library, entry_id),
        |entry_id, source| {
            library.source(source).is_some() && entry_in_book(library, entry_id, source)
        },
    )
}

/// The point for one book, when the reader has one there.
pub fn for_book<'a>(points: &'a [ContinuePoint], source: &str) -> Option<&'a ContinuePoint> {
    points.iter().find(|point| point.source == source)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(source: &str, entry: &str, read_at: f64) -> ContinuePoint {
        ContinuePoint {
            source: source.into(),
            entry_id: EntryId::parse(entry).unwrap(),
            read_at,
        }
    }

    /// A library where `ref/rule/a` sits in book `a` only and `ref/rule/b` in `a` and `b`.
    fn holds(entry: &EntryId, source: &str) -> bool {
        matches!(
            (entry.as_str(), source),
            ("ref/rule/a", "a") | ("ref/rule/b", "a") | ("ref/rule/b", "b")
        )
    }

    #[test]
    fn a_library_without_member_books_never_says_a_book_dropped_an_entry() {
        // Schema 4 and older: the catalog has no member books, so an unlisted book proves nothing.
        assert!(book_holds(4, false));
        assert!(book_holds(4, true));
        // Schema 5 and newer: the catalog is the authority.
        assert!(!book_holds(5, false));
        assert!(!book_holds(6, false));
        assert!(book_holds(5, true));
        assert!(book_holds(6, true));
    }

    #[test]
    fn a_visit_records_under_the_route_book_only_while_the_book_holds_the_entry() {
        let a = EntryId::parse("ref/rule/a").unwrap();
        assert_eq!(recordable_book(Some("a"), &a, holds), Some("a"));
        assert_eq!(recordable_book(Some("b"), &a, holds), None);
        assert_eq!(recordable_book(None, &a, holds), None);
    }

    #[test]
    fn a_point_is_found_by_its_book_and_not_by_another() {
        let points = [point("a", "ref/rule/a", 2.0), point("b", "ref/rule/b", 1.0)];
        assert_eq!(
            for_book(&points, "a").map(|point| point.entry_id.as_str()),
            Some("ref/rule/a")
        );
        assert!(for_book(&points, "c").is_none());
    }
}
