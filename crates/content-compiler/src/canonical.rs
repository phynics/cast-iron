//! Canonical conversion of the screened Pf Data corpus.
//!
//! This is the production converter behind the migration experiments. It reads
//! the verified [`Inventory`], converts every in-scope record to a schema-3
//! [`RuleEntry`], derives the racial traits, afflictions, and allowlisted rules,
//! and accounts for everything it does not emit.
//!
//! Every identifier is the `ref/` id from [`crate::alias::canonical_native_id`],
//! the same derivation the legacy alias catalog uses. The module has no reporting
//! side effects: [`convert_corpus`] returns entries, parent-to-child links,
//! findings, coverage rows, and excluded records, and the `data-*` commands render
//! those results. Nothing here is wired into the production PSRD build.

use crate::pf_data::{InputRecord, Inventory, description, expanded_input, pf_license_refs};
use crate::{
    pf_derived, pf_directives, pf_fields, pf_markdown, select::sha256_hex, slug::slug_segment,
    text::blocks_text,
};
use cast_iron_schema::{EntryId, EntryKind, Provenance, RuleEntry};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// A converter defect, a dangling reference, or a skipped derivation. These are
/// not coverage: a coverage row explains a deliberate omission, a finding names
/// something the converter should have handled.
#[derive(Debug, Serialize)]
pub struct Finding {
    pub severity: String,
    pub record: String,
    pub problem: String,
}

/// A construct the converter deliberately omits because it carries no typed
/// content, or a reference it cannot resolve inside a bounded selection. Each row
/// names the category, the exact detail, the reason, and the affected record, so
/// the omission is evidence rather than a silent drop.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CoverageException {
    pub category: String,
    pub detail: String,
    pub reason: String,
    pub record: String,
    pub occurrences: usize,
}

/// A record the converter did not emit, with the reason. A failed record is
/// excluded and reported (AGENTS.md rule 6); it never fails the whole build.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExcludedRecord {
    /// Upstream identity: `path#key` for a native record, or the derivative
    /// identity for a derived child.
    pub identity: String,
    /// The canonical id when the derivation produced one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `native`, `derived`, or `rule`.
    pub category: String,
    pub reason: String,
}

/// Deterministic accumulator keyed by category, detail, and record.
#[derive(Default)]
pub(crate) struct CoverageLog {
    rows: BTreeMap<(String, String, String), (String, usize)>,
}

impl CoverageLog {
    pub(crate) fn record(&mut self, category: &str, detail: &str, reason: &str, record: &str) {
        let key = (category.to_string(), detail.to_string(), record.to_string());
        let entry = self
            .rows
            .entry(key)
            .or_insert_with(|| (reason.to_string(), 0));
        entry.1 += 1;
    }

    pub(crate) fn into_rows(self) -> Vec<CoverageException> {
        self.rows
            .into_iter()
            .map(
                |((category, detail, record), (reason, occurrences))| CoverageException {
                    category,
                    detail,
                    reason,
                    record,
                    occurrences,
                },
            )
            .collect()
    }
}

pub(crate) const OUTSIDE_SAMPLE_REASON: &str = "The target exists in the pinned Pf Data inventory but is outside this bounded selection. The visible link text is kept; a full import resolves the link.";

pub(crate) const FAILED_TARGET_REASON: &str = "The target is selected in this run but its own conversion failed; the target's conversion finding is the root cause. The visible link text is kept rather than reported again as a dangling reference.";

pub(crate) const EXTERNAL_LINK_REASON: &str = "The target is an external web page, not an entry in the pinned inventory. The visible link text is kept; schema 3 has no external-link node.";

pub(crate) const NON_ENTRY_LINK_REASON: &str = "The target is an app route or a source-local shorthand, not an entry in the pinned inventory. The visible link text is kept.";

/// Classify an HTML construct that the converter strips as presentation-only.
/// `None` means the key is an actual missing renderer and must stay a finding.
pub(crate) fn presentation_exception(key: &str) -> Option<(&'static str, String, &'static str)> {
    let entry = match key {
        "heading_jump_list" => (
            "jump-list-metadata",
            "The `jl` attribute only groups the heading in the upstream on-page jump list; the heading text itself is rendered.",
        ),
        "linked_heading_jump_list" => (
            "jump-list-metadata",
            "The `jl` attribute only groups the linked heading in the upstream on-page jump list; the heading text and link are rendered.",
        ),
        "ab_jump_list_metadata" => (
            "jump-list-metadata",
            "The `jl` attribute groups an ability in the upstream on-page jump list; `strip` and empty values add no prose or rules text.",
        ),
        "ab_order_metadata" => (
            "ability-order-metadata",
            "The `order` attribute only sorts abilities in the upstream jump list. Entries keep their source order and the flag adds no prose.",
        ),
        "ab_structural_metadata" => (
            "ability-structure-metadata",
            "The `head`, `next`, `sub`, and `indent` flags describe the upstream ability layout. Rendered content already follows the source order.",
        ),
        "aside_container" => (
            "layout-container",
            "An `::aside` is a layout wrapper. Its contents are converted as ordinary blocks and are not dropped.",
        ),
        "block_container" => (
            "layout-container",
            "An upstream `:::block` applies a size or style to its following content. The wrapper is not body content; its text, and any title, are rendered.",
        ),
        "group_container" => (
            "layout-container",
            "An upstream `:::group` only groups following paragraphs for presentation. Its contents are converted as ordinary blocks and are not dropped.",
        ),
        "fake_footnotes_container" => (
            "layout-container",
            "`::fakeFootnotes` is a layout wrapper. Its `@FN` definitions are rendered by the inline pass, so no footnote text is lost.",
        ),
        "hr" => (
            "layout-separator",
            "A horizontal rule is a visual separator. Schema 3 has no divider block, and the surrounding paragraphs are preserved.",
        ),
        "unwrapped_tag:del" | "unwrapped_tag:s" | "unwrapped_tag:strike" => (
            "presentation-strike",
            "Strikethrough marks superseded wording. Schema 3 has no strike node, so the text is kept as ordinary content.",
        ),
        _ if key.starts_with("icon_directive:") => (
            "presentation-icon",
            "An `@I-` token selects an upstream UI icon. It carries no text or rules meaning; the surrounding prose is preserved.",
        ),
        _ if key.starts_with("class_options_external_list:") => (
            "external-class-options",
            "Favored class options for this class live in a separate generated upstream component. The body keeps an explicit pointer to that page.",
        ),
        _ => return None,
    };
    Some((entry.0, key.to_string(), entry.1))
}

