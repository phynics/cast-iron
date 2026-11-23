//! The reader's view of a book contents tree: which rows Contents lists, which chapter page a
//! row opens, and the anchors those pages expose.
//!
//! The compiler builds the search documents for book sections from these functions, and the
//! client renders the same rows, so a section result and its target cannot disagree. The
//! functions are pure and hold no knowledge of routes, DOM, or search.

use crate::{Block, BookIndex, BookNode, BookTree, EntryId, EntryKind, LegacyBook, RuleEntry};
use std::collections::{BTreeMap, HashMap, HashSet};

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

/// The DOM id of a row on the Contents page, which the page scrolls to once a `?c=` address has
/// resolved to the row's path. It follows the row's position, so it is never an address that
/// someone saves: a chapter page names its rows with [`section_anchors`], Contents addresses
/// name them with [`ContentsRow`], and addresses that were saved from older libraries open
/// through [`resolve_legacy_anchor`] and [`resolve_contents_row`].
pub fn toc_id(path: &[usize]) -> String {
    format!(
        "{LEGACY_ANCHOR_PREFIX}{}",
        path.iter()
            .map(|i| (i + 1).to_string())
            .collect::<Vec<_>>()
            .join("-")
    )
}

/// The prefix of a positional section anchor, the only kind that libraries up to corpus v0.1.31
/// (and the unreleased v0.1.32 and v0.1.33) used on chapter pages.
pub const LEGACY_ANCHOR_PREFIX: &str = "toc-";

/// The prefix of a stable section anchor on a chapter page.
pub const SECTION_ANCHOR_PREFIX: &str = "row-";

/// The path that a positional anchor names (`toc-2-26` is `[1, 25]`), or `None` when `anchor`
/// is not of that form. Components are 1-based, as [`toc_id`] writes them.
pub fn legacy_anchor_path(anchor: &str) -> Option<Vec<usize>> {
    let rest = anchor.strip_prefix(LEGACY_ANCHOR_PREFIX)?;
    rest.split('-')
        .map(|part| {
            let ok =
                !part.is_empty() && part.len() <= 5 && part.bytes().all(|b| b.is_ascii_digit());
            ok.then(|| part.parse::<usize>().ok())
                .flatten()
                .and_then(|n| n.checked_sub(1))
        })
        .collect()
}

/// What a row is, independent of where it sits: an entry row is its entry (and its heading
/// anchor, when the node names one), and a heading-only row is its compiler anchor or a slug of
/// its title.
fn row_identity(node: &BookNode) -> String {
    let anchor = |node: &BookNode| {
        node.anchor
            .as_deref()
            .filter(|anchor| !anchor.is_empty())
            .map(crate::headings::slugify)
            .filter(|slug| !slug.is_empty())
    };
    match &node.entry {
        Some(entry) => {
            let id = crate::headings::slugify(entry.as_str());
            match anchor(node) {
                Some(anchor) => format!("{id}--{anchor}"),
                None => id,
            }
        }
        None => {
            let own = anchor(node).unwrap_or_else(|| crate::headings::slugify(&node.title));
            format!("h-{}", if own.is_empty() { "section" } else { &own })
        }
    }
}

/// The stable anchors of every node under `chapter`, by the node's path in the book tree
/// (`chapter_path` is the chapter's own path). The chapter row has none: its page opens at the
/// top.
///
/// An anchor follows the row's identity and never its position. An entry row is
/// `row-<entry id>`; a heading-only row is `row-<parent identity>--h-<heading>`, because a
/// heading such as `Alternate Racial Traits` repeats under every race. The first row to claim an
/// anchor in the chapter keeps it, and a later one takes `-2`, `-3`, ... in tree order, so a
/// repeat of the same entry or heading under one parent still has a deterministic address.
///
/// The walk covers every node, whether Contents lists it or not: whether a kind is hidden
/// depends on the catalog and must not move an anchor. The compiler and the client call this
/// function, so a search result and the row it opens agree.
pub fn section_anchors(chapter: &BookNode, chapter_path: &[usize]) -> HashMap<Vec<usize>, String> {
    fn walk(
        node: &BookNode,
        parent: &str,
        path: &mut Vec<usize>,
        used: &mut HashSet<String>,
        out: &mut HashMap<Vec<usize>, String>,
    ) {
        for (index, child) in node.children.iter().enumerate() {
            path.push(index);
            let identity = row_identity(child);
            let base = if child.entry.is_some() {
                format!("{SECTION_ANCHOR_PREFIX}{identity}")
            } else {
                format!("{SECTION_ANCHOR_PREFIX}{parent}--{identity}")
            };
            let mut anchor = base.clone();
            let mut suffix = 2;
            while used.contains(&anchor) {
                anchor = format!("{base}-{suffix}");
                suffix += 1;
            }
            used.insert(anchor.clone());
            out.insert(path.clone(), anchor);
            walk(child, &identity, path, used, out);
            path.pop();
        }
    }
    let mut out = HashMap::new();
    walk(
        chapter,
        &row_identity(chapter),
        &mut chapter_path.to_vec(),
        &mut HashSet::new(),
        &mut out,
    );
    out
}

