//! The corpus-check rule for class-option powers (#537).
//!
//! A class-option power is a bold name paragraph followed by labeled rows. The decision
//! `2026-11-20-class-option-power-layout` fixes one canonical row order, and
//! `cast_iron_schema::power` holds the one classifier and validator for it. This rule runs them on
//! every `class_option` entry and reports each candidate that fails validation as a warning, with
//! its path, line, entry id, power name and reasons. It is a warning until the corpus is rewritten
//! (phynics/cast-iron-data#88); the pin of the rewritten corpus turns it into an error.

use crate::check::{Report, body_text};
use crate::load::{Corpus, Stored};
use cast_iron_schema::EntryKind;
use cast_iron_schema::power::{candidates, validate_power};

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
    let mut candidate_total = 0usize;
    let mut failing_total = 0usize;
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
        for candidate in found {
            candidate_total += 1;
            let Err(problems) = validate_power(candidate.blocks(&entry.body)) else {
                continue;
            };
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
    if failing_total > 0 {
        report.warnings.push(format!(
            "{failing_total} of {candidate_total} class-option powers are not in the canonical form, in {failing_entries} of {candidate_entries} entries that hold a power; the pin of the rewritten corpus makes each one an error"
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

        // The action row first is its own reason.
        let (found, _) = warnings(vec![option(
            "ref/domain/backwards",
            EntryKind::ClassOption,
            vec![
                name("Backwards (Su)"),
                row("Standard", "Touch."),
                row("Passive", "A bonus."),
            ],
        )]);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains("[action-before-passive]"), "{found:?}");
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
}
