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
use std::rc::Rc;

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
    /// A merged base's destination feat, read from the revision's printed link. A graph uses
    /// it to point the base's dependents at the feat the base became.
    merge_targets: BTreeMap<EntryId, EntryId>,
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
            merge_targets: BTreeMap::new(),
        }
    }

    /// Records where a merged base's revision says the feat went. The page that owns the
    /// revision entry reads its printed link and passes it here before the graph is derived.
    pub fn set_merge_target(&mut self, base: &EntryId, target: EntryId) {
        self.merge_targets.insert(base.clone(), target);
    }

    /// Every active revision that merged its base into another feat.
    pub fn merged_revisions(&self) -> impl Iterator<Item = &ActiveRevision> {
        self.by_revision
            .values()
            .filter(|active| active.label == RevisionLabel::Merged)
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
            rows.hidden_strings.insert(revision.to_string());
            if !self.is_collision(&active.base) && !active.is_revised() {
                rows.hidden_bases.insert(active.base.clone());
                rows.hidden_strings.insert(active.base.to_string());
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

    /// The graph that feat chains and relation panels walk while these revisions are in force.
    ///
    /// A revision row leaves with the base row it replaces. A removed base and every edge into
    /// it leave too; an edge into a merged base points at the feat that absorbed it. A revised
    /// base takes the revision's outgoing prerequisite edges, so a chain follows the revised
    /// text rather than the printed original.
    pub fn effective_graph(
        &self,
        relations: &Relations,
        visible: impl Fn(&EntryId) -> bool,
    ) -> Relations {
        let rows = self.rows(&visible);
        // Only a single revision in force substitutes. A collision keeps the printed base and
        // its prerequisites, exactly as the page keeps the original text under its notice.
        let prerequisites: BTreeMap<EntryId, Vec<(EntryId, Option<String>)>> = self
            .by_base
            .keys()
            .filter_map(|base| {
                let active = self.in_force(base)?;
                active
                    .is_revised()
                    .then(|| (base.clone(), relations.prerequisite_edges(&active.revision)))
            })
            .collect();
        // A book that is inactive is not part of the effective graph either, so a chain never
        // reaches a row that its own book hides.
        relations.with_effective_revisions(
            |id| !visible(id) || rows.hides(id),
            &self.merge_targets,
            &prerequisites,
        )
    }
}

/// The cached effective state of one active-book selection: the revision view, the rows that
/// leave lists and counts, and the graph that chains walk.
#[derive(Debug, Clone, Default)]
pub struct EffectiveRevisions {
    view: RevisionView,
    rows: RevisionRows,
    graph: Rc<Relations>,
}

impl EffectiveRevisions {
    pub fn build(
        relations: &Relations,
        is_active: impl Fn(&EntryId) -> bool,
        source_of: impl Fn(&EntryId) -> Option<String>,
        merge_target_of: impl Fn(&EntryId) -> Option<EntryId>,
        visible: impl Fn(&EntryId) -> bool,
    ) -> Self {
        let mut view = RevisionView::build(relations, is_active, source_of);
        let targets: Vec<(EntryId, EntryId)> = view
            .merged_revisions()
            .filter_map(|active| {
                merge_target_of(&active.revision).map(|target| (active.base.clone(), target))
            })
            .collect();
        for (base, target) in targets {
            view.set_merge_target(&base, target);
        }
        let rows = view.rows(&visible);
        let graph = Rc::new(view.effective_graph(relations, &visible));
        Self { view, rows, graph }
    }

    pub fn view(&self) -> &RevisionView {
        &self.view
    }

    pub fn rows(&self) -> &RevisionRows {
        &self.rows
    }

    pub fn graph(&self) -> Rc<Relations> {
        Rc::clone(&self.graph)
    }

    /// True for a row that an active revision keeps out of lists and counts.
    pub fn hides(&self, id: &str) -> bool {
        self.rows.hides_str(id)
    }
}

/// The row changes for one active-book selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevisionRows {
    hidden_revisions: BTreeSet<EntryId>,
    hidden_bases: BTreeSet<EntryId>,
    /// The same ids as strings, for count predicates that receive an indexed id.
    hidden_strings: BTreeSet<String>,
    /// Base id to the revision's book id, for the row's muted line.
    effective_source: BTreeMap<EntryId, String>,
    /// Removed or merged feats per revision book, for the line under the list.
    removed_by_source: BTreeMap<String, usize>,
}

impl RevisionRows {
    pub fn hides(&self, id: &EntryId) -> bool {
        self.hidden_revisions.contains(id) || self.hidden_bases.contains(id)
    }

