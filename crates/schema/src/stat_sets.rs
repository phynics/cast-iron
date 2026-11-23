//! Stat sets: the several statistics that one monster entry prints.
//!
//! A monster that prints more than one stat block (Air Elemental from Small to Elder, Ettercap,
//! Draugr) keeps each set under its own `##` heading, with the set's `###` sections and
//! `::: statblock` blocks below it. A **stat set** is a `##` heading whose section holds a stat
//! block row labelled `Challenge rating`. The section runs to the next heading of level 1 or 2.
//!
//! The compiler derives [`StatSet`] rows from this rule, the client lists them on the entry page
//! and the CR facet reads every set's rating. The corpus drift check reads the same rule to
//! limit `[fields]` to the first set. One function owns the rule, so the three cannot disagree.
//!
//! An entry with fewer than two sets has none: a single set is the entry itself.

use crate::headings::{keyed_headings, plain_text};
use crate::{Block, ChallengeRating, StatRow};
use serde::{Deserialize, Serialize};

/// The stat block row label that marks a stat set. The comparison ignores ASCII case.
pub const STAT_SET_MARKER_LABEL: &str = "Challenge rating";

/// One stat set of a creature entry.
///
/// Derived by the compiler and never written to the corpus. Additive: an entry without sets
/// omits the key, and an older reader ignores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatSet {
    /// The `##` heading text, such as `Small Air Elemental`.
    pub title: String,
    /// The URL anchor of that heading: its explicit `{#anchor}`, else the derived `sec-` key.
    pub anchor: String,
    /// The `Challenge rating` row, when it reads as a rating. `10/MR 4` reads as 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr: Option<ChallengeRating>,
    /// The `Size` row of the set, as printed and trimmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

/// A rating as a `Challenge rating` row prints it: `5`, `1/2`, or `10/MR 4` for a mythic one.
pub fn parse_stat_set_cr(value: &str) -> Option<ChallengeRating> {
    let value = value.trim();
    if let Some(parsed) = ChallengeRating::parse(value) {
        return Some(parsed);
    }
    let lead = value.split("/MR").next().unwrap_or(value);
    ChallengeRating::parse(lead.split_whitespace().next()?)
}

/// The top-level block index of each `##` heading that opens a stat set, with the index one
/// past the last block of its section.
fn spans(body: &[Block]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut start: Option<usize> = None;
    let mut marked = false;
    let mut close = |start: &mut Option<usize>, marked: &mut bool, end: usize| {
        if let Some(open) = start.take()
            && *marked
        {
            found.push((open, end));
        }
        *marked = false;
    };
    for (index, block) in body.iter().enumerate() {
        match block {
            Block::Heading { level, .. } if *level <= 2 => {
                close(&mut start, &mut marked, index);
                if *level == 2 {
                    start = Some(index);
                }
            }
            Block::StatBlock { rows, .. }
                if start.is_some()
                    && rows
                        .iter()
                        .any(|row| row.label.eq_ignore_ascii_case(STAT_SET_MARKER_LABEL)) =>
            {
                marked = true;
            }
            _ => {}
        }
    }
    close(&mut start, &mut marked, body.len());
    found
}

fn row_text(rows: &[&StatRow], label: &str) -> Option<String> {
    rows.iter()
        .find(|row| row.label.eq_ignore_ascii_case(label))
        .map(|row| plain_text(&row.value).trim().to_string())
        .filter(|text| !text.is_empty())
}

/// The stat sets of `body`, in document order. Empty unless the body holds at least two.
pub fn derive_stat_sets(body: &[Block]) -> Vec<StatSet> {
    let spans = spans(body);
    if spans.len() < 2 {
        return Vec::new();
    }
    let keys = keyed_headings(body);
    spans
        .into_iter()
        .filter_map(|(open, end)| {
            let heading = &body[open];
            let Block::Heading { content, .. } = heading else {
                return None;
            };
            let anchor = keys
                .iter()
                .find(|(block, _)| std::ptr::eq(*block, heading))
                .and_then(|(_, key)| key.clone())?;
            let rows: Vec<&StatRow> = body[open..end]
                .iter()
                .filter_map(|block| match block {
                    Block::StatBlock { rows, .. } => Some(rows),
                    _ => None,
                })
                .flatten()
                .collect();
            Some(StatSet {
                title: plain_text(content).trim().to_string(),
                anchor,
                cr: row_text(&rows, STAT_SET_MARKER_LABEL)
                    .and_then(|text| parse_stat_set_cr(&text)),
                size: row_text(&rows, "Size"),
            })
        })
        .collect()
}

