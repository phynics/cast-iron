//! Library-wide validation. NFR-06: malformed or missing records fail visibly.

use crate::{
    Block, BookIndex, BookNode, Catalog, EntryId, EntryKind, Inline, RelationEdge, RelationIndex,
    RelationKind, RuleEntry, SpellLists, TableRow, TypedFields,
};
use std::collections::{BTreeMap, BTreeSet};

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
    check_part_cycles(entries, &mut errors);
    errors
}

/// Checks the derived files against the entries they were built from: the relation index
/// holds exactly the entries' relations with correct counts, every book node names an entry
/// that exists, no entry sits twice in one tree, and every spell list refers to a spell.
pub fn validate_derived(
    entries: &[RuleEntry],
    relations: &RelationIndex,
    books: &BookIndex,
    lists: &SpellLists,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let ids: BTreeSet<&EntryId> = entries.iter().map(|e| &e.id).collect();
    let kinds: BTreeMap<&EntryId, EntryKind> = entries.iter().map(|e| (&e.id, e.kind)).collect();
    let problem = |entry: &str, text: String| ValidationError {
        entry: entry.to_string(),
        problem: text,
    };

    // relations.json
    let expected: BTreeSet<RelationEdge> = entries
        .iter()
        .flat_map(|e| {
            e.relations
                .iter()
                .map(|r| RelationEdge(e.id.clone(), r.kind, r.target.clone(), r.label.clone()))
        })
        .collect();
    let actual: BTreeSet<RelationEdge> = relations.edges.iter().cloned().collect();
    if actual.len() != relations.edges.len() {
        errors.push(problem("<relations>", "duplicate edges".into()));
    }
    if relations.edges.windows(2).any(|w| w[0] > w[1]) {
        errors.push(problem("<relations>", "edges are not sorted".into()));
    }
    if expected != actual {
        errors.push(problem(
            "<relations>",
            format!(
                "index holds {} edges but the entries hold {}",
                actual.len(),
                expected.len()
            ),
        ));
    }
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for RelationEdge(_, kind, _, _) in &relations.edges {
        *counts.entry(kind.slug().to_string()).or_default() += 1;
    }
    if counts != relations.counts {
        errors.push(problem(
            "<relations>",
            format!(
                "counts {:?} do not match edges {counts:?}",
                relations.counts
            ),
        ));
    }

    // books.json
    let mut sources = BTreeSet::new();
    for tree in &books.books {
        if !sources.insert(tree.source.as_str()) {
            errors.push(problem(
                &tree.source,
                "two trees for one publication".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        for chapter in &tree.chapters {
            check_node(chapter, &tree.source, &ids, &mut seen, &mut errors);
        }
    }

    // lists.json
    let positions = lists.spells.len();
    if lists.spells.windows(2).any(|w| w[0].0 >= w[1].0) {
        errors.push(problem("<lists>", "spells are not sorted by id".into()));
    }
    if !lists.details.is_empty() && lists.details.len() != positions {
        errors.push(problem(
            "<lists>",
            "details do not match the spell table".into(),
        ));
    }
    for spell in &lists.spells {
        if kinds.get(&spell.0) != Some(&EntryKind::Spell) {
            errors.push(problem(spell.0.as_str(), "list names a non-spell".into()));
        }
    }
    for class in &lists.classes {
        if let Some(entry) = &class.class
            && !matches!(
                kinds.get(entry),
                Some(EntryKind::Class | EntryKind::PrestigeClass)
            )
        {
            errors.push(problem(
                entry.as_str(),
                format!(
                    "list `{}` names a class entry that is not a class",
                    class.name
                ),
            ));
        }
        if class.levels.windows(2).any(|w| w[0].level >= w[1].level) {
            errors.push(problem(&class.name, "levels are not ascending".into()));
        }
        for level in &class.levels {
            if level.spells.iter().any(|&p| p as usize >= positions) {
                errors.push(problem(&class.name, "spell position out of range".into()));
            }
        }
    }
    errors
}

fn check_node(
    node: &BookNode,
    source: &str,
    ids: &BTreeSet<&EntryId>,
    seen: &mut BTreeSet<EntryId>,
    errors: &mut Vec<ValidationError>,
) {
    if node.title.trim().is_empty() {
        errors.push(ValidationError {
            entry: source.to_string(),
            problem: "book node has an empty title".into(),
        });
    }
    match &node.entry {
        Some(entry) => {
            if !ids.contains(entry) {
                errors.push(ValidationError {
                    entry: entry.to_string(),
                    problem: format!("book `{source}` names an unknown entry"),
                });
            } else if node.anchor.is_none() && !seen.insert(entry.clone()) {
                errors.push(ValidationError {
                    entry: entry.to_string(),
                    problem: format!("book `{source}` places the entry twice"),
                });
            }
        }
        None if node.anchor.is_some() && node.children.is_empty() => {
            errors.push(ValidationError {
                entry: source.to_string(),
                problem: format!("heading node `{}` holds nothing", node.title),
            });
        }
        None => {}
    }
    for child in &node.children {
        check_node(child, source, ids, seen, errors);
    }
}

fn error(id: &EntryId, problem: impl Into<String>) -> ValidationError {
    ValidationError {
        entry: id.to_string(),
        problem: problem.into(),
    }
}

/// `Part` must form a forest: following parents never returns to the start.
fn check_part_cycles(entries: &[RuleEntry], errors: &mut Vec<ValidationError>) {
    let mut parents: BTreeMap<&EntryId, Vec<&EntryId>> = BTreeMap::new();
    for entry in entries {
        for relation in &entry.relations {
            if relation.kind == RelationKind::Part {
                parents.entry(&entry.id).or_default().push(&relation.target);
            }
        }
    }
    fn visit<'a>(
        node: &'a EntryId,
        parents: &BTreeMap<&'a EntryId, Vec<&'a EntryId>>,
        state: &mut BTreeMap<&'a EntryId, bool>,
        errors: &mut Vec<ValidationError>,
    ) {
        // `true` while the node is on the current path, `false` once it is finished.
        match state.get(node) {
            Some(true) => {
                errors.push(error(node, "part relations form a cycle"));
                return;
            }
            Some(false) => return,
            None => {}
        }
        state.insert(node, true);
        for &target in parents.get(node).map(Vec::as_slice).unwrap_or_default() {
            visit(target, parents, state, errors);
        }
        state.insert(node, false);
    }
    let mut state = BTreeMap::new();
    for &start in parents.keys() {
        visit(start, &parents, &mut state, errors);
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
    // Provenance records where an imported entry came from. A hand-written entry has none,
    // so an empty list is valid.
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
            | (TypedFields::Trait(_), EntryKind::Trait)
            | (TypedFields::RacialTrait(_), EntryKind::RacialTrait)
            | (TypedFields::Archetype(_), EntryKind::Archetype)
            | (TypedFields::ClassOption(_), EntryKind::ClassOption)
            | (
                TypedFields::None,
                EntryKind::Rule
                    | EntryKind::Race
                    | EntryKind::Skill
                    | EntryKind::Affliction
                    | EntryKind::Trap
                    | EntryKind::Archetype
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
    let mut seen_relations = BTreeSet::new();
    for relation in &entry.relations {
        if !ids.contains(&relation.target) {
            errors.push(error(
                id,
                format!(
                    "{} relation to unknown entry {}",
                    relation.kind.slug(),
                    relation.target
                ),
            ));
        }
        if relation.target == *id {
            errors.push(error(
                id,
                format!("{} relation to itself", relation.kind.slug()),
            ));
        }
        if !seen_relations.insert((relation.kind, &relation.target)) {
            errors.push(error(
                id,
                format!(
                    "duplicate {} relation to {}",
                    relation.kind.slug(),
                    relation.target
                ),
            ));
        }
    }
    if let TypedFields::Feat(fields) = &entry.fields {
        for feat in &fields.prerequisite_feats {
            if !ids.contains(feat) {
                errors.push(error(id, format!("prerequisite feat {feat} is unknown")));
            }
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
            sources: vec![],
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
            relations: vec![],
        }
    }

    fn catalog(entries: &[RuleEntry]) -> Catalog {
        Catalog {
            sources: vec![SourceInfo {
                id: "cr".into(),
                title: "CR".into(),
                entry_count: entries.len() as u32,
                copyright: None,
                category: None,
                publication_date: None,
                series_kind: None,
                series: None,
                series_number: None,
                season: None,
                third_party: false,
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
    fn an_entry_without_provenance_is_valid() {
        let mut written = entry("a");
        written.provenance.clear();
        let entries = vec![written];
        assert_eq!(validate_library(&entries, &catalog(&entries)), vec![]);
    }

    #[test]
    fn broken_records_are_reported() {
        let mut dangling = entry("a");
        dangling.title = " ".into();
        dangling.source_ids = vec!["nope".into()];
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

    use crate::{BookTree, Relation, SpellListClass, SpellListLevel, SpellListSpell};

    fn related(id: &str, kind: RelationKind, target: &str) -> RuleEntry {
        let mut e = entry(id);
        e.relations = vec![Relation {
            kind,
            target: EntryId::parse(target).unwrap(),
            label: None,
        }];
        e
    }

    fn problems(entries: &[RuleEntry]) -> Vec<String> {
        validate_library(entries, &catalog(entries))
            .into_iter()
            .map(|e| e.problem)
            .collect()
    }

    #[test]
    fn a_dangling_self_and_duplicate_relation_are_reported() {
        let mut duplicate = related("b", RelationKind::Prerequisite, "a");
        duplicate.relations.push(duplicate.relations[0].clone());
        let entries = vec![
            related("a", RelationKind::Part, "missing"),
            duplicate,
            related("c", RelationKind::Archetype, "c"),
        ];
        let found = problems(&entries);
        for expected in [
            "part relation to unknown entry missing",
            "duplicate prerequisite relation to a",
            "archetype relation to itself",
        ] {
            assert!(
                found.iter().any(|p| p == expected),
                "{expected} missing from {found:?}"
            );
        }
    }

    #[test]
    fn part_relations_must_not_form_a_cycle() {
        let cyclic = vec![
            related("a", RelationKind::Part, "b"),
            related("b", RelationKind::Part, "c"),
            related("c", RelationKind::Part, "a"),
        ];
        assert!(
            problems(&cyclic)
                .iter()
                .any(|p| p == "part relations form a cycle")
        );
        // A prerequisite chain may loop back without being a part cycle, and a tree is fine.
        let tree = vec![
            related("a", RelationKind::Part, "b"),
            related("c", RelationKind::Part, "b"),
            entry("b"),
        ];
        assert_eq!(problems(&tree), Vec::<String>::new());
    }

    #[test]
    fn derived_files_must_match_the_entries() {
        let entries = vec![related("a", RelationKind::Part, "b"), entry("b")];
        let good_edges = RelationIndex {
            counts: [("part".to_string(), 1)].into(),
            edges: vec![RelationEdge::plain(
                EntryId::parse("a").unwrap(),
                RelationKind::Part,
                EntryId::parse("b").unwrap(),
            )],
        };
        let node = |id: &str| BookNode {
            title: id.into(),
            entry: Some(EntryId::parse(id).unwrap()),
            anchor: None,
            review: None,
            children: vec![],
        };
        let books = |chapters: Vec<BookNode>| BookIndex {
            books: vec![BookTree {
                source: "cr".into(),
                title: "CR".into(),
                basis: String::new(),
                review: Vec::new(),
                chapters,
            }],
        };
        let lists = SpellLists::default();
        assert_eq!(
            validate_derived(
                &entries,
                &good_edges,
                &books(vec![node("a"), node("b")]),
                &lists
            ),
            vec![]
        );

        let stale = RelationIndex::default();
        let found: Vec<String> = validate_derived(&entries, &stale, &books(vec![]), &lists)
            .into_iter()
            .map(|e| e.problem)
            .collect();
        assert!(
            found
                .iter()
                .any(|p| p.contains("index holds 0 edges but the entries hold 1"))
        );

        let twice = books(vec![node("a"), node("a"), node("ghost")]);
        let found: Vec<String> = validate_derived(&entries, &good_edges, &twice, &lists)
            .into_iter()
            .map(|e| e.problem)
            .collect();
        assert!(
            found.iter().any(|p| p.contains("places the entry twice")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|p| p.contains("names an unknown entry")),
            "{found:?}"
        );

        let wrong_counts = RelationIndex {
            counts: [("part".to_string(), 2)].into(),
            ..good_edges.clone()
        };
        let found = validate_derived(&entries, &wrong_counts, &books(vec![]), &lists);
        assert!(found.iter().any(|e| e.problem.contains("counts")));

        let bad_lists = SpellLists {
            spells: vec![SpellListSpell(EntryId::parse("a").unwrap(), "A".into())],
            classes: vec![SpellListClass {
                name: "Wizard".into(),
                class: None,
                levels: vec![SpellListLevel {
                    level: 1,
                    spells: vec![5],
                }],
            }],
            details: vec![],
        };
        let found = validate_derived(&entries, &good_edges, &books(vec![]), &bad_lists);
        assert!(found.iter().any(|e| e.problem == "list names a non-spell"));
        assert!(
            found
                .iter()
                .any(|e| e.problem == "spell position out of range")
        );

        let mut uneven = bad_lists.clone();
        uneven.details = vec![crate::SpellListDetail::default(); 2];
        let found = validate_derived(&entries, &good_edges, &books(vec![]), &uneven);
        assert!(
            found
                .iter()
                .any(|e| e.problem == "details do not match the spell table")
        );
    }
}
