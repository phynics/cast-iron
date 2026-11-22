//! The archetype list of a class page and its compatibility filter (issue #170, part D; decision
//! `2026-11-22-archetype-list-and-compatibility`).
//!
//! A class page lists its archetypes as rows (name, book, what it replaces) instead of printing
//! each one's blurb and a matrix of X marks. The same typed data answers two questions:
//!
//! - *Which archetypes keep this feature?* An archetype **touches** a feature when one of its
//!   resolved trades, a replacement or an alteration, names the feature's heading anchor.
//! - *Which archetypes can I combine with this one?* Two archetypes **conflict** when a resolved
//!   trade of one overlaps a resolved trade of the other: the same feature anchor, a shared
//!   level (a trade without `levels` covers every level) and a shared named power (a trade
//!   without a `power` covers every power).
//!
//! An unresolved trade, or an archetype that has no trades row, proves nothing: such an archetype
//! is never counted as compatible, and is listed apart as not checked. Everything here is pure,
//! over typed content, and has native tests.

use crate::class_options::replaces_line_of;
use cast_iron_schema::headings::plain_text;
use cast_iron_schema::power::split_ability_tag;
use cast_iron_schema::{Block, EntryId, FeatureRef, Inline, Trade, TradeOp, TradeRow};

/// One archetype of the class on the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: EntryId,
    pub title: String,
    /// The title of the book that prints it.
    pub book: String,
    /// `Replaces Arcane bond, Arcane school`: the printed names, or `None` when it replaces nothing.
    pub replaces: Option<String>,
    /// `Alters Bonus feats`, the printed names of the alterations.
    pub alters: Option<String>,
    /// The resolved trades, in print order.
    trades: Vec<FeatureTrade>,
    /// Whether every trade is resolved. False for a row without trades data or with a trade that
    /// names no feature.
    checked: bool,
    /// Whether the library has a trades row for the archetype at all.
    has_data: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FeatureTrade {
    feature: FeatureRef,
    printed: String,
}

impl Row {
    /// A row from the typed trades of the archetype. `trades` is `None` for a library without a
    /// row for it.
    pub fn new(id: EntryId, title: String, book: String, trades: Option<&TradeRow>) -> Self {
        let line = |op: TradeOp| {
            trades.and_then(|row| {
                let names = row.printed(op);
                (!names.is_empty()).then(|| format!("{} {}", op.label(), names.join(", ")))
            })
        };
        let resolved: Vec<FeatureTrade> = trades
            .map(|row| row.trades.iter().filter_map(feature_trade).collect())
            .unwrap_or_default();
        let checked =
            trades.is_some_and(|row| row.trades.iter().all(|trade| trade.feature.is_some()));
        Self {
            id,
            title,
            book,
            replaces: trades.and_then(replaces_line_of),
            alters: line(TradeOp::Alters),
            trades: resolved,
            checked,
            has_data: trades.is_some(),
        }
    }

    /// Whether the library has trades for this archetype. False says the data does not tell which
    /// class features it changes; true with [`Self::is_checked`] false says some trade names none.
    pub fn has_data(&self) -> bool {
        self.has_data
    }

    /// Whether the data can prove what this archetype touches.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Whether a resolved trade, a replacement or an alteration, names `anchor`.
    pub fn touches(&self, anchor: &str) -> bool {
        self.trades
            .iter()
            .any(|trade| trade.feature.anchor == anchor)
    }
}

fn feature_trade(trade: &Trade) -> Option<FeatureTrade> {
    Some(FeatureTrade {
        feature: trade.feature.clone()?,
        printed: trade.printed.clone(),
    })
}

