//! One book's Contents reading order, flattened once and shared by the entry footer.
//!
//! The order holds exactly the entries that Contents shows: a hidden kind is skipped at any
//! depth, the catch-all chapter is never a stop and is not walked, and a collapsed hub still
//! contributes every child in book order. Membership, the breadcrumb trail and previous/next
//! read this one structure, so they cannot disagree and no query walks the tree again.
//!
//! An entry placed twice in one book keeps its first Contents occurrence. The later placement is
//! ignored, including its path, title and chapter, so a step never lands on the same entry twice.
//!
//! Contents visibility depends on the immutable catalog kind, not on the active-book selection.
//! [`ReadingCache`] is therefore keyed by the book only and lives for one [`crate::library::Library`]
//! instance: a new content version starts with an empty cache.

use cast_iron_schema::books::{ChapterRow, Hide, chapter_rows, is_catch_all};
use cast_iron_schema::{BookNode, BookTree, EntryId};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// One entry in a book's Contents reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingStop {
    pub entry: EntryId,
    pub title: String,
    pub path: Vec<usize>,
}

/// A neighbour in the reading order. `chapter` is the raw corpus title of the chapter page the
/// stop belongs to; the view applies the display label. `crossing` says the step leaves the
/// current chapter, which the footer then names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingNeighbor {
    pub entry: EntryId,
    pub title: String,
    pub chapter: String,
    pub crossing: bool,
}

/// The flattened Contents order of one book, with an entry-position map and the chapter rows
/// that a crossing names.
#[derive(Debug, Clone, Default)]
pub struct BookReading {
    stops: Vec<ReadingStop>,
    places: HashMap<EntryId, usize>,
    chapters: Vec<ChapterRow>,
    /// Index into `chapters` for each stop, or `None` for a stop outside every listed chapter.
    chapter_of: Vec<Option<usize>>,
}

impl BookReading {
    /// Walk the book once. `hide` must be the library's Contents visibility for its immutable
    /// catalog; a caller that changes it cannot reuse this value.
    pub fn new(book: &BookTree, hide: Hide<'_>) -> Self {
        let mut reading = Self {
            chapters: chapter_rows(book, hide),
            ..Self::default()
        };
        let mut path = Vec::new();
        for (index, chapter) in book.chapters.iter().enumerate() {
            path.push(index);
            reading.walk(chapter, &mut path, hide);
            path.pop();
        }
        reading
    }

    fn walk(&mut self, node: &BookNode, path: &mut Vec<usize>, hide: Hide<'_>) {
        // The catch-all chapter is not shown and has no chapter page, so it is not in the order.
        if is_catch_all(node) {
            return;
        }
        if let Some(entry) = &node.entry
            && !hide(entry)
            && !self.places.contains_key(entry)
        {
            self.places.insert(entry.clone(), self.stops.len());
            self.chapter_of.push(self.chapter_index(path));
            self.stops.push(ReadingStop {
                entry: entry.clone(),
                title: node.title.clone(),
                path: path.clone(),
            });
        }
        for (index, child) in node.children.iter().enumerate() {
            path.push(index);
            self.walk(child, path, hide);
            path.pop();
        }
    }

    fn chapter_index(&self, path: &[usize]) -> Option<usize> {
        self.chapters
            .iter()
            .position(|row| path.starts_with(&row.path))
    }

    /// Every Contents-visible entry, in book order, with a later duplicate placement omitted.
    pub fn stops(&self) -> &[ReadingStop] {
        &self.stops
    }

    /// The first Contents occurrence of `id` in this book.
    pub fn stop(&self, id: &EntryId) -> Option<&ReadingStop> {
        self.places.get(id).and_then(|&at| self.stops.get(at))
    }

    /// True when this book's Contents lists `id`.
    pub fn contains(&self, id: &EntryId) -> bool {
        self.places.contains_key(id)
    }

    /// The previous and next Contents-visible entries around `id`. A step crosses into the next
    /// chapter when the chapter row changes; the ends have one neighbour only.
    pub fn neighbors(&self, id: &EntryId) -> (Option<ReadingNeighbor>, Option<ReadingNeighbor>) {
        let Some(&at) = self.places.get(id) else {
            return (None, None);
        };
        let here = self.chapter_of.get(at).copied().flatten();
        let step = |at: usize| {
            let stop = &self.stops[at];
            let chapter = self.chapter_of.get(at).copied().flatten();
            ReadingNeighbor {
                entry: stop.entry.clone(),
                title: stop.title.clone(),
                chapter: chapter
                    .and_then(|index| self.chapters.get(index))
                    .map_or_else(String::new, |row| row.title.clone()),
                crossing: chapter != here,
            }
        };
        let before = at.checked_sub(1).map(step);
        let after = (at + 1 < self.stops.len()).then(|| step(at + 1));
        (before, after)
    }
}

/// Lazy per-book cache for one active library. A book is built on its first query and every
/// later query returns the same `Rc`; the cache never outlives the library it belongs to.
#[derive(Debug, Default)]
pub(crate) struct ReadingCache {
    books: RefCell<Vec<OnceCell<Rc<BookReading>>>>,
}

