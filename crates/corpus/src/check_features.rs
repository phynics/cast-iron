//! The corpus-check rules for feature facts (corpus format 4, #540).
//!
//! An archetype, class option or base class may list `features = [{ anchor, level }]` and an
//! archetype or class option may name, in `trades[].by`, the feature whose printed text states
//! the trade. The rules:
//!
//! 1. every `features` anchor is a heading key of the body, and no anchor is listed twice;
//! 2. `level` agrees with an "At Nth level" or "Starting at Nth level" lead, when the first
//!    paragraph under the feature's heading has one;
//! 3. every `by` names an anchor of `features`.
//!
//! The drift check (`drift.rs`) compares each trade with the trade sentence of its feature.

use crate::check::{Report, line_of};
use crate::load::Corpus;
use crate::trade_sentences::level_lead;
use cast_iron_schema::headings::{keyed_headings, plain_text};
use cast_iron_schema::{Block, RuleEntry};

/// One feature section of a body: the heading with its key, and the blocks under it up to the
/// next heading of the same or a higher level.
pub(crate) struct Section<'a> {
    pub key: String,
    pub heading: &'a Block,
    pub blocks: &'a [Block],
}

/// The sections of the top-level headings of `blocks`. `body` is the whole body of the entry,
/// which assigns the keys, so the keys agree with the ones the page uses; `blocks` is `body` or a
/// prefix of it. A heading nested in a list or quote has no section.
pub(crate) fn sections<'a>(blocks: &'a [Block], body: &'a [Block]) -> Vec<Section<'a>> {
    let keyed = keyed_headings(body);
    let key_of = |block: &Block| {
        keyed
            .iter()
            .find(|(candidate, _)| std::ptr::eq(*candidate, block))
            .and_then(|(_, key)| key.clone())
    };
    let mut out = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let Block::Heading { level, .. } = block else {
            continue;
        };
        let Some(key) = key_of(block) else {
            continue;
        };
        let end = blocks[index + 1..]
            .iter()
            .position(|next| matches!(next, Block::Heading { level: next_level, .. } if next_level <= level))
            .map_or(blocks.len(), |offset| index + 1 + offset);
        out.push(Section {
            key,
            heading: block,
            blocks: &blocks[index + 1..end],
        });
    }
    out
}

