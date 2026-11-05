//! Isolated migration commands. A blocked gate exits nonzero after writing evidence.
use cast_iron_content_compiler::alias::{self, IdMap, LegacySnapshot};
use cast_iron_content_compiler::alias_propose;
use cast_iron_content_compiler::carry;
use cast_iron_content_compiler::carry_review;
use cast_iron_content_compiler::pf_data::{
    derived_coverage, experiment, import_all, inventory, write,
};
use cast_iron_content_compiler::pf_legal;
use cast_iron_content_compiler::pf_rules::RuleAllowlist;
use cast_iron_content_compiler::pipeline::{self, CanonicalOptions, Options, publish_extra_file};
use cast_iron_content_compiler::publications;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected audit, import, validate, report, legal, test, alias, or publications")?;
    let mut dataset = None;
    let mut registry = None;
    let mut out = None;
    let mut reports = None;
    let mut map = None;
    let mut legacy = None;
    let mut content = None;
    let mut psrd_dataset = None;
    let mut psrd_config = None;
    let mut publications_path = None;
    let mut canonical_dir = None;
    let mut threshold = None;
    let mut apply = false;
    let mut check = false;
    while let Some(flag) = args.next() {
        if flag == "--apply" {
            apply = true;
            continue;
        }
        if flag == "--check" {
            check = true;
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
            "--publications" => publications_path = Some(value),
            "--canonical-dir" => canonical_dir = Some(value),
            "--threshold" => {
                let value = value.to_string_lossy();
                threshold = Some(
                    value
                        .parse::<f64>()
                        .map_err(|e| format!("--threshold {value:?}: {e}"))?,
                );
            }
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
        "carry-review",
        "publications",
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
    if command == "publications" {
        return write_publications(
            &inv,
            &psrd_config,
            publications_path
                .as_deref()
                .unwrap_or(std::path::Path::new(publications::REGISTRY_PATH)),
            check,
        );
    }
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
    if command == "carry-review" {
        return write_carry_review(
            &inv,
            &allowlist,
            CarryReviewArgs {
                dataset: &dataset,
                registry: &registry,
                map_path: map
                    .as_deref()
                    .unwrap_or(std::path::Path::new("sources/canonical/id-map.toml")),
                legacy_path: legacy
                    .as_deref()
                    .unwrap_or(std::path::Path::new("sources/canonical/legacy-ids.json")),
                output_dir: &out,
                canonical_dir: canonical_dir
                    .as_deref()
                    .unwrap_or(std::path::Path::new("sources/canonical")),
                psrd_dataset: &psrd_dataset,
                psrd_config: &psrd_config,
                threshold: threshold.unwrap_or(carry_review::SIMILARITY_THRESHOLD),
                apply,
            },
        );
    }
    let config_dir = registry.parent().ok_or("registry has no parent")?;
    let psrd_notices = psrd_config
        .parent()
        .ok_or("the PSRD source config has no parent directory")?
        .join("legal-notices.json");
    let section15 = config_dir.join("section15");
    let legal = pf_legal::build(
        &inv,
        &pf_legal::Inputs {
            opengamecontent: &dataset.join("OPENGAMECONTENT.md"),
            psrd_notices: Some(&psrd_notices),
            aliases: &config_dir.join("notice-aliases.json"),
            out_of_scope: &config_dir.join("out-of-scope.json"),
            d20pfsrd_csv: &section15.join("d20pfsrd-section15.csv"),
            d20pfsrd_provenance: &section15.join("d20pfsrd-section15.provenance.json"),
            owner_copies: &section15.join("owner-copies.json"),
            owner_provenance: &section15.join("owner-copies.provenance.json"),
            notice_pending: &section15.join("notice-pending.json"),
        },
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

/// Generates the reviewed Pf Data publication registry, or checks the committed
/// copy when `--check` is set. Output is deterministic, so a check compares bytes.
fn write_publications(
    inv: &cast_iron_content_compiler::pf_data::Inventory,
    psrd_config: &Path,
    path: &Path,
    check: bool,
) -> Result<(), String> {
    let registry = publications::generate(inv, psrd_config)
        .map_err(|errors| problems("publication registry", errors))?;
    let bytes = registry.to_bytes()?;
    if check {
        let committed =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if committed != bytes {
            return Err(format!(
                "{} differs from the generated registry; run `cargo xtask data-publications`",
                path.display()
            ));
        }
        println!(
            "{} is current: {} publication(s), {} canonical source id(s)",
            path.display(),
            registry.publication.len(),
            registry.source_ids().len()
        );
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, &bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!(
        "wrote {}: {} publication(s), {} canonical source id(s)",
        path.display(),
        registry.publication.len(),
        registry.source_ids().len()
    );
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

/// Inputs for [`write_carry_review`].
struct CarryReviewArgs<'a> {
    dataset: &'a Path,
    registry: &'a Path,
    map_path: &'a Path,
    legacy_path: &'a Path,
    output_dir: &'a Path,
    canonical_dir: &'a Path,
    psrd_dataset: &'a Path,
    psrd_config: &'a Path,
    threshold: f64,
    apply: bool,
}

/// Reviews carried entries against same-kind, same-title canonical entries and
/// optionally merges the pairs whose bodies agree. Writes the JSON report and a
/// proposed map under `--out`; `--apply` rewrites the committed map.
fn write_carry_review(
    inv: &cast_iron_content_compiler::pf_data::Inventory,
    allowlist: &RuleAllowlist,
    args: CarryReviewArgs<'_>,
) -> Result<(), String> {
    let CarryReviewArgs {
        dataset,
        registry,
        map_path,
        legacy_path,
        output_dir,
        canonical_dir,
        psrd_dataset,
        psrd_config,
        threshold,
        apply,
    } = args;
    let options = CanonicalOptions {
        psrd: Options {
            source_config: psrd_config.to_path_buf(),
            dataset: psrd_dataset.to_path_buf(),
            out: PathBuf::from("generated/carry-review"),
            reports: PathBuf::from("generated/carry-review"),
            write_digest: false,
            strict: false,
            overrides: None,
        },
        pf_dataset: dataset.to_path_buf(),
        pf_registry: registry.to_path_buf(),
        canonical_dir: canonical_dir.to_path_buf(),
    };
    let library = pipeline::build_canonical(&options).map_err(|failure| {
        let mut message = format!("canonical build failed: {} problem(s)", failure.lines.len());
        for line in failure.lines {
            message.push_str("\n  ");
            message.push_str(&line);
        }
        message
    })?;
    let snapshot_bytes = std::fs::read(legacy_path)
        .map_err(|e| format!("cannot read {}: {e}", legacy_path.display()))?;
    let snapshot = LegacySnapshot::parse(&snapshot_bytes)
        .map_err(|e| format!("invalid {}: {e}", legacy_path.display()))?;
    let original = std::fs::read_to_string(map_path)
        .map_err(|e| format!("cannot read {}: {e}", map_path.display()))?;
    let map =
        IdMap::parse(&original).map_err(|e| format!("invalid {}: {e}", map_path.display()))?;
    let catalog = alias::catalog_from_inventory(inv, allowlist)?;
    let review = carry_review::review(&library, &map, &snapshot, &catalog, threshold)?;
    carry_review::write(&review, &original, map_path, output_dir, apply)?;
    println!("{}", review.report());
    println!(
        "wrote {} and {}",
        output_dir.join("id-map.reviewed.toml").display(),
        output_dir.join("carry-review.json").display()
    );
    if apply {
        println!("applied body-evidence merges to {}", map_path.display());
    } else {
        println!("dry run; pass --apply to rewrite {}", map_path.display());
    }
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
