//! What the Quick search list shows: the rows, and the keyboard movement through them. No
//! browser calls, so the rules are tested natively.

use crate::views::recent_searches::RecentSearch;
use crate::views::source_variants::normalized_title;
use cast_iron_schema::EntryId;
use cast_iron_search::HitSummary;

/// Suggestions shown under the field while the reader types.
pub const MAX_SUGGESTIONS: usize = 8;
/// Recently viewed rows shown before "Show more".
pub const MAX_RECENT_VIEWED: usize = 5;

/// An entry offered while typing.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub id: EntryId,
    pub title: String,
    /// The muted line: the kind, then the book.
    pub detail: String,
}

/// One recently viewed entry, resolved for display.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewedRow {
    pub id: EntryId,
    pub title: String,
    /// The muted line: the kind and book for a visit, the entry title for the Continue row.
    pub detail: String,
    /// The book of the Continue row, which its link carries in its URL.
    pub book: Option<String>,
}

/// One row of the list. Its position is the index that Up and Down move through.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    /// Opens the entry.
    Entry(Suggestion),
    /// "Continue {book}": opens the entry with that book's context.
    Continue(ViewedRow),
    /// A recently viewed entry.
    Viewed(ViewedRow),
    /// "See all results for …": opens the results page for the typed text.
    SeeAll(String),
    /// A recent search: runs it again in the scope it ran in.
    Search(RecentSearch),
    /// A starter search, offered when nothing is recent: runs in the scope the layer has.
    Starter(String),
}

/// What one group of the list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Suggestions,
    Recent,
    Starters,
    Viewed,
}

impl GroupKind {
    /// The visible heading, and the listbox's accessible name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Suggestions => "Suggestions",
            Self::Recent => "Recent searches",
            Self::Starters => "Try searching for",
            Self::Viewed => "Recently viewed",
        }
    }

    /// The suffix of the group's listbox and heading ids.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Suggestions => "suggestions",
            Self::Recent => "recent",
            Self::Starters => "starters",
            Self::Viewed => "viewed",
        }
    }
}

/// One group: a heading and the rows under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub kind: GroupKind,
    pub rows: Vec<Row>,
}

/// The whole list: the groups in display order, and how much of Recently viewed is folded away.
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub groups: Vec<Group>,
    /// Recently viewed rows that the five-row cap hides.
    pub hidden_viewed: usize,
    /// How many rows the Recently viewed group holds in total.
    pub viewed_total: usize,
}

impl List {
    /// True while the reader has typed text, so the one group is the suggestion popup.
    pub fn is_typing(&self) -> bool {
        self.groups
            .first()
            .is_some_and(|group| group.kind == GroupKind::Suggestions)
    }

