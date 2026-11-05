//! Typed relations between entries: racial traits to races, archetypes to classes, and feat
//! prerequisites. Every relation comes from structure the upstream data states (nesting, the
//! url hierarchy, the `subtype` field) or from an exact name match. Nothing is guessed: what
//! cannot be tied to a target is reported as unplaced or unread.

use crate::build::{NestedLink, Plan, TypedInput};
use crate::prereq::{self, Reading};
use crate::slug::slug_segment;
use crate::text::inline_text;
use cast_iron_schema::{
    Block, EntryId, EntryKind, Inline, PrerequisiteCoverage, Relation, RelationCoverage,
    RelationEdge, RelationIndex, RelationKind, RuleEntry, TypedFields, Unplaced, UnreadRequirement,
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

    // Feat prerequisites. The PSRD prerequisite text is plain; the canonical text can
    // carry upstream encoded links, which its own caller makes visible first.
    let resolved = feat_prerequisites(entries, &mut found, &mut coverage.prerequisites);

    finish(entries, found, resolved, coverage)
}

/// Builds the same typed relations over the canonical Pf Data entries.
///
/// The inputs are only [`RuleEntry`] values and the parent-to-child links the
/// converter synthesized, so nothing here depends on the PSRD build plan or its urls.
/// Every parent comes from structure the conversion already recorded:
///
/// * a racial trait's `parent` (or the race that nests it),
/// * an archetype's `ref/arc-<class>/...` family, or the class its own body names in
///   the `This is an archetype of ...` statement,
/// * a feat's prerequisite wording, read with [`crate::prereq`] after encoded links
///   are reduced to their visible text.
///
/// What cannot be tied to an entry is reported in coverage, not guessed.
pub fn apply_canonical(entries: &mut [RuleEntry], nested: &[NestedLink]) -> Report {
    let race_ids: BTreeSet<EntryId> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Race)
        .map(|e| e.id.clone())
        .collect();
    let class_ids: BTreeSet<EntryId> = entries
        .iter()
        .filter(|e| matches!(e.kind, EntryKind::Class | EntryKind::PrestigeClass))
        .map(|e| e.id.clone())
        .collect();
    let race_hosts: BTreeMap<&EntryId, &EntryId> = nested
        .iter()
        .rev()
        .filter(|link| race_ids.contains(&link.host))
        .map(|link| (&link.target, &link.host))
        .collect();
    let mut found: BTreeMap<EntryId, Vec<Relation>> = BTreeMap::new();
    let mut coverage = RelationCoverage::default();

    for entry in entries.iter() {
        match entry.kind {
            EntryKind::RacialTrait => {
                let parent = entry
                    .parent
                    .as_ref()
                    .filter(|parent| race_ids.contains(*parent))
                    .cloned()
                    .or_else(|| race_hosts.get(&entry.id).map(|id| (*id).clone()));
                match parent {
                    Some(parent) => {
                        let alternate =
                            matches!(
                                &entry.fields,
                                TypedFields::RacialTrait(f) if f.alternate
                            ) || entry.id.as_str().contains("/alternate-racial-traits/");
                        found.entry(entry.id.clone()).or_default().push(Relation {
                            kind: RelationKind::Part,
                            target: parent,
                            label: alternate.then(|| "alternate".to_string()),
                        });
                    }
                    None => coverage.unplaced_parts.push(Unplaced {
                        entry: entry.id.to_string(),
                        reason: format!(
                            "racial trait `{}` has no race parent and no race nests it",
                            entry.id
                        ),
                    }),
                }
            }
            EntryKind::Archetype => match canonical_archetype_parent(entry, &class_ids) {
                Some(parent) => found.entry(entry.id.clone()).or_default().push(Relation {
                    kind: RelationKind::Archetype,
                    target: parent,
                    label: None,
                }),
                None => coverage.unplaced_parts.push(Unplaced {
                    entry: entry.id.to_string(),
                    reason: canonical_unplaced_reason(entry),
                }),
            },
            _ => {}
        }
    }

    let resolved = feat_prerequisites_with(
        entries,
        &mut found,
        &mut coverage.prerequisites,
        visible_encoded_links,
    );

    finish(entries, found, resolved, coverage)
}

