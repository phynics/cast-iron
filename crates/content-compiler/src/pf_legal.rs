//! Section 15 notice coverage for the Pf Data publication registry.
//!
//! This module only reports and gates licensing evidence. It never writes the
//! production legal page or content library.

use crate::html;
use crate::pf_data::Inventory;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// One Section 15 entry, already decoded from its pinned notice source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub title: String,
    pub notice: String,
}

/// The Section 15 evidence for one publication referenced by an in-scope record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublicationLegal {
    pub publication: String,
    pub records: usize,
    /// `exact`, `normalized-prefix`, or `alias` for the Pf Data notice list;
    /// `psrd` or `d20pfsrd` when another pinned source supplied the first match.
    /// Absent when no notice matched.
    pub match_method: Option<String>,
    /// Every notice source that covered this publication, in source order:
    /// `pf-data`, `psrd`, then `d20pfsrd`.
    pub notice_sources: Vec<String>,
    /// Every applicable notice; empty when no notice matched.
    pub notices: Vec<Notice>,
}

/// One notice source's contribution to the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoticeSourceSummary {
    pub id: String,
    /// Records read from the source, including rows with no parseable title.
    pub rows: usize,
    pub matched_publications: usize,
    pub unused_rows: usize,
    pub unparsed_rows: usize,
}

/// A publication excluded before matching, retained so nothing disappears silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExcludedPublication {
    pub publication: String,
    pub records: usize,
}

/// Deterministic evidence written to `reports/migration-legal.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegalReport {
    pub dataset: String,
    pub commit: String,
    pub in_scope_records: usize,
    pub publication_count: usize,
    pub matched_publication_count: usize,
    pub unmatched_publication_count: usize,
    pub publications: Vec<PublicationLegal>,
    pub excluded_publications: Vec<ExcludedPublication>,
    /// Counts for every pinned notice source, in match order.
    pub notice_sources: Vec<NoticeSourceSummary>,
    /// Rows from the d20pfsrd export whose title could not be separated from
    /// their copyright text. They cannot match a publication, so they stay
    /// visible here instead of disappearing.
    pub unparsed_notices: Vec<Notice>,
    /// Unmatched entries from the pinned Pf Data OPENGAMECONTENT list.
    pub unmatched_notices: Vec<Notice>,
}

/// Paths for the three pinned notice sources and the reviewed matching inputs.
pub struct Inputs<'a> {
    pub opengamecontent: &'a Path,
    pub psrd_notices: Option<&'a Path>,
    pub aliases: &'a Path,
    pub out_of_scope: &'a Path,
    pub d20pfsrd_csv: &'a Path,
    pub d20pfsrd_provenance: &'a Path,
}

impl LegalReport {
    /// Publications that still need a Section 15 entry before a production build.
    pub fn blocking_publications(&self) -> Vec<&PublicationLegal> {
        self.publications
            .iter()
            .filter(|publication| publication.notices.is_empty())
            .collect()
    }

    /// The coverage-summary block. Stable shape for CI comparison.
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "publications": self.publication_count,
            "matched_publications": self.matched_publication_count,
            "unmatched_publications": self.unmatched_publication_count,
            "report": "reports/migration-legal.json",
        })
    }
}

/// Refuse a production build while any in-scope publication has no notice.
pub fn gate(report: &LegalReport) -> Result<(), String> {
    let missing = report.blocking_publications();
    if missing.is_empty() {
        return Ok(());
    }
    let titles: Vec<&str> = missing
        .iter()
        .map(|publication| publication.publication.as_str())
        .collect();
    Err(format!(
        "{} publication(s) with in-scope records have no matched Section 15 notice: {}",
        titles.len(),
        titles.join(", ")
    ))
}

