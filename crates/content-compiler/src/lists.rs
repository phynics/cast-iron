//! Spell lists per class and level, indexed from `SpellFields::levels`.
//!
//! The spell entries stay the source of truth. This pass only groups them so that "Wizard
//! level 3 spells" does not need every spell shard. Class names are grouped without regard
//! to case; the display name is the class entry's title when a class entry exists.

use crate::pipeline::school_facet;
use crate::text::inline_text;
use cast_iron_schema::{
    Block, EntryId, EntryKind, RuleEntry, SpellFields, SpellListClass, SpellListCoverage,
    SpellListDetail, SpellListLevel, SpellListSpell, SpellLists, TypedFields, UnmappedClass,
};
use std::collections::{BTreeMap, BTreeSet};

/// Longest gist, in characters. A row shows it on one line and cuts it with an ellipsis, so
/// the file does not need more than a phone line can use.
const GIST_CHARS: usize = 70;

/// The first sentence of the description, for the second line of a list row. The statistics
/// block, the `Sources:` line and headings are skipped; the first plain paragraph counts.
/// Empty when the entry has no such paragraph.
pub fn gist(body: &[Block]) -> String {
    let paragraph = body.iter().find_map(|block| match block {
        Block::Paragraph { content, .. } => {
            let text = inline_text(content);
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            // Only a citation line is skipped. A sentence that begins with the word "Source"
            // is description.
            let skip =
                text.is_empty() || text.starts_with("Sources:") || text.starts_with("Source:");
            (!skip).then_some(text)
        }
        _ => None,
    });
    let Some(text) = paragraph else {
        return String::new();
    };
    first_sentence_within(&text, GIST_CHARS)
}

/// Words that end in a period without ending a sentence. Compared in lowercase, without the
/// period and without a leading parenthesis.
const ABBREVIATIONS: [&str; 11] = [
    "max", "min", "ft", "lb", "lbs", "etc", "e.g", "i.e", "rd", "cf", "vs",
];

/// True when the period at `chars[i]` closes an abbreviation such as `max.` or `e.g.`.
fn ends_abbreviation(chars: &[char], i: usize) -> bool {
    let start = chars[..i]
        .iter()
        .rposition(|c| c.is_whitespace())
        .map_or(0, |p| p + 1);
    let word: String = chars[start..i]
        .iter()
        .collect::<String>()
        .trim_start_matches(['(', '[', '"'])
        .to_lowercase();
    ABBREVIATIONS.contains(&word.as_str())
}

/// The first sentence of `text`. A sentence ends at `.`, `!` or `?` followed by a space (or the
/// end of the text), but not at an abbreviation (`max.`, `ft.`, `e.g.`) and not inside an open
/// parenthesis or bracket. Longer than `limit` characters, it is cut at the last word boundary
/// that sits outside an open parenthesis. Any cut ends with `…`; a complete sentence loses only
/// its final period.
fn first_sentence_within(text: &str, limit: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut depth = 0i32;
    // The index just past the sentence end, if there is one.
    let mut end = None;
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            '.' | '!' | '?'
                if depth == 0
                    && chars.get(i + 1).is_none_or(|n| *n == ' ')
                    && !(c == '.' && ends_abbreviation(&chars, i)) =>
            {
                end = Some(i + 1);
                break;
            }
            _ => {}
        }
    }
    let end = end.unwrap_or(chars.len());
    if end <= limit {
        let sentence: String = chars[..end].iter().collect();
        return sentence.trim_end_matches('.').trim().to_string();
    }
    // Cut at a word boundary within the limit, then back out of any open parenthesis.
    let mut cut = limit;
    while cut > 0 && chars.get(cut).is_some_and(|c| *c != ' ') {
        cut -= 1;
    }
    let mut open: Vec<usize> = Vec::new();
    for (i, &c) in chars[..cut].iter().enumerate() {
        match c {
            '(' | '[' => open.push(i),
            ')' | ']' => {
                open.pop();
            }
            _ => {}
        }
    }
    if let Some(&first_open) = open.first() {
        cut = first_open;
    }
    let head: String = chars[..cut].iter().collect();
    let head = head.trim_end_matches([',', ';', ':', ' ', '.', '-', '+']);
    if head.is_empty() {
        // One long word: keep what fits.
        let head: String = chars[..limit.min(chars.len())].iter().collect();
        return format!("{head}…");
    }
    format!("{head}…")
}