/// The result of one conversion run. `entries` is sorted by id; `links` holds
/// the parent-to-child edges in the shape [`crate::build::NestedLink`] uses.
#[derive(Debug, Default)]
pub struct Converted {
    pub entries: Vec<RuleEntry>,
    pub links: Vec<crate::build::NestedLink>,
    /// Page attribution from each entry's screened `‹SOURCE book/page›` lines.
    /// One row per publication, in first-appearance order. `None` means the
    /// source line named the publication but gave no page number. An entry with
    /// no source line is absent.
    pub pages: BTreeMap<EntryId, Vec<(String, Option<u32>)>>,
    pub findings: Vec<Finding>,
    pub coverage: Vec<CoverageException>,
    pub excluded: Vec<ExcludedRecord>,
    pub selected_records: usize,
    /// Every allowlisted rule derivable from the whole in-scope corpus, even in a
    /// bounded sample run.
    pub derived_available: usize,
    /// Reviewed rules the allowlist expects for the selected hosts.
    pub derived_expected: usize,
    pub derived_rejected: usize,
    pub derived_records: usize,
    pub derived_by_kind: BTreeMap<String, usize>,
    pub derived_rules: usize,
    pub derived_sections: usize,
    pub derived_rows: usize,
    pub derived_skipped: usize,
    /// Ids of the derived entries and promoted rules, so a reporting layer can
    /// check them explicitly without relying on their id prefix.
    pub derived_entry_ids: BTreeSet<String>,
}

/// Convert the whole in-scope pinned corpus. This is the production API; no
/// output is placed in the production PSRD library yet.
pub fn convert_corpus(
    inv: &Inventory,
    allowlist: &crate::pf_rules::RuleAllowlist,
) -> Result<Converted, String> {
    convert(inv, allowlist, false)
}

/// Convert the bounded representative selection used by `data-report`. It runs
/// the same core as [`convert_corpus`]; only the selection differs.
pub fn convert_representative(
    inv: &Inventory,
    allowlist: &crate::pf_rules::RuleAllowlist,
) -> Result<Converted, String> {
    convert(inv, allowlist, true)
}

