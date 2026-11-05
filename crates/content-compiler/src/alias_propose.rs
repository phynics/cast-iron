//! Proposes dispositions for the `no_source` rows of `sources/canonical/id-map.toml`.
//!
//! The proposal tool is deliberately separate from generation. It reads the same
//! frozen snapshot and canonical catalog the alias gate uses, matches a legacy
//! `(kind, title, source)` tuple against a canonical record through
//! [`crate::alias::title_fit`], and writes a reviewable map plus a JSON report.
//! It never guesses from a slug alone.
//!
//! Matching rules, strongest evidence first:
//!
//! 1. The canonical kind must equal the legacy kind.
//! 2. The legacy source book must appear in the record's publication set.
//! 3. The titles must be equal after [`crate::alias::normalize_title`], or differ
//!    only by a comma inversion, a trailing family/category parenthetical, or both.
//!    Item parentheticals must be a measure/package or the weapon/armor slot label.
//! 4. A tie is resolved only by a documented deterministic rule:
//!    * a native record beats a derived duplicate;
//!    * a racial trait whose parent race is named by the legacy id path wins;
//!    * a `weapon-special-abilities` or `armor-special-abilities` path picks the
//!      candidate whose parenthetical or id names that slot.
//!      Everything else stays retired with an `ambiguous: <candidates>` note.
//!
//! Applying the proposals is idempotent: a second run finds no `no_source` rows
//! to change and rewrites the same bytes.

use crate::alias::{
    CanonicalCatalog, CanonicalRecord, IdMap, LegacyEntry, LegacySnapshot, RetirementReason,
    TitleFit,
};
use cast_iron_schema::EntryId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One proposed `[map]` or `[shared]` row.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub old: String,
    pub target: String,
    /// Upstream identity for a `record` or `derived` map row.
    pub identity: String,
    /// `native`, `derived:<kind>`, or `carried`.
    pub origin: String,
    pub fit: TitleFit,
    /// `title`, `native-over-derived`, `race-parent`, `weapon-variant`, or
    /// `armor-variant`.
    pub evidence: &'static str,
    pub books: Vec<String>,
    pub shared: bool,
}

/// One legacy id that matched more than one canonical record.
#[derive(Debug, Clone)]
pub struct Ambiguous {
    pub old: String,
    pub candidates: Vec<String>,
    pub books: Vec<String>,
}

/// A proposal that would merge onto a target an existing `[map]` row already
/// claims. The existing row is published and append-only, so the new id stays
/// retired instead of rewriting it as `[shared]`.
#[derive(Debug, Clone)]
pub struct Skipped {
    pub old: String,
    pub target: String,
    pub existing: String,
}

/// Counts used by the command report and the PR body.
#[derive(Debug, Clone, Default)]
pub struct KindReasonCount {
    pub kind: String,
    pub reason: String,
    pub count: usize,
}

/// The whole proposal run.
#[derive(Debug, Clone, Default)]
pub struct ProposalReport {
    pub legacy: usize,
    pub mapped_before: usize,
    pub shared_before: usize,
    pub retired_before: usize,
    pub retired_before_by_reason: BTreeMap<String, usize>,
    pub proposals: Vec<Proposal>,
    pub ambiguous: Vec<Ambiguous>,
    pub no_match: Vec<String>,
    pub skipped: Vec<Skipped>,
    pub mapped_after: usize,
    pub shared_after: usize,
    pub retired_after: usize,
    pub retired_after_by_reason: BTreeMap<String, usize>,
    pub remaining_by_kind: Vec<KindReasonCount>,
    pub proposed_by_kind: BTreeMap<String, usize>,
    pub evidence_counts: BTreeMap<String, usize>,
}