/// The part of `body` that an entry's `[fields]` describe. With two or more stat sets it ends
/// before the second set's heading, so the first set and everything above it stay; with fewer
/// the whole body stays.
pub fn first_stat_set_region(body: &[Block]) -> &[Block] {
    match spans(body).get(1) {
        Some((second, _)) => &body[..*second],
        None => body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Inline;

    fn heading(level: u8, title: &str, anchor: Option<&str>) -> Block {
        Block::Heading {
            level,
            anchor: anchor.map(str::to_string),
            content: vec![Inline::text(title)],
        }
    }

    fn row(label: &str, value: &str) -> StatRow {
        StatRow {
            label: label.into(),
            value: vec![Inline::text(value)],
        }
    }

    fn block(rows: Vec<StatRow>) -> Block {
        Block::StatBlock { title: None, rows }
    }

    fn set(title: &str, anchor: &str, cr: &str, size: &str) -> Vec<Block> {
        vec![
            heading(2, title, Some(anchor)),
            heading(3, "Defense", None),
            block(vec![row("Armor class", "17")]),
            block(vec![row("Challenge rating", cr), row("Size", size)]),
            heading(3, "Offense", None),
            block(vec![row("Melee", "slam +6")]),
        ]
    }

    #[test]
    fn two_sets_list_in_order_with_anchor_cr_and_size() {
        let mut body = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![Inline::text("lead")],
        }];
        body.extend(set("Small Air Elemental", "stat-small", "1", "Small"));
        body.extend(set("Medium Air Elemental", "stat-medium", "3", "Medium"));
        let sets = derive_stat_sets(&body);
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].title, "Small Air Elemental");
        assert_eq!(sets[0].anchor, "stat-small");
        assert_eq!(sets[0].cr, ChallengeRating::parse("1"));
        assert_eq!(sets[0].size.as_deref(), Some("Small"));
        assert_eq!(sets[1].anchor, "stat-medium");
        assert_eq!(sets[1].cr, ChallengeRating::parse("3"));
        assert_eq!(derive_stat_sets(&body), sets, "derivation is deterministic");
    }

    #[test]
    fn seven_sets_keep_document_order() {
        let sizes = [
            "Small", "Medium", "Large", "Huge", "Greater", "Elder", "Titan",
        ];
        let mut body = Vec::new();
        for (index, size) in sizes.iter().enumerate() {
            let anchor = format!("stat-{}", size.to_lowercase());
            body.extend(set(
                &format!("{size} Elemental"),
                &anchor,
                &(index * 2 + 1).to_string(),
                size,
            ));
        }
        let sets = derive_stat_sets(&body);
        assert_eq!(sets.len(), 7);
        let anchors: Vec<&str> = sets.iter().map(|set| set.anchor.as_str()).collect();
        assert_eq!(
            anchors,
            [
                "stat-small",
                "stat-medium",
                "stat-large",
                "stat-huge",
                "stat-greater",
                "stat-elder",
                "stat-titan"
            ]
        );
        let ratings: Vec<u32> = sets
            .iter()
            .map(|set| set.cr.unwrap().twenty_fourths / 24)
            .collect();
        assert_eq!(ratings, [1, 3, 5, 7, 9, 11, 13]);
    }

    #[test]
    fn one_set_is_the_entry_and_gets_no_list() {
        let body = set("Ettercap", "stat-ettercap", "3", "Medium");
        assert!(derive_stat_sets(&body).is_empty());
        assert_eq!(first_stat_set_region(&body), body.as_slice());
    }

    #[test]
    fn a_heading_without_a_rating_row_is_not_a_set() {
        let mut body = set("Small", "stat-small", "1", "Small");
        body.push(heading(2, "Ecology", Some("ecology")));
        body.push(block(vec![row("Environment", "any")]));
        body.extend(set("Large", "stat-large", "5", "Large"));
        let sets = derive_stat_sets(&body);
        assert_eq!(
            sets.iter()
                .map(|set| set.anchor.as_str())
                .collect::<Vec<_>>(),
            ["stat-small", "stat-large"]
        );
    }

    #[test]
    fn a_rating_row_above_any_heading_or_under_h1_marks_nothing() {
        let body = vec![
            block(vec![row("Challenge rating", "2")]),
            heading(2, "Lore", None),
            heading(2, "More lore", None),
        ];
        assert!(derive_stat_sets(&body).is_empty());
    }

    #[test]
    fn a_set_section_ends_at_the_next_second_level_heading() {
        // The rating row below a later `##` belongs to that heading, not to the first.
        let body = vec![
            heading(2, "A", Some("a")),
            block(vec![row("Armor class", "1")]),
            heading(2, "B", Some("b")),
            block(vec![row("Challenge rating", "2")]),
            heading(2, "C", Some("c")),
            block(vec![row("Challenge rating", "4")]),
        ];
        let sets = derive_stat_sets(&body);
        assert_eq!(
            sets.iter()
                .map(|set| set.anchor.as_str())
                .collect::<Vec<_>>(),
            ["b", "c"]
        );
    }

    #[test]
    fn a_heading_without_an_anchor_takes_its_derived_key() {
        let mut body = set("Small Air Elemental", "x", "1", "Small");
        body[0] = heading(2, "Small Air Elemental", None);
        body.extend(set("Large Air Elemental", "stat-large", "5", "Large"));
        assert_eq!(derive_stat_sets(&body)[0].anchor, "sec-small-air-elemental");
    }

    #[test]
    fn ratings_read_fractions_and_mythic_forms() {
        assert_eq!(parse_stat_set_cr("1/2"), ChallengeRating::parse("1/2"));
        assert_eq!(parse_stat_set_cr(" 10/MR 4 "), ChallengeRating::parse("10"));
        assert_eq!(
            parse_stat_set_cr("7 (XP 3,200)"),
            ChallengeRating::parse("7")
        );
        assert_eq!(parse_stat_set_cr("—"), None);
    }

    #[test]
    fn the_first_region_stops_before_the_second_set() {
        let mut body = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![Inline::text("lead")],
        }];
        body.extend(set("Small", "stat-small", "1", "Small"));
        let first = body.len();
        body.extend(set("Large", "stat-large", "5", "Large"));
        assert_eq!(first_stat_set_region(&body).len(), first);
    }
}
