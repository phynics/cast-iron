//! Splits a class progression table into the columns every class has and the spell columns.
//!
//! A printed class table carries Level, Base Attack Bonus, three saves and Special, and then
//! the spells per day of a caster, up to ten columns wide. On a phone the spell columns push
//! the saves off screen. The split works on the typed table: it reads the header cells and
//! moves the columns whose header names a spell level, or `Spells Per Day`, into a second
//! table. Level stays in both tables as the row key. The data is not changed.

use cast_iron_schema::{Inline, Table, TableCell, TableRow, headings::plain_text};

/// The two tables a progression splits into.
#[derive(Debug, Clone, PartialEq)]
pub struct Progression {
    /// Level, Base Attack Bonus, the saves, Special, and any other class column.
    pub core: Table,
    /// Level and the spell columns. `None` for a class without spell columns.
    pub spells: Option<Table>,
}

/// The one prestige-class column that holds spellcasting progression as text.
const SPELLS_PER_DAY: &str = "spells per day";

/// Whether a header names a spell column: a spell level (a bare `0`–`9`, or an ordinal such as
/// `1st`, `2nd`, `9th`) or the single `Spells Per Day` column of a prestige class. The corpus
/// spells that one both `Spells Per Day` and `Spells per Day`, so the match ignores case; do not
/// make it case-sensitive. Any other header, including a number above 9, is not a spell column.
pub fn is_spell_header(header: &str) -> bool {
    let header = header.trim().to_lowercase();
    if header == SPELLS_PER_DAY {
        return true;
    }
    let digits = header.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &header[digits.len()..];
    let numeric = !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit());
    match suffix {
        "" => numeric && digits.len() == 1,
        "st" | "nd" | "rd" | "th" => numeric,
        _ => false,
    }
}

/// The wording of the disclosure for a spell table, and of the table's region: slots by spell
/// level read `Spells per day`; a prestige class's single `Spells Per Day` column reads
/// `Spellcasting`, so the disclosure does not repeat the column header under it.
pub fn spells_labels(spells: &Table) -> (&'static str, &'static str) {
    let single = spells.head.first().is_some_and(|head| {
        matches!(head.cells.as_slice(), [_, only] if plain_text(&only.content).trim().to_lowercase() == SPELLS_PER_DAY)
    });
    if single {
        ("Spellcasting", "Spellcasting by level")
    } else {
        ("Spells per day", "Spells per day by level")
    }
}

/// The parts of an iterative attack bonus such as `+11/+6/+1`, each part keeping its trailing
/// slash, so a narrow cell can break after a slash. `None` for any other text.
pub fn attack_bonus_parts(text: &str) -> Option<Vec<&str>> {
    let numeric = text.contains('/')
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '/'));
    numeric.then(|| text.split_inclusive('/').collect())
}

fn header_texts(row: &TableRow) -> Vec<String> {
    row.cells
        .iter()
        .map(|cell| plain_text(&cell.content))
        .collect()
}

fn is_plain(row: &TableRow) -> bool {
    row.cells.iter().all(|c| c.colspan == 1 && c.rowspan == 1)
}

