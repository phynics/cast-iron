//! Typed relations between entries: racial traits to races, archetypes to classes, and feat
//! prerequisites. Every relation comes from structure the upstream data states (nesting, the
//! url hierarchy, the `subtype` field) or from an exact name match. Nothing is guessed: what
//! cannot be tied to a target is reported as unplaced or unread.

use crate::build::{NestedLink, Plan, TypedInput};
use crate::prereq::{self, Reading};
use crate::slug::slug_segment;
use cast_iron_schema::{
    EntryId, EntryKind, PrerequisiteCoverage, Relation, RelationCoverage, RelationEdge,
    RelationIndex, RelationKind, RuleEntry, TypedFields, Unplaced, UnreadRequirement,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Report {
    pub index: RelationIndex,
    pub coverage: RelationCoverage,
}

pub fn apply(entries: &mut [RuleEntry], plan: &Plan, nested: &[NestedLink]) -> Report {
    let typed: BTreeMap<&EntryId, &TypedInput> = plan.typed.iter().map(|t| (&t.id, t)).collect();
    let position: BTreeMap<EntryId, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id.clone(), i))
        .collect();
    let mut found: BTreeMap<EntryId, Vec<Relation>> = BTreeMap::new();
    let mut coverage = RelationCoverage::default();

    // Parents of racial traits and archetypes.
    let races: Vec<&RuleEntry> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Race)
        .collect();
    let classes: Vec<&RuleEntry> = entries
        .iter()
        .filter(|e| matches!(e.kind, EntryKind::Class | EntryKind::PrestigeClass))
        .collect();
    let race_hosts: BTreeMap<&EntryId, &EntryId> = nested
        .iter()
        .rev()
        .filter(|link| {
            position
                .get(&link.host)
                .is_some_and(|&i| entries[i].kind == EntryKind::Race)
        })
        .map(|link| (&link.target, &link.host))
        .collect();
    for entry in entries.iter() {
        let Some(input) = typed.get(&entry.id) else {
            continue;
        };
        match entry.kind {
            EntryKind::RacialTrait => {
                let parent = race_hosts
                    .get(&entry.id)
                    .map(|id| (*id).clone())
                    .or_else(|| ancestor_by_url(input, &races, &typed))
                    .or_else(|| race_by_name(input, &races, &typed));
                match parent {
                    Some(parent) => {
                        let alternate = matches!(
                            &entry.fields,
                            TypedFields::RacialTrait(f) if f.alternate
                        );
                        found.entry(entry.id.clone()).or_default().push(Relation {
                            kind: RelationKind::Part,
                            target: parent,
                            label: alternate.then(|| "alternate".to_string()),
                        });
                    }
                    None => coverage.unplaced_parts.push(Unplaced {
                        entry: entry.id.to_string(),
                        reason: format!(
                            "racial trait `{}` is not nested in a race, not below a race url, and names no single race",
                            input.node.url.as_deref().unwrap_or_default()
                        ),
                    }),
                }
            }
            EntryKind::Archetype => {
                let parent = ancestor_by_url(input, &classes, &typed)
                    .or_else(|| class_by_subtype(input, &classes));
                match parent {
                    Some(parent) => found.entry(entry.id.clone()).or_default().push(Relation {
                        kind: RelationKind::Archetype,
                        target: parent,
                        label: None,
                    }),
                    None => coverage.unplaced_parts.push(Unplaced {
                        entry: entry.id.to_string(),
                        reason: format!(
                            "archetype subtype {:?} names no class entry",
                            input.node.subtype.as_deref().unwrap_or_default()
                        ),
                    }),
                }
            }
            _ => {}
        }
    }

    // Feat prerequisites.
    let resolved = feat_prerequisites(entries, &mut found, &mut coverage.prerequisites);

    // Write the relations back, sorted.
    for (id, mut relations) in found {
        relations.sort();
        relations.dedup();
        if let Some(&i) = position.get(&id) {
            entries[i].relations = relations;
        }
    }
    for (id, feats) in resolved {
        if let Some(&i) = position.get(&id)
            && let TypedFields::Feat(fields) = &mut entries[i].fields
        {
            fields.prerequisite_feats = feats;
        }
    }

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

