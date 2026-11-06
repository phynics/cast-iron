//! Explicit anchors on every heading something links to.
//!
//! A heading without an anchor gets a URL key from its text (`sec-<slug>`), so
//! editing the text would break a link or a book node that names that key. The
//! compiled library therefore carries the key as an explicit anchor on each
//! heading that has an inbound link, and the key string stays the same, so
//! section URLs stay valid. Headings nothing links to keep their derived key.

use cast_iron_schema::headings::heading_keys;
use cast_iron_schema::{Block, BookIndex, BookNode, EntryId, RuleEntry};
use std::collections::{BTreeMap, BTreeSet};

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
    use cast_iron_schema::{
        BookTree, EntryKind, EntryReference, Inline, ParagraphStyle, TypedFields,
    };

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
            source_ids: vec!["book".into()],
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
