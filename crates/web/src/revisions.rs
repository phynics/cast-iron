//! Third-party revisions of a feat: the effective entry, the row suppression, and the search
//! overlay. Pure data, so every rule is tested natively.
//!
//! A revision is its own entry from its own book. It holds one `revision` edge to the feat it
//! revises. While the revision's book is active, exactly one revision in force replaces the
//! original's fields and body under the original id; two or more in force mean no substitution.
//! Removed and merged revisions leave the list but keep their original findable in search.

use crate::graph::Relations;
use cast_iron_schema::{EntryId, RuleEntry, TypedFields};
use cast_iron_search::HitSummary;
use std::collections::{BTreeMap, BTreeSet};

/// What a revision says happened to the feat it points at. The edge label picks it; `None`
/// means the text was revised in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RevisionLabel {
    Revised,
    Removed,
    Merged,
}

impl RevisionLabel {
    /// The label of a `revision` edge. An unknown label is not a revision state this reader
    /// understands; the corpus check keeps that from shipping.
    pub fn from_edge(label: Option<&str>) -> Option<Self> {
        match label {
            None => Some(Self::Revised),
            Some("removed") => Some(Self::Removed),
            Some("merged") => Some(Self::Merged),
            Some(_) => None,
        }
    }

    /// The word a notice uses: `Revised`, `Removed`, or `Merged`.
    pub fn notice_word(self) -> &'static str {
        match self {
            Self::Revised => "Revised",
            Self::Removed => "Removed",
            Self::Merged => "Merged",
        }
    }
}

/// One revision whose own book is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRevision {
    pub revision: EntryId,
    pub base: EntryId,
    pub label: RevisionLabel,
    /// The revision's primary publication, when the catalog has it.
    pub source: Option<String>,
}

impl ActiveRevision {
    pub fn is_revised(&self) -> bool {
        self.label == RevisionLabel::Revised
    }
}

/// The revision state of every base, built from the relation graph and the active books.
#[derive(Debug, Clone, Default)]
pub struct RevisionView {
    by_revision: BTreeMap<EntryId, ActiveRevision>,
    by_base: BTreeMap<EntryId, Vec<ActiveRevision>>,
}

impl RevisionView {
    /// Reads every revision edge whose own entry is active. The base's book plays no part: a
    /// revision is gated by its own book alone, and an inactive base never hides it.
    pub fn build(
        relations: &Relations,
        is_active: impl Fn(&EntryId) -> bool,
        source_of: impl Fn(&EntryId) -> Option<String>,
    ) -> Self {
        let mut by_revision = BTreeMap::new();
        let mut by_base: BTreeMap<EntryId, Vec<ActiveRevision>> = BTreeMap::new();
        for (revision, label, base) in relations.revision_edges() {
            if !is_active(revision) {
                continue;
            }
            let Some(label) = RevisionLabel::from_edge(label) else {
                continue;
            };
            let active = ActiveRevision {
                revision: revision.clone(),
                base: base.clone(),
                label,
                source: source_of(revision),
            };
            by_base
                .entry(base.clone())
                .or_default()
                .push(active.clone());
            by_revision.insert(revision.clone(), active);
        }
        Self {
            by_revision,
            by_base,
        }
    }

    /// The active revision held by this entry, when the entry is itself a revision.
    pub fn for_revision(&self, id: &EntryId) -> Option<&ActiveRevision> {
        self.by_revision.get(id)
    }

    /// Every active revision of `base`, in revision-id order.
    pub fn active_for(&self, base: &EntryId) -> &[ActiveRevision] {
        self.by_base.get(base).map_or(&[], Vec::as_slice)
    }

    /// The single active revision of `base`. `None` when there is none or when two or more
    /// are in force, because that case must not substitute.
    pub fn in_force(&self, base: &EntryId) -> Option<&ActiveRevision> {
        match self.by_base.get(base).map(Vec::as_slice) {
            Some([one]) => Some(one),
            _ => None,
        }
    }

    /// True when two or more revisions of `base` are active. The page then shows the original
    /// plus a `Revised by` list, never a last-writer-wins choice.
    pub fn is_collision(&self, base: &EntryId) -> bool {
        self.by_base.get(base).is_some_and(|rows| rows.len() > 1)
    }

