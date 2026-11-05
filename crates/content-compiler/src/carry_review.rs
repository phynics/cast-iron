//! Reviews carried PSRD entries that share a kind and normalized title with a
//! canonical Pf Data entry.
//!
//! `carry.rs` flags those pairs as possible duplicates. A title and a kind alone
//! are not evidence that two records are the same article, so this module adds
//! the missing evidence: the body text. It is a review tool, not a build gate.
//! Nothing changes unless a reviewer runs it with `--apply`.
//!
//! The metric is token-set coverage of the carried body by the candidate body:
//!
//! * `text::blocks_text` projects the typed body to plain text.
//! * Tokens are maximal runs of alphanumeric characters, lowercased, and tokens
//!   of one character are dropped. Order, punctuation, spacing, and case do not
//!   participate.
//! * Coverage is `|carried ∩ candidate| / |carried|`: the fraction of the
//!   carried entry's words that the candidate also contains. It is directional
//!   on purpose. Merging deletes the carried entry, so the candidate must cover
//!   the carried text. Symmetric Jaccard was measured first and rejected: a
//!   canonical page that combines several variants (all levels of *summon
//!   monster*, every alignment of *protection from evil*) scores 0.2-0.5 under
//!   Jaccard even when it contains the carried article word for word.
//!   Coverage also rejects the opposite case: a chapter or glossary page whose
//!   text is much larger than the candidate article scores low because most of
//!   its words have no counterpart.
//!
//! The decision rule is documented in `docs/decisions.md` under "A carried
//! duplicate is merged only on body evidence". A pair is accepted only when:
//!
//! 1. the carried entry's body and the candidate's body score at or above
//!    [`SIMILARITY_THRESHOLD`];
//! 2. the candidate's publication set contains the legacy book, using the same
//!    folding as the alias gate ([`crate::alias::canonical_source_id`]);
//! 3. the best qualifying candidate leads the runner-up by at least
//!    [`TIE_MARGIN`]. A narrower lead means body text cannot choose between two
//!    candidates, so the entry stays carried as `ambiguous`.
//!
//! Every other pair keeps the carried row and records a reason.

use crate::alias::{self, CanonicalCatalog, IdMap, LegacySnapshot, normalize_title};
use crate::alias_propose::{self, Proposal};
use crate::carry::CARRIED_PREFIX;
use crate::pipeline::Library;
use crate::text;
use cast_iron_schema::RuleEntry;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Token-set coverage threshold for accepting a body match.
///
/// Justified by the observed distribution in `docs/decisions.md`: no pair scores
/// between 0.744 and 0.828, so 0.80 sits inside an empty band. Every pair at or
/// above it survived the 40-pair spot check; pairs below it include both
/// different articles (a racial trait of another race) and chapter-level entries
/// that the candidate does not cover. Changing it needs a new distribution in the
/// decision record.
pub const SIMILARITY_THRESHOLD: f64 = 0.80;

/// Minimum gap between the best and the runner-up candidate when more than one
/// candidate passes the threshold. A narrower gap means the text does not pick a
/// winner, so the entry stays carried as `ambiguous`.
pub const TIE_MARGIN: f64 = 0.10;

/// Bodies shorter than this many tokens are not comparable. A stat-block stub or
/// an empty body would otherwise produce a misleading score in either direction.
pub const MIN_BODY_TOKENS: usize = 8;

/// One canonical entry a carried entry could duplicate.
#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    books: Vec<String>,
    tokens: BTreeSet<String>,
}

/// One scored pair, written for the spot-check table.
#[derive(Debug, Clone, Serialize)]
pub struct Pair {
    pub legacy: String,
    pub candidate: String,
    pub similarity: f64,
    pub book_match: bool,
    pub accepted: bool,
}

/// One accepted mapping.
#[derive(Debug, Clone, Serialize)]
pub struct Accepted {
    pub legacy: String,
    pub carried: String,
    pub target: String,
    /// Upstream identity for a `record` or `derived` row.
    pub identity: String,
    /// `native` or `derived:<kind>`, as the id map spells it.
    pub origin: String,
    pub similarity: f64,
    pub books: Vec<String>,
    pub shared: bool,
    /// Every row that must become `[shared]` for this target.
    pub shared_rows: Vec<SharedRow>,
}