/// Writes the found relations and the prerequisite-feat lists back, then builds the
/// sorted index. Both source paths share this step so their output shape cannot drift.
fn finish(
    entries: &mut [RuleEntry],
    found: BTreeMap<EntryId, Vec<Relation>>,
    resolved: BTreeMap<EntryId, Vec<EntryId>>,
    mut coverage: RelationCoverage,
) -> Report {
    let position: BTreeMap<EntryId, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id.clone(), i))
        .collect();
    // Rebuild the kinds this pass owns; leave any other relation on the entry alone.
    for entry in entries.iter_mut() {
        let own = match entry.kind {
            EntryKind::RacialTrait => Some(RelationKind::Part),
            EntryKind::Archetype => Some(RelationKind::Archetype),
            EntryKind::Feat => Some(RelationKind::Prerequisite),
            _ => None,
        };
        if let Some(kind) = own {
            entry.relations.retain(|relation| relation.kind != kind);
        }
    }
    for (id, mut relations) in found {
        // The schema forbids two relations of one kind to one target. A target that is
        // both required and named again as an alternative keeps the unlabelled edge
        // (sorting puts `None` before `Some`).
        relations.sort();
        relations.dedup_by(|a, b| a.kind == b.kind && a.target == b.target);
        if let Some(&i) = position.get(&id) {
            entries[i].relations.extend(relations);
            entries[i].relations.sort();
            entries[i]
                .relations
                .dedup_by(|a, b| a.kind == b.kind && a.target == b.target);
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

/// The class an archetype belongs to, from evidence the converter preserved.
fn canonical_archetype_parent(entry: &RuleEntry, classes: &BTreeSet<EntryId>) -> Option<EntryId> {
    // The route family is the exact protocol: `arc-fighter` names `ref/class/fighter`.
    if let Some(family) = entry.id.as_str().split('/').nth(1)
        && let Some(key) = family.strip_prefix("arc-")
        && let Ok(target) = EntryId::parse(format!("ref/class/{key}"))
        && classes.contains(&target)
    {
        return Some(target);
    }
    // A prestige-class archetype states its parent in its own body and links it.
    if body_states_archetype_of(&entry.body) {
        for link in &entry.links {
            if classes.contains(&link.target) {
                return Some(link.target.clone());
            }
        }
    }
    None
}

fn canonical_unplaced_reason(entry: &RuleEntry) -> String {
    if let Some(family) = entry.id.as_str().split('/').nth(1)
        && let Some(key) = family.strip_prefix("arc-")
    {
        return format!(
            "archetype `{}` names class `ref/class/{key}`, which is not in the entry set",
            entry.id
        );
    }
    format!(
        "archetype `{}` states no parent class and links no class entry",
        entry.id
    )
}

/// True when a paragraph states that the record is an archetype of a class.
fn body_states_archetype_of(blocks: &[Block]) -> bool {
    fn inlines(inlines: &[Inline]) -> bool {
        inline_text(inlines)
            .to_lowercase()
            .contains("is an archetype of")
    }
    blocks.iter().any(|block| match block {
        Block::Heading { content, .. } | Block::Paragraph { content, .. } => inlines(content),
        Block::List { items, .. } => items.iter().any(|item| body_states_archetype_of(item)),
        Block::Quote { content } => body_states_archetype_of(content),
        _ => false,
    })
}

/// Replaces every upstream encoded link `‹protocol/Name›` with its visible name, the
/// same text the renderer shows. The canonical prerequisite field keeps the encoded
/// form; relation reading must not treat the markup as part of a feat name.
fn visible_encoded_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('‹') {
        out.push_str(&rest[..start]);
        let after = &rest[start + '‹'.len_utf8()..];
        let Some(end) = after.find('›') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..end];
        let (_, display) = crate::pf_markdown::link_parts(inner);
        if display.is_empty() {
            out.push_str(&rest[start..start + '‹'.len_utf8() + end + '›'.len_utf8()]);
        } else {
            out.push_str(&display);
        }
        rest = &after[end + '›'.len_utf8()..];
    }
    out.push_str(rest);
    out
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
    feat_prerequisites_with(entries, found, coverage, str::to_string)
}

