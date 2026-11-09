//! The reader's view of a book contents tree: which rows Contents lists, which chapter page a
//! row opens, and the anchors those pages expose.
//!
//! The compiler builds the search documents for book sections from these functions, and the
//! client renders the same rows, so a section result and its target cannot disagree. The
//! functions are pure and hold no knowledge of routes, DOM, or search.

use crate::{Block, BookIndex, BookNode, BookTree, EntryId, EntryKind, RuleEntry};
use std::collections::{HashMap, HashSet};

/// Whether Contents leaves out the kinds that the `In this book` links reach.
///
/// The owner decided on 2026-11-07 that Contents does not list the kinds that have their own
/// list page. Every kind except Rules has one, and the kind links reach each as a list page
/// filtered to the book, so the exclusion is on.
pub const LEAVE_OUT_LINKED_KINDS: bool = true;

/// The kinds that `In this book` reaches: every kind with its own list page, which is every kind
/// except Rules. Rules stay in Contents because they have no list page of their own.
pub fn linked_kind(kind: EntryKind) -> bool {
    kind != EntryKind::Rule
}

/// True when Contents leaves a kind out. Always false while [`LEAVE_OUT_LINKED_KINDS`] is off.
pub fn hidden_from_contents(kind: EntryKind) -> bool {
    LEAVE_OUT_LINKED_KINDS && linked_kind(kind)
}

/// Says whether Contents leaves an entry out. An entry the catalog does not know is kept.
pub type Hide<'a> = &'a dyn Fn(&EntryId) -> bool;

/// True when a node still has something to list. A node whose entries are all left out has
/// nothing; a node with no entry anywhere under it is kept as the compiler wrote it.
pub fn shows_in_contents(node: &BookNode, hide: Hide<'_>) -> bool {
    fn all_hidden(node: &BookNode, hide: Hide<'_>, seen: &mut bool) -> bool {
        if let Some(entry) = &node.entry {
            *seen = true;
            if !hide(entry) {
                return false;
            }
        }
        node.children
            .iter()
            .all(|child| all_hidden(child, hide, seen))
    }
    let mut seen = false;
    !(all_hidden(node, hide, &mut seen) && seen)
}

/// The indices of the children of `node` that Contents lists, in book order.
pub fn listed_children(node: &BookNode, hide: Hide<'_>) -> Vec<usize> {
    (0..node.children.len())
        .filter(|&i| shows_in_contents(&node.children[i], hide))
        .collect()
}

/// The entries under a node, itself included, that Contents lists: the number a chapter shows.
pub fn listed_under(node: &BookNode, hide: Hide<'_>) -> usize {
    let own = node.entry.as_ref().is_some_and(|entry| !hide(entry));
    usize::from(own)
        + node
            .children
            .iter()
            .map(|child| listed_under(child, hide))
            .sum::<usize>()
}

/// True when `node` is itself an entry and that entry's body lists every entry below the node.
///
/// Only [`Block::EntryLink`] rows count. The renderer shows each as a link under the hub's own
/// page, which is the rule: Contents repeats a list that the page already holds. An inline
/// reference elsewhere in the body is not that list, so it never collapses a node. A node with
/// no entry, no entry below it, or one entry the page does not link stays a branch.
pub fn entry_lists_children(node: &BookNode, entry: &RuleEntry) -> bool {
    fn collect(node: &BookNode, own: &EntryId, out: &mut HashSet<EntryId>) {
        for child in &node.children {
            if let Some(id) = &child.entry
                && id != own
            {
                out.insert(id.clone());
            }
            collect(child, own, out);
        }
    }
    fn linked(blocks: &[Block], out: &mut HashSet<EntryId>) {
        for block in blocks {
            match block {
                Block::EntryLink { target, .. } => {
                    out.insert(target.clone());
                }
                Block::List { items, .. } => {
                    for item in items {
                        linked(item, out);
                    }
                }
                Block::Quote { content } => linked(content, out),
                _ => {}
            }
        }
    }
    let Some(own) = &node.entry else {
        return false;
    };
    let mut children = HashSet::new();
    collect(node, own, &mut children);
    if children.is_empty() {
        return false;
    }
    let mut links = HashSet::new();
    linked(&entry.body, &mut links);
    children.is_subset(&links)
}