impl ProposalReport {
    pub fn report(&self) -> String {
        let before = reason_counts(&self.retired_before_by_reason);
        let after = reason_counts(&self.retired_after_by_reason);
        let evidence: Vec<String> = self
            .evidence_counts
            .iter()
            .map(|(name, count)| format!("{name} {count}"))
            .collect();
        format!(
            "legacy {}: mapped {} -> {}, shared {} -> {}, retired {} -> {}; proposed {} (map {}, shared {}), ambiguous {}, no match {}, already claimed {}; evidence {}; before [{}]; after [{}]",
            self.legacy,
            self.mapped_before,
            self.mapped_after,
            self.shared_before,
            self.shared_after,
            self.retired_before,
            self.retired_after,
            self.proposals.len(),
            self.proposals.iter().filter(|row| !row.shared).count(),
            self.proposals.iter().filter(|row| row.shared).count(),
            self.ambiguous.len(),
            self.no_match.len(),
            self.skipped.len(),
            evidence.join(", "),
            before,
            after,
        )
    }
}

fn reason_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(reason, count)| format!("{reason} {count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Builds proposals for every `[retired]` row whose reason is `no_source`.
pub fn propose(
    map: &IdMap,
    snapshot: &LegacySnapshot,
    catalog: &CanonicalCatalog,
) -> Result<ProposalReport, String> {
    let mut report = ProposalReport {
        legacy: snapshot.count,
        mapped_before: map.map.len() + map.shared.len(),
        shared_before: map.shared.len(),
        retired_before: map.retired.len(),
        ..Default::default()
    };
    for row in map.retired.values() {
        *report
            .retired_before_by_reason
            .entry(row.reason.slug().to_string())
            .or_default() += 1;
    }

    // Index every canonical title form once. The stored rank is the strength of
    // the spelling that produced the form, so an exact title always beats a
    // parenthetical variant.
    let mut index: BTreeMap<(String, String), Vec<(&CanonicalRecord, u8)>> = BTreeMap::new();
    for record in catalog.iter() {
        for (form, rank) in crate::alias::title_forms(&record.title, &record.kind) {
            index
                .entry((record.kind.clone(), form))
                .or_default()
                .push((record, rank));
        }
    }

    let existing_claims = existing_claims(map, catalog);
    let existing_shared: BTreeSet<&str> = map.shared.keys().map(String::as_str).collect();
    let mut proposed: Vec<Proposal> = Vec::new();

    for entry in &snapshot.entries {
        let Some(retired) = map.retired.get(&entry.id) else {
            continue;
        };
        if retired.reason != RetirementReason::NoSource {
            continue;
        }
        let mut best: BTreeMap<&str, (&CanonicalRecord, u8)> = BTreeMap::new();
        let mut best_rank = u8::MAX;
        for (form, legacy_rank) in crate::alias::title_forms(&entry.title, &entry.kind) {
            let Some(candidates) = index.get(&(entry.kind.clone(), form)) else {
                continue;
            };
            for (record, record_rank) in candidates {
                if !record
                    .source_ids
                    .iter()
                    .any(|source| source == &entry.source)
                {
                    continue;
                }
                if crate::alias::title_fit(&entry.title, &record.title, &entry.kind).is_none() {
                    continue;
                }
                let rank = legacy_rank.max(*record_rank);
                if rank < best_rank {
                    best.clear();
                    best.insert(record.id.as_str(), (*record, rank));
                    best_rank = rank;
                } else if rank == best_rank {
                    best.insert(record.id.as_str(), (*record, rank));
                }
            }
        }
        match best.len() {
            0 => report.no_match.push(entry.id.clone()),
            1 => {
                let (_, (record, _)) = best.into_iter().next().expect("one candidate");
                proposed.push(proposal_for(entry, record, "title"));
            }
            _ => {
                let candidates: Vec<&CanonicalRecord> =
                    best.values().map(|(record, _)| *record).collect();
                if let Some((record, evidence)) = tie_break(entry, &candidates) {
                    proposed.push(proposal_for(entry, record, evidence));
                } else {
                    report.ambiguous.push(Ambiguous {
                        old: entry.id.clone(),
                        candidates: candidates.iter().map(|row| row.id.clone()).collect(),
                        books: vec![entry.source.clone()],
                    });
                }
            }
        }
    }

    // Group by target. A target with more than one legacy id needs every row to
    // be `[shared]`. An existing non-shared row cannot be changed, so a proposal
    // that would merge onto it is skipped and leaves the row retired.
    let mut groups: BTreeMap<&str, Vec<&Proposal>> = BTreeMap::new();
    for proposal in &proposed {
        groups
            .entry(proposal.target.as_str())
            .or_default()
            .push(proposal);
    }
    let mut accepted = Vec::new();
    for (target, rows) in groups {
        let existing = existing_claims.get(target).cloned().unwrap_or_default();
        let existing_non_shared = existing
            .iter()
            .any(|old| !existing_shared.contains(old.as_str()));
        if existing_non_shared {
            let existing = existing.first().cloned().unwrap_or_default();
            for row in rows {
                report.skipped.push(Skipped {
                    old: row.old.clone(),
                    target: target.to_string(),
                    existing: existing.clone(),
                });
            }
            continue;
        }
        let shared = existing.len() + rows.len() > 1;
        let mut books: BTreeSet<String> = existing
            .iter()
            .filter_map(|old| {
                snapshot
                    .entries
                    .iter()
                    .find(|entry| &entry.id == old)
                    .map(|entry| entry.source.clone())
            })
            .collect();
        for row in &rows {
            books.extend(row.books.iter().cloned());
        }
        let books: Vec<String> = books.into_iter().collect();
        for row in rows {
            let mut row = row.clone();
            row.shared = shared;
            if shared {
                row.books = books.clone();
            }
            accepted.push(row);
        }
    }
    accepted.sort_by(|a, b| a.old.cmp(&b.old));
    report.proposals = accepted;

    for row in &report.proposals {
        *report
            .proposed_by_kind
            .entry(kind_of(snapshot, &row.old))
            .or_default() += 1;
        *report
            .evidence_counts
            .entry(row.evidence.to_string())
            .or_default() += 1;
    }

    report.mapped_after = report.mapped_before + report.proposals.len();
    report.shared_after =
        report.shared_before + report.proposals.iter().filter(|row| row.shared).count();
    report.retired_after = report.retired_before.saturating_sub(report.proposals.len());
    report.retired_after_by_reason = report.retired_before_by_reason.clone();
    let proposed = report.proposals.len();
    let no_source = report
        .retired_after_by_reason
        .entry("no_source".to_string())
        .or_default();
    *no_source = no_source.saturating_sub(proposed);

    // Remaining retired rows by kind and reason after the proposals apply.
    let proposed_ids: BTreeSet<&str> = report
        .proposals
        .iter()
        .map(|row| row.old.as_str())
        .collect();
    let mut remaining: BTreeMap<(String, String), usize> = BTreeMap::new();
    let ambiguous: BTreeSet<&str> = report
        .ambiguous
        .iter()
        .map(|row| row.old.as_str())
        .collect();
    for entry in &snapshot.entries {
        if proposed_ids.contains(entry.id.as_str())
            || map.map.contains_key(&entry.id)
            || map.shared.contains_key(&entry.id)
        {
            continue;
        }
        let reason = if ambiguous.contains(entry.id.as_str()) {
            "no_source".to_string()
        } else {
            map.retired
                .get(&entry.id)
                .map(|row| row.reason.slug().to_string())
                .unwrap_or_else(|| "no_source".to_string())
        };
        *remaining.entry((entry.kind.clone(), reason)).or_default() += 1;
    }
    report.remaining_by_kind = remaining
        .into_iter()
        .map(|((kind, reason), count)| KindReasonCount {
            kind,
            reason,
            count,
        })
        .collect();
    Ok(report)
}

fn kind_of(snapshot: &LegacySnapshot, old: &str) -> String {
    snapshot
        .entries
        .iter()
        .find(|entry| entry.id == old)
        .map(|entry| entry.kind.clone())
        .unwrap_or_else(|| "unknown".into())
}

fn proposal_for(entry: &LegacyEntry, record: &CanonicalRecord, evidence: &'static str) -> Proposal {
    Proposal {
        old: entry.id.clone(),
        target: record.id.clone(),
        identity: record.identity.clone(),
        origin: record.origin.clone(),
        fit: crate::alias::title_fit(&entry.title, &record.title, &entry.kind)
            .expect("proposal is built from a matching title"),
        evidence,
        books: vec![entry.source.clone()],
        shared: false,
    }
}

/// Collects the canonical target of every existing row. A missing identity is
/// left out; generation reports it separately.
pub(crate) fn existing_claims(
    map: &IdMap,
    catalog: &CanonicalCatalog,
) -> BTreeMap<String, Vec<String>> {
    let mut claims: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (old, row) in &map.map {
        let target = if let Some(identity) = row.record.as_deref() {
            catalog.canonical_for_identity(identity)
        } else if let Some(identity) = row.derived.as_deref() {
            catalog.canonical_for_identity(identity)
        } else {
            row.target
                .as_deref()
                .filter(|target| catalog.contains(target))
        };
        if let Some(target) = target {
            claims
                .entry(target.to_string())
                .or_default()
                .push(old.clone());
        }
    }
    for (old, row) in &map.shared {
        if catalog.contains(&row.target) {
            claims
                .entry(row.target.clone())
                .or_default()
                .push(old.clone());
        }
    }
    claims
}

/// The documented deterministic tie-breaks. Returns `None` when the evidence
/// cannot separate the candidates.
fn tie_break<'a>(
    entry: &LegacyEntry,
    candidates: &[&'a CanonicalRecord],
) -> Option<(&'a CanonicalRecord, &'static str)> {
    let native: Vec<&&CanonicalRecord> = candidates
        .iter()
        .filter(|record| record.origin == "native")
        .collect();
    if native.len() == 1 && native.len() < candidates.len() {
        return Some((native[0], "native-over-derived"));
    }
    if entry.kind == "racial_trait"
        && let Some(token) = race_token(&entry.id)
    {
        let matches: Vec<&&CanonicalRecord> = candidates
            .iter()
            .filter(|record| {
                race_of(&record.id).is_some_and(|race| race_token_matches(token, race))
            })
            .collect();
        if matches.len() == 1 {
            return Some((matches[0], "race-parent"));
        }
    }
    let segments: BTreeSet<&str> = entry.id.split('/').collect();
    for slot in ["weapon", "armor"] {
        let segment = format!("{slot}-special-abilities");
        if !segments.contains(segment.as_str()) {
            continue;
        }
        let matches: Vec<&&CanonicalRecord> = candidates
            .iter()
            .filter(|record| {
                record
                    .title
                    .to_ascii_lowercase()
                    .contains(&format!("({slot})"))
                    || record.id.ends_with(&format!("-{slot}"))
            })
            .collect();
        if matches.len() == 1 {
            return Some((
                matches[0],
                if slot == "weapon" {
                    "weapon-variant"
                } else {
                    "armor-variant"
                },
            ));
        }
    }
    None
}

fn race_token(old: &str) -> Option<&str> {
    let parts: Vec<&str> = old.split('/').collect();
    parts
        .iter()
        .position(|part| *part == "races")
        .and_then(|index| parts.get(index + 1).copied())
}

fn race_of(canonical_id: &str) -> Option<&str> {
    let parts: Vec<&str> = canonical_id.split('/').collect();
    if parts.len() > 3 && parts[1] == "racial-trait" {
        Some(parts[2])
    } else {
        None
    }
}

fn race_token_matches(token: &str, race: &str) -> bool {
    let token = token.to_ascii_lowercase();
    let singular = match token.as_str() {
        "elves" => "elf",
        "dwarves" => "dwarf",
        "gnomes" => "gnome",
        "goblins" => "goblin",
        "halflings" => "halfling",
        "humans" => "human",
        "half-elves" => "half-elf",
        "half-orcs" => "half-orc",
        "merfolk" => "merfolk",
        _ => token.as_str(),
    };
    singular == race || token == race || token == format!("{race}s") || token == format!("{race}es")
}

/// Text for the merged map. Existing blocks are copied verbatim so an already
/// published row is not rewritten. New rows are inserted in sorted order.
pub fn merged_map_text(original: &str, report: &ProposalReport) -> Result<String, String> {
    let sections = Sections::parse(original)?;
    let mut map_blocks = sections.map;
    let mut shared_blocks = sections.shared;
    let mut retired_blocks = sections.retired;

    for proposal in &report.proposals {
        retired_blocks.remove(&proposal.old);
        if proposal.shared {
            shared_blocks.insert(proposal.old.clone(), shared_block(proposal));
        } else {
            map_blocks.insert(proposal.old.clone(), map_block(proposal));
        }
    }
    for ambiguous in &report.ambiguous {
        if let Some(block) = retired_blocks.get_mut(&ambiguous.old) {
            *block = retired_block(
                &ambiguous.old,
                RetirementReason::NoSource,
                Some(&format!("ambiguous: {}", ambiguous.candidates.join(", "))),
            );
        }
    }

    let mut out = sections.preamble;
    out.push_str(&sections.meta);
    push_section(&mut out, &map_blocks);
    push_section(&mut out, &shared_blocks);
    push_section(&mut out, &retired_blocks);
    Ok(out)
}

fn push_section(out: &mut String, blocks: &BTreeMap<String, String>) {
    for block in blocks.values() {
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(block);
    }
}

pub(crate) fn map_block(proposal: &Proposal) -> String {
    let mut block = format!("[map.\"{}\"]\n", escape(&proposal.old));
    if proposal.origin == "native" {
        block.push_str(&format!("record = \"{}\"\n", escape(&proposal.identity)));
    } else if proposal.origin.starts_with("derived:") {
        block.push_str(&format!("derived = \"{}\"\n", escape(&proposal.identity)));
    } else {
        block.push_str(&format!("target = \"{}\"\n", escape(&proposal.target)));
    }
    block
}

pub(crate) fn shared_block(proposal: &Proposal) -> String {
    format!(
        "[shared.\"{}\"]\ntarget = \"{}\"\nnote = \"one canonical record shared by legacy books {}; migration merges these rows\"\n",
        escape(&proposal.old),
        escape(&proposal.target),
        escape(&proposal.books.join(", ")),
    )
}

fn retired_block(old: &str, reason: RetirementReason, note: Option<&str>) -> String {
    let mut block = format!(
        "[retired.\"{}\"]\nreason = \"{}\"\n",
        escape(old),
        reason.slug()
    );
    if let Some(note) = note {
        block.push_str(&format!("note = \"{}\"\n", escape(note)));
    }
    block
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Debug)]
struct Sections {
    preamble: String,
    meta: String,
    map: BTreeMap<String, String>,
    shared: BTreeMap<String, String>,
    retired: BTreeMap<String, String>,
}