/// Parse the complete Section 15 list from the pinned OPENGAMECONTENT.md.
pub fn parse_notices(input: &str) -> Result<Vec<Notice>, String> {
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
                notices.push(Notice {
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AliasFile {
    aliases: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct ScopePolicy {
    #[serde(default)]
    publications: BTreeMap<String, String>,
}

/// Build the legal evidence from the inventory and the three pinned notice sources.
pub fn build(inventory: &Inventory, inputs: &Inputs<'_>) -> Result<LegalReport, String> {
    // Source order is part of the evidence: Pf Data, PSRD, then d20pfsrd.
    let opengamecontent = fs::read_to_string(inputs.opengamecontent)
        .map_err(|e| format!("{}: {e}", inputs.opengamecontent.display()))?;
    let mut sources = vec![NoticeSource {
        id: "pf-data",
        notices: parse_notices(&opengamecontent)?,
    }];
    if let Some(path) = inputs.psrd_notices {
        sources.push(NoticeSource {
            id: "psrd",
            notices: load_section_15(path)?,
        });
    }
    let (d20_notices, unparsed_notices) =
        load_d20pfsrd(inputs.d20pfsrd_csv, inputs.d20pfsrd_provenance)?;
    sources.push(NoticeSource {
        id: "d20pfsrd",
        notices: d20_notices,
    });

    let aliases = load_aliases(inputs.aliases)?;
    let scope = load_scope(inputs.out_of_scope)?;
    let excluded_records: BTreeSet<(String, String)> = inventory
        .out_of_scope
        .iter()
        .map(|record| (record.path.clone(), record.key.clone()))
        .collect();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut excluded_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut in_scope_records = 0usize;
    for record in &inventory.records {
        if excluded_records.contains(&(record.path.clone(), record.key.clone())) {
            continue;
        }
        in_scope_records += 1;
        for publication in &record.publications {
            if scope.publications.contains_key(publication) {
                *excluded_counts.entry(publication.clone()).or_default() += 1;
            } else {
                *counts.entry(publication.clone()).or_default() += 1;
            }
        }
    }
    let (publications, used) = match_publications(&counts, &sources, &aliases)?;
    let matched_publication_count = publications
        .iter()
        .filter(|publication| !publication.notices.is_empty())
        .count();
    let unmatched_publication_count = publications.len() - matched_publication_count;
    let notice_sources = sources
        .iter()
        .enumerate()
        .map(|(slot, source)| {
            let unparsed_rows = if source.id == "d20pfsrd" {
                unparsed_notices.len()
            } else {
                0
            };
            let rows = source.notices.len() + unparsed_rows;
            NoticeSourceSummary {
                id: source.id.to_string(),
                rows,
                matched_publications: publications
                    .iter()
                    .filter(|publication| {
                        publication.notice_sources.iter().any(|id| id == source.id)
                    })
                    .count(),
                unused_rows: rows - used[slot].len(),
                unparsed_rows,
            }
        })
        .collect();

    // Keep the informational unmatched list scoped to the Pf Data notice, the
    // source this report originally documented. The per-source summary above
    // accounts for the rest without pasting 1,200 unused third-party lines into
    // a committed report.
    let pf_data = &sources[0];
    let matched_titles: BTreeSet<String> = used[0]
        .iter()
        .map(|&index| pf_data.notices[index].title.clone())
        .collect();
    let mut unmatched_notices: Vec<Notice> = pf_data
        .notices
        .iter()
        .filter(|notice| !matched_titles.contains(notice.title.as_str()))
        .cloned()
        .collect();
    unmatched_notices.sort_by(|a, b| (&a.title, &a.notice).cmp(&(&b.title, &b.notice)));
    unmatched_notices.dedup();
    Ok(LegalReport {
        dataset: inventory.dataset.clone(),
        commit: inventory.commit.clone(),
        in_scope_records,
        publication_count: publications.len(),
        matched_publication_count,
        unmatched_publication_count,
        publications,
        excluded_publications: excluded_counts
            .into_iter()
            .map(|(publication, records)| ExcludedPublication {
                publication,
                records,
            })
            .collect(),
        notice_sources,
        unparsed_notices,
        unmatched_notices,
    })
}

#[derive(Debug)]
struct NoticeSource {
    id: &'static str,
    notices: Vec<Notice>,
}

fn load_aliases(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: AliasFile =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file.aliases)
}

#[derive(Debug, Deserialize)]
struct Section15File {
    section_15: Vec<Notice>,
}

fn load_section_15(path: &Path) -> Result<Vec<Notice>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: Section15File =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if file.section_15.is_empty() {
        return Err(format!("{}: Section 15 list is empty", path.display()));
    }
    Ok(file.section_15)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct D20Provenance {
    id: String,
    source: String,
    supplied: String,
    csv: String,
    sha256: String,
    rows: usize,
    note: String,
}

fn load_d20pfsrd(
    csv_path: &Path,
    provenance_path: &Path,
) -> Result<(Vec<Notice>, Vec<Notice>), String> {
    let text = fs::read_to_string(provenance_path)
        .map_err(|e| format!("{}: {e}", provenance_path.display()))?;
    let provenance: D20Provenance =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", provenance_path.display()))?;
    if provenance.id != "d20pfsrd-section15" {
        return Err(format!(
            "{}: unexpected notice source id {:?}",
            provenance_path.display(),
            provenance.id
        ));
    }
    if provenance.source.trim().is_empty()
        || provenance.supplied.trim().is_empty()
        || provenance.note.trim().is_empty()
    {
        return Err(format!(
            "{}: source, supplied, and note must describe the export",
            provenance_path.display()
        ));
    }
    let file_name = csv_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{}: invalid file name", csv_path.display()))?;
    if provenance.csv != file_name {
        return Err(format!(
            "{}: provenance names {} but the source file is {file_name}",
            provenance_path.display(),
            provenance.csv
        ));
    }
    let expected = provenance.sha256.to_ascii_lowercase();
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{}: sha256 must be 64 hexadecimal characters",
            provenance_path.display()
        ));
    }
    let bytes = fs::read(csv_path).map_err(|e| format!("{}: {e}", csv_path.display()))?;
    let actual = hex(&Sha256::digest(&bytes));
    if actual != expected {
        return Err(format!(
            "{}: SHA256 mismatch: expected {expected}, got {actual}",
            csv_path.display()
        ));
    }
    let text = String::from_utf8(bytes).map_err(|e| format!("{}: {e}", csv_path.display()))?;
    let records = parse_csv(&text)?;
    if records.len() != provenance.rows {
        return Err(format!(
            "{}: provenance says {} rows, parsed {}",
            csv_path.display(),
            provenance.rows,
            records.len()
        ));
    }
    let mut notices = Vec::new();
    let mut unparsed = Vec::new();
    for (index, record) in records.iter().enumerate() {
        if record.len() != 2 {
            return Err(format!(
                "{}: row {} has {} fields, expected 2",
                csv_path.display(),
                index + 1,
                record.len()
            ));
        }
        let notice = record[1].trim().to_string();
        if notice.is_empty() {
            continue;
        }
        match notice_title(&notice) {
            Some(title) => notices.push(Notice { title, notice }),
            None => unparsed.push(Notice {
                title: format!("d20pfsrd row {}", index + 1),
                notice,
            }),
        }
    }
    Ok((notices, unparsed))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Minimal RFC 4180 reader for the pinned export. Quoted fields may contain
/// commas and line breaks; `""` is an escaped quote. Nothing is rewritten.
fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, String> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut quoted = false;
    let mut at_field_start = true;
    while let Some(character) = chars.next() {
        if quoted {
            if character == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(character);
            }
            continue;
        }
        match character {
            '"' if at_field_start => {
                quoted = true;
                at_field_start = false;
            }
            ',' => {
                record.push(std::mem::take(&mut field));
                at_field_start = true;
            }
            '\r' | '\n' => {
                if character == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                if !(record.len() == 1 && record[0].is_empty()) {
                    records.push(std::mem::take(&mut record));
                } else {
                    record.clear();
                }
                at_field_start = true;
            }
            _ => {
                field.push(character);
                at_field_start = false;
            }
        }
    }
    if quoted {
        return Err("unterminated quoted field".into());
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    Ok(records)
}

