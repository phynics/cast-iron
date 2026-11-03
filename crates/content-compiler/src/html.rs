//! Upstream HTML to typed rich content. Raw HTML never leaves this module: only `Block` and
//! `Inline` values come out, and the client renders those node types alone.

use cast_iron_schema::{Block, EntryId, Inline, ParagraphStyle, Table, TableCell, TableRow};
use scraper::{ElementRef, Html, Node};

/// The compiler services that conversion needs.
pub trait Env {
    /// Resolves an upstream href. Implementations count the outcome.
    fn link(&mut self, href: &str) -> Option<(EntryId, Option<String>)>;
    /// Counts content that was removed or unwrapped on purpose.
    fn stripped(&mut self, key: &str);
}

/// An environment with no link targets, for plain-text extraction.
pub struct NullEnv;

impl Env for NullEnv {
    fn link(&mut self, _href: &str) -> Option<(EntryId, Option<String>)> {
        None
    }
    fn stripped(&mut self, _key: &str) {}
}

const KNOWN_ENTITIES: &[&str] = &[
    "mdash", "ndash", "times", "nbsp", "ldquo", "rdquo", "lsquo", "rsquo", "hellip", "bull",
    "dagger", "Dagger", "deg", "shy", "copy", "frac12", "plusmn", "minus",
];