/// Whether two references reach the same part of a class feature: the same anchor, a shared
/// level (no levels means every level) and a shared named power (no power means every power).
/// This is the compatibility matcher of the corpus format's "Two different comparisons", not the
/// exact duplicate check of `corpus-check`.
pub fn overlaps(a: &FeatureRef, b: &FeatureRef) -> bool {
    let same_class = match (&a.class, &b.class) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    };
    let shared_level =
        a.levels.is_empty() || b.levels.is_empty() || a.levels.iter().any(|l| b.levels.contains(l));
    let shared_power = match (&a.power, &b.power) {
        (Some(x), Some(y)) => x.eq_ignore_ascii_case(y),
        _ => true,
    };
    a.anchor == b.anchor && same_class && shared_level && shared_power
}

/// Whether two archetypes change the same part of a class feature.
pub fn conflicts(a: &Row, b: &Row) -> bool {
    a.trades
        .iter()
        .any(|x| b.trades.iter().any(|y| overlaps(&x.feature, &y.feature)))
}

/// A feature of the class that at least one archetype touches, for the `Keep` menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureOption {
    pub anchor: String,
    pub label: String,
    /// How many archetypes touch it.
    pub count: usize,
}

/// The headings of a class body that carry an explicit anchor, as `(anchor, title)` in page order,
/// without the ability tag (`Arcane Bond (Ex or Sp)` is `Arcane Bond`).
pub fn feature_headings(body: &[Block]) -> Vec<(String, String)> {
    body.iter()
        .filter_map(|block| match block {
            Block::Heading {
                anchor: Some(anchor),
                content,
                ..
            } => {
                let text = plain_text(content);
                Some((
                    anchor.clone(),
                    split_ability_tag(&text).0.trim().to_string(),
                ))
            }
            _ => None,
        })
        .collect()
}

/// A feature's name from its heading anchor, for a feature that has no heading on the page:
/// `bonus-feats` is `Bonus feats`. The printed name of a trade is not used, because it can name
/// one level of the feature (`5th-level bonus feat`) and the option is the whole feature.
pub fn anchor_label(anchor: &str) -> String {
    let words = anchor.replace('-', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The features the archetypes touch, in the order of the class's own headings. A feature that
/// has no heading in `headings` is named from its anchor (see [`anchor_label`]) and comes last.
/// Each shows how many archetypes touch it.
pub fn feature_options(rows: &[Row], headings: &[(String, String)]) -> Vec<FeatureOption> {
    let mut found: Vec<FeatureOption> = Vec::new();
    for (anchor, label) in headings {
        let count = rows.iter().filter(|row| row.touches(anchor)).count();
        if count > 0 {
            found.push(FeatureOption {
                anchor: anchor.clone(),
                label: label.clone(),
                count,
            });
        }
    }
    for row in rows {
        for trade in &row.trades {
            let anchor = &trade.feature.anchor;
            if found.iter().all(|option| &option.anchor != anchor) {
                let count = rows.iter().filter(|other| other.touches(anchor)).count();
                found.push(FeatureOption {
                    anchor: anchor.clone(),
                    label: anchor_label(anchor),
                    count,
                });
            }
        }
    }
    found
}

/// What the reader asked for: features to keep, and an archetype to combine with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choice {
    pub keep: Vec<String>,
    pub with: Option<EntryId>,
}

impl Choice {
    pub fn is_empty(&self) -> bool {
        self.keep.is_empty() && self.with.is_none()
    }
}

/// The rows a choice leaves, in their given order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shown {
    /// Rows the data proves compatible with the choice.
    pub compatible: Vec<usize>,
    /// Rows the data cannot prove either way, listed apart. Empty when nothing is chosen.
    pub unchecked: Vec<usize>,
}

/// How many archetypes the `Combine with` choice is made among: every row but the chosen one.
pub fn others(rows: &[Row], choice: &Choice) -> usize {
    let chosen = choice
        .with
        .as_ref()
        .is_some_and(|id| rows.iter().any(|row| &row.id == id));
    rows.len() - usize::from(chosen)
}

