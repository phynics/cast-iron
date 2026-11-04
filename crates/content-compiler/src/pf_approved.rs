//! Import a small, explicitly reviewed set of Pathfinder records.
//!
//! This adapter deliberately accepts only simple primary records. It does not expand
//! redirects, copies, tables, aliases, or compilation recipes.

use crate::{html, pf_data, pf_markdown};
use cast_iron_schema::{
    Block, CopyrightNotice, DatasetNotice, DatasetRef, EntryId, EntryKind, EntryReference,
    Exclusion, Provenance, RuleEntry, TypedFields,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub registry: PathBuf,
    pub dataset: PathBuf,
    pub selection: PathBuf,
}

#[derive(Debug)]
pub struct ApprovedImport {
    pub entries: Vec<RuleEntry>,
    pub datasets: Vec<DatasetRef>,
    pub notices: Vec<DatasetNotice>,
    pub copyrights: Vec<CopyrightNotice>,
    pub exclusions: Vec<Exclusion>,
    pub input_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    records: Vec<ApprovedRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovedRecord {
    path: String,
    key: String,
    expected_name: String,
    id: String,
    parent: Option<String>,
    source: String,
    expected_source: String,
    reason: String,
    #[serde(default)]
    topics: BTreeMap<String, String>,
}

pub fn load(config: &Config, config_dir: &Path) -> Result<ApprovedImport, String> {
    let resolve = |path: &Path| {
        if path.is_absolute() {
            Err(format!(
                "approved source path must be relative: {}",
                path.display()
            ))
        } else {
            Ok(config_dir.join(path))
        }
    };
    let registry = resolve(&config.registry)?;
    let dataset = resolve(&config.dataset)?;
    let selection_path = resolve(&config.selection)?;
    let inventory = pf_data::inventory(&dataset, &registry)?;
    let selection: Selection = read_json(&selection_path)?;
    if selection.records.is_empty() {
        return Err("approved source selection is empty".into());
    }

    let mut selected = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut entries = Vec::with_capacity(selection.records.len());
    let mut exclusions = Vec::new();
    for approved in &selection.records {
        if approved.reason.trim().is_empty() {
            return Err(format!(
                "{}:{} has no approval reason",
                approved.path, approved.key
            ));
        }
        if !approved.path.starts_with("json/") || approved.path.contains("..") {
            return Err(format!("invalid approved primary path {:?}", approved.path));
        }
        let key = (approved.path.clone(), approved.key.clone());
        if !selected.insert(key.clone()) {
            return Err(format!(
                "duplicate approved source record {}:{}",
                key.0, key.1
            ));
        }
        let id = EntryId::parse(approved.id.clone()).map_err(|e| e.to_string())?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate approved entry id {id}"));
        }
        let parent = approved
            .parent
            .as_ref()
            .map(|p| EntryId::parse(p.clone()).map_err(|e| e.to_string()))
            .transpose()?;
        let record = inventory
            .records
            .iter()
            .find(|r| r.path == approved.path && r.key == approved.key)
            .ok_or_else(|| {
                format!(
                    "approved record not found: {}:{}",
                    approved.path, approved.key
                )
            })?;
        validate_record(record, approved)?;
        let lines = description(&record.raw, &approved.path)?;
        let (mut body, links, render_error) = render_body(
            &lines,
            &approved.topics,
            &approved.expected_source,
            &approved.path,
        );
        if let Some(error) = render_error {
            return Err(error);
        }
        let mut references: BTreeSet<EntryReference> = links;
        for hierarchy_key in ["subtopics", "parent_topics", "siblings"] {
            for topic in string_array(&record.raw, hierarchy_key, &approved.path)? {
                match approved.topics.get(&topic) {
                    Some(target) => {
                        let target = EntryId::parse(target.clone()).map_err(|e| e.to_string())?;
                        references.insert(EntryReference {
                            target: target.clone(),
                            anchor: None,
                        });
                        if hierarchy_key == "subtopics" {
                            body.push(Block::EntryLink {
                                target,
                                title: topic_title(&topic),
                            });
                        }
                    }
                    None => exclusions.push(Exclusion {
                        path: format!("{}/{}:{}", approved.path, approved.key, topic),
                        records: 0,
                        reason: format!("unreviewed {hierarchy_key} target"),
                    }),
                }
            }
        }
        let protocol = record
            .protocol
            .as_ref()
            .ok_or_else(|| format!("no protocol route for {}", approved.path))?;
        let mut provenance = vec![Provenance {
            dataset: inventory.dataset.clone(),
            record: format!("{protocol}://{}", approved.key),
            path: approved.path.clone(),
        }];
        provenance.sort_by(|a, b| {
            (&a.dataset, &a.path, &a.record).cmp(&(&b.dataset, &b.path, &b.record))
        });
        let source_ids = vec![approved.source.clone()];
        entries.push(RuleEntry {
            id,
            kind: EntryKind::Rule,
            title: record.name.clone(),
            source_ids,
            section: Vec::new(),
            parent,
            body,
            fields: TypedFields::None,
            tags: Vec::new(),
            links: references.into_iter().collect(),
            provenance,
            license_refs: vec![crate::build::LICENSE_OGL.to_string()],
            relations: Vec::new(),
        });
    }

