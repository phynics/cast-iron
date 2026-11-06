//! The set of categories, their order, and the ways to browse each one.
//!
//! The order is a product decision (`PRODUCT.md`): quick lookup during play and character
//! research are equal, so the order puts the things used at the table first and the character
//! building categories next, then the rest. Matches over `EntryKind` stay exhaustive, so a new
//! kind cannot reach the hub without a description.

use crate::route::Route;
use crate::views::filter_menu::MenuSpec;
use crate::views::search::list_facets;
use cast_iron_schema::EntryKind;

/// The categories that `PRODUCT.md` names, in the order of the hub.
pub const PRIMARY: [EntryKind; 8] = [
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
        EntryKind::Rule => "Rules are read through Books and search",
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
        if n > 0 && kind != EntryKind::Rule && !PRIMARY.contains(&kind) && !MORE.contains(&kind) {
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
    /// A facet too large for a row of pills: one filter menu that narrows the list in place.
    Menu(MenuSpec),
    /// A related page, as a plain link in the filter row.
    Link {
        label: &'static str,
        route: Route,
    },
}

/// The feat type menu. About 40 types, so it has a find field.
pub const FEAT_TYPE_MENU: MenuSpec = MenuSpec {
    field: "feat_type",
    group: "Type",
    all_label: "All types",
    find_label: "Find a type",
    list_label: "Feat type",
    singular: "type",
    plural: "types",
};

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
            Lens::Menu(FEAT_TYPE_MENU),
            Lens::Link {
                label: "Feat chains",
                route: Route::FeatChains,
            },
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

/// The facets a kind's list page applies in place: those a filter menu of the kind owns, one
/// value for each. Other facets in a URL are left to search.
pub fn applied_facets(kind: EntryKind, facets: &[(String, String)]) -> Vec<(String, String)> {
    let menus: Vec<&str> = lenses(kind)
        .into_iter()
        .filter_map(|lens| match lens {
            Lens::Menu(spec) => Some(spec.field),
            _ => None,
        })
        .collect();
    list_facets(facets, kind)
        .into_iter()
        .filter(|(field, _)| menus.contains(&field.as_str()))
        .collect()
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
    fn every_browse_kind_has_one_place_and_rules_stay_in_books() {
        let mut all: Vec<EntryKind> = PRIMARY.iter().chain(MORE.iter()).copied().collect();
        assert_eq!(all.len(), EntryKind::ALL.len() - 1);
        all.sort();
        all.dedup();
        assert_eq!(all.len(), EntryKind::ALL.len() - 1, "a kind appears twice");
        for kind in EntryKind::ALL {
            assert_eq!(
                all.contains(&kind),
                kind != EntryKind::Rule,
                "unexpected hub membership for {kind:?}"
            );
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
    fn rules_are_excluded_while_unknown_kinds_are_kept() {
        // An old library has no traits, racial traits, or archetypes.
        let counts = [
            (EntryKind::Spell, 10),
            (EntryKind::Trait, 0),
            (EntryKind::Skill, 4),
            (EntryKind::Archetype, 0),
            (EntryKind::Rule, 99),
        ];
        let (primary, more) = arrange(&counts);
        assert_eq!(primary, [(EntryKind::Spell, 10)]);
        assert_eq!(more, [(EntryKind::Skill, 4)]);
        assert!(!primary.iter().any(|(kind, _)| *kind == EntryKind::Rule));
        assert!(!more.iter().any(|(kind, _)| *kind == EntryKind::Rule));
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
        // Feats: a menu for the type (the field the index holds), and a plain link to chains.
        let feat = lenses(EntryKind::Feat);
        assert_eq!(feat.len(), 2);
        assert_eq!(feat[0], Lens::Menu(FEAT_TYPE_MENU));
        assert_eq!(FEAT_TYPE_MENU.field, "feat_type");
        assert!(matches!(
            &feat[1],
            Lens::Link {
                route: Route::FeatChains,
                ..
            }
        ));
        // Only a field with a filter menu narrows a list in place. Spells have no menu yet.
        let facet = |f: &str, v: &str| (f.to_string(), v.to_string());
        let url = [
            facet("feat_type", "Combat"),
            facet("feat_type", "General"),
            facet("school", "evocation"),
        ];
        assert_eq!(
            applied_facets(EntryKind::Feat, &url),
            [facet("feat_type", "Combat")]
        );
        assert!(applied_facets(EntryKind::Spell, &url).is_empty());
        assert!(lens_skips(EntryKind::Class, "class_type", "prestige"));
        assert!(!lens_skips(EntryKind::Class, "class_type", "npc"));
    }
}
