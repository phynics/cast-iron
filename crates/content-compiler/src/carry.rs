//! Carries over legacy PSRD entries that the pinned Pf Data corpus does not replace.
//!
//! Section 4.5 of `docs/research/canonical-id-design.md` reserves
//! `ref/carried/<legacy-id>` for an already-typed PSRD entry that no canonical Pf
//! Data record replaces. The entry keeps its body, fields, source ids, license
//! refs, and PSRD provenance; only identifiers change. Every target it names is
//! rewritten through the reviewed alias map:
//!
//! * a legacy target with a canonical alias becomes its canonical target;
//! * a legacy target that is itself carried becomes its carried target;
//! * anything else is flattened to plain content and reported.
//!
//! The carried set is explicit in `sources/canonical/id-map.toml`: every `[map]`
//! row whose target is `ref/carried/<old>` names one carried record. The alias
//! gate adds exactly those records to its catalog, so a carried target can never
//! pass without a real carried entry. Collisions are build errors. Output is
//! sorted by legacy id and deterministic.

use crate::alias::{CanonicalCatalog, CanonicalRecord, IdMap, normalize_title};
use crate::pipeline::Library;
use cast_iron_schema::{
    Block, EntryId, EntryReference, Inline, ParagraphStyle, RuleEntry, StatRow, Table, TableCell,
    TableRow, TypedFields,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Reserved prefix for a carried legacy identifier.
pub const CARRIED_PREFIX: &str = "ref/carried/";

/// One carried legacy entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Carried {
    /// The legacy identifier that is carried.
    pub legacy: String,
    /// The canonical entry, with `ref/carried/<legacy>` as its id.
    pub entry: RuleEntry,
}

/// A canonical record whose kind and normalized title match a carried legacy
/// entry in any book. This is a review lead, never an automatic match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PossibleDuplicate {
    pub legacy: String,
    pub carried: String,
    pub candidates: Vec<String>,
}

/// Rewritten and flattened targets, by the kind of reference they occur in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CarryLinkCounts {
    pub body_rewritten: usize,
    pub body_flattened: usize,
    pub parent_rewritten: usize,
    pub parent_flattened: usize,
    pub relation_rewritten: usize,
    pub relation_flattened: usize,
    pub field_rewritten: usize,
    pub field_flattened: usize,
    pub rewritten: usize,
    pub flattened: usize,
}

/// Counts and review leads for one carry-over run, written under `carry_over` in
/// `reports/migration-coverage.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CarryReport {
    pub carried: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub by_book: BTreeMap<String, usize>,
    pub possible_duplicate_count: usize,
    pub possible_duplicates: Vec<PossibleDuplicate>,
    pub links: CarryLinkCounts,
}

/// The result of a carry-over run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarryOver {
    /// Carried entries sorted by legacy id.
    pub records: Vec<Carried>,
    /// Legacy ids carried because their planned canonical target was not emitted,
    /// keyed by legacy id and pointing at `ref/carried/<legacy>`.
    pub fallbacks: BTreeMap<String, String>,
    pub report: CarryReport,
}

