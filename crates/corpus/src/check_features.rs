//! The corpus-check rules for feature facts (corpus format 4, #540).
//!
//! An archetype, class option or base class may list `features = [{ anchor, level }]` and an
//! archetype or class option may name, in `trades[].by`, the features whose printed text states
//! the trade. The rules:
//!
//! 1. every `features` anchor is the explicit `{#anchor}` of a heading of the body, no anchor is
//!    listed twice, and the list follows body order;
//! 2. `level`, when written, agrees with an "At Nth level" or "Starting at Nth level" lead, when
//!    the first paragraph under the feature's heading has one;
//! 3. every `by` names anchors of `features`, in body order;
//! 4. every section after the opening text of an archetype or class option that prints a trade
//!    sentence is a listed feature, or sits inside one. The opening text (before the first listed
//!    feature) may print trades that no feature prints; those trades have no `by`.
//!
//! The drift check (`drift.rs`) compares each trade with the trade sentence of its feature.

use crate::check::{Report, line_of};
use crate::drift::is_sources_line;
use crate::load::Corpus;
use crate::trade_sentences::{level_lead, trade_sentences};
use cast_iron_schema::headings::{keyed_headings, plain_text};
use cast_iron_schema::{Block, EntryKind, RuleEntry};

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

/// The part of the body that an entry with `features` reads: the whole body, cut only at a
/// block that starts another publication's version, a `**Sources:**` paragraph that is not the
/// first block (and the `###` heading directly above it). A `###` that only `source_ids` marks
/// does not cut it, because the feature heads of a multi-publication entry may be `###`.
pub(crate) fn feature_part(entry: &RuleEntry) -> &[Block] {
    let body = &entry.body;
    for (index, block) in body.iter().enumerate() {
        if index > 0 && is_sources_line(block) {
            let at = if matches!(body[index - 1], Block::Heading { level: 3, .. }) {
                index - 1
            } else {
                index
            };
            return &body[..at];
        }
    }
    body
}

/// The opening text of an entry with `features`: the blocks of [`feature_part`] before the
/// heading of the first listed feature. A trade that no feature prints (the book states it before
/// the features begin) is stated here. With no listed feature section in the body, all of it.
pub(crate) fn opening_blocks(entry: &RuleEntry) -> &[Block] {
    let blocks = feature_part(entry);
    let features = entry.fields.features();
    let first = sections(blocks, &entry.body)
        .into_iter()
        .find(|section| features.iter().any(|feature| feature.anchor == section.key))
        .and_then(|section| {
            blocks
                .iter()
                .position(|block| std::ptr::eq(block, section.heading))
        });
    &blocks[..first.unwrap_or(blocks.len())]
}

pub(crate) fn check_features(corpus: &Corpus, report: &mut Report) {
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let features = entry.fields.features();
        let trades = entry.fields.trades();
        if features.is_empty() && trades.iter().all(|trade| trade.by.is_empty()) {
            continue;
        }
        let path = stored.path.display();
        let at = |needle: &str| format!("{path}:{}", line_of(&stored.text, needle));
        check_entry(entry, &at, report);
    }
}

