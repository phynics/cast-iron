//! Plain-text projections of typed content, for search documents and tests.

use cast_iron_schema::{Block, Inline, Table, TableRow};

pub fn inline_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    push_inlines(inlines, &mut out);
    out
}

fn push_inlines(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { v } => out.push_str(v),
            Inline::Break => out.push(' '),
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                push_inlines(c, out)
            }
            Inline::Link { c, .. } => push_inlines(c, out),
        }
    }
}

/// Joins blocks with newlines.
pub fn blocks_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    push_blocks(blocks, &mut out);
    out
}

fn line(out: &mut String) {
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
}

fn push_blocks(blocks: &[Block], out: &mut String) {
    for block in blocks {
        line(out);
        match block {
            Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
                push_inlines(content, out)
            }
            Block::List { items, .. } => {
                for item in items {
                    line(out);
                    push_blocks(item, out);
                }
            }
            Block::Table(table) => push_table(table, out),
            Block::Quote { content } => push_blocks(content, out),
            Block::StatBlock { title, rows } => {
                if let Some(title) = title {
                    out.push_str(title);
                    out.push('\n');
                }
                for row in rows {
                    out.push_str(&row.label);
                    out.push_str(": ");
                    push_inlines(&row.value, out);
                    out.push('\n');
                }
            }
            Block::EntryLink { title, .. } => out.push_str(title),
        }
    }
}

fn push_table(table: &Table, out: &mut String) {
    push_inlines(&table.caption, out);
    for rows in [&table.head, &table.body, &table.foot] {
        for row in rows {
            push_row(row, out);
        }
    }
}

fn push_row(row: &TableRow, out: &mut String) {
    line(out);
    for (i, cell) in row.cells.iter().enumerate() {
        if i > 0 {
            out.push_str(" | ");
        }
        push_inlines(&cell.content, out);
    }
}

/// Lowercases and collapses whitespace, for comparisons.
pub fn squash(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