/// The title is the text before the first copyright marker. Trailing
/// whitespace and sentence punctuation are removed; every other byte is kept.
fn notice_title(notice: &str) -> Option<String> {
    let copyright = find_ascii_case_insensitive(notice, "copyright");
    let symbol = notice.find('©');
    let marker = match (symbol, copyright) {
        (Some(symbol), Some(copyright)) => symbol.min(copyright),
        (Some(symbol), None) => symbol,
        (None, Some(copyright)) => copyright,
        (None, None) => return None,
    };
    let title = notice[..marker]
        .trim()
        .trim_end_matches([' ', '.', ',', ';', ':', '\t', '\r', '\n'])
        .trim()
        .to_string();
    (!tokens(&title).is_empty()).then_some(title)
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Every notice that matches one publication title in one notice list, in
/// Section 15 order. An empty result means no notice matched.
pub fn match_publication(
    publication: &str,
    notices: &[Notice],
    aliases: &BTreeMap<String, String>,
) -> Result<Vec<Notice>, String> {
    let index = NoticeIndex::new(notices);
    let Some((_, indices)) = index.matches(publication, aliases) else {
        return Ok(Vec::new());
    };
    Ok(indices
        .iter()
        .map(|&index| notices[index].clone())
        .collect())
}

/// Precomputed exact and normalized title indexes for one notice list.
struct NoticeIndex<'a> {
    notices: &'a [Notice],
    exact: BTreeMap<String, Vec<usize>>,
    normalized: BTreeMap<String, Vec<usize>>,
}