/// Reads `table` as a class progression, or returns `None` when it is not one. A progression
/// has a single header row that starts with `Level` and has `Base Attack Bonus`, and every
/// row has one plain cell per column. Its spell columns, if any, move to `spells`; every
/// other column stays in `core`.
pub fn split(table: &Table) -> Option<Progression> {
    let [head] = table.head.as_slice() else {
        return None;
    };
    let names = header_texts(head);
    let width = names.len();
    let level_first = names.first().is_some_and(|n| n.trim() == "Level");
    let has_bab = names.iter().any(|n| n.trim() == "Base Attack Bonus");
    let rows = || table.head.iter().chain(&table.body).chain(&table.foot);
    if !level_first
        || !has_bab
        || !rows().all(|r| is_plain(r) && r.cells.len() == width)
        || !table.foot.is_empty()
    {
        return None;
    }
    // Column 0 is the row key and always stays; a spell column is any other column.
    let spell_cols: Vec<usize> = (1..width).filter(|&i| is_spell_header(&names[i])).collect();
    let keep = |cols: &dyn Fn(usize) -> bool| -> Table {
        let pick = |row: &TableRow, body: bool| TableRow {
            cells: row
                .cells
                .iter()
                .enumerate()
                .filter(|(i, _)| *i == 0 || cols(*i))
                .map(|(i, cell)| TableCell {
                    // The level label names its row, so it reads as a row header.
                    header: cell.header || (body && i == 0),
                    ..cell.clone()
                })
                .collect(),
        };
        Table {
            caption: Vec::<Inline>::new(),
            head: table.head.iter().map(|r| pick(r, false)).collect(),
            body: table.body.iter().map(|r| pick(r, true)).collect(),
            foot: Vec::new(),
        }
    };
    let is_spell = |i: usize| spell_cols.contains(&i);
    let mut core = keep(&|i| !is_spell(i));
    core.caption = table.caption.clone();
    Some(Progression {
        core,
        spells: (!spell_cols.is_empty()).then(|| keep(&is_spell)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(text: &str, header: bool) -> TableCell {
        TableCell {
            header,
            colspan: 1,
            rowspan: 1,
            content: vec![Inline::text(text)],
        }
    }

    fn row(texts: &[&str], header: bool) -> TableRow {
        TableRow {
            cells: texts.iter().map(|t| cell(t, header)).collect(),
        }
    }

    fn table(head: &[&str], body: &[&[&str]]) -> Table {
        Table {
            caption: vec![],
            head: vec![row(head, true)],
            body: body.iter().map(|r| row(r, false)).collect(),
            foot: vec![],
        }
    }

    fn column(table: &Table, index: usize) -> Vec<String> {
        table
            .head
            .iter()
            .chain(&table.body)
            .map(|r| plain_text(&r.cells[index].content))
            .collect()
    }

    const CORE: [&str; 6] = [
        "Level",
        "Base Attack Bonus",
        "Fort Save",
        "Ref Save",
        "Will Save",
        "Special",
    ];

    fn head_of(extra: &[&str]) -> Vec<String> {
        CORE.iter().chain(extra).map(|s| s.to_string()).collect()
    }

    fn wizard_like() -> Table {
        let head = head_of(&["0", "1st", "2nd"]);
        let head: Vec<&str> = head.iter().map(String::as_str).collect();
        table(
            &head,
            &[
                &["1st", "+0", "+0", "+0", "+2", "Arcane bond", "3", "1", "-"],
                &["2nd", "+1", "+0", "+0", "+3", "Cantrips", "4", "2", "-"],
            ],
        )
    }

    #[test]
    fn spell_headers_are_levels_and_the_prestige_column() {
        for yes in [
            "0",
            "9",
            "1st",
            "2nd",
            "3rd",
            "9th",
            "10th",
            " 4th ",
            "Spells Per Day",
            "Spells per Day",
        ] {
            assert!(is_spell_header(yes), "{yes}");
        }
        for no in [
            "Level",
            "Special",
            "Unarmed Damage",
            "AC Bonus",
            "Sacred Weapon Damage",
            "1st-level",
            "th",
            "",
            "Spells",
            "10",
            "Spells Known",
            "Cantrips",
        ] {
            assert!(!is_spell_header(no), "{no}");
        }
    }

    #[test]
    fn a_caster_splits_into_core_and_spell_columns_with_level_in_both() {
        let split = split(&wizard_like()).expect("a progression splits");
        let spells = split.spells.as_ref().expect("a caster has spell columns");
        assert_eq!(column(&split.core, 0), ["Level", "1st", "2nd"]);
        assert_eq!(split.core.head[0].cells.len(), 6);
        assert_eq!(
            column(&split.core, 5),
            ["Special", "Arcane bond", "Cantrips"]
        );
        assert_eq!(column(spells, 0), ["Level", "1st", "2nd"]);
        assert_eq!(spells.head[0].cells.len(), 4);
        assert_eq!(column(spells, 1), ["0", "3", "4"]);
        assert_eq!(column(spells, 3), ["2nd", "-", "-"]);
    }

    #[test]
    fn the_level_cell_of_a_body_row_becomes_a_row_header() {
        let split = split(&wizard_like()).unwrap();
        for table in [&split.core, split.spells.as_ref().unwrap()] {
            assert!(table.body.iter().all(|r| r.cells[0].header));
            assert!(table.body.iter().all(|r| !r.cells[1].header));
        }
    }

    #[test]
    fn extra_class_columns_stay_in_the_core_table() {
        let head = head_of(&["Unarmed Damage", "0", "1st"]);
        let head: Vec<&str> = head.iter().map(String::as_str).collect();
        let t = table(
            &head,
            &[&["1st", "+0", "+2", "+2", "+2", "Bonus feat", "1d6", "1", "-"]],
        );
        let split = split(&t).unwrap();
        let spells = split.spells.as_ref().unwrap();
        assert_eq!(column(&split.core, 6), ["Unarmed Damage", "1d6"]);
        assert_eq!(column(spells, 1), ["0", "1"]);
    }

    #[test]
    fn a_prestige_class_spells_per_day_column_moves_as_one_column() {
        let head = head_of(&["Spells Per Day"]);
        let head: Vec<&str> = head.iter().map(String::as_str).collect();
        let t = table(
            &head,
            &[&["1st", "+0", "+0", "+0", "+1", "Hex", "+1 level"]],
        );
        let split = split(&t).unwrap();
        let spells = split.spells.as_ref().unwrap();
        assert_eq!(spells.head[0].cells.len(), 2);
        assert_eq!(column(spells, 1), ["Spells Per Day", "+1 level"]);
    }

    #[test]
    fn an_unknown_header_in_a_progression_stays_in_the_core_table() {
        // A column the split does not know, such as a new `Spells Known`, is never dropped and
        // never moved: it stays beside Special, and the spell table is what it was.
        let head = head_of(&["Spells Known", "0", "1st"]);
        let head: Vec<&str> = head.iter().map(String::as_str).collect();
        let body: &[&[&str]] = &[&["1st", "+0", "+0", "+0", "+2", "Hex", "4", "3", "1"]];
        let t = table(&head, body);
        let split = split(&t).expect("still a progression");
        let spells = split.spells.as_ref().unwrap();
        assert_eq!(column(&split.core, 6), ["Spells Known", "4"]);
        assert_eq!(split.core.head[0].cells.len(), 7);
        assert_eq!(column(spells, 0), ["Level", "1st"]);
        assert_eq!(column(spells, 1), ["0", "3"]);
        assert_eq!(column(spells, 2), ["1st", "1"]);
        // Every printed column is in exactly one table, except Level, which is in both.
        let cells = |t: &Table| t.head[0].cells.len() + t.body[0].cells.len();
        assert_eq!(cells(&split.core) + cells(spells) - 2, cells(&t));
    }

    #[test]
    fn a_table_without_any_known_column_shape_renders_unchanged() {
        // No Level-first header, so there is no split at all and the whole table is kept.
        let t = table(
            &["Spells Known", "Level", "Base Attack Bonus"],
            &[&["x", "1st", "+1"]],
        );
        assert_eq!(split(&t), None);
    }

    #[test]
    fn the_disclosure_is_worded_by_what_it_holds() {
        let slots = split(&wizard_like()).unwrap().spells.unwrap();
        assert_eq!(
            spells_labels(&slots),
            ("Spells per day", "Spells per day by level")
        );
        let head = head_of(&["Spells per Day"]);
        let head: Vec<&str> = head.iter().map(String::as_str).collect();
        let t = table(
            &head,
            &[&["1st", "+0", "+0", "+0", "+1", "Hex", "+1 level"]],
        );
        let single = split(&t).unwrap().spells.unwrap();
        assert_eq!(
            spells_labels(&single),
            ("Spellcasting", "Spellcasting by level")
        );
    }

    #[test]
    fn an_iterative_attack_bonus_breaks_after_each_slash() {
        assert_eq!(
            attack_bonus_parts("+16/+11/+6/+1"),
            Some(vec!["+16/", "+11/", "+6/", "+1"])
        );
        for no in ["+16", "Bonus feat", "1d6/day", "", "+1 / +2"] {
            assert_eq!(attack_bonus_parts(no), None, "{no}");
        }
    }

    #[test]
    fn a_non_caster_is_a_progression_without_a_spell_table() {
        let head = head_of(&[]);
        let head: Vec<&str> = head.iter().map(String::as_str).collect();
        let t = table(&head, &[&["1st", "+1", "+2", "+0", "+0", "Bonus feat"]]);
        let split = split(&t).expect("a non-caster is still a progression");
        assert_eq!(split.spells, None);
        assert_eq!(split.core.head[0].cells.len(), 6);
        assert!(split.core.body[0].cells[0].header);
    }

    #[test]
    fn other_tables_do_not_split() {
        // A spells-known table has Level and slots but no Base Attack Bonus.
        let t = table(&["Level", "0", "1st"], &[&["1st", "4", "2"]]);
        assert_eq!(split(&t), None);
        // The level column must lead.
        let head = head_of(&["1st"]);
        let mut shifted: Vec<&str> = head.iter().map(String::as_str).collect();
        shifted.swap(0, 5);
        let t = table(&shifted, &[&["x", "+1", "+2", "+0", "+0", "1st", "1"]]);
        assert_eq!(split(&t), None);
    }

    #[test]
    fn ragged_or_spanning_tables_do_not_split() {
        let mut t = wizard_like();
        t.body[1].cells.pop();
        assert_eq!(split(&t), None);
        let mut t = wizard_like();
        t.body[0].cells[5].colspan = 2;
        assert_eq!(split(&t), None);
    }

    #[test]
    fn the_caption_stays_with_the_core_table() {
        let mut t = wizard_like();
        t.caption = vec![Inline::text("Table: The Wizard")];
        let split = split(&t).unwrap();
        assert_eq!(plain_text(&split.core.caption), "Table: The Wizard");
        assert!(split.spells.unwrap().caption.is_empty());
    }
}