    // Every primary-inventory record is either selected or visible in coverage.
    let mut excluded_by_file: BTreeMap<String, (u32, BTreeSet<String>)> = BTreeMap::new();
    for record in &inventory.records {
        if selected.contains(&(record.path.clone(), record.key.clone())) {
            continue;
        }
        let reason = match record.shape.as_str() {
            "redirect" => "redirect or alias shape is not approved".to_string(),
            "copy" => "copy expansion is not approved".to_string(),
            "compilation" => "compilation expansion is not approved".to_string(),
            "auxiliary" | "publication" => {
                "auxiliary or publication record is not a rules entry".to_string()
            }
            shape => format!("record shape `{shape}` is not approved"),
        };
        let slot = excluded_by_file.entry(record.path.clone()).or_default();
        slot.0 += 1;
        slot.1.insert(reason);
    }
    exclusions.extend(
        excluded_by_file
            .into_iter()
            .map(|(path, (records, reasons))| Exclusion {
                path,
                records,
                reason: reasons.into_iter().collect::<Vec<_>>().join("; "),
            }),
    );
    exclusions.sort_by(|a, b| (&a.path, &a.reason).cmp(&(&b.path, &b.reason)));
    entries.sort_by(|a, b| a.id.cmp(&b.id));

    let pin: pf_data::SourcePin = read_json(&registry)?;
    let notices = vec![DatasetNotice {
        id: pin.id.clone(),
        repository: pin.repository.clone(),
        commit: pin.commit.clone(),
        note: "Pinned source inventory; only records named by the reviewed selection are imported."
            .into(),
    }];
    let datasets = vec![DatasetRef {
        id: pin.id,
        repository: pin.repository,
        commit: pin.commit,
    }];
    let copyrights = parse_copyrights(
        &fs::read_to_string(dataset.join("OPENGAMECONTENT.md")).map_err(|e| e.to_string())?,
    )?;
    Ok(ApprovedImport {
        entries,
        datasets,
        notices,
        copyrights,
        exclusions,
        input_digest: inventory.input_digest,
    })
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn validate_raw_shape(raw: &Value, path: &str, key: &str) -> Result<(), String> {
    let object = raw
        .as_object()
        .ok_or_else(|| format!("{path}:{key} is not an object"))?;
    const ALLOWED: &[&str] = &[
        "name",
        "sources",
        "description",
        "subtopics",
        "parent_topics",
        "siblings",
    ];
    if let Some(unknown) = object.keys().find(|k| !ALLOWED.contains(&k.as_str())) {
        return Err(format!(
            "unsupported source field {unknown:?} at {path}:{key}"
        ));
    }
    Ok(())
}

fn validate_record(record: &pf_data::InputRecord, approved: &ApprovedRecord) -> Result<(), String> {
    if record.shape != "description" {
        return Err(format!(
            "approved record is not a primary description: {}:{}",
            approved.path, approved.key
        ));
    }
    if record.name != approved.expected_name {
        return Err(format!(
            "approved record name mismatch at {}:{}: expected {:?}, found {:?}",
            approved.path, approved.key, approved.expected_name, record.name
        ));
    }
    validate_raw_shape(&record.raw, &approved.path, &approved.key)?;
    let raw_sources = pf_data::publication_names(&record.raw);
    if !raw_sources.iter().any(|s| s == &approved.expected_source) {
        return Err(format!(
            "approved record source mismatch at {}:{}",
            approved.path, approved.key
        ));
    }
    Ok(())
}

fn description(raw: &Value, path: &str) -> Result<Vec<String>, String> {
    raw.get("description")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{path} record has no description list"))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{path} description contains a non-string line"))
        })
        .collect()
}

fn string_array(raw: &Value, key: &str, path: &str) -> Result<Vec<String>, String> {
    match raw.get(key) {
        None => Ok(Vec::new()),
        Some(Value::Array(values)) => values
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{path} {key} contains a non-string"))
            })
            .collect(),
        Some(_) => Err(format!("{path} {key} is not a list")),
    }
}

fn topic_title(topic: &str) -> String {
    topic
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct ApprovedEnv<'a> {
    topics: &'a BTreeMap<String, String>,
    expected_source_href: String,
    error: Option<String>,
    references: BTreeSet<EntryReference>,
}

