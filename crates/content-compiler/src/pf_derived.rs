//! Low-risk derived records for the Pf Data cutover.
//!
//! Two families are fully mechanical in the pinned corpus, so the compiler derives
//! them instead of waiting for a per-record review:
//!
//! * standard racial traits of the seven Core Rulebook races, split out of their
//!   `::rtraits` directive;
//! * inline `::aff` affliction blocks in monster and rule records.
//!
//! Canonical IDs follow the cutover contract:
//!
//! ```text
//! ref/racial-trait/{race-slug}/{trait-slug}
//! ref/affliction/{host-slug}/{affliction-slug}
//! ```
//!
//! Both use the existing [`slug_segment`] contract. A collision is never resolved
//! with a suffix: the later derivation is rejected and reported, so the output is
//! always injective.

use crate::pf_data::InputRecord;
use crate::pf_directives;
use crate::slug::slug_segment;
use cast_iron_schema::{Block, EntryId, EntryKind};
use std::collections::BTreeMap;

/// The Pf Data title for the PF1e Core Rulebook. The seven core races are the
/// records that name it as a publication.
pub const CORE_RULEBOOK: &str = "PRPG Core Rulebook";

const RACIAL_TRAIT_PREFIX: &str = "ref/racial-trait";
const AFFLICTION_PREFIX: &str = "ref/affliction";

/// The derived families this layer produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DerivedKind {
    RacialTrait,
    Affliction,
}

impl DerivedKind {
    pub fn slug(self) -> &'static str {
        match self {
            Self::RacialTrait => "racial-trait",
            Self::Affliction => "affliction",
        }
    }

    pub fn kind(self) -> EntryKind {
        match self {
            Self::RacialTrait => EntryKind::RacialTrait,
            Self::Affliction => EntryKind::Affliction,
        }
    }
}

/// One derived record before it is joined to its converted parent.
#[derive(Debug, Clone)]
pub struct Derivation {
    pub id: EntryId,
    pub title: String,
    pub kind: DerivedKind,
    /// Canonical ID of the containing Pf Data record.
    pub parent: EntryId,
    /// `path#key` of the containing record, used to find the converted parent.
    pub parent_identity: String,
    /// Provenance `record`: `<key>|<child-kind>:<slug>`.
    pub provenance_record: String,
    /// Body Markdown lines, before typed rendering.
    pub body_lines: Vec<String>,
    /// Display name used for the link left in the parent body.
    pub name: String,
    /// Publication slugs inherited from the upstream record.
    pub source_ids: Vec<String>,
    /// Upstream file path, for provenance.
    pub path: String,
}

/// A derivation the compiler refused to make. Every rejection is reported.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Rejection {
    pub identity: String,
    pub problem: String,
}

/// Derive every eligible record. Deterministic from a sorted input slice.
pub fn plan(records: &[InputRecord], rejections: &mut Vec<Rejection>) -> Vec<Derivation> {
    let mut derivations = Vec::new();
    let mut claimed: BTreeMap<String, String> = BTreeMap::new();
    for record in records {
        if record.shape == "redirect" {
            // A redirect is an alias of another record; deriving children under the
            // alias key would duplicate the destination's children.
            continue;
        }
        let identity = format!("{}#{}", record.path, record.key);
        if is_core_race(record) {
            plan_racial_traits(
                record,
                &identity,
                &mut claimed,
                &mut derivations,
                rejections,
            );
        }
        if is_affliction_host(&record.path) {
            plan_afflictions(
                record,
                &identity,
                &mut claimed,
                &mut derivations,
                rejections,
            );
        }
    }
    derivations.sort_by(|a, b| a.id.cmp(&b.id));
    derivations
}

