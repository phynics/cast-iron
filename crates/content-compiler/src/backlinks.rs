//! Reverse body-link index: target entry to the entries whose bodies link to it.
//!
//! Every [`RuleEntry::links`] row is already a resolved internal reference. This pass turns
//! those rows around, drops the two cases that would duplicate another screen, and splits the
//! result into stable shards so the client reads one bucket per entry view.
//!
//! The rules, which the coverage report records:
//!
//! * A link from an entry to itself is dropped. "Referenced by" must not list the page the
//!   reader is already on.
//! * A link whose source states a typed [`Relation`] to the same target is dropped. The
//!   relation panels (racial traits, archetypes, feat prerequisites) already show that edge,
//!   so a second body-link row would duplicate it.
//! * One row per source and target. When a source reaches a target through more than one
//!   anchor, no anchor is kept, because no single heading is the reference.

use cast_iron_schema::{
    BACKLINK_FORMAT, BACKLINK_SHARDS, BacklinkCoverage, BacklinkIndex, BacklinkSource,
    BacklinkTarget, EntryId, RuleEntry, backlink_shard,
};
use std::collections::{BTreeMap, BTreeSet};

/// The descriptor, the shard contents, and the coverage rows for one compile.
pub struct Built {
    pub index: BacklinkIndex,
    /// One vector per shard, indexed by shard number. Each is sorted by target id.
    pub shards: Vec<Vec<BacklinkTarget>>,
    pub coverage: BacklinkCoverage,
}

