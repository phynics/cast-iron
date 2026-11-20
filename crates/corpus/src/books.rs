//! The book index of a corpus in the split layout (format 3, #425).
//!
//! Only the hand-curated books are stored, one `books/<publication>.toml` each. Every other
//! book is derived from the entries' `[[sources]]` pages by [`book_index`], a pure function that
//! the compiler and `corpus-check` both call, so the reachability gate sees the books exactly as
//! the client will. The derivation is deterministic: it sorts, and it never iterates a hash map.
//!
//! The rules, as the owner decided them on 2026-11-17 (`docs/decisions/`):
//!
//! * **Members.** A book holds every entry whose `source_ids` name it.
//! * **Page.** The first `[[sources]]` row for that book with a page, read as its leading
//!   integer: `12-14` and `12, 14` both give 12. No row, or no readable page, means no page.
//! * **Chapters.** `Page order` holds the entries with a page; `Unpaged` holds the rest. A chapter
//!   with no entry is left out.
//! * **Order.** Page, then kind slug, then lowercased title, then id. An entry with no page sorts
//!   after the paged ones by the last three keys alone. The id makes the order total.
//! * **Nesting.** An entry whose `parent` chain reaches another member nests under the nearest
//!   one, in the host's chapter and in the same order.
//! * **Curated books** keep their stored chapters and gain a last, hidden
//!   `New entries (page order)` chapter that holds the members the stored tree does not place.
//! * **Book order.** Curated books first, then derived books; each group by the title folded to
//!   lowercase ASCII letters and digits, then the id.

use crate::files::Publication;
use crate::load::Stored;
use cast_iron_schema::books::CATCH_ALL_TITLE;
use cast_iron_schema::{BookIndex, BookNode, BookTree, RuleEntry};
use std::collections::{BTreeMap, BTreeSet};

/// The directory of the curated books.
pub const BOOKS_DIR: &str = "books";

/// The chapter of a derived book that holds the entries with a page. The client matches the title.
pub const PAGE_ORDER_TITLE: &str = "Page order";

/// The chapter of a derived book that holds the entries with no page. The client matches the title.
pub const UNPAGED_TITLE: &str = "Unpaged";

const PAGE_ORDER_REVIEW: &str = "Entries follow their page number. Entries on the same page, and a child nested under its host, are not printed-order evidence.";
const UNPAGED_REVIEW: &str = "These entries name this publication but their source line gives no page number, so their position is unknown.";
const CATCH_ALL_REVIEW: &str = "Canonical entries of this publication that the curated PSRD tree does not place. They follow their page number; this is a page-ordered list, not a chapter position.";
const BOOK_REVIEW: [&str; 2] = [
    "Page attribution orders entries; it does not establish chapter boundaries or titles.",
    "Entries on the same page are not in printed order.",
];
const BASIS_PAGE_ORDER: &str = "Entries are ordered by the page each Pf Data record attributes to this publication. No printed chapter boundaries are known for this publication; this is a page-ordered list, not a chapter structure.";
const BASIS_BOTH: &str = "Entries with a page are ordered by that page; entries without one follow in an Unpaged section. No printed chapter boundaries are known for this publication; this is a page-ordered list, not a chapter structure.";
const BASIS_UNPAGED: &str = "No page number appears in this publication's Pf Data source lines, so the contents is one Unpaged list. Page order is evidence for entry order, not for chapter boundaries.";

/// How a corpus stores its books.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookLayout {
    /// One `books.toml` that holds every book, as formats 1 and 2 write it.
    Single,
    /// One `books/<publication>.toml` per curated book; the compiler derives the others.
    Split,
}

/// True for a chapter title that belongs to the compiler: a curated file may not use it.
pub fn is_reserved_chapter(title: &str) -> bool {
    title == PAGE_ORDER_TITLE || title == UNPAGED_TITLE || title == CATCH_ALL_TITLE
}