fn plan_racial_traits(
    record: &InputRecord,
    identity: &str,
    claimed: &mut BTreeMap<String, String>,
    derivations: &mut Vec<Derivation>,
    rejections: &mut Vec<Rejection>,
) {
    let race_slug = slug_segment(&record.key);
    if race_slug.is_empty() {
        rejections.push(Rejection {
            identity: identity.into(),
            problem: "race key has an empty slug".into(),
        });
        return;
    }
    let Some(parent) = native_id(record) else {
        rejections.push(Rejection {
            identity: identity.into(),
            problem: "parent record has no native protocol mapping".into(),
        });
        return;
    };
    let mut directives = 0;
    for line in description_lines(record) {
        let trimmed = strip_blockquote(&line);
        if !trimmed.starts_with("::rtraits") {
            continue;
        }
        directives += 1;
        if directives > 1 {
            rejections.push(Rejection {
                identity: identity.into(),
                problem: "more than one ::rtraits directive in one record".into(),
            });
            return;
        }
        let parsed = match pf_directives::rtraits(trimmed) {
            Ok(parsed) => parsed,
            Err(problem) => {
                rejections.push(Rejection {
                    identity: identity.into(),
                    problem: format!("::rtraits: {problem}"),
                });
                return;
            }
        };
        for trait_ in parsed.traits {
            let child = slug_segment(&trait_.title);
            if child.is_empty() {
                rejections.push(Rejection {
                    identity: format!("{identity}|racial-trait:{}", trait_.title),
                    problem: "trait title has an empty slug".into(),
                });
                continue;
            }
            let id = format!("{RACIAL_TRAIT_PREFIX}/{race_slug}/{child}");
            let Ok(id) = EntryId::parse(id) else {
                rejections.push(Rejection {
                    identity: format!("{identity}|racial-trait:{child}"),
                    problem: "derived id is not a valid entry id".into(),
                });
                continue;
            };
            if !claim(claimed, &id, identity, rejections) {
                continue;
            }
            let body = trait_.body();
            let title = trait_.title;
            derivations.push(Derivation {
                id,
                title: title.clone(),
                kind: DerivedKind::RacialTrait,
                parent: parent.clone(),
                parent_identity: identity.into(),
                provenance_record: format!("{}|racial-trait:{child}", record.key),
                body_lines: vec![if body.is_empty() { title.clone() } else { body }],
                name: title,
                source_ids: record
                    .publications
                    .iter()
                    .map(|publication| slug_segment(publication))
                    .collect(),
                path: record.path.clone(),
            });
        }
    }
}

fn plan_afflictions(
    record: &InputRecord,
    identity: &str,
    claimed: &mut BTreeMap<String, String>,
    derivations: &mut Vec<Derivation>,
    rejections: &mut Vec<Rejection>,
) {
    let host_slug = slug_segment(&record.key);
    if host_slug.is_empty() {
        rejections.push(Rejection {
            identity: identity.into(),
            problem: "affliction host key has an empty slug".into(),
        });
        return;
    }
    let Some(parent) = native_id(record) else {
        rejections.push(Rejection {
            identity: identity.into(),
            problem: "parent record has no native protocol mapping".into(),
        });
        return;
    };
    for line in description_lines(record) {
        let trimmed = strip_blockquote(&line);
        let Some(rest) = trimmed.strip_prefix("::aff") else {
            continue;
        };
        let parsed = match pf_directives::affliction(rest, false) {
            Ok(parsed) => parsed,
            Err(problem) => {
                rejections.push(Rejection {
                    identity: identity.into(),
                    problem: format!("::aff: {problem}"),
                });
                continue;
            }
        };
        let Some(name) = parsed.name else {
            rejections.push(Rejection {
                identity: identity.into(),
                problem: "::aff block has no name".into(),
            });
            continue;
        };
        let child = slug_segment(&name);
        if child.is_empty() {
            rejections.push(Rejection {
                identity: format!("{identity}|affliction:{name}"),
                problem: "affliction name has an empty slug".into(),
            });
            continue;
        }
        if parsed.markdown.trim().is_empty() {
            rejections.push(Rejection {
                identity: format!("{identity}|affliction:{child}"),
                problem: "affliction block has no stat lines".into(),
            });
            continue;
        }
        let id = format!("{AFFLICTION_PREFIX}/{host_slug}/{child}");
        let Ok(id) = EntryId::parse(id) else {
            rejections.push(Rejection {
                identity: format!("{identity}|affliction:{child}"),
                problem: "derived id is not a valid entry id".into(),
            });
            continue;
        };
        if !claim(claimed, &id, identity, rejections) {
            continue;
        }
        derivations.push(Derivation {
            id,
            title: name.clone(),
            kind: DerivedKind::Affliction,
            parent: parent.clone(),
            parent_identity: identity.into(),
            provenance_record: format!("{}|affliction:{child}", record.key),
            body_lines: vec![parsed.markdown],
            name,
            source_ids: record
                .publications
                .iter()
                .map(|publication| slug_segment(publication))
                .collect(),
            path: record.path.clone(),
        });
    }
}