fn check_entry(entry: &RuleEntry, at: &dyn Fn(&str) -> String, report: &mut Report) {
    let features = entry.fields.features();
    let blocks = feature_part(entry);
    let body_sections = sections(blocks, &entry.body);
    let mut last_position: Option<usize> = None;
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
        if let Some(level) = feature.level
            && !(1..=20).contains(&level)
        {
            report.problems.push(format!(
                "{place}: [fields] the feature `{}` has level {level}, outside 1 to 20",
                feature.anchor
            ));
        }
        let Some((position, section)) = body_sections
            .iter()
            .enumerate()
            .find(|(_, section)| section.key == feature.anchor)
        else {
            report.problems.push(format!(
                "{place}: [fields] the feature `{}` is not a heading key of the body; write the heading `## Name {{#{}}}`",
                feature.anchor, feature.anchor
            ));
            continue;
        };
        // A feature anchor becomes a public deep link, so the heading states it: a key that
        // the page derives from the heading text would change with the text.
        let explicit = matches!(
            section.heading,
            Block::Heading { anchor: Some(anchor), .. } if *anchor == feature.anchor
        );
        if !explicit {
            report.problems.push(format!(
                "{place}: [fields] the heading of the feature `{}` needs an explicit `{{#{}}}` anchor; a derived key changes with the heading text",
                feature.anchor, feature.anchor
            ));
        }
        if last_position.is_some_and(|last| position < last) {
            report.problems.push(format!(
                "{place}: [fields] the feature `{}` comes before an earlier one in the body; list `features` in body order",
                feature.anchor
            ));
        }
        last_position = Some(position);
        if let Some(level) = feature.level {
            let first = paragraph_texts(section.blocks).into_iter().next();
            if let Some(stated) = first.as_deref().and_then(level_lead)
                && stated != level
            {
                report.problems.push(format!(
                    "{place}: [fields] the feature `{}` has level {level}, but its first paragraph begins at level {stated}",
                    feature.anchor
                ));
            }
        }
    }
    let listed = |anchor: &str| features.iter().any(|feature| feature.anchor == anchor);
    for trade in entry.fields.trades() {
        let mut previous: Option<usize> = None;
        for by in &trade.by {
            let place = at(&format!("\"{}\"", trade.printed));
            let Some(index) = features.iter().position(|feature| &feature.anchor == by) else {
                let reason = if features.is_empty() {
                    "the entry has no `features`"
                } else {
                    "it is not an anchor of `features`"
                };
                report.problems.push(format!(
                    "{place}: [fields] the {} trade `{}` has `by` = `{by}`, but {reason}",
                    trade.op.slug(),
                    trade.printed
                ));
                continue;
            };
            if previous.is_some_and(|last| index < last) {
                report.problems.push(format!(
                    "{place}: [fields] the `by` list of the {} trade `{}` is not in body order (`{by}` comes before an earlier one)",
                    trade.op.slug(),
                    trade.printed
                ));
            }
            previous = Some(index);
        }
    }
    // Every section that prints a trade sentence must be a listed feature, or sit inside one.
    if features.is_empty() || !matches!(entry.kind, EntryKind::Archetype | EntryKind::ClassOption) {
        return;
    }
    let keyed = keyed_headings(&entry.body);
    let key_of = |block: &Block| {
        keyed
            .iter()
            .find(|(candidate, _)| std::ptr::eq(*candidate, block))
            .and_then(|(_, key)| key.clone())
    };
    let mut stack: Vec<(u8, Option<String>)> = Vec::new();
    // The opening text (before the first listed feature) may state trades that no feature prints.
    let opening = opening_blocks(entry).len();
    for (position, block) in blocks.iter().enumerate() {
        if let Block::Heading { level, .. } = block {
            while stack.last().is_some_and(|(open, _)| open >= level) {
                stack.pop();
            }
            stack.push((*level, key_of(block)));
            continue;
        }
        if position < opening {
            continue;
        }
        let texts = paragraph_texts(std::slice::from_ref(block));
        let Some(sentence) = texts.iter().flat_map(|text| trade_sentences(text)).next() else {
            continue;
        };
        let covered = stack
            .iter()
            .any(|(_, key)| key.as_deref().is_some_and(listed));
        if !covered {
            let home = stack.last().and_then(|(_, key)| key.clone()).map_or_else(
                || "before the first heading".to_string(),
                |key| format!("under the heading `{key}`"),
            );
            let needle = stack
                .last()
                .and_then(|(_, key)| key.as_ref())
                .map_or_else(String::new, |key| format!("{{#{key}}}"));
            report.problems.push(format!(
                "{}: [fields] a trade sentence ({} {}) stands {home}, which is not listed in `features`; list the feature, or the trade has no feature to name",
                at(&needle),
                sentence.op.slug(),
                sentence.phrase
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
        features: Vec<(&str, Option<u8>)>,
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

    fn by(op: TradeOp, printed: &str, by: &[&str]) -> Trade {
        Trade {
            by: by.iter().map(|anchor| anchor.to_string()).collect(),
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
                vec![("shattering-strike", Some(2))],
                vec![by(TradeOp::Replaces, "Bravery", &["shattering-strike"])]
            ))
            .is_empty()
        );
    }

    #[test]
    fn a_feature_anchor_must_be_a_heading_key() {
        let found = problems(archetype(body(), vec![("shatter", Some(2))], vec![]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("not a heading key"), "{found:?}");
    }

    #[test]
    fn a_feature_is_listed_once() {
        let found = problems(archetype(
            body(),
            vec![
                ("shattering-strike", Some(2)),
                ("shattering-strike", Some(2)),
            ],
            vec![],
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("listed twice"), "{found:?}");
    }

    #[test]
    fn a_feature_heading_states_its_anchor() {
        // The key `sec-shattering-strike` is derived from the text, so it is no deep link.
        let derived = vec![
            heading(None, "Shattering Strike"),
            text("At 2nd level, the fighter gains a bonus."),
        ];
        let found = problems(archetype(
            derived,
            vec![("sec-shattering-strike", Some(2))],
            vec![],
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("explicit `{#sec-shattering-strike}`"),
            "{found:?}"
        );
    }

    #[test]
    fn the_level_must_agree_with_the_lead_when_it_is_written() {
        let found = problems(archetype(
            body(),
            vec![("shattering-strike", Some(3))],
            vec![],
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("begins at level 2"), "{found:?}");
        // No level written: nothing to compare.
        assert!(problems(archetype(body(), vec![("shattering-strike", None)], vec![])).is_empty());
        // A paragraph without a lead states no level, so any level agrees.
        let quiet = vec![
            heading(Some("shattering-strike"), "Shattering Strike"),
            text("The fighter gains a bonus."),
        ];
        assert!(
            problems(archetype(
                quiet,
                vec![("shattering-strike", Some(5))],
                vec![]
            ))
            .is_empty()
        );
        let starting = vec![
            heading(Some("shattering-strike"), "Shattering Strike"),
            text("Starting at 4th level, the fighter gains a bonus."),
        ];
        assert!(
            problems(archetype(
                starting,
                vec![("shattering-strike", Some(4))],
                vec![]
            ))
            .is_empty()
        );
    }

    #[test]
    fn features_may_mix_leveled_and_unleveled_and_must_follow_body_order() {
        let body = vec![
            heading(Some("a"), "A"),
            text("At 1st level, a."),
            heading(Some("b"), "B"),
            text("Choose one."),
        ];
        assert!(
            problems(archetype(
                body.clone(),
                vec![("a", Some(1)), ("b", None)],
                vec![]
            ))
            .is_empty()
        );
        let found = problems(archetype(body, vec![("b", None), ("a", Some(1))], vec![]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("body order"), "{found:?}");
    }

    #[test]
    fn by_must_name_listed_features_in_body_order() {
        let found = problems(archetype(
            body(),
            vec![("shattering-strike", Some(2))],
            vec![by(TradeOp::Replaces, "Bravery", &["other"])],
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("not an anchor of `features`"),
            "{found:?}"
        );
        let none = problems(archetype(
            body(),
            vec![],
            vec![by(TradeOp::Replaces, "Bravery", &["shattering-strike"])],
        ));
        assert_eq!(none.len(), 1, "{none:?}");
        assert!(none[0].contains("no `features`"), "{none:?}");
        let two = vec![
            heading(Some("a"), "A"),
            text("At 1st level, a. This ability replaces bravery."),
            heading(Some("b"), "B"),
            text("At 2nd level, b. This ability replaces bravery."),
        ];
        let features = vec![("a", Some(1)), ("b", Some(2))];
        assert!(
            problems(archetype(
                two.clone(),
                features.clone(),
                vec![by(TradeOp::Replaces, "Bravery", &["a", "b"])]
            ))
            .is_empty()
        );
        let found = problems(archetype(
            two,
            features,
            vec![by(TradeOp::Replaces, "Bravery", &["b", "a"])],
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("not in body order"), "{found:?}");
    }

    #[test]
    fn a_section_with_a_trade_sentence_must_be_a_listed_feature() {
        let body = vec![
            heading(Some("a"), "A"),
            text("At 1st level, a. This ability replaces bravery."),
            heading(Some("b"), "B"),
            text("At 2nd level, b. This ability alters armor training."),
        ];
        let found = problems(archetype(
            body.clone(),
            vec![("a", Some(1))],
            vec![by(TradeOp::Replaces, "Bravery", &["a"])],
        ));
        // The opening text, before the first listed feature, may state trades with no `by`.
        let mut opened = body.clone();
        opened.insert(0, text("This ability replaces trap sense."));
        let opened_found = problems(archetype(
            opened,
            vec![("a", Some(1)), ("b", Some(2))],
            vec![
                by(TradeOp::Replaces, "Trap sense", &[]),
                by(TradeOp::Replaces, "Bravery", &["a"]),
                by(TradeOp::Alters, "Armor training", &["b"]),
            ],
        ));
        assert!(opened_found.is_empty(), "{opened_found:?}");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("not listed in `features`"), "{found:?}");
        assert!(found[0].contains("`b`"), "{found:?}");
        // A subsection of a listed feature belongs to it.
        let nested = vec![
            heading(Some("a"), "A"),
            text("At 1st level, a."),
            Block::Heading {
                level: 3,
                anchor: Some("inner".into()),
                content: vec![Inline::text("Inner")],
            },
            text("This ability replaces bravery."),
        ];
        assert!(problems(archetype(nested, vec![("a", Some(1))], vec![])).is_empty());
    }

    #[test]
    fn a_feature_after_a_heading_that_only_source_ids_marks_is_still_read() {
        // Two publications: the old normal part would stop at the first `###`.
        let body = vec![
            Block::Heading {
                level: 3,
                anchor: Some("a".into()),
                content: vec![Inline::text("A")],
            },
            text("At 1st level, a."),
            Block::Heading {
                level: 3,
                anchor: Some("b".into()),
                content: vec![Inline::text("B")],
            },
            text("At 2nd level, b."),
        ];
        let mut entry = archetype(body, vec![("a", Some(1)), ("b", Some(2))], vec![]);
        entry.source_ids = vec!["core-rulebook".into(), "other-book".into()];
        assert!(problems(entry).is_empty());
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
                level: Some(3),
            }],
            ..Default::default()
        });
        let found = problems(class);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("begins at level 2"), "{found:?}");
    }
}
