//! Coverage for a library that comes from the plain-text corpus.
//!
//! A [`CoverageReport`] has three kinds of content:
//!
//! * counts that the entries, the publication list, and the book index state, which
//!   [`derive`] recomputes every time, so a hand edit can never leave them stale;
//! * exclusion and rejection rows, which the corpus stores as `exclusions.toml`;
//! * a few results of the one-time import (the upstream digest, upstream record counts,
//!   how prerequisite wording was read, which parts were left unplaced, how upstream page
//!   numbers placed entries), which the corpus stores as the frozen `history.toml`.
//!
//! [`derive`] puts the three parts together.

use crate::backlinks;
use crate::lists;
use crate::relations;
use cast_iron_schema::{
    CoverageHistory, CoverageReport, Exclusion, LinkCoverage, PublicationCoverage, Rejection,
    RelationCoverage, RuleEntry, SourceInfo, UpstreamPublication,
};
use std::collections::{BTreeMap, BTreeSet};

/// Rebuilds the coverage report.
pub fn derive(
    entries: &[RuleEntry],
    sources: &[SourceInfo],
    excluded: &[Exclusion],
    rejected: &[Rejection],
    history: &CoverageHistory,
) -> Result<CoverageReport, String> {
    let mut by_kind: BTreeMap<String, u32> = BTreeMap::new();
    let mut included: BTreeMap<&str, u32> = BTreeMap::new();
    for entry in entries {
        *by_kind.entry(entry.kind.slug().to_string()).or_default() += 1;
        for source in &entry.source_ids {
            *included.entry(source.as_str()).or_default() += 1;
        }
    }
    let catalogue: BTreeMap<&str, &str> = sources
        .iter()
        .map(|source| (source.id.as_str(), source.title.as_str()))
        .collect();
    let upstream: BTreeMap<&str, &UpstreamPublication> = history
        .publications
        .iter()
        .map(|row| (row.source_id.as_str(), row))
        .collect();
    let mut ids: BTreeSet<&str> = upstream.keys().copied().collect();
    ids.extend(included.keys().copied());
    let by_publication = ids
        .into_iter()
        .map(|id| PublicationCoverage {
            source_id: id.to_string(),
            title: upstream
                .get(id)
                .and_then(|row| row.title.clone())
                .or_else(|| catalogue.get(id).map(|title| title.to_string()))
                .unwrap_or_else(|| id.to_string()),
            upstream_records: upstream.get(id).map_or(0, |row| row.upstream_records),
            included_entries: included.get(id).copied().unwrap_or(0),
        })
        .collect();

    let edges = relations::index_over(entries, RelationCoverage::default())
        .coverage
        .edges;
    let backlinks = backlinks::build(entries);
    let mut backlink_coverage = backlinks.coverage.clone();
    backlink_coverage.bytes = backlinks_bytes(&backlinks)?;

    Ok(CoverageReport {
        input_digest: history.input_digest.clone(),
        by_publication,
        by_kind,
        excluded: excluded.to_vec(),
        rejected: rejected.to_vec(),
        links: LinkCoverage {
            resolved: entries.iter().map(|entry| entry.links.len() as u32).sum(),
            out_of_selection: 0,
            broken: 0,
        },
        stripped: BTreeMap::new(),
        relations: RelationCoverage {
            edges,
            prerequisites: history.prerequisites.clone(),
            spell_lists: lists::build(entries).coverage,
            unplaced_parts: history.unplaced_parts.clone(),
            unpromoted: history.unpromoted.clone(),
            book_order: history.book_order.clone(),
        },
        backlinks: backlink_coverage,
    })
}

