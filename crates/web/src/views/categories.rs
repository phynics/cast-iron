//! The set of categories, their order, and the ways to browse each one.
//!
//! The order is a product decision (`PRODUCT.md`): quick lookup during play and character
//! research are equal, so the order puts the things used at the table first and the character
//! building categories next, then the rest. Matches over `EntryKind` stay exhaustive, so a new
//! kind cannot reach the hub without a description.

use crate::route::Route;
use cast_iron_schema::EntryKind;

/// The categories that `PRODUCT.md` names, in the order of the hub.
pub const PRIMARY: [EntryKind; 9] = [
    EntryKind::Rule,
    EntryKind::Spell,
    EntryKind::Feat,
    EntryKind::Class,
    EntryKind::PrestigeClass,
    EntryKind::Race,
    EntryKind::Trait,
    EntryKind::Item,
    EntryKind::Creature,
];

/// The remaining kinds. Archetypes and racial traits are reached from their class or race, and
/// they are categories of their own for readers who want the whole list.
pub const MORE: [EntryKind; 5] = [
    EntryKind::Archetype,
    EntryKind::RacialTrait,
    EntryKind::Skill,
    EntryKind::Affliction,
    EntryKind::Trap,
];

/// One line on what a category holds and how it is browsed.
pub fn blurb(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Rule => "Combat, magic, movement, and every other chapter of the rules text",
        EntryKind::Spell => "Spell descriptions, with lists by class and level",
        EntryKind::Feat => "Feats by type, with prerequisites and the chains they form",
        EntryKind::Class => "Core, base, and NPC classes, with their archetypes",
        EntryKind::PrestigeClass => "Advanced classes and what a character needs to enter them",
        EntryKind::Race => "Player and monster races, with their racial traits",
        EntryKind::Trait => "Character traits and drawbacks, by category",
        EntryKind::Item => "Weapons, armor, gear, and magic items",
        EntryKind::Creature => "Bestiary creatures, by type and challenge rating",
        EntryKind::Archetype => "Class variants, each listed under its class",
        EntryKind::RacialTrait => "Standard and alternate traits, each listed under its race",
        EntryKind::Skill => "What each skill does and how to use it",
        EntryKind::Affliction => "Diseases, poisons, curses, and other lasting effects",
        EntryKind::Trap => "Traps and their mechanics",
    }
}

/// Counts split into the primary categories (in hub order) and the rest, without empty kinds.
/// A kind that is in neither list lands in the second group, so it is never lost.
#[allow(clippy::type_complexity)]
pub fn arrange(
    counts: &[(EntryKind, usize)],
) -> (Vec<(EntryKind, usize)>, Vec<(EntryKind, usize)>) {
    let count_of = |kind: EntryKind| {
        counts
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, n)| *n)
            .filter(|n| *n > 0)
    };
    let primary = PRIMARY
        .iter()
        .filter_map(|&k| count_of(k).map(|n| (k, n)))
        .collect();
    let mut more: Vec<(EntryKind, usize)> = MORE
        .iter()
        .filter_map(|&k| count_of(k).map(|n| (k, n)))
        .collect();
    for &(kind, n) in counts {
        if n > 0 && !PRIMARY.contains(&kind) && !MORE.contains(&kind) {
            more.push((kind, n));
        }
    }
    (primary, more)
}

/// A way to look at a category besides the A to Z list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lens {
    /// A page of its own, such as spell lists.
    Page {
        label: &'static str,
        blurb: &'static str,
        route: Route,
    },
    /// The values of a search facet, each a link to the search results for that value.
    Facet {
        field: &'static str,
        label: &'static str,
    },
}