/// One node that a chapter page renders, and the address that opens it.
///
/// The chapter row itself has no anchor: its chapter page opens at the top. A nested node has
/// the id of its row on the page, which the page renders from [`section_anchors`].
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
        anchors: &HashMap<Vec<usize>, String>,
        out: &mut Vec<BookSection<'a>>,
    ) {
        for index in listed_children(node, hide) {
            path.push(index);
            let child = &node.children[index];
            out.push(BookSection {
                path: path.clone(),
                title: &child.title,
                chapter: chapter.to_string(),
                anchor: anchors.get(path.as_slice()).cloned(),
            });
            // A hub section's children are part of its own page, not rows on this one.
            if !child.lists_children {
                walk(child, path, chapter, hide, anchors, out);
            }
            path.pop();
        }
    }

    let mut out = Vec::new();
    for row in chapter_rows(book, hide) {
        let Some(node) = node_at_path(&book.chapters, &row.path) else {
            continue;
        };
        let anchors = section_anchors(node, &row.path);
        let mut path = row.path.clone();
        out.push(BookSection {
            title: &node.title,
            chapter: row.id.clone(),
            anchor: None,
            path: path.clone(),
        });
        walk(node, &mut path, &row.id, hide, &anchors, &mut out);
    }
    out
}

/// Where an address opens: a chapter page and the row on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionTarget {
    pub chapter: String,
    pub anchor: String,
}

/// The chapter page and row that carry `anchor` in `book`, or `None` when no rendered row has it.
/// Anchors are unique only within a chapter, so the chapter `prefer` is looked in first (the one
/// an address names), then the rest of the book in Contents order.
pub fn locate_section(
    book: &BookTree,
    hide: Hide<'_>,
    anchor: &str,
    prefer: Option<&str>,
) -> Option<SectionTarget> {
    let found = sections(book, hide)
        .into_iter()
        .filter(|section| section.anchor.as_deref() == Some(anchor))
        .map(|section| section.chapter)
        .collect::<Vec<_>>();
    let chapter = prefer
        .and_then(|prefer| found.iter().find(|chapter| chapter.as_str() == prefer))
        .or_else(|| found.first())?;
    Some(SectionTarget {
        chapter: chapter.clone(),
        anchor: anchor.to_string(),
    })
}

/// The row that an old positional address (`toc-2-26`) opens now, or `None` when the row is gone
/// and the address should open its chapter. `anchor` must be a positional anchor.
///
/// `table` is the library's legacy table for this book ([`crate::LegacySections`]): the stable anchor
/// that each published positional address stood for. A `None` table is a library built before
/// the table existed. Its own tree is the one that its positional addresses (and its search
/// index) were made against, so the row at the same path is the answer. A table that exists but
/// cannot be read is an empty `Some`, never `None`: position in the new tree is the bug this
/// table fixes.
///
/// `chapter` is the chapter id of the address. Its page is preferred when the row is in more than
/// one chapter.
pub fn resolve_legacy_anchor(
    book: &BookTree,
    hide: Hide<'_>,
    table: Option<&BTreeMap<String, String>>,
    chapter: &str,
    anchor: &str,
) -> Option<SectionTarget> {
    let path = legacy_anchor_path(anchor)?;
    match table {
        Some(table) => locate_section(book, hide, table.get(anchor)?, Some(chapter)),
        None => {
            let section = sections(book, hide)
                .into_iter()
                .find(|section| section.path == path)?;
            Some(SectionTarget {
                chapter: section.chapter,
                anchor: section.anchor?,
            })
        }
    }
}

