//! Reachability without the catch-all book chapter.
//!
//! The corpus trees put entries that no curated chapter claims into one final chapter,
//! `New entries (page order)`. The app will stop showing that chapter. This report finds every
//! installed entry that would then have no way in.
//!
//! An entry is reachable when at least one of these holds:
//!
//! 1. a book tree places it outside the catch-all chapter;
//! 2. its kind has a list page (every kind except [`EntryKind::Rule`]);
//! 3. a reachable entry links to it, or holds a typed relation to it;
//! 4. it holds an add-on, variant or revision relation to a reachable base.
//!
//! Rules 3 and 4 repeat until nothing changes. The result is deterministic: entries and
//! summaries are sorted, and the report holds no timestamps.

use cast_iron_schema::{BookIndex, BookNode, EntryId, EntryKind, RelationKind, RuleEntry};
use cast_iron_search::SearchIndex;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The corpus title of the chapter that the app will stop showing.
pub const CATCH_ALL_TITLE: &str = "New entries (page order)";

/// The report explains why `by_book` can add to more than the distinct total.
pub const BY_BOOK_NOTE: &str = "`unreachable` counts distinct entries. `by_book` counts catch-all placements, so one entry can appear in more than one book and its `unreachable` column can add to `unreachable_placements`, which is larger than the distinct total.";

/// One installed entry that no route reaches without the catch-all chapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnreachableEntry {
    pub id: String,
    pub title: String,
    pub kind: String,
    /// Every publication the entry names, in the corpus order.
    pub source_ids: Vec<String>,
    pub reason: String,
}

/// Unreachable counts for one publication's catch-all chapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BookSummary {
    pub source_id: String,
    pub title: String,
    /// Entries the book places in its catch-all chapter, reachable or not.
    pub catch_all_entries: u32,
    pub unreachable: u32,
    pub unreachable_rules: u32,
}

/// Counts per kind over the whole library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindSummary {
    pub kind: String,
    pub entries: u32,
    pub unreachable: u32,
}

/// One entry that the search index does not hold under its exact title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchProblem {
    pub id: String,
    pub title: String,
    pub reason: String,
}

/// The search-index half of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchSummary {
    pub entries: u32,
    /// Documents in the index.
    pub documents: u32,
    /// Entries whose id and exact title are both in the index.
    pub indexed: u32,
    pub problems: Vec<SearchProblem>,
}

/// `reachability.json`: the entries that only the catch-all chapter reaches, and the search
/// check that every entry is indexed under its exact title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub catch_all_title: String,
    pub entries: u32,
    pub reachable: u32,
    /// Distinct entries that no route reaches without the catch-all chapter.
    pub unreachable: u32,
    /// Sum of `by_book[].unreachable`. One entry can sit in several books' catch-all
    /// chapters, so this can be larger than `unreachable`.
    pub unreachable_placements: u32,
    /// Explains the distinct total against the per-book placements.
    pub by_book_note: String,
    /// Unreachable entries that no book tree places anywhere.
    pub unplaced: u32,
    pub unreachable_entries: Vec<UnreachableEntry>,
    pub by_book: Vec<BookSummary>,
    pub by_kind: Vec<KindSummary>,
    pub search: SearchSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Placement {
    /// A placement outside the catch-all chapter.
    outside: bool,
    /// Books that place the entry inside their catch-all chapter.
    catch_all_books: BTreeSet<String>,
}

