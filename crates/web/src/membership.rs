//! Book membership: which books an entry belongs to, and whether the reader sees it.
//!
//! This is the one place that answers the question. The compiler derives each entry's member
//! books from its `source_ids` and ships them in the catalog stub ([`EntryStub::books`]); the
//! client only reads them. Everything that depends on the answer goes through this module:
//! the discovery filter (search results, chips, list pages, relations), the Books counts, a
//! book's own page, the book a result row names, and the entry's meta line.
//!
//! The rules:
//!
//! - An entry is a **member** of each book in its `source_ids`, in that order. The first is its
//!   primary book ([`EntryStub::source`]).
//! - An entry is **visible** while any member book is active. Turning one book off never hides an
//!   entry that another active book also prints.
//! - Placement in `books.json` (a chapter, a page-ordered list, the catch-all) is presentation:
//!   Contents, the chapter pages and the reading footer. It decides neither membership nor
//!   visibility.
//! - A row **names** one member book: the first while it is active, else the highest-priority
//!   active one, else the first ([`preferred_book`]).
//!
//! Nothing here reads a signal, a `RefCell` or the browser, so native tests cover it (rule 8).

use cast_iron_schema::EntryStub;
use std::collections::{BTreeMap, BTreeSet};

/// The reader's active-book choice, read as a filter over catalog stubs. It borrows the set of
/// turned-off books; every book not in the set is active.
#[derive(Debug, Clone, Copy)]
pub struct BookMembership<'a> {
    disabled: &'a BTreeSet<String>,
}

impl<'a> BookMembership<'a> {
    pub fn new(disabled: &'a BTreeSet<String>) -> Self {
        Self { disabled }
    }

    /// Whether a book is on. An id the library does not know is on, as before.
    pub fn book_active(&self, book: &str) -> bool {
        !self.disabled.contains(book)
    }

    /// Whether the reader sees the entry: any member book is active.
    pub fn is_visible(&self, stub: &EntryStub) -> bool {
        members(stub).iter().any(|book| self.book_active(book))
    }

    /// The ids of the visible entries, in one pass. Discovery tests every hit, row and relation
    /// against this set.
    pub fn visible_ids(&self, entries: &[EntryStub]) -> BTreeSet<String> {
        entries
            .iter()
            .filter(|stub| self.is_visible(stub))
            .map(|stub| stub.id.to_string())
            .collect()
    }

    /// The member book a row names for this entry under the current choice.
    pub fn row_book<'s, K: Ord>(&self, stub: &'s EntryStub, rank: impl Fn(&str) -> K) -> &'s str {
        preferred_book(
            &stub.source,
            members(stub).iter().map(String::as_str),
            |book| self.book_active(book),
            rank,
        )
    }
}

/// Every book the entry belongs to, the primary one first. Never empty.
pub fn members(stub: &EntryStub) -> &[String] {
    stub.member_books()
}

/// Whether the entry belongs to `book`, whether or not the book is active.
pub fn is_member(stub: &EntryStub, book: &str) -> bool {
    members(stub).iter().any(|member| member == book)
}

/// The member book a book filter picked, when the entry has one: the first book of the entry
/// that the filter names (case is ignored, like the filter). A result list under a book filter
/// labels each row with that book, so a row never names a book the reader did not ask for while
/// the book they asked for prints it too.
pub fn chosen_book<'s>(stub: &'s EntryStub, chosen: &[String]) -> Option<&'s str> {
    members(stub)
        .iter()
        .find(|member| chosen.iter().any(|book| book.eq_ignore_ascii_case(member)))
        .map(String::as_str)
}

/// The book a listed row names, in one precedence: the book that revised the row (a revision
/// replaces its original in place, and the row keeps naming the revision's book), else the book
/// a book filter chose when the entry belongs to it, else `active_choice`, the member book the
/// reader's active books select (`BookMembership::row_book`). Search rows, Quick search rows and
/// browse rows all name their book through this.
pub fn listed_book<'s>(
    revised_by: Option<&'s str>,
    stub: &'s EntryStub,
    chosen: &[String],
    active_choice: impl FnOnce() -> &'s str,
) -> &'s str {
    revised_by
        .or_else(|| chosen_book(stub, chosen))
        .unwrap_or_else(active_choice)
}