/// Repairs entities that upstream wrote without the closing semicolon (`&mdash"`), and the
/// `&ndash1;` typo (`&ndash;1`). Everything else is left to the HTML parser.
pub fn fix_entities(input: &str, env: &mut dyn Env) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len() + 8);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
                end += 1;
            }
            let name = &input[start..end];
            if KNOWN_ENTITIES.contains(&name) {
                let next = bytes.get(end).copied();
                if next == Some(b';') {
                    // Well formed.
                } else if matches!(name, "ndash" | "mdash")
                    && next.is_some_and(|b| b.is_ascii_digit())
                {
                    let mut digits = end;
                    while digits < bytes.len() && bytes[digits].is_ascii_digit() {
                        digits += 1;
                    }
                    out.push('&');
                    out.push_str(name);
                    out.push(';');
                    out.push_str(&input[end..digits]);
                    i = if bytes.get(digits) == Some(&b';') {
                        digits + 1
                    } else {
                        digits
                    };
                    env.stripped("entity_repaired");
                    continue;
                } else if !next.is_some_and(|b| b.is_ascii_alphanumeric()) {
                    out.push('&');
                    out.push_str(name);
                    out.push(';');
                    i = end;
                    env.stripped("entity_repaired");
                    continue;
                }
            }
        }
        // Copy one UTF-8 character.
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&input[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn utf8_len(first: u8) -> usize {
    match first {
        0..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

/// Converts an HTML fragment to blocks.
pub fn to_blocks(html: &str, env: &mut dyn Env) -> Vec<Block> {
    let fixed = fix_entities(html, env);
    let fragment = Html::parse_fragment(&fixed);
    let mut walker = Walker { env };
    walker.blocks_of(fragment.root_element())
}

/// Converts an HTML fragment to inline content. Block structure becomes line breaks.
pub fn to_inlines(html: &str, env: &mut dyn Env) -> Vec<Inline> {
    if !html.contains(['<', '&']) {
        return normalize(vec![Inline::text(html)]);
    }
    let fixed = fix_entities(html, env);
    let fragment = Html::parse_fragment(&fixed);
    let mut walker = Walker { env };
    walker.inlines_of(fragment.root_element())
}

/// Decoded plain text of an HTML fragment, with whitespace normalized.
pub fn plain_text(html: &str) -> String {
    crate::text::inline_text(&to_inlines(html, &mut NullEnv))
}

struct Walker<'e> {
    env: &'e mut dyn Env,
}

const CONTAINERS: &[&str] = &[
    "div",
    "center",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "main",
    "nav",
    "figure",
    "figcaption",
    "form",
    "fieldset",
    "dl",
    "dd",
    "dt",
    "address",
    "details",
    "summary",
    "pre",
    "li",
    "body",
    "html",
];

const INLINE_BREAKERS: &[&str] = &[
    "p",
    "div",
    "center",
    "section",
    "article",
    "ul",
    "ol",
    "li",
    "table",
    "tr",
    "blockquote",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "dl",
    "dd",
    "dt",
    "pre",
    "caption",
    "hr",
];

impl Walker<'_> {
    fn blocks_of(&mut self, el: ElementRef<'_>) -> Vec<Block> {
        let mut out = Vec::new();
        let mut pending = Vec::new();
        for child in el.children() {
            match child.value() {
                Node::Text(text) => pending.push(Inline::text(text.to_string())),
                Node::Element(_) => {
                    let Some(ce) = ElementRef::wrap(child) else {
                        continue;
                    };
                    let name = ce.value().name();
                    if is_block(name) {
                        self.flush(&mut pending, &mut out);
                        self.block_el(ce, &mut out);
                    } else {
                        self.inline_el(ce, &mut pending);
                    }
                }
                _ => {}
            }
        }
        self.flush(&mut pending, &mut out);
        out
    }

    fn flush(&mut self, pending: &mut Vec<Inline>, out: &mut Vec<Block>) {
        let content = normalize(std::mem::take(pending));
        if !content.is_empty() {
            out.push(Block::Paragraph {
                style: ParagraphStyle::Normal,
                content,
            });
        }
    }

    fn block_el(&mut self, el: ElementRef<'_>, out: &mut Vec<Block>) {
        let name = el.value().name();
        match name {
            "p" => {
                let content = self.inlines_of(el);
                if !content.is_empty() {
                    out.push(Block::Paragraph {
                        style: class_style(el),
                        content,
                    });
                }
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = name[1..].parse::<u8>().unwrap_or(2).clamp(2, 6);
                let content = self.inlines_of(el);
                if !content.is_empty() {
                    out.push(Block::Heading {
                        level,
                        anchor: None,
                        content,
                    });
                }
            }
            "ul" | "ol" => {
                let mut items = Vec::new();
                let mut stray = Vec::new();
                for child in el.children() {
                    match child.value() {
                        Node::Element(e) if e.name() == "li" => {
                            if let Some(li) = ElementRef::wrap(child) {
                                let blocks = self.blocks_of(li);
                                if !blocks.is_empty() {
                                    items.push(blocks);
                                }
                            }
                        }
                        Node::Text(t) => stray.push(Inline::text(t.to_string())),
                        Node::Element(_) => {
                            if let Some(ce) = ElementRef::wrap(child) {
                                self.inline_el(ce, &mut stray);
                            }
                        }
                        _ => {}
                    }
                }
                let stray = normalize(stray);
                if !stray.is_empty() {
                    items.push(vec![Block::Paragraph {
                        style: ParagraphStyle::Normal,
                        content: stray,
                    }]);
                }
                if !items.is_empty() {
                    out.push(Block::List {
                        ordered: name == "ol",
                        items,
                    });
                }
            }
            "table" => {
                if let Some(table) = self.table(el) {
                    out.push(Block::Table(table));
                }
            }
            "blockquote" => {
                let content = self.blocks_of(el);
                if !content.is_empty() {
                    out.push(Block::Quote { content });
                }
            }
            "hr" => self.env.stripped("hr"),
            _ => {
                self.env.stripped(&format!("unwrapped_tag:{name}"));
                let inner = self.blocks_of(el);
                out.extend(inner);
            }
        }
    }

    fn inlines_of(&mut self, el: ElementRef<'_>) -> Vec<Inline> {
        let mut out = Vec::new();
        self.collect(el, &mut out);
        normalize(out)
    }

    fn collect(&mut self, el: ElementRef<'_>, out: &mut Vec<Inline>) {
        for child in el.children() {
            match child.value() {
                Node::Text(text) => out.push(Inline::text(text.to_string())),
                Node::Element(_) => {
                    if let Some(ce) = ElementRef::wrap(child) {
                        self.inline_el(ce, out);
                    }
                }
                _ => {}
            }
        }
    }

    fn wrapped(&mut self, el: ElementRef<'_>) -> Vec<Inline> {
        let mut inner = Vec::new();
        self.collect(el, &mut inner);
        inner
    }

    fn inline_el(&mut self, el: ElementRef<'_>, out: &mut Vec<Inline>) {
        let name = el.value().name();
        match name {
            "i" | "em" | "cite" | "dfn" | "var" => {
                let c = self.wrapped(el);
                out.push(Inline::Em { c });
            }
            "b" | "strong" => {
                let c = self.wrapped(el);
                out.push(Inline::Strong { c });
            }
            "sup" => {
                let c = self.wrapped(el);
                out.push(Inline::Sup { c });
            }
            "sub" => {
                let c = self.wrapped(el);
                out.push(Inline::Sub { c });
            }
            "br" => out.push(Inline::Break),
            "a" => {
                let c = self.wrapped(el);
                match el.value().attr("href") {
                    Some(href) => match self.env.link(href) {
                        Some((target, anchor)) => out.push(Inline::Link { target, anchor, c }),
                        None => out.extend(c),
                    },
                    None => out.extend(c),
                }
            }
            "img" => self.env.stripped("img"),
            "script" | "style" => self.env.stripped(&format!("removed_tag:{name}")),
            "td" | "th" => {
                self.collect(el, out);
                out.push(Inline::text(" "));
            }
            _ if INLINE_BREAKERS.contains(&name) => {
                self.env.stripped(&format!("flattened_tag:{name}"));
                out.push(Inline::Break);
                self.collect(el, out);
                out.push(Inline::Break);
            }
            _ => {
                self.env.stripped(&format!("unwrapped_tag:{name}"));
                self.collect(el, out);
            }
        }
    }

    fn table(&mut self, table: ElementRef<'_>) -> Option<Table> {
        let mut out = Table::default();
        for child in table.children() {
            let Some(ce) = ElementRef::wrap(child) else {
                continue;
            };
            match ce.value().name() {
                "caption" => out.caption = self.inlines_of(ce),
                "thead" => out.head.extend(self.rows(ce)),
                "tbody" => out.body.extend(self.rows(ce)),
                "tfoot" => out.foot.extend(self.rows(ce)),
                "tr" => out.body.extend(self.row(ce)),
                _ => {}
            }
        }
        let empty = out.caption.is_empty()
            && out.head.is_empty()
            && out.body.is_empty()
            && out.foot.is_empty();
        (!empty).then_some(out)
    }

    fn rows(&mut self, section: ElementRef<'_>) -> Vec<TableRow> {
        let mut rows = Vec::new();
        for child in section.children() {
            if let Some(tr) = ElementRef::wrap(child)
                && tr.value().name() == "tr"
            {
                rows.extend(self.row(tr));
            }
        }
        rows
    }

    fn row(&mut self, tr: ElementRef<'_>) -> Option<TableRow> {
        let mut cells = Vec::new();
        for child in tr.children() {
            let Some(cell) = ElementRef::wrap(child) else {
                continue;
            };
            let name = cell.value().name();
            if name != "td" && name != "th" {
                continue;
            }
            let span = |attr: &str| {
                cell.value()
                    .attr(attr)
                    .and_then(|v| v.trim().parse::<u16>().ok())
                    .filter(|v| *v > 0)
                    .unwrap_or(1)
            };
            cells.push(TableCell {
                header: name == "th",
                colspan: span("colspan"),
                rowspan: span("rowspan"),
                content: self.inlines_of(cell),
            });
        }
        (!cells.is_empty()).then_some(TableRow { cells })
    }
}

fn is_block(name: &str) -> bool {
    matches!(
        name,
        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "table" | "blockquote" | "hr"
    ) || CONTAINERS.contains(&name)
}

fn class_style(el: ElementRef<'_>) -> ParagraphStyle {
    let Some(class) = el.value().attr("class") else {
        return ParagraphStyle::Normal;
    };
    let tokens: Vec<&str> = class.split_whitespace().collect();
    if tokens
        .iter()
        .any(|t| *t == "indent" || *t == "stat-block-2")
    {
        ParagraphStyle::Indented
    } else if tokens.iter().any(|t| t.starts_with("stat-block")) {
        ParagraphStyle::StatLine
    } else {
        ParagraphStyle::Normal
    }
}

/// Collapses whitespace runs (including no-break spaces), drops soft hyphens, merges adjacent
/// text, removes empty nodes, and trims leading and trailing space and breaks.
pub fn normalize(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut last_space = true;
    let mut out = norm_seq(inlines, &mut last_space);
    trim_end(&mut out);
    out
}

fn norm_seq(inlines: Vec<Inline>, last_space: &mut bool) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { v } => {
                let mut text = String::with_capacity(v.len());
                for ch in v.chars() {
                    if ch == '\u{ad}' {
                        continue;
                    }
                    if ch.is_whitespace() {
                        if !*last_space {
                            text.push(' ');
                            *last_space = true;
                        }
                    } else {
                        text.push(ch);
                        *last_space = false;
                    }
                }
                if text.is_empty() {
                    continue;
                }
                if let Some(Inline::Text { v: prev }) = out.last_mut() {
                    prev.push_str(&text);
                } else {
                    out.push(Inline::Text { v: text });
                }
            }
            Inline::Break => {
                trim_end(&mut out);
                if !out.is_empty() {
                    out.push(Inline::Break);
                }
                *last_space = true;
            }
            Inline::Em { c } => {
                let c = norm_seq(c, last_space);
                if !c.is_empty() {
                    out.push(Inline::Em { c });
                }
            }
            Inline::Strong { c } => {
                let c = norm_seq(c, last_space);
                if !c.is_empty() {
                    out.push(Inline::Strong { c });
                }
            }
            Inline::Sup { c } => {
                let c = norm_seq(c, last_space);
                if !c.is_empty() {
                    out.push(Inline::Sup { c });
                }
            }
            Inline::Sub { c } => {
                let c = norm_seq(c, last_space);
                if !c.is_empty() {
                    out.push(Inline::Sub { c });
                }
            }
            Inline::Link { target, anchor, c } => {
                let c = norm_seq(c, last_space);
                if !c.is_empty() {
                    out.push(Inline::Link { target, anchor, c });
                }
            }
        }
    }
    out
}