/// The ids that reach the reader without the catch-all chapter, by rules 1 to 4.
///
/// The function reads only its arguments. It holds no file access and no time-dependent value.
pub fn reachable_without_catch_all(entries: &[RuleEntry], books: &BookIndex) -> BTreeSet<EntryId> {
    let placements = placements(entries, books);
    let installed: BTreeSet<&EntryId> = entries.iter().map(|entry| &entry.id).collect();
    let mut reachable: BTreeSet<EntryId> = entries
        .iter()
        .filter(|entry| {
            // A list page reaches every kind except Rules. This route assumes every book is
            // active: the compiler has no reader selection, and the app's kind lists filter by
            // the books a reader turned on. Third-party books are off by default, so a
            // non-rule entry that only a third-party catch-all places is still reachable here.
            // A revision-hidden entry also counts: the unfiltered list hides a removed,
            // merged, or revised base only while its revision's book is active, and the book's
            // own kind list still shows the entry. The report describes the installed library,
            // not one reader's active books.
            entry.kind != EntryKind::Rule
                || placements
                    .get(&entry.id)
                    .is_some_and(|placement| placement.outside)
        })
        .map(|entry| entry.id.clone())
        .collect();

    loop {
        let mut added: BTreeSet<EntryId> = BTreeSet::new();
        for entry in entries {
            if !reachable.contains(&entry.id) {
                continue;
            }
            // A body link and a typed relation both lead from the reachable entry to its
            // target.
            for target in entry
                .links
                .iter()
                .map(|link| &link.target)
                .chain(entry.relations.iter().map(|relation| &relation.target))
            {
                if installed.contains(target) && !reachable.contains(target) {
                    added.insert(target.clone());
                }
            }
        }
        for entry in entries {
            if reachable.contains(&entry.id) {
                continue;
            }
            // An add-on, variant or revision hangs off its base, so a reachable base reaches
            // it in this direction.
            let attached = entry.relations.iter().any(|relation| {
                matches!(
                    relation.kind,
                    RelationKind::Addon | RelationKind::Variant | RelationKind::Revision
                ) && reachable.contains(&relation.target)
            });
            if attached {
                added.insert(entry.id.clone());
            }
        }
        if added.is_empty() {
            break;
        }
        reachable.extend(added);
    }
    reachable
}

/// Builds `reachability.json` from the entries, the book trees, and the search index.
pub fn report(entries: &[RuleEntry], books: &BookIndex, index: &SearchIndex) -> Report {
    let placements = placements(entries, books);
    let reachable = reachable_without_catch_all(entries, books);

    let mut unreachable_by_kind: BTreeMap<String, u32> = BTreeMap::new();
    let mut entries_by_kind: BTreeMap<String, u32> = BTreeMap::new();
    for entry in entries {
        *entries_by_kind
            .entry(entry.kind.slug().to_string())
            .or_default() += 1;
    }

    let mut unreachable_entries: Vec<UnreachableEntry> = entries
        .iter()
        .filter(|entry| !reachable.contains(&entry.id))
        .map(|entry| {
            let placement = placements.get(&entry.id);
            *unreachable_by_kind
                .entry(entry.kind.slug().to_string())
                .or_default() += 1;
            UnreachableEntry {
                id: entry.id.to_string(),
                title: entry.title.clone(),
                kind: entry.kind.slug().to_string(),
                source_ids: entry.source_ids.clone(),
                reason: reason(placement),
            }
        })
        .collect();
    unreachable_entries.sort_by(|a, b| a.id.cmp(&b.id));

    let unreachable_ids: BTreeSet<&EntryId> = entries
        .iter()
        .filter(|entry| !reachable.contains(&entry.id))
        .map(|entry| &entry.id)
        .collect();
    let unplaced = entries
        .iter()
        .filter(|entry| !reachable.contains(&entry.id) && !placements.contains_key(&entry.id))
        .count() as u32;

    let titles: BTreeMap<&str, &str> = books
        .books
        .iter()
        .map(|book| (book.source.as_str(), book.title.as_str()))
        .collect();
    let mut catch_all: BTreeMap<String, BTreeSet<EntryId>> = BTreeMap::new();
    for (id, placement) in &placements {
        for book in &placement.catch_all_books {
            catch_all
                .entry(book.clone())
                .or_default()
                .insert(id.clone());
        }
    }
    let by_book: Vec<BookSummary> = catch_all
        .into_iter()
        .map(|(source_id, ids)| {
            let unreachable = ids.iter().filter(|id| unreachable_ids.contains(id)).count() as u32;
            let unreachable_rules = entries
                .iter()
                .filter(|entry| {
                    entry.kind == EntryKind::Rule
                        && unreachable_ids.contains(&entry.id)
                        && ids.contains(&entry.id)
                })
                .count() as u32;
            BookSummary {
                title: titles
                    .get(source_id.as_str())
                    .copied()
                    .unwrap_or(source_id.as_str())
                    .to_string(),
                source_id,
                catch_all_entries: ids.len() as u32,
                unreachable,
                unreachable_rules,
            }
        })
        .collect();

    let by_kind: Vec<KindSummary> = entries_by_kind
        .into_iter()
        .map(|(kind, entries)| KindSummary {
            unreachable: unreachable_by_kind.get(&kind).copied().unwrap_or(0),
            kind,
            entries,
        })
        .collect();

    let search = search_summary(entries, index);
    let unreachable = unreachable_entries.len() as u32;
    let unreachable_placements = by_book.iter().map(|book| book.unreachable).sum();
    Report {
        catch_all_title: CATCH_ALL_TITLE.to_string(),
        entries: entries.len() as u32,
        reachable: entries.len() as u32 - unreachable,
        unreachable,
        unreachable_placements,
        by_book_note: BY_BOOK_NOTE.to_string(),
        unplaced,
        unreachable_entries,
        by_book,
        by_kind,
        search,
    }
}

