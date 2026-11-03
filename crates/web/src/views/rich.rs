//! Typed renderers for entry content. Every `Block` and `Inline` variant becomes a Leptos
//! node; nothing here builds markup from strings. Pure helpers sit at the top and have native
//! tests.

use crate::route::{self, Route};
use cast_iron_schema::{Block, Inline, ParagraphStyle, StatRow, Table, TableCell, TableRow};
use leptos::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

// ---- pure helpers -----------------------------------------------------------------------

/// The text of inline content, without markup. Breaks become a space.
pub fn plain_text(items: &[Inline]) -> String {
    let mut out = String::new();
    push_plain(items, &mut out);
    out
}

fn push_plain(items: &[Inline], out: &mut String) {
    for item in items {
        match item {
            Inline::Text { v } => out.push_str(v),
            Inline::Break => out.push(' '),
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                push_plain(c, out)
            }
            Inline::Link { c, .. } => push_plain(c, out),
        }
    }
}

/// Lowercase `a-z0-9` words joined by `-`. Empty when the text has no such characters.
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    out
}

/// The DOM id of a heading: the compiler's anchor, else a slug of its text.
pub fn heading_id(anchor: Option<&str>, content: &[Inline]) -> Option<String> {
    match anchor {
        Some(anchor) if !anchor.is_empty() => Some(anchor.to_string()),
        _ => {
            let slug = slugify(&plain_text(content));
            (!slug.is_empty()).then(|| format!("sec-{slug}"))
        }
    }
}

/// The `scope` of a header cell. `section` is `"head"`, `"body"`, or `"foot"`.
pub fn th_scope(section: &str, cell: &TableCell, index_in_row: usize) -> &'static str {
    if section == "body" && index_in_row == 0 {
        if cell.rowspan > 1 { "rowgroup" } else { "row" }
    } else if cell.colspan > 1 {
        "colgroup"
    } else {
        "col"
    }
}

fn span_attr(value: u16) -> Option<String> {
    (value > 1).then(|| value.to_string())
}

/// Counts the grid cells a table occupies, as `(cells, with_colspan_or_rowspan)`.
/// Used by tests to check the typed table against upstream counts.
pub fn table_stats(table: &Table) -> (usize, usize) {
    let rows = table.head.iter().chain(&table.body).chain(&table.foot);
    let cells: Vec<&TableCell> = rows.flat_map(|r| r.cells.iter()).collect();
    let spanning = cells
        .iter()
        .filter(|c| c.colspan > 1 || c.rowspan > 1)
        .count();
    (cells.len(), spanning)
}

// ---- inline -----------------------------------------------------------------------------

pub fn inlines(items: &[Inline]) -> Vec<AnyView> {
    items.iter().map(inline).collect()
}

fn inline(item: &Inline) -> AnyView {
    match item {
        Inline::Text { v } => v.clone().into_any(),
        Inline::Em { c } => view! { <em>{inlines(c)}</em> }.into_any(),
        Inline::Strong { c } => view! { <strong>{inlines(c)}</strong> }.into_any(),
        Inline::Sup { c } => view! { <sup>{inlines(c)}</sup> }.into_any(),
        Inline::Sub { c } => view! { <sub>{inlines(c)}</sub> }.into_any(),
        Inline::Break => view! { <br /> }.into_any(),
        Inline::Link { target, anchor, c } => {
            let href = Route::Entry {
                id: target.clone(),
                anchor: anchor.clone(),
            }
            .href();
            view! { <a class="xref" href=href>{inlines(c)}</a> }.into_any()
        }
    }
}

// ---- blocks -----------------------------------------------------------------------------

/// Renders blocks in order. Consecutive entry links share one list.
pub fn blocks(items: &[Block]) -> Vec<AnyView> {
    let mut out = Vec::with_capacity(items.len());
    let mut run: Vec<AnyView> = Vec::new();
    for item in items {
        if let Block::EntryLink { target, title } = item {
            let href = Route::entry(target.clone()).href();
            run.push(view! { <li><a href=href>{title.clone()}</a></li> }.into_any());
            continue;
        }
        if !run.is_empty() {
            let list = std::mem::take(&mut run);
            out.push(view! { <ul class="entry-links">{list}</ul> }.into_any());
        }
        out.push(block(item));
    }
    if !run.is_empty() {
        out.push(view! { <ul class="entry-links">{run}</ul> }.into_any());
    }
    out
}

