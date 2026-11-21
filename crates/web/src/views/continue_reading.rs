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

/// Whether the library's catalog lists the entry among the member books of `source`, active or
/// not. An entry the catalog lacks is in no book.
pub fn entry_in_book(library: &Library, entry_id: &EntryId, source: &str) -> bool {
    library
        .stub(entry_id)
        .is_some_and(|stub| library.stub_is_in_source(stub, source))
}

/// The points whose book still holds their entry, in their order. `in_book` answers for an
/// already resolved entry id and book id. A point whose book dropped the entry is left out, and
/// every other point passes unchanged.
pub fn keep_member_points(
    points: Vec<ContinuePoint>,
    in_book: impl Fn(&EntryId, &str) -> bool,
) -> Vec<ContinuePoint> {
    points
        .into_iter()
        .filter(|point| in_book(&point.entry_id, &point.source))
        .collect()
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
    let resolved =
        store::resolve_continue_points(&stored, |entry_id| live_target(library, entry_id))
            .into_iter()
            .filter(|point| library.source(&point.source).is_some())
            .collect();
    keep_member_points(resolved, |entry_id, source| {
        entry_in_book(library, entry_id, source)
    })
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
    fn a_point_whose_book_dropped_the_entry_is_hidden_and_the_rest_keep_their_order() {
        let points = vec![
            point("b", "ref/rule/a", 3.0), // book b no longer holds a
            point("a", "ref/rule/b", 2.0),
            point("c", "ref/rule/b", 1.0), // book c never held b
        ];
        assert_eq!(
            keep_member_points(points, holds),
            [point("a", "ref/rule/b", 2.0)]
        );
    }

    #[test]
    fn a_point_whose_entry_and_book_still_match_is_never_lost() {
        let points = vec![point("a", "ref/rule/a", 2.0), point("b", "ref/rule/b", 1.0)];
        assert_eq!(keep_member_points(points.clone(), holds), points);
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
        let points = [
            ContinuePoint {
                source: "a".into(),
                entry_id: EntryId::parse("ref/rule/a").unwrap(),
                read_at: 2.0,
            },
            ContinuePoint {
                source: "b".into(),
                entry_id: EntryId::parse("ref/rule/b").unwrap(),
                read_at: 1.0,
            },
        ];
        assert_eq!(
            for_book(&points, "a").map(|point| point.entry_id.as_str()),
            Some("ref/rule/a")
        );
        assert!(for_book(&points, "c").is_none());
    }
}
