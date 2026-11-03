use cast_iron_content_compiler::pipeline::{Options, compile};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: cast-iron-compile --source-config <source.toml> --dataset <dir> --out <dir> --reports <dir> [--write-digest] [--strict] [--id-overrides <file>]";

fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut source_config = None;
    let mut dataset = None;
    let mut out = None;
    let mut reports = None;
    let mut overrides = None;
    let mut write_digest = false;
    let mut strict = false;
    let mut args = args;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--source-config" => source_config = Some(PathBuf::from(value("--source-config")?)),
            "--dataset" => dataset = Some(PathBuf::from(value("--dataset")?)),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--reports" => reports = Some(PathBuf::from(value("--reports")?)),
            "--id-overrides" => overrides = Some(PathBuf::from(value("--id-overrides")?)),
            "--write-digest" => write_digest = true,
            "--strict" => strict = true,
            "--help" | "-h" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let need = |v: Option<PathBuf>, n: &str| v.ok_or_else(|| format!("missing {n}\n{USAGE}"));
    Ok(Options {
        source_config: need(source_config, "--source-config")?,
        dataset: need(dataset, "--dataset")?,
        out: need(out, "--out")?,
        reports: need(reports, "--reports")?,
        write_digest,
        strict,
        overrides,
    })
}

fn main() -> ExitCode {
    let opts = match parse_args(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    // Rules trees nest up to 15 JSON levels. A large stack keeps deeper input from overflowing.
    let worker = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || compile(&opts));
    let result = match worker.map(|h| h.join()) {
        Ok(Ok(result)) => result,
        _ => {
            eprintln!("the compiler thread failed");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(o) => {
            println!("content version: {}", o.content_version);
            println!("entries: {}", o.entry_count);
            for (kind, n) in &o.by_kind {
                println!("  {kind}: {n}");
            }
            println!("total bytes: {}", o.total_bytes);
            println!(
                "largest shard: {} ({} bytes)",
                o.largest_shard.0, o.largest_shard.1
            );
            println!(
                "largest entry: {} ({} bytes)",
                o.largest_entry.0, o.largest_entry.1
            );
            println!(
                "links: resolved {}, out of selection {}, broken {}",
                o.links.resolved, o.links.out_of_selection, o.links.broken
            );
            println!("rejected records: {}", o.rejected.len());
            for r in &o.rejected {
                println!("  {}: {}", r.path, r.reason);
            }
            println!("duplicates: {:?}", o.dedupe);
            println!("notes: {}", o.notes);
            println!("stripped:");
            for (k, n) in &o.stripped {
                println!("  {k}: {n}");
            }
            ExitCode::SUCCESS
        }
        Err(failure) => {
            for line in failure.lines {
                eprintln!("{line}");
            }
            ExitCode::FAILURE
        }
    }
}
