//! Starred items on Home. A starred entry whose entry is gone from the current library still
//! shows, from its saved title, and is never deleted by a content change.

use crate::store::Bookmark;
use cast_iron_schema::EntryId;

/// One row per saved item, newest first. An aliased row collapses onto its canonical target
/// and shows the target's id, while a retired or unknown row keeps its own id and snapshot.
pub fn shown_items(
    bookmarks: &[Bookmark],
    target_of: impl Fn(&EntryId) -> EntryId,
) -> Vec<Bookmark> {
    let mut out: Vec<Bookmark> = Vec::new();
    for bookmark in bookmarks {
        let target = target_of(&bookmark.entry_id);
        if !out.iter().any(|shown| shown.entry_id == target) {
            out.push(Bookmark {
                entry_id: target,
                ..bookmark.clone()
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::EntryKind;

    fn row(entry: &str, title: &str, at: f64) -> Bookmark {
        Bookmark::new(EntryId::parse(entry).unwrap(), title, EntryKind::Spell, at)
    }

    #[test]
    fn shown_items_collapse_aliases_and_keep_retired_rows_under_their_own_id() {
        let rows = vec![
            row("legacy/a", "Old", 3.0),
            row("ref/rule/a", "New", 2.0),
            row("legacy/retired", "Gone", 1.0),
        ];
        let target_of = |stored: &EntryId| {
            if stored.as_str() == "legacy/a" {
                EntryId::parse("ref/rule/a").unwrap()
            } else {
                stored.clone()
            }
        };
        let shown = shown_items(&rows, target_of);
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].entry_id, EntryId::parse("ref/rule/a").unwrap());
        assert_eq!(shown[0].title_snapshot, "Old"); // the newest row of the group
        assert_eq!(shown[1].entry_id, EntryId::parse("legacy/retired").unwrap());
        assert_eq!(shown[1].title_snapshot, "Gone");
    }
}