impl Sections {
    fn parse(text: &str) -> Result<Self, String> {
        let lines: Vec<&str> = text.lines().collect();
        let Some(first) = lines.iter().position(|line| line.starts_with('[')) else {
            return Err("id map has no tables".into());
        };
        let mut preamble = lines[..first].join("\n");
        if !preamble.is_empty() {
            preamble.push('\n');
        }
        let mut sections = Self {
            preamble,
            meta: String::new(),
            map: BTreeMap::new(),
            shared: BTreeMap::new(),
            retired: BTreeMap::new(),
        };
        let mut index = first;
        while index < lines.len() {
            let header = lines[index];
            let mut end = index + 1;
            while end < lines.len() && !lines[end].starts_with('[') {
                end += 1;
            }
            let mut body: Vec<&str> = lines[index..end].to_vec();
            while body.len() > 1 && body.last().is_some_and(|line| line.trim().is_empty()) {
                body.pop();
            }
            let block = format!("{}\n", body.join("\n"));
            if header == "[meta]" {
                sections.meta = block;
            } else if let Some(key) = table_key(header, "map") {
                sections.map.insert(key, block);
            } else if let Some(key) = table_key(header, "shared") {
                sections.shared.insert(key, block);
            } else if let Some(key) = table_key(header, "retired") {
                sections.retired.insert(key, block);
            } else {
                return Err(format!("unrecognized id map table {header:?}"));
            }
            index = end;
        }
        Ok(sections)
    }
}