    /// The rows that a list must hide while `visible` says the entry's book is active.
    ///
    /// A revision hides when its base is visible, because the base row already shows the
    /// revision. A removed or merged base hides too. When the base is not visible the revision
    /// stays listed: it is the only entry the reader can reach.
    pub fn rows(&self, visible: impl Fn(&EntryId) -> bool) -> RevisionRows {
        let mut rows = RevisionRows::default();
        for (revision, active) in &self.by_revision {
            if !visible(&active.base) {
                continue;
            }
            rows.hidden_revisions.insert(revision.clone());
            if !self.is_collision(&active.base) && !active.is_revised() {
                rows.hidden_bases.insert(active.base.clone());
                if let Some(source) = &active.source {
                    *rows.removed_by_source.entry(source.clone()).or_default() += 1;
                }
            } else if self
                .in_force(&active.base)
                .is_some_and(ActiveRevision::is_revised)
                && let Some(source) = &active.source
            {
                rows.effective_source
                    .insert(active.base.clone(), source.clone());
            }
        }
        rows
    }
}

/// The row changes for one active-book selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevisionRows {
    hidden_revisions: BTreeSet<EntryId>,
    hidden_bases: BTreeSet<EntryId>,
    /// Base id to the revision's book id, for the row's muted line.
    effective_source: BTreeMap<EntryId, String>,
    /// Removed or merged feats per revision book, for the line under the list.
    removed_by_source: BTreeMap<String, usize>,
}

impl RevisionRows {
    pub fn hides(&self, id: &EntryId) -> bool {
        self.hidden_revisions.contains(id) || self.hidden_bases.contains(id)
    }

    /// The ids this selection keeps out of lists, in id order. A category count subtracts the
    /// entries of its own kind, so a revision never inflates the number on Home.
    pub fn hidden_ids(&self) -> impl Iterator<Item = &EntryId> {
        self.hidden_revisions.iter().chain(self.hidden_bases.iter())
    }

    /// The book id a row should name when a revised text is in force.
    pub fn source_of(&self, id: &EntryId) -> Option<&str> {
        self.effective_source.get(id).map(String::as_str)
    }

    /// The removed and merged count and book id, when one book accounts for every row that
    /// left the list. More than one book would need more than one line.
    pub fn removed_note(&self) -> Option<(usize, &str)> {
        match self.removed_by_source.iter().next() {
            Some((source, count)) if self.removed_by_source.len() == 1 => {
                Some((*count, source.as_str()))
            }
            _ => None,
        }
    }
}

/// The page's effective entry: the original id and metadata, with the revision's fields,
/// body, links, and prerequisite edges. The revision edge itself is not copied, so the page
/// never says "Revision of" about its own text.
pub fn effective_entry(base: &RuleEntry, revision: &RuleEntry) -> RuleEntry {
    RuleEntry {
        id: base.id.clone(),
        kind: base.kind,
        title: base.title.clone(),
        source_ids: base.source_ids.clone(),
        section: base.section.clone(),
        parent: base.parent.clone(),
        body: revision.body.clone(),
        fields: revision.fields.clone(),
        tags: base.tags.clone(),
        links: revision.links.clone(),
        provenance: base.provenance.clone(),
        license_refs: base.license_refs.clone(),
        relations: revision
            .relations
            .iter()
            .filter(|relation| relation.kind != cast_iron_schema::RelationKind::Revision)
            .cloned()
            .collect(),
    }
}

/// The typed feat fields whose values differ between the original and the revision, in the
/// order the stat block prints them. An empty list means only prose changed.
pub fn changed_fields(base: &TypedFields, revision: &TypedFields) -> Vec<&'static str> {
    let (TypedFields::Feat(base), TypedFields::Feat(revision)) = (base, revision) else {
        return Vec::new();
    };
    let mut changed = Vec::new();
    if base.feat_types != revision.feat_types {
        changed.push("Type");
    }
    if base.prerequisites != revision.prerequisites
        || base.prerequisite_feats != revision.prerequisite_feats
    {
        changed.push("Prerequisites");
    }
    if base.benefit != revision.benefit {
        changed.push("Benefit");
    }
    if base.normal != revision.normal {
        changed.push("Normal");
    }
    if base.special != revision.special {
        changed.push("Special");
    }
    changed
}

/// The reader-facing field list of a revised notice: `Benefit`, or `Text` when only prose
/// differs. The caller adds the book and the punctuation.
pub fn changed_phrase(changed: &[&str]) -> String {
    if changed.is_empty() {
        "Text".to_string()
    } else {
        changed.join(", ")
    }
}