/// One `[shared]` row the merge needs.
#[derive(Debug, Clone, Serialize)]
pub struct SharedRow {
    pub old: String,
    pub target: String,
    pub books: Vec<String>,
}

/// One carried entry that stays carried, with the reason.
#[derive(Debug, Clone, Serialize)]
pub struct Kept {
    pub legacy: String,
    pub reason: String,
    pub best_candidate: Option<String>,
    pub best_similarity: f64,
    pub best_book_match: bool,
}

/// The whole review. Deterministic: every collection is sorted.
#[derive(Debug, Clone, Default)]
pub struct CarryReview {
    pub threshold: f64,
    pub carried_total: usize,
    pub possible_duplicates: usize,
    pub pairs_examined: usize,
    pub accepted: Vec<Accepted>,
    pub kept: Vec<Kept>,
    pub kept_by_reason: BTreeMap<String, usize>,
    pub histogram: BTreeMap<String, usize>,
    pub pairs: Vec<Pair>,
}

impl CarryReview {
    pub fn report(&self) -> String {
        let reasons: Vec<String> = self
            .kept_by_reason
            .iter()
            .map(|(reason, count)| format!("{reason} {count}"))
            .collect();
        format!(
            "{} carried ({} possible duplicates), {} pairs at threshold {:.2}: merged {}, kept {} ({})",
            self.carried_total,
            self.possible_duplicates,
            self.pairs_examined,
            self.threshold,
            self.accepted.len(),
            self.kept.len(),
            reasons.join(", ")
        )
    }
}

/// Normalized token set of a body projection.
pub fn body_tokens(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|token| token.chars().count() > 1)
        .collect()
}

/// Token-set coverage: the fraction of `carried` tokens that `candidate` also
/// contains. An empty carried body scores zero, because missing text is not
/// evidence of a match.
pub fn similarity(carried: &BTreeSet<String>, candidate: &BTreeSet<String>) -> f64 {
    if carried.is_empty() {
        return 0.0;
    }
    let intersection = carried.intersection(candidate).count() as f64;
    intersection / carried.len() as f64
}