pub fn lenses(kind: EntryKind) -> Vec<Lens> {
    let facet = |field, label| Lens::Facet { field, label };
    match kind {
        EntryKind::Spell => vec![
            Lens::Page {
                label: "By class and level",
                blurb: "The spell list of each class, one level at a time",
                route: Route::Spells,
            },
            facet("school", "By school"),
        ],
        EntryKind::Feat => vec![
            Lens::Page {
                label: "Feat chains",
                blurb: "Feats that unlock others, and the feats that follow from them",
                route: Route::FeatChains,
            },
            facet("feat_type", "By feat type"),
        ],
        EntryKind::Class => vec![facet("class_type", "Core, base, and NPC")],
        EntryKind::Race => vec![facet("race_group", "By group")],
        EntryKind::Trait => vec![facet("trait_category", "By trait category")],
        EntryKind::Item => vec![facet("item_category", "By category")],
        EntryKind::Creature => vec![
            facet("creature_type", "By type"),
            facet("cr", "By challenge rating"),
        ],
        EntryKind::Archetype => vec![facet("archetype_class", "By class")],
        EntryKind::Rule
        | EntryKind::PrestigeClass
        | EntryKind::RacialTrait
        | EntryKind::Skill
        | EntryKind::Affliction
        | EntryKind::Trap => Vec::new(),
    }
}

/// Facet values a lens leaves out. The class lens is for base, core, and NPC classes; prestige
/// classes are a category of their own.
pub fn lens_skips(kind: EntryKind, field: &str, value: &str) -> bool {
    kind == EntryKind::Class && field == "class_type" && value == "prestige"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_place_in_the_hub() {
        let mut all: Vec<EntryKind> = PRIMARY.iter().chain(MORE.iter()).copied().collect();
        assert_eq!(all.len(), EntryKind::ALL.len());
        all.sort();
        all.dedup();
        assert_eq!(all.len(), EntryKind::ALL.len(), "a kind appears twice");
        for kind in EntryKind::ALL {
            assert!(all.contains(&kind), "{kind:?} is not in the hub");
            assert!(!blurb(kind).is_empty());
            assert!(!blurb(kind).ends_with('.'), "one line, no full stop");
        }
    }

    #[test]
    fn the_hub_order_is_play_then_character_research() {
        let counts: Vec<(EntryKind, usize)> = EntryKind::ALL.iter().map(|&k| (k, 3)).collect();
        let (primary, more) = arrange(&counts);
        let names: Vec<&str> = primary.iter().map(|(k, _)| k.label()).collect();
        assert_eq!(
            names,
            [
                "Rules",
                "Spells",
                "Feats",
                "Classes",
                "Prestige classes",
                "Races",
                "Traits",
                "Items",
                "Creatures"
            ]
        );
        assert_eq!(more.len(), 5);
    }

    #[test]
    fn empty_kinds_are_left_out_and_unknown_ones_are_kept() {
        // An old library has no traits, racial traits, or archetypes.
        let counts = [
            (EntryKind::Spell, 10),
            (EntryKind::Trait, 0),
            (EntryKind::Skill, 4),
            (EntryKind::Archetype, 0),
        ];
        let (primary, more) = arrange(&counts);
        assert_eq!(primary, [(EntryKind::Spell, 10)]);
        assert_eq!(more, [(EntryKind::Skill, 4)]);
    }

    #[test]
    fn lenses_point_at_pages_and_facets_the_index_has() {
        let spell = lenses(EntryKind::Spell);
        assert!(matches!(
            &spell[0],
            Lens::Page {
                route: Route::Spells,
                ..
            }
        ));
        assert!(matches!(
            &spell[1],
            Lens::Facet {
                field: "school",
                ..
            }
        ));
        assert!(lenses(EntryKind::Feat).iter().any(|l| matches!(
            l,
            Lens::Page {
                route: Route::FeatChains,
                ..
            }
        )));
        assert!(lenses(EntryKind::Rule).is_empty());
        assert!(lens_skips(EntryKind::Class, "class_type", "prestige"));
        assert!(!lens_skips(EntryKind::Class, "class_type", "npc"));
    }
}
