//! Archives of Nethys path coverage: the canonical entries of the pinned corpus without a
//! verified AoN path, by kind.
//!
//! The corpus stores an `aon` path only when a curation-time harvest verified it against an AoN
//! bulk listing (cast-iron-data `scripts/aon-harvest.py`). An entry without the field is the only
//! fact this report records: the corpus keeps no negative disposition, so absence does not say
//! whether the harvest found no page, found several pages, or never covered that kind. The report
//! names no cause per row, because the corpus holds no evidence for one.
//!
//! `cargo xtask aon-coverage` reads `.cache/sources/cast-iron-data` only and writes
//! `docs/research/aon-path-coverage.md` (provenance, per-kind totals, the publication pages) and
//! `docs/research/aon-path-coverage.tsv` (every entry without `aon`, sorted by kind then id).
//! `--check` writes nothing and fails when the committed files differ from the pinned corpus.
//! Nothing here fetches a page: the report is reproducible from the corpus pin alone.

use crate::{CorpusPin, Result, corpus_cache, hex, read_corpus_pin, root, verify_corpus_cache};
use cast_iron_corpus::Corpus;
use cast_iron_schema::EntryKind;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const REPORT: &str = "docs/research/aon-path-coverage.md";
const LIST: &str = "docs/research/aon-path-coverage.tsv";
const LIST_HEADER: &str = "id\tkind\ttitle";

/// The failure `cargo xtask check` reports for a stale report: it names the fix, because a
/// contributor meets this step without having asked for the report.
const STALE: &str = "AoN coverage report is stale; run `cargo xtask aon-coverage` and commit \
                     docs/research/aon-path-coverage.{md,tsv}";

/// One entry as the report sees it. `has_path` means the corpus file carries an `aon` field.
#[derive(Debug, Clone)]
struct EntryFact {
    id: String,
    kind: String,
    title: String,
    has_path: bool,
}

/// One publication and whether it carries an AoN source page.
#[derive(Debug, Clone)]
struct PublicationFact {
    id: String,
    title: String,
    has_path: bool,
}

/// The facts a render needs, separated from the corpus reader so the render can be tested.
struct Source {
    repository: String,
    tag: String,
    commit: String,
    tree: String,
    entries: Vec<EntryFact>,
    publications: Vec<PublicationFact>,
}

/// The two files and the counts the command prints.
#[derive(Debug)]
struct Output {
    markdown: String,
    list: String,
    entries: u32,
    matched: u32,
    missing: u32,
    publications: u32,
    publications_missing: u32,
}

/// `cargo xtask aon-coverage [--check]`.
pub fn run(args: &[String]) -> Result {
    let mut check = false;
    for argument in args {
        match argument.as_str() {
            "--check" => check = true,
            other => {
                return Err(format!(
                    "unknown argument {other:?} for aon-coverage\nusage: aon-coverage [--check]"
                ));
            }
        }
    }
    compare(check, false)
}

/// The `cargo xtask check` step: the same comparison, with the failure a contributor can act on.
/// The caller decides whether the corpus cache is present.
pub fn check() -> Result {
    compare(true, true)
}

/// Renders the report from the pinned corpus and either writes it or compares it.
fn compare(check: bool, for_check: bool) -> Result {
    verify_corpus_cache()?;
    let pin = read_corpus_pin()?;
    let corpus = Corpus::load(&corpus_cache()).map_err(|errors| {
        let mut message = String::from("the pinned corpus did not load:");
        for error in errors {
            let _ = write!(message, "\n  {error}");
        }
        message
    })?;
    let output = render(&source(&corpus, &pin))?;
    write(&root().join(REPORT), &output.markdown, check, for_check)?;
    write(&root().join(LIST), &output.list, check, for_check)?;
    println!(
        "{} entries: {} with an AoN path, {} without; {} publications: {} with a source page, {} without",
        output.entries,
        output.matched,
        output.missing,
        output.publications,
        output.publications - output.publications_missing,
        output.publications_missing,
    );
    if check {
        println!("{REPORT} and {LIST} match the pinned corpus");
    }
    Ok(())
}

