//! Isolated migration commands. A blocked gate exits nonzero after writing evidence.
use cast_iron_content_compiler::alias::{self, IdMap, LegacySnapshot};
use cast_iron_content_compiler::pf_data::{
    derived_coverage, experiment, import_all, inventory, write,
};
use cast_iron_content_compiler::pf_rules::RuleAllowlist;
use std::path::PathBuf;
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected audit, import, validate, report, test, or alias")?;
    let mut dataset = None;
    let mut registry = None;
    let mut out = None;
    let mut reports = None;
    let mut map = None;
    let mut legacy = None;
    let mut content = None;
    while let Some(flag) = args.next() {
        let value = PathBuf::from(args.next().ok_or_else(|| format!("{flag} needs a value"))?);
        match flag.as_str() {
            "--dataset" => dataset = Some(value),
            "--registry" => registry = Some(value),
            "--out" => out = Some(value),
            "--reports" => reports = Some(value),
            "--map" => map = Some(value),
            "--legacy" => legacy = Some(value),
            "--content" => content = Some(value),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if !["audit", "import", "validate", "report", "test", "alias"].contains(&command.as_str()) {
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
        );
    }
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
        "production_default":"psrd-data","cutover_verified":false
    });
    // Preserve an existing `derived_rules` block so `data-audit` does not strip
    // the committed counts when it rewrites the coverage summary.
    let coverage_path = reports.join("migration-coverage.json");
    if let Ok(bytes) = std::fs::read(&coverage_path)
        && let Ok(existing) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && existing.get("derived_rules").is_some()
    {
        summary["derived_rules"] = existing["derived_rules"].clone();
    }
    write(
        &coverage_path,
        &serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
    )?;
    println!(
        "{} JSON files, {} input records, {} registered publications",
        inv.json_files, inv.input_records, inv.registered_publications
    );
    println!(
        "{} derived records ({} racial traits, {} afflictions), {} rejected",
        derived.total,
        derived.by_kind.get("racial-trait").copied().unwrap_or(0),
        derived.by_kind.get("affliction").copied().unwrap_or(0),
        derived.rejected.len()
    );
    if command == "audit" {
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
) -> Result<(), String> {
    let snapshot_bytes = std::fs::read(legacy_path)
        .map_err(|e| format!("cannot read {}: {e}", legacy_path.display()))?;
    let snapshot = LegacySnapshot::parse(&snapshot_bytes)
        .map_err(|e| format!("invalid {}: {e}", legacy_path.display()))?;
    let map = IdMap::load(map_path)?;
    let catalog = alias::catalog_from_inventory(inv, allowlist)?;
    let (file, stats) =
        alias::generate(&map, &snapshot, &snapshot_bytes, &catalog).map_err(|errors| {
            let mut message = format!("{} alias gate(s) failed:", errors.len());
            for error in errors {
                message.push_str("\n  ");
                message.push_str(&error);
            }
            message
        })?;
    let manifest_bytes = std::fs::read(content_dir.join("manifest.json")).map_err(|e| {
        format!(
            "cannot read {}: {e}; run `cargo xtask content` first",
            content_dir.join("manifest.json").display()
        )
    })?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).map_err(|e| format!("invalid manifest: {e}"))?;
    let version = manifest
        .get("content_version")
        .and_then(serde_json::Value::as_str)
        .ok_or("manifest.json has no content_version")?;
    let target = content_dir.join(version).join(alias::ALIASES_FILE);
    write(&target, &alias::to_bytes(&file)?)?;
    println!("{} canonical records; {}", catalog.len(), stats.report());
    println!("wrote {}", target.display());
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