/// A Contents row as an address (`#/browse/book/<id>?c=…`) names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentsRow {
    /// By position in the tree, as `?c=3.2` did up to corpus v0.1.31: 0-based path. A pin that
    /// adds or removes a row moves it, so nothing writes this form any more; an address that
    /// carries it opens through [`resolve_contents_row`].
    Position(Vec<usize>),
    /// By identity: the permanent id of the chapter page that holds the row, and, below chapter
    /// level, the row's stable anchor on that page ([`section_anchors`]). Neither moves when a
    /// pin changes the tree.
    Stable {
        chapter: String,
        anchor: Option<String>,
    },
}

/// The stable address of the Contents row at `path` of `book`, or `None` when `path` names no
/// row of a listed chapter. This is the form that the trail above an entry writes.
pub fn contents_row(book: &BookTree, hide: Hide<'_>, path: &[usize]) -> Option<ContentsRow> {
    let first = *path.first()?;
    let row = chapter_rows(book, hide)
        .into_iter()
        .find(|row| row.path == [first])?;
    if path.len() == 1 {
        return Some(ContentsRow::Stable {
            chapter: row.id,
            anchor: None,
        });
    }
    let node = node_at_path(&book.chapters, &row.path)?;
    let anchor = section_anchors(node, &row.path).remove(path)?;
    Some(ContentsRow::Stable {
        chapter: row.id,
        anchor: Some(anchor),
    })
}

/// The tree path of the section that carries `anchor` on the chapter page `chapter`.
fn section_path(
    book: &BookTree,
    hide: Hide<'_>,
    chapter: &str,
    anchor: &str,
) -> Option<Vec<usize>> {
    sections(book, hide)
        .into_iter()
        .find(|section| section.chapter == chapter && section.anchor.as_deref() == Some(anchor))
        .map(|section| section.path)
}