fn block(item: &Block) -> AnyView {
    match item {
        Block::Heading {
            level,
            anchor,
            content,
        } => heading(
            *level,
            heading_id(anchor.as_deref(), content),
            inlines(content),
        ),
        Block::Paragraph { style, content } => {
            let class = match style {
                ParagraphStyle::Normal => None,
                ParagraphStyle::StatLine => Some("stat-line"),
                ParagraphStyle::Indented => Some("indented"),
            };
            view! { <p class=class>{inlines(content)}</p> }.into_any()
        }
        Block::List { ordered, items } => {
            let rows: Vec<AnyView> = items
                .iter()
                .map(|cell| view! { <li>{blocks(cell)}</li> }.into_any())
                .collect();
            if *ordered {
                view! { <ol>{rows}</ol> }.into_any()
            } else {
                view! { <ul>{rows}</ul> }.into_any()
            }
        }
        Block::Table(table) => rules_table(table),
        Block::Quote { content } => view! { <blockquote>{blocks(content)}</blockquote> }.into_any(),
        Block::StatBlock { title, rows } => stat_block(title.as_deref(), rows),
        Block::EntryLink { target, title } => {
            let href = Route::entry(target.clone()).href();
            view! { <ul class="entry-links"><li><a href=href>{title.clone()}</a></li></ul> }
                .into_any()
        }
    }
}

fn heading(level: u8, id: Option<String>, content: Vec<AnyView>) -> AnyView {
    match level {
        0..=2 => view! { <h2 id=id>{content}</h2> }.into_any(),
        3 => view! { <h3 id=id>{content}</h3> }.into_any(),
        4 => view! { <h4 id=id>{content}</h4> }.into_any(),
        5 => view! { <h5 id=id>{content}</h5> }.into_any(),
        _ => view! { <h6 id=id>{content}</h6> }.into_any(),
    }
}

static UNIQUE: AtomicU32 = AtomicU32::new(0);

fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", UNIQUE.fetch_add(1, Ordering::Relaxed))
}

fn stat_block(title: Option<&str>, rows: &[StatRow]) -> AnyView {
    let title_id = title.map(|_| unique("stat"));
    let heading = title.map(|t| {
        let id = title_id.clone();
        view! { <p class="stat-title" id=id>{t.to_string()}</p> }
    });
    let rows: Vec<AnyView> = rows
        .iter()
        .map(|row| {
            view! {
                <div class="stat-row">
                    <dt>{row.label.clone()}</dt>
                    <dd>{inlines(&row.value)}</dd>
                </div>
            }
            .into_any()
        })
        .collect();
    view! {
        <div class="stat-block">
            {heading}
            <dl aria-labelledby=title_id>{rows}</dl>
        </div>
    }
    .into_any()
}

fn rules_table(table: &Table) -> AnyView {
    let caption_id = unique("cap");
    let has_caption = !table.caption.is_empty();
    let caption = has_caption.then(|| {
        view! { <caption id=caption_id.clone()>{inlines(&table.caption)}</caption> }
    });
    let label = (!has_caption).then_some("Table");
    let labelled_by = has_caption.then(|| caption_id.clone());
    let head = (!table.head.is_empty()).then(|| {
        view! { <thead>{table_rows(&table.head, "head")}</thead> }
    });
    let body = view! { <tbody>{table_rows(&table.body, "body")}</tbody> };
    let foot = (!table.foot.is_empty()).then(|| {
        view! { <tfoot>{table_rows(&table.foot, "foot")}</tfoot> }
    });
    view! {
        <div class="table-wrap" role="region" tabindex="0" aria-labelledby=labelled_by aria-label=label>
            <table class="rules-table">{caption}{head}{body}{foot}</table>
        </div>
    }
    .into_any()
}

