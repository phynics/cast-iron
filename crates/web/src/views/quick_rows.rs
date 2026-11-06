//! What the Quick search list shows: the rows, and the keyboard movement through them. No
//! browser calls, so the rules are tested natively.

use crate::views::source_variants::normalized_title;
use cast_iron_schema::EntryId;
use cast_iron_search::HitSummary;

/// Suggestions shown under the field while the reader types.
pub const MAX_SUGGESTIONS: usize = 8;

/// An entry offered while typing.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub id: EntryId,
    pub title: String,
    /// The muted line: the kind, then the book.
    pub detail: String,
}

/// One row of the list. Its position is the index that Up and Down move through.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    /// Opens the entry.
    Entry(Suggestion),
    /// "See all results for …": opens the results page for the typed text.
    SeeAll(String),
    /// A recent or starter search: runs that search.
    Search(String),
}

/// Which list the layer is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Suggestions,
    Recent,
    Starters,
}

/// The rows for the field's text: suggestions with a final "See all" row while the reader has
/// typed something, otherwise the recent searches, otherwise the starter searches.
pub fn rows(
    text: &str,
    suggestions: &[Suggestion],
    recents: &[String],
    starters: &[&str],
) -> (Mode, Vec<Row>) {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if !text.is_empty() {
        let mut rows: Vec<Row> = suggestions.iter().cloned().map(Row::Entry).collect();
        rows.push(Row::SeeAll(text));
        (Mode::Suggestions, rows)
    } else if !recents.is_empty() {
        (
            Mode::Recent,
            recents.iter().cloned().map(Row::Search).collect(),
        )
    } else {
        (
            Mode::Starters,
            starters
                .iter()
                .map(|s| Row::Search((*s).to_owned()))
                .collect(),
        )
    }
}

/// Keeps the best-ranked hit of each title and kind, so a spell that several books reprint is
/// one suggestion, and stops at `limit`.
pub fn dedupe_hits(hits: Vec<HitSummary>, limit: usize) -> Vec<HitSummary> {
    let mut seen = std::collections::BTreeSet::new();
    hits.into_iter()
        .filter(|hit| seen.insert((hit.kind.clone(), normalized_title(&hit.title))))
        .take(limit)
        .collect()
}

/// Moves the highlight. Down from the field highlights the first row and stops at the last.
/// Up from the field highlights the last row, and Up from the first row returns to the field.
pub fn step(active: Option<usize>, len: usize, down: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    match (active, down) {
        (None, true) => Some(0),
        (None, false) => Some(len - 1),
        (Some(i), true) => Some((i + 1).min(len - 1)),
        (Some(0), false) => None,
        (Some(i), false) => Some(i - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str, title: &str, kind: &str) -> HitSummary {
        HitSummary {
            ord: 0,
            id: id.to_owned(),
            title: title.to_owned(),
            kind: kind.to_owned(),
            source: "core".to_owned(),
            score: 1.0,
        }
    }

    fn suggestion(title: &str) -> Suggestion {
        Suggestion {
            id: EntryId::parse(format!("ref/{}", title.to_lowercase())).unwrap(),
            title: title.to_owned(),
            detail: "Spell · Core".to_owned(),
        }
    }

    #[test]
    fn typing_shows_suggestions_then_see_all() {
        let (mode, list) = rows(
            "  fireb ",
            &[suggestion("Fireball")],
            &["old".to_owned()],
            &["x"],
        );
        assert_eq!(mode, Mode::Suggestions);
        assert_eq!(
            list,
            [
                Row::Entry(suggestion("Fireball")),
                Row::SeeAll("fireb".to_owned())
            ]
        );
    }

    #[test]
    fn typing_with_no_suggestion_still_offers_see_all() {
        let (mode, list) = rows("zzzz", &[], &[], &["x"]);
        assert_eq!(mode, Mode::Suggestions);
        assert_eq!(list, [Row::SeeAll("zzzz".to_owned())]);
    }

    #[test]
    fn an_empty_field_shows_recents_then_starters() {
        let recents = vec!["grapple".to_owned(), "fireball".to_owned()];
        let (mode, list) = rows("", &[], &recents, &["dragon"]);
        assert_eq!(mode, Mode::Recent);
        assert_eq!(
            list,
            [
                Row::Search("grapple".to_owned()),
                Row::Search("fireball".to_owned())
            ]
        );
        let (mode, list) = rows("   ", &[], &[], &["dragon", "fireball"]);
        assert_eq!(mode, Mode::Starters);
        assert_eq!(
            list,
            [
                Row::Search("dragon".to_owned()),
                Row::Search("fireball".to_owned())
            ]
        );
    }

    #[test]
    fn reprints_collapse_to_the_best_hit_and_the_list_is_capped() {
        let mut hits = vec![
            hit("a/fireball", "Fireball", "spell"),
            hit("b/fireball", "FIREBALL", "spell"),
            hit("a/fireball-feat", "Fireball", "feat"),
        ];
        for n in 0..12 {
            hits.push(hit(&format!("a/e{n}"), &format!("Entry {n}"), "rule"));
        }
        let kept = dedupe_hits(hits, MAX_SUGGESTIONS);
        assert_eq!(kept.len(), MAX_SUGGESTIONS);
        assert_eq!(kept[0].id, "a/fireball");
        assert_eq!(kept[1].id, "a/fireball-feat");
        assert!(kept.iter().all(|hit| hit.id != "b/fireball"));
    }

    #[test]
    fn the_highlight_moves_and_stops_at_the_ends() {
        assert_eq!(step(None, 0, true), None);
        assert_eq!(step(None, 3, true), Some(0));
        assert_eq!(step(Some(0), 3, true), Some(1));
        assert_eq!(step(Some(2), 3, true), Some(2));
        assert_eq!(step(Some(1), 3, false), Some(0));
        assert_eq!(step(Some(0), 3, false), None);
        assert_eq!(step(None, 3, false), Some(2));
    }
}