fn convert(
    inv: &Inventory,
    allowlist: &crate::pf_rules::RuleAllowlist,
    representative: bool,
) -> Result<Converted, String> {
    let out_of_scope: BTreeSet<(&str, &str)> = inv
        .out_of_scope
        .iter()
        .map(|record| (record.path.as_str(), record.key.as_str()))
        .collect();
    let in_scope =
        |record: &InputRecord| !out_of_scope.contains(&(record.path.as_str(), record.key.as_str()));
    let selected = if representative {
        representative_selection(inv, &in_scope)
    } else {
        full_selection(inv, &in_scope)
    };
    let mut result = Converted {
        selected_records: selected.len(),
        ..Default::default()
    };

    // Count the derivable rules across the whole in-scope corpus, even when this
    // run only converts a representative sample.
    for record in inv.records.iter().filter(|record| in_scope(record)) {
        let rules = allowlist.for_host(&record.path, &record.key);
        if rules.is_empty() {
            continue;
        }
        if let Ok(lines) = description(&record.raw) {
            result.derived_available += crate::pf_rules::extract(&lines, rules).len();
        }
    }

    let mut coverage = CoverageLog::default();
    // The PI screen ran over the whole inventory; report the omissions taken from
    // the records this run selected. The tracked coverage summary counts all of them.
    for omission in &inv.pi_omissions {
        let in_run = omission
            .record
            .split_once('#')
            .is_some_and(|(path, key)| selected.contains(&(path.to_string(), key.to_string())));
        if in_run {
            coverage.record(
                &omission.category,
                &omission.detail,
                &omission.reason,
                &omission.record,
            );
        }
    }

    let mut context = LinkContext {
        known: inv.records.iter().filter_map(native_key).collect(),
        selected: inv
            .records
            .iter()
            .filter(|record| selected.contains(&(record.path.clone(), record.key.clone())))
            .filter_map(native_key)
            .collect(),
        ..Default::default()
    };

    let mut expanded_inputs = Vec::new();
    for record in inv
        .records
        .iter()
        .filter(|record| selected.contains(&(record.path.clone(), record.key.clone())))
    {
        match crate::pf_records::expand(record, &inv.records) {
            Ok(raw) => {
                let mut expanded = expanded_input(record, raw);
                if expanded.publications.is_empty() {
                    let derived = derive_publications(&expanded.raw, &inv.records);
                    if !derived.is_empty() {
                        expanded.publications = derived;
                        coverage.record(
                            "publication-derived",
                            &format!("{}#{}", record.path, record.key),
                            "The upstream record carries no publication of its own; its subtopic records supply the attribution.",
                            &format!("{}#{}", record.path, record.key),
                        );
                    }
                }
                expanded_inputs.push(expanded);
            }
            Err(problem) => {
                let identity = format!("{}#{}", record.path, record.key);
                result.findings.push(Finding {
                    severity: "high".into(),
                    record: identity.clone(),
                    problem: problem.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity,
                    id: pf_derived::native_id(record).map(|id| id.to_string()),
                    category: "native".into(),
                    reason: problem,
                });
            }
        }
    }

    // Derived children are planned before conversion so that parent bodies can link
    // to them and their IDs count as known targets.
    let mut derived_rejections = Vec::new();
    let derivations = pf_derived::plan(&expanded_inputs, &mut derived_rejections);
    for rejection in &derived_rejections {
        // An unnamed `::aff` block has a recognised shape but no derivable
        // identity; that is a coverage gap, not a broken record.
        result.findings.push(Finding {
            severity: if rejection.problem.contains("has no name") {
                "medium".into()
            } else {
                "high".into()
            },
            record: rejection.identity.clone(),
            problem: rejection.problem.clone(),
        });
        result.excluded.push(ExcludedRecord {
            identity: rejection.identity.clone(),
            id: None,
            category: "derived".into(),
            reason: rejection.problem.clone(),
        });
    }
    result.derived_rejected = derived_rejections.len();
    for derivation in &derivations {
        context
            .derived
            .entry(derivation.parent_identity.clone())
            .or_default()
            .insert(derivation.name.clone(), derivation.id.clone());
        context
            .targets
            .insert(derivation.id.as_str().to_string(), derivation.id.clone());
    }

    let mut converted_inputs = Vec::new();
    for record in expanded_inputs {
        match convert_record(
            &record,
            &context,
            &mut Vec::new(),
            &mut CoverageLog::default(),
        ) {
            Ok(entry) => {
                if let Some(key) = native_key(&record) {
                    context.targets.insert(key, entry.id);
                }
                converted_inputs.push(record);
            }
            Err(problem) => {
                let identity = format!("{}#{}", record.path, record.key);
                result.findings.push(Finding {
                    severity: "high".into(),
                    record: identity.clone(),
                    problem: problem.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity,
                    id: pf_derived::native_id(&record).map(|id| id.to_string()),
                    category: "native".into(),
                    reason: problem,
                });
            }
        }
    }

    // Page attribution is read from the screened lines before rendering, so a
    // record's SOURCE markers survive even when the body rewrites them.
    let identity_pages: BTreeMap<String, Vec<(String, Option<u32>)>> = converted_inputs
        .iter()
        .filter_map(|record| {
            let lines = description(&record.raw).ok()?;
            let pages = source_attribution(&lines);
            (!pages.is_empty()).then(|| (format!("{}#{}", record.path, record.key), pages))
        })
        .collect();

    // Redirects are aliases of a canonical record, never duplicated rules pages.
    for alias in inv
        .records
        .iter()
        .filter(|record| record.shape == "redirect")
    {
        if let (Some(protocol), Some(target)) = (
            &alias.protocol,
            alias.raw.get("redirect").and_then(Value::as_str),
        ) {
            let key = format!("{protocol}/{target}");
            if let Some(id) = context.targets.get(&key).cloned() {
                context
                    .targets
                    .insert(format!("{protocol}/{}", alias.key), id);
            }
        }
    }
    // Links to a selected record that did not convert are a cascade of that
    // record's own finding. Record the failed identities once so the second pass
    // can explain each link instead of reporting a second dangling reference.
    context.failed = inv
        .records
        .iter()
        .filter(|record| selected.contains(&(record.path.clone(), record.key.clone())))
        .filter_map(native_key)
        .filter(|key| !context.targets.contains_key(key))
        .collect();

    let mut entry_order: Vec<RuleEntry> = Vec::new();
    let mut index_by_identity: BTreeMap<String, usize> = BTreeMap::new();
    for record in &converted_inputs {
        let identity = format!("{}#{}", record.path, record.key);
        match convert_record(record, &context, &mut result.findings, &mut coverage) {
            Ok(entry) => {
                if let Some(pages) = identity_pages.get(&identity) {
                    result.pages.insert(entry.id.clone(), pages.clone());
                }
                index_by_identity.insert(identity, entry_order.len());
                entry_order.push(entry);
            }
            Err(problem) => {
                // The first pass succeeded; a second-pass failure is still a
                // per-record exclusion, not a failed build.
                result.findings.push(Finding {
                    severity: "high".into(),
                    record: identity.clone(),
                    problem: problem.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity,
                    id: pf_derived::native_id(record).map(|id| id.to_string()),
                    category: "native".into(),
                    reason: problem,
                });
            }
        }
    }

    let mut derived_entries = Vec::new();
    let mut links = Vec::new();
    for derivation in &derivations {
        let parent = index_by_identity
            .get(&derivation.parent_identity)
            .map(|&index| &entry_order[index]);
        if parent.is_none() {
            result.findings.push(Finding {
                severity: "medium".into(),
                record: derivation.parent_identity.clone(),
                problem: format!(
                    "derived record {} was built without its host entry; host conversion is still blocked",
                    derivation.id
                ),
            });
        }
        match derive_entry(
            derivation,
            parent,
            &context,
            &mut result.findings,
            &mut coverage,
        ) {
            Ok(entry) => {
                let pages = source_attribution(&derivation.body_lines);
                let pages = if pages.is_empty() {
                    identity_pages
                        .get(&derivation.parent_identity)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    pages
                };
                if !pages.is_empty() {
                    result.pages.insert(entry.id.clone(), pages);
                }
                if let Some(parent) = parent {
                    links.push(crate::build::NestedLink {
                        host: parent.id.clone(),
                        target: entry.id.clone(),
                        trail: Vec::new(),
                        child: false,
                    });
                }
                derived_entries.push(entry);
            }
            Err(problem) => {
                result.findings.push(Finding {
                    severity: "high".into(),
                    record: derivation.id.to_string(),
                    problem: problem.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity: derivation.provenance_record.clone(),
                    id: Some(derivation.id.to_string()),
                    category: "derived".into(),
                    reason: problem,
                });
            }
        }
    }
    for entry in &derived_entries {
        *result
            .derived_by_kind
            .entry(entry.kind.slug().into())
            .or_default() += 1;
    }
    result.derived_records = derived_entries.len();

    // PSRD promotes some rule sections and table rows that Pf Data keeps inside
    // another record. The reviewed allowlist says which ones are derivable.
    // Seed the collision set with every entry already built so a rule cannot
    // silently claim an existing ID.
    let mut seen_ids: BTreeSet<EntryId> = entry_order
        .iter()
        .chain(derived_entries.iter())
        .map(|entry| entry.id.clone())
        .collect();
    let mut rule_entries: Vec<RuleEntry> = Vec::new();
    let by_identity: BTreeMap<String, &InputRecord> = converted_inputs
        .iter()
        .map(|record| (format!("{}#{}", record.path, record.key), record))
        .collect();
    for record in inv
        .records
        .iter()
        .filter(|record| in_scope(record))
        .filter(|record| selected.contains(&(record.path.clone(), record.key.clone())))
    {
        let rules = allowlist.for_host(&record.path, &record.key);
        if rules.is_empty() {
            continue;
        }
        result.derived_expected += rules.len();
        let identity = format!("{}#{}", record.path, record.key);
        // A host that failed conversion must not silently drop its allowlisted
        // rules. Report each one and count it as skipped.
        let Some(&expanded) = by_identity.get(&identity) else {
            result.derived_skipped += rules.len();
            for rule in rules {
                let reason = format!(
                    "allowlisted rule {} skipped; host record did not convert",
                    rule.rule
                );
                result.findings.push(Finding {
                    severity: "medium".into(),
                    record: identity.clone(),
                    problem: reason.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity: identity.clone(),
                    id: Some(rule.id.clone()),
                    category: "rule".into(),
                    reason,
                });
            }
            continue;
        };
        let Some(parent) = native_key(expanded).and_then(|key| context.targets.get(&key).cloned())
        else {
            result.derived_skipped += rules.len();
            for rule in rules {
                let reason = format!(
                    "allowlisted rule {} skipped; host record has no resolved parent",
                    rule.rule
                );
                result.findings.push(Finding {
                    severity: "medium".into(),
                    record: identity.clone(),
                    problem: reason.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity: identity.clone(),
                    id: Some(rule.id.clone()),
                    category: "rule".into(),
                    reason,
                });
            }
            continue;
        };
        let lines = match description(&expanded.raw) {
            Ok(lines) => lines,
            Err(problem) => {
                result.derived_skipped += rules.len();
                for rule in rules {
                    let reason = format!(
                        "allowlisted rule {} skipped; host description could not be read: {problem}",
                        rule.rule
                    );
                    result.findings.push(Finding {
                        severity: "medium".into(),
                        record: identity.clone(),
                        problem: reason.clone(),
                    });
                    result.excluded.push(ExcludedRecord {
                        identity: identity.clone(),
                        id: Some(rule.id.clone()),
                        category: "rule".into(),
                        reason,
                    });
                }
                continue;
            }
        };
        let extracted = crate::pf_rules::extract(&lines, rules);
        // Report every allowlisted rule that extraction did not produce, so a
        // missing heading or empty body is never a silent drop.
        let extracted_names: BTreeSet<&str> = extracted.iter().map(|e| e.rule.as_str()).collect();
        for rule in rules {
            if !extracted_names.contains(rule.rule.as_str()) {
                result.derived_skipped += 1;
                let reason = format!(
                    "allowlisted rule {} was not extracted from the host body",
                    rule.rule
                );
                result.findings.push(Finding {
                    severity: "medium".into(),
                    record: identity.clone(),
                    problem: reason.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity: identity.clone(),
                    id: Some(rule.id.clone()),
                    category: "rule".into(),
                    reason,
                });
            }
        }
        for extracted in extracted {
            let id = EntryId::parse(extracted.id.clone()).map_err(|e| e.to_string())?;
            if !seen_ids.insert(id.clone()) {
                result.derived_skipped += 1;
                let reason =
                    "duplicate derived id; another rule already claims this host and title"
                        .to_string();
                result.findings.push(Finding {
                    severity: "medium".into(),
                    record: extracted.id.clone(),
                    problem: reason.clone(),
                });
                result.excluded.push(ExcludedRecord {
                    identity: format!("{identity}|{}:{}", extracted.shape.slug(), extracted.slug),
                    id: Some(extracted.id.clone()),
                    category: "rule".into(),
                    reason,
                });
                continue;
            }
            match derive_rule_entry(
                expanded,
                &extracted,
                &context,
                &parent,
                &mut result.findings,
                &mut coverage,
            ) {
                Ok(entry) => {
                    match extracted.shape {
                        crate::pf_rules::RuleShape::Section => result.derived_sections += 1,
                        crate::pf_rules::RuleShape::Row => result.derived_rows += 1,
                    }
                    let pages = source_attribution(&extracted.body_lines);
                    let pages = if pages.is_empty() {
                        identity_pages.get(&identity).cloned().unwrap_or_default()
                    } else {
                        pages
                    };
                    if !pages.is_empty() {
                        result.pages.insert(entry.id.clone(), pages);
                    }
                    links.push(crate::build::NestedLink {
                        host: parent.clone(),
                        target: entry.id.clone(),
                        trail: Vec::new(),
                        child: true,
                    });
                    rule_entries.push(entry);
                }
                Err(problem) => {
                    result.derived_skipped += 1;
                    let reason = format!("derived rule {}: {problem}", extracted.id);
                    result.findings.push(Finding {
                        severity: "high".into(),
                        record: extracted.rule.clone(),
                        problem: reason.clone(),
                    });
                    result.excluded.push(ExcludedRecord {
                        identity: format!(
                            "{identity}|{}:{}",
                            extracted.shape.slug(),
                            extracted.slug
                        ),
                        id: Some(extracted.id),
                        category: "rule".into(),
                        reason,
                    });
                }
            }
        }
    }
    result.derived_rules = result.derived_sections + result.derived_rows;

    result.derived_entry_ids = derived_entries
        .iter()
        .chain(rule_entries.iter())
        .map(|entry| entry.id.to_string())
        .collect();
    result.entries.extend(entry_order);
    result.entries.extend(derived_entries);
    result.entries.extend(rule_entries);
    result.entries.sort_by(|a, b| a.id.cmp(&b.id));
    links.sort_by(|a, b| (&a.host, &a.target).cmp(&(&b.host, &b.target)));
    links.dedup();
    result.links = links;
    result.findings.sort_by(|a, b| {
        (&a.record, &a.problem, &a.severity).cmp(&(&b.record, &b.problem, &b.severity))
    });
    result.excluded.sort_by(|a, b| {
        (&a.identity, &a.id, &a.reason, &a.category).cmp(&(
            &b.identity,
            &b.id,
            &b.reason,
            &b.category,
        ))
    });
    result.coverage = coverage.into_rows();
    Ok(result)
}