/// Rewrites search hits so a revision appears once, under the original id, with the revision's
/// row detail. A base whose book is active never competes with its active revision: the
/// revision hit supplies the snippet (its text is the effective text) and the source (the
/// book that revised it). A removed or merged feat stays findable through its base id. When the
/// base's book is inactive the revision keeps its own id, because it is the only row.
pub fn effective_hits(
    hits: Vec<HitSummary>,
    view: &RevisionView,
    is_eligible: impl Fn(&str) -> bool,
) -> Vec<HitSummary> {
    // A base that a matching revision hit names will be shown from that hit, not from the
    // original's document.
    let mut replaced: BTreeSet<EntryId> = BTreeSet::new();
    for hit in &hits {
        let Ok(id) = EntryId::parse(hit.id.clone()) else {
            continue;
        };
        if let Some(active) = view.for_revision(&id)
            && is_eligible(active.base.as_str())
            && view.in_force(&active.base).is_some()
        {
            replaced.insert(active.base.clone());
        }
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::with_capacity(hits.len());
    for mut hit in hits {
        let Ok(id) = EntryId::parse(hit.id.clone()) else {
            if seen.insert(hit.id.clone()) {
                out.push(hit);
            }
            continue;
        };
        if let Some(active) = view.for_revision(&id) {
            if is_eligible(active.base.as_str())
                && let Some(in_force) = view.in_force(&active.base)
            {
                hit.id = in_force.base.to_string();
                if let Some(source) = &in_force.source {
                    hit.source = source.clone();
                }
            }
        } else if view.in_force(&id).is_some() && is_eligible(&hit.id) {
            if replaced.contains(&id) {
                // The revision hit carries the effective text.
                continue;
            }
            if let Some(source) = view.in_force(&id).and_then(|active| active.source.clone()) {
                hit.source = source;
            }
        }
        if seen.insert(hit.id.clone()) {
            out.push(hit);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{FeatFields, RelationEdge, RelationIndex, RelationKind};
    use std::collections::BTreeMap;

    fn id(value: &str) -> EntryId {
        EntryId::parse(value).unwrap()
    }

    fn relations(edges: &[(&str, Option<&str>, &str)]) -> Relations {
        Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: edges
                .iter()
                .map(|(revision, label, base)| {
                    RelationEdge(
                        id(revision),
                        RelationKind::Revision,
                        id(base),
                        label.map(String::from),
                    )
                })
                .collect(),
        })
    }

    fn feat(benefit: &str) -> TypedFields {
        TypedFields::Feat(FeatFields {
            benefit: Some(benefit.into()),
            ..Default::default()
        })
    }

    #[test]
    fn one_active_revision_is_in_force_and_changes_the_fields() {
        let graph = relations(&[("ref/feat/dodge/eitr", None, "ref/feat/dodge")]);
        let view = RevisionView::build(
            &graph,
            |_| true,
            |revision| {
                (revision.as_str() == "ref/feat/dodge/eitr").then(|| "the-elephant".to_string())
            },
        );
        let active = view.in_force(&id("ref/feat/dodge")).unwrap();
        assert_eq!(active.revision, id("ref/feat/dodge/eitr"));
        assert_eq!(active.label, RevisionLabel::Revised);
        assert_eq!(active.source.as_deref(), Some("the-elephant"));
        assert!(!view.is_collision(&id("ref/feat/dodge")));
    }

    #[test]
    fn removed_and_merged_revisions_keep_their_label_and_leave_the_list() {
        for (revision, label, word) in [
            ("ref/feat/power-attack/eitr", "removed", "Removed"),
            ("ref/feat/mobility/eitr", "merged", "Merged"),
        ] {
            let graph = relations(&[(revision, Some(label), revision.trim_end_matches("/eitr"))]);
            let view = RevisionView::build(&graph, |_| true, |_| Some("the-elephant".into()));
            let active = view
                .in_force(&id(revision.trim_end_matches("/eitr")))
                .unwrap();
            assert_eq!(active.label.notice_word(), word);
            let rows = view.rows(|_| true);
            assert!(rows.hides(&id(revision)));
            assert!(rows.hides(&id(revision.trim_end_matches("/eitr"))));
            let (count, source) = rows.removed_note().unwrap();
            assert_eq!(count, 1);
            assert_eq!(source, "the-elephant");
        }
    }

    #[test]
    fn an_inactive_book_changes_nothing() {
        let graph = relations(&[("ref/feat/dodge/eitr", None, "ref/feat/dodge")]);
        let view = RevisionView::build(
            &graph,
            |revision| revision.as_str() != "ref/feat/dodge/eitr",
            |_| Some("the-elephant".into()),
        );
        assert!(view.in_force(&id("ref/feat/dodge")).is_none());
        let rows = view.rows(|_| true);
        assert!(!rows.hides(&id("ref/feat/dodge/eitr")));
        assert!(rows.source_of(&id("ref/feat/dodge")).is_none());
    }

    #[test]
    fn two_revisions_in_force_do_not_substitute() {
        let graph = relations(&[
            ("ref/feat/dodge/a", None, "ref/feat/dodge"),
            ("ref/feat/dodge/b", None, "ref/feat/dodge"),
        ]);
        let view = RevisionView::build(&graph, |_| true, |_| Some("book".into()));
        assert!(view.is_collision(&id("ref/feat/dodge")));
        assert!(view.in_force(&id("ref/feat/dodge")).is_none());
        let rows = view.rows(|_| true);
        // Both revision rows still hide behind the original, but the original keeps its fields.
        assert!(rows.hides(&id("ref/feat/dodge/a")));
        assert!(rows.hides(&id("ref/feat/dodge/b")));
        assert!(rows.source_of(&id("ref/feat/dodge")).is_none());
    }

    #[test]
    fn a_revision_stays_listed_when_its_base_is_hidden() {
        let graph = relations(&[("ref/feat/dodge/eitr", None, "ref/feat/dodge")]);
        let view = RevisionView::build(&graph, |_| true, |_| Some("the-elephant".into()));
        let rows = view.rows(|_| false);
        assert!(!rows.hides(&id("ref/feat/dodge/eitr")));
        assert!(rows.source_of(&id("ref/feat/dodge")).is_none());
    }

    #[test]
    fn a_changed_field_is_named_in_print_order() {
        let base = feat("Old benefit.");
        let revision = feat("New benefit.");
        assert_eq!(changed_fields(&base, &revision), ["Benefit"]);
        assert_eq!(changed_phrase(&changed_fields(&base, &revision)), "Benefit");
        assert_eq!(changed_fields(&revision, &revision), Vec::<&str>::new());
        assert_eq!(changed_phrase(&[]), "Text");
    }

    #[test]
    fn effective_hits_show_one_row_under_the_original_id() {
        let graph = relations(&[("ref/feat/dodge/eitr", None, "ref/feat/dodge")]);
        let view = RevisionView::build(&graph, |_| true, |_| Some("the-elephant".into()));
        let hit = |id: &str, source: &str| HitSummary {
            ord: 0,
            id: id.into(),
            title: "Dodge".into(),
            kind: "feat".into(),
            source: source.into(),
            score: 1.0,
        };
        let hits = effective_hits(
            vec![
                hit("ref/feat/dodge", "core-rulebook"),
                hit("ref/feat/dodge/eitr", "the-elephant"),
            ],
            &view,
            |id| id.starts_with("ref/feat/dodge"),
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "ref/feat/dodge");
        assert_eq!(hits[0].source, "the-elephant");
    }

    #[test]
    fn a_base_hit_alone_takes_the_revision_book_detail() {
        let graph = relations(&[("ref/feat/dodge/eitr", None, "ref/feat/dodge")]);
        let view = RevisionView::build(&graph, |_| true, |_| Some("the-elephant".into()));
        let hits = effective_hits(
            vec![HitSummary {
                ord: 0,
                id: "ref/feat/dodge".into(),
                title: "Dodge".into(),
                kind: "feat".into(),
                source: "core-rulebook".into(),
                score: 1.0,
            }],
            &view,
            |_| true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source, "the-elephant");
    }

    #[test]
    fn effective_entry_keeps_the_original_identity_and_takes_the_revision_text() {
        let base = RuleEntry {
            id: id("ref/feat/dodge"),
            kind: cast_iron_schema::EntryKind::Feat,
            title: "Dodge".into(),
            source_ids: vec!["core-rulebook".into()],
            section: vec![],
            parent: None,
            body: vec![],
            fields: feat("Old benefit."),
            tags: vec![],
            links: vec![],
            provenance: vec![],
            license_refs: vec![],
            relations: vec![],
        };
        let mut revision = base.clone();
        revision.id = id("ref/feat/dodge/eitr");
        revision.title = "Dodge".into();
        revision.fields = feat("New benefit.");
        revision.relations = vec![cast_iron_schema::Relation {
            kind: RelationKind::Revision,
            target: base.id.clone(),
            label: None,
        }];
        let effective = effective_entry(&base, &revision);
        assert_eq!(effective.id, base.id);
        assert_eq!(effective.fields, revision.fields);
        assert!(
            effective
                .relations
                .iter()
                .all(|r| r.kind != RelationKind::Revision)
        );
    }
}
