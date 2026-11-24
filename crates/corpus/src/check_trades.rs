//! The corpus-check rules for typed trades (#451), beyond the ones `check_replaces_features`
//! already holds (a `feature` is an anchor of a base class, a `class` is a base class that holds
//! it) and the drift check (the joined trades agree with the `**Replaces:**` and `**Alters:**`
//! lines of the normal part, and an entry with such lines has trades). The reader holds the
//! shape rules: a trade has exactly one op key with a text, and `levels`, `power` and `class`
//! need a `feature`. The compiler rejects, in format 2, an anchor that several base classes hold
//! when the trade writes no `class`.
//!
//! A rule that format 1 cannot break is an error in both formats. A rule that the pinned
//! format-1 corpus may break is an error in format 2 and a warning in format 1, because the
//! corpus repository builds its checker from this repository's `main` and must stay green until
//! its migration (#452) lands.

use crate::check::{Report, base_class_ids, line_of};
use crate::load::Corpus;
use crate::trade_match::{BaseHeading, Verdict, overlaps, resolve};
use cast_iron_schema::{FeatureRef, Trade, TypedFields};
use std::collections::{BTreeMap, BTreeSet};

/// The problem with a trade's `levels`, if any: they run 1 to 20, ascending and unique.
fn bad_levels(feature: &FeatureRef) -> Option<String> {
    let levels = &feature.levels;
    if let Some(level) = levels.iter().find(|level| !(1..=20).contains(*level)) {
        return Some(format!("level {level} is outside 1 to 20"));
    }
    if levels.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Some(format!("{levels:?} is not ascending and unique"));
    }
    None
}