impl CarryOver {
    /// Adds one canonical catalog record per carried entry. The catalog is how the
    /// alias gate knows a `ref/carried/...` target exists; it never gains a record
    /// that this run did not produce.
    pub fn extend_catalog(&self, catalog: &mut CanonicalCatalog) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for carried in &self.records {
            let record = CanonicalRecord {
                id: carried.entry.id.as_str().to_string(),
                identity: format!("carried:{}", carried.legacy),
                kind: carried.entry.kind.slug().to_string(),
                title: carried.entry.title.clone(),
                source_ids: carried.entry.source_ids.clone(),
                origin: "carried".into(),
            };
            if let Err(error) = catalog.insert(record) {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Carries every entry named by a `ref/carried/...` map row in the PSRD library.
pub fn carry_over(
    library: &Library,
    map: &IdMap,
    catalog: &CanonicalCatalog,
) -> Result<CarryOver, Vec<String>> {
    carry_entries(&library.entries, map, catalog)
}

/// Carries the explicit `ref/carried/...` rows plus every legacy id whose planned
/// canonical target exists in the catalog but was not emitted by the converter.
///
/// The fallback is the "no OGC lost" half of the cutover policy. A row whose
/// target is legally excluded is not carried here: that target has a rights
/// disposition and the caller renders it as a rights retirement. A legacy id
/// that the PSRD library does not publish cannot be carried and stays retired
/// with its own reason.
pub fn carry_over_with_fallback(
    library: &Library,
    map: &IdMap,
    catalog: &CanonicalCatalog,
    emitted: &BTreeSet<String>,
    legally_excluded: &BTreeSet<String>,
) -> Result<CarryOver, Vec<String>> {
    carry_entries_with_fallback(&library.entries, map, catalog, emitted, legally_excluded)
}

/// The entry-only form of [`carry_over`], for fixtures and tests.
pub fn carry_entries(
    entries: &[RuleEntry],
    map: &IdMap,
    catalog: &CanonicalCatalog,
) -> Result<CarryOver, Vec<String>> {
    carry_entries_with(entries, map, catalog, &BTreeMap::new())
}

/// The entry-only form of [`carry_over_with_fallback`], for fixtures and tests.
pub fn carry_entries_with_fallback(
    entries: &[RuleEntry],
    map: &IdMap,
    catalog: &CanonicalCatalog,
    emitted: &BTreeSet<String>,
    legally_excluded: &BTreeSet<String>,
) -> Result<CarryOver, Vec<String>> {
    let fallbacks = fallback_targets(entries, map, catalog, emitted, legally_excluded);
    carry_entries_with(entries, map, catalog, &fallbacks)
}

/// Resolves map rows whose canonical target is missing from the installed library.
///
/// The target must exist in the catalog, must not be legally excluded, and must
/// not already be a carried row. Only ids that the PSRD library publishes are
/// fallback candidates; carrying an unpublished id would invent a record.
fn fallback_targets(
    entries: &[RuleEntry],
    map: &IdMap,
    catalog: &CanonicalCatalog,
    emitted: &BTreeSet<String>,
    legally_excluded: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let published: BTreeSet<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
    let mut out = BTreeMap::new();
    let mut consider = |old: &str, target: Option<&str>| {
        let Some(target) = target else {
            return;
        };
        if target.starts_with(CARRIED_PREFIX)
            || emitted.contains(target)
            || legally_excluded.contains(target)
            || !catalog.contains(target)
            || !published.contains(old)
        {
            return;
        }
        if let Ok(carried) = carried_target(old) {
            out.insert(old.to_string(), carried);
        }
    };
    for (old, row) in &map.map {
        let target = row
            .target
            .as_deref()
            .or_else(|| {
                row.record
                    .as_deref()
                    .and_then(|id| catalog.canonical_for_identity(id))
            })
            .or_else(|| {
                row.derived
                    .as_deref()
                    .and_then(|id| catalog.canonical_for_identity(id))
            });
        consider(old, target);
    }
    for (old, row) in &map.shared {
        consider(old, Some(row.target.as_str()));
    }
    out
}

fn carry_entries_with(
    entries: &[RuleEntry],
    map: &IdMap,
    catalog: &CanonicalCatalog,
    fallbacks: &BTreeMap<String, String>,
) -> Result<CarryOver, Vec<String>> {
    let mut errors = Vec::new();
    let by_id: BTreeMap<&str, &RuleEntry> = entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();

    // The map is the explicit carried set. A carried target is deterministic:
    // it must be exactly `ref/carried/<legacy>`.
    let mut carried: BTreeMap<String, String> = BTreeMap::new();
    for (old, row) in &map.map {
        let Some(target) = &row.target else {
            continue;
        };
        if !target.starts_with(CARRIED_PREFIX) {
            continue;
        }
        let expected = carried_target(old);
        match expected {
            Ok(expected) if expected == *target => {
                carried.insert(old.to_string(), target.clone());
            }
            Ok(_) => errors.push(format!(
                "map row {old:?}: carried target {target:?} must be {expected:?}"
            )),
            Err(error) => errors.push(format!("map row {old:?}: {error}")),
        }
    }
    for (old, target) in fallbacks {
        let expected = carried_target(old);
        match expected {
            Ok(expected) if expected == *target => {
                if carried.insert(old.clone(), target.clone()).is_some() {
                    errors.push(format!(
                        "legacy id {old:?} is both an explicit and a fallback carry-over"
                    ));
                }
            }
            Ok(_) => errors.push(format!(
                "fallback target for {old:?} is {target:?}, must be {expected:?}"
            )),
            Err(error) => errors.push(format!("fallback legacy id {old:?}: {error}")),
        }
    }

    let resolver = match Resolver::new(map, catalog) {
        Ok(resolver) => resolver,
        Err(mut problems) => {
            errors.append(&mut problems);
            Resolver::empty()
        }
    };

    // Canonical records a carried entry might duplicate. The lookup ignores the
    // book on purpose: a title match in another book is worth review.
    let mut title_index: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for record in catalog.iter() {
        if record.origin == "carried" {
            continue;
        }
        title_index
            .entry((record.kind.clone(), normalize_title(&record.title)))
            .or_default()
            .insert(record.id.clone());
    }

    let mut records = Vec::new();
    let mut counts = CarryLinkCounts::default();
    let mut possible_duplicates = Vec::new();
    let mut by_kind = BTreeMap::new();
    let mut by_book = BTreeMap::new();

    for (old, target) in &carried {
        let Some(entry) = by_id.get(old.as_str()) else {
            errors.push(format!(
                "carried legacy id {old:?} is not published by the PSRD library"
            ));
            continue;
        };
        if catalog.contains(target) {
            errors.push(format!(
                "carried id {target:?} collides with an existing canonical record"
            ));
            continue;
        }
        let Ok(id) = EntryId::parse(target.clone()) else {
            errors.push(format!("carried target {target:?} is not a valid entry id"));
            continue;
        };

        let mut out = (*entry).clone();
        out.id = id;
        rewrite_parent(&mut out, &resolver, &mut counts);
        out.body = rewrite_blocks(std::mem::take(&mut out.body), &resolver, &mut counts);
        out.links = collect_links(&out.body);
        rewrite_relations(&mut out, &resolver, &mut counts);
        rewrite_fields(&mut out, &resolver, &mut counts);

        let key = (out.kind.slug().to_string(), normalize_title(&out.title));
        if let Some(candidates) = title_index.get(&key) {
            possible_duplicates.push(PossibleDuplicate {
                legacy: old.clone(),
                carried: target.clone(),
                candidates: candidates.iter().cloned().collect(),
            });
        }

        *by_kind.entry(out.kind.slug().to_string()).or_default() += 1;
        let book = out
            .source_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        *by_book.entry(book).or_default() += 1;

        records.push(Carried {
            legacy: old.clone(),
            entry: out,
        });
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    counts.rewritten = counts.body_rewritten
        + counts.parent_rewritten
        + counts.relation_rewritten
        + counts.field_rewritten;
    counts.flattened = counts.body_flattened
        + counts.parent_flattened
        + counts.relation_flattened
        + counts.field_flattened;
    let report = CarryReport {
        carried: records.len(),
        by_kind,
        by_book,
        possible_duplicate_count: possible_duplicates.len(),
        possible_duplicates,
        links: counts,
    };
    Ok(CarryOver {
        records,
        fallbacks: fallbacks.clone(),
        report,
    })
}

/// The canonical id of a carried legacy id.
pub fn carried_target(legacy: &str) -> Result<String, String> {
    let legacy_id = EntryId::parse(legacy.to_string())
        .map_err(|error| format!("legacy id is not valid: {error}"))?;
    if legacy_id.as_str().starts_with("ref/") {
        return Err(format!("{legacy:?} is already in the canonical namespace"));
    }
    let target = format!("{CARRIED_PREFIX}{legacy}");
    EntryId::parse(target.clone())
        .map(|_| target)
        .map_err(|error| error.to_string())
}

/// Resolves one legacy target through the reviewed map. A target already in the
/// canonical namespace passes through unchanged.
struct Resolver {
    mapping: BTreeMap<String, String>,
}

impl Resolver {
    fn empty() -> Resolver {
        Resolver {
            mapping: BTreeMap::new(),
        }
    }

    fn new(map: &IdMap, catalog: &CanonicalCatalog) -> Result<Resolver, Vec<String>> {
        let mut errors = Vec::new();
        let mut mapping = BTreeMap::new();
        for (old, row) in &map.map {
            let target = if let Some(target) = &row.target {
                Some(target.clone())
            } else if let Some(identity) = &row.record {
                match catalog.canonical_for_identity(identity) {
                    Some(target) => Some(target.to_string()),
                    None => {
                        errors.push(format!(
                            "map row {old:?}: record {identity:?} does not exist in the pinned corpus"
                        ));
                        continue;
                    }
                }
            } else if let Some(identity) = &row.derived {
                match catalog.canonical_for_identity(identity) {
                    Some(target) => Some(target.to_string()),
                    None => {
                        errors.push(format!(
                            "map row {old:?}: derived {identity:?} was not produced by the canonical derivation"
                        ));
                        continue;
                    }
                }
            } else {
                None
            };
            if let Some(target) = target {
                mapping.insert(old.clone(), target);
            }
        }
        for (old, row) in &map.shared {
            mapping.insert(old.clone(), row.target.clone());
        }
        if errors.is_empty() {
            Ok(Resolver { mapping })
        } else {
            Err(errors)
        }
    }

    fn resolve(&self, target: &EntryId) -> Option<EntryId> {
        if target.as_str().starts_with("ref/") {
            return Some(target.clone());
        }
        self.mapping
            .get(target.as_str())
            .and_then(|target| EntryId::parse(target.clone()).ok())
    }
}

fn rewrite_parent(entry: &mut RuleEntry, resolver: &Resolver, counts: &mut CarryLinkCounts) {
    let Some(parent) = entry.parent.clone() else {
        return;
    };
    match resolver.resolve(&parent) {
        Some(target) => {
            if target != parent {
                counts.parent_rewritten += 1;
            }
            entry.parent = Some(target);
        }
        None => {
            counts.parent_flattened += 1;
            entry.parent = None;
        }
    }
}

fn rewrite_relations(entry: &mut RuleEntry, resolver: &Resolver, counts: &mut CarryLinkCounts) {
    let mut relations = Vec::new();
    for mut relation in std::mem::take(&mut entry.relations) {
        match resolver.resolve(&relation.target) {
            Some(target) => {
                if target != relation.target {
                    counts.relation_rewritten += 1;
                }
                relation.target = target;
                relations.push(relation);
            }
            None => counts.relation_flattened += 1,
        }
    }
    relations.sort();
    relations.dedup();
    entry.relations = relations;
}

fn rewrite_fields(entry: &mut RuleEntry, resolver: &Resolver, counts: &mut CarryLinkCounts) {
    let TypedFields::Feat(fields) = &mut entry.fields else {
        return;
    };
    let mut feats = Vec::new();
    for feat in std::mem::take(&mut fields.prerequisite_feats) {
        match resolver.resolve(&feat) {
            Some(target) => {
                if target != feat {
                    counts.field_rewritten += 1;
                }
                feats.push(target);
            }
            None => counts.field_flattened += 1,
        }
    }
    feats.sort();
    feats.dedup();
    fields.prerequisite_feats = feats;
}

fn rewrite_blocks(
    blocks: Vec<Block>,
    resolver: &Resolver,
    counts: &mut CarryLinkCounts,
) -> Vec<Block> {
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            Block::Heading {
                level,
                anchor,
                content,
            } => out.push(Block::Heading {
                level,
                anchor,
                content: rewrite_inlines(content, resolver, counts),
            }),
            Block::Paragraph { style, content } => out.push(Block::Paragraph {
                style,
                content: rewrite_inlines(content, resolver, counts),
            }),
            Block::List { ordered, items } => out.push(Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| rewrite_blocks(item, resolver, counts))
                    .collect(),
            }),
            Block::Table(table) => out.push(Block::Table(rewrite_table(table, resolver, counts))),
            Block::Quote { content } => out.push(Block::Quote {
                content: rewrite_blocks(content, resolver, counts),
            }),
            Block::StatBlock { title, rows } => out.push(Block::StatBlock {
                title,
                rows: rows
                    .into_iter()
                    .map(|row| StatRow {
                        label: row.label,
                        value: rewrite_inlines(row.value, resolver, counts),
                    })
                    .collect(),
            }),
            Block::EntryLink { target, title } => match resolver.resolve(&target) {
                Some(new_target) => {
                    if new_target != target {
                        counts.body_rewritten += 1;
                    }
                    out.push(Block::EntryLink {
                        target: new_target,
                        title,
                    });
                }
                None => {
                    counts.body_flattened += 1;
                    out.push(Block::Paragraph {
                        style: ParagraphStyle::Normal,
                        content: vec![Inline::Text { v: title }],
                    });
                }
            },
        }
    }
    out
}

