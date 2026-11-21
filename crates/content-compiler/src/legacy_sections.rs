//! The legacy section table (`sources/legacy-sections.json`): old positional section addresses
//! that a published library made, and the stable anchor of the row each one stood for.
//!
//! The table is frozen. Libraries up to corpus v0.1.31 named a chapter-page row `toc-<path>`; a
//! pin that adds or removes a row moves every later path, so those addresses are resolved here
//! and not against the current tree. No later library makes a positional anchor, so no pin adds
//! to the file, and the stable anchors it holds do not move when a pin changes the tree
//! (`cast_iron_schema::books::section_anchors`): a row that stays in its book keeps its old
//! address working, and a row that left opens its chapter. `cargo xtask legacy-sections` made the
//! file from the published tree, once. The build writes the bytes into every library as
//! `legacy-sections.json` and reports how many rows still find their row.

use cast_iron_schema::books::{
    SECTION_ANCHOR_PREFIX, hidden_from_contents, legacy_anchor_path, locate_section,
};
use cast_iron_schema::{
    BookIndex, EntryId, EntryKind, LEGACY_SECTIONS_FORMAT, LegacySections, RuleEntry,
};
use std::collections::BTreeMap;

/// The table in `bytes`, or one line per problem: an unreadable file, an unknown format, a key
/// that is not a positional anchor, or a value that is not a stable anchor.
pub fn parse(bytes: &[u8]) -> Result<LegacySections, Vec<String>> {
    let table: LegacySections = serde_json::from_slice(bytes)
        .map_err(|e| vec![format!("legacy-sections.json: cannot parse: {e}")])?;
    let mut problems = Vec::new();
    if table.format != LEGACY_SECTIONS_FORMAT {
        problems.push(format!(
            "legacy-sections.json: format {} is not {LEGACY_SECTIONS_FORMAT}",
            table.format
        ));
    }
    for (book, rows) in &table.books {
        for (old, new) in rows {
            if legacy_anchor_path(old).is_none() {
                problems.push(format!(
                    "legacy-sections.json: {book}: {old:?} is not a positional anchor"
                ));
            }
            if !new.starts_with(SECTION_ANCHOR_PREFIX) {
                problems.push(format!(
                    "legacy-sections.json: {book}: {old} maps to {new:?}, which is not a stable anchor"
                ));
            }
        }
    }
    if problems.is_empty() {
        Ok(table)
    } else {
        Err(problems)
    }
}

/// How many rows of the table there are, and how many still name a row of the current books.
pub fn coverage(
    table: &LegacySections,
    books: &BookIndex,
    entries: &[RuleEntry],
) -> (usize, usize) {
    let kinds: BTreeMap<&EntryId, EntryKind> = entries
        .iter()
        .map(|entry| (&entry.id, entry.kind))
        .collect();
    let hide = |id: &EntryId| {
        kinds
            .get(id)
            .is_some_and(|kind| hidden_from_contents(*kind))
    };
    let (mut rows, mut found) = (0, 0);
    for (source, table_rows) in &table.books {
        rows += table_rows.len();
        let Some(book) = books.books.iter().find(|book| &book.source == source) else {
            continue;
        };
        found += table_rows
            .values()
            .filter(|anchor| locate_section(book, &hide, anchor, None).is_some())
            .count();
    }
    (rows, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    #[test]
    fn the_committed_table_parses_and_maps_the_published_ranger_row() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../sources/legacy-sections.json"
        ))
        .unwrap();
        let table = parse(&bytes).unwrap_or_else(|problems| panic!("{}", problems.join("\n")));
        // v0.1.31 (and the unreleased v0.1.32) had Ranger at toc-2-26 and Rogue at toc-2-27 on
        // this page; v0.1.33 moved Rogue to toc-2-26. The table keeps the published meaning.
        let book = &table.books["blood-of-the-beast"];
        assert_eq!(book["toc-2-26"], "row-ref-class-ranger");
        assert_eq!(book["toc-2-27"], "row-ref-class-rogue");
    }

    #[test]
    fn a_malformed_table_names_each_problem() {
        let problems = parse(&table(
            r#"{"format":1,"books":{"b":{"toc-1-2":"row-x","oops":"row-y","toc-1":"z"}}}"#,
        ))
        .unwrap_err();
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(
            problems[0].contains("not a positional anchor"),
            "{problems:?}"
        );
        assert!(problems[1].contains("not a stable anchor"), "{problems:?}");
        let wrong = parse(&table(r#"{"format":2,"books":{}}"#)).unwrap_err();
        assert!(wrong[0].contains("format 2"), "{wrong:?}");
        assert!(parse(b"not json").is_err());
    }
}
