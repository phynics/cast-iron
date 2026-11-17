//! Quick reference (#437, `#/quick`): the Table tab. Chips to the rules that a table looks up
//! in the middle of play. The page holds ids, anchors and chip labels only; every word of a
//! rule stays in its entry, so nothing here copies text (DESIGN.md, "Quick reference").
//!
//! A chip whose entry the library does not hold is not drawn, and a group with no chip left is
//! not drawn. The list is one constant so that its order is the page's order and a test can
//! check it.

use crate::library::use_library;
use crate::route::Route;
use crate::views::util::set_title;
use cast_iron_schema::EntryId;
use leptos::prelude::*;

/// One chip: the words on it, the entry it opens, and a heading anchor inside that entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chip {
    pub label: &'static str,
    pub id: &'static str,
    /// A `sec-` heading anchor of the entry. Those anchors are derived from the heading text
    /// until #439 freezes them in the corpus.
    pub anchor: Option<&'static str>,
}

/// A titled run of chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub heading: &'static str,
    pub chips: &'static [Chip],
}

const fn chip(label: &'static str, id: &'static str) -> Chip {
    Chip {
        label,
        id,
        anchor: None,
    }
}

const fn chip_at(label: &'static str, id: &'static str, anchor: &'static str) -> Chip {
    Chip {
        label,
        id,
        anchor: Some(anchor),
    }
}

/// The Core Rulebook condition entries, A to Z by label. Grappled and Pinned share one entry
/// (the corpus holds them together), and so do ability damage and ability drain.
const CONDITIONS: &[Chip] = &[
    chip("Ability damage and drain", "ref/rule/ability-damage-drain"),
    chip("Bleed", "ref/rule/bleed"),
    chip("Blinded", "ref/misc/blinded"),
    chip("Broken", "ref/rule/broken-and-fragile"),
    chip("Confused", "ref/misc/confused"),
    chip("Cowering", "ref/misc/cowering"),
    chip("Dazed", "ref/misc/dazed"),
    chip("Dazzled", "ref/misc/dazzled"),
    chip("Dead", "ref/misc/dead"),
    chip("Deafened", "ref/misc/deafened"),
    chip("Disabled", "ref/misc/disabled"),
    chip("Dying", "ref/misc/dying"),
    chip("Energy drained", "ref/rule/energy-drained"),
    chip("Entangled", "ref/rule/entangled"),
    chip("Exhausted", "ref/misc/exhausted"),
    chip("Fascinated", "ref/misc/fascinated"),
    chip("Fatigued", "ref/misc/fatigued"),
    chip("Flat-footed", "ref/rule/flat-footed"),
    chip("Frightened", "ref/misc/frightened"),
    chip("Grappled", "ref/rule/grapple-conditions"),
    chip("Helpless", "ref/misc/helpless"),
    chip("Incorporeal", "ref/rule/incorporeal"),
    chip("Invisible", "ref/rule/invisible"),
    chip("Nauseated", "ref/misc/nauseated"),
    chip("Panicked", "ref/misc/panicked"),
    chip("Paralyzed", "ref/rule/paralyzed"),
    chip("Petrified", "ref/rule/petrified"),
    chip("Pinned", "ref/rule/grapple-conditions"),
    chip("Prone", "ref/rule/prone"),
    chip("Shaken", "ref/misc/shaken"),
    chip("Sickened", "ref/misc/sickened"),
    chip("Stable", "ref/misc/stable"),
    chip("Staggered", "ref/misc/staggered"),
    chip("Stunned", "ref/misc/stunned"),
    chip("Unconscious", "ref/misc/unconscious"),
];

const ACTIONS: &[Chip] = &[
    chip("Action types", "ref/rule/actions-in-combat"),
    chip("Standard", "ref/rule/standard-actions"),
    chip("Move", "ref/rule/move-actions"),
    chip("Full-round", "ref/rule/full-round-actions"),
    chip(
        "Swift and immediate",
        "ref/rule/swift-and-immediate-actions",
    ),
    chip("Free", "ref/rule/free-actions"),
    chip("Miscellaneous", "ref/rule/miscellaneous-actions"),
];

const COMBAT_MODIFIERS: &[Chip] = &[
    chip("Attack roll modifiers", "ref/rule/combat-modifiers"),
    chip_at(
        "AC modifiers",
        "ref/rule/combat-modifiers",
        "sec-table-8-6-armor-class-modifiers",
    ),
    chip("Cover", "ref/rule/cover"),
    chip("Concealment", "ref/rule/concealment"),
    chip("Flanking", "ref/rule/flanking"),
    chip("Helpless defenders", "ref/rule/helpless-defenders"),
];

const EVERY_SESSION: &[Chip] = &[
    chip("Attacks of opportunity", "ref/rule/attacks-of-opportunity"),
    chip_at(
        "What provokes",
        "ref/rule/attacks-of-opportunity",
        "sec-actions-that-provoke-attacks-of-opportunity",
    ),
    chip("Combat maneuvers", "ref/rule/combat-maneuvers"),
    chip("Grapple", "ref/rule/grapple"),
    chip_at(
        "Two-weapon fighting",
        "ref/rule/two-weapon-fighting",
        "sec-table-8-7-two-weapon-fighting-penalties",
    ),
    chip("Charge", "ref/rule/charge"),
    chip("5-foot step", "ref/rule/take-5-foot-step"),
    chip("Concentration", "ref/rule/concentration"),
    chip("Injury and death", "ref/rule/injury-and-death"),
];

