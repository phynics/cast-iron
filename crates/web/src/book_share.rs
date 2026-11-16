//! Sharing the active set (#408): the code for the books that are on now, and what a code would
//! change here. The codec is `book_code`; this module joins it to a catalog and to the presets.
//! Everything is pure, so native tests cover it.
//!
//! A code names slots (`SourceInfo::share_slot`), so it reads the same on any library that has
//! those books. Books this library has but the code does not describe (the sender's library was
//! older) keep their state. Slots the code turns on that this library has no book for are
//! skipped, and counted so the page can say so.

use crate::book_code::Shared;
use crate::book_presets::{self, Change, Preset};
use crate::fmt;
use cast_iron_schema::SourceInfo;
use std::collections::{BTreeMap, BTreeSet};

/// Book ids by slot. `None` when the catalog cannot share: it has no books, a book has no slot
/// (a library compiled before #408), or two books claim one slot.
pub fn slot_map(sources: &[SourceInfo]) -> Option<BTreeMap<u32, &str>> {
    if sources.is_empty() {
        return None;
    }
    let mut slots = BTreeMap::new();
    for source in sources {
        let slot = source.share_slot?;
        if slots.insert(slot, source.id.as_str()).is_some() {
            return None;
        }
    }
    Some(slots)
}

/// What the active books say, as the codec reads it: every slot up to the highest one this
/// library holds is described, and the slots of active books are on.
pub fn current(sources: &[SourceInfo], is_active: impl Fn(&str) -> bool) -> Option<Shared> {
    let slots = slot_map(sources)?;
    let described = slots.keys().next_back().map_or(0, |slot| slot + 1);
    let on = slots
        .iter()
        .filter(|(_, id)| is_active(id))
        .map(|(slot, _)| *slot)
        .collect();
    Some(Shared { described, on })
}

/// The code for the books that are on now, or `None` when the catalog cannot share.
pub fn code_for(sources: &[SourceInfo], is_active: impl Fn(&str) -> bool) -> Option<String> {
    current(sources, is_active).map(|shared| crate::book_code::encode(&shared))
}

/// What applying a code would do on this library, given which books are active now.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Offer {
    /// The books it turns on and off. Handing it to `Library::update_active_books` writes the
    /// set once, and `change.inverse()` is Undo.
    pub change: Change,
    /// Slots the code turns on that this library has no book for.
    pub skipped: usize,
    /// How many books are active after the change.
    pub active_after: usize,
    /// How many books the library holds.
    pub total: usize,
    /// The offered preset whose books are exactly the active ones afterwards, if any.
    pub preset: Option<Preset>,
}

impl Offer {
    /// True when the code asks for what is already set.
    pub fn is_empty(&self) -> bool {
        self.change.is_empty()
    }

    /// `Turns on 12 books, turns off 3.` Names only what changes, and says so when nothing does.
    pub fn summary(&self) -> String {
        let on = self.change.activate.len();
        let off = self.change.deactivate.len();
        match (on, off) {
            (0, 0) => "These are the books you already have on. Nothing changes.".to_string(),
            (on, 0) => format!("Turns on {}.", books(on)),
            (0, off) => format!("Turns off {}.", books(off)),
            (on, off) => format!("Turns on {}, turns off {}.", books(on), books(off)),
        }
    }

    /// `28 of 439 books active · Paizo Rulebooks`: the count line as it will read afterwards.
    pub fn result_line(&self) -> String {
        book_presets::count_line(
            self.active_after,
            self.total,
            Some(book_presets::name_or_custom(self.preset)),
        )
    }

    /// The sentence about skipped books, or `None` when none were skipped.
    pub fn skipped_line(&self) -> Option<String> {
        match self.skipped {
            0 => None,
            1 => Some(
                "1 book in this code is not in your library, so it is skipped. A newer library may have it."
                    .to_string(),
            ),
            n => Some(format!(
                "{} books in this code are not in your library, so they are skipped. A newer library may have them.",
                fmt::count(n)
            )),
        }
    }
}

fn books(n: usize) -> String {
    format!(
        "{} {}",
        fmt::count(n),
        if n == 1 { "book" } else { "books" }
    )
}