fn rewrite_inlines(
    inlines: Vec<Inline>,
    resolver: &Resolver,
    counts: &mut CarryLinkCounts,
) -> Vec<Inline> {
    let mut out = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match inline {
            Inline::Link { target, anchor, c } => match resolver.resolve(&target) {
                Some(new_target) => {
                    if new_target != target {
                        counts.body_rewritten += 1;
                    }
                    out.push(Inline::Link {
                        target: new_target,
                        anchor,
                        c: rewrite_inlines(c, resolver, counts),
                    });
                }
                None => {
                    counts.body_flattened += 1;
                    out.extend(rewrite_inlines(c, resolver, counts));
                }
            },
            Inline::Em { c } => out.push(Inline::Em {
                c: rewrite_inlines(c, resolver, counts),
            }),
            Inline::Strong { c } => out.push(Inline::Strong {
                c: rewrite_inlines(c, resolver, counts),
            }),
            Inline::Sup { c } => out.push(Inline::Sup {
                c: rewrite_inlines(c, resolver, counts),
            }),
            Inline::Sub { c } => out.push(Inline::Sub {
                c: rewrite_inlines(c, resolver, counts),
            }),
            other => out.push(other),
        }
    }
    out
}

fn rewrite_table(table: Table, resolver: &Resolver, counts: &mut CarryLinkCounts) -> Table {
    Table {
        caption: rewrite_inlines(table.caption, resolver, counts),
        head: table
            .head
            .into_iter()
            .map(|row| rewrite_row(row, resolver, counts))
            .collect(),
        body: table
            .body
            .into_iter()
            .map(|row| rewrite_row(row, resolver, counts))
            .collect(),
        foot: table
            .foot
            .into_iter()
            .map(|row| rewrite_row(row, resolver, counts))
            .collect(),
    }
}

