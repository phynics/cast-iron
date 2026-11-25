//! The corpus-check rule for book memberships without a citation row (#461).
//!
//! An entry's first book is its home and needs no row. Every later book in `source_ids` is a
//! claim that the book prints the entry or a section of it, and the structured citation (a
//! `[[sources]]` row, with or without a page) is the evidence that the corpus can check. A body
//! `**Sources:**` line or an `[[expanded_in]]` credit does not count: the first is free text, the
//! second sits outside membership.
//!
//! A membership without a row is an **error**. It was a warning while the cleanup of #461 ran;
//! the owner ruled on 2026-11-25 that the rule becomes an error once the pin carried the
//! cleanup (decision `2026-11-25-uncited-membership-is-an-error`). One membership may stay for a
//! named reason: the corpus root's `membership-exceptions.toml` lists it (see
//! `docs/corpus-format.md`, "Membership exceptions"). The list only shrinks: a row whose entry,
//! book or gap no longer exists is an error too (`remove the row`).

use crate::check::{Report, line_of};
use crate::load::Corpus;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

/// The optional file in the corpus root that lists the memberships allowed to have no row.
pub const EXCEPTIONS_FILE: &str = "membership-exceptions.toml";

/// `membership-exceptions.toml`: one `[[exception]]` row per membership.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionFile {
    #[serde(default)]
    exception: Vec<toml::Spanned<MembershipException>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MembershipException {
    /// The entry id (`ref/rule/gamemastery`).
    entry: String,
    /// The publication id that the entry lists in `source_ids`.
    book: String,
    /// Why the book stays a member without a row.
    reason: String,
    /// The issue that decides it, as `owner/repo#number`.
    issue: String,
}

/// The 1-based line that byte `offset` of `text` is on.
fn line_at(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

/// A row with the line of its first key.
#[derive(Debug)]
struct Row {
    line: usize,
    exception: MembershipException,
}

/// Whether `issue` reads `owner/repo#number`.
fn is_issue_reference(issue: &str) -> bool {
    let Some((repo, number)) = issue.split_once('#') else {
        return false;
    };
    let mut parts = repo.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let word = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
    };
    word(owner)
        && word(name)
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
}

/// Reads the exceptions file of the corpus in `dir`. A missing file holds no exception.
fn read_exceptions(dir: &Path) -> Result<Vec<Row>, String> {
    let path = dir.join(EXCEPTIONS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{EXCEPTIONS_FILE}:1: cannot read file: {error}")),
    };
    parse_exceptions(&text)
}

fn parse_exceptions(text: &str) -> Result<Vec<Row>, String> {
    let file: ExceptionFile = toml::from_str(text).map_err(|error| {
        let line = error.span().map_or(1, |span| line_at(text, span.start));
        format!("{EXCEPTIONS_FILE}:{line}: {}", error.message())
    })?;
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for spanned in file.exception {
        let line = line_at(text, spanned.span().start);
        let exception = spanned.into_inner();
        let at = |message: &str| {
            format!(
                "{EXCEPTIONS_FILE}:{line}: exception for `{}` in `{}`: {message}",
                exception.entry, exception.book
            )
        };
        if exception.reason.trim().is_empty() {
            return Err(at("`reason` needs a text"));
        }
        if !is_issue_reference(&exception.issue) {
            return Err(at("`issue` reads `owner/repo#number`"));
        }
        if !seen.insert((exception.entry.clone(), exception.book.clone())) {
            return Err(at("a second exception for the same entry and book"));
        }
        rows.push(Row { line, exception });
    }
    Ok(rows)
}

pub(crate) fn check_uncited_memberships(corpus: &Corpus, report: &mut Report) {
    match read_exceptions(&corpus.dir) {
        Ok(rows) => check_memberships(corpus, &rows, report),
        Err(error) => {
            report.problems.push(error);
            check_memberships(corpus, &[], report);
        }
    }
}