/// The candidate whose upstream url is the longest proper prefix of the entry's url.
fn ancestor_by_url(
    input: &TypedInput,
    candidates: &[&RuleEntry],
    typed: &BTreeMap<&EntryId, &TypedInput>,
) -> Option<EntryId> {
    let url = input.node.url.as_deref()?;
    candidates
        .iter()
        .filter_map(|candidate| {
            let candidate_url = typed.get(&candidate.id)?.node.url.as_deref()?;
            let rest = url.strip_prefix(candidate_url)?;
            rest.starts_with('/')
                .then_some((candidate_url.len(), candidate))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then(b.1.id.cmp(&a.1.id)))
        .map(|(_, candidate)| candidate.id.clone())
}

/// A race named by the url segment after `Races`, such as `Gnomes` in
/// `pfsrd://Advanced Players Guide/Rules/Races/Gnomes/Alternate Racial Traits/Master Tinker`.
/// When several books define that race, the one with the standard (Core Rulebook) subtype wins.
fn race_by_name(
    input: &TypedInput,
    races: &[&RuleEntry],
    typed: &BTreeMap<&EntryId, &TypedInput>,
) -> Option<EntryId> {
    let url = input.node.url.as_deref()?;
    let mut segments = url.split('/');
    segments.find(|s| *s == "Races")?;
    let wanted = slug_segment(segments.next()?);
    let named: Vec<&&RuleEntry> = races
        .iter()
        .filter(|race| slug_segment(&race.title) == wanted)
        .collect();
    match named.as_slice() {
        [only] => Some(only.id.clone()),
        [] => None,
        many => {
            let standard: Vec<_> = many
                .iter()
                .filter(|race| {
                    typed.get(&race.id).and_then(|t| t.node.subtype.as_deref())
                        == Some("standard_race")
                })
                .collect();
            match standard.as_slice() {
                [only] => Some(only.id.clone()),
                _ => None,
            }
        }
    }
}

/// The class whose title equals the archetype's `subtype`, preferring a plain class over a
/// prestige class.
fn class_by_subtype(input: &TypedInput, classes: &[&RuleEntry]) -> Option<EntryId> {
    let wanted = input.node.subtype.as_deref()?.trim().to_lowercase();
    classes
        .iter()
        .filter(|class| class.title.to_lowercase() == wanted)
        .min_by(|a, b| {
            (a.kind == EntryKind::PrestigeClass)
                .cmp(&(b.kind == EntryKind::PrestigeClass))
                .then(a.id.cmp(&b.id))
        })
        .map(|class| class.id.clone())
}

/// How a prerequisite name resolves against the feat entries.
#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    Feat(EntryId),
    Ambiguous,
    None,
}

/// Feats by normalized title, for exact-name resolution.
pub struct FeatIndex {
    by_key: BTreeMap<String, Vec<(EntryId, String)>>,
}

impl FeatIndex {
    pub fn new<'a>(feats: impl Iterator<Item = &'a RuleEntry>) -> Self {
        let mut by_key: BTreeMap<String, Vec<(EntryId, String)>> = BTreeMap::new();
        for feat in feats {
            let source = feat.source_ids.first().cloned().unwrap_or_default();
            for key in [
                slug_segment(&feat.title),
                slug_segment(&strip_parentheses(&feat.title)),
            ] {
                let list = by_key.entry(key).or_default();
                if !list.iter().any(|(id, _)| *id == feat.id) {
                    list.push((feat.id.clone(), source.clone()));
                }
            }
        }
        Self { by_key }
    }

    /// Resolves a name as written. A name with a parenthesis, such as
    /// `Weapon Focus (longsword)`, also tries the name without it. Several feats with one
    /// name resolve to the one in `source`, or else to none.
    pub fn resolve(&self, name: &str, source: &str) -> Resolution {
        for key in [slug_segment(name), slug_segment(&strip_parentheses(name))] {
            if key.is_empty() {
                continue;
            }
            match self.by_key.get(&key).map(Vec::as_slice) {
                Some([(id, _)]) => return Resolution::Feat(id.clone()),
                Some(many) => {
                    let same: Vec<_> = many.iter().filter(|(_, s)| s == source).collect();
                    return match same.as_slice() {
                        [(id, _)] => Resolution::Feat(id.clone()),
                        _ => Resolution::Ambiguous,
                    };
                }
                None => {}
            }
        }
        Resolution::None
    }
}