fn round(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn book_contains(books: &[String], legacy_book: &str) -> bool {
    books.iter().any(|book| book == legacy_book)
}

fn bucket(score: f64) -> String {
    let low = (score * 20.0).floor() / 20.0;
    let low = low.clamp(0.0, 0.95);
    format!("{low:.2}-{:.2}", low + 0.05)
}

/// Reviews a built canonical library. `library.entries` holds both the emitted
/// canonical entries and the carried ones this tool compares.
pub fn review(
    library: &Library,
    map: &IdMap,
    snapshot: &LegacySnapshot,
    catalog: &CanonicalCatalog,
    threshold: f64,
) -> Result<CarryReview, String> {
    review_entries(&library.entries, map, snapshot, catalog, threshold)
}

/// The entry-only form of [`review`], for fixtures and tests.
pub fn review_entries(
    entries: &[RuleEntry],
    map: &IdMap,
    snapshot: &LegacySnapshot,
    catalog: &CanonicalCatalog,
    threshold: f64,
) -> Result<CarryReview, String> {
    let legacy_by_id = snapshot.by_id();
    let carried_by_id: BTreeMap<&str, &RuleEntry> = entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();

    // Candidate index: emitted canonical entries only, grouped by the same
    // kind and normalized title that `carry.rs` uses to flag a possible
    // duplicate. Carried entries never become candidates for each other.
    let mut index: BTreeMap<(String, String), Vec<Candidate>> = BTreeMap::new();
    for entry in entries {
        if entry.id.as_str().starts_with(CARRIED_PREFIX) {
            continue;
        }
        index
            .entry((entry.kind.slug().to_string(), normalize_title(&entry.title)))
            .or_default()
            .push(Candidate {
                id: entry.id.as_str().to_string(),
                // The alias gate proves the book against the catalog record's
                // publication set, so the review must use the same set.
                books: catalog
                    .get(entry.id.as_str())
                    .map(|record| record.source_ids.clone())
                    .unwrap_or_default(),
                tokens: body_tokens(&text::blocks_text(&entry.body)),
            });
    }
    for candidates in index.values_mut() {
        candidates.sort_by(|a, b| a.id.cmp(&b.id));
    }

    // The reviewable carried set is the explicit `ref/carried/...` rows. A
    // fallback carry-over row names a planned canonical target that the
    // converter did not emit; rewriting that row from body evidence would
    // override the reviewed plan, so it stays carried with its own reason.
    let explicit: BTreeSet<&str> = map
        .map
        .iter()
        .filter(|(_, row)| {
            row.target
                .as_deref()
                .is_some_and(|target| target.starts_with(CARRIED_PREFIX))
        })
        .map(|(old, _)| old.as_str())
        .collect();
    let carried_ids: BTreeSet<&str> = entries
        .iter()
        .filter_map(|entry| entry.id.as_str().strip_prefix(CARRIED_PREFIX))
        .collect();

    let claims = alias_propose::existing_claims(map, catalog);
    let mut review = CarryReview {
        threshold,
        carried_total: carried_ids.len(),
        ..Default::default()
    };

    let mut accepted: Vec<Accepted> = Vec::new();
    let mut kept: Vec<Kept> = Vec::new();
    let mut pairs: Vec<Pair> = Vec::new();

    for legacy in &carried_ids {
        let carried = format!("{CARRIED_PREFIX}{legacy}");
        let Some(entry) = carried_by_id.get(carried.as_str()) else {
            continue;
        };
        let Some(legacy_entry) = legacy_by_id.get(legacy) else {
            return Err(format!(
                "carried id {carried:?} is not in the legacy snapshot"
            ));
        };
        let legacy_book = alias::canonical_source_id(&legacy_entry.source);
        let legacy_tokens = body_tokens(&text::blocks_text(&entry.body));
        let key = (
            legacy_entry.kind.clone(),
            normalize_title(&legacy_entry.title),
        );
        let candidates = index.get(&key).cloned().unwrap_or_default();
        if !candidates.is_empty() {
            review.possible_duplicates += 1;
        }

        let mut scored: Vec<(f64, bool, &Candidate)> = Vec::new();
        for candidate in &candidates {
            let score = similarity(&legacy_tokens, &candidate.tokens);
            let book_match = book_contains(&candidate.books, &legacy_book);
            scored.push((score, book_match, candidate));
            pairs.push(Pair {
                legacy: legacy.to_string(),
                candidate: candidate.id.clone(),
                similarity: round(score),
                book_match,
                // Filled in after the decision, so a pair row never disagrees
                // with the accepted list.
                accepted: false,
            });
        }
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.id.cmp(&b.2.id))
        });
        review.pairs_examined += scored.len();

        let best = scored.first().map(|(score, book, candidate)| Kept {
            legacy: legacy.to_string(),
            reason: String::new(),
            best_candidate: Some(candidate.id.clone()),
            best_similarity: round(*score),
            best_book_match: *book,
        });
        if let Some(mut kept_row) = best.clone() {
            if kept_row.best_book_match {
                *review
                    .histogram
                    .entry(bucket(kept_row.best_similarity))
                    .or_default() += 1;
            }
            if !explicit.contains(legacy) {
                kept_row.reason = "fallback_target".to_string();
                kept.push(kept_row);
                continue;
            }
            if legacy_tokens.len() < MIN_BODY_TOKENS {
                kept_row.reason = "short_body".to_string();
                kept.push(kept_row);
                continue;
            }
            let qualifying: Vec<&(f64, bool, &Candidate)> = scored
                .iter()
                .filter(|(score, book, _)| *score >= threshold && *book)
                .collect();
            let unique_best = qualifying.len() == 1
                || (qualifying.len() > 1 && qualifying[0].0 - qualifying[1].0 >= TIE_MARGIN);
            if unique_best && !qualifying.is_empty() {
                let (score, _, candidate) = *qualifying[0];
                let Some(record) = catalog.get(&candidate.id) else {
                    kept_row.reason = "candidate_unavailable".to_string();
                    kept.push(kept_row);
                    continue;
                };
                let (origin, identity) = if record.origin == "native" {
                    ("native".to_string(), record.identity.clone())
                } else if record.origin.starts_with("derived:") {
                    (record.origin.clone(), record.identity.clone())
                } else {
                    ("target".to_string(), record.id.clone())
                };
                accepted.push(Accepted {
                    legacy: legacy.to_string(),
                    carried: carried.clone(),
                    target: candidate.id.clone(),
                    identity,
                    origin,
                    similarity: round(score),
                    books: vec![legacy_book.clone()],
                    shared: false,
                    shared_rows: Vec::new(),
                });
                continue;
            }
            kept_row.reason = if qualifying.len() > 1 {
                "ambiguous".to_string()
            } else if scored
                .iter()
                .any(|(score, book, _)| *score >= threshold && !*book)
            {
                "book_mismatch".to_string()
            } else {
                "below_threshold".to_string()
            };
            kept.push(kept_row);
        } else {
            kept.push(Kept {
                legacy: legacy.to_string(),
                reason: if explicit.contains(legacy) {
                    "no_candidate".to_string()
                } else {
                    "fallback_target".to_string()
                },
                best_candidate: None,
                best_similarity: 0.0,
                best_book_match: false,
            });
        }
    }

    // A canonical record claimed by more than one legacy id needs every row to
    // be `[shared]`. Existing claims are part of the same decision, so the map
    // is rewritten once and the alias gate cannot see a half-merged target.
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, row) in accepted.iter().enumerate() {
        groups.entry(row.target.clone()).or_default().push(index);
    }
    let sources: BTreeMap<&str, &str> = snapshot
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.source.as_str()))
        .collect();
    for (target, indices) in groups {
        let mut shared_rows: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for &index in &indices {
            let row = &accepted[index];
            shared_rows
                .entry(row.legacy.clone())
                .or_default()
                .extend(row.books.iter().cloned());
        }
        for old in claims.get(&target).into_iter().flatten() {
            let books = sources
                .get(old.as_str())
                .map(|source| vec![alias::canonical_source_id(source)])
                .unwrap_or_default();
            shared_rows.entry(old.clone()).or_default().extend(books);
        }
        let shared = shared_rows.len() > 1;
        for &index in &indices {
            let row = &mut accepted[index];
            row.shared = shared;
            if shared {
                row.shared_rows = shared_rows
                    .iter()
                    .map(|(old, books)| SharedRow {
                        old: old.clone(),
                        target: target.clone(),
                        books: books.clone(),
                    })
                    .collect();
            }
        }
    }

    let accepted_legacy: BTreeSet<&str> = accepted.iter().map(|row| row.legacy.as_str()).collect();
    for pair in &mut pairs {
        pair.accepted = accepted_legacy.contains(pair.legacy.as_str())
            && accepted
                .iter()
                .any(|row| row.legacy == pair.legacy && row.target == pair.candidate);
    }
    pairs.sort_by(|a, b| (&a.legacy, &a.candidate).cmp(&(&b.legacy, &b.candidate)));
    accepted.sort_by(|a, b| a.legacy.cmp(&b.legacy));
    kept.sort_by(|a, b| a.legacy.cmp(&b.legacy));
    for row in &kept {
        *review.kept_by_reason.entry(row.reason.clone()).or_default() += 1;
    }
    review.accepted = accepted;
    review.kept = kept;
    review.pairs = pairs;
    Ok(review)
}

