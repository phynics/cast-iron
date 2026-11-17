//! The command line behind `cast-iron-corpus` and `cargo xtask corpus-check` /
//! `corpus-fmt`. Each command takes the arguments after its name, prints its result,
//! and returns an error message for a non-zero exit.

use crate::{Corpus, check, format_corpus, migrate, migrate_trades};
use std::fs;
use std::path::PathBuf;

pub const USAGE: &str = "\
cast-iron-corpus <command>

  check [DIR] [--drift-report FILE]
        Read every file under DIR (default: the current directory) and check ids,
        paths, sources, licences, links, and heading anchors. Fields that disagree
        with the body fail the check; --drift-report writes their full list.
  fmt [--check] [DIR]
        Rewrite every file into its one canonical form. With --check, change nothing:
        print the first differing line of each file that is not canonical and fail.
  migrate [DIR]
        Convert a corpus from an older format to this tool's format.
  migrate-trades [DIR] [--review FILE]
        Lower the trades of archetypes and class options to format 2, resolving each to a
        feature of a base class only where the printed name proves it, and write the trades
        that stay unresolved to FILE as TSV (default trade-review.tsv; not inside DIR).
        Idempotent.
";

struct Args {
    dir: PathBuf,
    flags: Vec<String>,
    values: Vec<(String, String)>,
}

/// Splits `[DIR] [--flag] [--name VALUE]`. `value_flags` take a following value.
fn parse(
    command: &str,
    args: &[String],
    flags: &[&str],
    value_flags: &[&str],
) -> Result<Args, String> {
    let mut dir = None;
    let mut found_flags = Vec::new();
    let mut values = Vec::new();
    let mut rest = args.iter();
    while let Some(argument) = rest.next() {
        if flags.contains(&argument.as_str()) {
            found_flags.push(argument.clone());
        } else if value_flags.contains(&argument.as_str()) {
            let value = rest
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))?;
            values.push((argument.clone(), value.clone()));
        } else if argument.starts_with('-') {
            return Err(format!(
                "unknown argument {argument:?} for {command}\n{USAGE}"
            ));
        } else if dir.replace(PathBuf::from(argument)).is_some() {
            return Err(format!("{command} takes one directory"));
        }
    }
    Ok(Args {
        dir: dir.unwrap_or_else(|| PathBuf::from(".")),
        flags: found_flags,
        values,
    })
}

fn load(dir: &std::path::Path) -> Result<Corpus, String> {
    Corpus::load(dir).map_err(|errors| {
        for error in &errors {
            eprintln!("{error}");
        }
        format!("could not read the corpus: {} error(s)", errors.len())
    })
}

pub fn run_check(args: &[String]) -> Result<(), String> {
    let args = parse("check", args, &[], &["--drift-report"])?;
    let corpus = load(&args.dir)?;
    let report = check(&corpus);
    if let Some((_, path)) = args
        .values
        .iter()
        .find(|(name, _)| name == "--drift-report")
    {
        fs::write(path, report.drift_report()).map_err(|e| format!("cannot write {path}: {e}"))?;
    }
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    if !report.problems.is_empty() {
        for problem in &report.problems {
            eprintln!("{problem}");
        }
        if !report.drift.is_empty() {
            let kinds: Vec<String> = report
                .drift_by_kind()
                .iter()
                .map(|(kind, count)| format!("{kind} {count}"))
                .collect();
            eprintln!(
                "drift: {} disagreement(s) ({})",
                report.drift.len(),
                kinds.join(", ")
            );
        }
        return Err(format!(
            "check failed with {} problem(s)",
            report.problems.len()
        ));
    }
    println!(
        "corpus ok: {} entries, {} publications, {} books, {} licences",
        corpus.entries.len(),
        corpus.publications.len(),
        corpus.books.books.len(),
        corpus.legal.licenses.len()
    );
    Ok(())
}