impl ReadingCache {
    pub(crate) fn get_or_build(
        &self,
        book: usize,
        build: impl FnOnce() -> BookReading,
    ) -> Rc<BookReading> {
        let mut books = self.books.borrow_mut();
        if books.len() <= book {
            books.resize_with(book + 1, OnceCell::new);
        }
        books[book].get_or_init(|| Rc::new(build())).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::EntryKind;
    use cast_iron_schema::books::linked_kind;
    use std::cell::Cell;

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

    /// Kinds by the middle part of the id, as the catalog would say them.
    fn kind_of(id: &EntryId) -> Option<EntryKind> {
        match id.as_str().split('/').nth(1)? {
            "spell" => Some(EntryKind::Spell),
            "feat" => Some(EntryKind::Feat),
            "magic-ring" | "item" => Some(EntryKind::Item),
            "class" => Some(EntryKind::Class),
            "prestige-class" => Some(EntryKind::PrestigeClass),
            "archetype" => Some(EntryKind::Archetype),
            "rule" => Some(EntryKind::Rule),
            "race" => Some(EntryKind::Race),
            _ => None,
        }
    }

    fn hide(id: &EntryId) -> bool {
        kind_of(id).is_some_and(linked_kind)
    }

    fn id(value: &str) -> EntryId {
        EntryId::parse(value).unwrap()
    }

    fn chapter_tree() -> BookTree {
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
                        node("Power Attack", Some("core/feat/power-attack"), vec![]),
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
                node(
                    "Magic",
                    Some("core/rule/magic"),
                    vec![node(
                        "Spells",
                        None,
                        vec![node("Fireball", Some("core/spell/fireball"), vec![])],
                    )],
                ),
                node("Credits", None, vec![]),
            ],
        }
    }

    #[test]
    fn the_order_follows_contents_and_skips_hidden_kinds_and_entries_without_a_node() {
        let tree = chapter_tree();
        let reading = BookReading::new(&tree, &hide);
        let order: Vec<(&str, Vec<usize>)> = reading
            .stops()
            .iter()
            .map(|stop| (stop.entry.as_str(), stop.path.clone()))
            .collect();
        assert_eq!(
            order,
            [
                ("core/rule/combat", vec![0]),
                ("core/rule/actions", vec![0, 0]),
                ("core/rule/partial-cover", vec![0, 2, 0]),
                ("core/rule/magic", vec![1]),
            ]
        );
        assert!(!reading.contains(&id("core/feat/power-attack")));
        assert!(!reading.contains(&id("core/spell/fireball")));
        // A heading without an entry is not a stop; the chapter still names its children.
        assert_eq!(
            reading.stop(&id("core/rule/partial-cover")).unwrap().path,
            vec![0, 2, 0]
        );
        assert_eq!(reading.chapters.len(), 3);
    }

    #[test]
    fn the_catch_all_chapter_is_never_a_stop_and_is_not_walked() {
        let tree = BookTree {
            source: "core".into(),
            title: "Core".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![
                node(
                    "Combat",
                    None,
                    vec![node("Alarm", Some("ref/rule/alarm"), vec![])],
                ),
                node(
                    cast_iron_schema::books::CATCH_ALL_TITLE,
                    None,
                    vec![node("Loose", Some("ref/rule/loose"), vec![])],
                ),
            ],
        };
        let reading = BookReading::new(&tree, &hide);
        let order: Vec<&str> = reading
            .stops()
            .iter()
            .map(|stop| stop.entry.as_str())
            .collect();
        assert_eq!(order, ["ref/rule/alarm"]);
        assert!(!reading.contains(&id("ref/rule/loose")));
        assert_eq!(reading.chapters.len(), 1);
    }

    #[test]
    fn neighbours_cross_chapters_stop_at_book_ends_and_skip_hidden_kinds() {
        let tree = chapter_tree();
        let reading = BookReading::new(&tree, &hide);

        let (before, after) = reading.neighbors(&id("core/rule/combat"));
        assert!(before.is_none());
        let after = after.unwrap();
        assert_eq!(after.entry, id("core/rule/actions"));
        assert!(!after.crossing);

        // The feat is skipped, so the nested rule follows the heading directly.
        let (before, after) = reading.neighbors(&id("core/rule/actions"));
        assert_eq!(before.unwrap().entry, id("core/rule/combat"));
        assert_eq!(after.unwrap().entry, id("core/rule/partial-cover"));

        // The chapter end continues into the next chapter and names it.
        let (_, after) = reading.neighbors(&id("core/rule/partial-cover"));
        let after = after.unwrap();
        assert_eq!(after.entry, id("core/rule/magic"));
        assert!(after.crossing);
        assert_eq!(after.chapter, "Magic");
        let (before, _) = reading.neighbors(&id("core/rule/magic"));
        let before = before.unwrap();
        assert_eq!(before.entry, id("core/rule/partial-cover"));
        assert!(before.crossing);
        assert_eq!(before.chapter, "Combat");

        // The last visible entry has no next; a hidden kind is not on the walk at all.
        assert!(reading.neighbors(&id("core/rule/magic")).1.is_none());
        assert_eq!(
            reading.neighbors(&id("core/feat/power-attack")),
            (None, None)
        );
        assert_eq!(reading.neighbors(&id("core/spell/fireball")), (None, None));
    }

    #[test]
    fn a_collapsed_hub_still_reads_through_its_children() {
        let mut tree = chapter_tree();
        tree.chapters[0].lists_children = true;
        let reading = BookReading::new(&tree, &hide);
        let order: Vec<&str> = reading
            .stops()
            .iter()
            .map(|stop| stop.entry.as_str())
            .collect();
        assert_eq!(
            order,
            [
                "core/rule/combat",
                "core/rule/actions",
                "core/rule/partial-cover",
                "core/rule/magic"
            ]
        );
        let (_, after) = reading.neighbors(&id("core/rule/combat"));
        assert_eq!(after.unwrap().entry, id("core/rule/actions"));
    }

    #[test]
    fn the_first_contents_occurrence_of_a_duplicate_entry_wins() {
        let tree = BookTree {
            source: "core".into(),
            title: "Core Rulebook".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![
                node(
                    "One",
                    None,
                    vec![
                        node("First", Some("core/rule/first"), vec![]),
                        node("Again", Some("core/rule/first"), vec![]),
                        node("Second", Some("core/rule/second"), vec![]),
                    ],
                ),
                node(
                    "Two",
                    None,
                    vec![node("Third", Some("core/rule/third"), vec![])],
                ),
            ],
        };
        let reading = BookReading::new(&tree, &hide);
        let order: Vec<&str> = reading
            .stops()
            .iter()
            .map(|stop| stop.entry.as_str())
            .collect();
        // The later placement adds no second stop.
        assert_eq!(
            order,
            ["core/rule/first", "core/rule/second", "core/rule/third"]
        );
        let first = reading.stop(&id("core/rule/first")).unwrap();
        assert_eq!(first.title, "First");
        assert_eq!(first.path, vec![0, 0]);
        // A step leaves the first occurrence and never returns to the duplicate.
        let (_, after) = reading.neighbors(&id("core/rule/first"));
        assert_eq!(after.unwrap().entry, id("core/rule/second"));
        let (before, after) = reading.neighbors(&id("core/rule/second"));
        assert_eq!(before.unwrap().entry, id("core/rule/first"));
        let after = after.unwrap();
        assert_eq!(after.entry, id("core/rule/third"));
        assert!(after.crossing);
    }

    #[test]
    fn an_unknown_or_entryless_id_has_no_position() {
        let tree = chapter_tree();
        let reading = BookReading::new(&tree, &hide);
        let unknown = id("core/rule/unknown");
        assert!(!reading.contains(&unknown));
        assert!(reading.stop(&unknown).is_none());
        assert_eq!(reading.neighbors(&unknown), (None, None));
    }

    #[test]
    fn repeated_queries_do_not_rebuild_or_reevaluate_the_book() {
        let tree = chapter_tree();
        let builds = Cell::new(0);
        let visits = Cell::new(0);
        let cache = ReadingCache::default();
        let first = cache.get_or_build(0, || {
            builds.set(builds.get() + 1);
            BookReading::new(&tree, &|id| {
                visits.set(visits.get() + 1);
                hide(id)
            })
        });
        let visits_after_build = visits.get();
        assert!(visits_after_build > 0);

        // Membership, trail lookup and neighbours are reads of the cached value: they neither
        // rebuild the order nor re-run the Contents visibility test.
        for _ in 0..5 {
            assert!(first.contains(&id("core/rule/actions")));
            assert!(first.stop(&id("core/rule/actions")).is_some());
            let _ = first.neighbors(&id("core/rule/actions"));
        }
        assert_eq!(visits.get(), visits_after_build);
        assert_eq!(builds.get(), 1);

        // A second book is another slot, and a hit returns the same value.
        let other = BookTree {
            source: "other".into(),
            title: "Other".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![node("Only", Some("other/rule/only"), vec![])],
        };
        let second = cache.get_or_build(1, || {
            builds.set(builds.get() + 1);
            BookReading::new(&other, &hide)
        });
        let again = cache.get_or_build(1, || {
            builds.set(builds.get() + 1);
            BookReading::new(&other, &hide)
        });
        assert!(Rc::ptr_eq(&second, &again));
        assert!(!Rc::ptr_eq(&first, &second));
        assert_eq!(builds.get(), 2);

        // A new library instance starts with an empty cache; it never answers from an old one.
        let fresh = ReadingCache::default();
        let rebuilt = fresh.get_or_build(0, || {
            builds.set(builds.get() + 1);
            BookReading::new(&tree, &hide)
        });
        assert!(!Rc::ptr_eq(&first, &rebuilt));
        assert_eq!(builds.get(), 3);
    }
}