/// The id map with every accepted merge applied. Blocks are replaced in place, so
/// the file keeps its original append order, comments, and separators; only the
/// accepted rows and the rows that share their target change.
pub fn merged_map_text(original: &str, review: &CarryReview) -> Result<String, String> {
    let lines: Vec<&str> = original.lines().collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| header_key(line).is_some())
        .map(|(index, _)| index)
        .collect();
    if starts.is_empty() {
        return Err("id map has no tables".into());
    }
    let mut blocks: Vec<(String, String)> = Vec::new();
    for (position, start) in starts.iter().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(lines.len());
        let mut body: Vec<&str> = lines[*start..end].to_vec();
        while body.len() > 1 && body.last().is_some_and(|line| line.trim().is_empty()) {
            body.pop();
        }
        blocks.push((
            header_key(lines[*start]).expect("filtered").to_string(),
            body.join("\n"),
        ));
    }
    let index: BTreeMap<String, usize> = blocks
        .iter()
        .enumerate()
        .map(|(position, (key, _))| (key.clone(), position))
        .collect();

    for row in &review.accepted {
        if row.shared {
            continue;
        }
        let position = *index
            .get(&row.legacy)
            .ok_or_else(|| format!("accepted row {:?} has no block in the id map", row.legacy))?;
        blocks[position].1 = alias_propose::map_block(&map_proposal(row))
            .trim_end()
            .to_string();
    }
    for row in &review.accepted {
        if !row.shared {
            continue;
        }
        for shared in &row.shared_rows {
            let position = *index
                .get(&shared.old)
                .ok_or_else(|| format!("shared row {:?} has no block in the id map", shared.old))?;
            blocks[position].1 = alias_propose::shared_block(&shared_proposal(shared))
                .trim_end()
                .to_string();
        }
    }

    let mut out = lines[..starts[0]].join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    for (position, (_, text)) in blocks.iter().enumerate() {
        if position > 0 {
            out.push('\n');
        }
        out.push_str(text);
        out.push('\n');
    }
    Ok(out)
}