fn table_key(header: &str, section: &str) -> Option<String> {
    let prefix = format!("[{section}.\"");
    if !header.starts_with(&prefix) || !header.ends_with("\"]") {
        return None;
    }
    Some(header[prefix.len()..header.len() - 2].to_string())
}

/// Writes the merged map and its JSON report. Both are deterministic.
pub fn write(
    report: &ProposalReport,
    original: &str,
    map_path: &Path,
    output_dir: &Path,
    apply: bool,
) -> Result<(), String> {
    let merged = merged_map_text(original, report)?;
    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    std::fs::write(output_dir.join("id-map.proposed.toml"), &merged).map_err(|e| e.to_string())?;
    let json = serde_json::json!({
        "legacy": report.legacy,
        "mapped_before": report.mapped_before,
        "shared_before": report.shared_before,
        "retired_before": report.retired_before,
        "retired_before_by_reason": report.retired_before_by_reason,
        "mapped_after": report.mapped_after,
        "shared_after": report.shared_after,
        "retired_after": report.retired_after,
        "retired_after_by_reason": report.retired_after_by_reason,
        "proposed_by_kind": report.proposed_by_kind,
        "evidence_counts": report.evidence_counts,
        "proposals": report.proposals.iter().map(|row| serde_json::json!({
            "old": row.old,
            "target": row.target,
            "identity": row.identity,
            "origin": row.origin,
            "fit": format!("{:?}", row.fit),
            "evidence": row.evidence,
            "books": row.books,
            "shared": row.shared,
        })).collect::<Vec<_>>(),
        "ambiguous": report.ambiguous.iter().map(|row| serde_json::json!({
            "old": row.old,
            "candidates": row.candidates,
            "books": row.books,
        })).collect::<Vec<_>>(),
        "no_match": report.no_match,
        "skipped": report.skipped.iter().map(|row| serde_json::json!({
            "old": row.old,
            "target": row.target,
            "existing": row.existing,
        })).collect::<Vec<_>>(),
        "remaining_by_kind": report.remaining_by_kind.iter().map(|row| serde_json::json!({
            "kind": row.kind,
            "reason": row.reason,
            "count": row.count,
        })).collect::<Vec<_>>(),
    });
    let mut bytes = serde_json::to_vec_pretty(&json).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    std::fs::write(output_dir.join("alias-proposals.json"), bytes).map_err(|e| e.to_string())?;
    if apply {
        std::fs::write(map_path, merged).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Checks that every proposed target is a valid `EntryId`. Kept separate so a
/// malformed catalog fails before any file is written.
pub fn validate_targets(report: &ProposalReport) -> Result<(), String> {
    for proposal in &report.proposals {
        EntryId::parse(&proposal.target)
            .map_err(|e| format!("proposal {} target: {e}", proposal.old))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alias::{CanonicalRecord, MapRow};

    fn snapshot() -> (LegacySnapshot, Vec<u8>) {
        let bytes = br#"{
          "format": 1,
          "count": 4,
          "entries": [
            {"id": "bestiary-2/monsters/aeon/akhana", "kind": "creature", "title": "Aeon, Akhana", "source": "bestiary-2"},
            {"id": "bestiary/monsters/example", "kind": "creature", "title": "Example", "source": "bestiary"},
            {"id": "core-rulebook/spells/fireball", "kind": "spell", "title": "Fireball", "source": "core-rulebook"},
            {"id": "core-rulebook/spells/fireball-2", "kind": "spell", "title": "Fireball", "source": "core-rulebook"}
          ]
        }"#
        .to_vec();
        (LegacySnapshot::parse(&bytes).unwrap(), bytes)
    }

    fn catalog() -> CanonicalCatalog {
        let mut catalog = CanonicalCatalog::default();
        for (id, identity, kind, title, origin) in [
            (
                "ref/monster/aeon-akhana",
                "json/monsters01.json#aeon_akhana",
                "creature",
                "Akhana (Aeon)",
                "native",
            ),
            (
                "ref/monster/example",
                "json/monsters02.json#example",
                "creature",
                "Example",
                "native",
            ),
            (
                "ref/spell/fireball",
                "json/spells.json#fireball",
                "spell",
                "Fireball",
                "native",
            ),
        ] {
            catalog
                .insert(CanonicalRecord {
                    id: id.into(),
                    identity: identity.into(),
                    kind: kind.into(),
                    title: title.into(),
                    source_ids: vec![
                        "bestiary".into(),
                        "bestiary-2".into(),
                        "core-rulebook".into(),
                    ],
                    origin: origin.into(),
                })
                .unwrap();
        }
        catalog
    }

    fn map() -> IdMap {
        let text = r#"[meta]
format = 1
legacy_count = 4
legacy_sha256 = "0000000000000000000000000000000000000000000000000000000000000000"

[retired."bestiary-2/monsters/aeon/akhana"]
reason = "no_source"
note = "alias not yet reviewed; no canonical derivation or approved carry-over"

[retired."bestiary/monsters/example"]
reason = "no_source"
note = "alias not yet reviewed; no canonical derivation or approved carry-over"

[retired."core-rulebook/spells/fireball"]
reason = "no_source"
note = "alias not yet reviewed; no canonical derivation or approved carry-over"

[retired."core-rulebook/spells/fireball-2"]
reason = "no_source"
note = "alias not yet reviewed; no canonical derivation or approved carry-over"
"#;
        IdMap::parse(text).unwrap()
    }

    #[test]
    fn inversion_and_family_parentheticals_are_proposed() {
        let (snapshot, _) = snapshot();
        let report = propose(&map(), &snapshot, &catalog()).unwrap();
        let akhana = report
            .proposals
            .iter()
            .find(|row| row.old == "bestiary-2/monsters/aeon/akhana")
            .unwrap();
        assert_eq!(akhana.target, "ref/monster/aeon-akhana");
        assert_eq!(akhana.fit, TitleFit::Inversion);
    }

    #[test]
    fn several_legacy_ids_for_one_target_become_shared() {
        let (snapshot, _) = snapshot();
        let report = propose(&map(), &snapshot, &catalog()).unwrap();
        let fireballs: Vec<_> = report
            .proposals
            .iter()
            .filter(|row| row.target == "ref/spell/fireball")
            .collect();
        assert_eq!(fireballs.len(), 2);
        assert!(fireballs.iter().all(|row| row.shared));
        assert!(
            fireballs[0]
                .books
                .iter()
                .any(|book| book == "core-rulebook")
        );
    }

    #[test]
    fn existing_map_targets_are_not_rewritten() {
        let (snapshot, _) = snapshot();
        let mut map = map();
        map.map.insert(
            "core-rulebook/spells/fireball".into(),
            MapRow {
                record: Some("json/spells.json#fireball".into()),
                ..Default::default()
            },
        );
        map.retired.remove("core-rulebook/spells/fireball");
        let report = propose(&map, &snapshot, &catalog()).unwrap();
        assert!(
            report
                .skipped
                .iter()
                .any(|row| row.old == "core-rulebook/spells/fireball-2"
                    && row.target == "ref/spell/fireball")
        );
        assert!(
            report
                .proposals
                .iter()
                .all(|row| row.old != "core-rulebook/spells/fireball-2")
        );
    }

    #[test]
    fn merging_is_idempotent_and_sorted() {
        let (snapshot, _) = snapshot();
        let original = r#"[meta]
format = 1
legacy_count = 4
legacy_sha256 = "0000000000000000000000000000000000000000000000000000000000000000"

[retired."bestiary-2/monsters/aeon/akhana"]
reason = "no_source"
note = "x"

[retired."bestiary/monsters/example"]
reason = "no_source"
note = "x"

[retired."core-rulebook/spells/fireball"]
reason = "no_source"
note = "x"

[retired."core-rulebook/spells/fireball-2"]
reason = "no_source"
note = "x"
"#;
        let map = IdMap::parse(original).unwrap();
        let report = propose(&map, &snapshot, &catalog()).unwrap();
        let first = merged_map_text(original, &report).unwrap();
        let merged = IdMap::parse(&first).unwrap();
        let second_report = propose(&merged, &snapshot, &catalog()).unwrap();
        let second = merged_map_text(&first, &second_report).unwrap();
        assert_eq!(first, second);
        assert!(first.contains("[map.\"bestiary-2/monsters/aeon/akhana\"]"));
        assert!(!first.contains("[retired.\"bestiary-2/monsters/aeon/akhana\"]"));
    }
}
