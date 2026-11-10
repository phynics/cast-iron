//! Heading keys: the URL anchor of every heading in an entry body.
//!
//! The client renders a heading's DOM id from this key and the corpus tooling checks
//! links against it, so the algorithm lives here once.

use crate::{Block, BookIndex, BookNode, EntryId, Inline, RuleEntry};
use std::collections::{BTreeMap, BTreeSet};

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

// ---- Explicit anchors on linked headings -------------------------------------------------
//
// A heading without an anchor gets a URL key from its text (`sec-<slug>`), so editing the
// text would break a link or a book node that names that key. Every heading that has an
// inbound link therefore carries its key as an explicit anchor. The key string stays the
// same, so section URLs stay valid. Headings nothing links to keep their derived key.

/// What the pass changed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AnchorReport {
    /// Headings that received an explicit anchor.
    pub anchored: usize,
    /// Entries that changed.
    pub entries: usize,
    /// Entries that needed every keyed heading anchored to keep the keys stable.
    pub fallbacks: usize,
}

/// Anchors named by links and book nodes, by target entry.
fn inbound(entries: &[RuleEntry], books: &BookIndex) -> BTreeMap<EntryId, BTreeSet<String>> {
    let mut found: BTreeMap<EntryId, BTreeSet<String>> = BTreeMap::new();
    for entry in entries {
        for link in &entry.links {
            if let Some(anchor) = &link.anchor {
                found
                    .entry(link.target.clone())
                    .or_default()
                    .insert(anchor.clone());
            }
        }
    }
    fn walk(node: &BookNode, found: &mut BTreeMap<EntryId, BTreeSet<String>>) {
        if let (Some(entry), Some(anchor)) = (&node.entry, &node.anchor) {
            found
                .entry(entry.clone())
                .or_default()
                .insert(anchor.clone());
        }
        for child in &node.children {
            walk(child, found);
        }
    }
    for tree in &books.books {
        for chapter in &tree.chapters {
            walk(chapter, &mut found);
        }
    }
    found
}

/// Calls `visit` on every heading in document order, descending into list items
/// and quotes, the same order [`heading_keys`] uses.
fn visit_headings(blocks: &mut [Block], visit: &mut impl FnMut(&mut Option<String>)) {
    for block in blocks {
        match block {
            Block::Heading { anchor, .. } => visit(anchor),
            Block::List { items, .. } => {
                for item in items {
                    visit_headings(item, visit);
                }
            }
            Block::Quote { content } => visit_headings(content, visit),
            _ => {}
        }
    }
}

/// Gives each linked heading an explicit anchor equal to its current key.
pub fn anchor_linked_headings(entries: &mut [RuleEntry], books: &BookIndex) -> AnchorReport {
    let wanted = inbound(entries, books);
    let mut report = AnchorReport::default();
    for entry in entries.iter_mut() {
        let Some(anchors) = wanted.get(&entry.id) else {
            continue;
        };
        let before = heading_keys(&entry.body);
        let mut changed = apply(&mut entry.body, &before, |key| anchors.contains(key));
        if changed == 0 {
            continue;
        }
        if heading_keys(&entry.body) != before {
            // Reserving more literal keys moved a duplicate suffix. Anchoring every
            // keyed heading makes each key literal and so stable.
            let mut again = entry.clone();
            changed = apply(&mut again.body, &before, |_| true);
            assert_eq!(
                heading_keys(&again.body),
                before,
                "anchoring every heading must keep the keys of {}",
                entry.id
            );
            entry.body = again.body;
            report.fallbacks += 1;
        }
        report.anchored += changed;
        report.entries += 1;
    }
    report
}