fn header_key(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("[map.\"")
        .or_else(|| line.strip_prefix("[shared.\""))
        .or_else(|| line.strip_prefix("[retired.\""))?;
    rest.strip_suffix("\"]")
}

fn map_proposal(row: &Accepted) -> Proposal {
    Proposal {
        old: row.legacy.clone(),
        target: row.target.clone(),
        identity: row.identity.clone(),
        origin: row.origin.clone(),
        fit: alias::TitleFit::Exact,
        evidence: "body-similarity",
        books: row.books.clone(),
        shared: false,
    }
}

fn shared_proposal(row: &SharedRow) -> Proposal {
    Proposal {
        old: row.old.clone(),
        target: row.target.clone(),
        identity: String::new(),
        origin: "target".to_string(),
        fit: alias::TitleFit::Exact,
        evidence: "body-similarity",
        books: row.books.clone(),
        shared: true,
    }
}

/// Writes the JSON report and the proposed map. `--apply` rewrites the committed
/// map only. Both files are deterministic.
pub fn write(
    review: &CarryReview,
    original: &str,
    map_path: &std::path::Path,
    output_dir: &std::path::Path,
    apply: bool,
) -> Result<(), String> {
    let merged = merged_map_text(original, review)?;
    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    std::fs::write(output_dir.join("id-map.reviewed.toml"), &merged).map_err(|e| e.to_string())?;
    let json = serde_json::json!({
        "format": 1,
        "note": "Pre-apply review of the carried possible-duplicate set. Rerunning after --apply reports only the remaining leads.",
        "map_sha256": crate::select::sha256_hex(original.as_bytes()),
        "threshold": review.threshold,
        "carried_total": review.carried_total,
        "possible_duplicates": review.possible_duplicates,
        "pairs_examined": review.pairs_examined,
        "merged": review.accepted.len(),
        "kept": review.kept.len(),
        "kept_by_reason": review.kept_by_reason,
        "histogram": review.histogram,
        "accepted": review.accepted,
        "kept_rows": review
            .kept
            .iter()
            .filter(|row| row.best_candidate.is_some())
            .collect::<Vec<_>>(),
        "pairs": review.pairs,
    });
    let mut bytes = serde_json::to_vec_pretty(&json).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    std::fs::write(output_dir.join("carry-review.json"), bytes).map_err(|e| e.to_string())?;
    if apply {
        std::fs::write(map_path, merged).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alias::{CanonicalRecord, IdMapMeta, MapRow};
    use cast_iron_schema::{
        Block, EntryId, EntryKind, Inline, ParagraphStyle, Provenance, TypedFields,
    };

    fn body(text: &str) -> Vec<Block> {
        vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![Inline::Text {
                v: text.to_string(),
            }],
        }]
    }

    fn entry(id: &str, kind: EntryKind, title: &str, source: &str, text: &str) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: title.into(),
            source_ids: vec![source.into()],
            section: vec!["Section".into()],
            parent: None,
            body: body(text),
            fields: TypedFields::None,
            tags: Vec::new(),
            links: Vec::new(),
            provenance: vec![Provenance {
                dataset: "psrd-data".into(),
                record: format!("pfsrd://Source/{title}"),
                path: format!("{source}/{id}.json"),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: Vec::new(),
        }
    }

    fn long_text(extra: &str) -> String {
        format!(
            "This is a long enough body of rules text used to compare two records. It names the same procedure and the same outcome. {extra}"
        )
    }

    fn snapshot() -> LegacySnapshot {
        let bytes = br#"{
          "format": 1,
          "count": 2,
          "entries": [
            {"id": "bestiary/monsters/example", "kind": "creature", "title": "Example", "source": "bestiary"},
            {"id": "bestiary/monsters/other", "kind": "creature", "title": "Other", "source": "bestiary"}
          ]
        }"#;
        LegacySnapshot::parse(bytes).unwrap()
    }

    fn map(rows: &[(&str, &str)]) -> IdMap {
        IdMap {
            meta: IdMapMeta {
                format: 1,
                legacy_count: 2,
                legacy_sha256: "0".repeat(64),
                note: None,
            },
            map: rows
                .iter()
                .map(|(old, target)| {
                    (
                        (*old).to_string(),
                        MapRow {
                            target: Some((*target).to_string()),
                            ..MapRow::default()
                        },
                    )
                })
                .collect(),
            shared: BTreeMap::new(),
            retired: BTreeMap::new(),
        }
    }

    fn catalog(records: &[(&str, &str, &str, &str, &str)]) -> CanonicalCatalog {
        let mut catalog = CanonicalCatalog::default();
        for (id, identity, kind, title, source) in records {
            catalog
                .insert(CanonicalRecord {
                    id: (*id).into(),
                    identity: (*identity).into(),
                    kind: (*kind).into(),
                    title: (*title).into(),
                    source_ids: vec![(*source).into()],
                    origin: "native".into(),
                })
                .unwrap();
        }
        catalog
    }

    #[test]
    fn token_similarity_ignores_case_punctuation_and_order() {
        let left = body_tokens("Stonecunning: dwarves notice stonework. Dwarves!");
        let right = body_tokens("DWARVES notice stonework stONecunning");
        assert_eq!(similarity(&left, &right), 1.0);
        assert_eq!(
            similarity(&body_tokens("alpha beta"), &body_tokens("gamma delta")),
            0.0
        );
        assert_eq!(
            similarity(&BTreeSet::new(), &body_tokens("alpha beta")),
            0.0
        );
        // Coverage is directional: a canonical page that combines variants
        // covers the carried text, while the carried text does not cover the
        // whole combined page.
        let carried = body_tokens("summon monster one celestial eagle");
        let combined = body_tokens(
            "summon monster one celestial eagle summon monster two fiendish wolf summon monster three dire bat",
        );
        assert_eq!(similarity(&carried, &combined), 1.0);
        assert!(similarity(&combined, &carried) < 0.5);
    }

    #[test]
    fn a_similar_body_is_accepted_and_rewrites_the_row() {
        let entries = vec![
            entry(
                "ref/carried/bestiary/monsters/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &long_text(""),
            ),
            entry(
                "ref/monster/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &long_text("A trailing sentence."),
            ),
        ];
        let map = map(&[(
            "bestiary/monsters/example",
            "ref/carried/bestiary/monsters/example",
        )]);
        let catalog = catalog(&[(
            "ref/monster/example",
            "json/monsters01.json#example",
            "creature",
            "Example",
            "bestiary",
        )]);
        let review = review_entries(&entries, &map, &snapshot(), &catalog, 0.6).unwrap();
        assert_eq!(review.accepted.len(), 1);
        assert_eq!(review.accepted[0].target, "ref/monster/example");
        let text = merged_map_text(
            "[meta]\nformat = 1\nlegacy_count = 2\nlegacy_sha256 = \"x\"\n\n[map.\"bestiary/monsters/example\"]\ntarget = \"ref/carried/bestiary/monsters/example\"\n\n[retired.\"bestiary/monsters/other\"]\nreason = \"no_source\"\n",
            &review,
        )
        .unwrap();
        assert!(
            text.contains("record = \"json/monsters01.json#example\""),
            "{text}"
        );
        assert!(
            !text.contains("ref/carried/bestiary/monsters/example"),
            "{text}"
        );
    }

    #[test]
    fn applying_a_merge_preserves_block_order() {
        let text = long_text("");
        let entries = vec![
            entry(
                "ref/carried/bestiary/monsters/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &text,
            ),
            entry(
                "ref/monster/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &text,
            ),
        ];
        let map = map(&[
            (
                "bestiary/monsters/other",
                "ref/carried/bestiary/monsters/other",
            ),
            (
                "bestiary/monsters/example",
                "ref/carried/bestiary/monsters/example",
            ),
        ]);
        let catalog = catalog(&[(
            "ref/monster/example",
            "json/monsters01.json#example",
            "creature",
            "Example",
            "bestiary",
        )]);
        let review = review_entries(&entries, &map, &snapshot(), &catalog, 0.6).unwrap();
        let original = "[meta]\nformat = 1\nlegacy_count = 2\nlegacy_sha256 = \"x\"\n\n[map.\"bestiary/monsters/other\"]\ntarget = \"ref/carried/bestiary/monsters/other\"\n\n[shared.\"bestiary/monsters/shared\"]\ntarget = \"ref/monster/shared\"\nnote = \"fixture\"\n\n[map.\"bestiary/monsters/example\"]\ntarget = \"ref/carried/bestiary/monsters/example\"\n";
        let merged = merged_map_text(original, &review).unwrap();
        let other = merged.find("[map.\"bestiary/monsters/other\"]").unwrap();
        let shared = merged
            .find("[shared.\"bestiary/monsters/shared\"]")
            .unwrap();
        let example = merged.find("[map.\"bestiary/monsters/example\"]").unwrap();
        assert!(other < shared && shared < example, "{merged}");
        assert!(merged.contains("record = \"json/monsters01.json#example\""));
    }

    #[test]
    fn a_book_mismatch_stays_carried() {
        let entries = vec![
            entry(
                "ref/carried/bestiary/monsters/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &long_text(""),
            ),
            entry(
                "ref/monster/example",
                EntryKind::Creature,
                "Example",
                "bestiary-2",
                &long_text(""),
            ),
        ];
        let map = map(&[(
            "bestiary/monsters/example",
            "ref/carried/bestiary/monsters/example",
        )]);
        let catalog = catalog(&[(
            "ref/monster/example",
            "json/monsters02.json#example",
            "creature",
            "Example",
            "bestiary-2",
        )]);
        let review = review_entries(&entries, &map, &snapshot(), &catalog, 0.6).unwrap();
        assert!(review.accepted.is_empty());
        assert_eq!(review.kept_by_reason["book_mismatch"], 1);
    }

    #[test]
    fn two_ids_for_one_target_become_shared() {
        let text = long_text("");
        let entries = vec![
            entry(
                "ref/carried/bestiary/monsters/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &text,
            ),
            entry(
                "ref/carried/bestiary/monsters/other",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &text,
            ),
            entry(
                "ref/monster/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &text,
            ),
        ];
        let map = map(&[
            (
                "bestiary/monsters/example",
                "ref/carried/bestiary/monsters/example",
            ),
            (
                "bestiary/monsters/other",
                "ref/carried/bestiary/monsters/other",
            ),
        ]);
        let mut snapshot = snapshot();
        snapshot.entries[1].title = "Example".into();
        let catalog = catalog(&[(
            "ref/monster/example",
            "json/monsters01.json#example",
            "creature",
            "Example",
            "bestiary",
        )]);
        let review = review_entries(&entries, &map, &snapshot, &catalog, 0.6).unwrap();
        assert_eq!(review.accepted.len(), 2);
        assert!(review.accepted.iter().all(|row| row.shared));
        assert!(review.accepted.iter().all(|row| row.shared_rows.len() == 2));
    }

    #[test]
    fn a_short_body_is_not_evidence() {
        let entries = vec![
            entry(
                "ref/carried/bestiary/monsters/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                "Short.",
            ),
            entry(
                "ref/monster/example",
                EntryKind::Creature,
                "Example",
                "bestiary",
                &long_text(""),
            ),
        ];
        let map = map(&[(
            "bestiary/monsters/example",
            "ref/carried/bestiary/monsters/example",
        )]);
        let catalog = catalog(&[(
            "ref/monster/example",
            "json/monsters01.json#example",
            "creature",
            "Example",
            "bestiary",
        )]);
        let review = review_entries(&entries, &map, &snapshot(), &catalog, 0.6).unwrap();
        assert!(review.accepted.is_empty());
        assert_eq!(review.kept_by_reason["short_body"], 1);
    }
}
