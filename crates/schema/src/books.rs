//! The reader's view of a book contents tree: which rows Contents lists, which chapter page a
//! row opens, and the anchors those pages expose.
//!
//! The compiler builds the search documents for book sections from these functions, and the
//! client renders the same rows, so a section result and its target cannot disagree. The
//! functions are pure and hold no knowledge of routes, DOM, or search.

use crate::{BookNode, BookTree, EntryId, EntryKind};

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
    if is_catch_all(node) {
        return false;
    }
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

/// The title that the compiler gives the chapter of entries that no curated chapter places.
pub const CATCH_ALL_TITLE: &str = "New entries (page order)";

/// Whether a node is the compiler's catch-all chapter. The app does not show it: its entries
/// reach the reader through their printed chapters, links, or list pages (#363).
pub fn is_catch_all(node: &BookNode) -> bool {
    node.title == CATCH_ALL_TITLE
}

/// The paths of the rows at the top of Contents. Every listed chapter is one row; the catch-all
/// is never listed. Paths are real: they index the whole tree, not the listed rows.
pub fn contents_roots(chapters: &[BookNode], hide: Hide<'_>) -> Vec<Vec<usize>> {
    (0..chapters.len())
        .filter(|&i| shows_in_contents(&chapters[i], hide))
        .map(|i| vec![i])
        .collect()
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
/// title the reader sees.
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
            out.push(BookSection {
                path: path.clone(),
                title: &node.children[index].title,
                chapter: chapter.to_string(),
                anchor: Some(toc_id(path)),
            });
            walk(&node.children[index], path, chapter, hide, out);
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
    fn the_catch_all_is_never_a_contents_row_or_a_section() {
        let book = BookTree {
            source: "core".into(),
            title: "Core".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![
                node(
                    CATCH_ALL_TITLE,
                    None,
                    vec![node("Grapple", Some("core/rule/grapple"), vec![])],
                ),
                node(
                    "Combat",
                    None,
                    vec![node("Cover", Some("core/rule/cover"), vec![])],
                ),
            ],
        };
        assert_eq!(contents_roots(&book.chapters, &hide), vec![vec![1]]);
        assert_eq!(
            chapter_rows(&book, &hide)
                .iter()
                .map(|row| (row.id.as_str(), row.title.as_str()))
                .collect::<Vec<_>>(),
            [("combat", "Combat")]
        );
        assert_eq!(
            sections(&book, &hide)
                .iter()
                .map(|section| section.title)
                .collect::<Vec<_>>(),
            ["Combat", "Cover"]
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
}