/// How many other archetypes the data proves compatible with `row`.
pub fn compatible_with(rows: &[Row], row: &Row) -> usize {
    narrow(
        rows,
        &Choice {
            keep: Vec::new(),
            with: Some(row.id.clone()),
        },
    )
    .compatible
    .len()
}

/// Narrow `rows` by `choice`. With nothing chosen every row is compatible and none is unchecked.
/// Otherwise a row is compatible when it touches none of the kept features and conflicts with
/// none of the chosen archetype's trades, and it is *unchecked* when the data cannot say (a trade
/// that names no feature, or no trades data). The chosen archetype itself is left out.
pub fn narrow(rows: &[Row], choice: &Choice) -> Shown {
    if choice.is_empty() {
        return Shown {
            compatible: (0..rows.len()).collect(),
            unchecked: Vec::new(),
        };
    }
    let with = choice
        .with
        .as_ref()
        .and_then(|id| rows.iter().find(|row| &row.id == id));
    // Nothing is compatible with an archetype the data cannot place (an unresolved trade, no
    // trades row, or an id that is not among the rows): it could clash with any of them. Every
    // other row is not checked.
    if choice.with.is_some() && with.is_none_or(|with| !with.checked) {
        return Shown {
            compatible: Vec::new(),
            unchecked: (0..rows.len())
                .filter(|&index| with.is_none_or(|with| with.id != rows[index].id))
                .collect(),
        };
    }
    let mut shown = Shown::default();
    for (index, row) in rows.iter().enumerate() {
        if with.is_some_and(|with| with.id == row.id) {
            continue;
        }
        if !row.checked {
            shown.unchecked.push(index);
            continue;
        }
        let keeps = choice.keep.iter().all(|anchor| !row.touches(anchor));
        let combines = with.is_none_or(|with| !conflicts(with, row));
        if keeps && combines {
            shown.compatible.push(index);
        }
    }
    shown
}

// ---- the page body ----------------------------------------------------------------------

/// The title of the section that lists the archetypes, as the corpus writes it.
const SECTION: &str = "archetypes";

/// The title of the matrix section that follows it, which the list replaces.
const MATRIX: &str = "archetypes by feature";

fn heading_of(block: &Block) -> Option<(u8, String)> {
    match block {
        Block::Heading { level, content, .. } => {
            Some((*level, plain_text(content).trim().to_lowercase()))
        }
        _ => None,
    }
}

/// Whether `block` is a bold name on a line of its own: `**Arcane Bomber**`.
fn is_name(block: &Block) -> bool {
    matches!(
        block,
        Block::Paragraph { content, .. }
            if matches!(content.as_slice(), [Inline::Strong { .. }])
    )
}

/// Whether `block` is the `**Class:** wizard` row that follows an archetype's name.
fn is_class_row(block: &Block) -> bool {
    matches!(
        block,
        Block::Paragraph { content, .. }
            if matches!(content.first(), Some(Inline::Strong { c }) if plain_text(c).trim() == "Class:")
    )
}

