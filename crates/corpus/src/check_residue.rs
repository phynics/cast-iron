//! The corpus-check guards for the import residue of class options
//! (phynics/cast-iron-data#88).
//!
//! The retired upstream import left eight defects in class-option entries. The corpus repository
//! fixed all of them (#95), and the survey `scripts/survey-class-option-residue.py` of that
//! repository reports 0 for each. These checks keep them at 0: each is an error, so a corpus
//! change that brings one back fails before a tag.
//!
//! | Defect | What it is |
//! |---|---|
//! | `skill-code` | a `Gained class skills:` or `Lost class skills:` value that is an upstream skill code (`kd`, `stl`, `perf\|dance`) |
//! | `spell-no-level` | a `Spells:` row that does not end in its granted level (`(3rd)`) |
//! | `spell-order` | a run of `Spells:` and `Cantrips:` rows whose stated levels are not ascending |
//! | `key-tilde` | a `**Key:** ~` row: the import's line-break marker |
//! | `misplaced-title` | a `Title:` row directly after a `Details:` row |
//! | `bonus-feats-glue` | a bonus-feat `Details:` row, followed by a `Title:` row, whose feat links have no separator |
//! | `import-key` | an `Id:`, `From:` or `Save: domain` row: a renderer instruction of the upstream |
//! | `glued-modifier` | a `Usage:` row such as `Charisma3 modifier times/day` |
//!
//! Only `class_option` entries are scanned, and only their top-level paragraphs, as the survey
//! does. The literal `~` in prose (`tilde-in-text`) and the numbered effect labels stay out: the
//! survey reports them, and the power validator (`check_powers`) handles the labels.

use crate::check::{Report, body_text};
use crate::load::{Corpus, Stored};
use cast_iron_schema::headings::plain_text;
use cast_iron_schema::power::{BlockKind, Row, classify};
use cast_iron_schema::{Block, EntryKind, Inline};

/// The upstream skill codes. A code that stands where a printed skill name belongs is a defect.
const SKILL_CODES: [&str; 35] = [
    "acro", "app", "bluff", "climb", "craft", "dd", "diplo", "dis", "ea", "fly", "ha", "heal",
    "intm", "ka", "kd", "ke", "kg", "kh", "kl", "kna", "kno", "kp", "kr", "ling", "per", "perf",
    "prof", "ride", "sm", "soh", "spc", "stl", "sur", "swim", "umd",
];

/// The level a `Spells:` row states at its end, `(3rd)`.
fn stated_level(text: &str) -> Option<u32> {
    let text = text.trim_end().strip_suffix(')')?;
    let (_, inner) = text.rsplit_once('(')?;
    let digits = inner
        .strip_suffix("st")
        .or_else(|| inner.strip_suffix("nd"))
        .or_else(|| inner.strip_suffix("rd"))
        .or_else(|| inner.strip_suffix("th"))?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The level a `Spells:` or `Cantrips:` row states: 0 for a cantrip row, `None` when it states none.
fn row_level(row: &Row<'_>) -> Option<u32> {
    if row.label == "Cantrips" {
        Some(0)
    } else {
        stated_level(&plain_text(row.content))
    }
}

/// `Charisma3 modifier`: letters, then one digit, then ` modifier` (the upstream `useMod`).
fn has_glued_modifier(text: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = " modifier";
    let mut from = 0;
    while let Some(found) = text[from..].find(needle) {
        let at = from + found;
        if at >= 2 && bytes[at - 1].is_ascii_digit() && bytes[at - 2].is_ascii_alphabetic() {
            return true;
        }
        from = at + needle.len();
    }
    false
}

/// Two links with nothing between them: `)[` in the source.
fn has_glued_links(content: &[Inline]) -> bool {
    content
        .windows(2)
        .any(|pair| matches!(pair, [Inline::Link { .. }, Inline::Link { .. }]))
}

/// The row of a top-level block, when it is one.
fn row_of(block: &Block) -> Option<Row<'_>> {
    match classify(block) {
        BlockKind::Row(row) => Some(row),
        _ => None,
    }
}

