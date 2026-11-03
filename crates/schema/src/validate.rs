//! Library-wide validation. NFR-06: malformed or missing records fail visibly.

use crate::{Block, Catalog, EntryId, EntryKind, Inline, RuleEntry, TableRow, TypedFields};
use std::collections::BTreeSet;

/// One validation failure, with the id of the record that caused it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    pub entry: String,
    pub problem: String,
}

/// Validates a complete library. An empty result means the library is valid.
pub fn validate_library(entries: &[RuleEntry], catalog: &Catalog) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    for entry in entries {
        if !ids.insert(&entry.id) {
            errors.push(error(&entry.id, "duplicate id"));
        }
    }
    let sources: BTreeSet<&str> = catalog.sources.iter().map(|s| s.id.as_str()).collect();
    for entry in entries {
        validate_entry(entry, &ids, &sources, &mut errors);
    }
    if catalog.entries.len() != entries.len() {
        errors.push(ValidationError {
            entry: "<catalog>".into(),
            problem: format!(
                "catalog has {} stubs for {} entries",
                catalog.entries.len(),
                entries.len()
            ),
        });
    }
    errors
}

fn error(id: &EntryId, problem: impl Into<String>) -> ValidationError {
    ValidationError {
        entry: id.to_string(),
        problem: problem.into(),
    }
}

fn validate_entry(
    entry: &RuleEntry,
    ids: &BTreeSet<&EntryId>,
    sources: &BTreeSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    let id = &entry.id;
    if entry.title.trim().is_empty() {
        errors.push(error(id, "empty title"));
    }
    if entry.source_ids.is_empty() {
        errors.push(error(id, "no source"));
    }
    for source in &entry.source_ids {
        if !sources.contains(source.as_str()) {
            errors.push(error(id, format!("unknown source {source}")));
        }
    }
    if entry.provenance.is_empty() {
        errors.push(error(id, "no provenance"));
    }
    if entry.license_refs.is_empty() {
        errors.push(error(id, "no license reference"));
    }
    if let Some(parent) = &entry.parent
        && !ids.contains(parent)
    {
        errors.push(error(id, format!("unknown parent {parent}")));
    }
    let fields_match = matches!(
        (&entry.fields, entry.kind),
        (TypedFields::Spell(_), EntryKind::Spell)
            | (TypedFields::Feat(_), EntryKind::Feat)
            | (TypedFields::Creature(_), EntryKind::Creature)
            | (TypedFields::Item(_), EntryKind::Item)
            | (
                TypedFields::Class(_),
                EntryKind::Class | EntryKind::PrestigeClass
            )
            | (
                TypedFields::None,
                EntryKind::Rule
                    | EntryKind::Race
                    | EntryKind::Skill
                    | EntryKind::Affliction
                    | EntryKind::Trap
            )
    );
    if !fields_match {
        errors.push(error(id, "typed fields do not match the entry kind"));
    }
    let mut seen_targets = BTreeSet::new();
    visit_blocks(
        &entry.body,
        &mut |target| {
            seen_targets.insert(target.clone());
        },
        &mut |level| {
            if !(2..=6).contains(&level) {
                errors.push(error(
                    id,
                    format!("heading level {level} is outside 2 to 6"),
                ));
            }
        },
    );
    for target in &seen_targets {
        if !ids.contains(target) {
            errors.push(error(id, format!("link to unknown entry {target}")));
        }
    }
    for reference in &entry.links {
        if !ids.contains(&reference.target) {
            errors.push(error(
                id,
                format!("reference to unknown entry {}", reference.target),
            ));
        }
    }
}

fn visit_blocks(
    blocks: &[Block],
    on_link: &mut impl FnMut(&EntryId),
    on_heading: &mut impl FnMut(u8),
) {
    for block in blocks {
        match block {
            Block::Heading { level, content, .. } => {
                on_heading(*level);
                visit_inlines(content, on_link);
            }
            Block::Paragraph { content, .. } => visit_inlines(content, on_link),
            Block::List { items, .. } => {
                for item in items {
                    visit_blocks(item, on_link, on_heading);
                }
            }
            Block::Table(table) => {
                visit_inlines(&table.caption, on_link);
                for rows in [&table.head, &table.body, &table.foot] {
                    visit_rows(rows, on_link);
                }
            }
            Block::Quote { content } => visit_blocks(content, on_link, on_heading),
            Block::StatBlock { rows, .. } => {
                for row in rows {
                    visit_inlines(&row.value, on_link);
                }
            }
            Block::EntryLink { target, .. } => on_link(target),
        }
    }
}

fn visit_rows(rows: &[TableRow], on_link: &mut impl FnMut(&EntryId)) {
    for row in rows {
        for cell in &row.cells {
            visit_inlines(&cell.content, on_link);
        }
    }
}

fn visit_inlines(inlines: &[Inline], on_link: &mut impl FnMut(&EntryId)) {
    for inline in inlines {
        match inline {
            Inline::Text { .. } | Inline::Break => {}
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                visit_inlines(c, on_link);
            }
            Inline::Link { target, c, .. } => {
                on_link(target);
                visit_inlines(c, on_link);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryStub, Provenance, SourceInfo};

    fn entry(id: &str) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Rule,
            title: "Title".into(),
            source_ids: vec!["cr".into()],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::None,
            tags: vec![],
            links: vec![],
            provenance: vec![Provenance {
                dataset: "d".into(),
                record: "r".into(),
                path: "p".into(),
            }],
            license_refs: vec!["ogl-1.0a".into()],
        }
    }

    fn catalog(entries: &[RuleEntry]) -> Catalog {
        Catalog {
            sources: vec![SourceInfo {
                id: "cr".into(),
                title: "CR".into(),
                entry_count: entries.len() as u32,
                copyright: None,
            }],
            entries: entries
                .iter()
                .map(|e| EntryStub {
                    id: e.id.clone(),
                    kind: e.kind,
                    title: e.title.clone(),
                    source: "cr".into(),
                    parent: None,
                    shard: "entries/rule-000.json".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn valid_library_has_no_errors() {
        let entries = vec![entry("a"), entry("b")];
        assert_eq!(validate_library(&entries, &catalog(&entries)), vec![]);
    }

    #[test]
    fn broken_records_are_reported() {
        let mut dangling = entry("a");
        dangling.title = " ".into();
        dangling.source_ids = vec!["nope".into()];
        dangling.provenance.clear();
        dangling.license_refs.clear();
        dangling.fields = TypedFields::Feat(Default::default());
        dangling.body = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![Inline::Link {
                target: EntryId::parse("missing").unwrap(),
                anchor: None,
                c: vec![],
            }],
        }];
        let duplicate = dangling.clone();
        let entries = vec![dangling, duplicate];
        let problems: Vec<String> = validate_library(&entries, &catalog(&entries))
            .into_iter()
            .map(|e| e.problem)
            .collect();
        for expected in [
            "duplicate id",
            "empty title",
            "unknown source nope",
            "no provenance",
            "no license reference",
            "typed fields do not match the entry kind",
            "link to unknown entry missing",
        ] {
            assert!(
                problems.iter().any(|p| p == expected),
                "{expected} missing from {problems:?}"
            );
        }
    }
}
