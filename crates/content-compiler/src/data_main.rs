//! Isolated migration commands. A blocked gate exits nonzero after writing evidence.
use cast_iron_content_compiler::pf_data::{experiment, import_all, inventory, write};
use std::path::PathBuf;
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected audit, import, validate, report, or test")?;
    let mut dataset = None;
    let mut registry = None;
    let mut out = None;
    let mut reports = None;
    while let Some(flag) = args.next() {
        let value = PathBuf::from(args.next().ok_or_else(|| format!("{flag} needs a value"))?);
        match flag.as_str() {
            "--dataset" => dataset = Some(value),
            "--registry" => registry = Some(value),
            "--out" => out = Some(value),
            "--reports" => reports = Some(value),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if !["audit", "import", "validate", "report", "test"].contains(&command.as_str()) {
        return Err(format!("unknown data command {command}"));
    }
    let dataset = dataset.ok_or("missing --dataset")?;
    let registry = registry.ok_or("missing --registry")?;
    let out = out.ok_or("missing --out")?;
    let reports = reports.ok_or("missing --reports")?;
    let inv = inventory(&dataset, &registry)?;
    write(
        &out.join("inventory.json"),
        &serde_json::to_vec_pretty(&inv).map_err(|e| e.to_string())?,
    )?;
    let summary = serde_json::json!({
        "dataset":inv.dataset,"commit":inv.commit,"input_digest":inv.input_digest,
        "json_files":inv.json_files,"input_records":inv.input_records,
        "registered_publications":inv.registered_publications,"by_file":inv.by_file,
        "by_shape":inv.by_shape,"by_publication":inv.by_publication,"directives":inv.directives,
        "unregistered_publications":inv.unregistered_publications,
        "out_of_scope_records":inv.out_of_scope.len(),
        "production_default":"psrd-data","cutover_verified":false
    });
    write(
        &reports.join("migration-coverage.json"),
        &serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
    )?;
    println!(
        "{} JSON files, {} input records, {} registered publications",
        inv.json_files, inv.input_records, inv.registered_publications
    );
    if command == "audit" {
        return Ok(());
    }
    let result = if ["import", "validate"].contains(&command.as_str()) {
        import_all(&inv, &out, &reports)?
    } else {
        experiment(&inv, &out, &reports)?
    };
    println!(
        "{} selected, {} converted, {} findings, {} search checks",
        result.selected_records,
        result.imported_records,
        result.findings.len(),
        result.search_checks
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