/// What `shared` would do. `None` when the catalog cannot share (see [`slot_map`]).
pub fn offer(
    shared: &Shared,
    sources: &[SourceInfo],
    is_active: impl Fn(&str) -> bool,
) -> Option<Offer> {
    let slots = slot_map(sources)?;
    let mut change = Change::default();
    let mut after: BTreeSet<&str> = BTreeSet::new();
    for (slot, id) in &slots {
        let now = is_active(id);
        // A slot at or above `described` is one the sender did not know: keep its state.
        let wanted = if *slot < shared.described {
            shared.on.contains(slot)
        } else {
            now
        };
        match (wanted, now) {
            (true, false) => change.activate.push((*id).to_string()),
            (false, true) => change.deactivate.push((*id).to_string()),
            _ => {}
        }
        if wanted {
            after.insert(id);
        }
    }
    let skipped = shared
        .on
        .iter()
        .filter(|slot| !slots.contains_key(slot))
        .count();
    let books = book_presets::books_of(sources);
    let preset = book_presets::matching(&books, |id| after.contains(id));
    Some(Offer {
        change,
        skipped,
        active_after: after.len(),
        total: sources.len(),
        preset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book_code;

    fn book(id: &str, slot: Option<u32>, category: &str, third_party: bool) -> SourceInfo {
        SourceInfo {
            id: id.to_string(),
            title: id.to_string(),
            entry_count: 1,
            copyright: None,
            category: Some(category.to_string()),
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party,
            aon: None,
            share_slot: slot,
        }
    }

    /// Two Rulebooks, a Companion, an Adventure, a third-party book, in slot order.
    fn library() -> Vec<SourceInfo> {
        vec![
            book("core", Some(0), "RPG", false),
            book("apg", Some(1), "RPG", false),
            book("companion", Some(2), "Player Companion", false),
            book("module", Some(3), "Module", false),
            book("third", Some(4), "RPG", true),
        ]
    }

    fn active_in<'a>(ids: &'a [&str]) -> impl Fn(&str) -> bool + 'a {
        move |id| ids.contains(&id)
    }

    #[test]
    fn a_catalog_without_slots_cannot_share() {
        let mut sources = library();
        sources[2].share_slot = None;
        assert!(slot_map(&sources).is_none());
        assert!(code_for(&sources, |_| true).is_none());
        assert!(slot_map(&[]).is_none());
        // Two books on one slot is a broken catalog.
        let mut sources = library();
        sources[1].share_slot = Some(0);
        assert!(slot_map(&sources).is_none());
    }

    #[test]
    fn a_custom_set_round_trips_to_the_same_books() {
        let sources = library();
        let on = ["core", "module", "third"];
        let code = code_for(&sources, active_in(&on)).unwrap();
        let shared = book_code::decode(&code).unwrap();
        // A receiver with every book off turns on exactly those.
        let result = offer(&shared, &sources, |_| false).unwrap();
        let mut turned_on = result.change.activate.clone();
        turned_on.sort();
        assert_eq!(turned_on, ["core", "module", "third"]);
        assert!(result.change.deactivate.is_empty());
        assert_eq!(result.skipped, 0);
        assert_eq!(result.preset, None);
    }

    #[test]
    fn the_receiver_matches_exactly_whatever_it_had() {
        let sources = library();
        let code = code_for(&sources, active_in(&["apg", "companion"])).unwrap();
        let shared = book_code::decode(&code).unwrap();
        let result = offer(&shared, &sources, active_in(&["core", "apg", "third"])).unwrap();
        assert_eq!(result.change.activate, ["companion"]);
        let mut off = result.change.deactivate.clone();
        off.sort();
        assert_eq!(off, ["core", "third"]);
        assert_eq!(result.summary(), "Turns on 1 book, turns off 2 books.");
        assert_eq!(result.active_after, 2);
    }

    #[test]
    fn a_preset_set_is_named_in_the_offer() {
        let sources = library();
        let books = book_presets::books_of(&sources);
        for preset in book_presets::offered(&books) {
            let members = preset.members(&books);
            let code = code_for(&sources, |id| members.contains(id)).unwrap();
            let shared = book_code::decode(&code).unwrap();
            let result = offer(&shared, &sources, |_| false).unwrap();
            assert_eq!(result.preset, Some(preset), "{}", preset.label());
            assert!(result.result_line().ends_with(preset.label()));
        }
    }

    #[test]
    fn a_code_that_matches_what_is_on_changes_nothing() {
        let sources = library();
        let on = ["core", "apg"];
        let code = code_for(&sources, active_in(&on)).unwrap();
        let shared = book_code::decode(&code).unwrap();
        let result = offer(&shared, &sources, active_in(&on)).unwrap();
        assert!(result.is_empty());
        assert_eq!(
            result.summary(),
            "These are the books you already have on. Nothing changes."
        );
        assert_eq!(result.preset, Some(Preset::PaizoRulebooks));
    }

    #[test]
    fn a_code_from_a_newer_library_skips_the_books_this_one_lacks() {
        let sources = library();
        // The sender had two more books at slots 5 and 6, and had both on.
        let newer = Shared {
            described: 7,
            on: [0u32, 5, 6].into_iter().collect(),
        };
        let result = offer(&newer, &sources, |_| false).unwrap();
        assert_eq!(result.change.activate, ["core"]);
        assert_eq!(result.skipped, 2);
        assert_eq!(
            result.skipped_line().as_deref(),
            Some(
                "2 books in this code are not in your library, so they are skipped. A newer library may have them."
            )
        );
        // An off slot with no book is not a skip: nothing was asked.
        let quiet = Shared {
            described: 7,
            on: [0u32].into_iter().collect(),
        };
        assert_eq!(offer(&quiet, &sources, |_| false).unwrap().skipped, 0);
    }

    #[test]
    fn a_book_the_sender_never_had_keeps_its_state() {
        // The code was made on a library of three books. This one has five.
        let sources = library();
        let old = Shared {
            described: 3,
            on: [0u32].into_iter().collect(),
        };
        let result = offer(&old, &sources, active_in(&["module", "third"])).unwrap();
        // Slots 3 and 4 were not described, so they stay on; slots 1 and 2 were off already.
        assert_eq!(result.change.activate, ["core"]);
        assert!(result.change.deactivate.is_empty());
        assert_eq!(result.active_after, 3);
    }

    #[test]
    fn the_code_survives_a_library_that_gains_a_book() {
        // The same slots, one new book at the end: the old code still means the same books.
        let before = library();
        let on = ["core", "companion"];
        let code = code_for(&before, active_in(&on)).unwrap();
        let mut after = library();
        after.insert(0, book("brand-new", Some(5), "RPG", false));
        after.reverse();
        let shared = book_code::decode(&code).unwrap();
        let result = offer(&shared, &after, |_| true).unwrap();
        let mut off = result.change.deactivate.clone();
        off.sort();
        // Everything the code left off goes off, and the new book, which it never described,
        // stays on.
        assert_eq!(off, ["apg", "module", "third"]);
        assert!(result.change.activate.is_empty());
        assert_eq!(result.active_after, 3);
    }

    #[test]
    fn the_summary_reads_for_one_side_and_for_a_single_book() {
        let mut offer = Offer::default();
        offer.change.activate = vec!["a".into()];
        assert_eq!(offer.summary(), "Turns on 1 book.");
        offer.change.activate.clear();
        offer.change.deactivate = (0..1234).map(|i| i.to_string()).collect();
        assert_eq!(offer.summary(), "Turns off 1,234 books.");
        assert!(offer.skipped_line().is_none());
        offer.skipped = 1;
        assert!(
            offer
                .skipped_line()
                .unwrap()
                .starts_with("1 book in this code")
        );
    }

    #[test]
    fn no_active_book_reads_as_no_books_active() {
        let sources = library();
        let shared = book_code::decode(&code_for(&sources, |_| false).unwrap()).unwrap();
        let result = offer(&shared, &sources, |_| true).unwrap();
        assert_eq!(result.change.deactivate.len(), 5);
        assert_eq!(result.result_line(), "No books active");
    }
}