fn strip_parentheses(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Reads every feat's prerequisite wording. Fills `found` with `Prerequisite` relations and
/// returns the resolved feats per feat.
fn feat_prerequisites(
    entries: &[RuleEntry],
    found: &mut BTreeMap<EntryId, Vec<Relation>>,
    coverage: &mut PrerequisiteCoverage,
) -> BTreeMap<EntryId, Vec<EntryId>> {
    let index = FeatIndex::new(entries.iter().filter(|e| e.kind == EntryKind::Feat));
    let mut resolved: BTreeMap<EntryId, Vec<EntryId>> = BTreeMap::new();
    for feat in entries.iter().filter(|e| e.kind == EntryKind::Feat) {
        coverage.feats += 1;
        let TypedFields::Feat(fields) = &feat.fields else {
            continue;
        };
        let Some(text) = fields.prerequisites.as_deref() else {
            continue;
        };
        coverage.feats_with_text += 1;
        let source = feat
            .source_ids
            .first()
            .map(String::as_str)
            .unwrap_or_default();
        let mut feats: BTreeSet<EntryId> = BTreeSet::new();
        for requirement in prereq::parse(text) {
            coverage.requirements += 1;
            let reading = match &requirement.reading {
                Reading::AbilityScore => "ability_score",
                Reading::BaseAttackBonus => "base_attack_bonus",
                Reading::SkillRanks => "skill_ranks",
                Reading::Level => "level",
                Reading::ClassFeature => "class_feature",
                Reading::RacialTrait => "racial_trait",
                Reading::Race => "race",
                Reading::Candidate(name) => {
                    let targets = resolve_candidate(&index, name, source);
                    if targets.is_empty() {
                        if matches!(index.resolve(name, source), Resolution::Ambiguous) {
                            coverage.ambiguous += 1;
                        }
                        coverage.unread.push(UnreadRequirement {
                            feat: feat.id.clone(),
                            text: requirement.text.clone(),
                        });
                        "other"
                    } else {
                        for target in targets {
                            if target == feat.id {
                                continue;
                            }
                            feats.insert(target.clone());
                            found.entry(feat.id.clone()).or_default().push(Relation {
                                kind: RelationKind::Prerequisite,
                                target,
                                label: requirement.alternative.then(|| "or".to_string()),
                            });
                        }
                        "feat"
                    }
                }
            };
            *coverage.by_reading.entry(reading.to_string()).or_default() += 1;
        }
        if !feats.is_empty() {
            coverage.feats_with_resolved_feat += 1;
            resolved.insert(feat.id.clone(), feats.into_iter().collect());
        }
    }
    coverage
        .unread
        .sort_by(|a, b| a.feat.cmp(&b.feat).then(a.text.cmp(&b.text)));
    resolved
}

/// The feats a candidate name stands for. `A with selected weapon` is `A`. `A and B` counts
/// when it does not resolve whole and both halves do.
fn resolve_candidate(index: &FeatIndex, name: &str, source: &str) -> Vec<EntryId> {
    if let Resolution::Feat(id) = index.resolve(name, source) {
        return vec![id];
    }
    if let Some((head, _)) = name.split_once(" with ")
        && let Resolution::Feat(id) = index.resolve(head, source)
    {
        return vec![id];
    }
    let halves: Vec<&str> = name.split(" and ").map(str::trim).collect();
    if halves.len() > 1 {
        let all: Vec<EntryId> = halves
            .iter()
            .filter_map(|half| match index.resolve(half, source) {
                Resolution::Feat(id) => Some(id),
                _ => None,
            })
            .collect();
        if all.len() == halves.len() {
            return all;
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{FeatFields, Provenance};

    fn feat(id: &str, title: &str, source: &str, prerequisites: Option<&str>) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Feat,
            title: title.into(),
            source_ids: vec![source.into()],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::Feat(FeatFields {
                prerequisites: prerequisites.map(str::to_string),
                ..FeatFields::default()
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

    #[test]
    fn names_resolve_exactly_and_ignore_punctuation_and_parentheses() {
        let feats = [
            feat("cr/feats/point-blank-shot", "Point-Blank Shot", "cr", None),
            feat("cr/feats/weapon-focus", "Weapon Focus", "cr", None),
            feat("cr/feats/heros-fortune", "Hero's Fortune", "cr", None),
        ];
        let index = FeatIndex::new(feats.iter());
        assert_eq!(
            index.resolve("Point Blank Shot", "cr"),
            Resolution::Feat(feats[0].id.clone())
        );
        assert_eq!(
            index.resolve("Weapon Focus (longsword)", "cr"),
            Resolution::Feat(feats[1].id.clone())
        );
        assert_eq!(
            index.resolve("Hero's Fortune", "cr"),
            Resolution::Feat(feats[2].id.clone())
        );
        assert_eq!(index.resolve("Weapon Mastery", "cr"), Resolution::None);
    }

    #[test]
    fn a_name_shared_by_two_books_resolves_only_within_the_same_book() {
        // Spider Step exists twice upstream.
        let feats = [
            feat("a/feats/spider-step", "Spider Step", "a", None),
            feat("b/feats/spider-step", "Spider Step", "b", None),
        ];
        let index = FeatIndex::new(feats.iter());
        assert_eq!(
            index.resolve("Spider Step", "b"),
            Resolution::Feat(feats[1].id.clone())
        );
        assert_eq!(index.resolve("Spider Step", "c"), Resolution::Ambiguous);
    }

    #[test]
    fn prerequisite_chains_become_relations_and_feat_lists() {
        // Greater Drag, Advanced Race Guide.
        let entries = vec![
            feat("arg/feats/improved-drag", "Improved Drag", "arg", None),
            feat("arg/feats/power-attack", "Power Attack", "arg", None),
            feat(
                "arg/feats/greater-drag",
                "Greater Drag",
                "arg",
                Some("Str 13, Improved Drag, Power Attack, base attack bonus +6."),
            ),
            feat(
                "arg/feats/marked",
                "Marked",
                "arg",
                Some("Power Attack, Dodge, or Cleave."),
            ),
        ];
        let mut found = BTreeMap::new();
        let mut coverage = PrerequisiteCoverage::default();
        let resolved = feat_prerequisites(&entries, &mut found, &mut coverage);
        let greater = EntryId::parse("arg/feats/greater-drag").unwrap();
        assert_eq!(
            resolved[&greater],
            [
                EntryId::parse("arg/feats/improved-drag").unwrap(),
                EntryId::parse("arg/feats/power-attack").unwrap()
            ]
        );
        assert_eq!(coverage.by_reading["ability_score"], 1);
        assert_eq!(coverage.by_reading["base_attack_bonus"], 1);
        assert_eq!(coverage.by_reading["feat"], 3);
        assert_eq!(coverage.by_reading["other"], 2);
        assert_eq!(
            coverage.unread.len(),
            2,
            "Dodge and Cleave are not in this library"
        );
        let marked = &found[&EntryId::parse("arg/feats/marked").unwrap()];
        assert_eq!(marked.len(), 1);
        assert_eq!(marked[0].label.as_deref(), Some("or"));
    }

    #[test]
    fn a_pair_joined_by_and_resolves_when_both_halves_do() {
        let feats = [
            feat(
                "a/feats/improved-two-weapon-fighting",
                "Improved Two-Weapon Fighting",
                "a",
                None,
            ),
            feat(
                "a/feats/two-weapon-fighting",
                "Two-Weapon Fighting",
                "a",
                None,
            ),
        ];
        let index = FeatIndex::new(feats.iter());
        let both = resolve_candidate(
            &index,
            "Improved Two-Weapon Fighting and Two-Weapon Fighting",
            "a",
        );
        assert_eq!(both.len(), 2);
        assert!(resolve_candidate(&index, "Dodge and Mobility", "a").is_empty());
    }

    #[test]
    fn a_name_with_a_selection_clause_resolves_to_the_named_feat() {
        // Greater Weapon Focus, Core Rulebook: `Weapon Focus with selected weapon`.
        let feats = [feat("cr/feats/weapon-focus", "Weapon Focus", "cr", None)];
        let index = FeatIndex::new(feats.iter());
        assert_eq!(
            resolve_candidate(&index, "Weapon Focus with selected weapon", "cr"),
            [feats[0].id.clone()]
        );
        assert!(resolve_candidate(&index, "Proficiency with selected weapon", "cr").is_empty());
    }
}
