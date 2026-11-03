//! Spell lists per class and level, indexed from `SpellFields::levels`.
//!
//! The spell entries stay the source of truth. This pass only groups them so that "Wizard
//! level 3 spells" does not need every spell shard. Class names are grouped without regard
//! to case; the display name is the class entry's title when a class entry exists.

use cast_iron_schema::{
    EntryId, EntryKind, RuleEntry, SpellListClass, SpellListCoverage, SpellListLevel,
    SpellListSpell, SpellLists, TypedFields, UnmappedClass,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Report {
    pub lists: SpellLists,
    pub coverage: SpellListCoverage,
}

pub fn build(entries: &[RuleEntry]) -> Report {
    // Class entries by lowercase title. A plain class wins over a prestige class.
    let mut class_entries: BTreeMap<String, (&RuleEntry, bool)> = BTreeMap::new();
    for entry in entries
        .iter()
        .filter(|e| matches!(e.kind, EntryKind::Class | EntryKind::PrestigeClass))
    {
        let prestige = entry.kind == EntryKind::PrestigeClass;
        class_entries
            .entry(entry.title.trim().to_lowercase())
            .and_modify(|current| {
                if current.1 && !prestige {
                    *current = (entry, prestige);
                }
            })
            .or_insert((entry, prestige));
    }

    struct ClassAcc<'a> {
        name: String,
        entry: Option<&'a RuleEntry>,
        // level -> spell ids
        levels: BTreeMap<u8, BTreeSet<&'a EntryId>>,
    }
    let mut classes: BTreeMap<String, ClassAcc> = BTreeMap::new();
    let mut spell_titles: BTreeMap<&EntryId, &str> = BTreeMap::new();
    let mut pairs_upstream = 0u32;
    let mut spells = 0u32;
    for entry in entries.iter().filter(|e| e.kind == EntryKind::Spell) {
        let TypedFields::Spell(fields) = &entry.fields else {
            continue;
        };
        spells += 1;
        spell_titles.insert(&entry.id, &entry.title);
        for level in &fields.levels {
            pairs_upstream += 1;
            let name = level.class.split_whitespace().collect::<Vec<_>>().join(" ");
            let key = name.to_lowercase();
            let acc = classes.entry(key.clone()).or_insert_with(|| {
                let mapped = class_entries.get(&key).map(|(e, _)| *e);
                ClassAcc {
                    name: mapped.map_or(name.clone(), |e| e.title.clone()),
                    entry: mapped,
                    levels: BTreeMap::new(),
                }
            });
            acc.levels.entry(level.level).or_default().insert(&entry.id);
        }
    }

    // Spell table: every spell that any list holds, by id.
    let used: BTreeSet<&EntryId> = classes
        .values()
        .flat_map(|c| c.levels.values().flatten().copied())
        .collect();
    let table: Vec<SpellListSpell> = used
        .iter()
        .map(|id| SpellListSpell((*id).clone(), spell_titles[id].to_string()))
        .collect();
    let position: BTreeMap<&EntryId, u32> = used
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i as u32))
        .collect();

    let mut list_classes: Vec<SpellListClass> = Vec::new();
    let mut pairs_distinct = 0u32;
    let mut with_entry = Vec::new();
    let mut unmapped = Vec::new();
    for acc in classes.values() {
        let mut levels = Vec::new();
        let mut distinct = BTreeSet::new();
        for (level, ids) in &acc.levels {
            let mut ids: Vec<&EntryId> = ids.iter().copied().collect();
            ids.sort_by(|a, b| {
                spell_titles[a]
                    .to_lowercase()
                    .cmp(&spell_titles[b].to_lowercase())
                    .then(a.cmp(b))
            });
            pairs_distinct += ids.len() as u32;
            distinct.extend(ids.iter().copied());
            levels.push(SpellListLevel {
                level: *level,
                spells: ids.iter().map(|id| position[id]).collect(),
            });
        }
        match acc.entry {
            Some(_) => with_entry.push(acc.name.clone()),
            None => unmapped.push(UnmappedClass {
                name: acc.name.clone(),
                spells: distinct.len() as u32,
            }),
        }
        list_classes.push(SpellListClass {
            name: acc.name.clone(),
            class: acc.entry.map(|e| e.id.clone()),
            levels,
        });
    }
    list_classes.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name))
    });
    with_entry.sort();
    unmapped.sort_by(|a, b| a.name.cmp(&b.name));
    Report {
        coverage: SpellListCoverage {
            spells,
            classes: list_classes.len() as u32,
            pairs_upstream,
            pairs_distinct,
            classes_with_entry: with_entry,
            unmapped,
        },
        lists: SpellLists {
            spells: table,
            classes: list_classes,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{Provenance, SpellFields, SpellLevel};

    fn spell(id: &str, title: &str, levels: &[(&str, u8)]) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Spell,
            title: title.into(),
            source_ids: vec!["cr".into()],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::Spell(SpellFields {
                school: "evocation".into(),
                levels: levels
                    .iter()
                    .map(|(class, level)| SpellLevel {
                        class: (*class).into(),
                        level: *level,
                    })
                    .collect(),
                ..SpellFields::default()
            }),
            tags: vec![],
            links: vec![],
            provenance: vec![Provenance {
                dataset: "d".into(),
                record: "r".into(),
                path: "p".into(),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: vec![],
        }
    }

    fn class(id: &str, title: &str) -> RuleEntry {
        let mut entry = spell(id, title, &[]);
        entry.kind = EntryKind::Class;
        entry.fields = TypedFields::Class(Default::default());
        entry
    }

    #[test]
    fn groups_by_class_and_level_and_keeps_unmapped_classes_visible() {
        // Real shapes: Fireball at 3 for Sorcerer and Wizard, Hunter repeated upstream.
        let entries = vec![
            class("cr/classes/wizard", "Wizard"),
            spell(
                "cr/spells/fireball",
                "Fireball",
                &[("Hunter", 3), ("Hunter", 3), ("Wizard", 3), ("wizard", 3)],
            ),
            spell(
                "cr/spells/acid-arrow",
                "Acid Arrow",
                &[("Wizard", 2), ("Hunter", 2)],
            ),
            spell("cr/spells/alarm", "Alarm", &[("Wizard", 1)]),
        ];
        let report = build(&entries);
        let wizard = report
            .lists
            .classes
            .iter()
            .find(|c| c.name == "Wizard")
            .unwrap();
        assert_eq!(wizard.class.as_ref().unwrap().as_str(), "cr/classes/wizard");
        let levels: Vec<u8> = wizard.levels.iter().map(|l| l.level).collect();
        assert_eq!(levels, [1, 2, 3]);
        let level_three: Vec<&str> = wizard.levels[2]
            .spells
            .iter()
            .map(|&p| report.lists.spells[p as usize].1.as_str())
            .collect();
        assert_eq!(level_three, ["Fireball"], "case variants and repeats merge");
        assert_eq!(report.coverage.pairs_upstream, 7);
        assert_eq!(report.coverage.pairs_distinct, 5);
        assert_eq!(report.coverage.unmapped.len(), 1);
        assert_eq!(report.coverage.unmapped[0].name, "Hunter");
        assert_eq!(report.coverage.unmapped[0].spells, 2);
        assert_eq!(report.coverage.classes_with_entry, ["Wizard"]);
    }

    #[test]
    fn spells_in_a_level_are_sorted_by_title() {
        let entries = vec![
            spell("a/z", "Zone of Truth", &[("Cleric", 2)]),
            spell("a/a", "Aid", &[("Cleric", 2)]),
        ];
        let report = build(&entries);
        let titles: Vec<&str> = report.lists.classes[0].levels[0]
            .spells
            .iter()
            .map(|&p| report.lists.spells[p as usize].1.as_str())
            .collect();
        assert_eq!(titles, ["Aid", "Zone of Truth"]);
    }
}