fn reason(placement: Option<&Placement>) -> String {
    let where_placed = match placement {
        None => "no book tree places it".to_string(),
        Some(placement) if !placement.outside => {
            let books: Vec<&str> = placement
                .catch_all_books
                .iter()
                .map(String::as_str)
                .collect();
            format!(
                "its only book placement is the catch-all chapter `{CATCH_ALL_TITLE}` of {}",
                books.join(", ")
            )
        }
        Some(_) => "no reachable route reaches it".to_string(),
    };
    format!(
        "{where_placed}; no reachable entry links to it or relates to it, and it has no add-on, variant or revision relation to a reachable base"
    )
}

/// Body links and relations that a reachable entry holds.
fn placements(entries: &[RuleEntry], books: &BookIndex) -> BTreeMap<EntryId, Placement> {
    let installed: BTreeSet<&EntryId> = entries.iter().map(|entry| &entry.id).collect();
    let mut placements: BTreeMap<EntryId, Placement> = BTreeMap::new();
    for book in &books.books {
        walk(
            &book.chapters,
            &book.source,
            false,
            &installed,
            &mut placements,
        );
    }
    placements
}

fn walk(
    nodes: &[BookNode],
    source: &str,
    in_catch_all: bool,
    installed: &BTreeSet<&EntryId>,
    placements: &mut BTreeMap<EntryId, Placement>,
) {
    for node in nodes {
        let in_catch_all = in_catch_all || node.title == CATCH_ALL_TITLE;
        if let Some(entry) = &node.entry
            && installed.contains(entry)
        {
            let placement = placements.entry(entry.clone()).or_default();
            if in_catch_all {
                placement.catch_all_books.insert(source.to_string());
            } else {
                placement.outside = true;
            }
        }
        walk(&node.children, source, in_catch_all, installed, placements);
    }
}