/// Removes trailing breaks and spaces, recursing into the last container.
fn trim_end(out: &mut Vec<Inline>) {
    loop {
        match out.last_mut() {
            Some(Inline::Break) => {
                out.pop();
            }
            Some(Inline::Text { v }) => {
                let trimmed = v.trim_end().len();
                v.truncate(trimmed);
                if v.is_empty() {
                    out.pop();
                } else {
                    return;
                }
            }
            Some(
                Inline::Em { c }
                | Inline::Strong { c }
                | Inline::Sup { c }
                | Inline::Sub { c }
                | Inline::Link { c, .. },
            ) => {
                trim_end(c);
                if c.is_empty() {
                    out.pop();
                } else {
                    return;
                }
            }
            None => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{blocks_text, inline_text};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct TestEnv {
        links: BTreeMap<String, (String, Option<String>)>,
        stripped: BTreeMap<String, u32>,
    }

    impl Env for TestEnv {
        fn link(&mut self, href: &str) -> Option<(EntryId, Option<String>)> {
            self.links
                .get(href)
                .map(|(id, a)| (EntryId::parse(id.clone()).unwrap(), a.clone()))
        }
        fn stripped(&mut self, key: &str) {
            *self.stripped.entry(key.to_string()).or_default() += 1;
        }
    }

    fn blocks(html: &str) -> Vec<Block> {
        to_blocks(html, &mut TestEnv::default())
    }

    #[test]
    fn maps_paragraph_emphasis_and_entities() {
        let b = blocks("<p>A <i>fireball</i> is &ldquo;hot&rdquo;&mdash;<b>very</b>.</p>");
        assert_eq!(
            blocks_text(&b),
            "A fireball is \u{201c}hot\u{201d}\u{2014}very."
        );
        let Block::Paragraph { content, .. } = &b[0] else {
            panic!()
        };
        assert!(matches!(content[1], Inline::Em { .. }));
        assert!(content.iter().any(|i| matches!(i, Inline::Strong { .. })));
    }

    #[test]
    fn stat_block_classes_pick_paragraph_styles() {
        let b = blocks(
            "<p class=\"stat-block-1\">a</p><p class=\"stat-block-2\">b</p><p class=\"indent\">c</p><p class=\"stat-block-rp\">d</p><p>e</p>",
        );
        let styles: Vec<_> = b
            .iter()
            .map(|b| match b {
                Block::Paragraph { style, .. } => *style,
                _ => panic!(),
            })
            .collect();
        use ParagraphStyle::*;
        assert_eq!(styles, [StatLine, Indented, Indented, StatLine, Normal]);
    }

    #[test]
    fn table_without_tbody_keeps_rows_and_spans() {
        // Literal shape from core_rulebook/class/core/barbarian.json, plus a real colspan/rowspan mix.
        let html = "<table><caption>Table: Starting Wealth</caption><thead><tr><th>Class</th><th>Starting Wealth</th></tr></thead><tr class=\"even\"><td>Barbarian</td><td>3d6 &times; 10 gp</td></tr></table>";
        let b = blocks(html);
        let Block::Table(t) = &b[0] else {
            panic!("{b:?}")
        };
        assert_eq!(inline_text(&t.caption), "Table: Starting Wealth");
        assert_eq!(t.head.len(), 1);
        assert_eq!(t.body.len(), 1);
        assert_eq!(inline_text(&t.body[0].cells[1].content), "3d6 \u{d7} 10 gp");

        let spans = "<table><thead><tr><th rowspan=\"2\">Level</th><th colspan=\"3\">Spells per Day</th></tr><tr><th>1st</th><th>2nd</th><th>3rd</th></tr></thead><tbody><tr><td>1st</td><td>1</td><td>&ndash;</td><td>&ndash;</td></tr></tbody><tfoot><tr><td colspan=\"4\">* note</td></tr></tfoot></table>";
        let b = blocks(spans);
        let Block::Table(t) = &b[0] else { panic!() };
        assert_eq!(t.head[0].cells[0].rowspan, 2);
        assert_eq!(t.head[0].cells[1].colspan, 3);
        assert!(t.head[0].cells[0].header);
        assert_eq!(t.head.len(), 2);
        assert_eq!(t.foot[0].cells[0].colspan, 4);
        assert_eq!(inline_text(&t.body[0].cells[2].content), "\u{2013}");
    }

    #[test]
    fn lists_and_quotes_nest() {
        let b = blocks(
            "<ul><li>one</li><li><b>two</b> x</li></ul><ol><li>a</li></ol><blockquote><p>q</p></blockquote>",
        );
        assert!(matches!(&b[0], Block::List { ordered: false, items } if items.len() == 2));
        assert!(matches!(&b[1], Block::List { ordered: true, .. }));
        assert!(matches!(&b[2], Block::Quote { .. }));
    }

    #[test]
    fn headings_are_clamped_to_two_through_six() {
        let b = blocks("<h1>a</h1><h3>b</h3><h6>c</h6>");
        let levels: Vec<u8> = b
            .iter()
            .map(|b| match b {
                Block::Heading { level, .. } => *level,
                _ => 0,
            })
            .collect();
        assert_eq!(levels, [2, 3, 6]);
    }

    #[test]
    fn malformed_html_keeps_all_text() {
        // Unclosed <b>, stray end tags, an unclosed <a>, and the invalid `&ndash1;` entity.
        let html = "<p><b>Prerequisites</b>: Str 13</p></p></b><p><b>Unclosed bold. <i>x</p><p>next</a></p>\
                    <table><tr><td>&ndash1; per additional type</td><td>open <a href=\"pfsrd://X/Y\">link</table>";
        let b = blocks(html);
        let text = blocks_text(&b);
        for needle in [
            "Prerequisites: Str 13",
            "Unclosed bold. x",
            "next",
            "\u{2013}1 per additional type",
            "open link",
        ] {
            assert!(text.contains(needle), "{needle:?} missing from {text:?}");
        }
        assert!(!text.contains('<') && !text.contains("&ndash"));
    }

    #[test]
    fn unknown_tags_unwrap_and_images_strip() {
        let mut env = TestEnv::default();
        let html = "<p>a<span class=\"x\"> b</span><big>c</big><wbr>d<sup3>3</sup3></p><div>loose</div><center>mid</center><p><img src=\"/p/a.jpg\">e</p>";
        let b = to_blocks(html, &mut env);
        assert_eq!(blocks_text(&b), "a bcd3\nloose\nmid\ne");
        assert_eq!(env.stripped["img"], 1);
        assert_eq!(env.stripped["unwrapped_tag:span"], 1);
        assert_eq!(env.stripped["unwrapped_tag:big"], 1);
        assert_eq!(env.stripped["unwrapped_tag:sup3"], 1);
        assert_eq!(env.stripped["unwrapped_tag:div"], 1);
    }

    #[test]
    fn links_resolve_or_become_plain_content() {
        let mut env = TestEnv::default();
        env.links
            .insert("pfsrd://A/B".into(), ("a/b".into(), Some("sec".into())));
        let b = to_blocks(
            "<p>See <a href=\"pfsrd://A/B\"><i>B</i></a> and <a href=\"pfsrd://Z\">Z</a>.</p>",
            &mut env,
        );
        let Block::Paragraph { content, .. } = &b[0] else {
            panic!()
        };
        assert!(matches!(&content[1], Inline::Link { anchor: Some(a), .. } if a == "sec"));
        assert_eq!(inline_text(content), "See B and Z.");
        assert!(
            !content
                .iter()
                .any(|i| matches!(i, Inline::Link { target, .. } if target.as_str() != "a/b"))
        );
    }

    #[test]
    fn whitespace_normalizes_and_empty_paragraphs_drop() {
        let b = blocks("<p>  a \n\t b&nbsp;&nbsp;c <br> d  </p><p> </p><p><br></p><p>&shy;</p>");
        assert_eq!(b.len(), 1);
        assert_eq!(blocks_text(&b), "a b c d");
    }

    #[test]
    fn repairs_semicolonless_entities() {
        assert_eq!(plain_text("Dr &mdash\" x"), "Dr \u{2014}\" x");
        assert_eq!(plain_text("&mdash"), "\u{2014}");
        assert_eq!(plain_text("&ndash1; each"), "\u{2013}1 each");
        assert_eq!(plain_text("&amp; &lt;b&gt;"), "& <b>");
    }

    #[test]
    fn script_content_never_reaches_output() {
        let b = blocks("<p>ok<script>alert(1)</script><img src=x onerror=alert(1)></p>");
        let json = serde_json::to_string(&b).unwrap();
        assert!(!json.contains("onerror"));
        assert_eq!(blocks_text(&b), "okalert(1)".replace("alert(1)", ""));
    }
}
