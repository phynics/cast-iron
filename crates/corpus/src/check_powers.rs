//! The corpus-check rule for class-option powers (#537).
//!
//! A class-option power is a bold name paragraph followed by labeled rows. The decision
//! `2026-11-20-class-option-power-layout` fixes one canonical row order, and
//! `cast_iron_schema::power` holds the one classifier and validator for it. This rule runs them on
//! every `class_option` entry and reports each candidate that fails validation as a warning, with
//! its path, line, entry id, power name and reasons. It stays a warning until the review queue is
//! resolved (#557); the app pull request that pins the resolved corpus makes it an error.
//!
//! A power that print cannot settle is listed in `power-exceptions.toml` in the corpus root (see
//! `docs/corpus-format.md`, "Class-option powers"): the rule skips a failing power that the file
//! names, and reports an exception that names no power, or a power that passes, as a warning.

use crate::check::{Report, body_text};
use crate::load::{Corpus, Stored};
use cast_iron_schema::EntryKind;
use cast_iron_schema::power::{candidates, validate_power};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The optional file in the corpus root that lists the powers allowed to fail validation.
pub const EXCEPTIONS_FILE: &str = "power-exceptions.toml";

/// `power-exceptions.toml`: one `[[exception]]` row per power. The corpus repository's script
/// writes `span_sha256` and checks it; the rule here does not read it.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionFile {
    #[serde(default)]
    exception: Vec<PowerException>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PowerException {
    /// The entry id (`ref/hkorder/order-of-the-chain`).
    id: String,
    /// The power's name as the entry writes it (`Reckoning`).
    name: String,
    /// Which power of that name in the entry, counting from 1.
    occurrence: u32,
    /// The SHA-256 of the power's source text, for the corpus repository's own check.
    #[serde(default)]
    #[allow(dead_code)]
    span_sha256: Option<String>,
    /// Why print cannot settle the power.
    reason: String,
    /// The book title and page that was checked, or why none exists.
    print: String,
}

/// An entry id, a power name and an occurrence.
type PowerKey = (String, String, u32);

impl PowerException {
    fn key(&self) -> PowerKey {
        (self.id.clone(), self.name.clone(), self.occurrence)
    }
}

