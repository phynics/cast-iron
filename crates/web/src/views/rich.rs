//! Typed renderers for entry content. Every `Block` and `Inline` variant becomes a Leptos
//! node; nothing here builds markup from strings. Pure helpers sit at the top and have native
//! tests.

use crate::route::{self, Route};
use cast_iron_schema::{Block, Inline, ParagraphStyle, StatRow, Table, TableCell, TableRow};
use leptos::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU32, Ordering};

// ---- pure helpers -----------------------------------------------------------------------

pub use cast_iron_schema::headings::{
    RELATED_ANCHOR, RESERVED_ANCHOR_KEYS, heading_keys, headings_in_order, keyed_headings,
    plain_text, slugify,
};

/// The DOM id for a heading key. The prefix keeps content ids away from app ids.
pub fn heading_dom_id(key: &str) -> String {
    format!("h-{key}")
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

/// A field label such as `Benefit:` reads as the typed field name `benefit`.
fn field_label(items: &[Inline]) -> String {
    plain_text(items)
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_lowercase()
}

/// Inlines with the quiet `changed` label after a stat label that a revision changed. The
/// printed feat body puts its fields in one paragraph as bold labels, so this is where a
/// revised page attaches the marker.
fn inlines_with_changed(items: &[Inline], changed: &BTreeSet<String>) -> Vec<AnyView> {
    items
        .iter()
        .map(|item| {
            let node = inline(item);
            let marked = matches!(item, Inline::Strong { c } if changed.contains(&field_label(c)));
            if marked {
                view! {
                    {node}
                    <span class="tag changed-tag">"changed"</span>
                }
                .into_any()
            } else {
                node
            }
        })
        .collect()
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
                book: None,
            }
            .href();
            view! { <a class="xref" href=href>{inlines(c)}</a> }.into_any()
        }
    }
}

// ---- blocks -----------------------------------------------------------------------------

/// Renders blocks in order. Consecutive entry links share one list.
pub fn blocks(items: &[Block]) -> Vec<AnyView> {
    Renderer::new(items).blocks(items)
}

/// Renders a body. Each heading finds its key by identity in the table that [`keyed_headings`]
/// built from the same slice, so the ids cannot drift from the keys whatever order it renders in.
struct Renderer {
    keys: BTreeMap<*const Block, Option<String>>,
    /// Lowercase labels of the stat rows that a revision changed.
    changed: BTreeSet<String>,
}

impl Renderer {
    fn new(body: &[Block]) -> Self {
        let keys = keyed_headings(body)
            .into_iter()
            .map(|(block, key)| (std::ptr::from_ref(block), key))
            .collect();
        Self {
            keys,
            changed: BTreeSet::new(),
        }
    }

    fn with_changed(mut self, changed: BTreeSet<String>) -> Self {
        self.changed = changed;
        self
    }
}