/// Records [`BookNode::lists_children`] on every node of `books`.
///
/// The compiler calls this after it has every entry body; the client reads the flag from
/// `books.json` and never loads a body to decide a row. `entries` may hold more than the books
/// place; only an id the tree names matters.
pub fn mark_listed_children(books: &mut BookIndex, entries: &[RuleEntry]) {
    let by_id: HashMap<&EntryId, &RuleEntry> =
        entries.iter().map(|entry| (&entry.id, entry)).collect();
    fn walk(node: &mut BookNode, by_id: &HashMap<&EntryId, &RuleEntry>) {
        node.lists_children = node
            .entry
            .as_ref()
            .and_then(|id| by_id.get(id))
            // A kind Contents leaves out has no row link to open, so its node keeps its branch.
            .is_some_and(|entry| {
                !hidden_from_contents(entry.kind) && entry_lists_children(node, entry)
            });
        for child in &mut node.children {
            walk(child, by_id);
        }
    }
    for book in &mut books.books {
        for chapter in &mut book.chapters {
            walk(chapter, &by_id);
        }
    }
}

/// The title that the compiler gives the chapter of entries that no curated chapter places.
pub const CATCH_ALL_TITLE: &str = "New entries (page order)";

/// What a reader sees for it. The title says how the list was made, which a reader does not
/// need; the corpus carries the old title, so the page renames it (the pin bump that renames it
/// at the source is the corpus owner's).
pub const CATCH_ALL_LABEL: &str = "More from this book";

/// The paths of the rows at the top of Contents. A book whose one remaining chapter is the
/// catch-all shows that chapter's rows at the top, not a lone `More from this book` heading
/// over them. Paths are real: they index the whole tree, not the listed rows.
pub fn contents_roots(chapters: &[BookNode], hide: Hide<'_>) -> Vec<Vec<usize>> {
    let listed: Vec<usize> = (0..chapters.len())
        .filter(|&i| shows_in_contents(&chapters[i], hide))
        .collect();
    if let [only] = listed[..] {
        let node = &chapters[only];
        if node.title == CATCH_ALL_TITLE && node.entry.is_none() {
            return listed_children(node, hide)
                .into_iter()
                .map(|child| vec![only, child])
                .collect();
        }
    }
    listed.into_iter().map(|i| vec![i]).collect()
}

/// One chapter page of a book: its permanent address and the path of the node it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterId {
    pub id: String,
    pub path: Vec<usize>,
}

/// The permanent id of a chapter: its compiler anchor, else a slug of its printed title. The
/// title is the corpus title, not the display label, so renaming a label cannot move the address.
/// A title with no ASCII letter or digit gets `chapter`, which later collisions still number.
pub fn chapter_base(node: &BookNode) -> String {
    if let Some(anchor) = node.anchor.as_deref().filter(|anchor| !anchor.is_empty()) {
        return anchor.to_string();
    }
    let slug = crate::headings::slugify(&node.title);
    if slug.is_empty() {
        "chapter".to_string()
    } else {
        slug
    }
}

/// The permanent ids of the chapters at `paths`, in the order given. The first chapter to claim
/// a base keeps it; a later one takes `-2`, `-3`, ... so a collision never moves the earlier id.
/// A chapter's position is never part of its id.
pub fn chapter_ids<'a>(
    chapters: &[BookNode],
    paths: impl IntoIterator<Item = &'a [usize]>,
) -> Vec<ChapterId> {
    let mut used = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter_map(|path| {
            let node = node_at_path(chapters, path)?;
            let base = chapter_base(node);
            let mut id = base.clone();
            let mut suffix = 2;
            while used.contains(&id) {
                id = format!("{base}-{suffix}");
                suffix += 1;
            }
            used.insert(id.clone());
            Some(ChapterId {
                id,
                path: path.to_vec(),
            })
        })
        .collect()
}

/// The node at a path of a list of chapter trees. `node_at` takes a whole book.
pub fn node_at_path<'a>(chapters: &'a [BookNode], path: &[usize]) -> Option<&'a BookNode> {
    let (first, rest) = path.split_first()?;
    let mut node = chapters.get(*first)?;
    for i in rest {
        node = node.children.get(*i)?;
    }
    Some(node)
}

/// One top-level row of Contents as a chapter page: the permanent id, the real tree path and the
/// label the reader sees. A book whose only listed chapter is the catch-all shows that chapter's
/// rows at the top, and each of those rows is a chapter page in the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterRow {
    pub id: String,
    pub path: Vec<usize>,
    /// The corpus title. The client applies its display label.
    pub title: String,
}

/// The chapter pages of a book, in Contents order. The ids follow the corpus anchor or the
/// printed title, never the row's position (see [`chapter_ids`]).
pub fn chapter_rows(book: &BookTree, hide: Hide<'_>) -> Vec<ChapterRow> {
    let roots = contents_roots(&book.chapters, hide);
    chapter_ids(&book.chapters, roots.iter().map(Vec::as_slice))
        .into_iter()
        .filter_map(|chapter| {
            let node = node_at_path(&book.chapters, &chapter.path)?;
            Some(ChapterRow {
                id: chapter.id,
                path: chapter.path,
                title: node.title.clone(),
            })
        })
        .collect()
}