fn claim(
    claimed: &mut BTreeMap<String, String>,
    id: &EntryId,
    identity: &str,
    rejections: &mut Vec<Rejection>,
) -> bool {
    if let Some(existing) = claimed.get(id.as_str()) {
        rejections.push(Rejection {
            identity: identity.into(),
            problem: format!(
                "derived id {} is already claimed by {existing}",
                id.as_str()
            ),
        });
        return false;
    }
    claimed.insert(id.as_str().to_string(), identity.into());
    true
}

/// Render a derivation's body through the ordinary Pf Markdown path.
pub fn body(derivation: &Derivation, env: &mut dyn crate::html::Env) -> Result<Vec<Block>, String> {
    crate::pf_markdown::render(&derivation.body_lines, env).map_err(|errors| errors.join("; "))
}

/// A record whose children this layer can derive.
pub fn is_candidate(record: &InputRecord) -> bool {
    if record.shape == "redirect" {
        return false;
    }
    let has = |needle: &str| {
        description_lines(record)
            .iter()
            .any(|line| line.contains(needle))
    };
    (is_core_race(record) && has("::rtraits")) || (is_affliction_host(&record.path) && has("::aff"))
}

pub fn is_core_race(record: &InputRecord) -> bool {
    let file = record.path.trim_start_matches("json/");
    file.starts_with("races")
        && record
            .publications
            .iter()
            .any(|publication| publication == CORE_RULEBOOK)
}

/// Monster and rule families only, as the derivation brief states. NPC records are
/// a separate family and are not part of this layer.
pub fn is_affliction_host(path: &str) -> bool {
    let file = path.trim_start_matches("json/");
    file.starts_with("monsters")
        || file.starts_with("monster_template")
        || file.starts_with("rules_")
        || file.starts_with("umr")
}

/// The canonical native ID of a Pf Data record, shared with the importer.
pub fn native_id(record: &InputRecord) -> Option<EntryId> {
    let protocol = record.protocol.as_deref()?;
    EntryId::parse(format!(
        "pf-data/{}/{}",
        slug_segment(protocol),
        slug_segment(&record.key)
    ))
    .ok()
}

