//! Pure lookups over the class-option group index (`option-groups.json`).
//!
//! The list page, the class page and the metadata lines all need the same three answers: which
//! group an option belongs to, which groups a class takes, and how many of a group's options the
//! active books keep. They live here so the rules are tested without a browser.

use cast_iron_schema::{EntryId, OptionGroup, OptionGroupIndex, TradeIndex, TradeOp, TradeRow};
use std::collections::HashSet;

/// The group that holds `id`, if the index has one.
pub fn group_of<'a>(index: &'a OptionGroupIndex, id: &EntryId) -> Option<&'a OptionGroup> {
    index.groups.iter().find(|group| group.entries.contains(id))
}

/// The groups a class takes, with the number of entries that `eligible` accepts. Empty groups
/// are left out, so a class page never opens a list with nothing in it.
pub fn groups_for_class<'a>(
    index: &'a OptionGroupIndex,
    class: &EntryId,
    eligible: impl Fn(&EntryId) -> bool,
) -> Vec<(&'a OptionGroup, usize)> {
    index
        .groups
        .iter()
        .filter(|group| group.classes.contains(class))
        .filter_map(|group| {
            let count = group.entries.iter().filter(|id| eligible(id)).count();
            (count > 0).then_some((group, count))
        })
        .collect()
}