/// The render input, read from the loaded corpus and the pin.
fn source(corpus: &Corpus, pin: &CorpusPin) -> Source {
    Source {
        repository: pin.repository.clone(),
        tag: pin.tag.clone().unwrap_or_default(),
        commit: pin.commit.clone(),
        tree: pin.tree_sha256.clone(),
        entries: corpus
            .entries
            .values()
            .map(|stored| EntryFact {
                id: stored.entry.id.as_str().to_string(),
                kind: stored.entry.kind.slug().to_string(),
                title: stored.entry.title.clone(),
                has_path: stored.entry.aon.is_some(),
            })
            .collect(),
        publications: corpus
            .publications
            .iter()
            .map(|publication| PublicationFact {
                id: publication.id.clone(),
                title: publication.title.clone(),
                has_path: publication.aon.is_some(),
            })
            .collect(),
    }
}

/// Renders both files and checks them against the facts before anything is written.
fn render(source: &Source) -> Result<Output> {
    let mut entries = source.entries.clone();
    entries.sort_by(|left, right| {
        (left.kind.as_str(), left.id.as_str()).cmp(&(right.kind.as_str(), right.id.as_str()))
    });
    let mut ids = BTreeSet::new();
    for entry in &entries {
        if EntryKind::from_slug(&entry.kind).is_none() {
            return Err(format!(
                "{}: `kind` {:?} is not an entry kind",
                entry.id, entry.kind
            ));
        }
        if !ids.insert(entry.id.as_str()) {
            return Err(format!("{}: the id occurs twice", entry.id));
        }
    }

    // Per kind: matched, missing. The set of kinds is the corpus's, not a hardcoded list.
    let mut tally: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
    for entry in &entries {
        let counts = tally.entry(entry.kind.as_str()).or_default();
        if entry.has_path {
            counts.0 += 1;
        } else {
            counts.1 += 1;
        }
    }
    let total: u32 = u32::try_from(entries.len()).map_err(|_| "too many entries")?;
    let matched: u32 = tally.values().map(|counts| counts.0).sum();
    let missing: u32 = tally.values().map(|counts| counts.1).sum();
    if matched + missing != total {
        return Err(format!(
            "totals disagree: {matched} matched + {missing} missing != {total} entries"
        ));
    }

    let list = render_list(&entries)?;
    let listed = parse_list(&list)?;
    let expected: BTreeSet<&str> = entries
        .iter()
        .filter(|entry| !entry.has_path)
        .map(|entry| entry.id.as_str())
        .collect();
    if listed.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err("the list does not hold exactly the entries without a path".into());
    }
    if listed.len() != missing as usize {
        return Err(format!(
            "the list holds {} rows, but {missing} entries have no path",
            listed.len()
        ));
    }
    let mut listed_by_kind: BTreeMap<&str, u32> = BTreeMap::new();
    for (kind, _) in listed.values() {
        *listed_by_kind.entry(kind).or_default() += 1;
    }
    for (kind, (_, expected_missing)) in &tally {
        let got = listed_by_kind.get(kind).copied().unwrap_or(0);
        if got != *expected_missing {
            return Err(format!(
                "kind `{kind}`: the list holds {got} rows, but the tally says {expected_missing}"
            ));
        }
    }

    let mut publications = source.publications.clone();
    publications.sort_by(|left, right| left.id.cmp(&right.id));
    let mut publication_ids = BTreeSet::new();
    for publication in &publications {
        if !publication_ids.insert(publication.id.as_str()) {
            return Err(format!(
                "publication {}: the id occurs twice",
                publication.id
            ));
        }
    }
    let publication_total: u32 =
        u32::try_from(publications.len()).map_err(|_| "too many publications")?;
    let publication_matched = publications.iter().filter(|row| row.has_path).count();
    let publication_missing: u32 = u32::try_from(publications.len() - publication_matched)
        .map_err(|_| "too many publications")?;
    if publication_matched + publication_missing as usize != publications.len() {
        return Err("publication totals disagree".into());
    }

    let markdown = render_markdown(
        source,
        &tally,
        total,
        matched,
        missing,
        &publications,
        &list,
    );
    Ok(Output {
        markdown,
        list,
        entries: total,
        matched,
        missing,
        publications: publication_total,
        publications_missing: publication_missing,
    })
}