/// The leading integer of a printed page: `12-14` and `12, 14` give 12; `inside cover` gives none.
fn lead_page(page: &str) -> Option<u32> {
    let digits: String = page
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// The page of `entry` in `book`: the first `[[sources]]` row for the book that has a readable page.
pub fn page_in(entry: &RuleEntry, book: &str) -> Option<u32> {
    entry
        .sources
        .iter()
        .filter(|row| row.publication == book)
        .find_map(|row| row.page.as_deref().and_then(lead_page))
}

/// Whether a printed page is one the derivation reads exactly: `N`, `N-N`, a comma list of those,
/// or `inside cover`. Anything else (a stray citation glued to a page, say) would give the entry a
/// wrong leading page or none.
pub fn is_clean_page(page: &str) -> bool {
    fn range(part: &str) -> bool {
        let mut numbers = part.split('-');
        let ok = |piece: Option<&str>| {
            piece
                .is_some_and(|piece| !piece.is_empty() && piece.bytes().all(|b| b.is_ascii_digit()))
        };
        ok(numbers.next())
            && (match numbers.next() {
                None => true,
                Some(second) => ok(Some(second)) && numbers.next().is_none(),
            })
    }
    page == "inside cover"
        || page
            .split(',')
            .all(|part| range(part.strip_prefix(' ').unwrap_or(part)))
}

/// The members of a book in print order, with their nesting: one node per entry, nested under the
/// nearest member on its `parent` chain. Each root comes with whether it has a page.
/// The nearest member on the `parent` chain of `entry`, if any.
fn host_of<'a>(
    entry: &'a RuleEntry,
    member_ids: &BTreeSet<&str>,
    entries: &'a BTreeMap<String, Stored>,
) -> Option<&'a str> {
    let mut seen = BTreeSet::new();
    let mut parent = entry.parent.as_ref().map(|id| id.as_str());
    while let Some(id) = parent {
        if member_ids.contains(id) {
            return Some(id);
        }
        if !seen.insert(id) {
            return None;
        }
        parent = entries
            .get(id)
            .and_then(|stored| stored.entry.parent.as_ref())
            .map(|id| id.as_str());
    }
    None
}