fn block_line(stored: &Stored, index: usize) -> usize {
    stored
        .block_lines
        .get(index)
        .copied()
        .unwrap_or_else(|| body_text(&stored.text).0)
}

pub(crate) fn check_class_option_residue(corpus: &Corpus, report: &mut Report) {
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        if entry.kind != EntryKind::ClassOption {
            continue;
        }
        let rows: Vec<Option<Row<'_>>> = entry.body.iter().map(row_of).collect();
        let mut problem = |index: usize, defect: &str, what: String| {
            report.problems.push(format!(
                "{}:{}: [residue] entry `{}`: {what} [{defect}]",
                stored.path.display(),
                block_line(stored, index),
                entry.id,
            ));
        };
        for (index, row) in rows.iter().enumerate() {
            let Some(row) = row else { continue };
            let value = plain_text(row.content);
            let value = value.trim();
            let next = rows.get(index + 1).and_then(Option::as_ref);
            let previous = index
                .checked_sub(1)
                .and_then(|before| rows[before].as_ref());
            match row.label.as_str() {
                "Gained class skills" | "Lost class skills" => {
                    let codes: Vec<&str> = value
                        .split(',')
                        .map(str::trim)
                        .filter(|token| !token.is_empty())
                        .filter(|token| {
                            let head = token.split('|').next().unwrap_or(token);
                            SKILL_CODES.contains(&head)
                        })
                        .collect();
                    if !codes.is_empty() {
                        problem(
                            index,
                            "skill-code",
                            format!(
                                "`{}:` holds the upstream skill code {}; write the skill name",
                                row.label,
                                codes
                                    .iter()
                                    .map(|code| format!("`{code}`"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        );
                    }
                }
                "Spells" if stated_level(value).is_none() => problem(
                    index,
                    "spell-no-level",
                    "a `Spells:` row does not end in its granted level, such as `(3rd)`".into(),
                ),
                "Title" if previous.is_some_and(|before| before.label == "Details") => problem(
                    index,
                    "misplaced-title",
                    "a `Title:` row follows a `Details:` row; the title is the row's label".into(),
                ),
                "Id" | "From" => problem(
                    index,
                    "import-key",
                    format!(
                        "the `{}:` row is an upstream renderer instruction",
                        row.label
                    ),
                ),
                "Save" if value == "domain" => problem(
                    index,
                    "import-key",
                    "the `Save: domain` row is an upstream renderer instruction".into(),
                ),
                "Usage" if has_glued_modifier(value) => problem(
                    index,
                    "glued-modifier",
                    format!(
                        "`Usage: {value}` has a glued ability modifier; write `3 + your Charisma modifier`"
                    ),
                ),
                "Details"
                    if next.is_some_and(|after| after.label == "Title")
                        && (has_glued_links(row.content) || value.contains('~')) =>
                {
                    problem(
                        index,
                        "bonus-feats-glue",
                        "a `Details:` row has feat links with no separator".into(),
                    )
                }
                _ => {}
            }
            if value == "~" {
                problem(
                    index,
                    "key-tilde",
                    format!(
                        "the `{}:` row holds only the import's line-break marker `~`",
                        row.label
                    ),
                );
            }
        }
        // Runs of consecutive `Spells:` and `Cantrips:` rows whose stated levels all exist.
        let mut at = 0;
        while at < rows.len() {
            let is_spell_row = |row: &Option<Row<'_>>| {
                row.as_ref()
                    .is_some_and(|row| row.label == "Spells" || row.label == "Cantrips")
            };
            if !is_spell_row(&rows[at]) {
                at += 1;
                continue;
            }
            let end = (at..rows.len())
                .find(|&index| !is_spell_row(&rows[index]))
                .unwrap_or(rows.len());
            let levels: Option<Vec<u32>> = rows[at..end]
                .iter()
                .map(|row| row.as_ref().and_then(row_level))
                .collect();
            if let Some(levels) = levels
                && levels.windows(2).any(|pair| pair[0] > pair[1])
            {
                report.problems.push(format!(
                    "{}:{}: [residue] entry `{}`: the bonus spells are not in ascending level order [spell-order]",
                    stored.path.display(),
                    block_line(stored, at),
                    entry.id,
                ));
            }
            at = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus, paragraph, rule};
    use cast_iron_schema::{ClassOptionFields, EntryId, TypedFields};

    fn strong(text: &str) -> Inline {
        Inline::Strong {
            c: vec![Inline::text(text)],
        }
    }

    fn row(label: &str, value: &str) -> Block {
        paragraph(vec![
            strong(&format!("{label}:")),
            Inline::text(format!(" {value}")),
        ])
    }

    fn link(text: &str) -> Inline {
        Inline::Link {
            target: EntryId::parse("ref/feat/toughness").expect("an id"),
            anchor: None,
            c: vec![Inline::text(text)],
        }
    }

    fn option(kind: EntryKind, body: Vec<Block>) -> cast_iron_schema::RuleEntry {
        let mut entry = rule("ref/sorcbloodline/example", body);
        if kind == EntryKind::ClassOption {
            entry.kind = kind;
            entry.fields = TypedFields::ClassOption(ClassOptionFields {
                group: "sorcerer-bloodline".into(),
                ..Default::default()
            });
        }
        entry
    }

    fn problems(kind: EntryKind, body: Vec<Block>) -> Vec<String> {
        let corpus = corpus(vec![option(kind, body)]);
        let mut report = Report::default();
        check_class_option_residue(&corpus, &mut report);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        report.problems
    }

    fn only(body: Vec<Block>, defect: &str) {
        let found = problems(EntryKind::ClassOption, body);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains(&format!("[{defect}]")), "{found:?}");
        assert!(
            found[0].contains("entry `ref/sorcbloodline/example`"),
            "{found:?}"
        );
        assert!(found[0].contains("[residue]"), "{found:?}");
    }

    #[test]
    fn a_clean_class_option_reports_nothing() {
        let body = vec![
            row("Gained class skills", "Knowledge (dungeoneering), Stealth"),
            row("Cantrips", "Light"),
            row("Spells", "Shield (3rd)"),
            row("Spells", "Shout (5th)"),
            row(
                "Details",
                "[Toughness](ref/feat/toughness), [Endurance](ref/feat/endurance)",
            ),
            row("Usage", "3 + Charisma modifier times/day"),
        ];
        assert!(problems(EntryKind::ClassOption, body).is_empty());
    }

    #[test]
    fn a_skill_code_is_an_error_and_a_printed_name_is_not() {
        only(vec![row("Gained class skills", "kd")], "skill-code");
        only(
            vec![row("Lost class skills", "Stealth, perf|dance")],
            "skill-code",
        );
        assert!(
            problems(
                EntryKind::ClassOption,
                vec![row("Gained class skills", "Fly, Swim")]
            )
            .is_empty()
        );
    }

    #[test]
    fn a_spell_row_needs_its_level() {
        only(vec![row("Spells", "Shield")], "spell-no-level");
        assert!(problems(EntryKind::ClassOption, vec![row("Spells", "Shield (1st)")]).is_empty());
        // Only the shape counts: a number and its suffix.
        assert!(problems(EntryKind::ClassOption, vec![row("Spells", "Shield (11th)")]).is_empty());
    }

    #[test]
    fn spells_out_of_numeric_order_are_an_error() {
        let body = vec![row("Spells", "A (11th)"), row("Spells", "B (3rd)")];
        only(body, "spell-order");
        // The text-sorted order of the upstream keys, with levels: 11, 13, 3.
        let body = vec![
            row("Spells", "A (11th)"),
            row("Spells", "B (13th)"),
            row("Spells", "C (3rd)"),
        ];
        only(body, "spell-order");
        let sorted = vec![
            row("Cantrips", "Light"),
            row("Spells", "B (3rd)"),
            row("Spells", "A (11th)"),
        ];
        assert!(problems(EntryKind::ClassOption, sorted).is_empty());
    }

    #[test]
    fn two_runs_are_checked_apart() {
        let body = vec![
            row("Spells", "A (9th)"),
            paragraph(vec![Inline::text("Between.")]),
            row("Spells", "B (1st)"),
        ];
        assert!(problems(EntryKind::ClassOption, body).is_empty());
    }

    #[test]
    fn a_tilde_row_is_an_error() {
        only(vec![row("Br", "~")], "key-tilde");
        assert!(problems(EntryKind::ClassOption, vec![row("Br", "a ~ b")]).is_empty());
    }

    #[test]
    fn a_title_after_details_is_an_error() {
        let body = vec![row("Details", "Toughness"), row("Title", "Bonus Feats")];
        only(body, "misplaced-title");
        let apart = vec![
            row("Details", "Toughness"),
            row("Sources", "A book"),
            row("Title", "Bonus Feats"),
        ];
        assert!(problems(EntryKind::ClassOption, apart).is_empty());
    }

    #[test]
    fn glued_bonus_feat_links_are_an_error() {
        let glued = paragraph(vec![
            strong("Details:"),
            Inline::text(" "),
            link("Toughness"),
            link("Endurance"),
        ]);
        let found = problems(
            EntryKind::ClassOption,
            vec![glued, row("Title", "Bonus Feats")],
        );
        assert!(
            found.iter().any(|p| p.contains("[bonus-feats-glue]")),
            "{found:?}"
        );
        // The misplaced title is reported too: it is a separate defect.
        assert!(
            found.iter().any(|p| p.contains("[misplaced-title]")),
            "{found:?}"
        );
        // Links with a separator are fine, and so are glued links when no `Title:` row follows.
        let spaced = paragraph(vec![
            strong("Details:"),
            Inline::text(" "),
            link("Toughness"),
            Inline::text(", "),
            link("Endurance"),
        ]);
        assert!(
            problems(
                EntryKind::ClassOption,
                vec![spaced, row("Title", "Bonus Feats")]
            )
            .iter()
            .all(|p| !p.contains("[bonus-feats-glue]"))
        );
    }

    #[test]
    fn upstream_renderer_rows_are_an_error() {
        only(vec![row("Id", "5")], "import-key");
        only(vec![row("From", "caster")], "import-key");
        only(vec![row("Save", "domain")], "import-key");
        assert!(problems(EntryKind::ClassOption, vec![row("Save", "Will negates")]).is_empty());
    }

    #[test]
    fn a_glued_modifier_in_usage_is_an_error() {
        only(
            vec![row("Usage", "Charisma3 modifier times/day")],
            "glued-modifier",
        );
        assert!(
            problems(
                EntryKind::ClassOption,
                vec![row("Usage", "3 + Charisma modifier times/day")]
            )
            .is_empty()
        );
        assert!(
            problems(
                EntryKind::ClassOption,
                vec![row("Special", "Charisma3 modifier")]
            )
            .is_empty()
        );
    }

    #[test]
    fn other_kinds_are_not_checked_and_the_line_names_the_row() {
        let body = vec![row("Br", "~")];
        assert!(problems(EntryKind::Rule, body.clone()).is_empty());
        let corpus = corpus(vec![option(EntryKind::ClassOption, body)]);
        let mut report = Report::default();
        check_class_option_residue(&corpus, &mut report);
        let line: usize = report.problems[0]
            .split("example.md:")
            .nth(1)
            .and_then(|rest| rest.split(':').next())
            .and_then(|digits| digits.parse().ok())
            .expect("a line number");
        let stored = &corpus.entries["ref/sorcbloodline/example"];
        assert_eq!(
            stored.text.lines().nth(line - 1),
            Some("**Br:** \\~"),
            "{}",
            stored.text
        );
    }
}