fn description_lines(record: &InputRecord) -> Vec<String> {
    record
        .raw
        .get("description")
        .and_then(serde_json::Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn strip_blockquote(line: &str) -> &str {
    line.trim_start().trim_start_matches('>').trim_start()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(path: &str, key: &str, publications: &[&str], raw: serde_json::Value) -> InputRecord {
        InputRecord {
            protocol: Some("race".into()),
            path: path.into(),
            key: key.into(),
            name: key.into(),
            shape: "description".into(),
            publications: publications.iter().map(|p| (*p).to_string()).collect(),
            raw,
        }
    }

    fn core_race(key: &str, traits: &str) -> InputRecord {
        record(
            "json/races.json",
            key,
            &[CORE_RULEBOOK],
            json!({"description": [format!("::rtraits{{plural=Dwarves main=\"{traits}\"}}")]}),
        )
    }

    fn host(path: &str, key: &str, afflictions: &[&str]) -> InputRecord {
        let mut lines = vec!["## Creature".to_string()];
        lines.extend(afflictions.iter().map(|line| (*line).to_string()));
        record(path, key, &["Bestiary"], json!({"description": lines}))
    }

    #[test]
    fn derives_standard_traits_with_the_canonical_rule() {
        let record = core_race(
            "dwarf",
            "+2 Constitution, +2 Wisdom, -2 Charisma|Dwarves are tough.~MEDIUM~Stonecunning|Dwarves notice stonework.",
        );
        let mut rejections = Vec::new();
        let derivations = plan(&[record], &mut rejections);
        assert!(rejections.is_empty(), "{rejections:?}");
        let ids: Vec<_> = derivations.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "ref/racial-trait/dwarf/attributes",
                "ref/racial-trait/dwarf/medium",
                "ref/racial-trait/dwarf/stonecunning"
            ]
        );
        assert!(derivations.iter().all(
            |d| d.kind == DerivedKind::RacialTrait && d.parent.as_str() == "pf-data/race/dwarf"
        ));
        assert_eq!(
            derivations[2].provenance_record,
            "dwarf|racial-trait:stonecunning"
        );
    }

    #[test]
    fn only_core_races_and_monster_rule_hosts_derive() {
        let mut non_core = core_race("aasimar", "+2 Wisdom|Aasimars are wise.");
        non_core.publications = vec!["Inner Sea Races".into()];
        let npc = host(
            "json/npcs1.json",
            "duergar",
            &["::aff[Poison (Ex)]{iconP type=Bite-injury saveF=17 freqR=4 effStr=1 cure1}"],
        );
        let monster = host(
            "json/monsters01.json",
            "xacarba",
            &[
                "::aff[Fiendish Bile]{iconP type=Bite-injury saveF=25 freqR=6 effStr=\"1d4\" cure2c}",
            ],
        );
        let mut rejections = Vec::new();
        let derivations = plan(&[non_core, npc, monster], &mut rejections);
        assert!(rejections.is_empty(), "{rejections:?}");
        assert_eq!(derivations.len(), 1);
        assert_eq!(
            derivations[0].id.as_str(),
            "ref/affliction/xacarba/fiendish-bile"
        );
        assert_eq!(
            derivations[0].provenance_record,
            "xacarba|affliction:fiendish-bile"
        );
    }

    #[test]
    fn duplicate_ids_are_rejected_instead_of_suffixed() {
        let record = host(
            "json/monsters01.json",
            "xacarba",
            &[
                "::aff[Poison (Ex)]{iconP type=Bite-injury saveF=25 freqR=6 effStr=1 cure1}",
                "::aff[poison ex]{iconP type=Bite-injury saveF=25 freqR=6 effStr=1 cure1}",
            ],
        );
        let mut rejections = Vec::new();
        let derivations = plan(&[record], &mut rejections);
        assert_eq!(derivations.len(), 1);
        assert_eq!(rejections.len(), 1);
        assert!(rejections[0].problem.contains("already claimed"));
    }

    #[test]
    fn blockquote_prefixed_afflictions_and_unnamed_blocks_are_handled() {
        let record = host(
            "json/monster_template4.json",
            "runeslave",
            &[
                ">::aff[Arcane Decay]{iconD disease saveF=15 freq=\"1/week\" eff=\"Gain one runescar\"}",
                "::aff{iconP type=Bite-injury saveF=15 freqR=2 effStr=1 cure1}",
            ],
        );
        let mut rejections = Vec::new();
        let derivations = plan(&[record], &mut rejections);
        assert_eq!(derivations.len(), 1);
        assert_eq!(
            derivations[0].id.as_str(),
            "ref/affliction/runeslave/arcane-decay"
        );
        assert_eq!(
            rejections,
            [Rejection {
                identity: "json/monster_template4.json#runeslave".into(),
                problem: "::aff block has no name".into()
            }]
        );
    }

    #[test]
    fn every_derived_id_is_unique_and_valid() {
        let records = vec![
            core_race(
                "dwarf",
                "+2 Constitution|Dwarves are tough.~MEDIUM~Stonecunning|Dwarves notice stonework.",
            ),
            core_race(
                "elf",
                "+2 Dexterity|Elves are nimble.~MEDIUM~Keen Senses|Elves are keen.",
            ),
        ];
        let mut rejections = Vec::new();
        let derivations = plan(&records, &mut rejections);
        let mut ids: Vec<_> = derivations.iter().map(|d| d.id.as_str()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count);
        assert!(ids.iter().all(|id| EntryId::parse(*id).is_ok()));
    }
}