fn rewrite_row(row: TableRow, resolver: &Resolver, counts: &mut CarryLinkCounts) -> TableRow {
    TableRow {
        cells: row
            .cells
            .into_iter()
            .map(|cell| TableCell {
                header: cell.header,
                colspan: cell.colspan,
                rowspan: cell.rowspan,
                content: rewrite_inlines(cell.content, resolver, counts),
            })
            .collect(),
    }
}

fn collect_links(blocks: &[Block]) -> Vec<EntryReference> {
    let mut links = BTreeSet::new();
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
                collect_inline_links(content, &mut links);
            }
            Block::List { items, .. } => {
                for item in items {
                    links.extend(collect_links(item));
                }
            }
            Block::Table(table) => {
                for cell in table
                    .head
                    .iter()
                    .chain(&table.body)
                    .chain(&table.foot)
                    .flat_map(|row| &row.cells)
                {
                    collect_inline_links(&cell.content, &mut links);
                }
            }
            Block::Quote { content } => links.extend(collect_links(content)),
            Block::StatBlock { rows, .. } => {
                for row in rows {
                    collect_inline_links(&row.value, &mut links);
                }
            }
            Block::EntryLink { target, .. } => {
                links.insert(EntryReference {
                    target: target.clone(),
                    anchor: None,
                });
            }
        }
    }
    links.into_iter().collect()
}