/// The DOM id of a contents node, so that a chapter page can scroll to it.
pub fn toc_id(path: &[usize]) -> String {
    format!(
        "toc-{}",
        path.iter()
            .map(|i| (i + 1).to_string())
            .collect::<Vec<_>>()
            .join("-")
    )
}

/// One node that a chapter page renders, and the address that opens it.
///
/// The chapter row itself has no anchor: its chapter page opens at the top. A nested node has
/// the id of its row on the page, which the page renders as [`toc_id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookSection<'a> {
    pub path: Vec<usize>,
    pub title: &'a str,
    /// The id of the chapter page that shows this node.
    pub chapter: String,
    /// The anchor on that page, or `None` for the chapter itself.
    pub anchor: Option<String>,
}

/// Every node that a chapter page renders, in contents order. The compiler indexes one book
/// section document per node, and the client renders the same rows, so every target exists.
pub fn sections<'a>(book: &'a BookTree, hide: Hide<'_>) -> Vec<BookSection<'a>> {
    fn walk<'a>(
        node: &'a BookNode,
        path: &mut Vec<usize>,
        chapter: &str,
        hide: Hide<'_>,
        out: &mut Vec<BookSection<'a>>,
    ) {
        for index in listed_children(node, hide) {
            path.push(index);
            let child = &node.children[index];
            out.push(BookSection {
                path: path.clone(),
                title: &child.title,
                chapter: chapter.to_string(),
                anchor: Some(toc_id(path)),
            });
            // A hub section's children are part of its own page, not rows on this one.
            if !child.lists_children {
                walk(child, path, chapter, hide, out);
            }
            path.pop();
        }
    }

    let mut out = Vec::new();
    for row in chapter_rows(book, hide) {
        let Some(node) = node_at_path(&book.chapters, &row.path) else {
            continue;
        };
        let mut path = row.path.clone();
        out.push(BookSection {
            title: &node.title,
            chapter: row.id.clone(),
            anchor: None,
            path: path.clone(),
        });
        walk(node, &mut path, &row.id, hide, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(title: &str, entry: Option<&str>, children: Vec<BookNode>) -> BookNode {
        BookNode {
            title: title.to_string(),
            entry: entry.map(|s| EntryId::parse(s).unwrap()),
            anchor: None,
            review: None,
            lists_children: false,
            children,
        }
    }

    fn hide(_: &EntryId) -> bool {
        false
    }

    fn tree() -> BookTree {
        BookTree {
            source: "core".into(),
            title: "Core Rulebook".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![
                node(
                    "Combat",
                    Some("core/rule/combat"),
                    vec![
                        node("Actions", Some("core/rule/actions"), vec![]),
                        node(
                            "Cover",
                            None,
                            vec![node(
                                "Partial Cover",
                                Some("core/rule/partial-cover"),
                                vec![],
                            )],
                        ),
                    ],
                ),
                node("Credits", None, vec![]),
            ],
        }
    }

    #[test]
    fn chapter_ids_use_the_anchor_or_a_title_slug_and_never_the_position() {
        let tree = tree();
        let ids = chapter_ids(&tree.chapters, [vec![0].as_slice(), vec![1].as_slice()]);
        assert_eq!(
            ids,
            [
                ChapterId {
                    id: "combat".into(),
                    path: vec![0]
                },
                ChapterId {
                    id: "credits".into(),
                    path: vec![1]
                },
            ]
        );
        let mut anchored = node("Printed Heading", None, vec![]);
        anchored.anchor = Some("printed-anchor".into());
        let empty = node("— : —", None, vec![]);
        let ids = chapter_ids(&[anchored, empty], [vec![0].as_slice(), vec![1].as_slice()]);
        assert_eq!(ids[0].id, "printed-anchor");
        assert_eq!(ids[1].id, "chapter");
    }

    #[test]
    fn sections_follow_the_chapter_pages_and_carry_rendered_anchors() {
        let book = tree();
        let rows = chapter_rows(&book, &hide);
        assert_eq!(
            rows.iter()
                .map(|row| (row.id.as_str(), row.title.as_str()))
                .collect::<Vec<_>>(),
            [("combat", "Combat"), ("credits", "Credits")]
        );
        let sections = sections(&book, &hide);
        let named: Vec<(&str, &str, Option<&str>)> = sections
            .iter()
            .map(|section| {
                (
                    section.title,
                    section.chapter.as_str(),
                    section.anchor.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            named,
            [
                ("Combat", "combat", None),
                ("Actions", "combat", Some("toc-1-1")),
                ("Cover", "combat", Some("toc-1-2")),
                ("Partial Cover", "combat", Some("toc-1-2-1")),
                ("Credits", "credits", None),
            ]
        );
    }

    #[test]
    fn hidden_kinds_are_not_sections() {
        let entry = |id: &str| EntryId::parse(id).unwrap();
        let book = BookTree {
            source: "core".into(),
            title: "Core".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![node(
                "Spells",
                None,
                vec![node("Fireball", Some("core/spell/fireball"), vec![])],
            )],
        };
        let hide = |id: &EntryId| id == &entry("core/spell/fireball");
        let sections = sections(&book, &hide);
        // The chapter has no visible entry under it, so it is not a chapter page.
        assert!(sections.is_empty());
    }

    fn entry_link(target: &str) -> Block {
        Block::EntryLink {
            target: EntryId::parse(target).unwrap(),
            title: target.into(),
        }
    }

    fn rule(id: &str, body: Vec<Block>) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Rule,
            title: id.into(),
            source_ids: vec!["core".into()],
            sources: vec![],
            section: vec![],
            parent: None,
            body,
            fields: crate::TypedFields::None,
            tags: vec![],
            links: vec![],
            provenance: vec![],
            license_refs: vec![],
            relations: vec![],
        }
    }

    #[test]
    fn a_hub_collapses_only_when_its_page_links_every_entry_below_it() {
        let hub = node(
            "Combat",
            Some("core/rule/combat"),
            vec![
                node("Actions", Some("core/rule/actions"), vec![]),
                node(
                    "Cover",
                    None,
                    vec![node(
                        "Partial Cover",
                        Some("core/rule/partial-cover"),
                        vec![],
                    )],
                ),
            ],
        );
        let body = vec![
            entry_link("core/rule/actions"),
            entry_link("core/rule/partial-cover"),
        ];
        assert!(entry_lists_children(&hub, &rule("core/rule/combat", body)));

        // One missing entry link keeps the branch, even when it is a deeper child.
        let partial = vec![entry_link("core/rule/actions")];
        assert!(!entry_lists_children(
            &hub,
            &rule("core/rule/combat", partial)
        ));

        // An inline reference is not the page's list of entry links.
        let inline = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![crate::Inline::Link {
                target: EntryId::parse("core/rule/actions").unwrap(),
                anchor: None,
                c: vec![crate::Inline::text("actions")],
            }],
        }];
        assert!(!entry_lists_children(
            &hub,
            &rule("core/rule/combat", inline)
        ));

        // A node without its own entry has no page to list anything.
        let heading = node(
            "Combat",
            None,
            vec![node("Actions", Some("core/rule/actions"), vec![])],
        );
        assert!(!entry_lists_children(
            &heading,
            &rule("core/rule/combat", vec![entry_link("core/rule/actions")])
        ));
    }

    #[test]
    fn marking_a_book_records_the_hub_flag_on_the_node() {
        let books = BookIndex {
            books: vec![BookTree {
                source: "core".into(),
                title: "Core".into(),
                basis: String::new(),
                review: vec![],
                chapters: vec![node(
                    "Combat",
                    Some("core/rule/combat"),
                    vec![node("Actions", Some("core/rule/actions"), vec![])],
                )],
            }],
        };
        let entries = vec![
            rule("core/rule/combat", vec![entry_link("core/rule/actions")]),
            rule("core/rule/actions", vec![]),
        ];
        let mut marked = books.clone();
        mark_listed_children(&mut marked, &entries);
        assert!(marked.books[0].chapters[0].lists_children);
        assert!(!marked.books[0].chapters[0].children[0].lists_children);
        // The input is untouched.
        assert!(!books.books[0].chapters[0].lists_children);
    }

    #[test]
    fn a_collapsed_hub_is_not_followed_for_section_targets() {
        let mut hub = node(
            "Actions",
            Some("core/rule/actions"),
            vec![node("Charge", Some("core/rule/charge"), vec![])],
        );
        hub.lists_children = true;
        let book = BookTree {
            source: "core".into(),
            title: "Core".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![node("Combat", None, vec![hub])],
        };
        let named: Vec<&str> = sections(&book, &hide)
            .iter()
            .map(|section| section.title)
            .collect();
        // The hub section keeps its own row; the child it already lists has no separate row.
        assert_eq!(named, ["Combat", "Actions"]);
    }
}
