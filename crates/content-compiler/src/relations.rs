//! The relation index: every typed relation stored on an entry, as one sorted edge list with
//! counts. The corpus stores each relation on its entry; this module only indexes them.

use cast_iron_schema::{RelationCoverage, RelationEdge, RelationIndex, RuleEntry};
use std::collections::{BTreeMap, BTreeSet};

pub struct Report {
    pub index: RelationIndex,
    pub coverage: RelationCoverage,
}

/// Builds the relation index from the relations stored on the entries.
///
/// The index and the per-entry relations must agree, which
/// [`cast_iron_schema::validate::validate_derived`] requires.
pub fn index_over(entries: &[RuleEntry], mut coverage: RelationCoverage) -> Report {
    let mut edges: BTreeSet<RelationEdge> = BTreeSet::new();
    for entry in entries.iter() {
        for relation in &entry.relations {
            edges.insert(RelationEdge(
                entry.id.clone(),
                relation.kind,
                relation.target.clone(),
            ));
        }
    }
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for RelationEdge(_, kind, _) in &edges {
        *counts.entry(kind.slug().to_string()).or_default() += 1;
    }
    coverage.edges = counts.clone();
    coverage
        .unplaced_parts
        .sort_by(|a, b| a.entry.cmp(&b.entry));
    Report {
        index: RelationIndex {
            counts,
            edges: edges.into_iter().collect(),
        },
        coverage,
    }
}