fn table_rows(rows: &[TableRow], section: &'static str) -> Vec<AnyView> {
    rows.iter()
        .map(|row| {
            let cells: Vec<AnyView> = row
                .cells
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let colspan = span_attr(cell.colspan);
                    let rowspan = span_attr(cell.rowspan);
                    if cell.header {
                        let scope = th_scope(section, cell, i);
                        view! {
                            <th scope=scope colspan=colspan rowspan=rowspan>
                                {inlines(&cell.content)}
                            </th>
                        }
                        .into_any()
                    } else {
                        view! {
                            <td colspan=colspan rowspan=rowspan>{inlines(&cell.content)}</td>
                        }
                        .into_any()
                    }
                })
                .collect();
            view! { <tr>{cells}</tr> }.into_any()
        })
        .collect()
}

// ---- body ---------------------------------------------------------------------------------

/// The body of an entry. The longest entry in the POC library (about 420 blocks, with a table
/// of 340 cells) renders in under 40 ms, and 60 ms with the CPU throttled 4x, so the whole body
/// renders at once. With an anchor, the view scrolls to it after the first frame.
#[component]
pub fn RichBody(body: Vec<Block>, anchor: Option<String>) -> impl IntoView {
    let content = blocks(&body);
    if let Some(anchor) = anchor {
        request_animation_frame(move || {
            route::scroll_to_anchor(&anchor);
        });
    }
    view! { <div class="rich">{content}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::EntryId;

    fn text(s: &str) -> Inline {
        Inline::text(s)
    }

    #[test]
    fn plain_text_flattens_every_inline() {
        let items = vec![
            text("A "),
            Inline::Em {
                c: vec![text("fire")],
            },
            Inline::Break,
            Inline::Strong {
                c: vec![text("ball")],
            },
            Inline::Sup { c: vec![text("2")] },
            Inline::Sub { c: vec![text("x")] },
            Inline::Link {
                target: EntryId::parse("a/b").unwrap(),
                anchor: None,
                c: vec![text("!")],
            },
        ];
        assert_eq!(plain_text(&items), "A fire ball2x!");
    }

    #[test]
    fn slugify_keeps_words() {
        assert_eq!(slugify("Spell Resistance (SR)"), "spell-resistance-sr");
        assert_eq!(slugify("  --  "), "");
        assert_eq!(slugify("Élan 5"), "lan-5");
    }

    #[test]
    fn heading_id_prefers_the_anchor() {
        let c = vec![text("Big Table")];
        assert_eq!(heading_id(Some("big"), &c).as_deref(), Some("big"));
        assert_eq!(heading_id(None, &c).as_deref(), Some("sec-big-table"));
        assert_eq!(heading_id(Some(""), &c).as_deref(), Some("sec-big-table"));
        assert_eq!(heading_id(None, &[text("!!")]), None);
    }

    #[test]
    fn th_scope_follows_position_and_span() {
        let cell = |colspan, rowspan| TableCell {
            header: true,
            colspan,
            rowspan,
            content: vec![],
        };
        assert_eq!(th_scope("head", &cell(1, 1), 0), "col");
        assert_eq!(th_scope("head", &cell(3, 1), 1), "colgroup");
        assert_eq!(th_scope("body", &cell(1, 1), 0), "row");
        assert_eq!(th_scope("body", &cell(1, 2), 0), "rowgroup");
        assert_eq!(th_scope("body", &cell(1, 1), 2), "col");
    }

    #[test]
    fn table_stats_count_spans() {
        let plain = TableCell {
            content: vec![text("a")],
            colspan: 1,
            rowspan: 1,
            header: false,
        };
        let span = TableCell {
            colspan: 2,
            ..plain.clone()
        };
        let table = Table {
            caption: vec![],
            head: vec![TableRow {
                cells: vec![span.clone()],
            }],
            body: vec![TableRow {
                cells: vec![plain.clone(), plain],
            }],
            foot: vec![],
        };
        assert_eq!(table_stats(&table), (3, 1));
    }
}