pub(crate) fn check_trades(
    corpus: &Corpus,
    keys: &BTreeMap<&str, Vec<(String, String)>>,
    report: &mut Report,
) {
    let format_1 = corpus.config.format == 1;
    let mut trades_total = 0usize;
    let mut unresolved_total = 0usize;
    let mut unresolved_entries = 0usize;
    let mut provable = 0usize;
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let trades: &[Trade] = match &entry.fields {
            TypedFields::Archetype(fields) => &fields.trades,
            TypedFields::ClassOption(fields) => &fields.trades,
            _ => continue,
        };
        if trades.is_empty() {
            continue;
        }
        let path = stored.path.display();
        let at = |trade: &Trade| {
            format!(
                "{path}:{}",
                line_of(&stored.text, &format!("\"{}\"", trade.printed))
            )
        };
        let classes = base_class_ids(corpus, entry);
        let headings: Vec<BaseHeading> = classes
            .iter()
            .filter_map(|class| keys.get(class).map(|list| (class, list)))
            .flat_map(|(class, list)| {
                list.iter().map(|(key, text)| BaseHeading {
                    class: class.to_string(),
                    key: key.clone(),
                    text: text.clone(),
                })
            })
            .collect();
        let words: BTreeSet<String> = classes
            .iter()
            .flat_map(|class| class.rsplit('/').next().unwrap_or(class).split('-'))
            .map(str::to_string)
            .collect();

        trades_total += trades.len();
        let mut entry_unresolved = 0usize;
        for (index, trade) in trades.iter().enumerate() {
            let label = trade.op.slug();
            let Some(feature) = &trade.feature else {
                entry_unresolved += 1;
                // A confident match for an unresolved trade means the migration was not run, or
                // a person left a provable trade unresolved.
                // A match that another trade of the key already holds is the migration's
                // `duplicate-feature`: it stays unresolved on purpose, so it is not a hint.
                if let Verdict::Resolved(found, rule) = resolve(&trade.printed, &headings, &words)
                    && !trades.iter().any(|other| {
                        other.op == trade.op
                            && other
                                .feature
                                .as_ref()
                                .is_some_and(|feature| overlaps(feature, &found))
                    })
                {
                    provable += 1;
                    if !format_1 {
                        report.warnings.push(format!(
                            "{}: [fields] the {label} trade `{}` is unresolved, but its printed name resolves to `{}` by rule `{}`; run `cast-iron-corpus migrate-trades` or write the feature",
                            at(trade),
                            trade.printed,
                            found.anchor,
                            rule.slug()
                        ));
                    }
                }
                continue;
            };
            if let Some(why) = bad_levels(feature) {
                report.problems.push(format!(
                    "{}: [fields] the {label} trade `{}` has `levels` where {why}",
                    at(trade),
                    trade.printed
                ));
            }
            for later in &trades[index + 1..] {
                let Some(other) = &later.feature else {
                    continue;
                };
                // An archetype may alter a feature and replace one instance of it (`Hexes` and
                // `6th-level hex`), so only trades of one key compete.
                if trade.op == later.op && overlaps(feature, other) {
                    let message = format!(
                        "{}: [fields] the {label} trade `{}` and the {label} trade `{}` both name feature `{}` at the same levels{}",
                        at(later),
                        trade.printed,
                        later.printed,
                        feature.anchor,
                        feature
                            .power
                            .as_ref()
                            .map_or(String::new(), |power| format!(" and power `{power}`"))
                    );
                    if format_1 {
                        report.warnings.push(message);
                    } else {
                        report.problems.push(message);
                    }
                }
            }
        }
        if entry_unresolved > 0 {
            unresolved_entries += 1;
            unresolved_total += entry_unresolved;
        }
    }
    if unresolved_total > 0 {
        let hint = if provable > 0 {
            if format_1 {
                format!(
                    "; {provable} of them have a provable match that `cast-iron-corpus migrate-trades` writes in format 2"
                )
            } else {
                format!("; {provable} of them have a provable match")
            }
        } else {
            String::new()
        };
        report.warnings.push(format!(
            "{unresolved_total} of {trades_total} trades in {unresolved_entries} entries are unresolved (no `feature`){hint}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::check;
    use crate::testkit::{corpus_in_format, heading, paragraph, rule};
    use cast_iron_schema::{
        ArchetypeFields, ClassOptionFields, EntryId, EntryKind, Inline, Relation, RelationKind,
        RuleEntry, TradeOp,
    };

    fn typed(mut entry: RuleEntry, kind: EntryKind, fields: TypedFields) -> RuleEntry {
        entry.kind = kind;
        entry.fields = fields;
        entry
    }

    fn class(id: &str, headings: &[(&str, &str)]) -> RuleEntry {
        typed(
            rule(
                id,
                headings
                    .iter()
                    .map(|(anchor, text)| heading(Some(anchor), text))
                    .collect(),
            ),
            EntryKind::Class,
            TypedFields::Class(Default::default()),
        )
    }

    fn fighter() -> RuleEntry {
        class(
            "ref/class/fighter",
            &[
                ("bravery", "Bravery (Ex)"),
                ("bonus-feats", "Bonus Feats"),
                ("weapon-training", "Weapon Training (Ex)"),
            ],
        )
    }

    fn lines(replaces: &str, alters: &str) -> Vec<cast_iron_schema::Block> {
        let line = |label: &str, text: &str| {
            paragraph(vec![
                Inline::Strong {
                    c: vec![Inline::text(format!("{label}:"))],
                },
                Inline::text(format!(" {text}")),
            ])
        };
        let mut body = Vec::new();
        if !replaces.is_empty() {
            body.push(line("Replaces", replaces));
        }
        if !alters.is_empty() {
            body.push(line("Alters", alters));
        }
        body
    }

    fn archetype(trades: Vec<Trade>, replaces: &str, alters: &str) -> RuleEntry {
        let mut entry = typed(
            rule("ref/arc-fighter/example", lines(replaces, alters)),
            EntryKind::Archetype,
            TypedFields::Archetype(ArchetypeFields {
                trades,
                ..Default::default()
            }),
        );
        entry.relations = vec![Relation {
            kind: RelationKind::Archetype,
            target: EntryId::parse("ref/class/fighter").unwrap(),
            label: None,
        }];
        entry
    }

    fn feature_trade(op: TradeOp, printed: &str, anchor: &str) -> Trade {
        Trade {
            by: None,
            op,
            printed: printed.into(),
            feature: Some(FeatureRef {
                anchor: anchor.into(),
                ..Default::default()
            }),
        }
    }

    fn narrowed(printed: &str, anchor: &str, levels: &[u8], power: Option<&str>) -> Trade {
        Trade {
            by: None,
            op: TradeOp::Replaces,
            printed: printed.into(),
            feature: Some(FeatureRef {
                anchor: anchor.into(),
                class: None,
                levels: levels.to_vec(),
                power: power.map(str::to_string),
            }),
        }
    }

    fn run(format: u32, entries: Vec<RuleEntry>) -> Report {
        check(&corpus_in_format(format, entries))
    }

    fn problems(report: &Report) -> String {
        report.problems.join("\n")
    }

    fn warnings(report: &Report) -> String {
        report.warnings.join("\n")
    }

    // Rule: `levels` run 1 to 20, ascending and unique.

    #[test]
    fn levels_in_range_ascending_and_unique_pass() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![narrowed("Bonus feats", "bonus-feats", &[2, 4, 20], None)],
                    "Bonus feats",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
    }

    #[test]
    fn levels_that_are_out_of_range_descending_or_repeated_fail() {
        for (levels, why) in [
            (vec![0], "level 0 is outside 1 to 20"),
            (vec![21], "level 21 is outside 1 to 20"),
            (vec![4, 2], "not ascending and unique"),
            (vec![4, 4], "not ascending and unique"),
        ] {
            let report = run(
                2,
                vec![
                    fighter(),
                    archetype(
                        vec![narrowed("Bonus feats", "bonus-feats", &levels, None)],
                        "Bonus feats",
                        "",
                    ),
                ],
            );
            let text = problems(&report);
            assert!(text.contains(why), "{levels:?}: {text}");
        }
    }

    // Rule: no two trades share an anchor with overlapping levels and the same power.

    #[test]
    fn trades_on_one_anchor_with_disjoint_levels_or_other_powers_pass() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![
                        narrowed("Bonus feats at 2nd level", "bonus-feats", &[2], None),
                        narrowed("Bonus feats at 4th level", "bonus-feats", &[4], None),
                        narrowed("Weapon training 1", "weapon-training", &[], Some("1")),
                        narrowed("Weapon training 2", "weapon-training", &[], Some("2")),
                    ],
                    "Bonus feats at 2nd level, Bonus feats at 4th level, Weapon training 1, Weapon training 2",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
    }

    #[test]
    fn two_trades_on_one_anchor_with_overlapping_levels_and_one_power_fail_in_format_2() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![
                        narrowed(
                            "Bonus feats at 2nd and 4th levels",
                            "bonus-feats",
                            &[2, 4],
                            None,
                        ),
                        narrowed("Bonus feats at 4th level", "bonus-feats", &[4], None),
                    ],
                    "Bonus feats at 2nd and 4th levels, Bonus feats at 4th level",
                    "",
                ),
            ],
        );
        assert!(
            problems(&report).contains("both name feature `bonus-feats` at the same levels"),
            "{}",
            problems(&report)
        );
        // No levels means every level, so a whole-feature trade overlaps a leveled one.
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![
                        narrowed("Bonus feats", "bonus-feats", &[], None),
                        narrowed("Bonus feats at 4th level", "bonus-feats", &[4], None),
                    ],
                    "Bonus feats, Bonus feats at 4th level",
                    "",
                ),
            ],
        );
        assert!(
            problems(&report).contains("both name feature `bonus-feats`"),
            "{}",
            problems(&report)
        );
    }

    #[test]
    fn an_archetype_may_alter_a_feature_and_replace_one_instance_of_it() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![
                        narrowed("Bonus feat gained at 6th level", "bonus-feats", &[6], None),
                        Trade {
                            by: None,
                            op: TradeOp::Alters,
                            printed: "Bonus feats".into(),
                            feature: Some(FeatureRef {
                                anchor: "bonus-feats".into(),
                                ..Default::default()
                            }),
                        },
                    ],
                    "Bonus feat gained at 6th level",
                    "Bonus feats",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
    }

    #[test]
    fn the_overlap_rule_is_a_warning_in_format_1() {
        let report = run(
            1,
            vec![
                fighter(),
                archetype(
                    vec![
                        feature_trade(TradeOp::Replaces, "Bravery", "bravery"),
                        feature_trade(TradeOp::Replaces, "Bravery (Ex)", "bravery"),
                    ],
                    "Bravery, Bravery (Ex)",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
        assert!(
            warnings(&report).contains("both name feature `bravery`"),
            "{}",
            warnings(&report)
        );
    }

    // Rule: unresolved trades are counted, and a provable match warns.

    #[test]
    fn unresolved_trades_are_counted_in_one_warning() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![
                        Trade::unresolved(TradeOp::Replaces, "Improved evasion"),
                        feature_trade(TradeOp::Replaces, "Bravery", "bravery"),
                    ],
                    "Improved evasion, Bravery",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
        assert!(
            warnings(&report).contains("1 of 2 trades in 1 entries are unresolved"),
            "{}",
            warnings(&report)
        );
        // Nothing provable, so no per-trade warning.
        assert!(
            !warnings(&report).contains("resolves to"),
            "{}",
            warnings(&report)
        );
    }

    #[test]
    fn an_unresolved_trade_with_a_provable_match_warns_in_format_2() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![Trade::unresolved(
                        TradeOp::Replaces,
                        "Bonus feat gained at 6th level",
                    )],
                    "Bonus feat gained at 6th level",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
        let text = warnings(&report);
        assert!(
            text.contains(
                "is unresolved, but its printed name resolves to `bonus-feats` by rule `levels`"
            ),
            "{text}"
        );
    }

    #[test]
    fn an_unresolved_duplicate_of_a_resolved_trade_is_not_a_hint() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![
                        feature_trade(TradeOp::Replaces, "Bravery", "bravery"),
                        Trade::unresolved(TradeOp::Replaces, "Bravery (Ex)"),
                    ],
                    "Bravery, Bravery (Ex)",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
        let text = warnings(&report);
        assert!(
            text.contains("1 of 2 trades in 1 entries are unresolved"),
            "{text}"
        );
        assert!(
            !text.contains("resolves to") && !text.contains("provable"),
            "{text}"
        );
    }

    #[test]
    fn a_format_1_corpus_gets_one_summary_and_no_per_trade_warning() {
        let report = run(
            1,
            vec![
                fighter(),
                archetype(
                    vec![Trade::unresolved(
                        TradeOp::Replaces,
                        "Bonus feat gained at 6th level",
                    )],
                    "Bonus feat gained at 6th level",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
        let text = warnings(&report);
        assert!(
            text.contains("1 of 1 trades in 1 entries are unresolved")
                && text.contains(
                    "1 of them have a provable match that `cast-iron-corpus migrate-trades` writes"
                ),
            "{text}"
        );
        assert!(!text.contains("but its printed name resolves"), "{text}");
    }

    #[test]
    fn a_corpus_whose_trades_all_resolve_has_no_unresolved_warning() {
        let report = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![feature_trade(TradeOp::Replaces, "Bravery", "bravery")],
                    "Bravery",
                    "",
                ),
            ],
        );
        assert!(report.problems.is_empty(), "{}", problems(&report));
        assert!(
            !warnings(&report).contains("unresolved"),
            "{}",
            warnings(&report)
        );
    }

    // Rules the drift check and the reader hold, shown at the check level.

    #[test]
    fn trades_that_agree_with_the_printed_lines_pass_and_a_difference_fails() {
        let ok = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![
                        feature_trade(TradeOp::Replaces, "Bravery", "bravery"),
                        Trade::unresolved(TradeOp::Alters, "Bonus feats"),
                    ],
                    "Bravery",
                    "Bonus feats",
                ),
            ],
        );
        assert!(ok.problems.is_empty(), "{}", problems(&ok));
        let drift = run(
            2,
            vec![
                fighter(),
                archetype(
                    vec![feature_trade(TradeOp::Replaces, "Bravery", "bravery")],
                    "Bonus feats",
                    "",
                ),
            ],
        );
        assert!(
            problems(&drift).contains("disagrees with the `**Replaces:**` line"),
            "{}",
            problems(&drift)
        );
    }

    #[test]
    fn an_entry_with_printed_lines_and_no_trades_fails_and_one_with_trades_passes() {
        let missing = run(2, vec![fighter(), archetype(vec![], "Bravery", "")]);
        assert!(
            problems(&missing).contains("`**Replaces:**` line"),
            "{}",
            problems(&missing)
        );
        let option = typed(
            rule("ref/domain/air", lines("", "")),
            EntryKind::ClassOption,
            TypedFields::ClassOption(ClassOptionFields {
                group: "domain".into(),
                ..Default::default()
            }),
        );
        let mut c = corpus_in_format(2, vec![option]);
        c.option_groups.groups.insert(
            "domain".into(),
            crate::OptionGroupDef {
                label: "Domains".into(),
                singular: "Domain".into(),
                classes: vec![],
            },
        );
        assert!(check(&c).problems.is_empty());
    }

    #[test]
    fn a_trade_with_one_op_key_and_a_text_reads_and_two_op_keys_or_no_text_fail() {
        use crate::config::Config;
        use crate::entry::read_entry;
        let config = Config {
            format: 2,
            ..Config::default()
        };
        let read = |row: &str| {
            let text = format!(
                "+++\nid = \"ref/arc-fighter/example\"\nkind = \"archetype\"\ntitle = \"Example\"\nsource_ids = [\"core-rulebook\"]\n\n[fields]\nkind = \"archetype\"\ntrades = [\n  {row},\n]\n+++\n"
            );
            read_entry(
                std::path::Path::new("entries/arc-fighter/example.md"),
                &text,
                &config,
            )
        };
        assert!(read("{ replaces = \"Bravery\", feature = \"bravery\" }").is_ok());
        for bad in [
            "{ replaces = \"Bravery\", alters = \"Bravery\" }",
            "{ feature = \"bravery\" }",
            "{ replaces = \"\" }",
            "{ replaces = \"  \" }",
        ] {
            assert!(read(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn levels_and_power_need_a_feature() {
        use crate::config::Config;
        use crate::entry::read_entry;
        let config = Config {
            format: 2,
            ..Config::default()
        };
        let read = |row: &str| {
            let text = format!(
                "+++\nid = \"ref/arc-fighter/example\"\nkind = \"archetype\"\ntitle = \"Example\"\nsource_ids = [\"core-rulebook\"]\n\n[fields]\nkind = \"archetype\"\ntrades = [\n  {row},\n]\n+++\n"
            );
            read_entry(
                std::path::Path::new("entries/arc-fighter/example.md"),
                &text,
                &config,
            )
        };
        assert!(
            read("{ replaces = \"Bravery\", feature = \"bravery\", levels = [2], power = \"x\" }")
                .is_ok()
        );
        for bad in [
            "{ replaces = \"Bravery\", levels = [2] }",
            "{ replaces = \"Bravery\", power = \"x\" }",
            "{ replaces = \"Bravery\", class = \"ref/class/fighter\" }",
        ] {
            let error = read(bad).unwrap_err();
            assert!(
                error.message.contains("no `feature`"),
                "{bad}: {}",
                error.message
            );
        }
    }
}