/// Builds the reverse index from the entries that are about to be packaged.
pub fn build(entries: &[RuleEntry]) -> Built {
    let mut by_target: BTreeMap<EntryId, BTreeMap<EntryId, BTreeSet<Option<String>>>> =
        BTreeMap::new();
    let mut excluded_self = 0u32;
    let mut excluded_relation = 0u32;

    for entry in entries {
        let relations: BTreeSet<&EntryId> = entry
            .relations
            .iter()
            .map(|relation| &relation.target)
            .collect();
        for link in &entry.links {
            if link.target == entry.id {
                excluded_self += 1;
                continue;
            }
            if relations.contains(&link.target) {
                excluded_relation += 1;
                continue;
            }
            by_target
                .entry(link.target.clone())
                .or_default()
                .entry(entry.id.clone())
                .or_default()
                .insert(link.anchor.clone());
        }
    }

    let mut shards: Vec<Vec<BacklinkTarget>> = (0..BACKLINK_SHARDS).map(|_| Vec::new()).collect();
    let mut links = 0u32;
    let mut targets = 0u32;
    for (target, sources) in by_target {
        let mut rows = Vec::with_capacity(sources.len());
        for (source, anchors) in sources {
            // A single anchor is a real section reference. Two or more, or a mix of an
            // anchor and the entry top, do not identify one section.
            let anchor = if anchors.len() == 1 {
                anchors.into_iter().next().flatten()
            } else {
                None
            };
            rows.push(BacklinkSource(source, anchor));
        }
        links += rows.len() as u32;
        targets += 1;
        let bucket = backlink_shard(target.as_str(), BACKLINK_SHARDS) as usize;
        shards[bucket].push(BacklinkTarget(target, rows));
    }

    Built {
        index: BacklinkIndex {
            format: BACKLINK_FORMAT,
            shards: BACKLINK_SHARDS,
            links,
            targets,
        },
        shards,
        coverage: BacklinkCoverage {
            links,
            targets,
            excluded_self,
            excluded_relation,
            bytes: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{
        Block, EntryKind, EntryReference, Inline, Provenance, Relation, RelationKind, TypedFields,
    };

    fn entry(id: &str, links: &[(&str, Option<&str>)]) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Rule,
            title: id.into(),
            aon: None,
            source_ids: vec!["s".into()],
            sources: vec![],
            section: vec![],
            parent: None,
            body: vec![Block::Paragraph {
                style: Default::default(),
                content: vec![Inline::text("x")],
            }],
            fields: TypedFields::None,
            tags: vec![],
            links: links
                .iter()
                .map(|(target, anchor)| EntryReference {
                    target: EntryId::parse(*target).unwrap(),
                    anchor: anchor.map(str::to_string),
                })
                .collect(),
            provenance: vec![Provenance {
                dataset: "d".into(),
                record: "r".into(),
                path: "p".into(),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: vec![],
        }
    }

    fn relation(target: &str) -> Relation {
        Relation {
            kind: RelationKind::Part,
            target: EntryId::parse(target).unwrap(),
            label: None,
        }
    }

    fn shard_json(built: &Built) -> Vec<String> {
        built
            .shards
            .iter()
            .map(|shard| serde_json::to_string(shard).unwrap())
            .collect()
    }

    #[test]
    fn output_is_byte_identical_between_runs() {
        let entries = vec![
            entry("ref/rule/a", &[("ref/rule/c", None)]),
            entry("ref/rule/b", &[("ref/rule/c", Some("section"))]),
        ];
        let first = build(&entries);
        let second = build(&entries);
        assert_eq!(first.index, second.index);
        assert_eq!(shard_json(&first), shard_json(&second));
    }

    #[test]
    fn self_links_are_excluded_and_counted() {
        let built = build(&[entry("ref/rule/a", &[("ref/rule/a", None)])]);
        assert_eq!(built.coverage.excluded_self, 1);
        assert_eq!(built.coverage.links, 0);
        assert_eq!(built.coverage.targets, 0);
        assert!(built.shards.iter().all(Vec::is_empty));
    }

    #[test]
    fn links_that_repeat_a_typed_relation_are_excluded_and_counted() {
        let mut source = entry("ref/rule/a", &[("ref/rule/b", None)]);
        source.relations.push(relation("ref/rule/b"));
        let built = build(&[source]);
        assert_eq!(built.coverage.excluded_relation, 1);
        assert_eq!(built.coverage.links, 0);
        assert!(built.shards.iter().all(Vec::is_empty));
    }

    #[test]
    fn a_variant_body_link_does_not_repeat_the_variants_row() {
        let mut variant = entry("ref/spell/mass-cure", &[("ref/spell/cure", None)]);
        variant.relations.push(Relation {
            kind: RelationKind::Variant,
            target: EntryId::parse("ref/spell/cure").unwrap(),
            label: None,
        });
        let built = build(&[variant]);
        assert_eq!(built.coverage.excluded_relation, 1);
        assert_eq!(built.coverage.links, 0);
        assert!(built.shards.iter().all(Vec::is_empty));
    }

    #[test]
    fn a_unique_anchor_is_kept_and_several_anchors_keep_none() {
        let built = build(&[
            entry("ref/rule/a", &[("ref/rule/b", Some("one"))]),
            entry(
                "ref/rule/c",
                &[("ref/rule/b", Some("one")), ("ref/rule/b", Some("two"))],
            ),
            entry("ref/rule/d", &[("ref/rule/b", None)]),
        ]);
        let target = built
            .shards
            .iter()
            .flatten()
            .find(|target| target.0.as_str() == "ref/rule/b")
            .expect("target b has a row");
        assert_eq!(
            target.1,
            vec![
                BacklinkSource(EntryId::parse("ref/rule/a").unwrap(), Some("one".into())),
                BacklinkSource(EntryId::parse("ref/rule/c").unwrap(), None),
                BacklinkSource(EntryId::parse("ref/rule/d").unwrap(), None),
            ]
        );
    }

    #[test]
    fn every_target_lands_in_exactly_one_shard_and_sources_stay_sorted() {
        let entries: Vec<RuleEntry> = (0..64)
            .map(|i| {
                entry(
                    &format!("ref/rule/source-{i:02}"),
                    &[("ref/rule/target", None)],
                )
            })
            .collect();
        let built = build(&entries);
        let expected = backlink_shard("ref/rule/target", BACKLINK_SHARDS) as usize;
        let holders: Vec<usize> = built
            .shards
            .iter()
            .enumerate()
            .filter(|(_, shard)| {
                shard
                    .iter()
                    .any(|target| target.0.as_str() == "ref/rule/target")
            })
            .map(|(index, _)| index)
            .collect();
        assert_eq!(holders, vec![expected]);
        let target = &built.shards[expected]
            .iter()
            .find(|target| target.0.as_str() == "ref/rule/target")
            .unwrap()
            .1;
        assert_eq!(target.len(), 64);
        assert!(target.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn shards_are_sorted_by_target_and_empty_shards_still_exist() {
        let entries: Vec<RuleEntry> = ["ref/rule/z", "ref/rule/a", "ref/rule/m"]
            .into_iter()
            .map(|id| entry(id, &[("ref/rule/shared", None)]))
            .collect();
        let built = build(&entries);
        assert_eq!(built.shards.len(), BACKLINK_SHARDS as usize);
        for shard in &built.shards {
            assert!(shard.windows(2).all(|pair| pair[0].0 < pair[1].0));
        }
        assert_eq!(built.index.links, 3);
        assert_eq!(built.index.targets, 1);
    }
}
