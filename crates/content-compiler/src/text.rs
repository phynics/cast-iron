//! Plain-text projections of typed content, for search documents and tests.

use cast_iron_schema::{Block, EntryReference, Inline, Table, TableRow};
use std::collections::BTreeSet;

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

/// Every internal reference a final body holds, deduplicated and sorted.
///
/// The link list is always a projection of the body, never an independent field. A
/// converter, policy pass, or settlement step that removes body content must not
/// leave a reference behind; callers recompute the list after the last body change.
pub fn collect_links(blocks: &[Block]) -> Vec<EntryReference> {
    let mut links = BTreeSet::new();
    collect_block_links(blocks, &mut links);
    links.into_iter().collect()
}

fn collect_block_links(blocks: &[Block], links: &mut BTreeSet<EntryReference>) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
                collect_inline_links(content, links);
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_block_links(item, links);
                }
            }
            Block::Table(table) => {
                collect_inline_links(&table.caption, links);
                for cell in table
                    .head
                    .iter()
                    .chain(&table.body)
                    .chain(&table.foot)
                    .flat_map(|row| &row.cells)
                {
                    collect_inline_links(&cell.content, links);
                }
            }
            Block::Quote { content } => collect_block_links(content, links),
            Block::StatBlock { rows, .. } => {
                for row in rows {
                    collect_inline_links(&row.value, links);
                }
            }
            Block::EntryLink { target, .. } => {
                links.insert(EntryReference {
                    target: target.clone(),
                    anchor: None,
                });
            }
        }
    }
}

fn collect_inline_links(inlines: &[Inline], links: &mut BTreeSet<EntryReference>) {
    for inline in inlines {
        match inline {
            Inline::Link { target, anchor, c } => {
                links.insert(EntryReference {
                    target: target.clone(),
                    anchor: anchor.clone(),
                });
                collect_inline_links(c, links);
            }
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                collect_inline_links(c, links);
            }
            Inline::Text { .. } | Inline::Break => {}
        }
    }
}

/// Lowercases and collapses whitespace, for comparisons.
pub fn squash(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