    pub fn len(&self) -> usize {
        self.groups.iter().map(|group| group.rows.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The row at the highlight index, counted across every group.
    pub fn row(&self, index: usize) -> Option<&Row> {
        self.groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .nth(index)
    }
}

/// The rows for the field's text: suggestions with a final "See all" row while the reader has
/// typed something, otherwise the recent searches (or the starter searches when none) and then
/// the recently viewed entries. The Continue row leads Recently viewed and counts toward the cap.
pub fn list(
    text: &str,
    suggestions: &[Suggestion],
    recents: &[RecentSearch],
    starters: &[&str],
    continue_row: Option<ViewedRow>,
    viewed: &[ViewedRow],
    viewed_expanded: bool,
) -> List {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if !text.is_empty() {
        let mut rows: Vec<Row> = suggestions.iter().cloned().map(Row::Entry).collect();
        rows.push(Row::SeeAll(text));
        return List {
            groups: vec![Group {
                kind: GroupKind::Suggestions,
                rows,
            }],
            hidden_viewed: 0,
            viewed_total: 0,
        };
    }
    let mut groups = Vec::new();
    let searches: Vec<Row> = if recents.is_empty() {
        starters
            .iter()
            .map(|starter| Row::Starter((*starter).to_owned()))
            .collect()
    } else {
        recents.iter().cloned().map(Row::Search).collect()
    };
    if !searches.is_empty() {
        groups.push(Group {
            kind: if recents.is_empty() {
                GroupKind::Starters
            } else {
                GroupKind::Recent
            },
            rows: searches,
        });
    }
    let all_viewed: Vec<Row> = continue_row
        .into_iter()
        .map(Row::Continue)
        .chain(viewed.iter().cloned().map(Row::Viewed))
        .collect();
    let viewed_total = all_viewed.len();
    let cap = if viewed_expanded {
        viewed_total
    } else {
        MAX_RECENT_VIEWED
    };
    let rows: Vec<Row> = all_viewed.into_iter().take(cap).collect();
    let hidden_viewed = viewed_total - rows.len();
    if !rows.is_empty() {
        groups.push(Group {
            kind: GroupKind::Viewed,
            rows,
        });
    }
    List {
        groups,
        hidden_viewed,
        viewed_total,
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
    use super::list as build;
    use super::*;
    use crate::views::scope::Scope;
    use cast_iron_schema::EntryKind;

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

    fn viewed(n: usize) -> ViewedRow {
        ViewedRow {
            id: EntryId::parse(format!("ref/rule/viewed-{n}")).unwrap(),
            title: format!("Viewed {n}"),
            detail: "Rule · Core Rulebook".to_owned(),
            book: None,
        }
    }

    fn continue_row() -> ViewedRow {
        ViewedRow {
            id: EntryId::parse("ref/rule/continue").unwrap(),
            title: "Continue Core Rulebook".to_owned(),
            detail: "Grapple".to_owned(),
            book: Some("core-rulebook".to_owned()),
        }
    }

    #[test]
    fn typing_shows_suggestions_then_see_all() {
        let list = build(
            "  fireb ",
            &[suggestion("Fireball")],
            &[RecentSearch::new("old", None)],
            &["x"],
            Some(continue_row()),
            &[viewed(1)],
            false,
        );
        assert!(list.is_typing());
        assert_eq!(list.groups.len(), 1);
        assert_eq!(
            list.groups[0].rows,
            [
                Row::Entry(suggestion("Fireball")),
                Row::SeeAll("fireb".to_owned())
            ]
        );
    }

    #[test]
    fn typing_with_no_suggestion_still_offers_see_all() {
        let list = build("zzzz", &[], &[], &["x"], None, &[], false);
        assert!(list.is_typing());
        assert_eq!(list.groups[0].rows, [Row::SeeAll("zzzz".to_owned())]);
    }

    #[test]
    fn an_empty_field_shows_recents_then_recently_viewed() {
        let feats = Scope {
            kind: EntryKind::Feat,
            facets: Vec::new(),
            source: None,
            source_title: None,
        };
        let recents = vec![
            RecentSearch::new("grapple", Some(feats)),
            RecentSearch::new("fireball", None),
        ];
        let list = build("", &[], &recents, &["dragon"], None, &[viewed(1)], false);
        assert_eq!(
            list.groups
                .iter()
                .map(|group| group.kind)
                .collect::<Vec<_>>(),
            [GroupKind::Recent, GroupKind::Viewed]
        );
        assert_eq!(
            list.groups[0].rows,
            [
                Row::Search(recents[0].clone()),
                Row::Search(recents[1].clone())
            ]
        );
        assert_eq!(list.groups[1].rows, [Row::Viewed(viewed(1))]);
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn starters_show_before_recently_viewed_when_nothing_is_recent() {
        let list = build(
            "   ",
            &[],
            &[],
            &["dragon", "fireball"],
            None,
            &[viewed(1)],
            false,
        );
        assert_eq!(
            list.groups
                .iter()
                .map(|group| group.kind)
                .collect::<Vec<_>>(),
            [GroupKind::Starters, GroupKind::Viewed]
        );
        assert_eq!(
            list.groups[0].rows,
            [
                Row::Starter("dragon".to_owned()),
                Row::Starter("fireball".to_owned())
            ]
        );
    }

    #[test]
    fn the_continue_row_leads_recently_viewed_and_counts_toward_the_cap() {
        let viewed_rows: Vec<ViewedRow> = (0..6).map(viewed).collect();
        let list = build("", &[], &[], &[], Some(continue_row()), &viewed_rows, false);
        assert_eq!(list.groups[0].kind, GroupKind::Viewed);
        // Five rows: the Continue row first, then the four newest visits.
        assert_eq!(list.groups[0].rows.len(), MAX_RECENT_VIEWED);
        assert_eq!(list.groups[0].rows[0], Row::Continue(continue_row()));
        assert_eq!(
            &list.groups[0].rows[1..],
            &viewed_rows[0..4]
                .iter()
                .cloned()
                .map(Row::Viewed)
                .collect::<Vec<_>>()[..]
        );
        assert_eq!(list.viewed_total, 7);
        assert_eq!(list.hidden_viewed, 2);
        // Expanding shows every row and hides nothing.
        let list = build("", &[], &[], &[], Some(continue_row()), &viewed_rows, true);
        assert_eq!(list.groups[0].rows.len(), 7);
        assert_eq!(list.hidden_viewed, 0);
    }

    #[test]
    fn an_empty_field_with_nothing_stored_has_no_groups() {
        let list = build("", &[], &[], &[], None, &[], false);
        assert!(list.groups.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.viewed_total, 0);
    }

    #[test]
    fn the_highlight_counts_across_groups() {
        let recents = [RecentSearch::new("grapple", None)];
        let list = build("", &[], &recents, &[], None, &[viewed(1), viewed(2)], false);
        assert_eq!(list.len(), 3);
        assert_eq!(list.row(0), Some(&Row::Search(recents[0].clone())));
        assert_eq!(list.row(1), Some(&Row::Viewed(viewed(1))));
        assert_eq!(list.row(2), Some(&Row::Viewed(viewed(2))));
        assert_eq!(list.row(3), None);
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
