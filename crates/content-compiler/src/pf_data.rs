//! Pinned primary-source inventory and isolated migration experiments.
//! No output from this module is placed in the production library.

use crate::{
    pf_derived, pf_directives, pf_fields, pf_markdown, select::sha256_hex, slug::slug_segment,
    text::blocks_text,
};
use cast_iron_schema::{Catalog, EntryId, EntryKind, Provenance, RuleEntry, SourceInfo};
use cast_iron_search::{DocInput, IndexBuilder, Query, SearchIndex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub id: String,
    pub repository: String,
    pub commit: String,
    pub archive_sha256: String,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputRecord {
    pub protocol: Option<String>,
    pub path: String,
    pub key: String,
    pub name: String,
    pub shape: String,
    pub publications: Vec<String>,
    /// Original input, retained only in the isolated build output.
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutOfScope {
    pub path: String,
    pub key: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Default, serde::Deserialize)]
struct ScopePolicy {
    #[serde(default)]
    publications: BTreeMap<String, String>,
    #[serde(default)]
    records: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct Inventory {
    pub dataset: String,
    pub commit: String,
    pub input_digest: String,
    pub json_files: usize,
    pub input_records: usize,
    pub registered_publications: usize,
    pub by_file: BTreeMap<String, usize>,
    pub by_shape: BTreeMap<String, usize>,
    pub by_publication: BTreeMap<String, usize>,
    pub directives: BTreeMap<String, usize>,
    pub unregistered_publications: Vec<String>,
    /// Records excluded before selection because their publication or record is outside PF1e OGL scope.
    pub out_of_scope: Vec<OutOfScope>,
    /// Product Identity and outbound-link omissions applied to `records` by
    /// `sources/pathfinder-data-1-e/pi-screen.json`, in record order.
    pub pi_omissions: Vec<PiOmission>,
    pub records: Vec<InputRecord>,
}

/// One screened omission, keyed by the upstream `path#key` it was taken from.
#[derive(Debug, Clone, Serialize)]
pub struct PiOmission {
    pub record: String,
    pub category: String,
    pub detail: String,
    pub reason: String,
}

pub fn inventory(dataset: &Path, registry: &Path) -> Result<Inventory, String> {
    let pin: SourcePin = read_json(registry)?;
    if pin.id != "pathfinder-data-1-e"
        || pin.repository != "https://github.com/jasontankapps/pathfinder-data-1-e"
        || !is_hash(&pin.commit, 40)
        || !is_hash(&pin.archive_sha256, 64)
    {
        return Err("invalid primary source identity or pin".into());
    }
    // Compare the file set too: a new or removed JSON file cannot go unnoticed.
    let mut actual: BTreeSet<String> = fs::read_dir(dataset.join("json"))
        .map_err(|e| e.to_string())?
        .map(|e| e.map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.path().extension().is_some_and(|e| e == "json"))
        .map(|e| format!("json/{}", e.file_name().to_string_lossy()))
        .collect();
    actual.extend(
        [
            "README.md",
            "JSON.md",
            "OPENGAMECONTENT.md",
            "ORCLICENSE.md",
            "GNULICENSE.md",
            "prebuild/basic_data_groups.js",
        ]
        .map(String::from),
    );
    if actual != pin.files.keys().cloned().collect() {
        return Err("source file set differs from the reviewed pin".into());
    }
    let mut digest_lines = String::new();
    for (path, hash) in &pin.files {
        if !is_hash(hash, 64) || path.contains("..") || path.starts_with('/') {
            return Err(format!("invalid pinned path or hash: {path}"));
        }
        let bytes = fs::read(dataset.join(path)).map_err(|e| format!("{path}: {e}"))?;
        if sha256_hex(&bytes) != *hash {
            return Err(format!("source hash mismatch: {path}"));
        }
        digest_lines.push_str(&format!("{path}:{hash}\n"));
    }
    let config_dir = registry.parent().ok_or("registry has no parent")?;
    let screen_path = config_dir.join("pi-screen.json");
    let screen = crate::pf_screen::PiScreen::load(&screen_path)?;
    let screen_bytes = fs::read(&screen_path).map_err(|e| e.to_string())?;
    digest_lines.push_str(&format!("pi-screen.json:{}\n", sha256_hex(&screen_bytes)));
    let publications: BTreeMap<String, Value> = read_json(&dataset.join("json/sources.json"))?;
    let routes: BTreeMap<String, String> = read_json(
        &registry
            .parent()
            .ok_or("registry has no parent")?
            .join("routes.json"),
    )?;
    let mut out = Inventory {
        dataset: pin.id,
        commit: pin.commit,
        input_digest: sha256_hex(digest_lines.as_bytes()),
        json_files: 0,
        input_records: 0,
        registered_publications: publications.len(),
        by_file: BTreeMap::new(),
        by_shape: BTreeMap::new(),
        by_publication: BTreeMap::new(),
        directives: BTreeMap::new(),
        unregistered_publications: Vec::new(),
        out_of_scope: Vec::new(),
        pi_omissions: Vec::new(),
        records: Vec::new(),
    };
    for path in pin.files.keys().filter(|p| p.starts_with("json/")) {
        out.json_files += 1;
        let value: Value = read_json(&dataset.join(path))?;
        let rows: Vec<(String, Value)> = match value {
            Value::Object(rows) => rows.into_iter().collect(),
            Value::Array(rows) => rows
                .into_iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v))
                .collect(),
            other => vec![("root".into(), other)],
        };
        out.by_file.insert(path.clone(), rows.len());
        for (key, raw) in rows {
            let shape = if path == "json/sources.json" {
                "publication"
            } else if path.contains("color") || path.contains("feat_tree") {
                "auxiliary"
            } else if key == "not_found" {
                "placeholder"
            } else if raw.get("redirect").is_some() {
                "redirect"
            } else if raw.get("copyof").is_some() {
                "copy"
            } else if raw.get("compileFrom").is_some() {
                "compilation"
            } else if raw.get("description").is_some() {
                "description"
            } else {
                "unsupported_shape"
            }
            .to_string();
            let sources = publication_names(&raw);
            for source in &sources {
                *out.by_publication.entry(source.clone()).or_default() += 1;
            }
            *out.by_shape.entry(shape.clone()).or_default() += 1;
            let lines = description(&raw).unwrap_or_default();
            for line in &lines {
                for directive in directives(line) {
                    *out.directives.entry(directive).or_default() += 1;
                }
            }
            out.records.push(InputRecord {
                protocol: routes.get(path).cloned(),
                path: path.clone(),
                key,
                name: raw
                    .get("name")
                    .or_else(|| raw.get("title"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                shape,
                publications: sources,
                raw,
            });
        }
    }
    out.records
        .sort_by(|a, b| (&a.path, &a.key).cmp(&(&b.path, &b.key)));
    out.input_records = out.records.len();
    // Screen after the directive inventory, so the inventory still counts every
    // upstream construct while every consumer sees only screened text.
    for record in &mut out.records {
        let Ok(lines) = description(&record.raw) else {
            continue;
        };
        let (kept, omissions) = screen.apply(&record.path, &lines);
        if omissions.is_empty() {
            continue;
        }
        record.raw["description"] = Value::from(kept);
        out.pi_omissions
            .extend(omissions.into_iter().map(|omission| PiOmission {
                record: format!("{}#{}", record.path, record.key),
                category: omission.category.into(),
                detail: omission.detail,
                reason: omission.reason,
            }));
    }
    let policy = scope_policy(registry)?;
    out.out_of_scope = out
        .records
        .iter()
        .filter_map(|record| {
            let record_reason = policy
                .records
                .get(&format!("{}#{}", record.path, record.key));
            // A record with several publications stays in scope when at least one
            // publication is in scope. Only blog/comic/PF2-only records are excluded.
            let publication_reason = (!record.publications.is_empty()
                && record
                    .publications
                    .iter()
                    .all(|publication| policy.publications.contains_key(publication)))
            .then(|| {
                record
                    .publications
                    .iter()
                    .find_map(|publication| policy.publications.get(publication))
            })
            .flatten();
            let reason = record_reason.or(publication_reason)?;
            Some(OutOfScope {
                path: record.path.clone(),
                key: record.key.clone(),
                name: record.name.clone(),
                reason: reason.clone(),
            })
        })
        .collect();
    let registered: BTreeSet<_> = publications
        .values()
        .filter_map(|v| v.get("title").and_then(Value::as_str))
        .collect();
    out.unregistered_publications = out
        .by_publication
        .keys()
        .filter(|name| !registered.contains(name.as_str()))
        .cloned()
        .collect();
    Ok(out)
}

fn scope_policy(registry: &Path) -> Result<ScopePolicy, String> {
    let path = registry
        .parent()
        .ok_or("registry has no parent")?
        .join("out-of-scope.json");
    if !path.exists() {
        return Ok(ScopePolicy::default());
    }
    read_json(&path)
}

pub fn publication_names(raw: &Value) -> Vec<String> {
    fn flatten(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::String(s) => {
                out.insert(s.clone());
            }
            Value::Array(xs) => {
                for x in xs {
                    flatten(x, out);
                }
            }
            _ => {}
        }
    }
    let mut names = BTreeSet::new();
    for key in ["sources", "compilationSources"] {
        if let Some(v) = raw.get(key) {
            flatten(v, &mut names);
        }
    }
    names.into_iter().collect()
}

fn is_hash(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn description(raw: &Value) -> Result<Vec<String>, String> {
    raw.get("description")
        .and_then(Value::as_array)
        .ok_or_else(|| "no description array".to_string())?
        .iter()
        .map(|v| {
            v.as_str()
                .map(String::from)
                .ok_or_else(|| "non-string description line".into())
        })
        .collect()
}

/// Inventory tokens without interpreting or dropping unknown syntax.
pub fn directives(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c != '@' && !(c == ':' && line[i..].starts_with("::")) {
            continue;
        }
        let prefix = if c == '@' { "@" } else { "::" };
        if c == ':' {
            while chars.peek().is_some_and(|(_, c)| *c == ':') {
                chars.next();
            }
        }
        let mut token = String::new();
        while chars
            .peek()
            .is_some_and(|(_, c)| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        {
            if let Some((_, c)) = chars.next() {
                token.push(c);
            }
        }
        if !token.is_empty() {
            result.push(format!("{prefix}{token}"));
        }
    }
    result
}

/// The entry kind a Pf Data file publishes, from its directory name.
pub fn record_kind(path: &str) -> EntryKind {
    let file = path.trim_start_matches("json/");
    if file.starts_with("spells") {
        EntryKind::Spell
    } else if file.starts_with("feat") {
        EntryKind::Feat
    } else if file.starts_with("archetypes") {
        EntryKind::Archetype
    } else if file.starts_with("prestige_classes") {
        EntryKind::PrestigeClass
    } else if file.starts_with("classes") || file.starts_with("npc_classes") {
        EntryKind::Class
    } else if file.starts_with("monsters") || file.starts_with("npcs") {
        EntryKind::Creature
    } else if file.starts_with("magic") || file.starts_with("equipment") {
        EntryKind::Item
    } else if file.starts_with("race") {
        EntryKind::Race
    } else if file.starts_with("traits") {
        EntryKind::Trait
    } else {
        EntryKind::Rule
    }
}

/// Coverage for the mechanical derived layer over the whole in-scope inventory.
///
/// `data-report` writes this block so the derived counts describe the corpus rather
/// than whichever records the representative sample happened to select.
#[derive(Debug, Default, Serialize)]
pub struct DerivedCoverage {
    pub total: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub records: Vec<DerivedRecordCoverage>,
    pub rejected: Vec<pf_derived::Rejection>,
}

#[derive(Debug, Serialize)]
pub struct DerivedRecordCoverage {
    pub id: String,
    pub kind: String,
    pub parent: String,
    pub provenance: String,
}

pub fn derived_coverage(inv: &Inventory) -> DerivedCoverage {
    let out_of_scope: BTreeSet<(String, String)> = inv
        .out_of_scope
        .iter()
        .map(|record| (record.path.clone(), record.key.clone()))
        .collect();
    let mut candidates = Vec::new();
    let mut rejected = Vec::new();
    for record in inv.records.iter().filter(|record| {
        !out_of_scope.contains(&(record.path.clone(), record.key.clone()))
            && pf_derived::is_candidate(record)
    }) {
        match crate::pf_records::expand(record, &inv.records) {
            Ok(raw) => candidates.push(expanded_input(record, raw)),
            Err(problem) => rejected.push(pf_derived::Rejection {
                identity: format!("{}#{}", record.path, record.key),
                problem,
            }),
        }
    }
    let derivations = pf_derived::plan(&candidates, &mut rejected);
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for derivation in &derivations {
        *by_kind.entry(derivation.kind.slug().into()).or_default() += 1;
    }
    DerivedCoverage {
        total: derivations.len(),
        by_kind,
        records: derivations
            .iter()
            .map(|derivation| DerivedRecordCoverage {
                id: derivation.id.to_string(),
                kind: derivation.kind.slug().into(),
                parent: derivation.parent.to_string(),
                provenance: derivation.provenance_record.clone(),
            })
            .collect(),
        rejected,
    }
}

fn expanded_input(record: &InputRecord, raw: Value) -> InputRecord {
    InputRecord {
        name: raw
            .get("name")
            .or_else(|| raw.get("title"))
            .and_then(Value::as_str)
            .unwrap_or(&record.name)
            .to_string(),
        publications: publication_names(&raw),
        shape: "description".into(),
        raw,
        ..record.clone()
    }
}

pub const FIXTURES: &[&str] = &[
    "Mortal Usher",
    "Golem's Conviction",
    "Cradle of Night",
    "Arcing Weapon",
    "Realm Retribution",
    "Apocalyptic Spell",
];

#[derive(Debug, Serialize)]
pub struct Finding {
    pub severity: String,
    pub record: String,
    pub problem: String,
}

/// A construct the converter deliberately omits because it carries no typed
/// content, or a reference it cannot resolve inside a bounded sample. Each row
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

/// Deterministic accumulator keyed by category, detail, and record.
#[derive(Default)]
struct CoverageLog {
    rows: BTreeMap<(String, String, String), (String, usize)>,
}

impl CoverageLog {
    fn record(&mut self, category: &str, detail: &str, reason: &str, record: &str) {
        let key = (category.to_string(), detail.to_string(), record.to_string());
        let entry = self
            .rows
            .entry(key)
            .or_insert_with(|| (reason.to_string(), 0));
        entry.1 += 1;
    }

    fn into_rows(self) -> Vec<CoverageException> {
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

const OUTSIDE_SAMPLE_REASON: &str = "The target exists in the pinned Pf Data inventory but is outside this bounded selection. The visible link text is kept; a full import resolves the link.";

/// Classify an HTML construct that the converter strips as presentation-only.
/// `None` means the key is an actual missing renderer and must stay a finding.
fn presentation_exception(key: &str) -> Option<(&'static str, String, &'static str)> {
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
        "fake_footnotes_container" => (
            "layout-container",
            "`::fakeFootnotes` is a layout wrapper. Its `@FN` definitions are rendered by the inline pass, so no footnote text is lost.",
        ),
        "hr" => (
            "layout-separator",
            "A horizontal rule is a visual separator. Schema 3 has no divider block, and the surrounding paragraphs are preserved.",
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

#[derive(Default)]
struct LinkContext {
    targets: BTreeMap<String, EntryId>,
    known: BTreeSet<String>,
    selected: BTreeSet<String>,
    /// Canonical ID of every derived child, keyed by parent identity and display name.
    derived: BTreeMap<String, BTreeMap<String, EntryId>>,
}

/// Every Pf Data entry is Open Game Content under the OGL that may name Paizo
/// Product Identity as a descriptive reference, so it carries both the OGL and the
/// Community Use notice path (docs/decisions.md, PI screen decision).
pub fn pf_license_refs() -> Vec<String> {
    vec![
        crate::build::LICENSE_OGL.into(),
        crate::legal::COMMUNITY_USE_NOTICE_PATH.into(),
    ]
}

fn native_key(r: &InputRecord) -> Option<String> {
    r.protocol.as_ref().map(|p| format!("{p}/{}", r.key))
}

/// Index records such as `rules_adv_gm01.json#advanced_gm_topics` carry no
/// publication of their own. Their subtopics do, so the union is the record's
/// publication set.
fn derive_publications(raw: &Value, records: &[InputRecord]) -> Vec<String> {
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

#[derive(Debug, Serialize)]
pub struct FixtureResult {
    pub name: String,
    pub records: Vec<String>,
    pub imported: bool,
    pub publications: Vec<String>,
    pub checks: BTreeMap<String, bool>,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct MigrationReport {
    pub scope: String,
    pub input_digest: String,
    pub dataset: String,
    pub commit: String,
    pub input_records: usize,
    pub selected_records: usize,
    pub imported_records: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub by_publication: BTreeMap<String, usize>,
    /// Entries this run built from Pf Data directives rather than one record.
    pub derived_records: usize,
    pub derived_by_kind: BTreeMap<String, usize>,
    /// Derivations refused in this run; each one is also a finding.
    pub derived_rejected: usize,
    /// Every allowlisted rule derivable from the in-scope corpus.
    pub derived_available: usize,
    /// Reviewed rules the allowlist expects for the selected hosts.
    pub derived_expected: usize,
    pub derived_rules: usize,
    pub derived_sections: usize,
    pub derived_rows: usize,
    pub derived_skipped: usize,
    pub out_of_scope: Vec<OutOfScope>,
    pub fixtures: Vec<FixtureResult>,
    pub findings: Vec<Finding>,
    /// Explained omissions: presentation metadata, layout wrappers, and
    /// references outside the bounded sample. These are not converter defects.
    pub coverage: Vec<CoverageException>,
    pub canonical_bytes: usize,
    pub search_bytes: usize,
    pub search_checks: usize,
    pub schema_errors: usize,
    pub publication_ready: bool,
    pub cutover_blockers: Vec<String>,
    pub browser_verification: String,
}

/// A bounded conversion experiment. All inputs stay available in inventory.json.
pub fn experiment(
    inv: &Inventory,
    out: &Path,
    reports: &Path,
    allowlist: &crate::pf_rules::RuleAllowlist,
) -> Result<MigrationReport, String> {
    run_import(inv, out, reports, false, allowlist)
}

pub fn import_all(
    inv: &Inventory,
    out: &Path,
    reports: &Path,
    allowlist: &crate::pf_rules::RuleAllowlist,
) -> Result<MigrationReport, String> {
    run_import(inv, out, reports, true, allowlist)
}

fn run_import(
    inv: &Inventory,
    out: &Path,
    reports: &Path,
    full: bool,
    allowlist: &crate::pf_rules::RuleAllowlist,
) -> Result<MigrationReport, String> {
    let out_of_scope: BTreeSet<(String, String)> = inv
        .out_of_scope
        .iter()
        .map(|record| (record.path.clone(), record.key.clone()))
        .collect();
    let in_scope =
        |record: &&InputRecord| !out_of_scope.contains(&(record.path.clone(), record.key.clone()));
    let mut selected: BTreeSet<(String, String)> = BTreeSet::new();
    // Mandatory records plus a bounded sample of every canonical family.
    for record in inv.records.iter().filter(in_scope) {
        if record.shape != "publication"
            && FIXTURES
                .iter()
                .any(|name| record.name == *name || record.name.starts_with(&format!("{name} (")))
        {
            selected.insert((record.path.clone(), record.key.clone()));
        }
    }
    for kind in EntryKind::ALL {
        for r in inv
            .records
            .iter()
            .filter(in_scope)
            .filter(|r| r.shape == "description" && record_kind(&r.path) == kind)
            .take(3)
        {
            selected.insert((r.path.clone(), r.key.clone()));
        }
    }
    for prefix in ["json/npcs", "json/rules_"] {
        for r in inv
            .records
            .iter()
            .filter(in_scope)
            .filter(|r| r.path.starts_with(prefix) && r.shape == "description")
            .take(2)
        {
            selected.insert((r.path.clone(), r.key.clone()));
        }
    }
    if full {
        selected = inv
            .records
            .iter()
            .filter(in_scope)
            .filter(|r| ["description", "copy", "compilation"].contains(&r.shape.as_str()))
            .map(|r| (r.path.clone(), r.key.clone()))
            .collect();
    }
    // Count the derivable rules across the whole in-scope corpus, even when this
    // run only converts a representative sample.
    let mut derived_available = 0usize;
    for record in inv.records.iter().filter(in_scope) {
        let rules = allowlist.for_host(&record.path, &record.key);
        if rules.is_empty() {
            continue;
        }
        if let Ok(lines) = description(&record.raw) {
            derived_available += crate::pf_rules::extract(&lines, rules).len();
        }
    }
    let mut report = MigrationReport {
        scope: if full { "full-input" } else { "representative" }.into(),
        input_digest:inv.input_digest.clone(),
        dataset: inv.dataset.clone(),
        commit: inv.commit.clone(),
        input_records: inv.input_records,
        selected_records: selected.len(),
        imported_records: 0,
        by_kind: BTreeMap::new(),
        by_publication:BTreeMap::new(),
        derived_records: 0,
        derived_by_kind: BTreeMap::new(),
        derived_rejected: 0,
        derived_available,
        derived_expected: 0,
        derived_rules: 0,
        derived_sections: 0,
        derived_rows: 0,
        derived_skipped: 0,
        out_of_scope: inv.out_of_scope.clone(),
        fixtures: Vec::new(),
        findings: Vec::new(),
        coverage: Vec::new(),
        canonical_bytes: 0,
        search_bytes: 0,
        search_checks: 0,
        schema_errors: 0,
        publication_ready: false,
        cutover_blockers: vec![
            "Section 15 notice reconciliation for every imported publication is incomplete".into(),
            "Existing public IDs, production packaging, and browser upgrade/offline acceptance have not been migrated".into(),
            "Comics are excluded permanently; Paizo Blog records are deferred to issue #36".into(),
        ],
        browser_verification: "not run; isolated compiler experiment".into(),
    };
    let mut entries = Vec::new();
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
            .filter(|r| selected.contains(&(r.path.clone(), r.key.clone())))
            .filter_map(native_key)
            .collect(),
        ..Default::default()
    };
    let mut expanded_inputs = Vec::new();
    for r in inv
        .records
        .iter()
        .filter(|r| selected.contains(&(r.path.clone(), r.key.clone())))
    {
        match crate::pf_records::expand(r, &inv.records) {
            Ok(raw) => {
                let mut record = expanded_input(r, raw);
                if record.publications.is_empty() {
                    let derived = derive_publications(&record.raw, &inv.records);
                    if !derived.is_empty() {
                        record.publications = derived;
                        coverage.record(
                            "publication-derived",
                            &format!("{}#{}", r.path, r.key),
                            "The upstream record carries no publication of its own; its subtopic records supply the attribution.",
                            &format!("{}#{}", r.path, r.key),
                        );
                    }
                }
                expanded_inputs.push(record);
            }
            Err(problem) => report.findings.push(Finding {
                severity: "high".into(),
                record: format!("{}#{}", r.path, r.key),
                problem,
            }),
        }
    }
    // Derived children are planned before conversion so that parent bodies can link
    // to them and their IDs count as known targets.
    let mut derived_rejections = Vec::new();
    let derivations = pf_derived::plan(&expanded_inputs, &mut derived_rejections);
    for rejection in &derived_rejections {
        report.findings.push(Finding {
            // An unnamed `::aff` block has a recognised shape but no derivable
            // identity; that is a coverage gap, not a broken record.
            severity: if rejection.problem.contains("has no name") {
                "medium".into()
            } else {
                "high".into()
            },
            record: rejection.identity.clone(),
            problem: rejection.problem.clone(),
        });
    }
    report.derived_rejected = derived_rejections.len();
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
    for r in expanded_inputs {
        match convert(&r, &context, &mut Vec::new(), &mut CoverageLog::default()) {
            Ok(entry) => {
                if let Some(key) = native_key(&r) {
                    context.targets.insert(key, entry.id);
                }
                converted_inputs.push(r);
            }
            Err(problem) => report.findings.push(Finding {
                severity: "high".into(),
                record: format!("{}#{}", r.path, r.key),
                problem,
            }),
        }
    }
    // Redirects are aliases of a canonical record, never duplicated rules pages.
    for alias in inv.records.iter().filter(|r| r.shape == "redirect") {
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
    let mut entry_order: Vec<RuleEntry> = Vec::new();
    let mut index_by_identity: BTreeMap<String, usize> = BTreeMap::new();
    for r in &converted_inputs {
        let entry = convert(r, &context, &mut report.findings, &mut coverage)?;
        index_by_identity.insert(format!("{}#{}", r.path, r.key), entry_order.len());
        entry_order.push(entry);
    }
    let mut derived_entries = Vec::new();
    for derivation in &derivations {
        let parent = index_by_identity
            .get(&derivation.parent_identity)
            .map(|&index| &entry_order[index]);
        if parent.is_none() {
            report.findings.push(Finding {
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
            &mut report.findings,
            &mut coverage,
        ) {
            Ok(entry) => derived_entries.push(entry),
            Err(problem) => report.findings.push(Finding {
                severity: "high".into(),
                record: derivation.id.to_string(),
                problem,
            }),
        }
    }
    for entry in &derived_entries {
        *report
            .derived_by_kind
            .entry(entry.kind.slug().into())
            .or_default() += 1;
    }
    report.derived_records = derived_entries.len();
    // PSRD promotes some rule sections and table rows that Pf Data keeps inside
    // another record. The reviewed allowlist says which ones are derivable.
    // Seed the collision set with every entry already built (natives and #47
    // derived children) so a rule cannot silently claim an existing ID.
    let mut seen_ids: BTreeSet<EntryId> = entry_order
        .iter()
        .chain(derived_entries.iter())
        .map(|entry| entry.id.clone())
        .collect();
    let mut rule_entries: Vec<RuleEntry> = Vec::new();
    let mut derived_expected = 0usize;
    let mut derived_sections = 0usize;
    let mut derived_rows = 0usize;
    let mut derived_skipped = 0usize;
    let by_identity: BTreeMap<String, &InputRecord> = converted_inputs
        .iter()
        .map(|r| (format!("{}#{}", r.path, r.key), r))
        .collect();
    for r in inv
        .records
        .iter()
        .filter(in_scope)
        .filter(|r| selected.contains(&(r.path.clone(), r.key.clone())))
    {
        let rules = allowlist.for_host(&r.path, &r.key);
        if rules.is_empty() {
            continue;
        }
        derived_expected += rules.len();
        let identity = format!("{}#{}", r.path, r.key);
        // The P2 rule: a host that failed conversion must not silently drop its
        // allowlisted rules. Report each one and count it as skipped.
        let Some(&expanded) = by_identity.get(&identity) else {
            derived_skipped += rules.len();
            for rule in rules {
                report.findings.push(Finding {
                    severity: "medium".into(),
                    record: identity.clone(),
                    problem: format!(
                        "allowlisted rule {} skipped; host record did not convert",
                        rule.rule
                    ),
                });
            }
            continue;
        };
        let Some(parent) = native_key(expanded).and_then(|key| context.targets.get(&key).cloned())
        else {
            derived_skipped += rules.len();
            for rule in rules {
                report.findings.push(Finding {
                    severity: "medium".into(),
                    record: identity.clone(),
                    problem: format!(
                        "allowlisted rule {} skipped; host record has no resolved parent",
                        rule.rule
                    ),
                });
            }
            continue;
        };
        let lines = description(&expanded.raw)?;
        let extracted = crate::pf_rules::extract(&lines, rules);
        // Report every allowlisted rule that extraction did not produce, so a
        // missing heading or empty body is never a silent drop.
        let extracted_names: BTreeSet<&str> = extracted.iter().map(|e| e.rule.as_str()).collect();
        for rule in rules {
            if !extracted_names.contains(rule.rule.as_str()) {
                derived_skipped += 1;
                report.findings.push(Finding {
                    severity: "medium".into(),
                    record: identity.clone(),
                    problem: format!(
                        "allowlisted rule {} was not extracted from the host body",
                        rule.rule
                    ),
                });
            }
        }
        for extracted in extracted {
            let id = EntryId::parse(extracted.id.clone()).map_err(|e| e.to_string())?;
            if !seen_ids.insert(id.clone()) {
                derived_skipped += 1;
                report.findings.push(Finding {
                    severity: "medium".into(),
                    record: extracted.id.clone(),
                    problem:
                        "duplicate derived id; another rule already claims this host and title"
                            .into(),
                });
                continue;
            }
            match derive_rule_entry(
                expanded,
                &extracted,
                &context,
                &parent,
                &mut report.findings,
                &mut coverage,
            ) {
                Ok(entry) => {
                    match extracted.shape {
                        crate::pf_rules::RuleShape::Section => derived_sections += 1,
                        crate::pf_rules::RuleShape::Row => derived_rows += 1,
                    }
                    rule_entries.push(entry);
                }
                Err(problem) => {
                    derived_skipped += 1;
                    report.findings.push(Finding {
                        severity: "high".into(),
                        record: extracted.rule.clone(),
                        problem: format!("derived rule {}: {problem}", extracted.id),
                    });
                }
            }
        }
    }
    report.derived_expected = derived_expected;
    report.derived_sections = derived_sections;
    report.derived_rows = derived_rows;
    report.derived_rules = derived_sections + derived_rows;
    report.derived_skipped = derived_skipped;
    entries.extend(entry_order);
    entries.extend(derived_entries);
    entries.extend(rule_entries);
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let mut sources = BTreeMap::new();
    for e in &entries {
        for source in &e.source_ids {
            sources.entry(source.clone()).or_insert(SourceInfo {
                id: source.clone(),
                title: source.clone(),
                entry_count: 0,
                copyright: None,
            });
        }
    }
    let catalog = Catalog {
        sources: sources.into_values().collect(),
        entries: entries
            .iter()
            .map(|e| cast_iron_schema::EntryStub {
                id: e.id.clone(),
                kind: e.kind,
                title: e.title.clone(),
                source: e.source_ids[0].clone(),
                parent: None,
                shard: "sample.json".into(),
            })
            .collect(),
    };
    let errors = cast_iron_schema::validate::validate_library(&entries, &catalog);
    report.schema_errors = errors.len();
    for error in errors {
        report.findings.push(Finding {
            severity: "critical".into(),
            record: error.entry,
            problem: error.problem,
        });
    }
    let bytes = serde_json::to_vec_pretty(&entries).map_err(|e| e.to_string())?;
    let output_name = if full { "full.json" } else { "sample.json" };
    write(out.join(output_name).as_path(), &bytes)?;
    let reread: Vec<RuleEntry> = read_json(&out.join(output_name))?;
    if reread != entries {
        return Err("canonical serialization round-trip changed entries".into());
    }
    let mut builder = IndexBuilder::new();
    for e in &reread {
        builder.add(DocInput {
            id: e.id.to_string(),
            title: e.title.clone(),
            kind: e.kind.slug().into(),
            source: e.source_ids[0].clone(),
            text: blocks_text(&e.body),
            ..Default::default()
        });
    }
    let index_bytes = builder.build().to_bytes();
    let search_name = if full {
        "full-search.bin"
    } else {
        "search.bin"
    };
    write(&out.join(search_name), &index_bytes)?;
    let index =
        SearchIndex::from_bytes(&fs::read(out.join(search_name)).map_err(|e| e.to_string())?)
            .map_err(|e| format!("{e:?}"))?;
    for entry in &reread {
        *report.by_kind.entry(entry.kind.slug().into()).or_default() += 1;
        for source in &entry.source_ids {
            *report.by_publication.entry(source.clone()).or_default() += 1;
        }
    }
    for entry in reread
        .iter()
        .enumerate()
        .filter(|(i, e)| !full || *i < 100 || FIXTURES.iter().any(|f| e.title.starts_with(f)))
        .map(|(_, e)| e)
    {
        for query in [
            entry.title.clone(),
            entry.title.chars().take(5).collect(),
            blocks_text(&entry.body)
                .split_whitespace()
                .find(|w| w.len() > 6)
                .unwrap_or(&entry.title)
                .to_string(),
        ] {
            report.search_checks += 1;
            if !index
                .search(&Query {
                    text: query.clone(),
                    limit: 1000,
                    ..Default::default()
                })
                .iter()
                .any(|hit| hit.id == entry.id.as_str())
            {
                report.findings.push(Finding {
                    severity: "high".into(),
                    record: entry.id.to_string(),
                    problem: format!("search did not retrieve record for {query:?}"),
                });
            }
        }
    }
    // Derived records sort after the bounded sample, so verify every one explicitly.
    // Title queries prove discovery by name; a body token is deliberately avoided
    // because common words exceed the result window in a 20,000-entry corpus.
    for entry in reread
        .iter()
        .filter(|entry| entry.id.as_str().starts_with("ref/"))
    {
        let tokens = cast_iron_search::tokenize(&entry.title);
        let longest = tokens
            .iter()
            .max_by_key(|token| token.len())
            .cloned()
            .unwrap_or_else(|| entry.title.clone());
        for query in [entry.title.clone(), longest] {
            if query.is_empty() {
                continue;
            }
            report.search_checks += 1;
            if !index
                .search(&Query {
                    text: query.clone(),
                    limit: 1000,
                    ..Default::default()
                })
                .iter()
                .any(|hit| hit.id == entry.id.as_str())
            {
                report.findings.push(Finding {
                    severity: "high".into(),
                    record: entry.id.to_string(),
                    problem: format!("search did not retrieve derived record for {query:?}"),
                });
            }
        }
    }
    for fixture in FIXTURES {
        let records: Vec<_> = inv
            .records
            .iter()
            .filter(|r| {
                r.shape != "publication"
                    && (r.name == *fixture || r.name.starts_with(&format!("{fixture} (")))
            })
            .collect();
        let entry = reread
            .iter()
            .find(|e| e.title == *fixture || e.title.starts_with(&format!("{fixture} (")));
        let (publication, page, needle, kind) = match *fixture {
            "Mortal Usher" => (
                "Pathfinder #144: Midwives to Death",
                Some("76"),
                "Reaping Strike",
                EntryKind::PrestigeClass,
            ),
            "Golem's Conviction" => (
                "Druma, Profit and Prophecy",
                Some("19"),
                "rudimentary programming",
                EntryKind::Feat,
            ),
            "Cradle of Night" => (
                "Cradle of Night",
                Some("57"),
                "shadowbound",
                EntryKind::Item,
            ),
            "Arcing Weapon" => (
                "Chronicle of Legends",
                Some("8"),
                "spellstrike",
                EntryKind::Feat,
            ),
            "Realm Retribution" => (
                "Chronicle of Legends",
                Some("14"),
                "Outer Plane",
                EntryKind::Spell,
            ),
            "Apocalyptic Spell" => (
                "Planar Adventures",
                Some("22"),
                "spell slot 1 level higher",
                EntryKind::Feat,
            ),
            _ => return Err(format!("fixture expectations missing: {fixture}")),
        };
        let rendered = entry.map(|e| blocks_text(&e.body)).unwrap_or_default();
        let mut checks = BTreeMap::from([
            ("exact_source_record".into(), records.len() == 1),
            (
                "publication".into(),
                records
                    .iter()
                    .any(|r| r.publications.iter().any(|s| s == publication))
                    && entry.is_some_and(|e| e.source_ids.contains(&slug_segment(publication))),
            ),
            (
                "page".into(),
                page.is_none_or(|p| rendered.contains(&format!("pg {p}"))),
            ),
            (
                "body".into(),
                rendered.contains(needle) && rendered.len() > 100,
            ),
            ("kind".into(), entry.is_some_and(|e| e.kind == kind)),
            (
                "formatting".into(),
                entry.is_some_and(|e| {
                    e.body
                        .iter()
                        .any(|b| matches!(b, cast_iron_schema::Block::Heading { .. }))
                }),
            ),
            (
                "references".into(),
                records.iter().all(|r| {
                    !report.findings.iter().any(|f| {
                        f.record == format!("{}#{}", r.path, r.key)
                            && f.problem.contains("reference")
                    })
                }),
            ),
        ]);
        checks.insert(
            "mechanics".into(),
            entry.is_some_and(|e| match &e.fields {
                cast_iron_schema::TypedFields::Spell(s) => {
                    !s.school.is_empty()
                        && !s.levels.is_empty()
                        && s.duration.is_some()
                        && s.saving_throw.is_some()
                }
                cast_iron_schema::TypedFields::Feat(f) => {
                    f.prerequisites.is_some() || f.benefit.is_some()
                }
                cast_iron_schema::TypedFields::Item(i) => {
                    i.aura.is_some() && i.caster_level.is_some()
                }
                cast_iron_schema::TypedFields::Class(c) => c.hit_die.is_some(),
                cast_iron_schema::TypedFields::None => {
                    kind == EntryKind::Archetype && rendered.contains("archetype of")
                }
                _ => false,
            }),
        );
        let passed = checks.values().all(|v| *v);
        report.fixtures.push(FixtureResult {
            name: fixture.to_string(),
            records: records
                .iter()
                .map(|r| format!("{}#{}", r.path, r.key))
                .collect(),
            imported: reread
                .iter()
                .any(|e| e.title == *fixture || e.title.starts_with(&format!("{fixture} ("))),
            publications: records
                .iter()
                .flat_map(|r| r.publications.clone())
                .collect(),
            checks,
            passed,
        });
        if records.is_empty() {
            report.findings.push(Finding {
                severity: "high".into(),
                record: fixture.to_string(),
                problem:
                    "mandatory fixture absent by exact name; manual alias investigation required"
                        .into(),
            });
        }
    }
    report.imported_records = reread.len();
    if report
        .findings
        .iter()
        .any(|f| ["critical", "high"].contains(&f.severity.as_str()))
    {
        report.cutover_blockers.push(
            "Conversion still has critical or high findings; see complete diagnostics".into(),
        );
    }
    if !report.fixtures.iter().all(|f| f.passed) {
        report
            .cutover_blockers
            .push("Mandatory fixture checks have not all passed".into());
    }
    report.publication_ready = report.cutover_blockers.is_empty();
    report.canonical_bytes = bytes.len();
    report.search_bytes = index_bytes.len();
    report.coverage = coverage.into_rows();
    report
        .findings
        .sort_by(|a, b| (&a.record, &a.problem).cmp(&(&b.record, &b.problem)));
    write(
        &reports.join(if full {
            "migration-full.json"
        } else {
            "migration-test.json"
        }),
        &serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )?;
    write(
        &reports.join(if full {
            "migration-full.md"
        } else {
            "migration-test.md"
        }),
        markdown_report(&report).as_bytes(),
    )?;
    Ok(report)
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
        if href.starts_with("source/") && self.context.known.contains(href) {
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

fn convert(
    r: &InputRecord,
    context: &LinkContext,
    findings: &mut Vec<Finding>,
    coverage: &mut CoverageLog,
) -> Result<RuleEntry, String> {
    if r.shape != "description" {
        return Err(format!("{} expansion is not implemented", r.shape));
    }
    // Table references have already been expanded by pf_records. Addenda still
    // require their own ordered assembly and cannot be silently omitted.
    if r.raw.get("addenda").is_some() {
        return Err("addendum expansion required".into());
    }
    let lines = description(&r.raw)?;
    if r.name.is_empty() || r.publications.is_empty() {
        return Err("missing name or publication metadata".into());
    }
    // Prestige-class archetypes share their parent family's upstream file.
    let kind = if r.protocol.as_deref() == Some("pclass")
        && lines
            .iter()
            .any(|l| l.starts_with("**This is an archetype of "))
    {
        EntryKind::Archetype
    } else {
        record_kind(&r.path)
    };
    let fields = pf_fields::extract(kind, &lines)?;
    let id = pf_derived::native_id(r).ok_or("no native protocol mapping")?;
    let mut env = ConvertEnv {
        record: format!("{}#{}", r.path, r.key),
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
        &format!("{}#{}", r.path, r.key),
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
        title: r.name.clone(),
        source_ids: r.publications.iter().map(|s| slug_segment(s)).collect(),
        section: Vec::new(),
        parent: None,
        body,
        fields,
        tags: Vec::new(),
        links: env.references.into_iter().collect(),
        provenance: vec![Provenance {
            dataset: "pathfinder-data-1-e".into(),
            record: r.key.clone(),
            path: r.path.clone(),
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
            .unwrap_or_else(|| vec![crate::build::LICENSE_OGL.into()]),
        relations,
    })
}

/// Build one derived rule entry from a nested section or table row. The body is
/// rendered through the shared typed converter so no content is dropped.
fn derive_rule_entry(
    r: &InputRecord,
    derived: &crate::pf_rules::ExtractedRule,
    context: &LinkContext,
    parent: &EntryId,
    findings: &mut Vec<Finding>,
    coverage: &mut CoverageLog,
) -> Result<RuleEntry, String> {
    let id = EntryId::parse(derived.id.clone()).map_err(|e| e.to_string())?;
    let record = format!(
        "{}#{}|{}:{}",
        r.path,
        r.key,
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
        source_ids: r.publications.iter().map(|s| slug_segment(s)).collect(),
        section: vec![r.name.clone()],
        parent: Some(parent.clone()),
        body,
        fields: cast_iron_schema::TypedFields::None,
        tags: Vec::new(),
        links: env.references.into_iter().collect(),
        provenance: vec![Provenance {
            dataset: "pathfinder-data-1-e".into(),
            record,
            path: r.path.clone(),
        }],
        license_refs: pf_license_refs(),
        relations: Vec::new(),
    })
}

pub fn markdown_report(r: &MigrationReport) -> String {
    let mut text = format!(
        "# Migration conversion report\n\nSource revision `{}`.\n\nObserved {} input records, {} selected, {} converted. Canonical data {} bytes; search {} bytes. {} search checks; {} schema errors.\n\nPublication gate: **blocked**. Browser checks: {}. These are experimental artifacts, not a production library.\n\n",
        r.commit,
        r.input_records,
        r.selected_records,
        r.imported_records,
        r.canonical_bytes,
        r.search_bytes,
        r.search_checks,
        r.schema_errors,
        r.browser_verification
    );
    text.push_str(&format!(
        "Scope: `{}`. Input digest `{}`.\n\n",
        r.scope, r.input_digest
    ));
    text.push_str(&format!(
        "Derived records built in this run: {} ({} racial traits, {} afflictions). Derivations refused: {}.\n\n",
        r.derived_records,
        r.derived_by_kind.get("racial-trait").copied().unwrap_or(0),
        r.derived_by_kind.get("affliction").copied().unwrap_or(0),
        r.derived_rejected
    ));
    text.push_str(&format!(
        "Derived rules: {} converted ({} sections, {} rows; {} expected in this scope, {} skipped). The in-scope corpus exposes {} derivable rules.\n\n",
        r.derived_rules,
        r.derived_sections,
        r.derived_rows,
        r.derived_expected,
        r.derived_skipped,
        r.derived_available
    ));
    for blocker in &r.cutover_blockers {
        text.push_str(&format!("- {blocker}\n"));
    }
    if !r.out_of_scope.is_empty() {
        text.push_str("\nOut-of-scope input, excluded before selection:\n\n");
        for record in &r.out_of_scope {
            text.push_str(&format!(
                "- `{}#{}` ({}): {}\n",
                record.path, record.key, record.name, record.reason
            ));
        }
    }
    text.push_str("\nFixture checks:\n\n");
    for f in &r.fixtures {
        let failed: Vec<_> = f
            .checks
            .iter()
            .filter(|(_, v)| !**v)
            .map(|(k, _)| k.as_str())
            .collect();
        text.push_str(&format!(
            "- {}: {}. Failed checks: {}.\n",
            f.name,
            if f.passed {
                "passed conversion checks"
            } else {
                "not passed"
            },
            if failed.is_empty() {
                "none".into()
            } else {
                failed.join(", ")
            }
        ));
    }
    text.push_str(&format!(
        "\nFindings: {} ({} medium, {} high, {} critical).\n",
        r.findings.len(),
        r.findings.iter().filter(|f| f.severity == "medium").count(),
        r.findings.iter().filter(|f| f.severity == "high").count(),
        r.findings
            .iter()
            .filter(|f| f.severity == "critical")
            .count(),
    ));
    for f in &r.findings {
        text.push_str(&format!(
            "- {}: `{}`: {}\n",
            f.severity,
            f.record,
            f.problem.replace('\n', " ")
        ));
    }
    if !r.coverage.is_empty() {
        let mut by_category: BTreeMap<&str, (&str, usize, usize, BTreeSet<&str>)> = BTreeMap::new();
        for row in &r.coverage {
            let entry =
                by_category
                    .entry(&row.category)
                    .or_insert((&row.reason, 0, 0, BTreeSet::new()));
            entry.1 += row.occurrences;
            entry.2 += 1;
            entry.3.insert(&row.record);
        }
        text.push('\n');
        text.push_str(&format!(
            "Coverage exceptions ({} {}, {} {}). These are explained, not defects:\n\n",
            r.coverage.len(),
            plural(r.coverage.len(), "row", "rows"),
            r.coverage.iter().map(|row| row.occurrences).sum::<usize>(),
            plural(
                r.coverage.iter().map(|row| row.occurrences).sum::<usize>(),
                "occurrence",
                "occurrences"
            )
        ));
        for (category, (reason, occurrences, rows, records)) in &by_category {
            text.push_str(&format!(
                "- `{category}`: {occurrences} {} in {rows} {} across {} {}. {reason}\n",
                plural(*occurrences, "occurrence", "occurrences"),
                plural(*rows, "row", "rows"),
                records.len(),
                plural(records.len(), "record", "records")
            ));
        }
        text.push('\n');
        for row in &r.coverage {
            text.push_str(&format!(
                "- `{}` `{}` ×{}: {}\n",
                row.category, row.detail, row.occurrences, row.record
            ));
        }
    }
    text
}

fn plural<'a>(count: usize, one: &'a str, many: &'a str) -> &'a str {
    if count == 1 { one } else { many }
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}
pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    fs::write(path, bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_include_unknown_directives() {
        assert_eq!(
            directives(":::unknown{a=1} @x[y] ::spell{foo}"),
            ["::unknown", "@x", "::spell"]
        );
    }
    #[test]
    fn pins_reject_noncanonical_hashes() {
        assert!(!is_hash("../x", 4));
        assert!(!is_hash(&"A".repeat(40), 40));
        assert!(is_hash(&"a".repeat(40), 40));
    }

    #[test]
    fn audit_detects_changed_bytes_and_unaccounted_files() {
        let root = std::env::temp_dir().join(format!("cast-iron-pin-test-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("json")).unwrap();
        fs::create_dir_all(root.join("prebuild")).unwrap();
        let mut files = BTreeMap::new();
        for (path, body) in [
            ("json/sources.json", r#"{"core":{"title":"Core"}}"#),
            (
                "json/feat.json",
                r#"{"a":{"name":"A","sources":["Core"],"description":["Benefit."]}}"#,
            ),
            ("README.md", "Source declaration"),
            ("JSON.md", "Format"),
            ("OPENGAMECONTENT.md", "Notice"),
            ("ORCLICENSE.md", "ORC notice"),
            ("GNULICENSE.md", "GNU notice"),
            ("prebuild/basic_data_groups.js", "Routing evidence"),
        ] {
            write(&root.join(path), body.as_bytes()).unwrap();
            files.insert(path, sha256_hex(body.as_bytes()));
        }
        let pin = serde_json::json!({"id":"pathfinder-data-1-e","repository":"https://github.com/jasontankapps/pathfinder-data-1-e","commit":"a".repeat(40),"archive_sha256":"b".repeat(64),"files":files});
        write(
            &root.join("source.json"),
            &serde_json::to_vec(&pin).unwrap(),
        )
        .unwrap();
        write(
            &root.join("routes.json"),
            br#"{"json/sources.json":"source","json/feat.json":"feat"}"#,
        )
        .unwrap();
        write(&root.join("pi-screen.json"), br#"{"sections":[]}"#).unwrap();
        let good = inventory(&root, &root.join("source.json")).unwrap();
        assert_eq!(good.input_records, 2);
        assert_eq!(good.by_shape["description"], 1);
        write(&root.join("json/feat.json"), b"{}").unwrap();
        assert!(
            inventory(&root, &root.join("source.json"))
                .unwrap_err()
                .contains("hash mismatch")
        );
        write(&root.join("json/new.json"), b"{}").unwrap();
        assert!(
            inventory(&root, &root.join("source.json"))
                .unwrap_err()
                .contains("file set")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_identity_survives_file_repartitioning() {
        let original = InputRecord {
            protocol: Some("feat".into()),
            path: "json/feat01.json".into(),
            key: "power_attack".into(),
            name: "Power Attack".into(),
            shape: "description".into(),
            publications: vec!["Core Rulebook".into()],
            raw: serde_json::json!({"description":["## Power Attack","","**Benefit:** Trade accuracy for damage."]}),
        };
        let mut moved = original.clone();
        moved.path = "json/feat99.json".into();
        let a = convert(
            &original,
            &LinkContext::default(),
            &mut Vec::new(),
            &mut CoverageLog::default(),
        )
        .unwrap();
        let b = convert(
            &moved,
            &LinkContext::default(),
            &mut Vec::new(),
            &mut CoverageLog::default(),
        )
        .unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(a.id.as_str(), "pf-data/feat/power-attack");
        assert_ne!(a.provenance, b.provenance);
    }

    #[test]
    fn real_inventory_derives_core_traits_and_inline_afflictions() {
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
        let inventory = inventory(
            &dataset,
            &root.join("sources/pathfinder-data-1-e/source.json"),
        )
        .expect("inventory");
        let coverage = derived_coverage(&inventory);
        assert_eq!(coverage.by_kind.get("racial-trait"), Some(&67));
        assert_eq!(coverage.by_kind.get("affliction"), Some(&155));
        assert_eq!(coverage.total, 222);
        let ids: BTreeSet<_> = coverage.records.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids.len(), coverage.records.len(), "derived ids repeat");
        assert!(ids.contains("ref/racial-trait/dwarf/stonecunning"));
        assert!(ids.contains("ref/racial-trait/human/attributes"));
        assert!(ids.contains("ref/affliction/xacarba/fiendish-bile"));
        assert!(
            coverage
                .rejected
                .iter()
                .all(|rejection| rejection.problem == "::aff block has no name")
        );
    }

    #[test]
    fn monster_core_and_the_legal_notice_are_out_of_scope() {
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
        let inventory = inventory(
            &dataset,
            &root.join("sources/pathfinder-data-1-e/source.json"),
        )
        .expect("inventory");
        assert!(inventory.out_of_scope.iter().any(|record| {
            record.path == "json/monster_families2.json" && record.key == "genie"
        }));
        assert!(inventory.out_of_scope.iter().any(|record| {
            record.path == "json/monster_families2.json" && record.key == "eurypterid"
        }));
        assert!(
            inventory
                .out_of_scope
                .iter()
                .any(|record| record.path == "json/main.json" && record.key == "ogc")
        );
        assert!(inventory.out_of_scope.iter().any(|record| {
            record.path == "json/equipment_weapons2.json" && record.key == "gnome_flick_mace"
        }));
        assert!(inventory.out_of_scope.iter().any(|record| {
            record.path == "json/prestige_classes.json"
                && record.key == "thought_thief_arcane_trickster"
        }));
        // A record that also appears in an in-scope PF1e book stays in scope.
        assert!(!inventory.out_of_scope.iter().any(|record| {
            record.path == "json/rules_basics01.json" && record.key == "rare_races"
        }));
        assert!(
            !inventory
                .out_of_scope
                .iter()
                .any(|record| record.path == "json/rules_basics04.json" && record.key == "magic")
        );
    }

    #[test]
    fn index_publications_come_from_their_subtopics() {
        let records = vec![
            InputRecord {
                protocol: Some("rule".into()),
                path: "json/rules_a.json".into(),
                key: "one".into(),
                name: "One".into(),
                shape: "description".into(),
                publications: vec!["Book A".into()],
                raw: serde_json::json!({}),
            },
            InputRecord {
                protocol: Some("rule".into()),
                path: "json/rules_b.json".into(),
                key: "two".into(),
                name: "Two".into(),
                shape: "description".into(),
                publications: vec!["Book B".into(), "Book A".into()],
                raw: serde_json::json!({}),
            },
        ];
        let raw = serde_json::json!({"subtopics": ["one", "two"]});
        assert_eq!(
            derive_publications(&raw, &records),
            ["Book A".to_string(), "Book B".to_string()]
        );
        assert!(derive_publications(&serde_json::json!({}), &records).is_empty());
    }

    fn sample_record(description: &[&str]) -> InputRecord {
        InputRecord {
            protocol: Some("feat".into()),
            path: "json/feat01.json".into(),
            key: "sample_feat".into(),
            name: "Sample Feat".into(),
            shape: "description".into(),
            publications: vec!["Core Rulebook".into()],
            raw: serde_json::json!({"description": description}),
        }
    }

    fn categories(rows: &[CoverageException]) -> BTreeSet<&str> {
        rows.iter().map(|row| row.category.as_str()).collect()
    }

    #[test]
    fn outside_sample_reference_is_explained_coverage_not_a_medium_finding() {
        let record = sample_record(&["## Sample Feat", "", "See the ‹spell/haste› spell."]);
        let context = LinkContext {
            known: BTreeSet::from(["spell/haste".to_string()]),
            ..Default::default()
        };
        let mut findings = Vec::new();
        let mut coverage = CoverageLog::default();
        let entry = convert(&record, &context, &mut findings, &mut coverage).unwrap();
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
        let rows = coverage.into_rows();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].category, "reference-outside-sample");
        assert_eq!(rows[0].detail, "spell/haste");
        assert_eq!(rows[0].record, "json/feat01.json#sample_feat");
        assert_eq!(rows[0].occurrences, 1);
        assert!(rows[0].reason.contains("bounded selection"));
        // The visible text remains even though the target is out of sample.
        assert!(blocks_text(&entry.body).contains("haste"));
    }

    #[test]
    fn presentation_constructs_are_explained_coverage_with_content_kept() {
        let record = sample_record(&[
            "## Sample Feat",
            "",
            "::h3[Traits]{jl}",
            "",
            ":::aside",
            "",
            "Aside prose stays.",
            "",
            ":::",
            "",
            "<hr>",
            "",
            "::classoptions{cls=fighter}",
            "",
            "An @I-boost icon and text.",
        ]);
        let mut findings = Vec::new();
        let mut coverage = CoverageLog::default();
        let entry = convert(
            &record,
            &LinkContext::default(),
            &mut findings,
            &mut coverage,
        )
        .unwrap();
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
        let rows = coverage.into_rows();
        assert_eq!(
            categories(&rows),
            BTreeSet::from([
                "external-class-options",
                "jump-list-metadata",
                "layout-container",
                "layout-separator",
                "presentation-icon",
            ]),
            "{rows:?}"
        );
        let text = blocks_text(&entry.body);
        assert!(text.contains("Aside prose stays."), "{text}");
        assert!(text.contains("favored class options"), "{text}");
    }

    #[test]
    fn unknown_html_construct_stays_a_medium_finding() {
        let record = sample_record(&["## Sample Feat", "", "<span>kept text</span>."]);
        let mut findings = Vec::new();
        let mut coverage = CoverageLog::default();
        let entry = convert(
            &record,
            &LinkContext::default(),
            &mut findings,
            &mut coverage,
        )
        .unwrap();
        assert!(coverage.into_rows().is_empty());
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, "medium");
        assert!(findings[0].problem.contains("unwrapped_tag:span"));
        // Unwrapping an unknown inline tag must not lose its text.
        assert!(blocks_text(&entry.body).contains("kept text"));
    }

    #[test]
    fn coverage_rows_are_deterministic_and_merge_duplicates() {
        let record = sample_record(&[
            "## Sample Feat",
            "",
            "One ‹spell/haste› and two ‹spell/haste›.",
        ]);
        let context = LinkContext {
            known: BTreeSet::from(["spell/haste".to_string()]),
            ..Default::default()
        };
        let mut coverage = CoverageLog::default();
        convert(&record, &context, &mut Vec::new(), &mut coverage).unwrap();
        let rows = coverage.into_rows();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].occurrences, 2);
    }
}