impl<'a> NoticeIndex<'a> {
    fn new(notices: &'a [Notice]) -> Self {
        let mut exact: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut normalized: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, notice) in notices.iter().enumerate() {
            exact
                .entry(normalize(&notice.title, false))
                .or_default()
                .push(index);
            let key = normalize(&notice.title, true);
            if !key.is_empty() {
                normalized.entry(key).or_default().push(index);
            }
        }
        Self {
            notices,
            exact,
            normalized,
        }
    }

    fn lookup(&self, table: &BTreeMap<String, Vec<usize>>, key: &str) -> Option<Vec<usize>> {
        table
            .get(key)
            .and_then(|matches| same_publication(matches, self.notices))
    }

    /// Every notice under the target title, including variants the normalizer
    /// folds together. An alias to `Pathfinder Chronicles Campaign Setting`
    /// must also reach `The Pathfinder Chronicles Campaign Setting`, because
    /// the export carries both 2008 statements.
    fn lookup_target(&self, target: &str) -> Option<Vec<usize>> {
        let mut indices: BTreeSet<usize> = self
            .lookup(&self.exact, &normalize(target, false))
            .unwrap_or_default()
            .into_iter()
            .collect();
        let key = normalize(target, true);
        if !key.is_empty()
            && let Some(folded) = self.lookup(&self.normalized, &key)
        {
            indices.extend(folded);
        }
        (!indices.is_empty()).then(|| indices.into_iter().collect())
    }

    /// Exact title, reviewed alias, then the deterministic normalizer.
    ///
    /// A reviewed alias whose target is absent from this particular source is
    /// not an error here: the alias may be satisfied by a later source. The
    /// union is validated once in [`match_publications`].
    fn matches(
        &self,
        publication: &str,
        aliases: &BTreeMap<String, String>,
    ) -> Option<(&'static str, Vec<usize>)> {
        let mut found = self.lookup(&self.exact, &normalize(publication, false));
        let mut method = found.as_ref().map(|_| "exact");
        if found.is_none()
            && let Some(indices) = aliases
                .get(publication)
                .and_then(|target| self.lookup_target(target))
        {
            found = Some(indices);
            method = Some("alias");
        }
        if found.is_none() {
            let key = normalize(publication, true);
            if !key.is_empty()
                && let Some(indices) = self.lookup(&self.normalized, &key)
            {
                found = Some(indices);
                method = Some("normalized-prefix");
            }
        }
        found.map(|indices| (method.unwrap_or("exact"), indices))
    }
}