fn order_members<'a>(
    book: &str,
    members: &[&'a RuleEntry],
    entries: &'a BTreeMap<String, Stored>,
) -> Vec<(bool, BookNode)> {
    let member_ids: BTreeSet<&str> = members.iter().map(|entry| entry.id.as_str()).collect();

    let mut children: BTreeMap<Option<&str>, Vec<&RuleEntry>> = BTreeMap::new();
    for &entry in members {
        children
            .entry(host_of(entry, &member_ids, entries))
            .or_default()
            .push(entry);
    }
    let sort = |list: &mut Vec<&RuleEntry>| {
        list.sort_by_cached_key(|entry| {
            let page = page_in(entry, book);
            (
                page.is_none(),
                page.unwrap_or(0),
                entry.kind.slug(),
                entry.title.to_lowercase(),
                entry.id.to_string(),
            )
        });
    };
    for list in children.values_mut() {
        sort(list);
    }

    fn node(
        entry: &RuleEntry,
        children: &BTreeMap<Option<&str>, Vec<&RuleEntry>>,
        placed: &mut BTreeSet<String>,
    ) -> BookNode {
        placed.insert(entry.id.to_string());
        let mut nested = Vec::new();
        for child in children
            .get(&Some(entry.id.as_str()))
            .map_or(&[][..], Vec::as_slice)
        {
            // A cycle in the `parent` chains would otherwise come back to a placed entry.
            if !placed.contains(child.id.as_str()) {
                nested.push(node(child, children, placed));
            }
        }
        BookNode {
            title: entry.title.clone(),
            entry: Some(entry.id.clone()),
            anchor: None,
            review: None,
            collapse: false,
            lists_children: false,
            children: nested,
        }
    }

    let mut placed = BTreeSet::new();
    let mut roots: Vec<(bool, BookNode)> = children
        .get(&None)
        .map(|list| {
            list.iter()
                .map(|&entry| {
                    (
                        page_in(entry, book).is_some(),
                        node(entry, &children, &mut placed),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    // A cycle in the `parent` chains would leave members with a host but no root above them.
    // `corpus-check` rejects such a cycle; the derivation still places every member.
    let mut orphans: Vec<&RuleEntry> = members
        .iter()
        .copied()
        .filter(|entry| !placed.contains(entry.id.as_str()))
        .collect();
    sort(&mut orphans);
    for entry in orphans {
        if !placed.contains(entry.id.as_str()) {
            roots.push((
                page_in(entry, book).is_some(),
                node(entry, &children, &mut placed),
            ));
        }
    }
    roots
}

fn chapter(title: &str, review: &str, children: Vec<BookNode>) -> BookNode {
    BookNode {
        title: title.to_string(),
        entry: None,
        anchor: None,
        review: Some(review.to_string()),
        collapse: false,
        lists_children: false,
        children,
    }
}

/// The book of a publication that has no curated file.
fn derived_book(
    publication: &Publication,
    members: &[&RuleEntry],
    entries: &BTreeMap<String, Stored>,
) -> BookTree {
    let (mut paged, mut unpaged) = (Vec::new(), Vec::new());
    for (has_page, node) in order_members(&publication.id, members, entries) {
        if has_page {
            paged.push(node);
        } else {
            unpaged.push(node);
        }
    }
    let basis = match (paged.is_empty(), unpaged.is_empty()) {
        (false, true) => BASIS_PAGE_ORDER,
        (false, false) => BASIS_BOTH,
        (true, false) => BASIS_UNPAGED,
        (true, true) => "",
    };
    let mut chapters = Vec::new();
    if !paged.is_empty() {
        chapters.push(chapter(PAGE_ORDER_TITLE, PAGE_ORDER_REVIEW, paged));
    }
    if !unpaged.is_empty() {
        chapters.push(chapter(UNPAGED_TITLE, UNPAGED_REVIEW, unpaged));
    }
    BookTree {
        source: publication.id.clone(),
        title: publication.title.clone(),
        basis: basis.to_string(),
        review: if chapters.is_empty() {
            Vec::new()
        } else {
            BOOK_REVIEW.map(str::to_string).to_vec()
        },
        chapters,
    }
}

/// Every entry a stored tree places, with or without an anchor.
fn placed_ids<'a>(nodes: &'a [BookNode], out: &mut BTreeSet<&'a str>) {
    for node in nodes {
        if let Some(entry) = &node.entry {
            out.insert(entry.as_str());
        }
        placed_ids(&node.children, out);
    }
}

/// A curated tree and the hidden chapter of the members it does not place.
fn curated_book(
    tree: &BookTree,
    members: &[&RuleEntry],
    entries: &BTreeMap<String, Stored>,
) -> BookTree {
    let mut placed = BTreeSet::new();
    placed_ids(&tree.chapters, &mut placed);
    let unplaced: Vec<&RuleEntry> = members
        .iter()
        .copied()
        .filter(|entry| !placed.contains(entry.id.as_str()))
        .collect();
    let mut book = tree.clone();
    if !unplaced.is_empty() {
        let nodes = order_members(&tree.source, &unplaced, entries)
            .into_iter()
            .map(|(_, node)| node)
            .collect();
        book.chapters
            .push(chapter(CATCH_ALL_TITLE, CATCH_ALL_REVIEW, nodes));
    }
    book
}

/// The title folded to lowercase ASCII letters and digits, the sort key of the book list.
fn fold(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// The books of a corpus in the split layout: every curated tree with its catch-all chapter, and a
/// derived tree for each publication that has no curated tree.
pub fn book_index(
    curated: &[BookTree],
    publications: &[Publication],
    entries: &BTreeMap<String, Stored>,
) -> BookIndex {
    let mut members: BTreeMap<&str, Vec<&RuleEntry>> = BTreeMap::new();
    for stored in entries.values() {
        let mut seen = BTreeSet::new();
        for source in &stored.entry.source_ids {
            if seen.insert(source.as_str()) {
                members
                    .entry(source.as_str())
                    .or_default()
                    .push(&stored.entry);
            }
        }
    }
    let none: Vec<&RuleEntry> = Vec::new();
    let members_of = |source: &str| members.get(source).unwrap_or(&none).as_slice();

    let mut stored: Vec<BookTree> = curated
        .iter()
        .map(|tree| curated_book(tree, members_of(&tree.source), entries))
        .collect();
    stored.sort_by_cached_key(|tree| (fold(&tree.title), tree.source.clone()));

    let curated_ids: BTreeSet<&str> = curated.iter().map(|tree| tree.source.as_str()).collect();
    let mut derived: Vec<BookTree> = publications
        .iter()
        .filter(|publication| !curated_ids.contains(publication.id.as_str()))
        .map(|publication| derived_book(publication, members_of(&publication.id), entries))
        .collect();
    derived.sort_by_cached_key(|tree| (fold(&tree.title), tree.source.clone()));

    stored.extend(derived);
    BookIndex { books: stored }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::rule;
    use cast_iron_schema::{Citation, EntryId, EntryKind};

    fn entry(id: &str, kind: EntryKind, title: &str, rows: &[(&str, Option<&str>)]) -> Stored {
        let mut entry = rule(id, vec![]);
        entry.kind = kind;
        entry.title = title.to_string();
        entry.source_ids = Vec::new();
        for (publication, page) in rows {
            if !entry.source_ids.iter().any(|source| source == publication) {
                entry.source_ids.push(publication.to_string());
            }
            if page.is_some() {
                entry.sources.push(Citation {
                    publication: publication.to_string(),
                    page: page.map(str::to_string),
                });
            }
        }
        Stored {
            path: crate::entry::entry_path(&entry.id),
            text: String::new(),
            block_lines: Vec::new(),
            entry,
        }
    }

    fn publication(id: &str, title: &str) -> Publication {
        Publication {
            id: id.into(),
            title: title.into(),
            aon: None,
            copyright: None,
            category: None,
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
            suffix: None,
        }
    }

    fn map(list: Vec<Stored>) -> BTreeMap<String, Stored> {
        list.into_iter()
            .map(|stored| (stored.entry.id.to_string(), stored))
            .collect()
    }

    fn titles(nodes: &[BookNode]) -> Vec<&str> {
        nodes.iter().map(|node| node.title.as_str()).collect()
    }

    #[test]
    fn a_page_is_the_leading_integer_of_the_first_row_that_has_one() {
        assert_eq!(lead_page("12-14"), Some(12));
        assert_eq!(lead_page("12, 14"), Some(12));
        assert_eq!(lead_page("263-4"), Some(263));
        assert_eq!(lead_page("inside cover"), None);
        let mut stored = entry("ref/rule/a", EntryKind::Rule, "A", &[("b", Some("x"))]);
        assert_eq!(page_in(&stored.entry, "b"), None);
        stored.entry.sources = vec![
            Citation {
                publication: "b".into(),
                page: Some("inside cover".into()),
            },
            Citation {
                publication: "c".into(),
                page: Some("9".into()),
            },
            Citation {
                publication: "b".into(),
                page: Some("40-41".into()),
            },
        ];
        assert_eq!(page_in(&stored.entry, "b"), Some(40));
        assert_eq!(page_in(&stored.entry, "c"), Some(9));
        assert_eq!(page_in(&stored.entry, "d"), None);
    }

    #[test]
    fn a_clean_page_is_a_number_a_range_a_list_or_the_inside_cover() {
        for page in [
            "12",
            "12-14",
            "263-4",
            "12, 14",
            "12,14",
            "9-10, 12",
            "inside cover",
        ] {
            assert!(is_clean_page(page), "{page}");
        }
        for page in [
            "",
            "58~Occult Origins/8",
            "39;Pathfinder #46: Wake of the Watcher/77",
            "12-",
            "-3",
            "1-2-3",
            "back cover",
            "12 ,14",
        ] {
            assert!(!is_clean_page(page), "{page:?}");
        }
    }

    #[test]
    fn a_derived_book_orders_by_page_then_kind_title_and_id() {
        let entries = map(vec![
            entry(
                "ref/rule/late",
                EntryKind::Rule,
                "Late",
                &[("b", Some("30"))],
            ),
            entry(
                "ref/spell/b",
                EntryKind::Spell,
                "Beta",
                &[("b", Some("12"))],
            ),
            entry("ref/feat/z", EntryKind::Feat, "Zed", &[("b", Some("12"))]),
            entry(
                "ref/feat/a2",
                EntryKind::Feat,
                "alpha",
                &[("b", Some("12"))],
            ),
            entry(
                "ref/feat/a1",
                EntryKind::Feat,
                "Alpha",
                &[("b", Some("12"))],
            ),
            entry("ref/rule/none-b", EntryKind::Rule, "Nope", &[("b", None)]),
            entry(
                "ref/rule/none-a",
                EntryKind::Rule,
                "Aardvark",
                &[("b", None)],
            ),
        ]);
        let index = book_index(&[], &[publication("b", "Book")], &entries);
        let book = &index.books[0];
        assert_eq!(titles(&book.chapters), ["Page order", "Unpaged"]);
        // Page 12: kind slug (feat < spell), then lowercased title, then id.
        let order: Vec<_> = book.chapters[0]
            .children
            .iter()
            .map(|n| n.entry.as_ref().unwrap().as_str())
            .collect();
        assert_eq!(
            order,
            [
                "ref/feat/a1",
                "ref/feat/a2",
                "ref/feat/z",
                "ref/spell/b",
                "ref/rule/late"
            ]
        );
        assert_eq!(
            titles(&book.chapters[1].children),
            ["Aardvark", "Nope"],
            "unpaged entries follow by the same tie-break"
        );
        assert_eq!(book.basis, BASIS_BOTH);
        assert_eq!(book.review.len(), 2);
    }

    #[test]
    fn the_chapters_and_the_basis_follow_the_pages_that_exist() {
        let only_paged = map(vec![entry(
            "ref/rule/a",
            EntryKind::Rule,
            "A",
            &[("b", Some("1"))],
        )]);
        let book = &book_index(&[], &[publication("b", "Book")], &only_paged).books[0];
        assert_eq!(titles(&book.chapters), ["Page order"]);
        assert_eq!(book.basis, BASIS_PAGE_ORDER);
        let only_unpaged = map(vec![entry(
            "ref/rule/a",
            EntryKind::Rule,
            "A",
            &[("b", None)],
        )]);
        let book = &book_index(&[], &[publication("b", "Book")], &only_unpaged).books[0];
        assert_eq!(titles(&book.chapters), ["Unpaged"]);
        assert_eq!(book.basis, BASIS_UNPAGED);
        let book = &book_index(&[], &[publication("b", "Book")], &BTreeMap::new()).books[0];
        assert!(book.chapters.is_empty() && book.basis.is_empty() && book.review.is_empty());
    }

    #[test]
    fn a_child_nests_under_the_nearest_member_on_its_parent_chain() {
        let mut host = entry(
            "ref/rule/host",
            EntryKind::Rule,
            "Host",
            &[("b", Some("5"))],
        );
        host.entry.parent = None;
        // `mid` is not a member of the book; its parent is.
        let mut mid = entry(
            "ref/rule/mid",
            EntryKind::Rule,
            "Mid",
            &[("other", Some("1"))],
        );
        mid.entry.parent = Some(EntryId::parse("ref/rule/host").unwrap());
        let mut leaf = entry("ref/spell/leaf", EntryKind::Spell, "Leaf", &[("b", None)]);
        leaf.entry.parent = Some(EntryId::parse("ref/rule/mid").unwrap());
        let entries = map(vec![host, mid, leaf]);
        let book = &book_index(&[], &[publication("b", "Book")], &entries).books[0];
        // The leaf has no page but nests under its host, in the host's chapter.
        assert_eq!(titles(&book.chapters), ["Page order"]);
        assert_eq!(titles(&book.chapters[0].children[0].children), ["Leaf"]);
    }

    #[test]
    fn a_parent_cycle_still_places_every_member() {
        let mut a = entry("ref/rule/a", EntryKind::Rule, "A", &[("b", Some("1"))]);
        let mut c = entry("ref/rule/c", EntryKind::Rule, "C", &[("b", Some("2"))]);
        a.entry.parent = Some(EntryId::parse("ref/rule/c").unwrap());
        c.entry.parent = Some(EntryId::parse("ref/rule/a").unwrap());
        let entries = map(vec![a, c]);
        let book = &book_index(&[], &[publication("b", "Book")], &entries).books[0];
        let mut placed = BTreeSet::new();
        placed_ids(&book.chapters, &mut placed);
        assert_eq!(placed.len(), 2);
    }

    #[test]
    fn a_curated_book_keeps_its_chapters_and_gains_a_catch_all() {
        let entries = map(vec![
            entry("ref/rule/a", EntryKind::Rule, "A", &[("b", Some("1"))]),
            entry("ref/rule/b", EntryKind::Rule, "B", &[("b", Some("2"))]),
            entry("ref/rule/c", EntryKind::Rule, "C", &[("b", None)]),
            entry("ref/rule/d", EntryKind::Rule, "D", &[("other", Some("3"))]),
        ]);
        let curated = BookTree {
            source: "b".into(),
            title: "Book".into(),
            basis: "checked".into(),
            review: vec!["note".into()],
            chapters: vec![BookNode {
                title: "Chapter 1".into(),
                entry: None,
                anchor: None,
                review: None,
                collapse: false,
                lists_children: false,
                children: vec![BookNode {
                    title: "A".into(),
                    entry: Some(EntryId::parse("ref/rule/a").unwrap()),
                    anchor: Some("x".into()),
                    review: None,
                    collapse: false,
                    lists_children: false,
                    children: vec![],
                }],
            }],
        };
        let index = book_index(
            std::slice::from_ref(&curated),
            &[publication("b", "Book"), publication("other", "Other")],
            &entries,
        );
        // Curated books come first, derived books after.
        assert_eq!(
            index
                .books
                .iter()
                .map(|b| b.source.as_str())
                .collect::<Vec<_>>(),
            ["b", "other"]
        );
        let book = &index.books[0];
        assert_eq!(titles(&book.chapters), ["Chapter 1", CATCH_ALL_TITLE]);
        assert_eq!(titles(&book.chapters[1].children), ["B", "C"]);
        assert_eq!(book.basis, "checked");
        assert_eq!(&book.chapters[0], &curated.chapters[0]);
    }

    #[test]
    fn a_curated_book_with_no_loose_entry_has_no_catch_all() {
        let entries = map(vec![entry(
            "ref/rule/a",
            EntryKind::Rule,
            "A",
            &[("b", Some("1"))],
        )]);
        let curated = BookTree {
            source: "b".into(),
            title: "Book".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![BookNode {
                title: "A".into(),
                entry: Some(EntryId::parse("ref/rule/a").unwrap()),
                anchor: None,
                review: None,
                collapse: false,
                lists_children: false,
                children: vec![],
            }],
        };
        let index = book_index(
            std::slice::from_ref(&curated),
            &[publication("b", "Book")],
            &entries,
        );
        assert_eq!(index.books, [curated]);
    }

    #[test]
    fn books_sort_by_folded_title_then_id_and_the_output_is_stable() {
        let pubs = [
            publication("z", "Zeta"),
            publication("b2", "Bestiary 2"),
            publication("a", "Alpha-Book"),
            publication("b1", "Bestiary 10"),
            publication("a2", "Alpha Book"),
        ];
        let entries = BTreeMap::new();
        let ids = |index: &BookIndex| {
            index
                .books
                .iter()
                .map(|b| b.source.clone())
                .collect::<Vec<_>>()
        };
        let index = book_index(&[], &pubs, &entries);
        // "alphabook" twice: the id breaks the tie. "bestiary10" sorts before "bestiary2".
        assert_eq!(ids(&index), ["a", "a2", "b1", "b2", "z"]);
        assert_eq!(index, book_index(&[], &pubs, &entries));
    }
}
