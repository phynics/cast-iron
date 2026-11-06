use cast_iron_content_compiler::corpus_build::compile_corpus;
use cast_iron_content_compiler::pipeline::{CanonicalOptions, Options, compile, compile_canonical};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: cast-iron-compile --corpus <dir> --out <dir> --reports <dir>\n   or: cast-iron-compile --source-config <source.toml> --dataset <dir> --out <dir> --reports <dir> [--write-digest] [--strict] [--id-overrides <file>] [--canonical --pf-dataset <dir> --pf-registry <source.json> --canonical-dir <dir>]";

struct Args {
    options: Options,
    canonical: Option<CanonicalOptions>,
    /// Compile from the plain-text corpus at this directory instead of an upstream dataset.
    corpus: Option<PathBuf>,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut source_config = None;
    let mut dataset = None;
    let mut out = None;
    let mut reports = None;
    let mut overrides = None;
    let mut pf_dataset = None;
    let mut pf_registry = None;
    let mut canonical_dir = None;
    let mut canonical = false;
    let mut corpus = None;
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
            "--pf-dataset" => pf_dataset = Some(PathBuf::from(value("--pf-dataset")?)),
            "--pf-registry" => pf_registry = Some(PathBuf::from(value("--pf-registry")?)),
            "--canonical-dir" => canonical_dir = Some(PathBuf::from(value("--canonical-dir")?)),
            "--corpus" => corpus = Some(PathBuf::from(value("--corpus")?)),
            "--canonical" => canonical = true,
            "--write-digest" => write_digest = true,
            "--strict" => strict = true,
            "--help" | "-h" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let need = |v: Option<PathBuf>, n: &str| v.ok_or_else(|| format!("missing {n}\n{USAGE}"));
    if corpus.is_some() {
        if source_config.is_some()
            || dataset.is_some()
            || canonical
            || pf_dataset.is_some()
            || pf_registry.is_some()
            || canonical_dir.is_some()
            || overrides.is_some()
            || write_digest
            || strict
        {
            return Err(format!(
                "--corpus reads no upstream dataset and takes only --out and --reports\n{USAGE}"
            ));
        }
        let options = Options {
            source_config: PathBuf::new(),
            dataset: PathBuf::new(),
            out: need(out, "--out")?,
            reports: need(reports, "--reports")?,
            write_digest: false,
            strict: false,
            overrides: None,
        };
        return Ok(Args {
            options,
            canonical: None,
            corpus,
        });
    }
    let options = Options {
        source_config: need(source_config, "--source-config")?,
        dataset: need(dataset, "--dataset")?,
        out: need(out, "--out")?,
        reports: need(reports, "--reports")?,
        write_digest,
        strict,
        overrides,
    };
    let canonical = if canonical {
        Some(CanonicalOptions {
            psrd: options.clone(),
            pf_dataset: need(pf_dataset, "--pf-dataset")?,
            pf_registry: need(pf_registry, "--pf-registry")?,
            canonical_dir: need(canonical_dir, "--canonical-dir")?,
        })
    } else {
        if pf_dataset.is_some() || pf_registry.is_some() || canonical_dir.is_some() {
            return Err(format!(
                "--pf-dataset, --pf-registry, and --canonical-dir require --canonical\n{USAGE}"
            ));
        }
        None
    };
    Ok(Args {
        options,
        canonical,
        corpus,
    })
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let Args {
        options,
        canonical,
        corpus,
    } = args;
    // Rules trees nest up to 15 JSON levels. A large stack keeps deeper input from overflowing.
    let worker = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || match (corpus, canonical) {
            (Some(corpus), _) => compile_corpus(&corpus, &options.out, &options.reports),
            (None, Some(canonical)) => compile_canonical(&canonical),
            (None, None) => compile(&options),
        });
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
            println!("relations: {:?}", o.relations.edges);
            println!(
                "feat prerequisites: {} feats with text, {} with a resolved feat; readings {:?}",
                o.relations.prerequisites.feats_with_text,
                o.relations.prerequisites.feats_with_resolved_feat,
                o.relations.prerequisites.by_reading
            );
            println!(
                "spell lists: {} classes, {} unmapped",
                o.relations.spell_lists.classes,
                o.relations.spell_lists.unmapped.len()
            );
            println!(
                "book order: {} placed, {} unplaced; unplaced parts {}; unpromoted {}",
                o.relations.book_order.placed,
                o.relations.book_order.unplaced.len(),
                o.relations.unplaced_parts.len(),
                o.relations.unpromoted.len()
            );
            println!(
                "backlinks: {} rows, {} targets, {} self excluded, {} relation excluded, {} bytes",
                o.backlinks.links,
                o.backlinks.targets,
                o.backlinks.excluded_self,
                o.backlinks.excluded_relation,
                o.backlinks.bytes
            );
            for (name, bytes) in &o.derived_bytes {
                println!("{name}: {bytes} bytes");
            }
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