/// Checks that every entry is in the index under its exact title.
fn search_summary(entries: &[RuleEntry], index: &SearchIndex) -> SearchSummary {
    let mut titles: BTreeMap<&str, &str> = BTreeMap::new();
    for ord in 0..index.doc_count() as u32 {
        if let Some(doc) = index.doc(ord) {
            titles.entry(doc.id).or_insert(doc.title);
        }
    }
    let mut problems = Vec::new();
    let mut indexed = 0u32;
    for entry in entries {
        match titles.get(entry.id.as_str()) {
            None => problems.push(SearchProblem {
                id: entry.id.to_string(),
                title: entry.title.clone(),
                reason: "not in the search index".to_string(),
            }),
            Some(title) if *title != entry.title => problems.push(SearchProblem {
                id: entry.id.to_string(),
                title: entry.title.clone(),
                reason: format!("the index holds the title `{title}` instead"),
            }),
            Some(_) => {
                indexed += 1;
                if cast_iron_search::normalize(&entry.title).is_empty() {
                    problems.push(SearchProblem {
                        id: entry.id.to_string(),
                        title: entry.title.clone(),
                        reason: "the title has no searchable tokens".to_string(),
                    });
                }
            }
        }
    }
    problems.sort_by(|a, b| a.id.cmp(&b.id));
    SearchSummary {
        entries: entries.len() as u32,
        documents: index.doc_count() as u32,
        indexed,
        problems,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{
        Block, BookNode, BookTree, EntryReference, Inline, ParagraphStyle, Relation, TypedFields,
    };
    use cast_iron_search::{DocInput, IndexBuilder};

    fn entry(id: &str, kind: EntryKind) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: id.into(),
            source_ids: vec!["book".into()],
            sources: vec![],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::None,
            tags: vec![],
            race_group: None,
            links: vec![],
            provenance: vec![],
            license_refs: vec![],
            relations: vec![],
        }
    }

    fn link(mut entry: RuleEntry, target: &str) -> RuleEntry {
        let target = EntryId::parse(target).unwrap();
        entry.body = vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![Inline::Link {
                target: target.clone(),
                anchor: None,
                c: vec![Inline::text("x")],
            }],
        }];
        entry.links = vec![EntryReference {
            target,
            anchor: None,
        }];
        entry
    }

    fn relation(mut entry: RuleEntry, kind: RelationKind, target: &str) -> RuleEntry {
        entry.relations = vec![Relation {
            kind,
            target: EntryId::parse(target).unwrap(),
            label: None,
        }];
        entry
    }

    fn chapter(title: &str, entry: Option<&str>) -> BookNode {
        BookNode {
            title: title.into(),
            entry: entry.map(|id| EntryId::parse(id).unwrap()),
            anchor: None,
            review: None,
            children: vec![],
        }
    }

    fn book_named(source: &str, title: &str, chapters: Vec<BookNode>) -> BookTree {
        BookTree {
            source: source.into(),
            title: title.into(),
            basis: String::new(),
            review: vec![],
            chapters,
        }
    }

    fn book(chapters: Vec<BookNode>) -> BookIndex {
        BookIndex {
            books: vec![book_named("book", "Book", chapters)],
        }
    }

    fn report_for(entries: &[RuleEntry], books: &BookIndex) -> Report {
        let mut builder = IndexBuilder::new();
        for entry in entries {
            builder.add(DocInput {
                id: entry.id.to_string(),
                title: entry.title.clone(),
                kind: entry.kind.slug().to_string(),
                source: entry.source_ids[0].clone(),
                ..Default::default()
            });
        }
        report(entries, books, &builder.build())
    }

    fn ids(report: &Report) -> Vec<&str> {
        report
            .unreachable_entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect()
    }

    #[test]
    fn an_entry_only_in_the_catch_all_chapter_is_reported() {
        let entries = vec![entry("ref/rule/a", EntryKind::Rule)];
        let books = book(vec![chapter(CATCH_ALL_TITLE, Some("ref/rule/a"))]);
        let report = report_for(&entries, &books);
        assert_eq!(ids(&report), vec!["ref/rule/a"]);
        assert_eq!(report.unreachable, 1);
        assert_eq!(report.unreachable_placements, 1);
        assert_eq!(report.by_book[0].source_id, "book");
        assert_eq!(report.by_book[0].catch_all_entries, 1);
        assert_eq!(report.by_book[0].unreachable_rules, 1);
        assert_eq!(report.by_book_note, BY_BOOK_NOTE);
        assert!(
            report.unreachable_entries[0]
                .reason
                .contains(CATCH_ALL_TITLE)
        );
    }

    #[test]
    fn an_entry_linked_from_a_placed_entry_is_not_reported() {
        let entries = vec![
            entry("ref/rule/a", EntryKind::Rule),
            link(entry("ref/rule/b", EntryKind::Rule), "ref/rule/a"),
        ];
        let books = book(vec![
            chapter("Printed chapter", Some("ref/rule/b")),
            chapter(CATCH_ALL_TITLE, Some("ref/rule/a")),
        ]);
        let report = report_for(&entries, &books);
        assert!(ids(&report).is_empty(), "{:?}", ids(&report));
        assert_eq!(report.reachable, 2);
    }

    #[test]
    fn a_non_rule_kind_is_reachable_through_its_list_page() {
        let entries = vec![entry("ref/spell/a", EntryKind::Spell)];
        let books = book(vec![chapter(CATCH_ALL_TITLE, Some("ref/spell/a"))]);
        let report = report_for(&entries, &books);
        assert!(ids(&report).is_empty(), "{:?}", ids(&report));
        assert_eq!(report.by_book[0].unreachable, 0);
        assert_eq!(report.by_book[0].catch_all_entries, 1);
    }

    #[test]
    fn an_add_on_attached_to_a_reachable_base_is_not_reported() {
        let entries = vec![
            entry("ref/rule/base", EntryKind::Rule),
            relation(
                entry("ref/rule/addon", EntryKind::Rule),
                RelationKind::Addon,
                "ref/rule/base",
            ),
        ];
        let books = book(vec![
            chapter("Printed chapter", Some("ref/rule/base")),
            chapter(CATCH_ALL_TITLE, Some("ref/rule/addon")),
        ]);
        let report = report_for(&entries, &books);
        assert!(ids(&report).is_empty(), "{:?}", ids(&report));
    }

    #[test]
    fn a_relation_from_a_reachable_entry_reaches_its_target() {
        let entries = vec![
            relation(
                entry("ref/rule/source", EntryKind::Rule),
                RelationKind::Part,
                "ref/rule/target",
            ),
            entry("ref/rule/target", EntryKind::Rule),
        ];
        let books = book(vec![
            chapter("Printed chapter", Some("ref/rule/source")),
            chapter(CATCH_ALL_TITLE, Some("ref/rule/target")),
        ]);
        let report = report_for(&entries, &books);
        assert!(ids(&report).is_empty(), "{:?}", ids(&report));
    }

    #[test]
    fn a_missing_or_mistitled_index_document_is_a_search_problem() {
        let entries = vec![entry("ref/rule/a", EntryKind::Rule)];
        let books = book(vec![chapter("Printed chapter", Some("ref/rule/a"))]);
        let index = IndexBuilder::new().build();
        let report = report(&entries, &books, &index);
        assert_eq!(report.search.indexed, 0);
        assert_eq!(report.search.problems[0].id, "ref/rule/a");
        assert_eq!(report.search.problems[0].reason, "not in the search index");
    }

    #[test]
    fn rules_that_link_only_to_each_other_are_both_reported() {
        let entries = vec![
            link(entry("ref/rule/a", EntryKind::Rule), "ref/rule/b"),
            link(entry("ref/rule/b", EntryKind::Rule), "ref/rule/a"),
        ];
        let books = book(vec![
            chapter(CATCH_ALL_TITLE, Some("ref/rule/a")),
            chapter(CATCH_ALL_TITLE, Some("ref/rule/b")),
        ]);
        let report = report_for(&entries, &books);
        assert_eq!(ids(&report), vec!["ref/rule/a", "ref/rule/b"]);
        assert_eq!(report.unreachable, 2);
    }

    #[test]
    fn one_entry_in_several_catch_alls_is_one_distinct_entry_and_several_placements() {
        let entries = vec![entry("ref/rule/a", EntryKind::Rule)];
        let books = BookIndex {
            books: vec![
                book_named(
                    "book-a",
                    "Book A",
                    vec![chapter(CATCH_ALL_TITLE, Some("ref/rule/a"))],
                ),
                book_named(
                    "book-b",
                    "Book B",
                    vec![chapter(CATCH_ALL_TITLE, Some("ref/rule/a"))],
                ),
            ],
        };
        let report = report_for(&entries, &books);
        assert_eq!(ids(&report), vec!["ref/rule/a"]);
        assert_eq!(report.unreachable, 1);
        assert_eq!(report.unreachable_placements, 2);
        assert_eq!(
            report
                .by_book
                .iter()
                .map(|book| book.unreachable)
                .sum::<u32>(),
            2
        );
        assert!(report.by_book_note.contains("more than one book"));
    }

    #[test]
    fn a_deep_catch_all_placement_still_counts_as_catch_all() {
        let entries = vec![entry("ref/rule/a", EntryKind::Rule)];
        let mut nested = chapter(CATCH_ALL_TITLE, None);
        nested.children.push(chapter("Section", Some("ref/rule/a")));
        let books = book(vec![nested]);
        let report = report_for(&entries, &books);
        assert_eq!(ids(&report), vec!["ref/rule/a"]);
    }
}