fn check_memberships(corpus: &Corpus, rows: &[Row], report: &mut Report) {
    let mut listed: BTreeSet<(&str, &str)> = BTreeSet::new();
    for row in rows {
        let (entry_id, book) = (row.exception.entry.as_str(), row.exception.book.as_str());
        let stale = |message: &str| {
            format!(
                "{EXCEPTIONS_FILE}:{}: [memberships] exception for `{entry_id}` in `{book}`: {message}; remove the row",
                row.line
            )
        };
        let Some(stored) = corpus.entries.get(entry_id) else {
            report.problems.push(stale("the entry does not exist"));
            continue;
        };
        let entry = &stored.entry;
        if entry.source_ids.first().is_some_and(|first| first == book) {
            report
                .problems
                .push(stale("the book is the entry's first and needs no row"));
        } else if !entry.source_ids.iter().any(|id| id == book) {
            report
                .problems
                .push(stale("the entry does not list the book in `source_ids`"));
        } else if entry
            .sources
            .iter()
            .any(|source| source.publication == book)
        {
            report
                .problems
                .push(stale("a `[[sources]]` row cites the book now"));
        } else {
            listed.insert((entry_id, book));
        }
    }
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        for book in entry.source_ids.iter().skip(1) {
            if entry.sources.iter().any(|row| &row.publication == book)
                || listed.contains(&(entry.id.as_str(), book.as_str()))
            {
                continue;
            }
            report.problems.push(format!(
                "{}:{}: [memberships] entry `{}` lists `{book}` in `source_ids` but no `[[sources]]` row cites it; add the row if the book prints the entry, or remove the book if it does not (#461)",
                stored.path.display(),
                line_of(&stored.text, "source_ids"),
                entry.id
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus, rule};
    use cast_iron_schema::Citation;

    const GM_ROW: &str = "[[exception]]\nentry = \"ref/rule/shared\"\nbook = \"second-book\"\nreason = \"The home book is undecided.\"\nissue = \"phynics/cast-iron-data#277\"\n";

    /// A corpus with `ref/rule/shared`, a member of `second-book` (no row) and of `third-book`
    /// (a row).
    fn fixture() -> Corpus {
        let mut entry = rule("ref/rule/shared", vec![]);
        entry.source_ids.push("second-book".into());
        entry.source_ids.push("third-book".into());
        entry.sources.push(Citation {
            publication: "third-book".into(),
            page: Some("7".into()),
        });
        let mut c = corpus(vec![entry]);
        for id in ["second-book", "third-book"] {
            let mut publication = c.publications[0].clone();
            publication.id = id.into();
            publication.title = id.into();
            c.publications.push(publication);
        }
        c
    }

    fn run(corpus: &Corpus, file: &str) -> Report {
        let rows = parse_exceptions(file).expect("a valid file");
        let mut report = Report::default();
        check_memberships(corpus, &rows, &mut report);
        report
    }

    #[test]
    fn an_unlisted_membership_without_a_row_is_an_error() {
        let report = run(&fixture(), "");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].starts_with("entries/rule/shared.md:5: [memberships] entry `ref/rule/shared` lists `second-book`"),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn a_listed_membership_is_allowed() {
        let report = run(&fixture(), GM_ROW);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn a_first_book_and_cited_later_books_need_no_exception() {
        let mut c = fixture();
        c.entries
            .get_mut("ref/rule/shared")
            .expect("entry")
            .entry
            .sources
            .push(Citation {
                publication: "second-book".into(),
                page: None,
            });
        assert!(run(&c, "").problems.is_empty());
    }

    #[test]
    fn a_row_that_is_no_longer_needed_is_an_error() {
        // The book is cited now.
        let mut c = fixture();
        c.entries
            .get_mut("ref/rule/shared")
            .expect("entry")
            .entry
            .sources
            .push(Citation {
                publication: "second-book".into(),
                page: None,
            });
        let report = run(&c, GM_ROW);
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].contains("a `[[sources]]` row cites the book now")
                && report.problems[0].ends_with("remove the row"),
            "{:?}",
            report.problems
        );
        // The entry is gone.
        let report = run(&corpus(vec![]), GM_ROW);
        assert!(
            report.problems[0].contains("the entry does not exist"),
            "{:?}",
            report.problems
        );
        // The book left `source_ids`, or is the entry's first.
        let report = run(&fixture(), &GM_ROW.replace("second-book", "other-book"));
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.contains("does not list the book in `source_ids`")),
            "{:?}",
            report.problems
        );
        let report = run(&fixture(), &GM_ROW.replace("second-book", "core-rulebook"));
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.contains("the book is the entry's first")),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn a_row_covers_only_its_own_pair() {
        let mut c = fixture();
        let entry = &mut c.entries.get_mut("ref/rule/shared").expect("entry").entry;
        entry.source_ids.push("fourth-book".into());
        let report = run(&c, GM_ROW);
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].contains("`fourth-book`"),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn the_exceptions_file_is_read_strictly() {
        assert!(parse_exceptions("").expect("an empty file").is_empty());
        assert_eq!(parse_exceptions(GM_ROW).expect("valid").len(), 1);
        for (broken, fragment) in [
            (
                GM_ROW.replace("The home book is undecided.", " "),
                "needs a text",
            ),
            (
                GM_ROW.replace("phynics/cast-iron-data#277", "277"),
                "`owner/repo#number`",
            ),
            (
                GM_ROW.replace("phynics/cast-iron-data#277", "phynics/cast-iron-data#"),
                "`owner/repo#number`",
            ),
            (format!("{GM_ROW}{GM_ROW}"), "a second exception"),
            (
                GM_ROW.replace("issue =", "extra = 1\nissue ="),
                "unknown field",
            ),
            (
                GM_ROW.replace("book = \"second-book\"\n", ""),
                "missing field `book`",
            ),
        ] {
            let error = parse_exceptions(&broken).expect_err(&broken);
            assert!(
                error.starts_with("membership-exceptions.toml:") && error.contains(fragment),
                "{error}"
            );
        }
    }

    #[test]
    fn a_missing_file_is_an_empty_list_and_a_bad_one_is_a_problem() {
        let dir =
            std::env::temp_dir().join(format!("cast-iron-memberships-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temporary directory");
        assert!(read_exceptions(&dir).expect("no file").is_empty());
        std::fs::write(dir.join(EXCEPTIONS_FILE), "[[exception]]\nentry = 1\n").expect("write");
        let mut c = fixture();
        c.dir = dir.clone();
        let mut report = Report::default();
        check_uncited_memberships(&c, &mut report);
        std::fs::remove_dir_all(&dir).ok();
        // The file error comes first; with no row read, the membership is an error too.
        assert_eq!(report.problems.len(), 2, "{:?}", report.problems);
        assert!(report.problems[0].starts_with("membership-exceptions.toml:2:"));
        assert!(report.problems[1].contains("[memberships]"));
    }
}