    /// The same answer for the string ids that the search index carries.
    pub fn hides_str(&self, id: &str) -> bool {
        self.hidden_strings.contains(id)
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
        aon: None,
        source_ids: base.source_ids.clone(),
        sources: revision.sources.clone(),
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

    fn graph(edges: &[(&str, RelationKind, &str, Option<&str>)]) -> Relations {
        Relations::new(RelationIndex {
            counts: BTreeMap::new(),
            edges: edges
                .iter()
                .map(|(source, kind, target, label)| {
                    RelationEdge(id(source), *kind, id(target), label.map(String::from))
                })
                .collect(),
        })
    }

    #[test]
    fn an_effective_graph_follows_the_revision_prerequisites() {
        let all = graph(&[
            ("c/base", RelationKind::Prerequisite, "c/old", None),
            ("c/revision", RelationKind::Prerequisite, "c/new", None),
            ("c/revision", RelationKind::Revision, "c/base", None),
            ("c/dependent", RelationKind::Prerequisite, "c/base", None),
        ]);
        let state =
            EffectiveRevisions::build(&all, |_| true, |_| Some("book".into()), |_| None, |_| true);
        let effective = state.graph();
        assert_eq!(
            effective.targets_of(&id("c/base"), RelationKind::Prerequisite),
            [&id("c/new")]
        );
        assert!(
            !effective
                .targets_of(&id("c/base"), RelationKind::Prerequisite)
                .contains(&&id("c/old"))
        );
        // The base still carries its dependents; the revision itself is not a chain row.
        assert_eq!(effective.leads_to(&id("c/base")), [&id("c/dependent")]);
        assert!(
            effective
                .targets_of(&id("c/revision"), RelationKind::Prerequisite)
                .is_empty()
        );
    }

    #[test]
    fn a_collision_keeps_the_printed_prerequisites() {
        let all = graph(&[
            ("c/base", RelationKind::Prerequisite, "c/old", None),
            ("c/revision-a", RelationKind::Prerequisite, "c/new-a", None),
            ("c/revision-b", RelationKind::Prerequisite, "c/new-b", None),
            ("c/revision-a", RelationKind::Revision, "c/base", None),
            ("c/revision-b", RelationKind::Revision, "c/base", None),
        ]);
        let state =
            EffectiveRevisions::build(&all, |_| true, |_| Some("book".into()), |_| None, |_| true);
        assert!(state.view().is_collision(&id("c/base")));
        assert!(state.view().in_force(&id("c/base")).is_none());
        let effective = state.graph();
        // Two revisions in force mean no substitution: the base keeps the edges it printed and
        // neither revision supplies prerequisites.
        assert_eq!(
            effective.targets_of(&id("c/base"), RelationKind::Prerequisite),
            [&id("c/old")]
        );
        for revision in ["c/revision-a", "c/revision-b"] {
            assert!(
                effective
                    .targets_of(&id(revision), RelationKind::Prerequisite)
                    .is_empty()
            );
        }
    }

    #[test]
    fn a_removed_or_merged_feat_leaves_the_effective_graph() {
        let all = graph(&[
            (
                "c/removed-revision",
                RelationKind::Revision,
                "c/removed",
                Some("removed"),
            ),
            (
                "c/merged-revision",
                RelationKind::Revision,
                "c/merged",
                Some("merged"),
            ),
            ("c/dependent", RelationKind::Prerequisite, "c/removed", None),
            (
                "c/merged-dependent",
                RelationKind::Prerequisite,
                "c/merged",
                None,
            ),
            ("c/target", RelationKind::Prerequisite, "c/root", None),
            ("c/merged", RelationKind::Prerequisite, "c/root", None),
        ]);
        let mut view = RevisionView::build(&all, |_| true, |_| Some("book".into()));
        view.set_merge_target(&id("c/merged"), id("c/target"));
        let rows = view.rows(|_| true);
        let effective = view.effective_graph(&all, |_| true);
        // Both the removed base and its revision leave, with every edge into the base.
        assert!(rows.hides(&id("c/removed")));
        assert!(rows.hides(&id("c/removed-revision")));
        assert!(effective.leads_to(&id("c/removed")).is_empty());
        assert!(
            effective
                .targets_of(&id("c/removed"), RelationKind::Prerequisite)
                .is_empty()
        );
        // The merged base leaves, and its dependents point at the feat that absorbed it.
        assert!(rows.hides(&id("c/merged")));
        assert_eq!(
            effective.targets_of(&id("c/merged-dependent"), RelationKind::Prerequisite),
            [&id("c/target")]
        );
    }

    #[test]
    fn the_effective_rows_name_the_ids_that_counts_must_skip() {
        let graph = relations(&[
            ("ref/feat/dodge/eitr", None, "ref/feat/dodge"),
            (
                "ref/feat/power-attack/eitr",
                Some("removed"),
                "ref/feat/power-attack",
            ),
        ]);
        let state = EffectiveRevisions::build(
            &graph,
            |_| true,
            |_| Some("book".into()),
            |_| None,
            |_| true,
        );
        assert!(state.hides("ref/feat/dodge/eitr"));
        assert!(!state.hides("ref/feat/dodge"));
        assert!(state.hides("ref/feat/power-attack"));
        assert!(state.hides("ref/feat/power-attack/eitr"));
    }

    #[test]
    fn effective_entry_keeps_the_original_identity_and_takes_the_revision_text() {
        let base = RuleEntry {
            id: id("ref/feat/dodge"),
            kind: cast_iron_schema::EntryKind::Feat,
            title: "Dodge".into(),
            aon: None,
            source_ids: vec!["core-rulebook".into()],
            sources: vec![],
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
