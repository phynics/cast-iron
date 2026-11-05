//! Section 15 notice coverage for the Pf Data publication registry.
//!
//! This module only reports and gates licensing evidence. It never writes the
//! production legal page or content library.

use crate::html;
use crate::pf_data::Inventory;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// One Section 15 entry, already decoded from the pinned OPENGAMECONTENT.md.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Notice {
    pub title: String,
    pub notice: String,
}

/// The Section 15 evidence for one publication referenced by an in-scope record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublicationLegal {
    pub publication: String,
    pub records: usize,
    /// `exact`, `normalized-prefix`, or `alias`; absent when no notice matched.
    pub match_method: Option<String>,
    /// Every applicable notice; empty when no notice matched.
    pub notices: Vec<Notice>,
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
    pub unmatched_notices: Vec<Notice>,
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

/// Build the legal evidence from the inventory, the pinned notice, and reviewed aliases.
pub fn build(
    inventory: &Inventory,
    opengamecontent: &Path,
    aliases_path: &Path,
    out_of_scope_path: &Path,
) -> Result<LegalReport, String> {
    let text = fs::read_to_string(opengamecontent)
        .map_err(|e| format!("{}: {e}", opengamecontent.display()))?;
    let notices = parse_notices(&text)?;
    let aliases = load_aliases(aliases_path)?;
    let scope = load_scope(out_of_scope_path)?;
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
    let (publications, used) = match_publications(&counts, &notices, &aliases)?;
    let matched_publication_count = publications
        .iter()
        .filter(|publication| !publication.notices.is_empty())
        .count();
    let unmatched_publication_count = publications.len() - matched_publication_count;
    let matched_titles: BTreeSet<String> = used
        .iter()
        .map(|&index| notices[index].title.clone())
        .collect();
    let mut unmatched_notices: Vec<Notice> = notices
        .into_iter()
        .filter(|notice| !matched_titles.contains(notice.title.as_str()))
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
        unmatched_notices,
    })
}

fn load_aliases(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: AliasFile =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file.aliases)
}

