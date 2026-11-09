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

/// The current book source for an old book address or stored row. A current id is unchanged.
pub fn aliased_book_source(source: &str) -> &str {
    RETIRED_BOOK_SOURCES
        .iter()
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
