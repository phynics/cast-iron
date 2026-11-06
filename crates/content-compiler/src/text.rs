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

/// Merges adjacent `Text` nodes and drops empty ones, in every inline run of the bodies.
///
/// The plain-text corpus has no spelling for a text-node boundary, so the compiled value
/// must not carry one. The merged text reads and searches the same.
pub fn normalize_text_nodes(blocks: &mut [Block]) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
                normalize_inlines(content);
            }
            Block::List { items, .. } => {
                for item in items {
                    normalize_text_nodes(item);
                }
            }
            Block::Table(table) => {
                normalize_inlines(&mut table.caption);
                for row in table
                    .head
                    .iter_mut()
                    .chain(&mut table.body)
                    .chain(&mut table.foot)
                {
                    for cell in &mut row.cells {
                        normalize_inlines(&mut cell.content);
                    }
                }
            }
            Block::Quote { content } => normalize_text_nodes(content),
            Block::StatBlock { rows, .. } => {
                for row in rows {
                    normalize_inlines(&mut row.value);
                }
            }
            Block::EntryLink { .. } => {}
        }
    }
}

fn normalize_inlines(inlines: &mut Vec<Inline>) {
    let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
    for mut inline in std::mem::take(inlines) {
        match &mut inline {
            Inline::Text { v } if v.is_empty() => continue,
            Inline::Text { v } => {
                if let Some(Inline::Text { v: previous }) = out.last_mut() {
                    previous.push_str(v);
                    continue;
                }
            }
            Inline::Em { c }
            | Inline::Strong { c }
            | Inline::Sup { c }
            | Inline::Sub { c }
            | Inline::Link { c, .. } => normalize_inlines(c),
            Inline::Break => {}
        }
        out.push(inline);
    }
    *inlines = out;
}

/// Lowercases and collapses whitespace, for comparisons.
pub fn squash(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{ParagraphStyle, TableCell};

    fn paragraph(content: Vec<Inline>) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }
    }

    #[test]
    fn adjacent_and_empty_text_nodes_are_normalized_everywhere() {
        let mut blocks = vec![
            paragraph(vec![
                Inline::text("1 "),
                Inline::text("night hag"),
                Inline::text(""),
                Inline::Em {
                    c: vec![Inline::text("a"), Inline::text("b")],
                },
                Inline::text(""),
                Inline::Break,
                Inline::text("x"),
            ]),
            Block::List {
                ordered: false,
                items: vec![vec![paragraph(vec![Inline::text("p"), Inline::text("q")])]],
            },
            Block::Table(Table {
                body: vec![TableRow {
                    cells: vec![TableCell {
                        header: false,
                        colspan: 1,
                        rowspan: 1,
                        content: vec![Inline::text("c"), Inline::text("d")],
                    }],
                }],
                ..Table::default()
            }),
        ];
        normalize_text_nodes(&mut blocks);
        assert_eq!(
            blocks[0],
            paragraph(vec![
                Inline::text("1 night hag"),
                Inline::Em {
                    c: vec![Inline::text("ab")]
                },
                Inline::Break,
                Inline::text("x"),
            ])
        );
        assert_eq!(
            blocks[1],
            Block::List {
                ordered: false,
                items: vec![vec![paragraph(vec![Inline::text("pq")])]]
            }
        );
        let Block::Table(table) = &blocks[2] else {
            panic!("table");
        };
        assert_eq!(table.body[0].cells[0].content, vec![Inline::text("cd")]);
    }

    #[test]
    fn normalization_keeps_the_plain_text_and_the_links() {
        let mut blocks = vec![paragraph(vec![
            Inline::text("see "),
            Inline::text(""),
            Inline::text("this"),
        ])];
        let before = blocks_text(&blocks);
        normalize_text_nodes(&mut blocks);
        assert_eq!(blocks_text(&blocks), before);
    }
}
