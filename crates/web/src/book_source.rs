//! Retired book source ids and the state that carries them.
//!
//! A book id appears in an address and in the reader's stored state. A retired id must keep
//! resolving in both places, through one table.

use std::collections::BTreeSet;

/// The id of the printed GameMastery Guide tree and the publication id that now holds it.
/// The corpus imported the same book twice: the printed tree used the first id, and a
/// dataset-topic tree used the publication id.
const PRINTED_GAME_MASTERY_GUIDE: &str = "gamemastery-guide";
const GAME_MASTERY_GUIDE: &str = "game-mastery-guide";

/// Book source ids that retired, paired with the book that now holds their contents.
const RETIRED_BOOK_SOURCES: &[(&str, &str)] = &[(PRINTED_GAME_MASTERY_GUIDE, GAME_MASTERY_GUIDE)];

/// Listing trees that corpus v0.1.24 deleted. Each held only a `Page order` and an `Unpaged`
/// chapter that repeated the entries of the printed book, paired with that book. An address
/// for the old book opens the printed book; an address for one of its chapters has no
/// equivalent, so it opens the printed book's page.
const RETIRED_LISTING_TREES: &[(&str, &str)] = &[
    ("prpg-core-rulebook", "core-rulebook"),
    ("pathfinder-rpg-bestiary", "bestiary"),
];

/// One-off trees that corpus v0.1.24 deleted: a blog post or a comic issue, never a printed
/// book. Every entry in one also sits in the printed book it names, so the tree has no
/// replacement. An address for it opens Books.
const RETIRED_ONE_OFF_TREES: &[&str] = &[
    "paizo-blog-golarion-day-subdomains",
    "paizo-blog-sable-company-marine",
    "paizo-blog-the-gauntlet-will-be-ours",
    "pathfinder-comics-11",
    "pathfinder-comics-3",
    "pathfinder-comics-5",
    "pathfinder-worldscape-2",
];

/// Where an address for a deleted book tree goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retirement {
    /// The page of this book.
    Book(&'static str),
    /// The Books page.
    Books,
}

/// The destination of an address for a book tree that the corpus deleted, or `None` for a
/// current book and for a tree that merged into another (see [`aliased_book_source`]). A
/// chapter address of a deleted tree uses the same destination, because its chapters did not
/// carry over.
pub fn retirement(source: &str) -> Option<Retirement> {
    RETIRED_LISTING_TREES
        .iter()
        .find_map(|(old, new)| (*old == source).then_some(Retirement::Book(new)))
        .or_else(|| {
            RETIRED_ONE_OFF_TREES
                .contains(&source)
                .then_some(Retirement::Books)
        })
}

/// The current book source for an old book address or stored row. A current id is unchanged,
/// and so is a deleted one-off tree: it has no replacement (see [`retirement`]).
pub fn aliased_book_source(source: &str) -> &str {
    RETIRED_BOOK_SOURCES
        .iter()
        .chain(RETIRED_LISTING_TREES)
        .find_map(|(old, new)| (*old == source).then_some(*new))
        .unwrap_or(source)
}

/// True when the installed library still places a tree under a retired source id. The stored
/// disabled set is migrated only after that tree is gone.
pub fn has_retired_book(books: &crate::graph::BookLookup) -> bool {
    RETIRED_BOOK_SOURCES
        .iter()
        .any(|(old, _)| books.book(old).is_some())
}

/// The disabled-book set after the book-source merge.
///
/// The second tree used the publication id, so a pre-merge disable of `game-mastery-guide`
/// meant "hide the topic tree" while the printed tree stayed active. The merged book is
/// therefore disabled only when the printed tree was disabled, alone or together with the
/// topic tree. A disable of the topic tree alone is dropped.
///
/// Run this once when the installed library no longer places a tree under the retired id.
/// It is not idempotent on its own: after the merge, a disabled `game-mastery-guide` is the
/// reader's choice for the merged book and must be kept.
pub fn migrate_disabled_books(stored: &BTreeSet<String>) -> BTreeSet<String> {
    let mut next = stored.clone();
    let printed_disabled = next.remove(PRINTED_GAME_MASTERY_GUIDE);
    next.remove(GAME_MASTERY_GUIDE);
    if printed_disabled {
        next.insert(GAME_MASTERY_GUIDE.to_string());
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn a_retired_book_id_maps_to_the_book_that_now_holds_it() {
        assert_eq!(
            aliased_book_source("gamemastery-guide"),
            "game-mastery-guide"
        );
        for current in ["game-mastery-guide", "core-rulebook"] {
            assert_eq!(aliased_book_source(current), current);
        }
    }

    #[test]
    fn a_deleted_listing_tree_maps_to_its_printed_book() {
        assert_eq!(aliased_book_source("prpg-core-rulebook"), "core-rulebook");
        assert_eq!(aliased_book_source("pathfinder-rpg-bestiary"), "bestiary");
        assert_eq!(
            retirement("prpg-core-rulebook"),
            Some(Retirement::Book("core-rulebook"))
        );
        assert_eq!(
            retirement("pathfinder-rpg-bestiary"),
            Some(Retirement::Book("bestiary"))
        );
    }

    #[test]
    fn a_deleted_one_off_tree_goes_to_books() {
        for source in RETIRED_ONE_OFF_TREES {
            assert_eq!(retirement(source), Some(Retirement::Books), "{source}");
            assert_eq!(aliased_book_source(source), *source);
        }
    }

    #[test]
    fn a_current_book_is_not_retired() {
        for current in ["core-rulebook", "bestiary", "game-mastery-guide"] {
            assert_eq!(retirement(current), None, "{current}");
        }
        // The merged tree keeps its own chapters, so only its source id moves.
        assert_eq!(retirement("gamemastery-guide"), None);
    }

    #[test]
    fn a_disabled_print_tree_carries_to_the_merged_book() {
        assert_eq!(
            migrate_disabled_books(&ids(&["gamemastery-guide"])),
            ids(&["game-mastery-guide"])
        );
        assert_eq!(
            migrate_disabled_books(&ids(&["gamemastery-guide", "game-mastery-guide"])),
            ids(&["game-mastery-guide"])
        );
    }

    #[test]
    fn a_disabled_topic_tree_alone_leaves_the_merged_book_active() {
        assert_eq!(
            migrate_disabled_books(&ids(&["game-mastery-guide"])),
            ids(&[])
        );
        // Another book's choice is untouched.
        assert_eq!(
            migrate_disabled_books(&ids(&["game-mastery-guide", "core-rulebook"])),
            ids(&["core-rulebook"])
        );
        assert_eq!(
            migrate_disabled_books(&ids(&["gamemastery-guide", "core-rulebook"])),
            ids(&["game-mastery-guide", "core-rulebook"])
        );
    }
}