impl Renderer {
    fn blocks(&mut self, items: &[Block]) -> Vec<AnyView> {
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
            out.push(self.block(item));
        }
        if !run.is_empty() {
            out.push(view! { <ul class="entry-links">{run}</ul> }.into_any());
        }
        out
    }

    fn block(&mut self, item: &Block) -> AnyView {
        match item {
            Block::Heading { level, content, .. } => {
                let key = self.keys.get(&std::ptr::from_ref(item)).cloned().flatten();
                debug_assert!(
                    self.keys.contains_key(&std::ptr::from_ref(item)),
                    "a rendered heading is missing from the heading traversal"
                );
                let id = key.as_deref().map(heading_dom_id);
                heading(*level, id, inlines(content))
            }
            Block::Paragraph { style, content } => {
                let class = match style {
                    ParagraphStyle::Normal => None,
                    ParagraphStyle::StatLine => Some("stat-line"),
                    ParagraphStyle::Indented => Some("indented"),
                };
                view! { <p class=class>{inlines_with_changed(content, &self.changed)}</p> }
                    .into_any()
            }
            Block::List { ordered, items } => {
                let rows: Vec<AnyView> = items
                    .iter()
                    .map(|cell| view! { <li>{self.blocks(cell)}</li> }.into_any())
                    .collect();
                if *ordered {
                    view! { <ol>{rows}</ol> }.into_any()
                } else {
                    view! { <ul>{rows}</ul> }.into_any()
                }
            }
            Block::Table(table) => rules_table(table),
            Block::Quote { content } => {
                view! { <blockquote>{self.blocks(content)}</blockquote> }.into_any()
            }
            Block::StatBlock { title, rows } => stat_block(title.as_deref(), rows, &self.changed),
            Block::EntryLink { target, title } => {
                let href = Route::entry(target.clone()).href();
                view! { <ul class="entry-links"><li><a href=href>{title.clone()}</a></li></ul> }
                    .into_any()
            }
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

fn stat_block(title: Option<&str>, rows: &[StatRow], changed: &BTreeSet<String>) -> AnyView {
    let title_id = title.map(|_| unique("stat"));
    let heading = title.map(|t| {
        let id = title_id.clone();
        view! { <p class="stat-title" id=id>{t.to_string()}</p> }
    });
    let rows: Vec<AnyView> = rows
        .iter()
        .map(|row| {
            let changed = changed.contains(&row.label.trim().to_lowercase()).then(|| {
                view! { <span class="tag changed-tag">"changed"</span> }
            });
            view! {
                <div class="stat-row">
                    <dt>{row.label.clone()}{changed}</dt>
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
pub fn RichBody(
    body: Vec<Block>,
    anchor: Option<String>,
    /// Lowercase labels of stat rows that a revision changed.
    #[prop(optional)]
    changed: Vec<String>,
) -> impl IntoView {
    let changed = changed.into_iter().collect::<BTreeSet<_>>();
    let content = Renderer::new(&body).with_changed(changed).blocks(&body);
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

    fn h(anchor: Option<&str>, title: &str) -> Block {
        Block::Heading {
            level: 2,
            anchor: anchor.map(String::from),
            content: vec![text(title)],
        }
    }

    #[test]
    fn heading_dom_ids_are_namespaced_away_from_app_ids() {
        let body = vec![h(Some("main"), "Main")];
        let keys = heading_keys(&body);
        assert_eq!(keys, vec![Some("main".to_string())]);
        assert_eq!(heading_dom_id(keys[0].as_deref().unwrap()), "h-main");
    }

    #[test]
    fn duplicate_heading_text_gets_distinct_keys() {
        let body = vec![h(None, "X"), h(None, "X")];
        assert_eq!(
            heading_keys(&body),
            vec![Some("sec-x".to_string()), Some("sec-x-2".to_string())]
        );
    }

    #[test]
    fn duplicate_suffix_skips_a_literal_key_later_in_the_body() {
        let body = vec![h(None, "X"), h(None, "X"), h(Some("sec-x-2"), "Y")];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("sec-x".to_string()),
                Some("sec-x-3".to_string()),
                Some("sec-x-2".to_string()),
            ]
        );
    }

    #[test]
    fn empty_anchor_falls_back_to_the_slug_and_an_empty_slug_has_no_key() {
        let body = vec![h(Some(""), "Big Table"), h(None, "!!")];
        assert_eq!(
            heading_keys(&body),
            vec![Some("sec-big-table".to_string()), None]
        );
    }

    #[test]
    fn heading_keys_follow_nested_blocks_in_document_order() {
        let body = vec![
            h(None, "X"),
            Block::List {
                ordered: false,
                items: vec![vec![h(None, "X")]],
            },
            Block::Quote {
                content: vec![h(Some("q"), "Q")],
            },
        ];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("sec-x".to_string()),
                Some("sec-x-2".to_string()),
                Some("q".to_string()),
            ]
        );
    }

    #[test]
    fn a_content_heading_never_takes_a_reserved_app_anchor() {
        let body = vec![
            h(Some(RELATED_ANCHOR), "Related"),
            h(Some(RELATED_ANCHOR), "Related again"),
            h(None, "Entry Related"),
        ];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("entry-related-2".to_string()),
                Some("entry-related-3".to_string()),
                Some("sec-entry-related".to_string()),
            ]
        );
        assert!(RESERVED_ANCHOR_KEYS.contains(&RELATED_ANCHOR));
    }

    #[test]
    fn a_reserved_anchor_suffix_skips_a_literal_key_in_the_body() {
        let body = vec![
            h(Some(RELATED_ANCHOR), "A"),
            h(Some("entry-related-2"), "B"),
        ];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("entry-related-3".to_string()),
                Some("entry-related-2".to_string()),
            ]
        );
    }

    #[test]
    fn the_traversal_visits_nested_headings_and_the_renderer_table_matches_it() {
        let body = vec![
            h(None, "A"),
            Block::List {
                ordered: true,
                items: vec![
                    vec![h(None, "B")],
                    vec![Block::Quote {
                        content: vec![h(None, "C")],
                    }],
                ],
            },
            Block::Quote {
                content: vec![h(None, "D")],
            },
            h(None, "A"),
        ];
        let visited = headings_in_order(&body);
        let titles: Vec<String> = visited
            .iter()
            .map(|block| match block {
                Block::Heading { content, .. } => plain_text(content),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(titles, ["A", "B", "C", "D", "A"]);
        // The renderer looks each heading up by identity in this table.
        let renderer = Renderer::new(&body);
        assert_eq!(renderer.keys.len(), 5);
        for (block, key) in keyed_headings(&body) {
            assert_eq!(renderer.keys.get(&std::ptr::from_ref(block)), Some(&key));
        }
        let last = visited[4];
        assert_eq!(
            renderer.keys[&std::ptr::from_ref(last)].as_deref(),
            Some("sec-a-2")
        );
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