/// The TSV: one row per entry without a path, sorted by kind then id, plus its header.
fn render_list(entries: &[EntryFact]) -> Result<String> {
    let mut out = String::from(LIST_HEADER);
    out.push('\n');
    for entry in entries.iter().filter(|entry| !entry.has_path) {
        out.push_str(&entry.id);
        out.push('\t');
        out.push_str(&entry.kind);
        out.push('\t');
        let title = one_line(&entry.title);
        if title.is_empty() {
            return Err(format!("{}: the title is empty", entry.id));
        }
        out.push_str(&title);
        out.push('\n');
    }
    Ok(out)
}

/// Reads the rendered TSV back: id -> (kind, title). Fails on a duplicate or a malformed row.
fn parse_list(list: &str) -> Result<BTreeMap<String, (String, String)>> {
    let mut rows = BTreeMap::new();
    let mut lines = list.lines();
    if lines.next() != Some(LIST_HEADER) {
        return Err("the list does not start with its header".into());
    }
    for line in lines {
        let mut fields = line.split('\t');
        let (Some(id), Some(kind), Some(title), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(format!("the list row {line:?} does not hold three fields"));
        };
        if rows
            .insert(id.to_string(), (kind.to_string(), title.to_string()))
            .is_some()
        {
            return Err(format!("the list holds {id} twice"));
        }
    }
    Ok(rows)
}

/// The report. No timestamp: the corpus pin and the file paths are the provenance.
fn render_markdown(
    source: &Source,
    tally: &BTreeMap<&str, (u32, u32)>,
    total: u32,
    matched: u32,
    missing: u32,
    publications: &[PublicationFact],
    list: &str,
) -> String {
    let publication_missing: Vec<&PublicationFact> =
        publications.iter().filter(|row| !row.has_path).collect();
    let mut out = String::new();
    out.push_str("# Archives of Nethys path coverage\n\n");
    out.push_str(
        "Every canonical entry of the pinned corpus that has no verified Archives of Nethys path, \
         with per-kind totals. This report is generated from the corpus alone and fetches no page.\n\n",
    );

    out.push_str("## Pinned corpus\n\n| | |\n|---|---|\n");
    let label = source
        .repository
        .strip_prefix("https://github.com/")
        .unwrap_or(&source.repository);
    let _ = writeln!(
        out,
        "| Corpus | [`{label}`]({}/tree/{}) |",
        source.repository, source.commit
    );
    let _ = writeln!(out, "| Tag | `{}` |", source.tag);
    let _ = writeln!(out, "| Commit | `{}` |", source.commit);
    let _ = writeln!(out, "| Tree hash | `{}` |", source.tree);
    let _ = writeln!(out, "| Entries | {} |", thousands(total));
    let _ = writeln!(
        out,
        "| Publications | {} |",
        thousands(publications.len() as u32)
    );

    out.push_str("\n## Totals by kind\n\n");
    out.push_str(
        "`Matched` counts entries whose corpus file stores an `aon` path; `Missing` counts entries \
         without one. `corpus-check` accepts only a relative `*Display.aspx` path on `aonprd.com`, \
         so a stored path is the verified path the app renders.\n\n",
    );
    out.push_str("| Kind | Matched | Missing | Total |\n|---|---:|---:|---:|\n");
    for (kind, (kind_matched, kind_missing)) in tally {
        let _ = writeln!(
            out,
            "| {kind} | {} | {} | {} |",
            thousands(*kind_matched),
            thousands(*kind_missing),
            thousands(kind_matched + kind_missing)
        );
    }
    let _ = writeln!(
        out,
        "| **Total** | **{}** | **{}** | **{}** |",
        thousands(matched),
        thousands(missing),
        thousands(total)
    );

    out.push_str("\n## Publications\n\n");
    out.push_str("| | Count |\n|---|---:|\n");
    let _ = writeln!(
        out,
        "| With a source page | {} |",
        thousands(publications.len() as u32 - publication_missing.len() as u32)
    );
    let _ = writeln!(
        out,
        "| Without a source page | {} |",
        thousands(publication_missing.len() as u32)
    );
    let _ = writeln!(out, "| Total | {} |", thousands(publications.len() as u32));
    if !publication_missing.is_empty() {
        out.push_str("\n### Publications without a source page\n\n| Id | Title |\n|---|---|\n");
        for publication in &publication_missing {
            let _ = writeln!(
                out,
                "| `{}` | {} |",
                publication.id,
                escape_cell(&publication.title)
            );
        }
    }

    out.push_str("\n## The per-entry list\n\n");
    let digest = hex(&Sha256::digest(list.as_bytes()));
    let _ = writeln!(
        out,
        "[`aon-path-coverage.tsv`](aon-path-coverage.tsv) holds one row per missing entry, sorted \
         by kind then id: `id`, `kind`, and `title`, with a header row. {} data rows.",
        thousands(missing)
    );
    let _ = writeln!(out, "\nSHA-256 of the list: `{digest}`\n");

    out.push_str("## What a missing row means\n\n");
    out.push_str(
        "A missing row is a canonical entry whose corpus file has no `aon` field. That is the \
         whole fact the corpus records. The corpus stores only paths that a curation-time harvest \
         verified against an AoN bulk listing; it stores no negative disposition. A missing row \
         therefore does not say whether the harvest found no page (unmatched), found several pages \
         (ambiguous), or never covered the kind. The report does not attach those causes to rows.\n\n",
    );
    out.push_str(
        "The harvest's own report is [cast-iron-data #67](https://github.com/phynics/cast-iron-data/pull/67). \
         It separates matched, ambiguous and unmatched per kind, and it is the evidence for the \
         cause at kind level. It ran on an earlier corpus revision, so its totals are not repeated \
         here: every entry that the harvest left without a link and that is still in the corpus is \
         in this list, and the per-kind counts above are the current corpus's. In that harvest, \
         rules, afflictions, traps, skills and racial traits had no AoN bulk display-page listing, \
         and item listings were category indexes, so those kinds stayed without a link rather than \
         being guessed.\n\n",
    );
    out.push_str(
        "Paths are not guessed. No row here is a candidate, a near match or a base page; the app \
         renders a link only when the corpus stores an `aon` path.\n\n",
    );

    out.push_str("## Rerun\n\n```sh\n");
    out.push_str(
        "cargo xtask fetch-sources          # download and tree-hash-check the pinned corpus\n",
    );
    out.push_str("cargo xtask aon-coverage           # rewrite this report and the list\n");
    out.push_str("cargo xtask aon-coverage --check   # fail when the committed files differ\n");
    out.push_str("```\n\n");
    out.push_str(
        "The command reads `.cache/sources/cast-iron-data` only, sorts by kind then id, and writes \
         no timestamp, so two runs on one corpus pin produce byte-identical files. Before writing, \
         it checks that every entry appears once, that each kind's matched plus missing equals its \
         total, that the list's ids are exactly the ids of the entries without a path, and that the \
         list's row count and per-kind counts agree with the tables. Regenerate this report in the \
         pull request that changes the corpus pin.\n",
    );
    out
}

