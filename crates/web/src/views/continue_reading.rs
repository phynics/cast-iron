//! "Continue reading" per book.
//!
//! A point is written only when an entry route carries `?book=<source>`, which is how a visit
//! from a book page, a chapter page or a previous/next link arrives (#161, #346). Opening the
//! same entry from search leaves the stored point alone.
//!
//! The reader keeps one point per book, in the `meta` store next to `recent`. Home shows the
//! most recently read book and the book page shows that book's own point. Stored ids resolve
//! through the alias map; a point whose entry the library no longer holds is hidden. Resolution
//! happens on read, so a stale row never reaches the screen.

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

/// Every stored point that still resolves to a known book and entry, newest first. A point for a
/// book the library no longer installs is hidden the same way as a missing entry.
pub async fn load(library: &Library) -> Vec<ContinuePoint> {
    let stored = store::continue_points().await.unwrap_or_default();
    store::resolve_continue_points(&stored, |entry_id| live_target(library, entry_id))
        .into_iter()
        .filter(|point| library.source(&point.source).is_some())
        .collect()
}

/// The point for one book, when the reader has one there.
pub fn for_book<'a>(points: &'a [ContinuePoint], source: &str) -> Option<&'a ContinuePoint> {
    points.iter().find(|point| point.source == source)
}

#[cfg(test)]
mod tests {
    use super::*;

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
