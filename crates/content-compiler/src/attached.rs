//! Derived indexes for attached entries: the option-group vocabulary and the typed
//! replacements. Both are small views over data that stays on the entries and in the corpus.

use cast_iron_corpus::Corpus;
use cast_iron_schema::{
    ClassOptionFields, EntryId, OptionGroup, OptionGroupIndex, ReplacesIndex, ReplacesRow,
    RuleEntry, TypedFields,
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

/// `replaces.json`: the printed items that each archetype or class option replaces. Sorted by
/// id. An entry with no `replaces` items is left out, so the file stays empty until slice 6
/// types the printed lines.
pub fn replaces(entries: &[RuleEntry]) -> ReplacesIndex {
    let mut rows: BTreeMap<EntryId, Vec<String>> = BTreeMap::new();
    for entry in entries {
        let replaces = match &entry.fields {
            TypedFields::Archetype(fields) => &fields.replaces,
            TypedFields::ClassOption(fields) => &fields.replaces,
            _ => continue,
        };
        if !replaces.is_empty() {
            rows.insert(entry.id.clone(), replaces.clone());
        }
    }
    ReplacesIndex {
        entries: rows
            .into_iter()
            .map(|(id, replaces)| ReplacesRow { id, replaces })
            .collect(),
    }
}