fn representative_selection(
    inv: &Inventory,
    in_scope: &impl Fn(&InputRecord) -> bool,
) -> BTreeSet<(String, String)> {
    let mut selected: BTreeSet<(String, String)> = BTreeSet::new();
    // Mandatory records plus a bounded sample of every canonical family.
    for record in inv.records.iter().filter(|record| in_scope(record)) {
        if record.shape != "publication"
            && crate::pf_data::FIXTURES
                .iter()
                .any(|name| record.name == *name || record.name.starts_with(&format!("{name} (")))
        {
            selected.insert((record.path.clone(), record.key.clone()));
        }
    }
    for kind in EntryKind::ALL {
        for record in inv
            .records
            .iter()
            .filter(|record| in_scope(record))
            .filter(|record| {
                record.shape == "description" && crate::pf_data::record_kind(&record.path) == kind
            })
            .take(3)
        {
            selected.insert((record.path.clone(), record.key.clone()));
        }
    }
    for prefix in ["json/npcs", "json/rules_"] {
        for record in inv
            .records
            .iter()
            .filter(|record| in_scope(record))
            .filter(|record| record.path.starts_with(prefix) && record.shape == "description")
            .take(2)
        {
            selected.insert((record.path.clone(), record.key.clone()));
        }
    }
    selected
}

fn full_selection(
    inv: &Inventory,
    in_scope: &impl Fn(&InputRecord) -> bool,
) -> BTreeSet<(String, String)> {
    inv.records
        .iter()
        .filter(|record| in_scope(record))
        .filter(|record| ["description", "copy", "compilation"].contains(&record.shape.as_str()))
        .map(|record| (record.path.clone(), record.key.clone()))
        .collect()
}

pub(crate) fn native_key(record: &InputRecord) -> Option<String> {
    record
        .protocol
        .as_ref()
        .map(|protocol| format!("{protocol}/{}", record.key))
}

/// Index records such as `rules_adv_gm01.json#advanced_gm_topics` carry no
/// publication of their own. Their subtopics do, so the union is the record's
/// publication set.
pub(crate) fn derive_publications(raw: &Value, records: &[InputRecord]) -> Vec<String> {
    let Some(subtopics) = raw.get("subtopics").and_then(Value::as_array) else {
        return Vec::new();
    };
    let keys: BTreeSet<&str> = subtopics.iter().filter_map(Value::as_str).collect();
    let mut publications = BTreeSet::new();
    for record in records {
        if keys.contains(record.key.as_str()) {
            publications.extend(record.publications.iter().cloned());
        }
    }
    publications.into_iter().collect()
}

/// Page attribution from a record's screened source markers.
///
/// Pf Data writes attribution as `‹SOURCE Book/12;Other Book/7-9›`, or as a
/// `source="Book/12"` attribute on a directive. Each part names one publication
/// and, when present, the page where the record starts. A range or list keeps
/// its first page. A publication repeated in one record keeps its earliest page,
/// because that is where the record first appears in print. The order follows
/// the record's first mention, so two runs over the same input agree.
pub(crate) fn source_attribution(lines: &[String]) -> Vec<(String, Option<u32>)> {
    let mut order: Vec<String> = Vec::new();
    let mut pages: BTreeMap<String, Option<u32>> = BTreeMap::new();
    let mut add = |text: &str| {
        for part in text.split([';', '~']) {
            let Some((publication, page)) = split_source_part(part) else {
                continue;
            };
            if !pages.contains_key(&publication) {
                order.push(publication.clone());
            }
            let slot = pages.entry(publication).or_insert(page);
            if let Some(page) = page {
                *slot = Some(slot.map_or(page, |earlier| earlier.min(page)));
            }
        }
    };
    for line in lines {
        for marker in source_markers(line) {
            add(marker);
        }
        for value in source_attributes(line) {
            add(value);
        }
    }
    order
        .into_iter()
        .map(|publication| {
            let page = pages[&publication];
            (publication, page)
        })
        .collect()
}

