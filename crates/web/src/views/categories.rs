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
    /// The values of a search facet, each a link to the search results for that value.
    Facet {
        field: &'static str,
        label: &'static str,
    },
    /// A facet too large for a row of pills: one filter menu that narrows the list in place.
    Menu(MenuSpec),
    /// A related page, as a plain link in the filter row.
    Link { label: &'static str, route: Route },
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

/// The spell school menu. Nine schools, so it has no find field.
pub const SCHOOL_MENU: MenuSpec = MenuSpec {
    field: "school",
    group: "School",
    all_label: "All schools",
    find_label: "Find a school",
    list_label: "Spell school",
    singular: "school",
    plural: "schools",
};

/// The trait category menu: Combat, Faith, Regional and the rest.
pub const TRAIT_CATEGORY_MENU: MenuSpec = MenuSpec {
    field: "trait_category",
    group: "Category",
    all_label: "All categories",
    find_label: "Find a category",
    list_label: "Trait category",
    singular: "category",
    plural: "categories",
};

/// The item category menu. About 57 categories, so the find field matters.
pub const ITEM_CATEGORY_MENU: MenuSpec = MenuSpec {
    field: "item_category",
    group: "Category",
    all_label: "All categories",
    find_label: "Find a category",
    list_label: "Item category",
    singular: "category",
    plural: "categories",
};

/// The archetype menu by the class it changes.
pub const ARCHETYPE_CLASS_MENU: MenuSpec = MenuSpec {
    field: "archetype_class",
    group: "Class",
    all_label: "All classes",
    find_label: "Find a class",
    list_label: "Archetype class",
    singular: "class",
    plural: "classes",
};

/// The creature type menu. Thirteen types, so it has a find field.
pub const CREATURE_TYPE_MENU: MenuSpec = MenuSpec {
    field: "creature_type",
    group: "Type",
    all_label: "All types",
    find_label: "Find a type",
    list_label: "Creature type",
    singular: "type",
    plural: "types",
};

/// The creature challenge rating menu. About 35 values, shown in numeric order by
/// `sort_facet_pairs`, with a find field because there are more than ten.
pub const CREATURE_CR_MENU: MenuSpec = MenuSpec {
    field: "cr",
    group: "CR",
    all_label: "All CRs",
    find_label: "Find a CR",
    list_label: "Challenge rating",
    singular: "rating",
    plural: "ratings",
};

/// The facet that sorts a kind's list into headed sections instead of A to Z. A short list with
/// a handful of well-known groups shows them all at once, so it needs no filter.
pub const CLASS_GROUP_FIELD: &str = "class_type";

/// The facet a kind's list is grouped by, if it is.
pub fn grouped_by(kind: EntryKind) -> Option<&'static str> {
    (kind == EntryKind::Class).then_some(CLASS_GROUP_FIELD)
}

/// The class groups in the order the books present them: the Core Rulebook classes, then the
/// base classes of later books, the hybrids, the occult classes, the unchained revisions, the
/// alternate classes and the non-player classes. Prestige classes are a category of their own.
const CLASS_GROUP_ORDER: [&str; 7] = [
    "core",
    "base",
    "hybrid",
    "occult",
    "unchained",
    "alternate",
    "npc",
];

/// The class group values in display order. A value that is not in the known list keeps its
/// place after them, alphabetically, so a new group in the corpus is shown rather than lost.
/// The values are the corpus's own and are not renamed here.
pub fn order_class_groups(values: &[String]) -> Vec<String> {
    let rank = |value: &str| {
        CLASS_GROUP_ORDER
            .iter()
            .position(|known| known.eq_ignore_ascii_case(value))
            .unwrap_or(CLASS_GROUP_ORDER.len())
    };
    let mut ordered: Vec<String> = values
        .iter()
        .filter(|value| !lens_skips(EntryKind::Class, CLASS_GROUP_FIELD, value))
        .cloned()
        .collect();
    ordered.sort_by_key(|value| (rank(value), value.to_lowercase()));
    ordered
}

