//! `fmt`: rewrite every corpus file into its one canonical form.
//!
//! The rule is `write(read(file)) == file` for every file. The reader accepts some
//! spellings that the writer never produces (extra blanks in a pipe cell, `2.` list
//! numbers, double blank lines, a missing final newline, `*` for emphasis); `fmt`
//! rewrites them. It also adds the explicit anchor to a heading that a link reaches.

use crate::config::write_config;
use crate::entry::{entry_path, write_entry};
use crate::error::Error;
use crate::files::{
    write_aliases, write_books, write_exclusions, write_history, write_legal, write_option_groups,
    write_publication_list,
};
use crate::load::Corpus;
use cast_iron_schema::RuleEntry;
use cast_iron_schema::headings::anchor_linked_headings;
use std::fs;
use std::path::{Path, PathBuf};

/// One file whose text is not canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Path relative to the corpus root.
    pub path: PathBuf,
    /// First line (1-based) that differs.
    pub line: usize,
    /// What the canonical file has on that line; `None` past its end.
    pub expected: Option<String>,
    /// What the file has on that line; `None` past its end.
    pub found: Option<String>,
    /// The canonical text.
    pub text: String,
}

impl Change {
    /// `path:line: expected ..., found ...`
    pub fn describe(&self) -> String {
        let show = |line: &Option<String>| match line {
            Some(text) if text.chars().count() > 120 => {
                format!("`{}...`", text.chars().take(120).collect::<String>())
            }
            Some(text) => format!("`{text}`"),
            None => "<end of file>".to_string(),
        };
        format!(
            "{}:{}: expected {}, found {}",
            self.path.display(),
            self.line,
            show(&self.expected),
            show(&self.found)
        )
    }
}

/// What a `fmt` run saw.
#[derive(Debug, Default, Clone)]
pub struct FmtReport {
    /// Files compared.
    pub files: usize,
    /// Files that are not canonical, in path order.
    pub changes: Vec<Change>,
}

/// The first line where `found` departs from `expected`.
fn first_difference(path: &Path, expected: &str, found: &str) -> Option<Change> {
    if expected == found {
        return None;
    }
    let mut expected_lines = expected.split('\n');
    let mut found_lines = found.split('\n');
    let mut line = 0;
    loop {
        line += 1;
        let (e, f) = (expected_lines.next(), found_lines.next());
        if e != f {
            // A file with the right text but no final newline ends one line early.
            return Some(Change {
                path: path.to_path_buf(),
                line,
                expected: e.map(str::to_string),
                found: f.map(str::to_string),
                text: expected.to_string(),
            });
        }
        e?;
    }
}