/// Every notice that matches one publication title, in Section 15 order.
///
/// This is the single-publication form of [`build`], used when the canonical
/// legal page unions PSRD and Pf Data notices. An empty result means no notice
/// matched.
pub fn match_publication(
    publication: &str,
    notices: &[Notice],
    aliases: &BTreeMap<String, String>,
) -> Result<Vec<Notice>, String> {
    let index = NoticeIndex::new(notices);
    let Some((_, indices)) = index.matches(publication, aliases)? else {
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

    /// Exact title, reviewed alias, then the deterministic normalizer.
    fn matches(
        &self,
        publication: &str,
        aliases: &BTreeMap<String, String>,
    ) -> Result<Option<(&'static str, Vec<usize>)>, String> {
        let mut found = self.lookup(&self.exact, &normalize(publication, false));
        let mut method = found.as_ref().map(|_| "exact");
        if found.is_none()
            && let Some(target) = aliases.get(publication)
        {
            let key = normalize(target, true);
            let target_found = self
                .lookup(&self.exact, &normalize(target, false))
                .or_else(|| {
                    (!key.is_empty())
                        .then(|| self.lookup(&self.normalized, &key))
                        .flatten()
                });
            match target_found {
                Some(indices) => {
                    found = Some(indices);
                    method = Some("alias");
                }
                None => {
                    return Err(format!(
                        "notice alias for {publication:?} points to a missing Section 15 title: {target:?}"
                    ));
                }
            }
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
        Ok(found.map(|indices| (method.unwrap_or("exact"), indices)))
    }
}

fn load_scope(path: &Path) -> Result<ScopePolicy, String> {
    if !path.exists() {
        return Ok(ScopePolicy::default());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Match sorted publication counts against the notice list.
///
/// Precedence is exact title, reviewed alias, then the deterministic normalizer.
/// A title that names several notices matches only when every candidate carries the
/// same notice text; otherwise the publication stays unmatched for review.
fn match_publications(
    counts: &BTreeMap<String, usize>,
    notices: &[Notice],
    aliases: &BTreeMap<String, String>,
) -> Result<(Vec<PublicationLegal>, BTreeSet<usize>), String> {
    for publication in aliases.keys() {
        if !counts.contains_key(publication) {
            return Err(format!(
                "notice alias names no in-scope publication: {publication}"
            ));
        }
    }
    let index = NoticeIndex::new(notices);
    let mut rows = Vec::with_capacity(counts.len());
    let mut used = BTreeSet::new();
    for (publication, records) in counts {
        let found = index.matches(publication, aliases)?;
        let (method, indices) = match found {
            Some((method, indices)) => (Some(method.to_owned()), indices),
            None => (None, Vec::new()),
        };
        used.extend(indices.iter().copied());
        rows.push(PublicationLegal {
            publication: publication.clone(),
            records: *records,
            match_method: method,
            notices: indices
                .iter()
                .map(|&index| notices[index].clone())
                .collect(),
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
        let (rows, used) = match_publications(&counts, &notices, &BTreeMap::new()).unwrap();
        assert_eq!(rows[0].publication, "Core Rulebook");
        assert_eq!(rows[0].match_method.as_deref(), Some("normalized-prefix"));
        assert_eq!(rows[1].match_method.as_deref(), Some("exact"));
        assert!(rows[2].notices.is_empty());
        assert_eq!(used, BTreeSet::from([0, 1]));
    }

    #[test]
    fn reviewed_alias_must_point_at_a_real_notice() {
        let notices = vec![notice("Pathfinder Module: River Into Darkness", "© 2008.")];
        let counts = BTreeMap::from([("Rivers into Darkness".to_owned(), 1)]);
        let aliases = BTreeMap::from([(
            "Rivers into Darkness".to_owned(),
            "Pathfinder Module: River Into Darkness".to_owned(),
        )]);
        let (rows, _) = match_publications(&counts, &notices, &aliases).unwrap();
        assert_eq!(rows[0].match_method.as_deref(), Some("alias"));

        let broken =
            BTreeMap::from([("Rivers into Darkness".to_owned(), "Missing Book".to_owned())]);
        assert!(
            match_publications(&counts, &notices, &broken)
                .unwrap_err()
                .contains("missing Section 15 title")
        );
        let unknown = BTreeMap::from([("Other Book".to_owned(), "Anything".to_owned())]);
        assert!(
            match_publications(&counts, &notices, &unknown)
                .unwrap_err()
                .contains("no in-scope publication")
        );
    }

    #[test]
    fn gate_blocks_only_when_an_in_scope_publication_is_missing() {
        let matched = PublicationLegal {
            publication: "Core Rulebook".into(),
            records: 1,
            match_method: Some("exact".into()),
            notices: vec![notice("Core Rulebook", "© 2009.")],
        };
        let missing = PublicationLegal {
            publication: "Lost Book".into(),
            records: 3,
            match_method: None,
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
        let report = build(
            &inventory,
            &dataset.join("OPENGAMECONTENT.md"),
            &config.join("notice-aliases.json"),
            &config.join("out-of-scope.json"),
        )
        .expect("legal report");
        assert_eq!(report.publication_count, 453);
        assert_eq!(report.matched_publication_count, 400);
        assert_eq!(report.unmatched_publication_count, 53);
        assert_eq!(report.blocking_publications().len(), 53);
        for (publication, title) in [
            (
                "Heroes of the Streets",
                "Pathfinder Player Companion: Heroes of the Street",
            ),
            (
                "Pathfinder #119: Prisoners of the Blight",
                "Pathfinder Adventure Path #119: Prisoner of the Blight",
            ),
            (
                "Realm of the Fellnight Queen",
                "Pathfinder Module: Realm of the Fellknight Queen",
            ),
        ] {
            let row = report
                .publications
                .iter()
                .find(|row| row.publication == publication)
                .expect("reviewed alias publication");
            assert_eq!(row.match_method.as_deref(), Some("alias"));
            assert_eq!(row.notices.len(), 1);
            assert_eq!(row.notices[0].title, title);
        }
        let again = build(
            &inventory,
            &dataset.join("OPENGAMECONTENT.md"),
            &config.join("notice-aliases.json"),
            &config.join("out-of-scope.json"),
        )
        .unwrap();
        assert_eq!(report, again);
    }
}