impl html::Env for ApprovedEnv<'_> {
    fn link(&mut self, href: &str) -> Option<(EntryId, Option<String>)> {
        if href == self.expected_source_href {
            return None;
        }
        let key = self.topics.get(href).or_else(|| {
            href.rsplit('/')
                .next()
                .and_then(|last| self.topics.get(last))
        });
        match key {
            Some(target) => match EntryId::parse(target.clone()) {
                Ok(target) => {
                    self.references.insert(EntryReference {
                        target: target.clone(),
                        anchor: None,
                    });
                    Some((target, None))
                }
                Err(error) => {
                    self.error = Some(format!("invalid approved topic target for {href}: {error}"));
                    None
                }
            },
            None => {
                self.error = Some(format!("unapproved link {href}"));
                None
            }
        }
    }
    fn stripped(&mut self, _key: &str) {}
}

fn render_body(
    lines: &[String],
    topics: &BTreeMap<String, String>,
    expected_source: &str,
    path: &str,
) -> (Vec<Block>, BTreeSet<EntryReference>, Option<String>) {
    // The upstream SOURCE-link encoder keeps spaces as underscores, unlike entry ids.
    let source_slug = crate::slug::slug_segment(expected_source).replace('-', "_");
    let mut env = ApprovedEnv {
        topics,
        expected_source_href: format!("source/{source_slug}"),
        error: None,
        references: BTreeSet::new(),
    };
    match pf_markdown::render(lines, &mut env) {
        Ok(body) => {
            let error = env.error.map(|e| format!("{path}: {e}"));
            (body, env.references, error)
        }
        Err(errors) => (
            Vec::new(),
            BTreeSet::new(),
            Some(format!("{path}: {}", errors.join("; "))),
        ),
    }
}

fn parse_copyrights(input: &str) -> Result<Vec<CopyrightNotice>, String> {
    let mut in_section = false;
    let mut found_end = false;
    let mut notices = Vec::new();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed == "**15. COPYRIGHT NOTICE**" {
            in_section = true;
            continue;
        }
        if in_section && trimmed == "## Product Identity" {
            found_end = true;
            break;
        }
        if !in_section || trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("**")
            && let Some((title, notice)) = rest.split_once("**")
        {
            let notice = notice.trim();
            if !title.trim().is_empty() && !notice.is_empty() {
                notices.push(CopyrightNotice {
                    title: html::plain_text(title.trim()),
                    notice: html::plain_text(notice),
                });
            }
        }
    }
    if !in_section || !found_end || notices.is_empty() {
        return Err("OPENGAMECONTENT.md has no complete Section 15 copyright list".into());
    }
    Ok(notices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_section_15_titles_and_decodes_entities() {
        let input = "before\n**15. COPYRIGHT NOTICE**\n**Core &amp; More** &copy; 2025, Holder.\n## Product Identity\n";
        assert_eq!(
            parse_copyrights(input).unwrap(),
            vec![CopyrightNotice {
                title: "Core & More".into(),
                notice: "© 2025, Holder.".into(),
            }]
        );
    }

    #[test]
    fn requires_both_copyright_section_boundaries() {
        assert!(parse_copyrights("**15. COPYRIGHT NOTICE**\n**Title** Copyright 2025\n").is_err());
    }

    #[test]
    fn rejects_mismatched_approved_identity_and_publication() {
        let record = pf_data::InputRecord {
            protocol: Some("rule".into()),
            path: "json/rules.json".into(),
            key: "magic".into(),
            name: "Magic and Spellcasting".into(),
            shape: "description".into(),
            publications: vec!["PRPG Core Rulebook".into()],
            raw: serde_json::json!({
                "name": "Magic and Spellcasting",
                "sources": ["PRPG Core Rulebook"],
                "description": ["text"]
            }),
        };
        let mut approved = ApprovedRecord {
            path: "json/rules.json".into(),
            key: "magic".into(),
            expected_name: "Wrong name".into(),
            id: "pf-data/rule/magic".into(),
            parent: None,
            source: "core-rulebook".into(),
            expected_source: "PRPG Core Rulebook".into(),
            reason: "reviewed".into(),
            topics: BTreeMap::new(),
        };
        assert!(
            validate_record(&record, &approved)
                .unwrap_err()
                .contains("name mismatch")
        );
        approved.expected_name = record.name.clone();
        approved.expected_source = "Advanced Player's Guide".into();
        assert!(
            validate_record(&record, &approved)
                .unwrap_err()
                .contains("source mismatch")
        );
    }

    #[test]
    fn loads_the_reviewed_magic_and_spellcasting_passage() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let config = Config {
            registry: "../pathfinder-data-1-e/source.json".into(),
            dataset: "../../.cache/sources/pathfinder-data-1-e".into(),
            selection: "../pathfinder-data-1-e/approved.json".into(),
        };
        let result = load(&config, &root.join("sources/psrd-data")).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].title, "Magic and Spellcasting");
        assert_eq!(result.entries[0].id.as_str(), "pf-data/rule/magic");
        assert!(result.entries[0].body.iter().any(|b| matches!(b, Block::EntryLink { target, .. } if target.as_str() == "core-rulebook/rules/magic/casting-spells")));
        let covered: u64 = result.exclusions.iter().map(|e| u64::from(e.records)).sum();
        assert_eq!(covered + result.entries.len() as u64, 34_530);
    }
}
