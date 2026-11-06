//! Heading keys: the URL anchor of every heading in an entry body.
//!
//! The client renders a heading's DOM id from this key and the corpus tooling checks
//! links against it, so the algorithm lives here once.

use crate::{Block, Inline};
use std::collections::BTreeSet;

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

/// A heading's base key: the compiler's anchor, else `sec-` plus a slug of its text.
fn heading_base(anchor: Option<&str>, content: &[Inline]) -> Option<String> {
    match anchor {
        Some(anchor) if !anchor.is_empty() => Some(anchor.to_string()),
        _ => {
            let slug = slugify(&plain_text(content));
            (!slug.is_empty()).then(|| format!("sec-{slug}"))
        }
    }
}

/// The anchor that jumps to the Related panel of an entry page.
pub const RELATED_ANCHOR: &str = "entry-related";

/// Route anchors that target app elements on an entry page. `scroll_to_anchor` falls back to the
/// raw id for them, so content must never take one of these keys: `heading_keys` treats each as
/// already used and gives a colliding content heading the next free suffix.
pub const RESERVED_ANCHOR_KEYS: &[&str] = &[RELATED_ANCHOR];

/// Every heading block in `body`, in document order, descending into list items and quotes.
///
/// This is the single heading traversal. Key assignment, the article contents, and the renderer's
/// id lookup all start from it, so a new `Block` variant that nests blocks needs one change here.
pub fn headings_in_order(body: &[Block]) -> Vec<&Block> {
    fn walk<'a>(blocks: &'a [Block], out: &mut Vec<&'a Block>) {
        for block in blocks {
            match block {
                Block::Heading { .. } => out.push(block),
                Block::List { items, .. } => {
                    for item in items {
                        walk(item, out);
                    }
                }
                Block::Quote { content } => walk(content, out),
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    walk(body, &mut found);
    found
}

/// Each heading block of `body` with its final URL anchor key, in document order.
///
/// The first heading with a base key keeps it. Later headings with the same base get `-2`,
/// `-3`, ... skipping any key that another heading in this body already claims and any
/// [`RESERVED_ANCHOR_KEYS`] entry. An empty anchor falls back to a slug of the heading text; a
/// heading with neither has no key.
pub fn keyed_headings(body: &[Block]) -> Vec<(&Block, Option<String>)> {
    let headings = headings_in_order(body);
    let bases: Vec<Option<String>> = headings
        .iter()
        .map(|block| match block {
            Block::Heading {
                anchor, content, ..
            } => heading_base(anchor.as_deref(), content),
            _ => None,
        })
        .collect();
    // Reserve every literal key first, so a generated suffix never steals one of them.
    let literal: BTreeSet<String> = bases.iter().flatten().cloned().collect();
    let mut used: BTreeSet<String> = RESERVED_ANCHOR_KEYS
        .iter()
        .map(|key| (*key).to_string())
        .collect();
    let mut keyed = Vec::with_capacity(bases.len());
    for (block, base) in headings.into_iter().zip(bases) {
        let Some(base) = base else {
            keyed.push((block, None));
            continue;
        };
        let mut key = base.clone();
        if used.contains(&key) {
            let mut suffix = 2u32;
            loop {
                let candidate = format!("{base}-{suffix}");
                if !literal.contains(candidate.as_str()) && !used.contains(&candidate) {
                    key = candidate;
                    break;
                }
                suffix += 1;
            }
        }
        used.insert(key.clone());
        keyed.push((block, Some(key)));
    }
    keyed
}

/// Final URL anchor keys for every heading in `body`, in document order. See [`keyed_headings`].
pub fn heading_keys(body: &[Block]) -> Vec<Option<String>> {
    keyed_headings(body)
        .into_iter()
        .map(|(_, key)| key)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(anchor: Option<&str>, text: &str) -> Block {
        Block::Heading {
            level: 2,
            anchor: anchor.map(str::to_string),
            content: vec![Inline::text(text)],
        }
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
    fn a_literal_key_later_in_the_body_is_reserved() {
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
    fn the_related_panel_key_is_reserved() {
        let body = vec![h(None, "Entry Related")];
        assert_eq!(
            heading_keys(&body),
            vec![Some("sec-entry-related".to_string())]
        );
        let body = vec![h(Some(RELATED_ANCHOR), "Related")];
        assert_eq!(
            heading_keys(&body),
            vec![Some(format!("{RELATED_ANCHOR}-2"))]
        );
    }

    #[test]
    fn headings_inside_lists_and_quotes_are_keyed_in_document_order() {
        let body = vec![
            Block::Quote {
                content: vec![h(None, "In Quote")],
            },
            Block::List {
                ordered: false,
                items: vec![vec![h(None, "In List")]],
            },
            h(None, "After"),
        ];
        assert_eq!(
            heading_keys(&body),
            vec![
                Some("sec-in-quote".to_string()),
                Some("sec-in-list".to_string()),
                Some("sec-after".to_string()),
            ]
        );
    }
}