/// Sets the anchor of each heading whose key passes `select`; returns how many
/// headings changed.
fn apply(body: &mut [Block], keys: &[Option<String>], select: impl Fn(&str) -> bool) -> usize {
    let mut index = 0;
    let mut changed = 0;
    visit_headings(body, &mut |anchor| {
        if let Some(Some(key)) = keys.get(index)
            && select(key)
            && anchor.as_deref() != Some(key.as_str())
        {
            *anchor = Some(key.clone());
            changed += 1;
        }
        index += 1;
    });
    changed
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

#[cfg(test)]
mod anchor_tests {
    use super::*;
    use crate::{BookTree, EntryKind, EntryReference, ParagraphStyle, TypedFields};

    fn heading(anchor: Option<&str>, text: &str) -> Block {
        Block::Heading {
            level: 2,
            anchor: anchor.map(str::to_string),
            content: vec![Inline::text(text)],
        }
    }

    fn entry(id: &str, body: Vec<Block>, links: &[(&str, &str)]) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Rule,
            title: id.into(),
            aon: None,
            source_ids: vec!["book".into()],
            sources: vec![],
            section: vec![],
            parent: None,
            body,
            fields: TypedFields::None,
            tags: vec![],
            links: links
                .iter()
                .map(|(target, anchor)| EntryReference {
                    target: EntryId::parse(*target).unwrap(),
                    anchor: Some((*anchor).into()),
                })
                .collect(),
            provenance: vec![],
            license_refs: vec![],
            relations: vec![],
        }
    }

    fn anchors_of(entry: &RuleEntry) -> Vec<Option<String>> {
        let mut found = Vec::new();
        let mut body = entry.body.clone();
        visit_headings(&mut body, &mut |anchor| found.push(anchor.clone()));
        found
    }

    #[test]
    fn a_linked_derived_heading_gets_its_key_as_an_explicit_anchor() {
        let mut entries = vec![
            entry("ref/rule/a", vec![], &[("ref/rule/b", "sec-deep-dive")]),
            entry(
                "ref/rule/b",
                vec![heading(None, "Deep Dive"), heading(None, "Unlinked")],
                &[],
            ),
        ];
        let before = heading_keys(&entries[1].body);
        let report = anchor_linked_headings(&mut entries, &BookIndex { books: vec![] });
        assert_eq!(
            report,
            AnchorReport {
                anchored: 1,
                entries: 1,
                fallbacks: 0
            }
        );
        assert_eq!(
            anchors_of(&entries[1]),
            vec![Some("sec-deep-dive".to_string()), None]
        );
        assert_eq!(heading_keys(&entries[1].body), before, "keys do not move");
    }

    #[test]
    fn a_book_node_counts_as_an_inbound_link() {
        let mut entries = vec![entry("ref/rule/b", vec![heading(None, "Chapter One")], &[])];
        let books = BookIndex {
            books: vec![BookTree {
                source: "book".into(),
                title: "Book".into(),
                basis: "reviewed".into(),
                review: vec![],
                chapters: vec![BookNode {
                    title: "One".into(),
                    entry: Some(EntryId::parse("ref/rule/b").unwrap()),
                    anchor: Some("sec-chapter-one".into()),
                    review: None,
                    collapse: false,
                    lists_children: false,
                    children: vec![],
                }],
            }],
        };
        anchor_linked_headings(&mut entries, &books);
        assert_eq!(
            anchors_of(&entries[0]),
            vec![Some("sec-chapter-one".to_string())]
        );
    }

    #[test]
    fn duplicate_headings_keep_their_suffixed_keys() {
        let body = vec![
            heading(None, "X"),
            heading(None, "X"),
            heading(Some("sec-x-2"), "Y"),
        ];
        let mut entries = vec![
            entry("ref/rule/a", vec![], &[("ref/rule/b", "sec-x-3")]),
            entry("ref/rule/b", body, &[]),
        ];
        let before = heading_keys(&entries[1].body);
        assert_eq!(before[1].as_deref(), Some("sec-x-3"));
        anchor_linked_headings(&mut entries, &BookIndex { books: vec![] });
        assert_eq!(heading_keys(&entries[1].body), before);
        assert_eq!(anchors_of(&entries[1])[1].as_deref(), Some("sec-x-3"));
    }

    #[test]
    fn headings_in_lists_and_quotes_are_anchored_in_document_order() {
        let body = vec![
            Block::Quote {
                content: vec![heading(None, "Quoted")],
            },
            Block::List {
                ordered: false,
                items: vec![vec![
                    Block::Paragraph {
                        style: ParagraphStyle::Normal,
                        content: vec![Inline::text("p")],
                    },
                    heading(None, "Listed"),
                ]],
            },
        ];
        let mut entries = vec![
            entry(
                "ref/rule/a",
                vec![],
                &[("ref/rule/b", "sec-listed"), ("ref/rule/b", "sec-quoted")],
            ),
            entry("ref/rule/b", body, &[]),
        ];
        anchor_linked_headings(&mut entries, &BookIndex { books: vec![] });
        assert_eq!(
            anchors_of(&entries[1]),
            vec![
                Some("sec-quoted".to_string()),
                Some("sec-listed".to_string())
            ]
        );
    }

    #[test]
    fn the_pass_is_idempotent() {
        let mut entries = vec![
            entry("ref/rule/a", vec![], &[("ref/rule/b", "sec-deep")]),
            entry("ref/rule/b", vec![heading(None, "Deep")], &[]),
        ];
        let books = BookIndex { books: vec![] };
        anchor_linked_headings(&mut entries, &books);
        let once = entries.clone();
        let report = anchor_linked_headings(&mut entries, &books);
        assert_eq!(entries, once);
        assert_eq!(report, AnchorReport::default());
    }
}