/// The path of the Contents row that an address opens in `book`, or `None` when the row is gone
/// (the page then opens at the top of Contents).
///
/// A stable address finds its chapter by id and its row by anchor. A row that moved to another
/// chapter is still found, as with section anchors ([`locate_section`]).
///
/// A positional address (`?c=3.2`) goes through `legacy`, the library's table for this book:
/// the chapter id of a top-level row, or the stable anchor of a row below it, that the last
/// published tree gave that position. A `None` table is a library built before the table
/// existed, whose own tree made the position. A table that exists but could not be read is an
/// empty `Some`, never `None`: reading the position against the new tree is the bug this fixes.
pub fn resolve_contents_row(
    book: &BookTree,
    hide: Hide<'_>,
    legacy: Option<&LegacyBook>,
    row: &ContentsRow,
) -> Option<Vec<usize>> {
    let chapter_path = |chapter: &str| {
        chapter_rows(book, hide)
            .into_iter()
            .find(|row| row.id == chapter)
            .map(|row| row.path)
    };
    match row {
        ContentsRow::Stable {
            chapter,
            anchor: None,
        } => chapter_path(chapter),
        ContentsRow::Stable {
            chapter,
            anchor: Some(anchor),
        } => {
            let target = locate_section(book, hide, anchor, Some(chapter))?;
            section_path(book, hide, &target.chapter, &target.anchor)
        }
        ContentsRow::Position(path) => {
            let first = *path.first()?;
            let Some(legacy) = legacy else {
                return node_at_path(&book.chapters, path).map(|_| path.clone());
            };
            let chapter = legacy.chapters.get(&toc_id(&[first]));
            if path.len() == 1 {
                return chapter_path(chapter?);
            }
            let anchor = legacy.sections.get(&toc_id(path))?;
            let target = locate_section(book, hide, anchor, chapter.map(String::as_str))?;
            section_path(book, hide, &target.chapter, &target.anchor)
        }
    }
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
            collapse: false,
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
                ("Actions", "combat", Some("row-core-rule-actions")),
                ("Cover", "combat", Some("row-core-rule-combat--h-cover")),
                (
                    "Partial Cover",
                    "combat",
                    Some("row-core-rule-partial-cover")
                ),
                ("Credits", "credits", None),
            ]
        );
    }

    fn classes(names: &[&str]) -> BookTree {
        BookTree {
            source: "blood-of-the-beast".into(),
            title: "Blood of the Beast".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![
                node("Introduction", None, vec![]),
                node(
                    "Classes",
                    None,
                    names
                        .iter()
                        .map(|name| {
                            node(
                                name,
                                Some(&format!("ref/class/{}", name.to_lowercase())),
                                vec![],
                            )
                        })
                        .collect(),
                ),
            ],
        }
    }

    fn anchor_of(book: &BookTree, title: &str) -> Option<String> {
        sections(book, &hide)
            .into_iter()
            .find(|section| section.title == title)
            .and_then(|section| section.anchor)
    }

    #[test]
    fn an_anchor_follows_the_row_and_never_its_position() {
        let before = classes(&["Paladin", "Ranger", "Rogue"]);
        // Two rows arrive ahead of Ranger and Rogue, and one is removed.
        let after = classes(&["Alchemist", "Barbarian", "Ranger", "Rogue"]);
        for title in ["Ranger", "Rogue"] {
            assert_eq!(
                anchor_of(&before, title),
                anchor_of(&after, title),
                "{title}"
            );
        }
        assert_eq!(
            anchor_of(&before, "Ranger").as_deref(),
            Some("row-ref-class-ranger")
        );
        // The same rows sit at other positions: the old positional id differs.
        assert_ne!(
            toc_id(&sections(&before, &hide)[3].path),
            toc_id(&sections(&after, &hide)[4].path)
        );
    }

    #[test]
    fn a_repeated_row_keeps_the_first_anchor_and_the_next_takes_a_number() {
        let mut chapter = node(
            "Races",
            None,
            vec![
                node("Dwarves", Some("core/race/dwarf"), vec![]),
                node("Dwarves again", Some("core/race/dwarf"), vec![]),
                node("Dwarves a third time", Some("core/race/dwarf"), vec![]),
            ],
        );
        // A heading that repeats under different parents is told apart by its parent.
        chapter.children.push(node(
            "Alternate Traits",
            None,
            vec![node("Alternate Traits", None, vec![])],
        ));
        chapter.children.push(node(
            "Other",
            Some("core/rule/other"),
            vec![node("Alternate Traits", None, vec![])],
        ));
        let anchors = section_anchors(&chapter, &[3]);
        let at = |path: &[usize]| anchors[path].as_str();
        assert_eq!(at(&[3, 0]), "row-core-race-dwarf");
        assert_eq!(at(&[3, 1]), "row-core-race-dwarf-2");
        assert_eq!(at(&[3, 2]), "row-core-race-dwarf-3");
        assert_eq!(at(&[3, 3]), "row-h-races--h-alternate-traits");
        assert_eq!(at(&[3, 3, 0]), "row-h-alternate-traits--h-alternate-traits");
        assert_eq!(at(&[3, 4, 0]), "row-core-rule-other--h-alternate-traits");
        let unique: HashSet<&String> = anchors.values().collect();
        assert_eq!(unique.len(), anchors.len());
        // The chapter row itself has no anchor, and a heading claims its compiler anchor.
        assert!(!anchors.contains_key(&[3][..]));
        let mut named = node("Printed Heading", None, vec![]);
        named.anchor = Some("Printed Anchor".into());
        let chapter = node("Chapter", Some("core/rule/chapter"), vec![named]);
        assert_eq!(
            section_anchors(&chapter, &[0])[&[0, 0][..]],
            "row-core-rule-chapter--h-printed-anchor"
        );
    }

    #[test]
    fn what_a_kind_hides_does_not_move_an_anchor() {
        let book = classes(&["Paladin", "Ranger", "Rogue"]);
        let hide_paladin = |id: &EntryId| id.as_str() == "ref/class/paladin";
        let anchor = |hide: Hide<'_>| {
            sections(&book, hide)
                .into_iter()
                .find(|section| section.title == "Rogue")
                .and_then(|section| section.anchor)
        };
        assert_eq!(anchor(&hide), anchor(&hide_paladin));
    }

    #[test]
    fn a_positional_anchor_names_its_path() {
        assert_eq!(legacy_anchor_path("toc-2-26"), Some(vec![1, 25]));
        assert_eq!(legacy_anchor_path("toc-1"), Some(vec![0]));
        for bad in [
            "toc-",
            "toc-0",
            "toc-1-",
            "toc-a",
            "toc-1--2",
            "row-ref-class-ranger",
            "toc-123456",
            "",
        ] {
            assert_eq!(legacy_anchor_path(bad), None, "{bad:?}");
        }
        // The Contents page keeps writing the positional id that `?c=` scrolls to.
        assert_eq!(toc_id(&[1, 25]), "toc-2-26");
        assert_eq!(legacy_anchor_path(&toc_id(&[1, 25])), Some(vec![1, 25]));
    }

    #[test]
    fn an_old_address_opens_the_row_it_stood_for_and_a_gone_row_opens_the_chapter() {
        // The published tree has Paladin, Ranger and Rogue at toc-2-1, toc-2-2 and toc-2-3.
        let published = classes(&["Paladin", "Ranger", "Rogue"]);
        let table: BTreeMap<String, String> = sections(&published, &hide)
            .into_iter()
            .filter_map(|section| Some((toc_id(&section.path), section.anchor?)))
            .collect();
        assert_eq!(table["toc-2-2"], "row-ref-class-ranger");
        assert_eq!(table["toc-2-3"], "row-ref-class-rogue");

        // A later pin adds Alchemist and Barbarian ahead of Ranger and drops Paladin. Read by
        // position, toc-2-2 would now open Barbarian and toc-2-3 Ranger.
        let current = classes(&["Alchemist", "Barbarian", "Ranger", "Rogue"]);
        let open = |old: &str| resolve_legacy_anchor(&current, &hide, Some(&table), "classes", old);
        assert_eq!(
            open("toc-2-2"),
            Some(SectionTarget {
                chapter: "classes".into(),
                anchor: "row-ref-class-ranger".into()
            })
        );
        assert_eq!(
            open("toc-2-3").map(|target| target.anchor),
            Some("row-ref-class-rogue".into())
        );
        // Paladin is gone: its address opens the chapter.
        assert_eq!(open("toc-2-1"), None);
        // No published tree made this address: it opens the chapter too.
        assert_eq!(open("toc-2-99"), None);
        // Not a positional address at all.
        assert_eq!(open("row-ref-class-ranger"), None);
    }

    #[test]
    fn a_library_without_the_table_resolves_an_old_address_against_its_own_tree() {
        // That library's own search index and tree made the positional addresses.
        let book = classes(&["Paladin", "Ranger", "Rogue"]);
        assert_eq!(
            resolve_legacy_anchor(&book, &hide, None, "classes", "toc-2-2"),
            Some(SectionTarget {
                chapter: "classes".into(),
                anchor: "row-ref-class-ranger".into()
            })
        );
        assert_eq!(
            resolve_legacy_anchor(&book, &hide, None, "classes", "toc-2-9"),
            None
        );
    }

    #[test]
    fn a_row_in_two_chapters_opens_in_the_chapter_the_address_names() {
        let ranger = |title: &str| {
            node(
                title,
                None,
                vec![node("Ranger", Some("ref/class/ranger"), vec![])],
            )
        };
        let book = BookTree {
            source: "b".into(),
            title: "B".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![ranger("First"), ranger("Second")],
        };
        let at = |prefer: Option<&str>| {
            locate_section(&book, &hide, "row-ref-class-ranger", prefer).map(|t| t.chapter)
        };
        assert_eq!(at(Some("second")).as_deref(), Some("second"));
        assert_eq!(at(Some("first")).as_deref(), Some("first"));
        // No preference, or a chapter that does not hold the row: the first that does.
        assert_eq!(at(None).as_deref(), Some("first"));
        assert_eq!(at(Some("third")).as_deref(), Some("first"));
        let table: BTreeMap<String, String> =
            [("toc-2-1".to_string(), "row-ref-class-ranger".to_string())].into();
        assert_eq!(
            resolve_legacy_anchor(&book, &hide, Some(&table), "second", "toc-2-1")
                .map(|t| t.chapter)
                .as_deref(),
            Some("second")
        );
        // A table that could not be read is an empty table: nothing resolves by position.
        let empty = BTreeMap::new();
        assert_eq!(
            resolve_legacy_anchor(&book, &hide, Some(&empty), "second", "toc-2-1"),
            None
        );
    }

    #[test]
    fn locating_a_section_names_the_chapter_that_holds_it() {
        let book = classes(&["Paladin", "Ranger"]);
        assert_eq!(
            locate_section(&book, &hide, "row-ref-class-ranger", None),
            Some(SectionTarget {
                chapter: "classes".into(),
                anchor: "row-ref-class-ranger".into()
            })
        );
        assert_eq!(
            locate_section(&book, &hide, "row-ref-class-rogue", None),
            None
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
            expanded_in: vec![],
            stat_sets: vec![],
            section: vec![],
            parent: None,
            body,
            fields: crate::TypedFields::None,
            tags: vec![],
            race_group: None,
            links: vec![],
            aon: None,
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

    /// The table that the build of `published` would hold for its Contents rows.
    fn legacy_of(published: &BookTree) -> LegacyBook {
        LegacyBook {
            sections: sections(published, &hide)
                .into_iter()
                .filter_map(|section| Some((toc_id(&section.path), section.anchor?)))
                .collect(),
            chapters: chapter_rows(published, &hide)
                .into_iter()
                .map(|row| (toc_id(&row.path), row.id))
                .collect(),
        }
    }

    #[test]
    fn a_contents_address_names_the_row_and_survives_a_pin_that_adds_rows() {
        let published = classes(&["Paladin", "Ranger", "Rogue"]);
        let ranger = contents_row(&published, &hide, &[1, 1]).unwrap();
        assert_eq!(
            ranger,
            ContentsRow::Stable {
                chapter: "classes".into(),
                anchor: Some("row-ref-class-ranger".into())
            }
        );
        let chapter = contents_row(&published, &hide, &[1]).unwrap();
        assert_eq!(
            chapter,
            ContentsRow::Stable {
                chapter: "classes".into(),
                anchor: None
            }
        );
        // A later pin adds a chapter in front and two classes before Ranger.
        let mut current = classes(&["Alchemist", "Barbarian", "Paladin", "Ranger", "Rogue"]);
        current
            .chapters
            .insert(0, node("Foreword", Some("ref/rule/foreword"), vec![]));
        assert_eq!(
            resolve_contents_row(&current, &hide, None, &ranger),
            Some(vec![2, 3])
        );
        assert_eq!(
            resolve_contents_row(&current, &hide, None, &chapter),
            Some(vec![2])
        );
        // The row that left the book, and a chapter that never existed, open the top.
        let gone = classes(&["Alchemist"]);
        assert_eq!(resolve_contents_row(&gone, &hide, None, &ranger), None);
        let unknown = ContentsRow::Stable {
            chapter: "nope".into(),
            anchor: None,
        };
        assert_eq!(resolve_contents_row(&current, &hide, None, &unknown), None);
    }

    #[test]
    fn a_row_that_moved_to_another_chapter_is_still_found() {
        let ranger = ContentsRow::Stable {
            chapter: "introduction".into(),
            anchor: Some("row-ref-class-ranger".into()),
        };
        let current = classes(&["Ranger"]);
        assert_eq!(
            resolve_contents_row(&current, &hide, None, &ranger),
            Some(vec![1, 0])
        );
    }

    #[test]
    fn an_old_positional_contents_address_opens_the_row_it_stood_for() {
        let published = classes(&["Paladin", "Ranger", "Rogue"]);
        let legacy = legacy_of(&published);
        assert_eq!(legacy.chapters["toc-2"], "classes");
        let current = classes(&["Alchemist", "Barbarian", "Ranger", "Rogue"]);
        let open = |path: Vec<usize>| {
            resolve_contents_row(&current, &hide, Some(&legacy), &ContentsRow::Position(path))
        };
        // `?c=2.2` was Ranger; the same position now holds Barbarian.
        assert_eq!(open(vec![1, 1]), Some(vec![1, 2]));
        assert_eq!(open(vec![1, 2]), Some(vec![1, 3]));
        // The chapter row.
        assert_eq!(open(vec![1]), Some(vec![1]));
        // Paladin is gone, and no published tree made `?c=2.9`: the top of Contents.
        assert_eq!(open(vec![1, 0]), None);
        assert_eq!(open(vec![1, 8]), None);
        assert_eq!(open(vec![7]), None);
        // The chapter moves when a chapter is inserted before it; its id does not.
        let mut shifted = current.clone();
        shifted
            .chapters
            .insert(0, node("Foreword", Some("ref/rule/f"), vec![]));
        assert_eq!(
            resolve_contents_row(
                &shifted,
                &hide,
                Some(&legacy),
                &ContentsRow::Position(vec![1])
            ),
            Some(vec![2])
        );
    }

    #[test]
    fn a_library_without_the_table_reads_a_position_against_its_own_tree() {
        let book = classes(&["Paladin", "Ranger"]);
        let at = |path: Vec<usize>| {
            resolve_contents_row(&book, &hide, None, &ContentsRow::Position(path))
        };
        assert_eq!(at(vec![1, 1]), Some(vec![1, 1]));
        assert_eq!(at(vec![1, 9]), None);
    }

    #[test]
    fn a_table_that_could_not_be_read_never_falls_back_to_position() {
        let book = classes(&["Alchemist", "Ranger"]);
        let empty = LegacyBook::default();
        for path in [vec![1], vec![1, 1]] {
            assert_eq!(
                resolve_contents_row(&book, &hide, Some(&empty), &ContentsRow::Position(path)),
                None
            );
        }
    }
}