pub fn lenses(kind: EntryKind) -> Vec<Lens> {
    let facet = |field, label| Lens::Facet { field, label };
    match kind {
        EntryKind::Spell => vec![
            Lens::Menu(SCHOOL_MENU),
            Lens::Link {
                label: "Spell lists",
                route: Route::Spells,
            },
        ],
        EntryKind::Feat => vec![
            Lens::Menu(FEAT_TYPE_MENU),
            Lens::Link {
                label: "Feat chains",
                route: Route::FeatChains,
            },
        ],
        // Classes are listed under headings by `class_type`, see `grouped_by`.
        EntryKind::Class => Vec::new(),
        EntryKind::Race => vec![facet("race_group", "By group")],
        EntryKind::Trait => vec![Lens::Menu(TRAIT_CATEGORY_MENU)],
        EntryKind::Item => vec![Lens::Menu(ITEM_CATEGORY_MENU)],
        EntryKind::Creature => vec![Lens::Menu(CREATURE_TYPE_MENU), Lens::Menu(CREATURE_CR_MENU)],
        EntryKind::Archetype => vec![Lens::Menu(ARCHETYPE_CLASS_MENU)],
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

/// Facet values the class groups leave out. Prestige classes are a category of their own.
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
        // Spells: a menu for the school, and a plain link to the spell lists.
        let spell = lenses(EntryKind::Spell);
        assert_eq!(spell.len(), 2);
        assert_eq!(spell[0], Lens::Menu(SCHOOL_MENU));
        assert!(matches!(
            &spell[1],
            Lens::Link {
                label: "Spell lists",
                route: Route::Spells,
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
        // Only a field with a filter menu of the kind narrows its list in place.
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
        assert_eq!(
            applied_facets(EntryKind::Spell, &url),
            [facet("school", "evocation")]
        );
        assert!(lens_skips(EntryKind::Class, "class_type", "prestige"));
        assert!(!lens_skips(EntryKind::Class, "class_type", "npc"));
    }

    #[test]
    fn the_large_kinds_have_one_menu_each_and_classes_are_grouped_instead() {
        let menu = |kind| match lenses(kind).as_slice() {
            [Lens::Menu(spec), ..] => Some(*spec),
            _ => None,
        };
        assert_eq!(menu(EntryKind::Item), Some(ITEM_CATEGORY_MENU));
        assert_eq!(menu(EntryKind::Trait), Some(TRAIT_CATEGORY_MENU));
        assert_eq!(menu(EntryKind::Archetype), Some(ARCHETYPE_CLASS_MENU));
        assert_eq!(ITEM_CATEGORY_MENU.field, "item_category");
        assert_eq!(TRAIT_CATEGORY_MENU.field, "trait_category");
        assert_eq!(ARCHETYPE_CLASS_MENU.field, "archetype_class");
        // The fields are the ones the index holds for the kind.
        for (kind, spec) in [
            (EntryKind::Item, ITEM_CATEGORY_MENU),
            (EntryKind::Trait, TRAIT_CATEGORY_MENU),
            (EntryKind::Archetype, ARCHETYPE_CLASS_MENU),
            (EntryKind::Spell, SCHOOL_MENU),
        ] {
            let url = [(spec.field.to_string(), "x".to_string())];
            assert_eq!(applied_facets(kind, &url), url, "{kind:?}");
        }
        // Creatures: two menus, type and challenge rating, that narrow the one list together.
        assert_eq!(
            lenses(EntryKind::Creature),
            [Lens::Menu(CREATURE_TYPE_MENU), Lens::Menu(CREATURE_CR_MENU)]
        );
        assert_eq!(CREATURE_TYPE_MENU.field, "creature_type");
        assert_eq!(CREATURE_CR_MENU.field, "cr");
        // Classes: headed sections, no menu, no pills. Races keep their old lens until the
        // library has group data.
        assert!(lenses(EntryKind::Class).is_empty());
        assert_eq!(grouped_by(EntryKind::Class), Some("class_type"));
        assert_eq!(grouped_by(EntryKind::Item), None);
        assert!(matches!(lenses(EntryKind::Race)[..], [Lens::Facet { .. }]));
    }

    #[test]
    fn two_creature_facets_narrow_the_list_together_and_keep_their_url_order() {
        let facet = |f: &str, v: &str| (f.to_string(), v.to_string());
        let url = [facet("creature_type", "dragon"), facet("cr", "10")];
        assert_eq!(
            applied_facets(EntryKind::Creature, &url),
            [facet("creature_type", "dragon"), facet("cr", "10")]
        );
        // A second value for one field is not a second filter: the list narrows by one value
        // for each field, so a hand-made URL keeps the first.
        let repeated = [
            facet("creature_type", "dragon"),
            facet("creature_type", "ooze"),
            facet("cr", "10"),
        ];
        assert_eq!(
            applied_facets(EntryKind::Creature, &repeated),
            [facet("creature_type", "dragon"), facet("cr", "10")]
        );
        // Facets a creature menu does not own stay with search.
        assert_eq!(
            applied_facets(EntryKind::Creature, &[facet("size", "Large")]),
            []
        );
    }

    #[test]
    fn class_groups_follow_the_order_of_the_books_and_keep_unknown_ones() {
        let values = |list: &[&str]| list.iter().map(|v| v.to_string()).collect::<Vec<_>>();
        assert_eq!(
            order_class_groups(&values(&[
                "npc",
                "alternate",
                "prestige",
                "unchained",
                "occult",
                "hybrid",
                "base",
                "core"
            ])),
            [
                "core",
                "base",
                "hybrid",
                "occult",
                "unchained",
                "alternate",
                "npc"
            ]
        );
        // A group the list does not know goes last, alphabetically; casing is the corpus's.
        assert_eq!(
            order_class_groups(&values(&["Zeta", "core", "alpha", "NPC"])),
            ["core", "NPC", "alpha", "Zeta"]
        );
        assert!(order_class_groups(&[]).is_empty());
    }
}