/// Compares every corpus file with its canonical form. With `write`, rewrites the
/// files that differ. Returns the read errors when the corpus does not parse.
pub fn format_corpus(dir: &Path, write: bool) -> std::result::Result<FmtReport, Vec<Error>> {
    let corpus = Corpus::load(dir)?;
    let mut report = FmtReport::default();

    // Anchor the headings that links and book nodes reach, across all entries.
    let mut entries: Vec<RuleEntry> = corpus
        .entries
        .values()
        .map(|stored| stored.entry.clone())
        .collect();
    anchor_linked_headings(&mut entries, &corpus.books);

    let mut compare = |relative: PathBuf, expected: String, found: &str| {
        report.files += 1;
        if let Some(change) = first_difference(&relative, &expected, found) {
            report.changes.push(change);
        }
    };
    for entry in &entries {
        let stored = &corpus.entries[entry.id.as_str()];
        compare(
            entry_path(&entry.id),
            write_entry(entry, &corpus.config),
            &stored.text,
        );
    }
    let mut stored_files: Vec<(&str, String)> = vec![
        ("corpus.toml", write_config(&corpus.config)),
        (
            "publications.toml",
            write_publication_list(&corpus.publications),
        ),
        ("books.toml", write_books(&corpus.books)),
        ("legal.toml", write_legal(&corpus.legal)),
        ("exclusions.toml", write_exclusions(&corpus.exclusions)),
    ];
    if let Some(history) = &corpus.history {
        stored_files.push(("history.toml", write_history(history)));
    }
    if let Some(aliases) = &corpus.aliases {
        stored_files.push(("aliases.toml", write_aliases(aliases)));
    }
    // A missing vocabulary file means no groups; `fmt` never creates it.
    if dir.join("option-groups.toml").is_file() {
        stored_files.push((
            "option-groups.toml",
            write_option_groups(&corpus.option_groups),
        ));
    }
    for (name, expected) in stored_files {
        let found = fs::read_to_string(dir.join(name))
            .map_err(|e| vec![Error::new(name, 1, format!("cannot read file: {e}"))])?;
        compare(PathBuf::from(name), expected, &found);
    }
    report.changes.sort_by(|a, b| a.path.cmp(&b.path));

    if write {
        for change in &report.changes {
            fs::write(dir.join(&change.path), &change.text).map_err(|e| {
                vec![Error::new(
                    &change.path,
                    1,
                    format!("cannot write file: {e}"),
                )]
            })?;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::write_config;
    use crate::files::{write_books, write_exclusions, write_legal};
    use crate::testkit::{corpus, heading, link, paragraph, rule};
    use cast_iron_schema::Inline;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-iron-fmt-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_tree(dir: &Path, source: &Corpus, overrides: &[(&str, &str)]) {
        for stored in source.entries.values() {
            let path = dir.join(entry_path(&stored.entry.id));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, write_entry(&stored.entry, &source.config)).unwrap();
        }
        fs::write(dir.join("corpus.toml"), write_config(&source.config)).unwrap();
        fs::write(
            dir.join("publications.toml"),
            write_publication_list(&source.publications),
        )
        .unwrap();
        fs::write(dir.join("books.toml"), write_books(&source.books)).unwrap();
        fs::write(dir.join("legal.toml"), write_legal(&source.legal)).unwrap();
        fs::write(
            dir.join("exclusions.toml"),
            write_exclusions(&source.exclusions),
        )
        .unwrap();
        for (name, text) in overrides {
            fs::write(dir.join(name), text).unwrap();
        }
    }

    #[test]
    fn a_canonical_corpus_is_already_formatted() {
        let dir = temp_dir("clean");
        let source = corpus(vec![rule(
            "ref/rule/a",
            vec![
                heading(None, "Title"),
                paragraph(vec![Inline::text("text")]),
            ],
        )]);
        write_tree(&dir, &source, &[]);
        let report = format_corpus(&dir, false).unwrap();
        assert!(report.changes.is_empty(), "{:?}", report.changes);
        assert_eq!(report.files, 6);
        let _ = fs::remove_dir_all(&dir);
    }

    const MESSY: &str = "+++
id = \"ref/rule/a\"
kind = \"rule\"
title = \"a\"
source_ids = [\"core-rulebook\"]

[fields]
kind = \"none\"
+++

Some *emphasis* here.


| a  |  b |
| --- | --- |
|  1 |2  |

1. one
2. two
3. three";

    #[test]
    fn check_names_the_first_differing_line_and_fmt_fixes_everything() {
        let dir = temp_dir("messy");
        let source = corpus(vec![rule("ref/rule/a", vec![])]);
        write_tree(&dir, &source, &[("entries/rule/a.md", MESSY)]);

        let report = format_corpus(&dir, false).unwrap();
        assert_eq!(report.changes.len(), 1);
        let change = &report.changes[0];
        assert_eq!(change.path, Path::new("entries/rule/a.md"));
        // The body starts on line 11; the first non-canonical line is the emphasis.
        assert_eq!(change.line, 11, "{}", change.describe());
        assert!(
            change.describe().contains("_emphasis_"),
            "{}",
            change.describe()
        );
        // Nothing was written by a check.
        assert_eq!(
            fs::read_to_string(dir.join("entries/rule/a.md")).unwrap(),
            MESSY
        );

        let report = format_corpus(&dir, true).unwrap();
        assert_eq!(report.changes.len(), 1);
        let fixed = fs::read_to_string(dir.join("entries/rule/a.md")).unwrap();
        assert!(
            fixed.ends_with("1. three\\n") || fixed.ends_with("1. three\n"),
            "{fixed}"
        );
        assert!(fixed.contains("|a|b|"), "{fixed}");
        assert!(fixed.contains("|1|2|"), "{fixed}");
        assert!(!fixed.contains("\n\n\n"), "{fixed}");
        assert!(fixed.contains("1. one\n1. two\n1. three\n"), "{fixed}");

        // A second run finds nothing: fmt is idempotent.
        assert!(format_corpus(&dir, false).unwrap().changes.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_final_newline_is_a_change_at_the_end_of_the_file() {
        let dir = temp_dir("newline");
        let source = corpus(vec![rule(
            "ref/rule/a",
            vec![paragraph(vec![Inline::text("x")])],
        )]);
        let text = write_entry(&source.entries["ref/rule/a"].entry, &source.config);
        write_tree(
            &dir,
            &source,
            &[("entries/rule/a.md", text.trim_end_matches('\n'))],
        );
        let report = format_corpus(&dir, false).unwrap();
        let change = &report.changes[0];
        assert_eq!(change.found, None);
        assert_eq!(change.expected.as_deref(), Some(""));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fmt_adds_the_anchor_a_link_reaches_and_check_names_it() {
        let dir = temp_dir("anchor");
        let a = rule(
            "ref/rule/a",
            vec![paragraph(vec![link("ref/rule/b", Some("sec-deep"), "b")])],
        );
        let b = rule("ref/rule/b", vec![heading(None, "Deep")]);
        let source = corpus(vec![a, b]);
        write_tree(&dir, &source, &[]);
        let report = format_corpus(&dir, false).unwrap();
        assert_eq!(report.changes.len(), 1);
        assert!(
            report.changes[0].describe().contains("## Deep {#sec-deep}"),
            "{}",
            report.changes[0].describe()
        );
        format_corpus(&dir, true).unwrap();
        let fixed = fs::read_to_string(dir.join("entries/rule/b.md")).unwrap();
        assert!(fixed.contains("## Deep {#sec-deep}"), "{fixed}");
        assert!(format_corpus(&dir, false).unwrap().changes.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_reader_rejects_comments_raw_html_and_carriage_returns_with_a_line() {
        let dir = temp_dir("reject");
        let source = corpus(vec![rule("ref/rule/a", vec![])]);
        let body = |text: &str| MESSY.replace("Some *emphasis* here.", text);
        for (text, fragment, line) in [
            (body("<!-- hidden note -->"), "comment", 11),
            (body("a <b>bold</b> word"), "raw HTML", 11),
            (MESSY.replace('\n', "\r\n"), "carriage return", 1),
        ] {
            write_tree(&dir, &source, &[("entries/rule/a.md", &text)]);
            let errors = format_corpus(&dir, false).unwrap_err();
            let message = errors[0].to_string();
            assert!(message.contains(fragment), "{message}");
            assert_eq!(errors[0].line, line, "{message}");
            assert!(message.starts_with("entries/rule/a.md:"), "{message}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stored_toml_files_are_checked_too() {
        let dir = temp_dir("toml");
        let source = corpus(vec![rule("ref/rule/a", vec![])]);
        write_tree(
            &dir,
            &source,
            &[(
                "corpus.toml",
                "# the defaults\nformat = 1\n\n[defaults]\nlicense_refs = [\"ogl-1.0a\", \"legal#community-use\"]\n",
            )],
        );
        let report = format_corpus(&dir, false).unwrap();
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].path, Path::new("corpus.toml"));
        assert_eq!(report.changes[0].line, 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
