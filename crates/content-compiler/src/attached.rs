//! Derived indexes for attached entries: the option-group vocabulary and the typed trades. The
//! option groups are a view over the entries and the corpus; the trades move out of the entries
//! into their own file.

use cast_iron_corpus::Corpus;
use cast_iron_schema::{
    ClassOptionFields, EntryId, EntryKind, OptionGroup, OptionGroupIndex, RuleEntry, Trade,
    TradeIndex, TradeRow, TypedFields,
};
use std::collections::{BTreeMap, BTreeSet};

/// `option-groups.json`: every group the corpus declares, with the `class_option` entries that
/// belong to it. Sorted by slug; entry ids are sorted inside a group.
pub fn option_groups(corpus: &Corpus, entries: &[RuleEntry]) -> OptionGroupIndex {
    let mut used: BTreeMap<&str, BTreeSet<EntryId>> = BTreeMap::new();
    for entry in entries {
        let TypedFields::ClassOption(ClassOptionFields { group, .. }) = &entry.fields else {
            continue;
        };
        used.entry(group.as_str())
            .or_default()
            .insert(entry.id.clone());
    }
    let groups = corpus
        .option_groups
        .groups
        .iter()
        .map(|(slug, group)| OptionGroup {
            slug: slug.clone(),
            label: group.label.clone(),
            singular: group.singular.clone(),
            classes: group
                .classes
                .iter()
                .filter_map(|id| EntryId::parse(id).ok())
                .collect(),
            entries: used
                .remove(slug.as_str())
                .map(|ids| ids.into_iter().collect())
                .unwrap_or_default(),
        })
        .collect();
    OptionGroupIndex { groups }
}

/// What the trades lowering needs to know about the classes an entry trades against.
type ClassAnchors<'a> = BTreeMap<&'a str, BTreeSet<String>>;

