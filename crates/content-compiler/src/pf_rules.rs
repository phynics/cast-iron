//! Derivation of standalone rule entries from nested Pf Data content.
//!
//! PSRD promotes some rule sections and table rows that Pf Data keeps inside a
//! parent record. The canonical cutover needs those as standalone
//! [`EntryKind::Rule`] records. Matching is driven by the reviewed allowlist in
//! `sources/pathfinder-data-1-e/derived-rules.json`; this module only extracts
//! the approved headings and rows. It never guesses a title or a host.

use crate::slug::slug_segment;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One reviewed derivation. `host` is `path#key` inside the pinned dataset.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AllowlistRule {
    /// The PSRD rule identity the derivation stands in for.
    pub rule: String,
    pub title: String,
    #[serde(default)]
    pub sources: Vec<String>,
    pub host: String,
    /// The reviewed route protocol for the host, part of the canonical id.
    pub protocol: String,
    /// `section` or `row`.
    pub shape: String,
    /// The heading or row label copied from the Pf record.
    pub heading: String,
    /// The canonical id the generator expects. Verified against the id rule.
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct AllowlistFile {
    format: String,
    version: u32,
    rules: Vec<AllowlistRule>,
}

/// The reviewed derivations, indexed by parent record.
#[derive(Debug, Default)]
pub struct RuleAllowlist {
    by_host: BTreeMap<String, Vec<AllowlistRule>>,
    len: usize,
}

impl RuleAllowlist {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json(&bytes)
    }

    /// Load when present. A checkout without the file still compiles content.
    pub fn load_optional(path: &Path) -> Result<Self, String> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default())
        }
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let file: AllowlistFile =
            serde_json::from_slice(bytes).map_err(|e| format!("derived rule allowlist: {e}"))?;
        if file.format != "cast-iron-derived-rules" || file.version != 1 {
            return Err("unsupported derived rule allowlist format or version".into());
        }
        let mut by_host: BTreeMap<String, Vec<AllowlistRule>> = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for rule in file.rules {
            if !matches!(rule.shape.as_str(), "section" | "row") {
                return Err(format!("{}: unknown shape {:?}", rule.rule, rule.shape));
            }
            if !ids.insert(rule.id.clone()) {
                return Err(format!("duplicate derived id {}", rule.id));
            }
            let expected = derived_id_from_host(&rule.host, &rule.protocol, &rule.title)?;
            if expected != rule.id {
                return Err(format!(
                    "{}: allowlist id {} does not match the id rule ({expected})",
                    rule.rule, rule.id
                ));
            }
            by_host.entry(rule.host.clone()).or_default().push(rule);
        }
        for rules in by_host.values_mut() {
            rules.sort_by(|a, b| (&a.id, &a.shape, &a.heading).cmp(&(&b.id, &b.shape, &b.heading)));
        }
        Ok(Self {
            len: by_host.values().map(Vec::len).sum(),
            by_host,
        })
    }

    pub fn for_host(&self, path: &str, key: &str) -> &[AllowlistRule] {
        self.by_host
            .get(&format!("{path}#{key}"))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleShape {
    Section,
    Row,
}

impl RuleShape {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Section => "section",
            Self::Row => "row",
        }
    }
}

/// A heading or row that the allowlist approved for this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedRule {
    pub rule: String,
    pub title: String,
    pub id: String,
    pub shape: RuleShape,
    /// The original heading or row label, kept for provenance.
    pub heading: String,
    pub slug: String,
    /// Source lines rendered by the existing Markdown adapter.
    pub body_lines: Vec<String>,
}