fn load_scope(path: &Path) -> Result<ScopePolicy, String> {
    if !path.exists() {
        return Ok(ScopePolicy::default());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Match sorted publication counts against every notice source, in source order.
///
/// Within one source the precedence is exact title, reviewed alias, then the
/// deterministic normalizer. Every source is tried so a title covered by more
/// than one list keeps all applicable notices. A title that names several
/// notices matches only when every candidate carries some notice text; several
/// printings of one book therefore all apply.
fn match_publications(
    counts: &BTreeMap<String, usize>,
    sources: &[NoticeSource],
    aliases: &BTreeMap<String, String>,
) -> Result<(Vec<PublicationLegal>, Vec<BTreeSet<usize>>), String> {
    for publication in aliases.keys() {
        if !counts.contains_key(publication) {
            return Err(format!(
                "notice alias names no in-scope publication: {publication}"
            ));
        }
    }
    let indexes: Vec<NoticeIndex<'_>> = sources
        .iter()
        .map(|source| NoticeIndex::new(&source.notices))
        .collect();
    for (publication, target) in aliases {
        if !indexes
            .iter()
            .any(|index| index.lookup_target(target).is_some())
        {
            return Err(format!(
                "notice alias for {publication:?} points to a missing Section 15 title: {target:?}"
            ));
        }
    }

    let mut rows = Vec::with_capacity(counts.len());
    let mut used: Vec<BTreeSet<usize>> = sources.iter().map(|_| BTreeSet::new()).collect();
    for (publication, records) in counts {
        let mut noticed = Vec::new();
        let mut notice_sources = Vec::new();
        let mut method = None;
        for (slot, index) in indexes.iter().enumerate() {
            let Some((source_method, indices)) = index.matches(publication, aliases) else {
                continue;
            };
            let source = sources[slot].id;
            if method.is_none() {
                method = Some(if source == "pf-data" {
                    source_method.to_owned()
                } else {
                    source.to_owned()
                });
            }
            if !notice_sources.iter().any(|seen| seen == source) {
                notice_sources.push(source.to_owned());
            }
            for &notice_index in &indices {
                used[slot].insert(notice_index);
                let notice = sources[slot].notices[notice_index].clone();
                if !noticed.contains(&notice) {
                    noticed.push(notice);
                }
            }
        }
        rows.push(PublicationLegal {
            publication: publication.clone(),
            records: *records,
            match_method: method,
            notice_sources,
            notices: noticed,
        });
    }
    Ok((rows, used))
}

/// Every distinct notice text among the candidates for one title key. Several
/// printings of one book (the Core Rulebook has a 2009 and a 2010 notice) all apply,
/// and Section 15 must carry each, so no candidate is dropped in favour of the first.
fn same_publication(matches: &[usize], notices: &[Notice]) -> Option<Vec<usize>> {
    let mut seen = BTreeSet::new();
    let found: Vec<usize> = matches
        .iter()
        .copied()
        .filter(|index| seen.insert(notices[*index].notice.clone()))
        .collect();
    (!found.is_empty()).then_some(found)
}

/// Lowercase alphanumeric tokens. `and` and `&` describe the same connector.
fn tokens(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty() && *token != "and")
        .map(str::to_owned)
        .collect()
}

const PREFIXES: &[&[&str]] = &[
    &["pathfinder", "roleplaying", "game"],
    &["pathfinder", "rpg"],
    &["pathfinder", "player", "companion"],
    &["pathfinder", "campaign", "setting"],
    &["pathfinder", "module"],
    &["pathfinder", "companion"],
    &["pathfinder", "chronicles"],
    &["pathfinder", "adventure", "path"],
    &["pathfinder", "society"],
    &["gamemastery", "module"],
    &["pathfinder"],
    &["prpg"],
];

fn starts_with(words: &[String], prefix: &[&str]) -> bool {
    words.len() >= prefix.len()
        && words
            .iter()
            .zip(prefix)
            .all(|(word, expected)| word.as_str() == *expected)
}

fn is_number(word: &str) -> bool {
    !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_module_code(word: &str) -> bool {
    let letters = word.bytes().take_while(u8::is_ascii_alphabetic).count();
    (1..=2).contains(&letters)
        && word.len() > letters
        && word[letters..].bytes().all(|byte| byte.is_ascii_digit())
}

fn strip_affixes(mut words: Vec<String>) -> Vec<String> {
    loop {
        let mut changed = false;
        for prefix in PREFIXES {
            if starts_with(&words, prefix) {
                words.drain(..prefix.len());
                changed = true;
            }
        }
        if matches!(
            words.first().map(String::as_str),
            Some("volume" | "scenario")
        ) {
            words.remove(0);
            changed = true;
        }
        if words.first().is_some_and(|word| is_module_code(word)) {
            words.remove(0);
            changed = true;
        }
        if words.len() > 1 && is_number(&words[0]) && words[1] == "the" {
            words.remove(1);
            changed = true;
        }
        if words.first().is_some_and(|word| word == "the") {
            words.remove(0);
            changed = true;
        }
        if !changed {
            return words;
        }
    }
}

fn normalize(value: &str, strip: bool) -> String {
    let words = tokens(value);
    let words = if strip { strip_affixes(words) } else { words };
    words.concat()
}

/// Write the deterministic report. The caller decides whether to commit it.
pub fn write_report(path: &Path, report: &LegalReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(title: &str, text: &str) -> Notice {
        Notice {
            title: title.to_owned(),
            notice: text.to_owned(),
        }
    }

    fn one_source(notices: Vec<Notice>) -> Vec<NoticeSource> {
        vec![NoticeSource {
            id: "pf-data",
            notices,
        }]
    }

    #[test]
    fn every_distinct_notice_for_a_title_key_applies() {
        let notices = vec![
            notice("Core Rulebook", "© 2009 A"),
            notice("Core Rulebook", "© 2010 A"),
            notice("Core Rulebook", "© 2010 A"),
            notice("Pathfinder Core Rulebook", "© 2011 B"),
        ];
        assert_eq!(same_publication(&[0, 1, 2], &notices), Some(vec![0, 1]));
        assert_eq!(same_publication(&[0, 3], &notices), Some(vec![0, 3]));
        assert_eq!(same_publication(&[], &notices), None);
    }

    #[test]
    fn parses_section_15_and_decodes_entities() {
        let input = "before\n**15. COPYRIGHT NOTICE**\n**Core &amp; More** &copy; 2025, Holder.\n## Product Identity\n";
        assert_eq!(
            parse_notices(input).unwrap(),
            vec![notice("Core & More", "© 2025, Holder.")]
        );
    }

    #[test]
    fn requires_both_section_boundaries() {
        assert!(parse_notices("**15. COPYRIGHT NOTICE**\n**Title** Copyright 2025\n").is_err());
    }

    #[test]
    fn normalizes_known_book_prefixes_and_title_variants() {
        assert_eq!(
            normalize("Core Rulebook", true),
            normalize("Pathfinder Roleplaying Game Core Rulebook", true)
        );
        assert_eq!(
            normalize("PRPG Core Rulebook", true),
            normalize("Pathfinder RPG Core Rulebook", true)
        );
        assert_eq!(
            normalize("Pathfinder #25: The Bastards of Erebus", true),
            normalize(
                "Pathfinder Adventure Path volume #25: Bastards of Erebus",
                true
            )
        );
        assert_eq!(
            normalize("Pathfinder Module J1: Entombed with the Pharaohs", true),
            normalize("Entombed with the Pharaohs", true)
        );
        assert_eq!(
            normalize("Faiths and Philosophies", true),
            normalize("Pathfinder Player Companion: Faiths & Philosophies", true)
        );
        assert_eq!(
            normalize("Pathfinder #144: Midwives to Death", true),
            normalize("Pathfinder Adventure Path #144: Midwives to Death", true)
        );
    }

    #[test]
    fn exact_match_wins_and_prefix_match_is_explained() {
        let notices = vec![
            notice(
                "Pathfinder Roleplaying Game Core Rulebook",
                "© 2009, Paizo.",
            ),
            notice("Pathfinder RPG Bestiary", "© 2009, Paizo."),
        ];
        let counts = BTreeMap::from([
            ("Core Rulebook".to_owned(), 2),
            ("Pathfinder RPG Bestiary".to_owned(), 1),
            ("Unknown Book".to_owned(), 1),
        ]);
        let sources = one_source(notices);
        let (rows, used) = match_publications(&counts, &sources, &BTreeMap::new()).unwrap();
        assert_eq!(rows[0].publication, "Core Rulebook");
        assert_eq!(rows[0].match_method.as_deref(), Some("normalized-prefix"));
        assert_eq!(rows[0].notice_sources, ["pf-data"]);
        assert_eq!(rows[1].match_method.as_deref(), Some("exact"));
        assert!(rows[2].notices.is_empty());
        assert!(rows[2].notice_sources.is_empty());
        assert_eq!(used, vec![BTreeSet::from([0, 1])]);
    }

    #[test]
    fn reviewed_alias_must_point_at_a_real_notice() {
        let notices = vec![notice("Pathfinder Module: River Into Darkness", "© 2008.")];
        let counts = BTreeMap::from([("Rivers into Darkness".to_owned(), 1)]);
        let aliases = BTreeMap::from([(
            "Rivers into Darkness".to_owned(),
            "Pathfinder Module: River Into Darkness".to_owned(),
        )]);
        let sources = one_source(notices);
        let (rows, _) = match_publications(&counts, &sources, &aliases).unwrap();
        assert_eq!(rows[0].match_method.as_deref(), Some("alias"));

        let broken =
            BTreeMap::from([("Rivers into Darkness".to_owned(), "Missing Book".to_owned())]);
        assert!(
            match_publications(&counts, &sources, &broken)
                .unwrap_err()
                .contains("missing Section 15 title")
        );
        let unknown = BTreeMap::from([("Other Book".to_owned(), "Anything".to_owned())]);
        assert!(
            match_publications(&counts, &sources, &unknown)
                .unwrap_err()
                .contains("no in-scope publication")
        );
    }

    #[test]
    fn every_source_contributes_and_the_first_one_names_the_method() {
        let sources = vec![
            NoticeSource {
                id: "pf-data",
                notices: vec![notice("Core Rulebook", "© 2009, Paizo.")],
            },
            NoticeSource {
                id: "psrd",
                notices: vec![notice(
                    "Pathfinder Roleplaying Game Core Rulebook",
                    "© 2010, Paizo.",
                )],
            },
            NoticeSource {
                id: "d20pfsrd",
                notices: vec![notice(
                    "Pathfinder Roleplaying Game Core Rulebook",
                    "© 2013, Paizo.",
                )],
            },
        ];
        let counts = BTreeMap::from([("Core Rulebook".to_owned(), 1)]);
        let (rows, used) = match_publications(&counts, &sources, &BTreeMap::new()).unwrap();
        assert_eq!(rows[0].match_method.as_deref(), Some("exact"));
        assert_eq!(rows[0].notice_sources, ["pf-data", "psrd", "d20pfsrd"]);
        assert_eq!(rows[0].notices.len(), 3);
        assert_eq!(used[0], BTreeSet::from([0]));
        assert_eq!(used[1], BTreeSet::from([0]));
        assert_eq!(used[2], BTreeSet::from([0]));
    }

    #[test]
    fn d20_title_stops_before_the_copyright_marker_and_trims_only_whitespace_and_punctuation() {
        assert_eq!(
            notice_title(
                "Pathfinder Campaign Setting: The Inner Sea World Guide. © 2011, Paizo Publishing, LLC; Authors: A, B. "
            )
            .as_deref(),
            Some("Pathfinder Campaign Setting: The Inner Sea World Guide")
        );
        assert_eq!(
            notice_title(
                "Pathfinder Chronicles Campaign Setting. Copyright 2008, Paizo Publishing, LLC; Authors: A."
            )
            .as_deref(),
            Some("Pathfinder Chronicles Campaign Setting")
        );
        assert_eq!(
            notice_title("Tome of Horrors Complete. © 2011, Necromancer Games, Inc.").as_deref(),
            Some("Tome of Horrors Complete")
        );
        assert_eq!(
            notice_title("Kobold Quarterly Blog, 2010, http://example"),
            None
        );
    }

    #[test]
    fn csv_reader_keeps_quoted_commas_and_line_breaks() {
        let rows = parse_csv("1,\"a, b\"\n2,\"line one\nline two\"\n3,plain").unwrap();
        assert_eq!(
            rows,
            vec![
                vec!["1".to_owned(), "a, b".to_owned()],
                vec!["2".to_owned(), "line one\nline two".to_owned()],
                vec!["3".to_owned(), "plain".to_owned()],
            ]
        );
        assert!(parse_csv("1,\"unterminated").is_err());
    }

    #[test]
    fn d20_source_is_verified_against_its_provenance() {
        let root = std::env::temp_dir().join(format!("pf-legal-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let csv = root.join("notice.csv");
        fs::write(&csv, "1,\"Book. © 2020, Holder.\"\n").unwrap();
        let digest = hex(&Sha256::digest(fs::read(&csv).unwrap()));
        let provenance = root.join("notice.provenance.json");
        fs::write(
            &provenance,
            serde_json::to_vec(&serde_json::json!({
                "id": "d20pfsrd-section15",
                "source": "d20pfsrd.com Section 15 statements",
                "supplied": "2026-11-05",
                "csv": "notice.csv",
                "sha256": digest,
                "rows": 1,
                "note": "test"
            }))
            .unwrap(),
        )
        .unwrap();
        let (notices, unparsed) = load_d20pfsrd(&csv, &provenance).unwrap();
        assert_eq!(notices, vec![notice("Book", "Book. © 2020, Holder.")]);
        assert!(unparsed.is_empty());

        fs::write(&csv, "1,Changed. © 2020, Holder.\n").unwrap();
        let error = load_d20pfsrd(&csv, &provenance).unwrap_err();
        assert!(error.contains("SHA256 mismatch"), "{error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gate_blocks_only_when_an_in_scope_publication_is_missing() {
        let matched = PublicationLegal {
            publication: "Core Rulebook".into(),
            records: 1,
            match_method: Some("exact".into()),
            notice_sources: vec!["pf-data".into()],
            notices: vec![notice("Core Rulebook", "© 2009.")],
        };
        let missing = PublicationLegal {
            publication: "Lost Book".into(),
            records: 3,
            match_method: None,
            notice_sources: Vec::new(),
            notices: Vec::new(),
        };
        let report = LegalReport {
            dataset: "pathfinder-data-1-e".into(),
            commit: "a".repeat(40),
            in_scope_records: 4,
            publication_count: 2,
            matched_publication_count: 1,
            unmatched_publication_count: 1,
            publications: vec![matched.clone(), missing],
            excluded_publications: Vec::new(),
            notice_sources: Vec::new(),
            unparsed_notices: Vec::new(),
            unmatched_notices: Vec::new(),
        };
        let error = gate(&report).unwrap_err();
        assert!(error.contains("Lost Book"), "{error}");
        assert!(!error.contains("Core Rulebook"), "{error}");

        let complete = LegalReport {
            publication_count: 1,
            matched_publication_count: 1,
            unmatched_publication_count: 0,
            publications: vec![matched],
            ..report
        };
        assert!(gate(&complete).is_ok());
    }

    #[test]
    fn real_inventory_report_is_complete_and_stable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let dataset = root.join(".cache/sources/pathfinder-data-1-e");
        if !dataset.join("json").is_dir() {
            eprintln!("skipping: the pinned primary source is not present");
            return;
        }
        let registry = root.join("sources/pathfinder-data-1-e/source.json");
        let inventory = crate::pf_data::inventory(&dataset, &registry).expect("inventory");
        let config = root.join("sources/pathfinder-data-1-e");
        let section15 = config.join("section15");
        let inputs = Inputs {
            opengamecontent: &dataset.join("OPENGAMECONTENT.md"),
            psrd_notices: Some(&root.join("sources/psrd-data/legal-notices.json")),
            aliases: &config.join("notice-aliases.json"),
            out_of_scope: &config.join("out-of-scope.json"),
            d20pfsrd_csv: &section15.join("d20pfsrd-section15.csv"),
            d20pfsrd_provenance: &section15.join("d20pfsrd-section15.provenance.json"),
        };
        let report = build(&inventory, &inputs).expect("legal report");
        assert_eq!(report.publication_count, 453);
        assert_eq!(report.matched_publication_count, 431);
        assert_eq!(report.unmatched_publication_count, 22);
        assert_eq!(report.blocking_publications().len(), 22);
        for (publication, source, title) in [
            (
                "Heroes of the Streets",
                "alias",
                "Pathfinder Player Companion: Heroes of the Street",
            ),
            (
                "Pathfinder #119: Prisoners of the Blight",
                "alias",
                "Pathfinder Adventure Path #119: Prisoner of the Blight",
            ),
            (
                "Realm of the Fellnight Queen",
                "alias",
                "Pathfinder Module: Realm of the Fellknight Queen",
            ),
            (
                "Pathfinder Campaign Setting",
                "d20pfsrd",
                "Pathfinder Chronicles Campaign Setting",
            ),
            (
                "Revenge of the Kobold King",
                "d20pfsrd",
                "Pathfinder Module D1.5: Revenge of the Kobold King",
            ),
            (
                "Bestiary 5",
                "d20pfsrd",
                "Pathfinder Roleplaying Game Bestiary 5",
            ),
            (
                "Advanced Race Guide",
                "psrd",
                "Pathfinder Roleplaying Game Advanced Race Guide",
            ),
        ] {
            let row = report
                .publications
                .iter()
                .find(|row| row.publication == publication)
                .expect("reviewed publication");
            assert_eq!(row.match_method.as_deref(), Some(source), "{publication}");
            assert!(
                row.notices.iter().any(|notice| notice.title == title),
                "{publication} has no notice titled {title:?}"
            );
        }
        for (publication, expected) in [("Pathfinder Campaign Setting", 2), ("Rival Guide", 2)] {
            let row = report
                .publications
                .iter()
                .find(|row| row.publication == publication)
                .expect("multi-notice publication");
            assert_eq!(
                row.notices.len(),
                expected,
                "{publication} must keep every distinct statement"
            );
        }
        let again = build(&inventory, &inputs).unwrap();
        assert_eq!(report, again);
    }
}