fn collect_inline_links(inlines: &[Inline], links: &mut BTreeSet<EntryReference>) {
    for inline in inlines {
        match inline {
            Inline::Link { target, anchor, c } => {
                links.insert(EntryReference {
                    target: target.clone(),
                    anchor: anchor.clone(),
                });
                collect_inline_links(c, links);
            }
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                collect_inline_links(c, links);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alias::{IdMapMeta, MapRow};
    use cast_iron_schema::{EntryKind, Provenance, Relation};

    fn entry(id: &str, kind: EntryKind, title: &str, source: &str) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: title.into(),
            source_ids: vec![source.into()],
            section: vec!["Section".into()],
            parent: None,
            body: Vec::new(),
            fields: TypedFields::None,
            tags: Vec::new(),
            links: Vec::new(),
            provenance: vec![Provenance {
                dataset: "psrd-data".into(),
                record: format!("pfsrd://Source/{title}"),
                path: format!("{source}/{id}.json"),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: Vec::new(),
        }
    }

    fn link(target: &str, text: &str) -> Inline {
        Inline::Link {
            target: EntryId::parse(target).unwrap(),
            anchor: None,
            c: vec![Inline::Text { v: text.into() }],
        }
    }

    fn map(rows: &[(&str, &str)]) -> IdMap {
        IdMap {
            meta: IdMapMeta {
                format: 1,
                legacy_count: 0,
                legacy_sha256: "0".repeat(64),
                note: None,
            },
            map: rows
                .iter()
                .map(|(old, target)| {
                    (
                        (*old).to_string(),
                        MapRow {
                            target: Some((*target).to_string()),
                            ..MapRow::default()
                        },
                    )
                })
                .collect(),
            shared: BTreeMap::new(),
            retired: BTreeMap::new(),
        }
    }

    fn catalog(records: &[(&str, &str, &str, &str)]) -> CanonicalCatalog {
        let mut catalog = CanonicalCatalog::default();
        for (id, kind, title, source) in records {
            catalog
                .insert(CanonicalRecord {
                    id: (*id).into(),
                    identity: format!("fixture:{id}"),
                    kind: (*kind).into(),
                    title: (*title).into(),
                    source_ids: vec![(*source).into()],
                    origin: "native".into(),
                })
                .unwrap();
        }
        catalog
    }

    #[test]
    fn carried_ids_use_the_reserved_prefix_and_refuse_canonical_input() {
        assert_eq!(
            carried_target("bestiary/monsters/example").unwrap(),
            "ref/carried/bestiary/monsters/example"
        );
        assert!(carried_target("ref/rule/magic").is_err());
        assert!(carried_target("Not A Slug").is_err());
    }

    #[test]
    fn every_target_is_rewritten_three_ways() {
        let mut source = entry(
            "bestiary/monsters/example",
            EntryKind::Creature,
            "Example",
            "bestiary",
        );
        source.parent = Some(EntryId::parse("bestiary/rules/mapped-parent").unwrap());
        source.body = vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                link("bestiary/rules/mapped", "mapped"),
                Inline::Text { v: " ".into() },
                link("bestiary/rules/carried", "carried"),
                Inline::Text { v: " ".into() },
                link("bestiary/rules/retired", "retired"),
            ],
        }];
        source.relations = vec![
            Relation {
                kind: cast_iron_schema::RelationKind::Part,
                target: EntryId::parse("bestiary/rules/mapped-parent").unwrap(),
                label: None,
            },
            Relation {
                kind: cast_iron_schema::RelationKind::Prerequisite,
                target: EntryId::parse("bestiary/rules/retired").unwrap(),
                label: None,
            },
        ];
        source.fields = TypedFields::Feat(cast_iron_schema::FeatFields {
            prerequisite_feats: vec![
                EntryId::parse("bestiary/rules/mapped").unwrap(),
                EntryId::parse("bestiary/rules/carried").unwrap(),
                EntryId::parse("bestiary/rules/retired").unwrap(),
            ],
            ..Default::default()
        });

        let carried_parent = entry(
            "bestiary/rules/carried-parent",
            EntryKind::Rule,
            "Carried parent",
            "bestiary",
        );
        let carried_link = entry(
            "bestiary/rules/carried",
            EntryKind::Rule,
            "Carried link",
            "bestiary",
        );
        let map = map(&[
            (
                "bestiary/monsters/example",
                "ref/carried/bestiary/monsters/example",
            ),
            ("bestiary/rules/mapped", "ref/rule/mapped"),
            ("bestiary/rules/mapped-parent", "ref/rule/mapped-parent"),
            (
                "bestiary/rules/carried",
                "ref/carried/bestiary/rules/carried",
            ),
            (
                "bestiary/rules/carried-parent",
                "ref/carried/bestiary/rules/carried-parent",
            ),
        ]);
        let catalog = catalog(&[("ref/rule/mapped", "rule", "Mapped", "bestiary")]);
        let out = carry_entries(&[source, carried_parent, carried_link], &map, &catalog).unwrap();

        let example = &out.records[0].entry;
        assert_eq!(
            example.parent.as_ref().unwrap().as_str(),
            "ref/rule/mapped-parent"
        );
        let Block::Paragraph { content, .. } = &example.body[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(
            content,
            &vec![
                link("ref/rule/mapped", "mapped"),
                Inline::Text { v: " ".into() },
                link("ref/carried/bestiary/rules/carried", "carried"),
                Inline::Text { v: " ".into() },
                Inline::Text {
                    v: "retired".into()
                },
            ]
        );
        assert_eq!(example.relations.len(), 1);
        assert_eq!(
            example.relations[0].target.as_str(),
            "ref/rule/mapped-parent"
        );
        let TypedFields::Feat(fields) = &example.fields else {
            panic!("expected feat fields");
        };
        assert_eq!(
            fields
                .prerequisite_feats
                .iter()
                .map(EntryId::as_str)
                .collect::<Vec<_>>(),
            ["ref/carried/bestiary/rules/carried", "ref/rule/mapped"]
        );
        assert_eq!(out.report.links.body_rewritten, 2);
        assert_eq!(out.report.links.body_flattened, 1);
        assert_eq!(out.report.links.parent_rewritten, 1);
        assert_eq!(out.report.links.relation_rewritten, 1);
        assert_eq!(out.report.links.relation_flattened, 1);
        assert_eq!(out.report.links.field_rewritten, 2);
        assert_eq!(out.report.links.field_flattened, 1);
    }

    #[test]
    fn an_unresolved_parent_is_dropped_and_reported() {
        let mut source = entry(
            "bestiary/monsters/example",
            EntryKind::Creature,
            "Example",
            "bestiary",
        );
        source.parent = Some(EntryId::parse("bestiary/rules/retired").unwrap());
        let map = map(&[(
            "bestiary/monsters/example",
            "ref/carried/bestiary/monsters/example",
        )]);
        let out = carry_entries(&[source], &map, &CanonicalCatalog::default()).unwrap();
        assert!(out.records[0].entry.parent.is_none());
        assert_eq!(out.report.links.parent_flattened, 1);
    }

    #[test]
    fn a_carried_id_colliding_with_a_canonical_record_fails() {
        let source = entry(
            "bestiary/monsters/example",
            EntryKind::Creature,
            "Example",
            "bestiary",
        );
        let map = map(&[(
            "bestiary/monsters/example",
            "ref/carried/bestiary/monsters/example",
        )]);
        let catalog = catalog(&[(
            "ref/carried/bestiary/monsters/example",
            "creature",
            "Example",
            "bestiary",
        )]);
        let errors = carry_entries(&[source], &map, &catalog).unwrap_err();
        assert!(errors[0].contains("collides"), "{errors:?}");
    }

    #[test]
    fn a_same_title_and_kind_candidate_is_flagged_not_mapped() {
        let source = entry(
            "bestiary/monsters/example",
            EntryKind::Creature,
            "Example",
            "bestiary",
        );
        let map = map(&[(
            "bestiary/monsters/example",
            "ref/carried/bestiary/monsters/example",
        )]);
        let catalog = catalog(&[("ref/monster/example", "creature", "Example", "bestiary-2")]);
        let out = carry_entries(&[source], &map, &catalog).unwrap();
        assert_eq!(
            out.records[0].entry.id.as_str(),
            "ref/carried/bestiary/monsters/example"
        );
        assert_eq!(out.report.possible_duplicate_count, 1);
        assert_eq!(
            out.report.possible_duplicates[0].candidates,
            ["ref/monster/example"]
        );
    }

    #[test]
    fn carry_output_is_deterministic() {
        let source = entry(
            "bestiary/monsters/example",
            EntryKind::Creature,
            "Example",
            "bestiary",
        );
        let map = map(&[(
            "bestiary/monsters/example",
            "ref/carried/bestiary/monsters/example",
        )]);
        let first = carry_entries(
            std::slice::from_ref(&source),
            &map,
            &CanonicalCatalog::default(),
        )
        .unwrap();
        let second = carry_entries(&[source], &map, &CanonicalCatalog::default()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn an_unemitted_target_falls_back_to_carry_over() {
        let source = entry(
            "bestiary/monsters/example",
            EntryKind::Creature,
            "Example",
            "bestiary",
        );
        let map = map(&[("bestiary/monsters/example", "ref/monster/example")]);
        let catalog = catalog(&[("ref/monster/example", "creature", "Example", "bestiary")]);
        let out = carry_entries_with_fallback(
            &[source],
            &map,
            &catalog,
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
        .unwrap();
        assert_eq!(out.records.len(), 1);
        assert_eq!(
            out.records[0].entry.id.as_str(),
            "ref/carried/bestiary/monsters/example"
        );
        assert_eq!(
            out.fallbacks["bestiary/monsters/example"],
            "ref/carried/bestiary/monsters/example"
        );
    }

    #[test]
    fn an_emitted_target_or_a_legally_excluded_one_is_not_carried() {
        let source = entry(
            "bestiary/monsters/example",
            EntryKind::Creature,
            "Example",
            "bestiary",
        );
        let map = map(&[("bestiary/monsters/example", "ref/monster/example")]);
        let catalog = catalog(&[("ref/monster/example", "creature", "Example", "bestiary")]);
        let emitted: BTreeSet<String> = ["ref/monster/example".to_string()].into();
        let out = carry_entries_with_fallback(
            std::slice::from_ref(&source),
            &map,
            &catalog,
            &emitted,
            &BTreeSet::new(),
        )
        .unwrap();
        assert!(out.records.is_empty());

        let excluded: BTreeSet<String> = ["ref/monster/example".to_string()].into();
        let out =
            carry_entries_with_fallback(&[source], &map, &catalog, &BTreeSet::new(), &excluded)
                .unwrap();
        assert!(out.records.is_empty());
    }

    #[test]
    fn a_legacy_id_the_psrd_library_does_not_publish_stays_retired() {
        let map = map(&[("bestiary/monsters/example", "ref/monster/example")]);
        let catalog = catalog(&[("ref/monster/example", "creature", "Example", "bestiary")]);
        let out =
            carry_entries_with_fallback(&[], &map, &catalog, &BTreeSet::new(), &BTreeSet::new())
                .unwrap();
        assert!(out.records.is_empty());
        assert!(out.fallbacks.is_empty());
    }
}
