use cast_iron_content_compiler::corpus_build::compile_corpus;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: cast-iron-compile --corpus <dir> --out <dir> --reports <dir> [--published <file>] [--corpus-tag <tag>]";

struct Args {
    corpus: PathBuf,
    out: PathBuf,
    reports: PathBuf,
    published: Option<PathBuf>,
    corpus_tag: Option<String>,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut corpus = None;
    let mut out = None;
    let mut reports = None;
    let mut published = None;
    let mut corpus_tag = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--corpus" => corpus = Some(PathBuf::from(value("--corpus")?)),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--reports" => reports = Some(PathBuf::from(value("--reports")?)),
            "--published" => published = Some(PathBuf::from(value("--published")?)),
            "--corpus-tag" => corpus_tag = Some(value("--corpus-tag")?),
            "--help" | "-h" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let need = |v: Option<PathBuf>, n: &str| v.ok_or_else(|| format!("missing {n}\n{USAGE}"));
    Ok(Args {
        corpus: need(corpus, "--corpus")?,
        out: need(out, "--out")?,
        reports: need(reports, "--reports")?,
        published,
        corpus_tag: corpus_tag.filter(|tag| !tag.is_empty()),
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
    // A large stack keeps deeply nested entry bodies from overflowing it.
    let worker = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || {
            compile_corpus(
                &args.corpus,
                &args.out,
                &args.reports,
                args.published.as_deref(),
                args.corpus_tag.as_deref(),
            )
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
            println!("notes: {}", o.notes);
            println!("stripped:");
            for (k, n) in &o.stripped {
                println!("  {k}: {n}");
            }
            if o.reachability.unreachable > 0 {
                eprintln!(
                    "warning: {} of {} entries are unreachable without the catch-all chapter `{}`; see reachability.json",
                    o.reachability.unreachable,
                    o.reachability.entries,
                    o.reachability.catch_all_title
                );
                for book in &o.reachability.by_book {
                    if book.unreachable > 0 {
                        eprintln!(
                            "  {}: {} unreachable ({} rules) of {} catch-all entries",
                            book.source_id,
                            book.unreachable,
                            book.unreachable_rules,
                            book.catch_all_entries
                        );
                    }
                }
                if o.reachability.unplaced > 0 {
                    eprintln!(
                        "  {} unreachable entries are not placed in any book tree",
                        o.reachability.unplaced
                    );
                }
            }
            for problem in &o.reachability.search.problems {
                eprintln!(
                    "warning: search index: {} (`{}`): {}",
                    problem.id, problem.title, problem.reason
                );
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