/// The canonical derived id: `ref/rule/{host-slug}/{rule-slug}`.
///
/// The host slug combines the reviewed route protocol and record key so that
/// same-named keys in different families cannot collide. The rest of the
/// record slug is the rule title, as the cutover contract specifies.
pub fn derived_id(protocol: &str, key: &str, title: &str) -> Result<String, String> {
    let host = slug_segment(&format!("{protocol}-{key}"));
    let rule = slug_segment(title);
    if host.is_empty() || rule.is_empty() {
        return Err(format!(
            "cannot derive id from {protocol:?}/{key:?}/{title:?}"
        ));
    }
    let id = format!("ref/rule/{host}/{rule}");
    cast_iron_schema::EntryId::parse(id.clone()).map_err(|e| e.to_string())?;
    Ok(id)
}

fn derived_id_from_host(host: &str, protocol: &str, title: &str) -> Result<String, String> {
    let (_path, key) = host
        .split_once('#')
        .ok_or_else(|| format!("derived host {host:?} is not path#key"))?;
    derived_id(protocol, key, title)
}

/// Extract every allowlisted heading or row from one expanded Pf record.
///
/// The result is sorted by canonical id. A duplicate id within the host is
/// dropped and reported by the caller, because two derivations must not share a
/// public identifier.
pub fn extract(lines: &[String], rules: &[AllowlistRule]) -> Vec<ExtractedRule> {
    let headings = heading_index(lines);
    let rows = row_index(lines);
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for rule in rules {
        let wanted = normalize_title(&rule.heading);
        let extracted = match rule.shape.as_str() {
            "section" => headings
                .iter()
                .find(|heading| normalize_title(&heading.title) == wanted)
                .and_then(|heading| {
                    section_body(lines, heading.line, heading.level)
                        .map(|body| (heading.title.clone(), body))
                }),
            "row" => rows
                .iter()
                .find(|row| normalize_title(&row.title) == wanted)
                .map(|row| (row.title.clone(), row.body.clone())),
            _ => None,
        };
        let Some((heading, body_lines)) = extracted else {
            continue;
        };
        if body_lines.iter().all(|line| line.trim().is_empty()) {
            continue;
        }
        let slug = slug_segment(&rule.title);
        if slug.is_empty() || !seen.insert(rule.id.clone()) {
            continue;
        }
        out.push(ExtractedRule {
            rule: rule.rule.clone(),
            title: rule.title.clone(),
            id: rule.id.clone(),
            shape: if rule.shape == "section" {
                RuleShape::Section
            } else {
                RuleShape::Row
            },
            heading,
            slug,
            body_lines,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[derive(Debug)]
struct Heading {
    line: usize,
    level: u8,
    title: String,
}

#[derive(Debug)]
struct Row {
    title: String,
    body: Vec<String>,
}

fn heading_index(lines: &[String]) -> Vec<Heading> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(line, text)| {
            parse_heading(text).map(|(level, title)| Heading { line, level, title })
        })
        .collect()
}

fn parse_heading(text: &str) -> Option<(u8, String)> {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix("::") {
        let (name, tail) = rest.split_once('[')?;
        let end = tail.rfind(']')?;
        let title = tail[..end].trim();
        if title.is_empty() {
            return None;
        }
        let level = match name {
            "mainheader" => 1,
            "sh" | "gh" | "h" => 2,
            _ => name.strip_prefix('h')?.parse().ok()?,
        };
        return (1..=6).contains(&level).then(|| (level, title.to_string()));
    }
    let hashes = text.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&hashes) || text.as_bytes().get(hashes) != Some(&b' ') {
        return None;
    }
    let title = text[hashes..].trim();
    (!title.is_empty()).then(|| (hashes as u8, title.to_string()))
}

fn section_body(lines: &[String], start: usize, level: u8) -> Option<Vec<String>> {
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, text)| {
            parse_heading(text).and_then(|(next, _)| (next <= level).then_some(index))
        })
        .unwrap_or(lines.len());
    let body: Vec<String> = lines[start + 1..end]
        .iter()
        .map(|line| line.to_string())
        .collect();
    trim_blank(body)
}

fn trim_blank(mut lines: Vec<String>) -> Option<Vec<String>> {
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    (!lines.is_empty()).then_some(lines)
}