pub fn run_fmt(args: &[String]) -> Result<(), String> {
    let args = parse("fmt", args, &["--check"], &[])?;
    let check_only = args.flags.iter().any(|flag| flag == "--check");
    let report = format_corpus(&args.dir, !check_only).map_err(|errors| {
        for error in &errors {
            eprintln!("{error}");
        }
        format!("fmt could not read the corpus: {} error(s)", errors.len())
    })?;
    if check_only && !report.changes.is_empty() {
        for change in report.changes.iter().take(20) {
            eprintln!("{}", change.describe());
        }
        if report.changes.len() > 20 {
            eprintln!("... and {} more", report.changes.len() - 20);
        }
        return Err(format!(
            "{} of {} files are not formatted; run `cast-iron-corpus fmt`",
            report.changes.len(),
            report.files
        ));
    }
    println!(
        "fmt: {} files, {} {}",
        report.files,
        report.changes.len(),
        if check_only {
            "unformatted"
        } else {
            "rewritten"
        }
    );
    Ok(())
}

pub fn run_migrate(args: &[String]) -> Result<(), String> {
    let args = parse("migrate", args, &[], &[])?;
    println!("{}", migrate(&args.dir)?);
    Ok(())
}

pub fn run_migrate_trades(args: &[String]) -> Result<(), String> {
    let args = parse("migrate-trades", args, &[], &["--review"])?;
    let review = args
        .values
        .iter()
        .find(|(name, _)| name == "--review")
        .map_or_else(
            || PathBuf::from("trade-review.tsv"),
            |(_, path)| PathBuf::from(path),
        );
    // The queue is not a corpus file: a file inside the corpus would fail the next check.
    let corpus_dir = fs::canonicalize(&args.dir)
        .map_err(|e| format!("cannot read {}: {e}", args.dir.display()))?;
    let review_dir = review
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    fs::create_dir_all(review_dir)
        .map_err(|e| format!("cannot create {}: {e}", review_dir.display()))?;
    let review_dir = fs::canonicalize(review_dir)
        .map_err(|e| format!("cannot read {}: {e}", review_dir.display()))?;
    if review_dir.starts_with(&corpus_dir) {
        return Err(format!(
            "the review file {} is inside the corpus; give --review a path outside {}",
            review.display(),
            args.dir.display()
        ));
    }
    let report = migrate_trades(&args.dir)?;
    fs::write(&review, report.review_tsv())
        .map_err(|e| format!("cannot write {}: {e}", review.display()))?;
    print!("{}", report.summary());
    println!(
        "migrated {} to format {}; review queue: {}; now run `cast-iron-corpus check` and `cast-iron-corpus fmt --check`",
        args.dir.display(),
        crate::CORPUS_FORMAT,
        review.display()
    );
    Ok(())
}

/// Runs `cast-iron-corpus <command> ...`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("check") => run_check(&args[1..]),
        Some("fmt") => run_fmt(&args[1..]),
        Some("migrate") => run_migrate(&args[1..]),
        Some("migrate-trades") => run_migrate_trades(&args[1..]),
        Some("help" | "--help" | "-h") | None => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command {other:?}\n{USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_split_into_a_directory_flags_and_values() {
        let args = parse(
            "x",
            &[
                "data".into(),
                "--check".into(),
                "--drift-report".into(),
                "f.md".into(),
            ],
            &["--check"],
            &["--drift-report"],
        )
        .unwrap();
        assert_eq!(args.dir, PathBuf::from("data"));
        assert_eq!(args.flags, ["--check"]);
        assert_eq!(
            args.values,
            [("--drift-report".to_string(), "f.md".to_string())]
        );
        let default = parse("x", &[], &[], &[]).unwrap();
        assert_eq!(default.dir, PathBuf::from("."));
    }

    #[test]
    fn migrate_trades_refuses_a_review_file_inside_the_corpus() {
        let dir = std::env::temp_dir().join(format!("cast-iron-cli-review-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let review = dir.join("review.tsv");
        let error = run_migrate_trades(&[
            dir.display().to_string(),
            "--review".into(),
            review.display().to_string(),
        ])
        .unwrap_err();
        assert!(error.contains("inside the corpus"), "{error}");
        assert!(!review.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_arguments_and_commands_are_errors_that_show_the_usage() {
        assert!(parse("x", &["--nope".into()], &[], &[]).is_err());
        assert!(parse("x", &["a".into(), "b".into()], &[], &[]).is_err());
        assert!(parse("x", &["--drift-report".into()], &[], &["--drift-report"]).is_err());
        let error = run(&["frobnicate".into()]).unwrap_err();
        assert!(error.contains("cast-iron-corpus <command>"), "{error}");
        assert!(run(&["help".into()]).is_ok());
    }
}
