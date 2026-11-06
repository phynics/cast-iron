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

/// How close two engine scores must be to count as a near tie: within this share of the best
/// score of the run. `fireb` scores Firebrand 522.5, Fireball 518.7 and Firebrand (spell) 518.2,
/// which the engine ranks by a margin of under one percent; the next title scores 515.7.
const NEAR_TIE: f32 = 0.01;

fn near_tie(best: f32, score: f32) -> bool {
    (best - score).abs() <= NEAR_TIE * best.abs().max(1.0)
}

/// Among hits whose engine scores are within [`NEAR_TIE`] of the best of their run and whose
/// title starts with the typed text, puts the shorter title first: `fireb` completes to
/// Fireball before Firebrand. A run starts at the first hit that is not within the band of the
/// run before it. Everything else keeps the engine's order, so only a near tie is broken and the
/// ranking is never reversed across a real score gap. The results page does not use this.
pub fn shortest_completion_first(hits: Vec<HitSummary>, typed: &str) -> Vec<HitSummary> {
    let typed = normalized_title(typed);
    let completes = |hit: &HitSummary| normalized_title(&hit.title).starts_with(&typed);
    let mut out = Vec::with_capacity(hits.len());
    let mut hits = hits.into_iter().peekable();
    while let Some(first) = hits.next() {
        let best = first.score;
        let mut run = vec![first];
        while let Some(next) = hits.next_if(|hit| near_tie(best, hit.score)) {
            run.push(next);
        }
        // A stable sort: completions first, shortest first, the rest in engine order.
        run.sort_by_key(|hit| {
            if completes(hit) {
                (0, hit.title.chars().count())
            } else {
                (1, 0)
            }
        });
        out.extend(run);
    }
    out
}

/// The highlight if it still names a row of a list of `len` rows.
pub fn valid_active(active: Option<usize>, len: usize) -> Option<usize> {
    active.filter(|&index| index < len)
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

    fn scored(id: &str, title: &str, score: f32) -> HitSummary {
        HitSummary {
            score,
            ..hit(id, title, "spell")
        }
    }

    fn ids(hits: &[HitSummary]) -> Vec<&str> {
        hits.iter().map(|hit| hit.id.as_str()).collect()
    }

    #[test]
    fn in_a_near_tie_the_shorter_completion_comes_first() {
        // The engine's real scores for `fireb`: Firebrand just ahead of Fireball.
        let engine = vec![
            scored("brand-arch", "Firebrand", 522.5),
            scored("ball", "Fireball", 518.68),
            scored("brand-spell", "Firebrand", 518.16),
            scored("trap", "Fireball Trap", 515.7),
        ];
        let ranked = shortest_completion_first(engine, "fireb");
        // Fireball leads the first run; Fireball Trap (515.7) is within 1% of 522.5 as well and
        // is longer than both, so it follows them.
        assert_eq!(ids(&ranked), ["ball", "brand-arch", "brand-spell", "trap"]);
        // When the text is the longer word, only Firebrand completes it and stays first.
        let engine = vec![
            scored("brand", "Firebrand", 600.0),
            scored("ball", "Fireball", 590.0),
        ];
        assert_eq!(
            ids(&shortest_completion_first(engine, "firebrand")),
            ["brand", "ball"]
        );
    }

    #[test]
    fn a_real_score_gap_is_never_crossed() {
        let engine = vec![
            scored("brand", "Firebrand", 2.1),
            scored("ball", "Fireball", 2.0),
        ];
        assert_eq!(
            ids(&shortest_completion_first(engine, "fireb")),
            ["brand", "ball"]
        );
        let engine = vec![
            scored("a", "Fire Shield Greater", 300.0),
            scored("b", "Fireball", 200.0),
            scored("c", "Wall of Fire", 199.0),
            scored("d", "Firebrand", 198.5),
            scored("e", "Fire", 100.0),
        ];
        // Within the 200 run, completions go first (shortest first); `Wall of Fire` does not
        // start with the text and keeps its place after them. Other runs stay put.
        assert_eq!(
            ids(&shortest_completion_first(engine, "fire")),
            ["a", "b", "d", "c", "e"]
        );
        // Equal length completions keep the engine's order.
        let engine = vec![scored("x", "Firefly", 1.0), scored("y", "Firebug", 1.0)];
        assert_eq!(ids(&shortest_completion_first(engine, "fire")), ["x", "y"]);
    }

    #[test]
    fn the_highlight_never_names_a_missing_row() {
        assert_eq!(valid_active(None, 3), None);
        assert_eq!(valid_active(Some(2), 3), Some(2));
        assert_eq!(valid_active(Some(3), 3), None);
        assert_eq!(valid_active(Some(0), 0), None);
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