fn row_index(lines: &[String]) -> Vec<Row> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let text = lines[index].trim();
        if let Some(title) = row_directive(text) {
            if let Some(body) = row_body(text) {
                out.push(Row { title, body });
            }
            index += 1;
            continue;
        }
        if table_header(lines, index) {
            let header = table_cells(text);
            if header.len() < 2 {
                index += 1;
                continue;
            }
            let mut row = index + 2;
            while row < lines.len() && lines[row].trim().starts_with('|') {
                let cells = table_cells(lines[row].trim());
                if cells.len() >= 2
                    && let Some(first) = cells.first()
                    && !first.trim().is_empty()
                {
                    let body = definition_lines(&header[1..], &cells[1..]);
                    if !body.is_empty() {
                        out.push(Row {
                            title: first.trim().to_string(),
                            body,
                        });
                    }
                }
                row += 1;
            }
            index = row;
            continue;
        }
        index += 1;
    }
    out
}

fn row_directive(text: &str) -> Option<String> {
    let rest = text.strip_prefix("::row[")?;
    let end = rest.find(']')?;
    let title = rest[..end].trim();
    (!title.is_empty()).then(|| title.to_string())
}

fn row_body(text: &str) -> Option<Vec<String>> {
    let attrs = crate::pf_markdown::attributes(text).ok()?;
    let mut body = Vec::new();
    for (key, value) in attrs {
        if value.trim().is_empty() {
            continue;
        }
        let label = if key == "info" {
            "Details".to_string()
        } else {
            humanize(&key)
        };
        body.push(format!("**{label}:** {value}"));
    }
    (!body.is_empty()).then_some(body)
}

fn table_header(lines: &[String], index: usize) -> bool {
    let Some(next) = lines.get(index + 1) else {
        return false;
    };
    lines[index].trim().starts_with('|')
        && next.trim().starts_with('|')
        && next
            .trim()
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && next.contains('-')
}

fn table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn definition_lines(headers: &[String], cells: &[String]) -> Vec<String> {
    headers
        .iter()
        .zip(cells)
        .filter(|(header, cell)| !header.trim().is_empty() && !cell.trim().is_empty())
        .map(|(header, cell)| format!("**{}:** {cell}", header.trim()))
        .collect()
}

fn humanize(key: &str) -> String {
    let label = key.replace('_', " ");
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => key.to_string(),
    }
}

