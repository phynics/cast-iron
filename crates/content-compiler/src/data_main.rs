//! Isolated migration commands. A blocked gate exits nonzero after writing evidence.
use cast_iron_content_compiler::alias::{self, IdMap, LegacySnapshot};
use cast_iron_content_compiler::alias_propose;
use cast_iron_content_compiler::carry;
use cast_iron_content_compiler::pf_data::{
    derived_coverage, experiment, import_all, inventory, write,
};
use cast_iron_content_compiler::pf_legal;
use cast_iron_content_compiler::pf_rules::RuleAllowlist;
use cast_iron_content_compiler::pipeline::{self, publish_extra_file};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected audit, import, validate, report, legal, test, or alias")?;
    let mut dataset = None;
    let mut registry = None;
    let mut out = None;
    let mut reports = None;
    let mut map = None;
    let mut legacy = None;
    let mut content = None;
    let mut psrd_dataset = None;
    let mut psrd_config = None;
    let mut apply = false;
    while let Some(flag) = args.next() {
        if flag == "--apply" {
            apply = true;
            continue;
        }
        let value = PathBuf::from(args.next().ok_or_else(|| format!("{flag} needs a value"))?);
        match flag.as_str() {
            "--dataset" => dataset = Some(value),
            "--registry" => registry = Some(value),
            "--out" => out = Some(value),
            "--reports" => reports = Some(value),
            "--map" => map = Some(value),
            "--legacy" => legacy = Some(value),
            "--content" => content = Some(value),
            "--psrd-dataset" => psrd_dataset = Some(value),
            "--psrd-config" => psrd_config = Some(value),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    let psrd_dataset = psrd_dataset.unwrap_or_else(|| PathBuf::from(".cache/sources/psrd-data"));
    let psrd_config = psrd_config.unwrap_or_else(|| PathBuf::from("sources/psrd-data/source.toml"));
    if ![
        "audit",
        "import",
        "validate",
        "report",
        "legal",
        "test",
        "alias",
        "alias-propose",
    ]
    .contains(&command.as_str())
    {
        return Err(format!("unknown data command {command}"));
    }
    let dataset = dataset.ok_or("missing --dataset")?;
    let registry = registry.ok_or("missing --registry")?;
    let out = out.ok_or("missing --out")?;
    let reports = reports.ok_or("missing --reports")?;
    let inv = inventory(&dataset, &registry)?;
    let derived = derived_coverage(&inv);
    let allowlist = RuleAllowlist::load_optional(
        &registry
            .parent()
            .ok_or("registry has no parent")?
            .join("derived-rules.json"),
    )?;
    if command == "alias-propose" {
        return write_proposals(
            &inv,
            &allowlist,
            map.as_deref()
                .unwrap_or(std::path::Path::new("sources/canonical/id-map.toml")),
            legacy
                .as_deref()
                .unwrap_or(std::path::Path::new("sources/canonical/legacy-ids.json")),
            &out,
            apply,
        );
    }
    if command == "alias" {
        return write_aliases(
            &inv,
            &allowlist,
            map.as_deref()
                .unwrap_or(std::path::Path::new("sources/canonical/id-map.toml")),
            legacy
                .as_deref()
                .unwrap_or(std::path::Path::new("sources/canonical/legacy-ids.json")),
            content
                .as_deref()
                .unwrap_or(std::path::Path::new("generated/content")),
            &psrd_dataset,
            &psrd_config,
        );
    }
    let config_dir = registry.parent().ok_or("registry has no parent")?;
    let legal = pf_legal::build(
        &inv,
        &dataset.join("OPENGAMECONTENT.md"),
        &config_dir.join("notice-aliases.json"),
        &config_dir.join("out-of-scope.json"),
    )?;
    write(
        &out.join("inventory.json"),
        &serde_json::to_vec_pretty(&inv).map_err(|e| e.to_string())?,
    )?;
    let mut summary = serde_json::json!({
        "dataset":inv.dataset,"commit":inv.commit,"input_digest":inv.input_digest,
        "json_files":inv.json_files,"input_records":inv.input_records,
        "registered_publications":inv.registered_publications,"by_file":inv.by_file,
        "by_shape":inv.by_shape,"by_publication":inv.by_publication,"directives":inv.directives,
        "unregistered_publications":inv.unregistered_publications,
        "out_of_scope_records":inv.out_of_scope.len(),
        "derived":derived,
        "pi_screen":pi_screen_summary(&inv),
        "legal":legal.summary(),
        "production_default":"psrd-data","cutover_verified":false
    });
    let coverage_path = reports.join("migration-coverage.json");
    if command == "report" {
        let map_path = map
            .as_deref()
            .unwrap_or(Path::new("sources/canonical/id-map.toml"));
        let carried = carry_report(&inv, &allowlist, map_path, &psrd_dataset, &psrd_config)?;
        summary["carry_over"] = serde_json::to_value(&carried).map_err(|e| e.to_string())?;
    }
    // Preserve an existing `derived_rules` block so `data-audit` does not strip
    // the committed counts when it rewrites the coverage summary. The `carry_over`
    // block is only recomputed by `report`, so other commands preserve it too.
    if let Ok(bytes) = std::fs::read(&coverage_path)
        && let Ok(existing) = serde_json::from_slice::<serde_json::Value>(&bytes)
    {
        if existing.get("derived_rules").is_some() {
            summary["derived_rules"] = existing["derived_rules"].clone();
        }
        if command != "report"
            && let Some(carry_over) = existing.get("carry_over")
        {
            summary["carry_over"] = carry_over.clone();
        }
    }
    write(
        &coverage_path,
        &serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
    )?;
    if matches!(command.as_str(), "report" | "legal") {
        pf_legal::write_report(&reports.join("migration-legal.json"), &legal)?;
    }
    println!(
        "{} JSON files, {} input records, {} registered publications",
        inv.json_files, inv.input_records, inv.registered_publications
    );
    println!(
        "{} legal publications, {} matched, {} unmatched",
        legal.publication_count, legal.matched_publication_count, legal.unmatched_publication_count
    );
    println!(
        "{} derived records ({} racial traits, {} afflictions), {} rejected",
        derived.total,
        derived.by_kind.get("racial-trait").copied().unwrap_or(0),
        derived.by_kind.get("affliction").copied().unwrap_or(0),
        derived.rejected.len()
    );
    if matches!(command.as_str(), "audit" | "legal") {
        return Ok(());
    }
    let result = if ["import", "validate"].contains(&command.as_str()) {
        import_all(&inv, &out, &reports, &allowlist)?
    } else {
        experiment(&inv, &out, &reports, &allowlist)?
    };
    // Record the rule-derivation counts in the tracked coverage summary.
    if let Ok(bytes) = std::fs::read(&coverage_path)
        && let Ok(mut existing) = serde_json::from_slice::<serde_json::Value>(&bytes)
    {
        existing["derived_rules"] = serde_json::json!({
            "scope": result.scope,
            "allowlist": allowlist.len(),
            "available": result.derived_available,
            "expected": result.derived_expected,
            "derived": result.derived_rules,
            "sections": result.derived_sections,
            "rows": result.derived_rows,
            "skipped": result.derived_skipped,
        });
        write(
            &coverage_path,
            &serde_json::to_vec_pretty(&existing).map_err(|e| e.to_string())?,
        )?;
    }
    println!(
        "{} selected, {} converted, {} findings, {} search checks, {} derived rules ({} sections, {} rows)",
        result.selected_records,
        result.imported_records,
        result.findings.len(),
        result.search_checks,
        result.derived_rules,
        result.derived_sections,
        result.derived_rows
    );
    if ["validate", "test", "import"].contains(&command.as_str()) && !result.publication_ready {
        let report = if ["import", "validate"].contains(&command.as_str()) {
            "reports/migration-full.md"
        } else {
            "reports/migration-test.md"
        };
        return Err(format!(
            "migration acceptance blocked; see {report}. Production content is unchanged"
        ));
    }
    Ok(())
}

/// Generates `generated/content/<version>/aliases.json` and runs every alias gate.
fn write_aliases(
    inv: &cast_iron_content_compiler::pf_data::Inventory,
    allowlist: &RuleAllowlist,
    map_path: &std::path::Path,
    legacy_path: &std::path::Path,
    content_dir: &std::path::Path,
    psrd_dataset: &Path,
    psrd_config: &Path,
) -> Result<(), String> {
    let snapshot_bytes = std::fs::read(legacy_path)
        .map_err(|e| format!("cannot read {}: {e}", legacy_path.display()))?;
    let snapshot = LegacySnapshot::parse(&snapshot_bytes)
        .map_err(|e| format!("invalid {}: {e}", legacy_path.display()))?;
    let map = IdMap::load(map_path)?;
    let catalog = alias::catalog_from_inventory(inv, allowlist)?;
    let library = psrd_library(psrd_dataset, psrd_config)?;
    let carried = carry::carry_over(&library, &map, &catalog)
        .map_err(|errors| problems("carry-over", errors))?;
    let mut catalog = catalog;
    carried
        .extend_catalog(&mut catalog)
        .map_err(|errors| problems("carried catalog", errors))?;
    let (file, stats) = alias::generate(&map, &snapshot, &snapshot_bytes, &catalog)
        .map_err(|errors| problems("alias gate(s)", errors))?;
    let bytes = alias::to_bytes(&file)?;
    let version = publish_extra_file(content_dir, alias::ALIASES_FILE, bytes).map_err(|e| {
        format!("{e}; run `cargo xtask content` first, then `cargo xtask data-alias`")
    })?;
    println!(
        "{} carried entries ({} possible duplicates, {} links rewritten, {} flattened)",
        carried.report.carried,
        carried.report.possible_duplicate_count,
        carried.report.links.rewritten,
        carried.report.links.flattened
    );
    println!("{} canonical records; {}", catalog.len(), stats.report());
    println!(
        "wrote {}",
        content_dir
            .join(version)
            .join(alias::ALIASES_FILE)
            .display()
    );
    Ok(())
}

/// Builds the PSRD library in memory. Carry-over and the alias gate are the only
/// consumers; the production package remains untouched.
fn psrd_library(dataset: &Path, config: &Path) -> Result<pipeline::Library, String> {
    let opts = pipeline::Options {
        source_config: config.to_path_buf(),
        dataset: dataset.to_path_buf(),
        out: PathBuf::from("generated/carry-over"),
        reports: PathBuf::from("generated/carry-over"),
        write_digest: false,
        strict: false,
        overrides: None,
    };
    pipeline::build_psrd(&opts).map_err(|failure| {
        let mut message = format!("PSRD build failed: {} problem(s)", failure.lines.len());
        for line in failure.lines {
            message.push_str("\n  ");
            message.push_str(&line);
        }
        message
    })
}

fn carry_report(
    inv: &cast_iron_content_compiler::pf_data::Inventory,
    allowlist: &RuleAllowlist,
    map_path: &Path,
    psrd_dataset: &Path,
    psrd_config: &Path,
) -> Result<carry::CarryReport, String> {
    let map = IdMap::load(map_path)?;
    let catalog = alias::catalog_from_inventory(inv, allowlist)?;
    let library = psrd_library(psrd_dataset, psrd_config)?;
    let carried = carry::carry_over(&library, &map, &catalog)
        .map_err(|errors| problems("carry-over", errors))?;
    Ok(carried.report)
}

fn problems(context: &str, errors: Vec<String>) -> String {
    let mut message = format!("{context} failed: {} problem(s):", errors.len());
    for error in errors {
        message.push_str("\n  ");
        message.push_str(&error);
    }
    message
}

/// Counts of the PI screen's omissions over the whole in-scope inventory.
fn pi_screen_summary(inv: &cast_iron_content_compiler::pf_data::Inventory) -> serde_json::Value {
    let mut by_category = std::collections::BTreeMap::<&str, usize>::new();
    let mut records = std::collections::BTreeSet::new();
    for omission in &inv.pi_omissions {
        *by_category.entry(omission.category.as_str()).or_default() += 1;
        records.insert(omission.record.as_str());
    }
    serde_json::json!({
        "omissions": inv.pi_omissions.len(),
        "records": records.len(),
        "by_category": by_category,
    })
}

/// Proposes dispositions for every `no_source` row and optionally applies them.
///
/// The command is deterministic and idempotent. It writes the proposed map and a
/// JSON report under `--out`, then rewrites the committed map only with `--apply`.
fn write_proposals(
    inv: &cast_iron_content_compiler::pf_data::Inventory,
    allowlist: &RuleAllowlist,
    map_path: &std::path::Path,
    legacy_path: &std::path::Path,
    output_dir: &std::path::Path,
    apply: bool,
) -> Result<(), String> {
    let snapshot_bytes = std::fs::read(legacy_path)
        .map_err(|e| format!("cannot read {}: {e}", legacy_path.display()))?;
    let snapshot = LegacySnapshot::parse(&snapshot_bytes)
        .map_err(|e| format!("invalid {}: {e}", legacy_path.display()))?;
    let original = std::fs::read_to_string(map_path)
        .map_err(|e| format!("cannot read {}: {e}", map_path.display()))?;
    let map =
        IdMap::parse(&original).map_err(|e| format!("invalid {}: {e}", map_path.display()))?;
    let catalog = alias::catalog_from_inventory(inv, allowlist)?;
    let report = alias_propose::propose(&map, &snapshot, &catalog)?;
    alias_propose::validate_targets(&report)?;
    alias_propose::write(&report, &original, map_path, output_dir, apply)?;
    println!("{}", report.report());
    println!(
        "wrote {} and {}",
        output_dir.join("id-map.proposed.toml").display(),
        output_dir.join("alias-proposals.json").display()
    );
    if apply {
        println!("applied proposals to {}", map_path.display());
    } else {
        println!("dry run; pass --apply to rewrite {}", map_path.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
