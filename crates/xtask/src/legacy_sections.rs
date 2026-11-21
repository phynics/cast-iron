//! The table of old section addresses (`sources/legacy-sections.json`, `LegacySections`).
//!
//! Libraries up to corpus v0.1.31 named a row of a chapter page by its position, `toc-2-26`. A
//! chapter page now names a row by its identity, so those addresses need a table: for each row
//! of the last published tree, its positional anchor and its stable anchor. This command builds
//! the table once from that published tree. The table is frozen, because no later library makes a
//! positional anchor, so a pin never needs it regenerated: the compiler writes the committed file
//! into every library and the client resolves the stable anchor against the tree it has.
//!
//! `cargo xtask legacy-sections --books <books.json> --catalog <catalog.json> [--check]` reads the
//! `books.json` and `catalog.json` of the published library (the catalog says which kinds
//! Contents hid, which decides the rows the page rendered) and writes the table. `--check` writes
//! nothing and fails when the committed file differs.

use crate::{Result, root};
use cast_iron_schema::books::{chapter_rows, sections, toc_id};
use cast_iron_schema::{
    BookIndex, Catalog, EntryId, EntryKind, LEGACY_SECTIONS_FORMAT, LegacySections,
};
use std::collections::BTreeMap;
use std::fs;

/// The committed table, relative to the repository root.
pub const TABLE: &str = "sources/legacy-sections.json";

/// The table for a published library: every row that a chapter page rendered, as its positional
/// anchor and its stable anchor, and every top-level Contents row, as its positional anchor and
/// its chapter id.
pub fn build(books: &BookIndex, catalog: &Catalog) -> LegacySections {
    let kinds: BTreeMap<&EntryId, EntryKind> = catalog
        .entries
        .iter()
        .map(|stub| (&stub.id, stub.kind))
        .collect();
    let hide = |id: &EntryId| {
        kinds
            .get(id)
            .is_some_and(|kind| cast_iron_schema::books::hidden_from_contents(*kind))
    };
    let mut table = LegacySections {
        format: LEGACY_SECTIONS_FORMAT,
        books: BTreeMap::new(),
        chapters: BTreeMap::new(),
    };
    for book in &books.books {
        let rows: BTreeMap<String, String> = sections(book, &hide)
            .into_iter()
            .filter_map(|section| Some((toc_id(&section.path), section.anchor?)))
            .collect();
        if !rows.is_empty() {
            table.books.insert(book.source.clone(), rows);
        }
        // The top-level Contents rows, which `?c=3` names and `sections` does not list.
        let chapters: BTreeMap<String, String> = chapter_rows(book, &hide)
            .into_iter()
            .map(|row| (toc_id(&row.path), row.id))
            .collect();
        if !chapters.is_empty() {
            table.chapters.insert(book.source.clone(), chapters);
        }
    }
    table
}

/// The file bytes of a table: pretty JSON, one row per line, with a trailing newline.
pub fn render(table: &LegacySections) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(table).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn read(path: &str) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))
}

pub fn run(args: &[String]) -> Result {
    let (mut books, mut catalog, mut check) = (None, None, false);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--books" => books = args.next().cloned(),
            "--catalog" => catalog = args.next().cloned(),
            "--check" => check = true,
            other => {
                return Err(format!(
                    "unknown argument {other:?} for legacy-sections\n\
                     usage: legacy-sections --books <books.json> --catalog <catalog.json> [--check]"
                ));
            }
        }
    }
    let (Some(books), Some(catalog)) = (books, catalog) else {
        return Err("legacy-sections needs --books and --catalog".into());
    };
    let books: BookIndex =
        serde_json::from_slice(&read(&books)?).map_err(|e| format!("cannot parse {books}: {e}"))?;
    let catalog: Catalog = serde_json::from_slice(&read(&catalog)?)
        .map_err(|e| format!("cannot parse {catalog}: {e}"))?;
    let table = build(&books, &catalog);
    let bytes = render(&table)?;
    let path = root().join(TABLE);
    if check {
        let committed = fs::read(&path).map_err(|e| format!("cannot read {TABLE}: {e}"))?;
        return if committed == bytes {
            Ok(())
        } else {
            Err(format!(
                "{TABLE} differs from the table of the library given"
            ))
        };
    }
    fs::write(&path, &bytes).map_err(|e| format!("cannot write {TABLE}: {e}"))?;
    let rows: usize = table.books.values().map(BTreeMap::len).sum();
    let chapters: usize = table.chapters.values().map(BTreeMap::len).sum();
    println!(
        "wrote {TABLE}: {rows} rows and {chapters} chapter rows in {} books, {} bytes",
        table.books.len(),
        bytes.len()
    );
    Ok(())
}