/// Case, punctuation, and trailing `(Ex)`/`(Su)`/`(Sp)` do not change identity.
pub fn normalize_title(value: &str) -> String {
    let value = value.trim();
    // Strip a trailing `(Ex)`, `(Su)`, `(Sp)`, or combination. Identity keeps
    // the title, not the display marker.
    let value = if value.ends_with(')')
        && let Some(open) = value.rfind('(')
    {
        let inner = &value[open + 1..value.len() - 1];
        let marker = inner
            .split(',')
            .map(str::trim)
            .all(|part| matches!(part, "Ex" | "Su" | "Sp"));
        if marker { value[..open].trim() } else { value }
    } else {
        value
    };
    let mut words = Vec::new();
    let mut word = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            word.push(ch.to_ascii_lowercase());
        } else {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            if ch == '&' {
                words.push("and".into());
            }
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(shape: &str, heading: &str) -> AllowlistRule {
        AllowlistRule {
            rule: format!("legacy/{shape}/{heading}"),
            title: heading.into(),
            sources: Vec::new(),
            host: "json/rules.json#host".into(),
            protocol: "rule".into(),
            shape: shape.into(),
            heading: heading.into(),
            id: derived_id("rule", "host", heading).unwrap(),
        }
    }

    #[test]
    fn id_rule_uses_host_and_title_slugs() {
        assert_eq!(
            derived_id("umr", "rake", "Rake").unwrap(),
            "ref/rule/umr-rake/rake"
        );
        assert_eq!(
            derived_id("rules_gm07", "sanity_and_madness", "Curing Insanity").unwrap(),
            "ref/rule/rules-gm07-sanity-and-madness/curing-insanity"
        );
        assert!(derived_id("", "", "Title").is_err());
        assert!(derived_id("rule", "host", "...").is_err());
    }

    #[test]
    fn extracts_mainheader_sh_and_markdown_sections() {
        let lines = vec![
            "intro".to_string(),
            "::mainheader[Curing Insanity]".into(),
            "".into(),
            "First cure paragraph.".into(),
            "## Nested detail".into(),
            "Nested paragraph.".into(),
            "## Types of Insanity".into(),
            "Type paragraph.".into(),
            "::sh[Other]".into(),
            "Other paragraph.".into(),
        ];
        let found = extract(
            &lines,
            &[
                rule("section", "Curing Insanity"),
                rule("section", "Types of Insanity"),
                rule("section", "Other"),
            ],
        );
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].title, "Curing Insanity");
        assert!(found[0].body_lines.join(" ").contains("First cure"));
        assert!(found[0].body_lines.join(" ").contains("Nested detail"));
        assert_eq!(found[1].title, "Other");
        assert_eq!(found[2].title, "Types of Insanity");
    }

    #[test]
    fn extracts_row_directives_and_markdown_tables() {
        let lines = vec![
            "::row[Amphibious]{info=\"The army can move in water.\"}".into(),
            "| Special Ability | Description |".into(),
            "| --- | --- |".into(),
            "| Bleed | The army takes 1d6 damage. |".into(),
            "| Not a rule | ignored |".into(),
        ];
        let found = extract(&lines, &[rule("row", "Amphibious"), rule("row", "Bleed")]);
        assert_eq!(found.len(), 2);
        assert!(found[0].body_lines.join(" ").contains("**Details:**"));
        assert!(found[1].body_lines.join(" ").contains("**Description:**"));
    }

    #[test]
    fn normalizes_titles_and_type_markers() {
        assert_eq!(normalize_title("Amphibious (Ex)"), "amphibious");
        assert_eq!(
            normalize_title("  Age, Height & Weight  "),
            "age height and weight"
        );
        assert_eq!(normalize_title("Will-o'-Wisp"), "will o wisp");
    }

    #[test]
    fn unlisted_headings_are_ignored() {
        let lines = vec!["## Ignored".into(), "Body.".into()];
        assert!(extract(&lines, &[rule("section", "Wanted")]).is_empty());
    }

    #[test]
    fn allowlist_indexes_by_parent_and_verifies_the_id_rule() {
        let id = derived_id("rule", "host", "Curing Insanity").unwrap();
        let json = format!(
            r#"{{"format":"cast-iron-derived-rules","version":1,"rules":[{{"rule":"legacy","title":"Curing Insanity","sources":[],"host":"json/rules.json#host","protocol":"rule","shape":"section","heading":"Curing Insanity","id":"{id}"}}]}}"#
        );
        let allowlist = RuleAllowlist::from_json(json.as_bytes()).unwrap();
        assert_eq!(allowlist.len(), 1);
        assert_eq!(allowlist.for_host("json/rules.json", "host").len(), 1);
        assert!(allowlist.for_host("json/rules.json", "other").is_empty());

        let bad = json.replace(&id, "ref/rule/wrong/host");
        assert!(
            RuleAllowlist::from_json(bad.as_bytes())
                .unwrap_err()
                .contains("does not match")
        );
    }

    #[test]
    fn duplicate_ids_are_dropped_within_a_host() {
        let lines = vec![
            "## Same".into(),
            "First.".into(),
            "## Same".into(),
            "Second.".into(),
        ];
        let found = extract(&lines, &[rule("section", "Same")]);
        assert_eq!(found.len(), 1);
        assert!(found[0].body_lines.join(" ").contains("First"));
    }
}