/// The class body without the printed archetype blurbs and without the `Archetypes by Feature`
/// matrix: the list takes their place. The introduction of the section stays, and so does every
/// other block. `None` when the body has no `Archetypes` section at its top level, so a page
/// that this cannot read keeps its text.
///
/// A blurb starts at a bold name followed by a `Class:` row and runs to the end of the section.
/// The matrix is the section titled `Archetypes by Feature`, a heading and the blocks up to the
/// next heading of the same level or above.
pub fn fold_body(body: &[Block]) -> Option<Vec<Block>> {
    let top = body
        .iter()
        .filter_map(heading_of)
        .map(|(level, _)| level)
        .min()?;
    let section = body
        .iter()
        .position(|block| heading_of(block) == Some((top, SECTION.to_string())))?;
    let end_of = |from: usize| {
        body.iter()
            .enumerate()
            .skip(from + 1)
            .find(|(_, block)| heading_of(block).is_some_and(|(level, _)| level <= top))
            .map_or(body.len(), |(index, _)| index)
    };
    let end = end_of(section);
    let blurbs = (section + 1..end)
        .find(|&index| index + 1 < end && is_name(&body[index]) && is_class_row(&body[index + 1]))
        .unwrap_or(end);
    let matrix = (end..body.len())
        .find(|&index| heading_of(&body[index]) == Some((top, MATRIX.to_string())))
        .map(|index| index..end_of(index));
    let mut out: Vec<Block> = body[..blurbs].to_vec();
    match matrix {
        Some(range) => {
            out.extend_from_slice(&body[end..range.start]);
            out.extend_from_slice(&body[range.end..]);
        }
        None => out.extend_from_slice(&body[end..]),
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::ParagraphStyle;

    fn id(value: &str) -> EntryId {
        EntryId::parse(value).unwrap()
    }

    fn feature(anchor: &str) -> FeatureRef {
        FeatureRef {
            anchor: anchor.into(),
            ..FeatureRef::default()
        }
    }

    fn trade(op: TradeOp, printed: &str, feature: Option<FeatureRef>) -> Trade {
        Trade {
            op,
            printed: printed.into(),
            feature,
        }
    }

    fn row(name: &str, trades: Vec<Trade>) -> Row {
        Row::new(
            id(&format!(
                "ref/arc-wizard/{}",
                name.to_lowercase().replace(' ', "-")
            )),
            name.into(),
            "Advanced Player's Guide".into(),
            Some(&TradeRow {
                id: id("ref/arc-wizard/x"),
                trades,
            }),
        )
    }

    fn replaces(printed: &str, anchor: &str) -> Trade {
        trade(TradeOp::Replaces, printed, Some(feature(anchor)))
    }

    fn at(anchor: &str, levels: &[u8], power: Option<&str>) -> FeatureRef {
        FeatureRef {
            anchor: anchor.into(),
            class: None,
            levels: levels.to_vec(),
            power: power.map(String::from),
        }
    }

    fn rows() -> Vec<Row> {
        vec![
            row(
                "Arcane Bomber",
                vec![
                    replaces("Arcane bond", "arcane-bond"),
                    replaces("Cantrips", "cantrips"),
                ],
            ),
            row(
                "Arcane Physician",
                vec![replaces("Scribe Scroll", "scribe-scroll")],
            ),
            row(
                "Bonded Wizard",
                vec![
                    replaces("Scribe Scroll", "scribe-scroll"),
                    trade(
                        TradeOp::Replaces,
                        "5th-level bonus feat",
                        Some(at("bonus-feats", &[5], None)),
                    ),
                ],
            ),
            row(
                "Chronomancer",
                vec![trade(
                    TradeOp::Replaces,
                    "10th-level bonus feat",
                    Some(at("bonus-feats", &[10], None)),
                )],
            ),
            row(
                "Odd One",
                vec![
                    replaces("Arcane bond", "arcane-bond"),
                    trade(TradeOp::Replaces, "Something unplaced", None),
                ],
            ),
        ]
    }

    fn alters(printed: &str, anchor: &str) -> Trade {
        trade(TradeOp::Alters, printed, Some(feature(anchor)))
    }

    fn titles(rows: &[Row], indexes: &[usize]) -> Vec<String> {
        indexes.iter().map(|&i| rows[i].title.clone()).collect()
    }

    #[test]
    fn two_archetypes_conflict_when_they_change_the_same_feature() {
        let rows = rows();
        assert!(conflicts(&rows[1], &rows[2]), "both replace Scribe Scroll");
        assert!(conflicts(&rows[0], &rows[4]), "both replace Arcane bond");
        assert!(!conflicts(&rows[0], &rows[1]));
        assert!(!conflicts(&rows[1], &rows[3]));
    }

    #[test]
    fn different_levels_of_one_feature_do_not_conflict_and_the_whole_feature_does() {
        let fifth = at("bonus-feats", &[5], None);
        let tenth = at("bonus-feats", &[10], None);
        let whole = at("bonus-feats", &[], None);
        let both = at("bonus-feats", &[5, 10], None);
        assert!(!overlaps(&fifth, &tenth));
        assert!(overlaps(&fifth, &whole), "the whole feature covers a level");
        assert!(overlaps(&whole, &tenth));
        assert!(overlaps(&both, &tenth));
        assert!(!overlaps(&fifth, &at("cantrips", &[5], None)));
    }

    #[test]
    fn a_named_power_matches_the_same_power_or_the_whole_feature() {
        let warp = at("bloodline-powers", &[], Some("Warp Touch"));
        let other = at("bloodline-powers", &[], Some("Claws"));
        let whole = at("bloodline-powers", &[], None);
        assert!(overlaps(&warp, &warp));
        assert!(!overlaps(&warp, &other));
        assert!(overlaps(&warp, &whole));
        assert!(overlaps(&whole, &other));
    }

    #[test]
    fn a_feature_of_another_class_is_never_the_same_feature() {
        let mut a = feature("spells");
        let mut b = feature("spells");
        a.class = Some(id("ref/class/wizard"));
        b.class = Some(id("ref/class/sorcerer"));
        assert!(!overlaps(&a, &b));
        b.class = Some(id("ref/class/wizard"));
        assert!(overlaps(&a, &b));
        // A trade that names no class takes the class of the page.
        assert!(overlaps(&a, &feature("spells")));
    }

    #[test]
    fn an_alteration_clashes_with_a_replacement_and_with_another_alteration() {
        let alters_only = row("Alterer", vec![alters("Arcane bond", "arcane-bond")]);
        let replacer = row("Replacer", vec![replaces("Arcane bond", "arcane-bond")]);
        let also_alters = row("Also", vec![alters("Arcane bond", "arcane-bond")]);
        let other = row("Other", vec![replaces("Cantrips", "cantrips")]);
        assert!(conflicts(&alters_only, &replacer));
        assert!(conflicts(&alters_only, &also_alters));
        assert!(!conflicts(&alters_only, &other));
        // Keeping the feature leaves out an archetype that only alters it.
        let all = vec![alters_only, replacer, other];
        let shown = narrow(
            &all,
            &Choice {
                keep: vec!["arcane-bond".into()],
                with: None,
            },
        );
        assert_eq!(titles(&all, &shown.compatible), ["Other"]);
    }

    #[test]
    fn a_trade_with_levels_meets_a_trade_with_a_power_only_where_both_can_apply() {
        let level_five = at("bloodline-powers", &[5], None);
        let power_claws = at("bloodline-powers", &[], Some("Claws"));
        // One names levels, the other a power: each covers the other's whole range.
        assert!(overlaps(&level_five, &power_claws));
        // Both name a level and a power: both must be shared.
        let five_claws = at("bloodline-powers", &[5], Some("Claws"));
        let nine_claws = at("bloodline-powers", &[9], Some("Claws"));
        let five_warp = at("bloodline-powers", &[5], Some("Warp Touch"));
        assert!(!overlaps(&five_claws, &nine_claws));
        assert!(!overlaps(&five_claws, &five_warp));
        assert!(overlaps(&five_claws, &five_claws));
        // The power compares without case.
        assert!(overlaps(
            &at("bloodline-powers", &[], Some("claws")),
            &power_claws
        ));
    }

    #[test]
    fn a_chosen_archetype_the_data_cannot_place_leaves_nothing_compatible() {
        let all = rows();
        let with = |value: &str| Choice {
            keep: Vec::new(),
            with: Some(id(value)),
        };
        // An unresolved trade: every other row is not checked, none is compatible.
        let odd = all.iter().find(|r| r.title == "Odd One").unwrap();
        let shown = narrow(&all, &with(odd.id.as_str()));
        assert!(shown.compatible.is_empty());
        assert_eq!(
            titles(&all, &shown.unchecked),
            [
                "Arcane Bomber",
                "Arcane Physician",
                "Bonded Wizard",
                "Chronomancer"
            ]
        );
        // No trades row.
        let mut with_nodata = rows();
        with_nodata.push(Row::new(
            id("ref/arc-wizard/nodata"),
            "No Data".into(),
            "Book".into(),
            None,
        ));
        let shown = narrow(&with_nodata, &with("ref/arc-wizard/nodata"));
        assert!(shown.compatible.is_empty());
        assert_eq!(shown.unchecked, [0, 1, 2, 3, 4]);
        // An id that is not among the rows: nothing is compatible, every row is not checked.
        let shown = narrow(&all, &with("ref/arc-wizard/gone"));
        assert!(shown.compatible.is_empty());
        assert_eq!(shown.unchecked, [0, 1, 2, 3, 4]);
        // The count's total leaves the chosen archetype out only when it is among the rows.
        assert_eq!(others(&all, &with(odd.id.as_str())), 4);
        assert_eq!(others(&all, &with("ref/arc-wizard/gone")), 5);
        assert_eq!(others(&all, &Choice::default()), 5);
    }

    #[test]
    fn a_row_says_whether_the_library_has_trades_for_it() {
        let nodata = Row::new(id("ref/arc-wizard/n"), "N".into(), "B".into(), None);
        assert!(!nodata.has_data() && !nodata.is_checked());
        let odd = rows().remove(4);
        assert!(odd.has_data() && !odd.is_checked());
    }

    #[test]
    fn a_feature_without_a_heading_is_named_from_its_anchor() {
        assert_eq!(anchor_label("bonus-feats"), "Bonus feats");
        assert_eq!(anchor_label("cantrips"), "Cantrips");
        assert_eq!(anchor_label(""), "");
        // A whole-feature option is not named after one level of it.
        let rows = vec![row(
            "Level only",
            vec![trade(
                TradeOp::Replaces,
                "5th-level bonus feat",
                Some(at("bonus-feats", &[5], None)),
            )],
        )];
        let options = feature_options(&rows, &[]);
        assert_eq!(options[0].label, "Bonus feats");
    }

    #[test]
    fn compatible_with_counts_the_other_archetypes_that_can_combine() {
        let all = rows();
        assert_eq!(compatible_with(&all, &all[1]), 2);
        // The archetype with an unresolved trade combines with nothing the data proves.
        assert_eq!(compatible_with(&all, &all[4]), 0);
    }

    #[test]
    fn keeping_features_leaves_the_archetypes_that_do_not_touch_them() {
        let rows = rows();
        let none = narrow(&rows, &Choice::default());
        assert_eq!(none.compatible, [0, 1, 2, 3, 4]);
        assert!(none.unchecked.is_empty());

        let keep = |anchors: &[&str]| Choice {
            keep: anchors.iter().map(|a| a.to_string()).collect(),
            with: None,
        };
        let shown = narrow(&rows, &keep(&["arcane-bond"]));
        assert_eq!(
            titles(&rows, &shown.compatible),
            ["Arcane Physician", "Bonded Wizard", "Chronomancer"]
        );
        // The row with an unplaced trade could touch the feature: it is not checked, not shown.
        assert_eq!(titles(&rows, &shown.unchecked), ["Odd One"]);

        let shown = narrow(&rows, &keep(&["arcane-bond", "bonus-feats"]));
        assert_eq!(titles(&rows, &shown.compatible), ["Arcane Physician"]);
        // Keeping the whole feature excludes a replacement of any one level.
        let shown = narrow(&rows, &keep(&["bonus-feats"]));
        assert_eq!(
            titles(&rows, &shown.compatible),
            ["Arcane Bomber", "Arcane Physician"]
        );
    }

    #[test]
    fn choosing_an_archetype_leaves_the_ones_it_can_combine_with() {
        let rows = rows();
        let with = |name: &str| Choice {
            keep: Vec::new(),
            with: Some(
                rows.iter()
                    .find(|row| row.title == name)
                    .unwrap()
                    .id
                    .clone(),
            ),
        };
        let shown = narrow(&rows, &with("Arcane Physician"));
        // Bonded Wizard replaces Scribe Scroll too; the chosen archetype is not listed.
        assert_eq!(
            titles(&rows, &shown.compatible),
            ["Arcane Bomber", "Chronomancer"]
        );
        assert_eq!(titles(&rows, &shown.unchecked), ["Odd One"]);
        // Different levels of bonus feats combine.
        let shown = narrow(&rows, &with("Bonded Wizard"));
        assert_eq!(
            titles(&rows, &shown.compatible),
            ["Arcane Bomber", "Chronomancer"]
        );
        // Both choices together.
        let both = Choice {
            keep: vec!["cantrips".into()],
            with: with("Arcane Physician").with,
        };
        let shown = narrow(&rows, &both);
        assert_eq!(titles(&rows, &shown.compatible), ["Chronomancer"]);
    }

    #[test]
    fn a_row_with_no_trades_data_is_never_proven_compatible() {
        let nodata = Row::new(
            id("ref/arc-wizard/nodata"),
            "No Data".into(),
            "Book".into(),
            None,
        );
        assert!(!nodata.is_checked());
        assert!(nodata.replaces.is_none());
        let mut all = rows();
        all.push(nodata);
        let shown = narrow(
            &all,
            &Choice {
                keep: vec!["cantrips".into()],
                with: None,
            },
        );
        assert!(shown.unchecked.contains(&5));
        assert!(!shown.compatible.contains(&5));
        // A row that replaces and alters nothing, with a trades row, is checked and compatible.
        let empty = Row::new(
            id("ref/arc-wizard/empty"),
            "Empty".into(),
            "Book".into(),
            Some(&TradeRow {
                id: id("ref/arc-wizard/empty"),
                trades: Vec::new(),
            }),
        );
        assert!(empty.is_checked());
    }

    #[test]
    fn a_row_names_what_it_replaces_and_alters() {
        let r = row(
            "Mixed",
            vec![
                replaces("Arcane bond", "arcane-bond"),
                trade(TradeOp::Alters, "Bonus feats", Some(feature("bonus-feats"))),
                replaces("Cantrips", "cantrips"),
            ],
        );
        assert_eq!(
            r.replaces.as_deref(),
            Some("Replaces Arcane bond, Cantrips")
        );
        assert_eq!(r.alters.as_deref(), Some("Alters Bonus feats"));
        // An alteration touches the feature as a replacement does.
        assert!(r.touches("bonus-feats"));
        let plain = row("Plain", vec![replaces("Cantrips", "cantrips")]);
        assert_eq!(plain.alters, None);
    }

    #[test]
    fn the_keep_menu_lists_the_touched_features_in_the_order_of_the_headings() {
        let rows = rows();
        let headings = vec![
            (
                "weapon-and-armor-proficiencies".to_string(),
                "Weapon and Armor Proficiencies".to_string(),
            ),
            ("arcane-bond".to_string(), "Arcane Bond".to_string()),
            ("bonus-feats".to_string(), "Bonus Feats".to_string()),
            ("scribe-scroll".to_string(), "Scribe Scroll".to_string()),
        ];
        let options = feature_options(&rows, &headings);
        let seen: Vec<(&str, &str, usize)> = options
            .iter()
            .map(|o| (o.anchor.as_str(), o.label.as_str(), o.count))
            .collect();
        // `cantrips` has no heading here: it is named from its anchor and comes last.
        assert_eq!(
            seen,
            [
                ("arcane-bond", "Arcane Bond", 2),
                ("bonus-feats", "Bonus Feats", 2),
                ("scribe-scroll", "Scribe Scroll", 2),
                ("cantrips", "Cantrips", 1),
            ]
        );
    }

    #[test]
    fn feature_headings_drop_the_ability_tag_and_need_an_anchor() {
        let body = vec![
            Block::Heading {
                level: 4,
                anchor: Some("arcane-bond".into()),
                content: vec![Inline::text("Arcane Bond (Ex or Sp)")],
            },
            Block::Heading {
                level: 4,
                anchor: None,
                content: vec![Inline::text("No anchor")],
            },
            Block::Heading {
                level: 4,
                anchor: Some("bonus-feats".into()),
                content: vec![Inline::text("Bonus Feats")],
            },
        ];
        assert_eq!(
            feature_headings(&body),
            [
                ("arcane-bond".to_string(), "Arcane Bond".to_string()),
                ("bonus-feats".to_string(), "Bonus Feats".to_string()),
            ]
        );
    }

    fn h(level: u8, title: &str) -> Block {
        Block::Heading {
            level,
            anchor: None,
            content: vec![Inline::text(title)],
        }
    }

    fn p(content: Vec<Inline>) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }
    }

    fn bold(value: &str) -> Inline {
        Inline::Strong {
            c: vec![Inline::text(value)],
        }
    }

    fn class_body() -> Vec<Block> {
        vec![
            p(vec![Inline::text("Role: a wizard.")]),
            h(3, "Class Features"),
            p(vec![Inline::text("Features.")]),
            h(3, "Archetypes"),
            p(vec![
                bold("Source"),
                Inline::text(" Advanced Player's Guide"),
            ]),
            p(vec![Inline::text("Introduction to archetypes.")]),
            p(vec![bold("Arcane Bomber")]),
            p(vec![bold("Class:"), Inline::text(" wizard")]),
            p(vec![bold("Description:"), Inline::text(" Bombs.")]),
            p(vec![bold("Arcane Physician")]),
            p(vec![bold("Class:"), Inline::text(" wizard")]),
            h(3, "Archetypes by Feature"),
            Block::Table(cast_iron_schema::Table::default()),
            h(3, "Favored Class Bonuses"),
            p(vec![Inline::text("Human: +1.")]),
        ]
    }

    fn texts(body: &[Block]) -> Vec<String> {
        body.iter()
            .map(|block| match block {
                Block::Heading { content, .. } => format!("# {}", plain_text(content)),
                Block::Paragraph { content, .. } => plain_text(content),
                Block::Table(_) => "[table]".to_string(),
                _ => "[other]".to_string(),
            })
            .collect()
    }

    #[test]
    fn folding_keeps_the_introduction_and_drops_the_blurbs_and_the_matrix() {
        let folded = fold_body(&class_body()).unwrap();
        assert_eq!(
            texts(&folded),
            [
                "Role: a wizard.",
                "# Class Features",
                "Features.",
                "# Archetypes",
                "Source Advanced Player's Guide",
                "Introduction to archetypes.",
                "# Favored Class Bonuses",
                "Human: +1.",
            ]
        );
    }

    #[test]
    fn a_body_without_an_archetypes_section_is_left_alone() {
        let body = vec![h(3, "Class Features"), p(vec![Inline::text("Text.")])];
        assert_eq!(fold_body(&body), None);
        // Only a top-level heading counts: an archetype's own deeper `Archetypes` is not it.
        let deeper = vec![
            h(3, "Class Features"),
            h(4, "Archetypes"),
            p(vec![bold("X")]),
        ];
        assert_eq!(fold_body(&deeper), None);
    }

    #[test]
    fn a_section_without_blurbs_or_matrix_folds_to_itself() {
        let body = vec![
            h(3, "Archetypes"),
            p(vec![Inline::text("Only an introduction.")]),
            h(3, "Alternate Capstones"),
        ];
        assert_eq!(fold_body(&body), Some(body.clone()));
    }
}