/// Every group, in page order. The page, its tests and the browser spec all read this list.
pub const GROUPS: &[Group] = &[
    Group {
        heading: "Conditions",
        chips: CONDITIONS,
    },
    Group {
        heading: "Actions",
        chips: ACTIONS,
    },
    Group {
        heading: "Combat modifiers",
        chips: COMBAT_MODIFIERS,
    },
    Group {
        heading: "Every session",
        chips: EVERY_SESSION,
    },
];

/// The route that a chip opens, or `None` when its id is not a well-formed entry id.
fn chip_route(chip: &Chip) -> Option<Route> {
    let id = EntryId::parse(chip.id).ok()?;
    Some(Route::Entry {
        id,
        anchor: chip.anchor.map(str::to_string),
        book: None,
        section: None,
    })
}

#[component]
pub fn Quick() -> impl IntoView {
    set_title("Quick reference");
    let library = use_library();
    let groups = GROUPS
        .iter()
        .filter_map(|group| {
            let chips: Vec<(&'static str, String)> = group
                .chips
                .iter()
                .filter(|chip| {
                    EntryId::parse(chip.id)
                        .ok()
                        .is_some_and(|id| library.stub(&id).is_some())
                })
                .filter_map(|chip| chip_route(chip).map(|route| (chip.label, route.href())))
                .collect();
            if chips.is_empty() {
                return None;
            }
            let heading_id = format!(
                "quick-{}",
                cast_iron_schema::headings::slugify(group.heading)
            );
            let items = chips
                .into_iter()
                .map(|(label, href)| {
                    view! { <li><a class="jump" href=href>{label}</a></li> }
                })
                .collect_view();
            let labelled_by = heading_id.clone();
            Some(view! {
                <section aria-labelledby=labelled_by>
                    <h2 id=heading_id class="sub-head">{group.heading}</h2>
                    <ul class="jump-list">{items}</ul>
                </section>
            })
        })
        .collect_view();
    view! {
        <h1>"Quick reference"</h1>
        {groups}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn all() -> impl Iterator<Item = &'static Chip> {
        GROUPS.iter().flat_map(|group| group.chips.iter())
    }

    #[test]
    fn groups_come_in_the_designed_order() {
        let headings: Vec<_> = GROUPS.iter().map(|g| g.heading).collect();
        assert_eq!(
            headings,
            ["Conditions", "Actions", "Combat modifiers", "Every session"]
        );
        assert_eq!(GROUPS[0].chips.len(), 35);
        assert_eq!(GROUPS[1].chips.len(), 7);
        assert_eq!(GROUPS[2].chips.len(), 6);
        assert_eq!(GROUPS[3].chips.len(), 9);
        let labels =
            |group: usize| -> Vec<&str> { GROUPS[group].chips.iter().map(|c| c.label).collect() };
        assert_eq!(
            labels(1),
            [
                "Action types",
                "Standard",
                "Move",
                "Full-round",
                "Swift and immediate",
                "Free",
                "Miscellaneous"
            ]
        );
        assert_eq!(
            labels(2),
            [
                "Attack roll modifiers",
                "AC modifiers",
                "Cover",
                "Concealment",
                "Flanking",
                "Helpless defenders"
            ]
        );
        assert_eq!(
            labels(3),
            [
                "Attacks of opportunity",
                "What provokes",
                "Combat maneuvers",
                "Grapple",
                "Two-weapon fighting",
                "Charge",
                "5-foot step",
                "Concentration",
                "Injury and death"
            ]
        );
    }

    #[test]
    fn conditions_run_a_to_z() {
        let labels: Vec<String> = GROUPS[0]
            .chips
            .iter()
            .map(|c| c.label.to_lowercase())
            .collect();
        let mut sorted = labels.clone();
        sorted.sort();
        assert_eq!(labels, sorted);
    }

    #[test]
    fn labels_are_unique() {
        let mut seen = HashSet::new();
        for chip in all() {
            assert!(seen.insert(chip.label), "duplicate label {}", chip.label);
        }
    }

    #[test]
    fn each_target_appears_once_except_the_declared_shared_entries() {
        // One entry holds both halves of these pairs, and the corpus has no anchor between
        // them, so two chips open the same target. Any other repeat is a mistake.
        const SHARED: &[(&str, &str)] = &[("Grappled", "Pinned")];
        let mut first_seen: Vec<(&Chip, &str)> = Vec::new();
        for chip in all() {
            match first_seen
                .iter()
                .find(|(c, _)| c.id == chip.id && c.anchor == chip.anchor)
            {
                None => first_seen.push((chip, chip.label)),
                Some((_, first)) => assert!(
                    SHARED.contains(&(*first, chip.label)),
                    "{} repeats the target of {first}",
                    chip.label
                ),
            }
        }
    }

    #[test]
    fn every_chip_is_a_canonical_id_and_an_anchor_is_a_derived_section() {
        for chip in all() {
            assert!(chip.id.starts_with("ref/"), "{} is not a ref id", chip.id);
            assert!(EntryId::parse(chip.id).is_ok(), "{}", chip.id);
            if let Some(anchor) = chip.anchor {
                assert!(
                    anchor.starts_with("sec-"),
                    "{} has anchor {anchor}",
                    chip.label
                );
            }
            let route = chip_route(chip).expect("route");
            assert_eq!(Route::parse(&route.href()), route, "{}", chip.label);
        }
    }
}