/// The plain text of every paragraph in `blocks`, in order, including those in lists and quotes.
pub(crate) fn paragraph_texts(blocks: &[Block]) -> Vec<String> {
    let mut out = Vec::new();
    for block in blocks {
        match block {
            Block::Paragraph { content, .. } => out.push(plain_text(content)),
            Block::Quote { content } => out.extend(paragraph_texts(content)),
            Block::List { items, .. } => {
                for item in items {
                    out.extend(paragraph_texts(item));
                }
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn check_features(corpus: &Corpus, report: &mut Report) {
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let features = entry.fields.features();
        let trades = entry.fields.trades();
        if features.is_empty() && trades.iter().all(|trade| trade.by.is_none()) {
            continue;
        }
        let path = stored.path.display();
        let at = |needle: &str| format!("{path}:{}", line_of(&stored.text, needle));
        check_entry(entry, &at, report);
    }
}

fn check_entry(entry: &RuleEntry, at: &dyn Fn(&str) -> String, report: &mut Report) {
    let features = entry.fields.features();
    let body_sections = sections(&entry.body, &entry.body);
    for (index, feature) in features.iter().enumerate() {
        let place = at(&format!("anchor = \"{}\"", feature.anchor));
        if features[..index]
            .iter()
            .any(|other| other.anchor == feature.anchor)
        {
            report.problems.push(format!(
                "{place}: [fields] the feature `{}` is listed twice",
                feature.anchor
            ));
            continue;
        }
        if !(1..=20).contains(&feature.level) {
            report.problems.push(format!(
                "{place}: [fields] the feature `{}` has level {}, outside 1 to 20",
                feature.anchor, feature.level
            ));
        }
        let Some(section) = body_sections
            .iter()
            .find(|section| section.key == feature.anchor)
        else {
            report.problems.push(format!(
                "{place}: [fields] the feature `{}` is not a heading key of the body; write the heading `## Name {{#{}}}`",
                feature.anchor, feature.anchor
            ));
            continue;
        };
        let _ = section.heading;
        let first = paragraph_texts(section.blocks).into_iter().next();
        if let Some(stated) = first.as_deref().and_then(level_lead)
            && stated != feature.level
        {
            report.problems.push(format!(
                "{place}: [fields] the feature `{}` has level {}, but its first paragraph begins at level {stated}",
                feature.anchor, feature.level
            ));
        }
    }
    for trade in entry.fields.trades() {
        let Some(by) = &trade.by else {
            continue;
        };
        if !features.iter().any(|feature| &feature.anchor == by) {
            let reason = if features.is_empty() {
                "the entry has no `features`".to_string()
            } else {
                "it is not an anchor of `features`".to_string()
            };
            report.problems.push(format!(
                "{}: [fields] the {} trade `{}` has `by = \"{by}\"`, but {reason}",
                at(&format!("\"{}\"", trade.printed)),
                trade.op.slug(),
                trade.printed
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus_in_format, heading, paragraph, rule};
    use cast_iron_schema::{
        ArchetypeFields, ClassFields, EntryKind, FeatureFact, Inline, Trade, TradeOp, TypedFields,
    };

    fn text(text: &str) -> Block {
        paragraph(vec![Inline::text(text)])
    }

    fn archetype(
        body: Vec<Block>,
        features: Vec<(&str, u8)>,
        trades: Vec<Trade>,
    ) -> cast_iron_schema::RuleEntry {
        let mut entry = rule("ref/archetype/two-handed-fighter", body);
        entry.kind = EntryKind::Archetype;
        entry.fields = TypedFields::Archetype(ArchetypeFields {
            features: features
                .into_iter()
                .map(|(anchor, level)| FeatureFact {
                    anchor: anchor.into(),
                    level,
                })
                .collect(),
            trades,
            ..Default::default()
        });
        entry
    }

    fn by(op: TradeOp, printed: &str, by: &str) -> Trade {
        Trade {
            by: Some(by.into()),
            ..Trade::unresolved(op, printed)
        }
    }

    fn problems(entry: cast_iron_schema::RuleEntry) -> Vec<String> {
        let corpus = corpus_in_format(4, vec![entry]);
        let mut report = Report::default();
        check_features(&corpus, &mut report);
        report.problems
    }

    fn body() -> Vec<Block> {
        vec![
            heading(Some("shattering-strike"), "Shattering Strike (Ex)"),
            text(
                "At 2nd level, a two-handed fighter gains a bonus. This ability replaces bravery.",
            ),
        ]
    }

    #[test]
    fn a_consistent_entry_has_no_problem() {
        assert!(
            problems(archetype(
                body(),
                vec![("shattering-strike", 2)],
                vec![by(TradeOp::Replaces, "Bravery", "shattering-strike")]
            ))
            .is_empty()
        );
    }

    #[test]
    fn a_feature_anchor_must_be_a_heading_key() {
        let found = problems(archetype(body(), vec![("shatter", 2)], vec![]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("not a heading key"), "{found:?}");
    }

    #[test]
    fn a_feature_is_listed_once() {
        let found = problems(archetype(
            body(),
            vec![("shattering-strike", 2), ("shattering-strike", 2)],
            vec![],
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("listed twice"), "{found:?}");
    }

    #[test]
    fn the_level_must_agree_with_the_lead() {
        let found = problems(archetype(body(), vec![("shattering-strike", 3)], vec![]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("begins at level 2"), "{found:?}");
        // A paragraph without a lead states no level, so any level agrees.
        let quiet = vec![
            heading(Some("shattering-strike"), "Shattering Strike"),
            text("The fighter gains a bonus."),
        ];
        assert!(problems(archetype(quiet, vec![("shattering-strike", 5)], vec![])).is_empty());
        let starting = vec![
            heading(Some("shattering-strike"), "Shattering Strike"),
            text("Starting at 4th level, the fighter gains a bonus."),
        ];
        assert!(problems(archetype(starting, vec![("shattering-strike", 4)], vec![])).is_empty());
    }

    #[test]
    fn by_must_name_a_listed_feature() {
        let found = problems(archetype(
            body(),
            vec![("shattering-strike", 2)],
            vec![by(TradeOp::Replaces, "Bravery", "other")],
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("not an anchor of `features`"),
            "{found:?}"
        );
        let none = problems(archetype(
            body(),
            vec![],
            vec![by(TradeOp::Replaces, "Bravery", "shattering-strike")],
        ));
        assert_eq!(none.len(), 1, "{none:?}");
        assert!(none[0].contains("no `features`"), "{none:?}");
    }

    #[test]
    fn a_feature_section_ends_at_the_next_heading_of_its_level() {
        let blocks = vec![
            heading(Some("a"), "A"),
            text("one"),
            Block::Heading {
                level: 3,
                anchor: Some("inner".into()),
                content: vec![Inline::text("Inner")],
            },
            text("two"),
            heading(Some("b"), "B"),
            text("three"),
        ];
        let found = sections(&blocks, &blocks);
        let keys: Vec<(&str, usize)> = found
            .iter()
            .map(|section| (section.key.as_str(), section.blocks.len()))
            .collect();
        assert_eq!(keys, [("a", 3), ("inner", 1), ("b", 1)]);
    }

    #[test]
    fn a_base_class_may_carry_features() {
        let mut class = rule(
            "ref/class/fighter",
            vec![
                heading(Some("bravery"), "Bravery (Ex)"),
                text("Starting at 2nd level, a fighter gains a bonus."),
            ],
        );
        class.kind = EntryKind::Class;
        class.fields = TypedFields::Class(ClassFields {
            features: vec![FeatureFact {
                anchor: "bravery".into(),
                level: 3,
            }],
            ..Default::default()
        });
        let found = problems(class);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("begins at level 2"), "{found:?}");
    }
}