/// Marks a corpus number. The corpus is always far below 2^53, so the split is exact.
fn thousands(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A title on one line: a tab or line break would break the TSV and the Markdown table.
fn one_line(title: &str) -> String {
    title
        .chars()
        .map(|ch| match ch {
            '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

/// A title with the Markdown table's cell separator escaped.
fn escape_cell(title: &str) -> String {
    one_line(title).replace('|', "\\|")
}

/// Writes a file, or with `check` fails when the committed file differs. `for_check` reports the
/// stale report the way `cargo xtask check` must, and treats a missing file the same way.
fn write(path: &Path, content: &str, check: bool, for_check: bool) -> Result {
    if check {
        let current = fs::read_to_string(path);
        if current.as_deref().ok() == Some(content) {
            return Ok(());
        }
        if for_check {
            return Err(STALE.into());
        }
        return Err(match current {
            Ok(_) => format!(
                "{} is stale: the pinned corpus produces different content; run `cargo xtask aon-coverage`",
                path.display()
            ),
            Err(e) => format!("{}: {e}; run `cargo xtask aon-coverage`", path.display()),
        });
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, content).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, kind: &str, title: &str, has_path: bool) -> EntryFact {
        EntryFact {
            id: id.into(),
            kind: kind.into(),
            title: title.into(),
            has_path,
        }
    }

    fn publication(id: &str, title: &str, has_path: bool) -> PublicationFact {
        PublicationFact {
            id: id.into(),
            title: title.into(),
            has_path,
        }
    }

    fn source() -> Source {
        Source {
            repository: "https://github.com/phynics/cast-iron-data".into(),
            tag: "v0.1.26".into(),
            commit: "a4b3ef6".into(),
            tree: "55180e4".into(),
            entries: vec![
                entry("ref/feat/zeta", "feat", "Zeta", false),
                entry("ref/feat/alpha", "feat", "Alpha", true),
                entry("ref/spell/beta", "spell", "Beta", false),
                entry("ref/trait/gamma", "trait", "Gamma", false),
            ],
            publications: vec![
                publication("core-rulebook", "Core Rulebook", true),
                publication("odd-book", "Odd | Book", false),
            ],
        }
    }

    #[test]
    fn the_list_holds_only_entries_without_a_path_in_kind_then_id_order() {
        let output = render(&source()).unwrap();
        assert_eq!(
            output.list,
            "id\tkind\ttitle\nref/feat/zeta\tfeat\tZeta\nref/spell/beta\tspell\tBeta\nref/trait/gamma\ttrait\tGamma\n"
        );
        assert_eq!((output.entries, output.matched, output.missing), (4, 1, 3));
    }

    #[test]
    fn the_report_counts_every_kind_and_names_the_missing_publication() {
        let output = render(&source()).unwrap();
        assert!(output.markdown.contains("| feat | 1 | 1 | 2 |"));
        assert!(output.markdown.contains("| spell | 0 | 1 | 1 |"));
        assert!(output.markdown.contains("| trait | 0 | 1 | 1 |"));
        assert!(
            output
                .markdown
                .contains("| **Total** | **1** | **3** | **4** |")
        );
        assert!(output.markdown.contains("| `odd-book` | Odd \\| Book |"));
        assert!(output.markdown.contains("3 data rows"));
        assert!(
            output
                .markdown
                .contains(&hex(&Sha256::digest(output.list.as_bytes())))
        );
    }

    #[test]
    fn two_renders_of_one_corpus_are_byte_identical() {
        let first = render(&source()).unwrap();
        let second = render(&source()).unwrap();
        assert_eq!(first.list, second.list);
        assert_eq!(first.markdown, second.markdown);
    }

    #[test]
    fn the_render_fails_when_an_id_occurs_twice() {
        let mut source = source();
        source
            .entries
            .push(entry("ref/feat/zeta", "feat", "Zeta again", false));
        assert_eq!(
            render(&source).unwrap_err(),
            "ref/feat/zeta: the id occurs twice"
        );
    }

    #[test]
    fn the_render_fails_on_an_unknown_kind() {
        let mut source = source();
        source
            .entries
            .push(entry("ref/thing/x", "thing", "X", false));
        assert!(render(&source).unwrap_err().contains("not an entry kind"));
    }

    #[test]
    fn the_render_fails_on_an_empty_title() {
        let mut source = source();
        source
            .entries
            .push(entry("ref/feat/empty", "feat", "", false));
        assert!(render(&source).unwrap_err().contains("the title is empty"));
    }

    #[test]
    fn a_title_is_flattened_to_one_line() {
        assert_eq!(one_line("A\tB\nC"), "A B C");
        assert_eq!(escape_cell("A | B"), "A \\| B");
    }

    #[test]
    fn thousands_groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(31_800), "31,800");
        assert_eq!(thousands(12_467), "12,467");
    }

    #[test]
    fn the_check_step_passes_identical_content_and_names_the_stale_report() {
        let path = std::env::temp_dir().join(format!("aon-coverage-{}.md", std::process::id()));
        let _ = fs::remove_file(&path);
        assert_eq!(write(&path, "same\n", true, true).unwrap_err(), STALE);
        fs::write(&path, "same\n").unwrap();
        assert_eq!(write(&path, "same\n", true, true), Ok(()));
        assert_eq!(write(&path, "other\n", true, true).unwrap_err(), STALE);
        assert_eq!(
            write(&path, "other\n", true, false).unwrap_err(),
            format!(
                "{} is stale: the pinned corpus produces different content; run `cargo xtask aon-coverage`",
                path.display()
            )
        );
        fs::remove_file(&path).unwrap();
    }
}