/// Bytes of the backlink descriptor plus every shard, as the packaging step counts them.
fn backlinks_bytes(built: &backlinks::Built) -> Result<u64, String> {
    let mut bytes = serde_json::to_vec(&built.index)
        .map_err(|e| e.to_string())?
        .len() as u64;
    for shard in &built.shards {
        bytes += serde_json::to_vec(shard).map_err(|e| e.to_string())?.len() as u64;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{
        Block, EntryId, EntryKind, EntryReference, Inline, ParagraphStyle, Relation, RelationKind,
        TypedFields,
    };

    fn entry(id: &str, kind: EntryKind, sources: &[&str], target: Option<&str>) -> RuleEntry {
        let mut entry = RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: id.into(),
            source_ids: sources.iter().map(|s| s.to_string()).collect(),
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::None,
            tags: vec![],
            links: vec![],
            provenance: vec![],
            license_refs: vec![],
            relations: vec![],
        };
        if let Some(target) = target {
            entry.body = vec![Block::Paragraph {
                style: ParagraphStyle::Normal,
                content: vec![Inline::Link {
                    target: EntryId::parse(target).unwrap(),
                    anchor: None,
                    c: vec![Inline::text("x")],
                }],
            }];
            entry.links = vec![EntryReference {
                target: EntryId::parse(target).unwrap(),
                anchor: None,
            }];
        }
        entry
    }

    fn source(id: &str, title: &str) -> SourceInfo {
        SourceInfo {
            id: id.into(),
            title: title.into(),
            entry_count: 0,
            copyright: None,
            category: None,
            publication_date: None,
        }
    }

    #[test]
    fn counts_come_from_the_entries_and_rows_from_the_files() {
        let mut a = entry(
            "ref/rule/a",
            EntryKind::Rule,
            &["book", "other"],
            Some("ref/rule/b"),
        );
        a.relations = vec![Relation {
            kind: RelationKind::Part,
            target: EntryId::parse("ref/rule/b").unwrap(),
            label: None,
        }];
        let entries = vec![a, entry("ref/rule/b", EntryKind::Rule, &["book"], None)];
        let sources = vec![source("book", "Book"), source("other", "Other")];
        let history = CoverageHistory {
            input_digest: "0".repeat(64),
            publications: vec![
                UpstreamPublication {
                    source_id: "book".into(),
                    upstream_records: 7,
                    title: None,
                },
                UpstreamPublication {
                    source_id: "gone".into(),
                    upstream_records: 2,
                    title: Some("Gone".into()),
                },
            ],
            ..Default::default()
        };
        let excluded = vec![Exclusion {
            path: "json/x.json".into(),
            records: 4,
            reason: "art".into(),
        }];
        let rejected = vec![Rejection {
            path: "json/y.json#z".into(),
            reason: "bad".into(),
        }];
        let report = derive(&entries, &sources, &excluded, &rejected, &history).unwrap();
        assert_eq!(report.by_kind["rule"], 2);
        assert_eq!(report.links.resolved, 1);
        assert_eq!(report.relations.edges["part"], 1);
        assert_eq!(report.excluded, excluded);
        assert_eq!(report.rejected, rejected);
        let rows: Vec<_> = report
            .by_publication
            .iter()
            .map(|row| {
                (
                    row.source_id.as_str(),
                    row.title.as_str(),
                    row.upstream_records,
                    row.included_entries,
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                ("book", "Book", 7, 2),
                ("gone", "Gone", 2, 0),
                ("other", "Other", 0, 1),
            ]
        );
        assert!(report.backlinks.bytes > 0);
    }

    #[test]
    fn counts_follow_an_edit_instead_of_staying_frozen() {
        let sources = vec![source("book", "Book")];
        let history = CoverageHistory {
            input_digest: "0".repeat(64),
            ..Default::default()
        };
        let before = derive(
            &[entry("ref/rule/a", EntryKind::Rule, &["book"], None)],
            &sources,
            &[],
            &[],
            &history,
        )
        .unwrap();
        let after = derive(
            &[
                entry("ref/rule/a", EntryKind::Rule, &["book"], None),
                entry("ref/rule/b", EntryKind::Rule, &["book"], Some("ref/rule/a")),
            ],
            &sources,
            &[],
            &[],
            &history,
        )
        .unwrap();
        assert_eq!(before.by_kind["rule"], 1);
        assert_eq!(after.by_kind["rule"], 2);
        assert_eq!(after.links.resolved, 1);
        assert_eq!(after.by_publication[0].included_entries, 2);
    }
}