/// Settles the `class` of every featured trade of one entry, in place.
///
/// `candidates` are the base classes of the entry (the `archetype` relation targets of an
/// archetype, the classes of the group of a class option) and `anchors` holds each class's
/// heading anchors. A trade whose anchor only one candidate holds keeps no `class`, written or
/// not. When several candidates hold the anchor, the trade must name one of them. An anchor that
/// no candidate holds, or a class that is not a candidate or lacks the anchor, is an error.
///
/// A corpus-format-1 trade cannot name a class, so with `strict` false an anchor that several
/// candidates hold leaves the trade unresolved (`feature` is `None`, the printed name stays)
/// instead of failing: a filter never treats an unresolved trade as compatible.
fn lower_entry(
    id: &EntryId,
    trades: &mut [Trade],
    candidates: &[&str],
    anchors: &ClassAnchors<'_>,
    strict: bool,
) -> Vec<String> {
    let mut errors = Vec::new();
    let candidate_list = || {
        if candidates.is_empty() {
            "no base class".to_string()
        } else {
            candidates
                .iter()
                .map(|class| format!("`{class}`"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    for trade in trades.iter_mut() {
        let Some(feature) = trade.feature.as_mut() else {
            continue;
        };
        let holders: Vec<&str> = candidates
            .iter()
            .copied()
            .filter(|class| {
                anchors
                    .get(class)
                    .is_some_and(|set| set.contains(&feature.anchor))
            })
            .collect();
        if let Some(named) = &feature.class {
            if !holders.contains(&named.as_str()) {
                errors.push(format!(
                    "{id}: the {} trade `{}` names class `{named}` for feature `{}`, but that class is not a base class holding the anchor; the base classes that hold it are {}",
                    trade.op.slug(),
                    trade.printed,
                    feature.anchor,
                    if holders.is_empty() {
                        "none".to_string()
                    } else {
                        holders
                            .iter()
                            .map(|class| format!("`{class}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ));
                continue;
            }
            if holders.len() == 1 {
                feature.class = None;
            }
            continue;
        }
        match holders.len() {
            0 => errors.push(format!(
                "{id}: the {} trade `{}` has feature `{}`, which is not a heading anchor of {}",
                trade.op.slug(),
                trade.printed,
                feature.anchor,
                candidate_list()
            )),
            1 => {}
            _ if strict => errors.push(format!(
                "{id}: the {} trade `{}` has feature `{}`, which {} hold; add `class` to say which",
                trade.op.slug(),
                trade.printed,
                feature.anchor,
                holders
                    .iter()
                    .map(|class| format!("`{class}`"))
                    .collect::<Vec<_>>()
                    .join(" and ")
            )),
            _ => trade.feature = None,
        }
    }
    errors
}

/// `trades.json`: the trades of every archetype and class option, one row per entry that has
/// any, sorted by id. Lowers the trades first (see [`lower_entry`]), then moves them out of the
/// entries: the entry shards hold no trades, and the format-1 anchor lists are dropped.
///
/// Fails with every entry that the authored data cannot settle. A corpus of format 1 never fails
/// for an ambiguous class, because it has no way to name one.
pub fn trades(corpus: &Corpus, entries: &mut [RuleEntry]) -> Result<TradeIndex, Vec<String>> {
    let strict = corpus.config.format >= 2;
    let mut anchors: ClassAnchors<'_> = BTreeMap::new();
    for class in corpus
        .entries
        .values()
        .filter(|stored| stored.entry.kind == EntryKind::Class)
    {
        anchors.insert(
            class.entry.id.as_str(),
            cast_iron_corpus::heading_keys(&class.entry)
                .into_iter()
                .map(|(key, _)| key)
                .collect(),
        );
    }
    let mut rows: BTreeMap<EntryId, Vec<Trade>> = BTreeMap::new();
    let mut errors = Vec::new();
    for entry in entries.iter_mut() {
        if entry.fields.trades().is_empty() {
            continue;
        }
        // The corpus holds the entry as read; the candidates come from it, not from the clone.
        let candidates: Vec<&str> = corpus
            .entries
            .get(entry.id.as_str())
            .map(|stored| cast_iron_corpus::base_class_ids(corpus, &stored.entry))
            .unwrap_or_default();
        let id = entry.id.clone();
        if let Some(list) = entry.fields.trades_mut() {
            errors.extend(lower_entry(&id, list, &candidates, &anchors, strict));
            rows.insert(id, std::mem::take(list));
        }
        match &mut entry.fields {
            TypedFields::Archetype(fields) => fields.legacy_features = Default::default(),
            TypedFields::ClassOption(fields) => fields.legacy_features = Default::default(),
            _ => {}
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(TradeIndex {
        entries: rows
            .into_iter()
            .map(|(id, trades)| TradeRow { id, trades })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{FeatureRef, TradeOp};

    fn id() -> EntryId {
        EntryId::parse("ref/arc-fighter/example").unwrap()
    }

    fn anchors<'a>(rows: &[(&'a str, &[&str])]) -> ClassAnchors<'a> {
        rows.iter()
            .map(|(class, keys)| (*class, keys.iter().map(|key| key.to_string()).collect()))
            .collect()
    }

    fn featured(anchor: &str, class: Option<&str>) -> Trade {
        Trade {
            op: TradeOp::Replaces,
            printed: anchor.to_string(),
            feature: Some(FeatureRef {
                anchor: anchor.into(),
                class: class.map(|class| EntryId::parse(class).unwrap()),
                ..Default::default()
            }),
        }
    }

    fn class_of(trade: &Trade) -> Option<&str> {
        trade
            .feature
            .as_ref()
            .and_then(|feature| feature.class.as_ref())
            .map(EntryId::as_str)
    }

    #[test]
    fn an_anchor_that_one_class_holds_keeps_no_class_even_when_it_is_written() {
        let table = anchors(&[("ref/class/fighter", &["bravery"])]);
        let mut trades = vec![
            featured("bravery", None),
            featured("bravery", Some("ref/class/fighter")),
        ];
        let errors = lower_entry(&id(), &mut trades, &["ref/class/fighter"], &table, true);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(trades.iter().all(|trade| class_of(trade).is_none()));
    }

    #[test]
    fn an_anchor_that_several_classes_hold_keeps_its_written_class() {
        let table = anchors(&[
            ("ref/class/bard", &["bardic-performance"]),
            ("ref/class/skald", &["bardic-performance", "rage-song"]),
        ]);
        let candidates = ["ref/class/bard", "ref/class/skald"];
        let mut trades = vec![
            featured("bardic-performance", Some("ref/class/skald")),
            featured("rage-song", None),
        ];
        let errors = lower_entry(&id(), &mut trades, &candidates, &table, true);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(class_of(&trades[0]), Some("ref/class/skald"));
        assert_eq!(class_of(&trades[1]), None);
    }

    #[test]
    fn an_ambiguous_anchor_without_a_class_is_an_error_in_format_2() {
        let table = anchors(&[
            ("ref/class/bard", &["bardic-performance"]),
            ("ref/class/skald", &["bardic-performance"]),
        ]);
        let mut trades = vec![featured("bardic-performance", None)];
        let errors = lower_entry(
            &id(),
            &mut trades,
            &["ref/class/bard", "ref/class/skald"],
            &table,
            true,
        );
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0]
                .contains("`ref/class/bard` and `ref/class/skald` hold; add `class` to say which"),
            "{errors:?}"
        );
    }

    #[test]
    fn an_ambiguous_format_1_anchor_degrades_to_an_unresolved_trade() {
        let table = anchors(&[
            ("ref/class/bard", &["bardic-performance"]),
            ("ref/class/skald", &["bardic-performance"]),
        ]);
        let mut trades = vec![featured("bardic-performance", None)];
        let errors = lower_entry(
            &id(),
            &mut trades,
            &["ref/class/bard", "ref/class/skald"],
            &table,
            false,
        );
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(trades[0].printed, "bardic-performance");
        assert!(trades[0].feature.is_none());
    }

    #[test]
    fn an_unknown_anchor_and_a_wrong_class_are_errors() {
        let table = anchors(&[
            ("ref/class/bard", &["bardic-performance"]),
            ("ref/class/skald", &["rage-song"]),
        ]);
        let candidates = ["ref/class/bard", "ref/class/skald"];
        let mut trades = vec![
            featured("missing", None),
            featured("bardic-performance", Some("ref/class/skald")),
            featured("bardic-performance", Some("ref/class/rogue")),
        ];
        let errors = lower_entry(&id(), &mut trades, &candidates, &table, true);
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].contains("not a heading anchor of `ref/class/bard`, `ref/class/skald`"));
        assert!(errors[1].contains("names class `ref/class/skald`"));
        assert!(errors[2].contains("names class `ref/class/rogue`"));
    }

    #[test]
    fn an_unresolved_trade_passes_through_untouched() {
        let mut trades = vec![Trade::unresolved(TradeOp::Alters, "2nd-level bonus feat")];
        let errors = lower_entry(&id(), &mut trades, &[], &BTreeMap::new(), true);
        assert!(errors.is_empty());
        assert_eq!(trades[0].printed, "2nd-level bonus feat");
    }
}