fn detail(entry: &RuleEntry, fields: &SpellFields) -> SpellListDetail {
    let clean = |value: &Option<String>| {
        value
            .as_deref()
            .map(|v| v.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
    };
    SpellListDetail {
        school: school_facet(&fields.school),
        casting_time: clean(&fields.casting_time),
        saving_throw: clean(&fields.saving_throw),
        gist: gist(&entry.body),
    }
}
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
    let mut spell_details: BTreeMap<&EntryId, SpellListDetail> = BTreeMap::new();
    let mut pairs_upstream = 0u32;
    let mut spells = 0u32;
    for entry in entries.iter().filter(|e| e.kind == EntryKind::Spell) {
        let TypedFields::Spell(fields) = &entry.fields else {
            continue;
        };
        spells += 1;
        spell_titles.insert(&entry.id, &entry.title);
        spell_details.insert(&entry.id, detail(entry, fields));
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
    let details: Vec<SpellListDetail> = used.iter().map(|id| spell_details[id].clone()).collect();
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
            details,
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
            sources: vec![],
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
            race_group: None,
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

    fn text(v: &str) -> Vec<cast_iron_schema::Inline> {
        vec![cast_iron_schema::Inline::Text { v: v.into() }]
    }

    fn paragraph(v: &str) -> Block {
        Block::Paragraph {
            style: Default::default(),
            content: text(v),
        }
    }

    #[test]
    fn the_gist_is_the_first_sentence_after_the_statistics_and_sources() {
        let body = vec![
            Block::StatBlock {
                title: None,
                rows: vec![],
            },
            paragraph("Sources: PRPG Core Rulebook pg 258"),
            paragraph("Fire  leaps\nfrom your hand. It burns. More text."),
        ];
        assert_eq!(gist(&body), "Fire leaps from your hand");
        assert_eq!(gist(&[paragraph("Sources: x")]), "");
        assert_eq!(gist(&[]), "");
    }

    #[test]
    fn a_long_sentence_is_cut_at_a_word_and_marked() {
        let long = "word ".repeat(40);
        let gist = first_sentence_within(long.trim(), 20);
        assert_eq!(gist, "word word word word…");
        assert!(gist.chars().count() <= 21);
        assert_eq!(first_sentence_within("No stop here", 100), "No stop here");
        // A cut that lands exactly on a word boundary keeps that word.
        assert_eq!(
            first_sentence_within("alpha beta gamma delta", 10),
            "alpha beta…"
        );
    }

    #[test]
    fn abbreviations_do_not_end_a_sentence() {
        let gist = |t: &str| first_sentence_within(t, 100);
        assert_eq!(
            gist("The target takes 1d6 damage (max. 5d6) per level. More."),
            "The target takes 1d6 damage (max. 5d6) per level"
        );
        assert_eq!(
            gist("Range is 30 ft. longer than normal. More."),
            "Range is 30 ft. longer than normal"
        );
        assert_eq!(
            gist("It weighs 5 lbs. at most. Next."),
            "It weighs 5 lbs. at most"
        );
        assert_eq!(
            gist("Fire, cold, etc. also work. Next."),
            "Fire, cold, etc. also work"
        );
        assert_eq!(
            gist("Use it, e.g. on a rope. Next."),
            "Use it, e.g. on a rope"
        );
        assert_eq!(
            gist("That is, i.e. a thing. Next."),
            "That is, i.e. a thing"
        );
        assert_eq!(gist("Wait 1 min. then go. Next."), "Wait 1 min. then go");
        assert_eq!(
            gist("Lasts 1 rd. per level. Next."),
            "Lasts 1 rd. per level"
        );
        assert_eq!(gist("See cf. the table. Next."), "See cf. the table");
        // `!` and `?` still end a sentence; a real period too.
        assert_eq!(gist("Run! Hide."), "Run!");
        assert_eq!(gist("Is it done? Yes."), "Is it done?");
        assert_eq!(gist("Done. Next."), "Done");
    }

    #[test]
    fn the_greater_magic_fang_gist_is_not_cut_inside_its_parenthesis() {
        let text = "One natural weapon gets +1/four levels (max. +5). The weapon also gets more.";
        assert_eq!(
            first_sentence_within(text, 100),
            "One natural weapon gets +1/four levels (max. +5)"
        );
    }

    #[test]
    fn a_cut_never_ends_inside_an_open_parenthesis() {
        let text = "The weapon gets a bonus on every attack roll and damage roll (max. +5 and more to come later";
        let gist = first_sentence_within(text, 70);
        assert_eq!(
            gist,
            "The weapon gets a bonus on every attack roll and damage roll…"
        );
        assert!(!gist.contains('('));
        // A parenthesis that never closes does not end the sentence early.
        assert_eq!(
            first_sentence_within("Opens (see text. Still open. Never closes", 100),
            "Opens (see text. Still open. Never closes"
        );
    }

    #[test]
    fn only_a_citation_line_is_skipped() {
        let body = vec![
            paragraph("Source energy flows from the caster. More."),
            paragraph("Sources: ignored"),
        ];
        assert_eq!(gist(&body), "Source energy flows from the caster");
        let cited = vec![paragraph("Source: Core Rulebook"), paragraph("Real text.")];
        assert_eq!(gist(&cited), "Real text");
    }

    #[test]
    fn rows_carry_school_casting_time_save_and_gist_in_table_order() {
        let mut entry = spell("a/b", "Bolt", &[("Wizard", 1)]);
        if let TypedFields::Spell(fields) = &mut entry.fields {
            fields.school = "evocation [fire]".into();
            fields.casting_time = Some("1 standard  action".into());
            fields.saving_throw = Some("Reflex half".into());
        }
        entry.body = vec![paragraph("A bolt of fire.")];
        let bare = spell("a/a", "Alarm", &[("Wizard", 1)]);
        let report = build(&[entry, bare]);
        assert_eq!(report.lists.spells.len(), report.lists.details.len());
        assert_eq!(report.lists.spells[0].1, "Alarm");
        assert_eq!(report.lists.details[0].casting_time, "");
        assert_eq!(report.lists.details[0].gist, "");
        let bolt = &report.lists.details[1];
        assert_eq!(bolt.school, "evocation");
        assert_eq!(bolt.casting_time, "1 standard action");
        assert_eq!(bolt.saving_throw, "Reflex half");
        assert_eq!(bolt.gist, "A bolt of fire");
    }
}