/// The book an effective-revision pass set on a search hit, if it did. The index records the
/// entry's primary book; `revisions::effective_hits` overwrites it with the revising book, so a
/// hit whose source differs from its stub's carries a revision attribution.
pub fn revision_source<'s>(stub: &EntryStub, hit_source: &'s str) -> Option<&'s str> {
    (stub.source != hit_source).then_some(hit_source)
}

/// How many entries each book holds, whether or not it is active. An entry counts once for each
/// of its books, so the counts add up to more than the number of entries. A book with no entries
/// has no key. A key is allocated once, when its first entry arrives.
pub fn count_by_book<'a>(
    entries: impl IntoIterator<Item = &'a EntryStub>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::<String, usize>::new();
    for stub in entries {
        for book in members(stub) {
            match counts.get_mut(book) {
                Some(count) => *count += 1,
                None => {
                    counts.insert(book.clone(), 1);
                }
            }
        }
    }
    counts
}

/// The book a reader should see named first. `first` wins while it is active, so the label is
/// unchanged when every book is on. When it is off, take the active member with the lowest
/// `rank` (the highest-priority tier), keeping the caller's order inside a rank. With every
/// member off, keep `first`, so a label still names one book.
pub fn preferred_book<'a, K: Ord>(
    first: &'a str,
    members: impl IntoIterator<Item = &'a str>,
    is_active: impl Fn(&str) -> bool,
    rank: impl Fn(&str) -> K,
) -> &'a str {
    if is_active(first) {
        return first;
    }
    members
        .into_iter()
        .filter(|book| *book != first && is_active(book))
        .min_by_key(|book| rank(book))
        .unwrap_or(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{EntryId, EntryKind};

    fn stub(id: &str, books: &[&str]) -> EntryStub {
        EntryStub {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Feat,
            title: id.to_string(),
            source: books[0].to_string(),
            books: if books.len() > 1 {
                books.iter().map(|book| book.to_string()).collect()
            } else {
                Vec::new()
            },
            parent: None,
            shard: "entries/feat-000.json".into(),
        }
    }

    fn off(books: &[&str]) -> BTreeSet<String> {
        books.iter().map(|book| book.to_string()).collect()
    }

    #[test]
    fn an_entry_is_visible_while_any_member_book_is_active() {
        // Acrobatic is a Core Rulebook feat that Mythic Adventures also lists.
        let acrobatic = stub("ref/feat/acrobatic", &["core", "mythic"]);
        let none_off = BTreeSet::new();
        let mythic_off = off(&["mythic"]);
        let core_off = off(&["core"]);
        let both_off = off(&["core", "mythic"]);
        assert!(BookMembership::new(&none_off).is_visible(&acrobatic));
        assert!(BookMembership::new(&mythic_off).is_visible(&acrobatic));
        assert!(BookMembership::new(&core_off).is_visible(&acrobatic));
        assert!(!BookMembership::new(&both_off).is_visible(&acrobatic));
    }

    #[test]
    fn a_stub_without_member_books_belongs_to_its_source_alone() {
        let dodge = stub("ref/feat/dodge", &["core"]);
        assert!(dodge.books.is_empty());
        assert_eq!(members(&dodge), ["core"]);
        assert!(is_member(&dodge, "core"));
        assert!(!is_member(&dodge, "mythic"));
        let core_off = off(&["core"]);
        assert!(!BookMembership::new(&core_off).is_visible(&dodge));
        // A book the choice does not mention is on.
        let unrelated = off(&["apg"]);
        assert!(BookMembership::new(&unrelated).is_visible(&dodge));
    }

    #[test]
    fn visible_ids_keep_every_entry_with_an_active_member() {
        let entries = [
            stub("ref/feat/a", &["core"]),
            stub("ref/feat/b", &["core", "mythic"]),
            stub("ref/feat/c", &["mythic"]),
            stub("ref/feat/d", &["mythic", "um"]),
        ];
        let mythic_off = off(&["mythic"]);
        let ids = BookMembership::new(&mythic_off).visible_ids(&entries);
        assert_eq!(
            ids,
            BTreeSet::from([
                "ref/feat/a".to_string(),
                "ref/feat/b".to_string(),
                "ref/feat/d".to_string()
            ])
        );
        let none_off = BTreeSet::new();
        assert_eq!(
            BookMembership::new(&none_off).visible_ids(&entries).len(),
            4
        );
    }

    #[test]
    fn placement_is_not_an_input() {
        // The module takes only stubs and the active set: a chapter, a listing or a book order
        // cannot change who is a member or who is visible.
        let stub = stub("ref/feat/acrobatic", &["core", "mythic"]);
        let mythic_off = off(&["mythic"]);
        assert!(BookMembership::new(&mythic_off).is_visible(&stub));
    }

    #[test]
    fn counts_give_an_entry_to_every_book_that_prints_it() {
        let entries = [
            stub("ref/feat/a", &["core"]),
            stub("ref/feat/b", &["core", "mythic"]),
            stub("ref/feat/c", &["mythic"]),
        ];
        let counts = count_by_book(&entries);
        assert_eq!(counts.get("core"), Some(&2));
        assert_eq!(counts.get("mythic"), Some(&2));
        assert!(!counts.contains_key("um"));
        assert!(count_by_book(&[]).is_empty());
    }

    fn rank(book: &str) -> u8 {
        match book {
            "core" => 0,
            "pc" | "pc2" => 1,
            _ => 3,
        }
    }

    #[test]
    fn a_row_names_its_first_book_while_that_book_is_active() {
        assert_eq!(preferred_book("pc", ["pc", "core"], |_| true, rank), "pc");
    }

    #[test]
    fn a_row_names_the_highest_priority_active_book_when_the_first_is_off() {
        let active = |book: &str| matches!(book, "pc" | "core" | "ap");
        // The first book is off; Core outranks the Companion and the Adventure.
        assert_eq!(
            preferred_book("settings", ["settings", "pc", "core", "ap"], active, rank),
            "core"
        );
        // Inside one rank the caller's order decides.
        assert_eq!(
            preferred_book("settings", ["settings", "pc", "pc2"], active, rank),
            "pc"
        );
        // With every member off, the first still names the entry.
        assert_eq!(
            preferred_book("settings", ["settings", "core"], |_| false, rank),
            "settings"
        );
    }

    #[test]
    fn a_book_filter_names_the_chosen_member_book() {
        let acrobatic = stub("ref/feat/acrobatic", &["core", "mythic", "um"]);
        let choose = |books: &[&str]| books.iter().map(|b| b.to_string()).collect::<Vec<_>>();
        assert_eq!(chosen_book(&acrobatic, &choose(&["um"])), Some("um"));
        assert_eq!(chosen_book(&acrobatic, &choose(&["UM"])), Some("um"));
        // Two chosen books: the entry's own order decides.
        assert_eq!(
            chosen_book(&acrobatic, &choose(&["um", "mythic"])),
            Some("mythic")
        );
        assert_eq!(chosen_book(&acrobatic, &choose(&["apg"])), None);
        assert_eq!(chosen_book(&acrobatic, &[]), None);
    }

    #[test]
    fn a_revision_book_beats_the_filter_and_the_active_choice() {
        let dodge = stub("ref/feat/dodge", &["core", "um"]);
        let chosen = vec!["um".to_string()];
        let active = || "core";
        // An effective revision names its own book, even under a filter that names another.
        assert_eq!(
            listed_book(Some("elephant"), &dodge, &chosen, active),
            "elephant"
        );
        assert_eq!(listed_book(None, &dodge, &chosen, active), "um");
        assert_eq!(listed_book(None, &dodge, &[], active), "core");
    }

    #[test]
    fn a_hit_source_that_differs_from_the_stub_is_a_revision_attribution() {
        let dodge = stub("ref/feat/dodge", &["core"]);
        assert_eq!(revision_source(&dodge, "elephant"), Some("elephant"));
        assert_eq!(revision_source(&dodge, "core"), None);
    }

    #[test]
    fn the_row_book_follows_the_active_choice() {
        let acrobatic = stub("ref/feat/acrobatic", &["mythic", "core"]);
        let none_off = BTreeSet::new();
        let mythic_off = off(&["mythic"]);
        let both_off = off(&["mythic", "core"]);
        assert_eq!(
            BookMembership::new(&none_off).row_book(&acrobatic, rank),
            "mythic"
        );
        assert_eq!(
            BookMembership::new(&mythic_off).row_book(&acrobatic, rank),
            "core"
        );
        assert_eq!(
            BookMembership::new(&both_off).row_book(&acrobatic, rank),
            "mythic"
        );
    }
}