/// The inside of every `‹SOURCE …›` marker on a line.
fn source_markers(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('‹') {
        let after = &rest[start + '‹'.len_utf8()..];
        let Some(end) = after.find('›') else { break };
        let inner = &after[..end];
        if let Some(text) = inner.strip_prefix("SOURCE ") {
            out.push(text);
        }
        rest = &after[end + '›'.len_utf8()..];
    }
    out
}

/// Every `source="…"` attribute value on a line.
fn source_attributes(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("source=\"") {
        let after = &rest[start + "source=\"".len()..];
        let Some(end) = after.find('"') else { break };
        let value = after[..end].trim();
        let value = value.strip_prefix("‹SOURCE ").unwrap_or(value);
        out.push(value.strip_suffix('›').unwrap_or(value));
        rest = &after[end + 1..];
    }
    out
}

/// Split one source part into its publication and its first page.
///
/// The page separator is a slash whose suffix starts with a digit. A slash in a
/// publication title, or a suffix that is not a page, stays part of the title.
fn split_source_part(part: &str) -> Option<(String, Option<u32>)> {
    let part = part.trim().trim_end_matches('~').trim();
    if part.is_empty() {
        return None;
    }
    if let Some((publication, pages)) = part.rsplit_once('/') {
        let publication = publication.trim();
        let pages = pages.trim();
        if !publication.is_empty()
            && pages.starts_with(|c: char| c.is_ascii_digit())
            && let Some(page) = leading_page(pages)
        {
            return Some((publication.to_string(), Some(page)));
        }
    }
    Some((part.to_string(), None))
}

fn leading_page(value: &str) -> Option<u32> {
    let digits: String = value
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

#[derive(Default)]
pub(crate) struct LinkContext {
    pub(crate) targets: BTreeMap<String, EntryId>,
    pub(crate) known: BTreeSet<String>,
    pub(crate) selected: BTreeSet<String>,
    /// Selected native keys whose record did not convert. A link to one of these
    /// is explained coverage, not a second unresolved-reference finding: the
    /// record's own conversion finding is the root cause.
    pub(crate) failed: BTreeSet<String>,
    /// Canonical ID of every derived child, keyed by parent identity and display name.
    pub(crate) derived: BTreeMap<String, BTreeMap<String, EntryId>>,
}

/// Resolves upstream hrefs and records an entry's references and diagnostics.
struct ConvertEnv<'a> {
    record: String,
    findings: &'a mut Vec<Finding>,
    coverage: &'a mut CoverageLog,
    context: &'a LinkContext,
    references: BTreeSet<cast_iron_schema::EntryReference>,
    current: &'a EntryId,
    anchors: BTreeMap<String, String>,
}

impl crate::html::Env for ConvertEnv<'_> {
    fn link(&mut self, href: &str) -> Option<(EntryId, Option<String>)> {
        if let Some(key) = href.strip_prefix('#')
            && let Some(anchor) = self.anchors.get(key)
        {
            self.references.insert(cast_iron_schema::EntryReference {
                target: self.current.clone(),
                anchor: Some(anchor.clone()),
            });
            return Some((self.current.clone(), Some(anchor.clone())));
        }
        if let Some(target) = self.context.targets.get(href) {
            self.references.insert(cast_iron_schema::EntryReference {
                target: target.clone(),
                anchor: None,
            });
            return Some((target.clone(), None));
        }
        // Publication references are verified against the native source registry;
        // the current inline model has entry links only, so retain their text.
        if href.to_ascii_lowercase().starts_with("source/") && self.context.known.contains(href) {
            return None;
        }
        if href.contains("://") {
            self.coverage
                .record("external-link", href, EXTERNAL_LINK_REASON, &self.record);
            return None;
        }
        if href.to_ascii_lowercase().starts_with("source/") {
            self.coverage.record(
                "source-reference",
                href,
                NON_ENTRY_LINK_REASON,
                &self.record,
            );
            return None;
        }
        let outside = self.context.known.contains(href) && !self.context.selected.contains(href);
        if outside {
            // The target is real but not in this bounded selection, so the link
            // text stays and the omission is recorded as explained coverage.
            self.coverage.record(
                "reference-outside-sample",
                href,
                OUTSIDE_SAMPLE_REASON,
                &self.record,
            );
        } else if self.context.failed.contains(href) {
            // The target was selected but its own conversion failed. That record
            // already carries the root-cause finding; do not repeat it per link.
            self.coverage.record(
                "reference-to-failed-record",
                href,
                FAILED_TARGET_REASON,
                &self.record,
            );
        } else if !href.starts_with('#') && !href.contains('/') {
            // Same-document app routes and source-local shorthand are not entries.
            self.coverage
                .record("non-entry-link", href, NON_ENTRY_LINK_REASON, &self.record);
        } else {
            self.findings.push(Finding {
                severity: "high".into(),
                record: self.record.clone(),
                problem: format!("unresolved reference: {href}"),
            });
        }
        None
    }

    fn stripped(&mut self, key: &str) {
        if let Some((category, detail, reason)) = presentation_exception(key) {
            self.coverage
                .record(category, &detail, reason, &self.record);
            return;
        }
        self.findings.push(Finding {
            severity: "medium".into(),
            record: self.record.clone(),
            problem: format!("HTML conversion: {key}"),
        });
    }
}