/// The 1-based line that byte `offset` of `text` is on.
fn line_at(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

/// Reads and validates the exceptions file of the corpus in `dir`. A missing file holds no
/// exception.
fn read_exceptions(dir: &Path) -> Result<Vec<PowerException>, String> {
    let path = dir.join(EXCEPTIONS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{EXCEPTIONS_FILE}:1: cannot read file: {error}")),
    };
    parse_exceptions(&text)
}

fn parse_exceptions(text: &str) -> Result<Vec<PowerException>, String> {
    let file: ExceptionFile = toml::from_str(text).map_err(|error| {
        let line = error.span().map_or(1, |span| line_at(text, span.start));
        format!("{EXCEPTIONS_FILE}:{line}: {}", error.message())
    })?;
    let mut seen = BTreeSet::new();
    for row in &file.exception {
        let at = |message: String| {
            format!(
                "{EXCEPTIONS_FILE}:1: exception `{}` of entry `{}`: {message}",
                row.name, row.id
            )
        };
        if row.occurrence == 0 {
            return Err(at("`occurrence` counts from 1".into()));
        }
        if row.reason.trim().is_empty() || row.print.trim().is_empty() {
            return Err(at("`reason` and `print` need a text".into()));
        }
        if !seen.insert(row.key()) {
            return Err(at(format!(
                "a second exception for occurrence {}",
                row.occurrence
            )));
        }
    }
    Ok(file.exception)
}

/// The 1-based line of the top-level body block `index`, as the parser recorded it. An entry that
/// carries no recorded lines (one built in a test) falls back to the first line of its body.
fn block_line(stored: &Stored, index: usize) -> usize {
    stored
        .block_lines
        .get(index)
        .copied()
        .unwrap_or_else(|| body_text(&stored.text).0)
}

pub(crate) fn check_class_option_powers(corpus: &Corpus, report: &mut Report) {
    match read_exceptions(&corpus.dir) {
        Ok(exceptions) => check_powers(corpus, &exceptions, report),
        Err(error) => {
            report.problems.push(error);
            check_powers(corpus, &[], report);
        }
    }
}

fn check_powers(corpus: &Corpus, exceptions: &[PowerException], report: &mut Report) {
    let wanted: BTreeSet<PowerKey> = exceptions.iter().map(PowerException::key).collect();
    // The exceptions that name a failing power, and those that name one that passes.
    let mut used: BTreeSet<PowerKey> = BTreeSet::new();
    let mut passing: BTreeSet<PowerKey> = BTreeSet::new();
    let mut candidate_total = 0usize;
    let mut failing_total = 0usize;
    let mut excepted_total = 0usize;
    let mut failing_entries = 0usize;
    let mut candidate_entries = 0usize;
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        if entry.kind != EntryKind::ClassOption {
            continue;
        }
        let mut failing = 0usize;
        let found = candidates(&entry.body);
        if !found.is_empty() {
            candidate_entries += 1;
        }
        let mut occurrences: BTreeMap<String, u32> = BTreeMap::new();
        for candidate in found {
            candidate_total += 1;
            let occurrence = occurrences.entry(candidate.name.clone()).or_insert(0);
            *occurrence += 1;
            let key: PowerKey = (entry.id.to_string(), candidate.name.clone(), *occurrence);
            let excepted = wanted.contains(&key);
            let Err(problems) = validate_power(candidate.blocks(&entry.body)) else {
                if excepted {
                    passing.insert(key);
                }
                continue;
            };
            if excepted {
                used.insert(key);
                excepted_total += 1;
                continue;
            }
            failing += 1;
            let reasons: Vec<String> = problems
                .iter()
                .map(|problem| format!("{problem} [{}]", problem.reason()))
                .collect();
            report.warnings.push(format!(
                "{}:{}: [powers] entry `{}`: the power `{}` is not in the canonical form: {}",
                stored.path.display(),
                block_line(stored, candidate.start),
                entry.id,
                candidate.name,
                reasons.join("; ")
            ));
        }
        failing_total += failing;
        if failing > 0 {
            failing_entries += 1;
        }
    }
    for row in exceptions {
        let key = row.key();
        if used.contains(&key) {
            continue;
        }
        let why = if passing.contains(&key) {
            "the power is in the canonical form; remove the exception"
        } else {
            "no class-option power matches; remove the exception"
        };
        report.warnings.push(format!(
            "{EXCEPTIONS_FILE}: [powers] exception for `{}` (occurrence {}) of entry `{}`: {why}",
            row.name, row.occurrence, row.id
        ));
    }
    if failing_total > 0 {
        let excepted = if excepted_total > 0 {
            format!("; {excepted_total} more have an exception in `{EXCEPTIONS_FILE}`")
        } else {
            String::new()
        };
        report.warnings.push(format!(
            "{failing_total} of {candidate_total} class-option powers are not in the canonical form, in {failing_entries} of {candidate_entries} entries that hold a power{excepted}; the pin of the resolved corpus makes each one an error"
        ));
    } else if excepted_total > 0 {
        report.warnings.push(format!(
            "{excepted_total} of {candidate_total} class-option powers have an exception in `{EXCEPTIONS_FILE}` and no other is outside the canonical form"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus, paragraph, rule};
    use cast_iron_schema::{Block, ClassOptionFields, Inline, TypedFields};

    fn strong(text: &str) -> Inline {
        Inline::Strong {
            c: vec![Inline::text(text)],
        }
    }

    fn name(text: &str) -> Block {
        paragraph(vec![strong(text)])
    }

    fn row(label: &str, value: &str) -> Block {
        paragraph(vec![
            strong(&format!("{label}:")),
            Inline::text(format!(" {value}")),
        ])
    }

    fn option(id: &str, kind: EntryKind, body: Vec<Block>) -> cast_iron_schema::RuleEntry {
        let mut entry = rule(id, body);
        if kind == EntryKind::ClassOption {
            entry.kind = kind;
            entry.fields = TypedFields::ClassOption(ClassOptionFields {
                group: "hex".into(),
                ..Default::default()
            });
        }
        entry
    }

    fn warnings(entries: Vec<cast_iron_schema::RuleEntry>) -> (Vec<String>, Corpus) {
        let corpus = corpus(entries);
        let mut report = Report::default();
        check_class_option_powers(&corpus, &mut report);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        (report.warnings, corpus)
    }

    /// A power with its `Gained` row after the effect, and the same power in canonical form.
    fn failing(label: &str) -> Vec<Block> {
        vec![
            name(label),
            row("Passive", "It works."),
            row("Gained", "At 1st Level"),
        ]
    }

    fn canonical(label: &str) -> Vec<Block> {
        vec![
            name(label),
            row("Gained", "At 1st Level"),
            row("Passive", "It works."),
        ]
    }

    #[test]
    fn a_failing_power_is_a_warning_with_its_place_id_name_and_reason() {
        let mut body = canonical("Fine Power (Ex)");
        body.extend(failing("Bad Power (Su)"));
        let (warnings, corpus) = warnings(vec![option(
            "ref/hex/example",
            EntryKind::ClassOption,
            body,
        )]);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        let warning = &warnings[0];
        assert!(warning.contains("entries/hex/example.md:"), "{warning}");
        assert!(warning.contains("entry `ref/hex/example`"), "{warning}");
        assert!(warning.contains("the power `Bad Power (Su)`"), "{warning}");
        assert!(warning.contains("[out-of-order]"), "{warning}");
        // The line is the line of the name in the entry file.
        let line: usize = warning
            .split("example.md:")
            .nth(1)
            .and_then(|rest| rest.split(':').next())
            .and_then(|digits| digits.parse().ok())
            .expect("a line number");
        let stored = &corpus.entries["ref/hex/example"];
        assert_eq!(
            stored.text.lines().nth(line - 1),
            Some("**Bad Power (Su)**"),
            "{}",
            stored.text
        );
        // The summary names the counts.
        assert!(
            warnings[1].contains(
                "1 of 2 class-option powers are not in the canonical form, in 1 of 1 entries"
            ),
            "{warnings:?}"
        );
    }

    #[test]
    fn two_powers_with_one_name_report_their_own_lines() {
        let mut body = failing("Twin (Su)");
        body.extend(failing("Twin (Su)"));
        let (warnings, _) = warnings(vec![option("ref/hex/twin", EntryKind::ClassOption, body)]);
        let lines: Vec<&str> = warnings[..2]
            .iter()
            .map(|warning| {
                warning
                    .split("twin.md:")
                    .nth(1)
                    .unwrap()
                    .split(':')
                    .next()
                    .unwrap()
            })
            .collect();
        assert_ne!(lines[0], lines[1], "{warnings:?}");
    }

    #[test]
    fn a_matching_line_inside_an_earlier_paragraph_does_not_take_the_warning() {
        // A hard break puts `**Bad Power (Su)**` alone on a line of a prose paragraph.
        let prose = paragraph(vec![
            Inline::text("Intro"),
            Inline::Break,
            strong("Bad Power (Su)"),
        ]);
        let mut body = vec![prose];
        body.extend(failing("Bad Power (Su)"));
        let (warnings, corpus) =
            warnings(vec![option("ref/hex/prose", EntryKind::ClassOption, body)]);
        let line: usize = warnings[0]
            .split("prose.md:")
            .nth(1)
            .and_then(|rest| rest.split(':').next())
            .and_then(|digits| digits.parse().ok())
            .expect("a line number");
        let text = &corpus.entries["ref/hex/prose"].text;
        let lines: Vec<&str> = text.lines().collect();
        let matching: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == "**Bad Power (Su)**")
            .map(|(i, _)| i + 1)
            .collect();
        assert_eq!(matching.len(), 2, "{text}");
        assert_eq!(line, matching[1], "{text}");
    }

    #[test]
    fn the_owner_cases_of_2026_11_21_report_nothing_or_their_own_reason() {
        let canonical = vec![
            name("Noble Visage (Su)"),
            row("Passive", "A bonus."),
            row("Standard", "Touch an ally."),
            row("Usage", "3 + Wisdom modifier times/day"),
            row("Replaces", "The inspiring word ability"),
            name("Natural Spell Combat (Ex)"),
            row("Choice", "One spell."),
            row("Ability", "Use spell combat."),
            row("Special", "More than once."),
            paragraph(vec![Inline::text("Another paragraph of the note.")]),
            // A section label that holds upgrade-like rows is no power.
            name("Base Evolutions"),
            row("At 1st Level", "Gain an evolution."),
        ];
        let (none, _) = warnings(vec![option(
            "ref/domain/nobility",
            EntryKind::ClassOption,
            canonical,
        )]);
        assert!(none.is_empty(), "{none:?}");

        // The action row first, then the passive row, is print's order and valid.
        let (none, _) = warnings(vec![option(
            "ref/domain/backwards",
            EntryKind::ClassOption,
            vec![
                name("Backwards (Su)"),
                row("Standard", "Touch."),
                row("Passive", "A bonus."),
            ],
        )]);
        assert!(none.is_empty(), "{none:?}");

        // Two actions are two effect rows.
        let (found, _) = warnings(vec![option(
            "ref/domain/twice",
            EntryKind::ClassOption,
            vec![
                name("Twice (Su)"),
                row("Standard", "Touch."),
                row("Swift", "Again."),
            ],
        )]);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains("[multiple-effect-rows]"), "{found:?}");
    }

    #[test]
    fn canonical_powers_and_other_kinds_report_nothing() {
        let (none, _) = warnings(vec![option(
            "ref/hex/fine",
            EntryKind::ClassOption,
            canonical("Fine (Ex)"),
        )]);
        assert!(none.is_empty(), "{none:?}");
        // The same failing body in an entry of another kind is not checked.
        let (none, _) = warnings(vec![option(
            "ref/rule/other",
            EntryKind::Rule,
            failing("Bad (Su)"),
        )]);
        assert!(none.is_empty(), "{none:?}");
    }

    fn excepted(entries: Vec<cast_iron_schema::RuleEntry>, file: &str) -> Vec<String> {
        let corpus = corpus(entries);
        let exceptions = parse_exceptions(file).expect("a valid exceptions file");
        let mut report = Report::default();
        check_powers(&corpus, &exceptions, &mut report);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        report.warnings
    }

    const BAD_POWER_EXCEPTION: &str = r#"
[[exception]]
id = "ref/hex/example"
name = "Bad Power (Su)"
occurrence = 1
reason = "print cannot settle it"
print = "Core Rulebook p. 1"
"#;

    #[test]
    fn an_exception_silences_the_failing_power_it_names_and_the_summary_counts_it() {
        let mut body = canonical("Fine Power (Ex)");
        body.extend(failing("Bad Power (Su)"));
        body.extend(failing("Other Bad Power (Su)"));
        let warnings = excepted(
            vec![option("ref/hex/example", EntryKind::ClassOption, body)],
            BAD_POWER_EXCEPTION,
        );
        // One warning for the power without an exception, then the summary.
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(
            warnings[0].contains("`Other Bad Power (Su)`"),
            "{warnings:?}"
        );
        assert!(
            warnings[1].contains("1 of 3 class-option powers are not in the canonical form")
                && warnings[1].contains("1 more have an exception in `power-exceptions.toml`"),
            "{warnings:?}"
        );
    }

    #[test]
    fn with_every_failing_power_excepted_one_note_remains() {
        let warnings = excepted(
            vec![option(
                "ref/hex/example",
                EntryKind::ClassOption,
                failing("Bad Power (Su)"),
            )],
            BAD_POWER_EXCEPTION,
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].contains("1 of 1 class-option powers have an exception"),
            "{warnings:?}"
        );
    }

    #[test]
    fn an_exception_counts_the_occurrence_among_powers_of_one_name() {
        let mut body = canonical("Twin (Su)");
        body.extend(failing("Twin (Su)"));
        let file = BAD_POWER_EXCEPTION
            .replace("Bad Power (Su)", "Twin (Su)")
            .replace("occurrence = 1", "occurrence = 2");
        let warnings = excepted(
            vec![option("ref/hex/example", EntryKind::ClassOption, body)],
            &file,
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("have an exception"), "{warnings:?}");
        // The same exception for occurrence 1 names a power that passes.
        let mut body = canonical("Twin (Su)");
        body.extend(failing("Twin (Su)"));
        let file = file.replace("occurrence = 2", "occurrence = 1");
        let warnings = excepted(
            vec![option("ref/hex/example", EntryKind::ClassOption, body)],
            &file,
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("remove the exception")
                    && warning.contains("canonical form")),
            "{warnings:?}"
        );
    }

    #[test]
    fn an_exception_that_names_no_power_or_a_passing_power_is_a_warning() {
        let warnings = excepted(
            vec![option(
                "ref/hex/example",
                EntryKind::ClassOption,
                canonical("Bad Power (Su)"),
            )],
            BAD_POWER_EXCEPTION,
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].contains("power-exceptions.toml: [powers] exception for `Bad Power (Su)`")
                && warnings[0].contains("the power is in the canonical form"),
            "{warnings:?}"
        );
        let warnings = excepted(
            vec![option(
                "ref/hex/other",
                EntryKind::ClassOption,
                failing("Bad Power (Su)"),
            )],
            BAD_POWER_EXCEPTION,
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("no class-option power matches")),
            "{warnings:?}"
        );
        // The failing power in the other entry still warns.
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("is not in the canonical form: ")),
            "{warnings:?}"
        );
    }

    #[test]
    fn the_exceptions_file_is_read_strictly() {
        assert!(parse_exceptions("").expect("an empty file").is_empty());
        let rows = parse_exceptions(&format!("{BAD_POWER_EXCEPTION}span_sha256 = \"ab\"\n"))
            .expect("the optional hash is accepted");
        assert_eq!(rows.len(), 1);
        for (broken, fragment) in [
            (
                BAD_POWER_EXCEPTION.replace("occurrence = 1", "occurrence = 0"),
                "counts from 1",
            ),
            (
                BAD_POWER_EXCEPTION.replace("print cannot settle it", " "),
                "need a text",
            ),
            (
                BAD_POWER_EXCEPTION.replace("occurrence = 1", "occurrence = 1\nextra = 1"),
                "unknown field",
            ),
            (
                format!("{BAD_POWER_EXCEPTION}{BAD_POWER_EXCEPTION}"),
                "a second exception",
            ),
        ] {
            let error = parse_exceptions(&broken).expect_err(&broken);
            assert!(
                error.starts_with("power-exceptions.toml:") && error.contains(fragment),
                "{error}"
            );
        }
    }

    #[test]
    fn a_bad_exceptions_file_is_a_problem_and_a_missing_one_is_not() {
        let dir = std::env::temp_dir().join(format!("cast-iron-powers-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temporary directory");
        assert!(
            read_exceptions(&dir)
                .expect("no file, no exception")
                .is_empty()
        );
        std::fs::write(dir.join(EXCEPTIONS_FILE), "[[exception]]\nid = 1\n").expect("write");
        let error = read_exceptions(&dir).expect_err("a bad file");
        assert!(error.starts_with("power-exceptions.toml:2:"), "{error}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