/// The shared prerequisite reader. `normalize` turns the stored wording into the text
/// that [`crate::prereq`] reads; the PSRD build passes the text unchanged.
fn feat_prerequisites_with(
    entries: &[RuleEntry],
    found: &mut BTreeMap<EntryId, Vec<Relation>>,
    coverage: &mut PrerequisiteCoverage,
    normalize: impl Fn(&str) -> String,
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
        let text = normalize(text);
        let source = feat
            .source_ids
            .first()
            .map(String::as_str)
            .unwrap_or_default();
        let mut feats: BTreeSet<EntryId> = BTreeSet::new();
        for requirement in prereq::parse(&text) {
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

    fn canonical_entry(id: &str, kind: EntryKind, title: &str, parent: Option<&str>) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: title.into(),
            source_ids: vec!["core".into()],
            section: vec![],
            parent: parent.map(|id| EntryId::parse(id).unwrap()),
            body: vec![],
            fields: TypedFields::None,
            tags: vec![],
            links: vec![],
            provenance: vec![Provenance {
                dataset: "pathfinder-data-1-e".into(),
                record: title.into(),
                path: "json/test.json".into(),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: vec![],
        }
    }

    fn canonical_feat(id: &str, title: &str, prerequisites: Option<&str>) -> RuleEntry {
        let mut entry = canonical_entry(id, EntryKind::Feat, title, None);
        entry.fields = TypedFields::Feat(FeatFields {
            prerequisites: prerequisites.map(str::to_string),
            ..FeatFields::default()
        });
        entry
    }

    #[test]
    fn canonical_racial_traits_link_to_their_race_by_parent_or_nesting() {
        let race = canonical_entry("ref/race/dwarf", EntryKind::Race, "Dwarf", None);
        let mut standard = canonical_entry(
            "ref/racial-trait/dwarf/stonecunning",
            EntryKind::RacialTrait,
            "Stonecunning",
            Some("ref/race/dwarf"),
        );
        standard.fields =
            TypedFields::RacialTrait(cast_iron_schema::RacialTraitFields { alternate: false });
        let mut alternate = canonical_entry(
            "ref/racial-trait/dwarf/alternate-racial-traits/ironhide",
            EntryKind::RacialTrait,
            "Ironhide",
            None,
        );
        alternate.fields =
            TypedFields::RacialTrait(cast_iron_schema::RacialTraitFields { alternate: true });
        let mut entries = vec![race, standard, alternate];
        let nested = [NestedLink {
            host: EntryId::parse("ref/race/dwarf").unwrap(),
            target: EntryId::parse("ref/racial-trait/dwarf/alternate-racial-traits/ironhide")
                .unwrap(),
            trail: vec![],
            child: false,
        }];
        let report = apply_canonical(&mut entries, &nested);

        let dwarf = EntryId::parse("ref/race/dwarf").unwrap();
        let standard = EntryId::parse("ref/racial-trait/dwarf/stonecunning").unwrap();
        let alternate =
            EntryId::parse("ref/racial-trait/dwarf/alternate-racial-traits/ironhide").unwrap();
        assert_eq!(report.coverage.edges.get("part"), Some(&2));
        assert!(report.index.edges.contains(&RelationEdge(
            standard.clone(),
            RelationKind::Part,
            dwarf.clone()
        )));
        assert!(report.index.edges.contains(&RelationEdge(
            alternate.clone(),
            RelationKind::Part,
            dwarf.clone()
        )));
        assert!(report.coverage.unplaced_parts.is_empty());
        let standard_label = entries[1]
            .relations
            .iter()
            .find(|r| r.kind == RelationKind::Part)
            .and_then(|r| r.label.as_deref());
        let alternate_label = entries[2]
            .relations
            .iter()
            .find(|r| r.kind == RelationKind::Part)
            .and_then(|r| r.label.as_deref());
        assert_eq!(standard_label, None);
        assert_eq!(alternate_label, Some("alternate"));
    }

    #[test]
    fn canonical_archetypes_resolve_by_protocol_family_and_stated_parent() {
        let class = canonical_entry("ref/class/fighter", EntryKind::Class, "Fighter", None);
        let arc = canonical_entry(
            "ref/arc-fighter/aerial-assaulter",
            EntryKind::Archetype,
            "Aerial Assaulter",
            None,
        );
        let prestige = canonical_entry(
            "ref/pclass/arcane-archer",
            EntryKind::PrestigeClass,
            "Arcane Archer",
            None,
        );
        let mut stated = canonical_entry(
            "ref/pclass/deadeye-devotee-arcane-archer",
            EntryKind::Archetype,
            "Deadeye Devotee (Arcane Archer)",
            None,
        );
        let target = prestige.id.clone();
        stated.body = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![Inline::Strong {
                c: vec![
                    Inline::text("This is an archetype of the "),
                    Inline::Link {
                        target: target.clone(),
                        anchor: None,
                        c: vec![Inline::text("Arcane Archer")],
                    },
                    Inline::text("."),
                ],
            }],
        }];
        stated.links = vec![cast_iron_schema::EntryReference {
            target: target.clone(),
            anchor: None,
        }];
        let orphan = canonical_entry(
            "ref/arc-companion/aberrant-companion",
            EntryKind::Archetype,
            "Aberrant Companion",
            None,
        );
        let mut entries = vec![class, arc, prestige, stated, orphan];
        let report = apply_canonical(&mut entries, &[]);

        assert!(report.index.edges.contains(&RelationEdge(
            EntryId::parse("ref/arc-fighter/aerial-assaulter").unwrap(),
            RelationKind::Archetype,
            EntryId::parse("ref/class/fighter").unwrap()
        )));
        assert!(report.index.edges.contains(&RelationEdge(
            EntryId::parse("ref/pclass/deadeye-devotee-arcane-archer").unwrap(),
            RelationKind::Archetype,
            target
        )));
        let unplaced = &report.coverage.unplaced_parts;
        assert_eq!(unplaced.len(), 1);
        assert_eq!(unplaced[0].entry, "ref/arc-companion/aberrant-companion");
        assert!(
            unplaced[0].reason.contains("ref/class/companion"),
            "{}",
            unplaced[0].reason
        );
    }

    #[test]
    fn canonical_feat_prerequisites_read_visible_names_and_build_a_chain() {
        let entries = vec![
            canonical_feat("ref/feat/iron-will", "Iron Will", None),
            canonical_feat(
                "ref/feat/improved-iron-will",
                "Improved Iron Will",
                Some("‹feat/Iron Will›."),
            ),
            canonical_feat(
                "ref/feat/clarity-of-pain",
                "Clarity of Pain",
                Some("Con 13, ‹feat/Improved Iron Will›, ‹feat/Iron Will›."),
            ),
            canonical_feat(
                "ref/feat/exorcising-mutilation",
                "Exorcising Mutilation",
                Some(
                    "Con 15, ‹feat/Clarity of Pain›, ‹feat/Improved Iron Will›, ‹feat/Iron Will›.",
                ),
            ),
        ];
        let mut entries = entries;
        let report = apply_canonical(&mut entries, &[]);

        let edge = |source: &str, target: &str| {
            RelationEdge(
                EntryId::parse(source).unwrap(),
                RelationKind::Prerequisite,
                EntryId::parse(target).unwrap(),
            )
        };
        assert!(
            report
                .index
                .edges
                .contains(&edge("ref/feat/improved-iron-will", "ref/feat/iron-will"))
        );
        assert!(report.index.edges.contains(&edge(
            "ref/feat/clarity-of-pain",
            "ref/feat/improved-iron-will"
        )));
        assert!(
            report
                .index
                .edges
                .contains(&edge("ref/feat/clarity-of-pain", "ref/feat/iron-will"))
        );
        assert!(report.index.edges.contains(&edge(
            "ref/feat/exorcising-mutilation",
            "ref/feat/clarity-of-pain"
        )));
        assert_eq!(report.coverage.prerequisites.by_reading["feat"], 6);
        assert_eq!(report.coverage.prerequisites.by_reading["ability_score"], 2);
        assert!(report.coverage.prerequisites.unread.is_empty());
        let clarity = &entries[2];
        assert_eq!(
            clarity.fields,
            TypedFields::Feat(FeatFields {
                prerequisites: Some("Con 13, ‹feat/Improved Iron Will›, ‹feat/Iron Will›.".into()),
                prerequisite_feats: vec![
                    EntryId::parse("ref/feat/improved-iron-will").unwrap(),
                    EntryId::parse("ref/feat/iron-will").unwrap(),
                ],
                ..FeatFields::default()
            })
        );
    }

    #[test]
    fn a_target_named_twice_keeps_one_unlabelled_relation() {
        // Totemic Disciple, Ultimate Wilderness: the feat is required, then named again
        // inside a later prose clause. The schema allows one edge per kind and target.
        let mut entries = vec![
            canonical_feat("ref/feat/totemic-initiate", "Totemic Initiate", None),
            canonical_feat(
                "ref/feat/totemic-disciple",
                "Totemic Disciple",
                Some(
                    "Str 15; Dex 13; ‹feat/Totemic Initiate›; base attack bonus +9; no barbarian totem rage powers except those chosen with this feat or Totemic Initiate; nonlawful alignment.",
                ),
            ),
        ];
        let report = apply_canonical(&mut entries, &[]);
        let disciple = &entries[1].relations;
        assert_eq!(
            disciple
                .iter()
                .filter(|r| r.target.as_str() == "ref/feat/totemic-initiate")
                .count(),
            1
        );
        assert_eq!(
            disciple
                .iter()
                .find(|r| r.target.as_str() == "ref/feat/totemic-initiate")
                .and_then(|r| r.label.as_deref()),
            None
        );
        assert_eq!(report.coverage.edges.get("prerequisite"), Some(&1));
    }

    #[test]
    fn canonical_relations_are_deterministic() {
        let base = vec![
            canonical_entry("ref/race/dwarf", EntryKind::Race, "Dwarf", None),
            {
                let mut trait_ = canonical_entry(
                    "ref/racial-trait/dwarf/stonecunning",
                    EntryKind::RacialTrait,
                    "Stonecunning",
                    Some("ref/race/dwarf"),
                );
                trait_.fields = TypedFields::RacialTrait(cast_iron_schema::RacialTraitFields {
                    alternate: false,
                });
                trait_
            },
            canonical_entry("ref/class/fighter", EntryKind::Class, "Fighter", None),
            canonical_entry(
                "ref/arc-fighter/aerial-assaulter",
                EntryKind::Archetype,
                "Aerial Assaulter",
                None,
            ),
            canonical_feat("ref/feat/iron-will", "Iron Will", None),
            canonical_feat(
                "ref/feat/improved-iron-will",
                "Improved Iron Will",
                Some("‹feat/Iron Will›."),
            ),
        ];
        let mut first = base.clone();
        let mut second = base;
        let first_report = apply_canonical(&mut first, &[]);
        let second_report = apply_canonical(&mut second, &[]);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first_report.index, second_report.index);
        assert_eq!(first_report.coverage, second_report.coverage);
    }

    /// The full pinned corpus is large; this test skips when the cache is absent, like the
    /// canonical conversion tests.
    #[test]
    fn real_corpus_relations_place_core_traits_a_fighter_archetype_and_a_feat_chain() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dataset = root.join(".cache/sources/pathfinder-data-1-e");
        if !dataset.join("json").is_dir() {
            eprintln!("skipping: the pinned primary source is not present");
            return;
        }
        let inventory = crate::pf_data::inventory(
            &dataset,
            &root.join("sources/pathfinder-data-1-e/source.json"),
        )
        .unwrap();
        let allowlist = crate::pf_rules::RuleAllowlist::load_optional(
            &root.join("sources/pathfinder-data-1-e/derived-rules.json"),
        )
        .unwrap();
        let converted = crate::canonical::convert_corpus(&inventory, &allowlist).unwrap();
        let base = converted.entries;
        let mut first = base.clone();
        let report = apply_canonical(&mut first, &converted.links);

        let id = |text: &str| EntryId::parse(text).unwrap();
        let part_targets: BTreeSet<&EntryId> = report
            .index
            .edges
            .iter()
            .filter(|RelationEdge(_, kind, target)| {
                *kind == RelationKind::Part && *target == id("ref/race/dwarf")
            })
            .map(|RelationEdge(source, _, _)| source)
            .collect();
        assert!(
            part_targets.contains(&id("ref/racial-trait/dwarf/stonecunning")),
            "Dwarf's standard traits: {part_targets:?}"
        );
        assert!(
            part_targets.len() >= 10,
            "Dwarf should have its standard traits, found {}",
            part_targets.len()
        );
        assert!(report.index.edges.contains(&RelationEdge(
            id("ref/arc-fighter/aerial-assaulter"),
            RelationKind::Archetype,
            id("ref/class/fighter")
        )));
        let chain = [
            ("ref/feat/improved-iron-will", "ref/feat/iron-will"),
            ("ref/feat/clarity-of-pain", "ref/feat/improved-iron-will"),
            ("ref/feat/clarity-of-pain", "ref/feat/iron-will"),
            ("ref/feat/exorcising-mutilation", "ref/feat/clarity-of-pain"),
        ];
        for (source, target) in chain {
            assert!(
                report.index.edges.contains(&RelationEdge(
                    id(source),
                    RelationKind::Prerequisite,
                    id(target)
                )),
                "{source} should require {target}"
            );
        }
        assert!(
            !report
                .index
                .edges
                .iter()
                .any(|RelationEdge(source, _, target)| source == target),
            "no entry may require or contain itself"
        );

        let mut second = base;
        let again = apply_canonical(&mut second, &converted.links);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(report.index, again.index);
        assert_eq!(report.coverage, again.coverage);
    }
}