/// Replace `::rtraits` and named `::aff` blocks with links to their derived records.
///
/// Retains inline Markdown for a trait or block that has no derivation, so a
/// rejected child never disappears from its parent.
fn rewrite_derived_directives(
    lines: &mut [String],
    identity: &str,
    context: &LinkContext,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = context.derived.get(identity) else {
        return;
    };
    let quote = |prefix: &str, text: &str| -> String {
        if prefix.trim().is_empty() {
            return text.to_string();
        }
        text.lines()
            .map(|l| format!("{prefix}{l}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    for line in lines.iter_mut() {
        let trimmed = line.trim_start().trim_start_matches('>').trim_start();
        let prefix_len = line.len() - trimmed.len();
        let prefix = &line[..prefix_len];
        if trimmed.starts_with("::rtraits") {
            match pf_directives::rtraits(trimmed) {
                Ok(parsed) => {
                    let mut rendered = Vec::new();
                    for trait_ in parsed.traits {
                        match names.get(&trait_.title) {
                            Some(id) => {
                                rendered.push(format!("* [{}]({})", trait_.title, id.as_str()))
                            }
                            None => rendered.push(pf_directives::trait_markdown(&trait_)),
                        }
                    }
                    if !rendered.is_empty() {
                        *line = quote(prefix, &rendered.join("\n\n"));
                    }
                }
                Err(problem) => findings.push(Finding {
                    severity: "high".into(),
                    record: identity.into(),
                    problem: format!("::rtraits: {problem}"),
                }),
            }
        } else if let Some(rest) = trimmed.strip_prefix("::aff") {
            match pf_directives::affliction(rest, true) {
                Ok(parsed) => {
                    if let Some(name) = &parsed.name
                        && let Some(id) = names.get(name)
                    {
                        *line = quote(
                            prefix,
                            &format!("* [Affliction: {}]({})", name, id.as_str()),
                        );
                        continue;
                    }
                    *line = quote(prefix, &parsed.markdown);
                }
                Err(problem) => findings.push(Finding {
                    severity: "high".into(),
                    record: identity.into(),
                    problem: format!("::aff: {problem}"),
                }),
            }
        }
    }
}

/// Upstream addendum codewords. The pinned OGC `json/` stores only the codeword;
/// the reminder text exists only in upstream application code, which is GPL and not
/// Open Game Content (AGENTS.md rule 12). The codeword is therefore validated and the
/// omitted reminder is recorded as coverage instead of being copied.
const ADDENDUM_CODEWORDS: &[&str] = &[
    "1pt",
    "2pts",
    "3pts",
    "4pts",
    "avenger",
    "blood",
    "devilbane",
    "hiddenstrike",
    "ninjasneak",
    "primarybomb",
    "slayersneak",
    "sneakattack",
    "stalker",
    "studiedstrike",
    "totem",
    "uniquepatron",
];

const ADDENDUM_REASON: &str = "The upstream record names a reminder by codeword only. Its text lives in the upstream renderer code, which is GPL and not Open Game Content, so it is not copied; the record body is complete without it.";

fn addenda_codewords(raw: &Value) -> Result<Vec<&str>, String> {
    let Some(value) = raw.get("addenda") else {
        return Ok(Vec::new());
    };
    let codes = value
        .as_array()
        .ok_or_else(|| "addenda is not an array".to_string())?;
    let mut out = Vec::new();
    for code in codes {
        let code = code
            .as_str()
            .ok_or_else(|| "non-string addenda codeword".to_string())?;
        if !ADDENDUM_CODEWORDS.contains(&code) {
            return Err(format!("unknown addenda codeword {code}"));
        }
        out.push(code);
    }
    Ok(out)
}

pub(crate) fn convert_record(
    record: &InputRecord,
    context: &LinkContext,
    findings: &mut Vec<Finding>,
    coverage: &mut CoverageLog,
) -> Result<RuleEntry, String> {
    if record.shape != "description" {
        return Err(format!("{} expansion is not implemented", record.shape));
    }
    // Table references have already been expanded by pf_records. Addenda are
    // upstream codewords whose reminder text is not Open Game Content.
    for code in addenda_codewords(&record.raw)? {
        coverage.record(
            "addendum-reminder-omitted",
            code,
            ADDENDUM_REASON,
            &format!("{}#{}", record.path, record.key),
        );
    }
    let lines = description(&record.raw)?;
    if record.name.is_empty() || record.publications.is_empty() {
        return Err("missing name or publication metadata".into());
    }
    // Prestige-class archetypes share their parent family's upstream file.
    let kind = if record.protocol.as_deref() == Some("pclass")
        && lines
            .iter()
            .any(|l| l.starts_with("**This is an archetype of "))
    {
        EntryKind::Archetype
    } else {
        crate::pf_data::record_kind(&record.path)
    };
    let fields = pf_fields::extract(kind, &lines)?;
    let id = pf_derived::native_id(record).ok_or("no native protocol mapping")?;
    let mut env = ConvertEnv {
        record: format!("{}#{}", record.path, record.key),
        findings,
        coverage,
        context,
        references: BTreeSet::new(),
        current: &id,
        anchors: pf_markdown::anchors(&lines),
    };
    let mut render_lines = lines.clone();
    if matches!(fields, cast_iron_schema::TypedFields::Spell(_)) {
        for line in &mut render_lines {
            if line.trim_start().starts_with("::spell{") {
                let attrs = pf_markdown::attributes(line)?;
                *line = attrs
                    .get("source")
                    .map(|s| format!("‹SOURCE {s}›"))
                    .unwrap_or_default();
            }
        }
    }
    rewrite_derived_directives(
        &mut render_lines,
        &format!("{}#{}", record.path, record.key),
        context,
        &mut *env.findings,
    );
    let mut body =
        pf_markdown::render(&render_lines, &mut env).map_err(|errors| errors.join("; "))?;
    if let cast_iron_schema::TypedFields::Spell(spell) = &fields {
        use cast_iron_schema::{Block, Inline, StatRow};
        let mut rows = vec![
            StatRow {
                label: "School".into(),
                value: vec![Inline::text(&spell.school)],
            },
            StatRow {
                label: "Level".into(),
                value: vec![Inline::text(
                    spell
                        .levels
                        .iter()
                        .map(|l| format!("{} {}", l.class, l.level))
                        .collect::<Vec<_>>()
                        .join(", "),
                )],
            },
        ];
        for (label, value) in [
            ("Casting time", &spell.casting_time),
            ("Components", &spell.components),
            ("Range", &spell.range),
            ("Duration", &spell.duration),
            ("Saving throw", &spell.saving_throw),
            ("Spell resistance", &spell.spell_resistance),
        ] {
            if let Some(value) = value {
                rows.push(StatRow {
                    label: label.into(),
                    value: vec![Inline::text(value)],
                });
            }
        }
        for (label, value) in &spell.effects {
            rows.push(StatRow {
                label: label.clone(),
                value: vec![Inline::text(value)],
            });
        }
        if !spell.subschools.is_empty() {
            rows.push(StatRow {
                label: "Subschool".into(),
                value: vec![Inline::text(spell.subschools.join(", "))],
            });
        }
        if !spell.descriptors.is_empty() {
            rows.push(StatRow {
                label: "Descriptors".into(),
                value: vec![Inline::text(spell.descriptors.join(", "))],
            });
        }
        body.insert(
            0,
            Block::StatBlock {
                title: Some("Spell statistics".into()),
                rows,
            },
        );
    }
    if blocks_text(&body).trim().is_empty() {
        return Err("empty rendered description".into());
    }
    Ok(RuleEntry {
        id: id.clone(),
        kind,
        title: record.name.clone(),
        source_ids: record
            .publications
            .iter()
            .map(|source| slug_segment(source))
            .collect(),
        section: Vec::new(),
        parent: None,
        body,
        fields,
        tags: Vec::new(),
        links: env.references.into_iter().collect(),
        provenance: vec![Provenance {
            dataset: "pathfinder-data-1-e".into(),
            record: record.key.clone(),
            path: record.path.clone(),
        }],
        license_refs: pf_license_refs(),
        relations: Vec::new(),
    })
}

/// Build one derived entry from its planned record. The host entry is optional
/// because a host that still fails conversion must not suppress self-contained
/// affliction or trait content; the parent link is omitted in that case.
fn derive_entry(
    derivation: &pf_derived::Derivation,
    parent: Option<&RuleEntry>,
    context: &LinkContext,
    findings: &mut Vec<Finding>,
    coverage: &mut CoverageLog,
) -> Result<RuleEntry, String> {
    let mut env = ConvertEnv {
        record: derivation.parent_identity.clone(),
        findings,
        coverage,
        context,
        references: BTreeSet::new(),
        current: &derivation.id,
        anchors: BTreeMap::new(),
    };
    let body = pf_derived::body(derivation, &mut env)?;
    if blocks_text(&body).trim().is_empty() {
        return Err("empty rendered derived body".into());
    }
    let (fields, relations) = match derivation.kind {
        pf_derived::DerivedKind::RacialTrait => (
            cast_iron_schema::TypedFields::RacialTrait(cast_iron_schema::RacialTraitFields {
                alternate: false,
            }),
            parent
                .map(|parent| cast_iron_schema::Relation {
                    kind: cast_iron_schema::RelationKind::Part,
                    target: parent.id.clone(),
                    label: None,
                })
                .into_iter()
                .collect(),
        ),
        pf_derived::DerivedKind::Affliction => (cast_iron_schema::TypedFields::None, Vec::new()),
    };
    Ok(RuleEntry {
        id: derivation.id.clone(),
        kind: derivation.kind.kind(),
        title: derivation.title.clone(),
        source_ids: derivation.source_ids.clone(),
        section: Vec::new(),
        parent: parent.map(|parent| parent.id.clone()),
        body,
        fields,
        tags: Vec::new(),
        links: env.references.into_iter().collect(),
        provenance: vec![Provenance {
            dataset: "pathfinder-data-1-e".into(),
            record: derivation.provenance_record.clone(),
            path: derivation.path.clone(),
        }],
        license_refs: parent
            .map(|parent| parent.license_refs.clone())
            .unwrap_or_else(pf_license_refs),
        relations,
    })
}

/// Build one derived rule entry from a nested section or table row. The body is
/// rendered through the shared typed converter so no content is dropped.
fn derive_rule_entry(
    input: &InputRecord,
    derived: &crate::pf_rules::ExtractedRule,
    context: &LinkContext,
    parent: &EntryId,
    findings: &mut Vec<Finding>,
    coverage: &mut CoverageLog,
) -> Result<RuleEntry, String> {
    let id = EntryId::parse(derived.id.clone()).map_err(|e| e.to_string())?;
    let record = format!(
        "{}#{}|{}:{}",
        input.path,
        input.key,
        derived.shape.slug(),
        derived.slug
    );
    let mut env = ConvertEnv {
        record: record.clone(),
        findings,
        coverage,
        context,
        references: BTreeSet::new(),
        current: &id,
        anchors: BTreeMap::new(),
    };
    // A nested spell section can carry the source spell directive. Its mechanics
    // belong to the native spell entry, so retain only the attribution marker.
    let mut render_lines = derived.body_lines.clone();
    for line in &mut render_lines {
        if line.trim_start().starts_with("::spell{")
            && let Ok(attrs) = pf_markdown::attributes(line)
        {
            *line = attrs
                .get("source")
                .map(|source| format!("‹SOURCE {source}›"))
                .unwrap_or_default();
        }
    }
    let body = pf_markdown::render(&render_lines, &mut env).map_err(|errors| errors.join("; "))?;
    if blocks_text(&body).trim().is_empty() {
        return Err("empty derived description".into());
    }
    Ok(RuleEntry {
        id: id.clone(),
        kind: EntryKind::Rule,
        title: derived.title.clone(),
        source_ids: input
            .publications
            .iter()
            .map(|source| slug_segment(source))
            .collect(),
        section: vec![input.name.clone()],
        parent: Some(parent.clone()),
        body,
        fields: cast_iron_schema::TypedFields::None,
        tags: Vec::new(),
        links: env.references.into_iter().collect(),
        provenance: vec![Provenance {
            dataset: "pathfinder-data-1-e".into(),
            record,
            path: input.path.clone(),
        }],
        license_refs: pf_license_refs(),
        relations: Vec::new(),
    })
}

/// Stable, deterministic hash of the serialized entries. Two conversion runs over
/// the same inventory must produce the same digest.
pub fn entries_digest(entries: &[RuleEntry]) -> Result<String, String> {
    let bytes = serde_json::to_vec(entries).map_err(|e| e.to_string())?;
    Ok(sha256_hex(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pf_rules::RuleAllowlist;
    use serde_json::json;

    fn inventory(records: Vec<InputRecord>) -> Inventory {
        Inventory {
            dataset: "pathfinder-data-1-e".into(),
            commit: "0".repeat(40),
            input_digest: "0".repeat(64),
            json_files: 1,
            input_records: records.len(),
            registered_publications: 1,
            by_file: BTreeMap::new(),
            by_shape: BTreeMap::new(),
            by_publication: BTreeMap::new(),
            directives: BTreeMap::new(),
            unregistered_publications: Vec::new(),
            out_of_scope: Vec::new(),
            pi_omissions: Vec::new(),
            records,
        }
    }

    fn record(
        protocol: &str,
        path: &str,
        key: &str,
        name: &str,
        description: Vec<String>,
    ) -> InputRecord {
        InputRecord {
            protocol: Some(protocol.into()),
            path: path.into(),
            key: key.into(),
            name: name.into(),
            shape: "description".into(),
            publications: vec!["PRPG Core Rulebook".into()],
            raw: json!({"description": description, "sources": ["PRPG Core Rulebook"]}),
        }
    }

    fn rule_allowlist() -> RuleAllowlist {
        RuleAllowlist::from_json(
            br#"{
                "format": "cast-iron-derived-rules",
                "version": 1,
                "rules": [{
                    "rule": "core-rulebook/rules/host/test-rule",
                    "title": "Test Rule",
                    "host": "json/rules_test.json#host",
                    "protocol": "rule",
                    "shape": "section",
                    "heading": "Test Rule",
                    "id": "ref/rule/rule-host/test-rule"
                }]
            }"#,
        )
        .unwrap()
    }

    fn sample_inventory() -> Inventory {
        inventory(vec![
            record(
                "race",
                "json/races.json",
                "dwarf",
                "Dwarf",
                vec![
                    "Dwarves are a sturdy people.".into(),
                    "::rtraits{plural=Dwarves main=\"+2 Constitution|Dwarves are tough.~MEDIUM~Stonecunning|Dwarves notice stonework.\"}".into(),
                ],
            ),
            record(
                "monster",
                "json/monsters01.json",
                "xacarba",
                "Xacarba",
                vec![
                    "## Xacarba".into(),
                    "".into(),
                    "::aff[Fiendish Bile]{iconP type=Bite-injury saveF=25 freqR=6 effStr=\"1d4\" cure2c}".into(),
                ],
            ),
            record(
                "rule",
                "json/rules_test.json",
                "host",
                "Host",
                vec!["## Test Rule".into(), "Rules-bearing text.".into()],
            ),
        ])
    }

    #[test]
    fn conversion_emits_canonical_ids_links_and_both_license_refs() {
        let inv = sample_inventory();
        let converted = convert_corpus(&inv, &rule_allowlist()).unwrap();
        let ids: BTreeSet<&str> = converted.entries.iter().map(|e| e.id.as_str()).collect();
        for entry in &converted.entries {
            assert!(
                entry.id.as_str().starts_with("ref/"),
                "{} is not canonical",
                entry.id
            );
            assert_eq!(
                entry.license_refs,
                crate::pf_data::pf_license_refs(),
                "{}",
                entry.id
            );
        }
        assert!(ids.contains("ref/race/dwarf"));
        assert!(ids.contains("ref/racial-trait/dwarf/stonecunning"));
        assert!(ids.contains("ref/monster/xacarba"));
        assert!(ids.contains("ref/affliction/xacarba/fiendish-bile"));
        assert!(ids.contains("ref/rule/rule-host/test-rule"));
        assert!(converted.excluded.is_empty(), "{:?}", converted.excluded);

        let linked: BTreeSet<(&str, &str, bool)> = converted
            .links
            .iter()
            .map(|link| (link.host.as_str(), link.target.as_str(), link.child))
            .collect();
        assert!(linked.contains(&(
            "ref/race/dwarf",
            "ref/racial-trait/dwarf/stonecunning",
            false
        )));
        assert!(linked.contains(&("ref/rule/host", "ref/rule/rule-host/test-rule", true)));
        for link in &converted.links {
            assert!(ids.contains(link.host.as_str()));
            assert!(ids.contains(link.target.as_str()));
            assert!(link.target.as_str().starts_with("ref/"));
        }
        for entry in &converted.entries {
            for link in &entry.links {
                assert!(
                    link.target.as_str().starts_with("ref/"),
                    "{} links to non-canonical {}",
                    entry.id,
                    link.target
                );
                assert!(
                    ids.contains(link.target.as_str()),
                    "{} links to missing {}",
                    entry.id,
                    link.target
                );
            }
        }
    }

    #[test]
    fn repeated_conversion_is_byte_identical() {
        let inv = sample_inventory();
        let allowlist = rule_allowlist();
        let first = convert_corpus(&inv, &allowlist).unwrap();
        let second = convert_corpus(&inv, &allowlist).unwrap();
        assert_eq!(
            serde_json::to_vec(&first.entries).unwrap(),
            serde_json::to_vec(&second.entries).unwrap()
        );
        assert_eq!(
            entries_digest(&first.entries),
            entries_digest(&second.entries)
        );
        assert_eq!(first.links, second.links);
        assert_eq!(first.pages, second.pages);
        assert_eq!(first.excluded, second.excluded);
    }

    #[test]
    fn page_attribution_reads_markers_and_source_attributes() {
        let lines = vec![
            ">‹SOURCE Book/12;Other Book/7-9›".into(),
            "::spell{source=\"Third Book/4\"}".into(),
            "‹SOURCE Book/10›".into(),
            "‹SOURCE No Page›".into(),
            "‹SOURCE A/58~B/8›".into(),
        ];
        assert_eq!(
            source_attribution(&lines),
            vec![
                ("Book".into(), Some(10)),
                ("Other Book".into(), Some(7)),
                ("Third Book".into(), Some(4)),
                ("No Page".into(), None),
                ("A".into(), Some(58)),
                ("B".into(), Some(8)),
            ]
        );
    }

    #[test]
    fn derived_children_inherit_the_host_page_attribution() {
        let inv = inventory(vec![record(
            "race",
            "json/races.json",
            "dwarf",
            "Dwarf",
            vec![
                "‹SOURCE PRPG Core Rulebook/20›".into(),
                "Dwarves are a sturdy people.".into(),
                "::rtraits{plural=Dwarves main=\"+2 Constitution|Dwarves are tough.~MEDIUM~Stonecunning|Dwarves notice stonework.\"}".into(),
            ],
        )]);
        let converted = convert_corpus(&inv, &RuleAllowlist::default()).unwrap();
        let pages = vec![("PRPG Core Rulebook".to_string(), Some(20))];
        for id in ["ref/race/dwarf", "ref/racial-trait/dwarf/stonecunning"] {
            let entry = converted
                .entries
                .iter()
                .find(|entry| entry.id.as_str() == id)
                .unwrap_or_else(|| panic!("{id} was not converted"));
            assert_eq!(converted.pages.get(&entry.id), Some(&pages), "{id}");
        }
    }

    /// The production contract: conversion and the alias catalog share one id
    /// derivation. Every emitted id is offered by the catalog, and every catalog
    /// id is either emitted or explicitly excluded with a reason.
    #[test]
    fn real_corpus_ids_match_the_alias_catalog() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dataset = root.join(".cache/sources/pathfinder-data-1-e");
        if !dataset.join("json").is_dir() {
            eprintln!("skipping: the pinned primary source is not present");
            return;
        }
        let inv = crate::pf_data::inventory(
            &dataset,
            &root.join("sources/pathfinder-data-1-e/source.json"),
        )
        .unwrap();
        let allowlist = RuleAllowlist::load_optional(
            &root.join("sources/pathfinder-data-1-e/derived-rules.json"),
        )
        .unwrap();
        let converted = convert_corpus(&inv, &allowlist).unwrap();
        let catalog = crate::alias::catalog_from_inventory(&inv, &allowlist).unwrap();

        let converted_ids: BTreeSet<&str> =
            converted.entries.iter().map(|e| e.id.as_str()).collect();
        for entry in &converted.entries {
            for link in &entry.links {
                assert!(
                    link.target.as_str().starts_with("ref/"),
                    "{} links to non-canonical {}",
                    entry.id,
                    link.target
                );
                assert!(
                    converted_ids.contains(link.target.as_str()),
                    "{} links to missing {}",
                    entry.id,
                    link.target
                );
            }
        }
        for record in &inv.out_of_scope {
            let Some(protocol) = inv
                .records
                .iter()
                .find(|input| input.path == record.path && input.key == record.key)
                .and_then(|input| input.protocol.as_deref())
            else {
                continue;
            };
            let id = crate::alias::canonical_native_id(protocol, &record.key).unwrap();
            assert!(
                !converted_ids.contains(id.as_str()),
                "out-of-scope {}#{} was emitted as {id}",
                record.path,
                record.key
            );
        }
        for id in &converted_ids {
            assert!(
                catalog.contains(id),
                "converted id {id} is absent from the alias catalog"
            );
        }
        let mut accounted: BTreeSet<&str> = converted_ids;
        for row in &converted.excluded {
            assert!(
                !row.reason.trim().is_empty(),
                "exclusion {} has no reason",
                row.identity
            );
            if let Some(id) = &row.id {
                accounted.insert(id.as_str());
            }
        }
        for record in catalog.iter() {
            assert!(
                accounted.contains(record.id.as_str()),
                "catalog id {} is neither converted nor excluded with a reason",
                record.id
            );
        }
    }
}