/// The ids of one group that `allowed` accepts. An unknown slug has no ids.
pub fn group_ids(
    index: &OptionGroupIndex,
    slug: &str,
    allowed: impl Fn(&EntryId) -> bool,
) -> HashSet<String> {
    index
        .groups
        .iter()
        .find(|group| group.slug == slug)
        .map(|group| {
            group
                .entries
                .iter()
                .filter(|id| allowed(id))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The options a group serves more than one class to. Such a group shows a class scope on its
/// list page; a group that only one class takes needs none.
pub fn serves_several_classes(group: &OptionGroup) -> bool {
    group.classes.len() > 1
}

/// The muted line of one trade row: `Replaces Arcane bond, 10th-level bonus feat`. It names the
/// printed names of the replaced features only; `None` when the row replaces nothing. The row
/// reads the same from `trades.json` and from the `replaces.json` of an older library.
pub fn replaces_line_of(row: &TradeRow) -> Option<String> {
    trade_line_of(row, TradeOp::Replaces)
}

/// `Replaces …` or `Alters …`: the printed names of the trades with `op`, or `None` when the
/// row has none.
pub fn trade_line_of(row: &TradeRow, op: TradeOp) -> Option<String> {
    let names = row.printed(op);
    (!names.is_empty()).then(|| format!("{} {}", op.label(), names.join(", ")))
}

/// The muted archetype line for the entry `id`. `None` when the library has no trades for it.
pub fn replaces_line(index: Option<&TradeIndex>, id: &EntryId) -> Option<String> {
    replaces_line_of(index?.row(id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{OptionGroup, ReplacesIndex, ReplacesRow, Trade};

    fn id(value: &str) -> EntryId {
        EntryId::parse(value).unwrap()
    }

    fn group(
        slug: &str,
        label: &str,
        singular: &str,
        classes: &[&str],
        entries: &[&str],
    ) -> OptionGroup {
        OptionGroup {
            slug: slug.into(),
            label: label.into(),
            singular: singular.into(),
            classes: classes.iter().map(|value| id(value)).collect(),
            entries: entries.iter().map(|value| id(value)).collect(),
        }
    }

    fn index() -> OptionGroupIndex {
        OptionGroupIndex {
            groups: vec![
                group(
                    "arcane-discovery",
                    "Arcane discoveries",
                    "Arcane discovery",
                    &["ref/class/wizard"],
                    &["ref/arcanedisc/a", "ref/arcanedisc/b"],
                ),
                group(
                    "alchemist-discovery",
                    "Alchemist discoveries",
                    "Alchemist discovery",
                    &["ref/class/alchemist", "ref/class/investigator"],
                    &["ref/alchemist/a"],
                ),
                group(
                    "empty",
                    "Empty group",
                    "Empty group",
                    &["ref/class/wizard"],
                    &[],
                ),
            ],
        }
    }

    #[test]
    fn a_group_is_found_by_the_option_it_holds() {
        let index = index();
        assert_eq!(
            group_of(&index, &id("ref/arcanedisc/b")).map(|g| g.singular.as_str()),
            Some("Arcane discovery")
        );
        assert!(group_of(&index, &id("ref/unknown/x")).is_none());
    }

    #[test]
    fn a_class_sees_its_groups_with_only_the_counts_the_active_books_keep() {
        let index = index();
        let wizard = id("ref/class/wizard");
        // The empty group is left out; the discovered group counts only eligible entries.
        let groups = groups_for_class(&index, &wizard, |option| {
            option.as_str() != "ref/arcanedisc/b"
        });
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0.slug, "arcane-discovery");
        assert_eq!(groups[0].1, 1);
        assert!(groups_for_class(&index, &wizard, |_| false).is_empty());
        assert!(groups_for_class(&index, &id("ref/class/rogue"), |_| true).is_empty());
    }

    #[test]
    fn a_group_narrows_its_list_to_the_ids_the_page_allows() {
        let index = index();
        let all = group_ids(&index, "arcane-discovery", |_| true);
        assert_eq!(all.len(), 2);
        let allowed = group_ids(&index, "arcane-discovery", |option| {
            option.as_str() == "ref/arcanedisc/a"
        });
        assert_eq!(allowed, HashSet::from(["ref/arcanedisc/a".to_string()]));
        assert!(group_ids(&index, "missing", |_| true).is_empty());
    }

    #[test]
    fn a_shared_group_shows_a_scope_and_a_replaces_line_names_its_items() {
        let index = index();
        assert!(serves_several_classes(&index.groups[1]));
        assert!(!serves_several_classes(&index.groups[0]));
        let replaces = TradeIndex {
            entries: vec![
                TradeRow {
                    id: id("ref/arc-wizard/empty"),
                    trades: vec![Trade::unresolved(TradeOp::Alters, "Arcane school")],
                },
                TradeRow {
                    id: id("ref/arc-wizard/scrollmaster"),
                    trades: vec![
                        Trade::unresolved(TradeOp::Replaces, "Arcane bond"),
                        Trade::unresolved(TradeOp::Replaces, "10th-level bonus feat"),
                        Trade::unresolved(TradeOp::Alters, "Arcane school"),
                    ],
                },
            ],
        };
        assert_eq!(
            replaces_line(Some(&replaces), &id("ref/arc-wizard/scrollmaster")),
            Some("Replaces Arcane bond, 10th-level bonus feat".to_string())
        );
        // A row that only alters has no Replaces line.
        assert_eq!(
            replaces_line(Some(&replaces), &id("ref/arc-wizard/empty")),
            None
        );
        assert_eq!(
            replaces_line(None, &id("ref/arc-wizard/scrollmaster")),
            None
        );
    }

    #[test]
    fn an_older_library_shows_its_printed_names_from_replaces_json() {
        // A schema-4 or schema-5 library has `replaces.json` and no `trades.json`.
        let older = ReplacesIndex {
            entries: vec![ReplacesRow {
                id: id("ref/arc-wizard/scrollmaster"),
                replaces: vec!["Arcane bond".into()],
            }],
        };
        let index = TradeIndex::from_replaces(older);
        assert_eq!(
            replaces_line(Some(&index), &id("ref/arc-wizard/scrollmaster")),
            Some("Replaces Arcane bond".to_string())
        );
        // No feature: a compatibility filter has nothing to compare, so it must not match.
        assert!(
            index.entries[0]
                .trades
                .iter()
                .all(|trade| trade.feature.is_none())
        );
    }
}
