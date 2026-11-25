//! The order of a title-sorted list, with modifier titles placed after their base (#678).
//!
//! A reader who looks for `Weapon Focus` also wants `Greater Weapon Focus`, and print files the
//! two together. A plain title sort files the second under G. This module gives each row a
//! place in the list from the titles of the whole list:
//!
//! - A title that begins with a modifier word (`Greater`, `Improved`, ...) and whose rest is the
//!   title of another row sits directly after that row, under that row's letter. It keeps its
//!   printed title.
//! - When no row has the rest as its title, two or more modifier titles of the same rest still
//!   gather: they sit after the weakest of them (`Greater Bull Rush` after `Improved Bull Rush`,
//!   under I). A modifier title that is alone (`Improved Initiative`) does not move.
//! - Inside a family the base comes first, then the members by [`LADDER`] strength, then by
//!   title.
//! - A mythic version (`X, Mythic`) sits directly after the row `X`, whatever `X` is.
//!
//! Modifiers are a closed list matched as a whole leading word. `Extra`, `Master` and `Expert`
//! are not on it: they name feats of their own. Only a leading modifier moves a row: a title
//! with a trailing modifier (`Restoration, Greater`) already sorts beside its base, and `, Mythic`
//! is the one trailing form the corpus uses for a version of another row.
//!
//! The order depends on the set of titles, so it is computed for each list, never for one row.
//! It reads each title once and looks the rest up in hash sets, so it costs one sort and
//! `O(n)` lookups for a list of any size. The output never follows hash-map iteration order: the
//! sets and maps are only looked up, and the sort key ends in the row's id and its position.

use crate::views::util::{group_letter, sort_key};
use std::collections::{HashMap, HashSet};

/// The modifier words and their strength, weakest first. Words on one step have no order of
/// their own (`Lesser` and `Minor`); they sort by title. `Advanced` is the first step above the
/// base, as in `Advanced / Greater / Superior Gathlain Magic`. `Mass` and `Communal` name a
/// spell's group form, which reads as a step above `Greater`.
const LADDER: &[(&str, u8)] = &[
    ("lesser", 1),
    ("minor", 1),
    ("improved", 2),
    ("advanced", 2),
    ("greater", 3),
    ("major", 3),
    ("mass", 4),
    ("communal", 4),
    ("superior", 5),
    ("grand", 6),
    ("masterful", 7),
    ("supreme", 8),
    ("elite", 8),
    ("true", 9),
    ("legendary", 9),
];

/// The suffix of a mythic version's title.
const MYTHIC: &str = ", mythic";

/// One row's place in the list.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Placed {
    /// The row's index in the slice the list came from.
    pub index: usize,
    /// The letter section the row sits under: the letter of the row it is placed after, which
    /// is its own letter when it does not move.
    pub letter: char,
}

/// The strength of a leading modifier word and the title that follows it, for a lowercase
/// title. `None` when the first word is not a modifier or nothing follows it.
fn split_modifier(lower: &str) -> Option<(u8, &str)> {
    let (word, rest) = lower.split_once(char::is_whitespace)?;
    let rest = rest.trim_start();
    let (_, rank) = LADDER.iter().find(|(name, _)| *name == word)?;
    (!rest.is_empty()).then_some((*rank, rest))
}

/// The base of a mythic version, for a lowercase title.
fn mythic_base(lower: &str) -> Option<&str> {
    lower
        .strip_suffix(MYTHIC)
        .filter(|base| !base.trim().is_empty())
}

/// What the list knows about its titles.
struct Titles<'a> {
    /// Every lowercase title of the list.
    present: HashSet<&'a str>,
    /// For a rest that is no title of the list but has two or more modifier titles: the weakest
    /// of those titles.
    heads: HashMap<&'a str, &'a str>,
}

impl<'a> Titles<'a> {
    fn new(lowers: &'a [String]) -> Self {
        let present: HashSet<&str> = lowers.iter().map(String::as_str).collect();
        // `(members, weakest)` per rest. A title counts once; the result does not depend on the
        // order the set is read in.
        let mut groups: HashMap<&str, (usize, (u8, &str))> = HashMap::new();
        for &title in &present {
            if mythic_base(title).is_some() {
                continue;
            }
            let Some((rank, rest)) = split_modifier(title) else {
                continue;
            };
            if present.contains(rest) {
                continue;
            }
            let entry = groups.entry(rest).or_insert((0, (rank, title)));
            entry.0 += 1;
            entry.1 = entry.1.min((rank, title));
        }
        let heads = groups
            .into_iter()
            .filter(|(_, (members, _))| *members >= 2)
            .map(|(rest, (_, (_, head)))| (rest, head))
            .collect();
        Titles { present, heads }
    }

    /// Where a title sorts: the title it follows, and its strength after it (0 for a row that
    /// does not follow another).
    fn place(&self, title: &'a str) -> (&'a str, u8) {
        let Some((rank, rest)) = split_modifier(title) else {
            return (title, 0);
        };
        if self.present.contains(rest) {
            // The rest may itself follow another row (`Improved Greater Foo`): follow that one.
            return (self.place(rest).0, rank);
        }
        match self.heads.get(rest) {
            Some(&head) if head != title => (head, rank),
            _ => (title, 0),
        }
    }
}

/// The order of a list: each row's index and letter section, in display order. `rows` holds
/// each row's title and id, in any order.
pub fn list_order(rows: &[(&str, &str)]) -> Vec<Placed> {
    let lowers: Vec<String> = rows.iter().map(|(title, _)| sort_key(title)).collect();
    let titles = Titles::new(&lowers);
    // (the title the row follows, strength, 1 for a mythic version, own title, id, position)
    let keys: Vec<(&str, u8, u8, &str, &str, usize)> = lowers
        .iter()
        .zip(rows)
        .enumerate()
        .map(|(index, (lower, (_, id)))| {
            let (anchor, rank, mythic) = match mythic_base(lower) {
                Some(base) => {
                    let (anchor, rank) = titles.place(base);
                    (anchor, rank, 1)
                }
                None => {
                    let (anchor, rank) = titles.place(lower);
                    (anchor, rank, 0)
                }
            };
            (anchor, rank, mythic, lower.as_str(), *id, index)
        })
        .collect();
    let mut order: Vec<usize> = (0..keys.len()).collect();
    cast_iron_sort::sort_by(&mut order, |&a, &b| keys[a].cmp(&keys[b]));
    order
        .into_iter()
        .map(|index| Placed {
            index,
            letter: group_letter(keys[index].0),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The titles in display order, each with its letter section.
    fn arranged(titles: &[&str]) -> Vec<(char, String)> {
        let ids: Vec<String> = (0..titles.len()).map(|i| format!("ref/x/{i:05}")).collect();
        let rows: Vec<(&str, &str)> = titles
            .iter()
            .copied()
            .zip(ids.iter().map(String::as_str))
            .collect();
        list_order(&rows)
            .into_iter()
            .map(|placed| (placed.letter, titles[placed.index].to_string()))
            .collect()
    }

    fn names(titles: &[&str]) -> Vec<String> {
        arranged(titles)
            .into_iter()
            .map(|(_, title)| title)
            .collect()
    }

    #[test]
    fn a_greater_title_sits_after_its_base_and_its_mythic_version_under_the_base_letter() {
        let shown = arranged(&[
            "Greater Weapon Focus",
            "Weapon Focus, Mythic",
            "Weapon Finesse",
            "Weapon Focus",
            "Greater Weapon Focus, Mythic",
            "Weapon Specialization",
            "Greater Fortitude",
        ]);
        assert_eq!(
            shown,
            [
                ('G', "Greater Fortitude"),
                ('W', "Weapon Finesse"),
                ('W', "Weapon Focus"),
                ('W', "Weapon Focus, Mythic"),
                ('W', "Greater Weapon Focus"),
                ('W', "Greater Weapon Focus, Mythic"),
                ('W', "Weapon Specialization"),
            ]
            .map(|(letter, title)| (letter, title.to_string()))
        );
    }

    #[test]
    fn a_family_runs_base_then_improved_then_greater() {
        assert_eq!(
            names(&[
                "Greater Two-Weapon Fighting",
                "Two-Weapon Defense",
                "Improved Two-Weapon Fighting",
                "Two-Weapon Fighting",
                "Two-Weapon Fighting, Mythic",
            ]),
            [
                "Two-Weapon Defense",
                "Two-Weapon Fighting",
                "Two-Weapon Fighting, Mythic",
                "Improved Two-Weapon Fighting",
                "Greater Two-Weapon Fighting",
            ]
        );
    }

    #[test]
    fn without_a_base_the_members_gather_at_the_weakest_one() {
        let shown = arranged(&[
            "Improved Bull Rush, Mythic",
            "Greater Bull Rush",
            "Improved Bull Rush",
            "Improved Bull Rushing Charge",
            "Bull Strike",
            "Greater Bull Rush, Mythic",
        ]);
        assert_eq!(
            shown,
            [
                ('B', "Bull Strike"),
                ('I', "Improved Bull Rush"),
                ('I', "Improved Bull Rush, Mythic"),
                ('I', "Greater Bull Rush"),
                ('I', "Greater Bull Rush, Mythic"),
                ('I', "Improved Bull Rushing Charge"),
            ]
            .map(|(letter, title)| (letter, title.to_string()))
        );
    }

    #[test]
    fn a_lone_modifier_title_does_not_move() {
        assert_eq!(
            arranged(&["Improved Initiative", "Initiative Boost", "Dodge"]),
            [
                ('D', "Dodge"),
                ('I', "Improved Initiative"),
                ('I', "Initiative Boost")
            ]
            .map(|(letter, title)| (letter, title.to_string()))
        );
        // The rest of a lone modifier title is not a row, and the mythic version of a lone
        // title follows that title.
        assert_eq!(
            names(&["Improved Initiative, Mythic", "Improved Initiative"]),
            ["Improved Initiative", "Improved Initiative, Mythic"]
        );
    }

    #[test]
    fn rage_power_totems_run_lesser_then_greater_after_the_base() {
        assert_eq!(
            names(&[
                "Greater Spirit Totem",
                "Spirit Totem",
                "Lesser Spirit Totem",
                "Spirit Vengeance",
                "Spirit Totem, Mythic",
            ]),
            [
                "Spirit Totem",
                "Spirit Totem, Mythic",
                "Lesser Spirit Totem",
                "Greater Spirit Totem",
                "Spirit Vengeance",
            ]
        );
    }

    #[test]
    fn a_mass_spell_sits_after_its_base() {
        assert_eq!(
            names(&[
                "Mass Cure Light Wounds",
                "Cure Moderate Wounds",
                "Cure Light Wounds",
                "Mass Cure Moderate Wounds",
            ]),
            [
                "Cure Light Wounds",
                "Mass Cure Light Wounds",
                "Cure Moderate Wounds",
                "Mass Cure Moderate Wounds",
            ]
        );
    }

    #[test]
    fn extra_master_and_expert_are_not_modifiers() {
        let shown = arranged(&[
            "Rage",
            "Extra Rage",
            "Master Craftsman",
            "Craftsman",
            "Expert Tracker",
            "Tracker",
        ]);
        assert_eq!(
            shown,
            [
                ('C', "Craftsman"),
                ('E', "Expert Tracker"),
                ('E', "Extra Rage"),
                ('M', "Master Craftsman"),
                ('R', "Rage"),
                ('T', "Tracker"),
            ]
            .map(|(letter, title)| (letter, title.to_string()))
        );
    }

    #[test]
    fn a_title_that_only_starts_with_a_modifier_word_stays() {
        // `Greater Than` has no base row, and `Greatest Hit` and `Truesight` do not start with
        // the whole word.
        assert_eq!(
            arranged(&["Greater Than", "Greatest Hit", "Truesight", "Hit", "Sight"]),
            [
                ('G', "Greater Than"),
                ('G', "Greatest Hit"),
                ('H', "Hit"),
                ('S', "Sight"),
                ('T', "Truesight"),
            ]
            .map(|(letter, title)| (letter, title.to_string()))
        );
        // A modifier word alone is a title of its own, not a modifier.
        assert_eq!(
            names(&["Greater", "Greater Foo"]),
            ["Greater", "Greater Foo"]
        );
    }

    #[test]
    fn modifiers_run_in_the_order_of_the_ladder() {
        assert_eq!(
            names(&[
                "True Foo",
                "Masterful Foo",
                "Grand Foo",
                "Superior Foo",
                "Major Foo",
                "Greater Foo",
                "Improved Foo",
                "Minor Foo",
                "Lesser Foo",
                "Advanced Foo",
                "Foo",
            ]),
            [
                "Foo",
                "Lesser Foo",
                "Minor Foo",
                "Advanced Foo",
                "Improved Foo",
                "Greater Foo",
                "Major Foo",
                "Superior Foo",
                "Grand Foo",
                "Masterful Foo",
                "True Foo",
            ]
        );
    }

    #[test]
    fn matching_ignores_case_and_the_order_is_the_same_for_any_input_order() {
        let titles = [
            "greater weapon focus",
            "WEAPON FOCUS",
            "Weapon Focus, Mythic",
            "Zeal",
            "Improved Bull Rush",
            "Greater Bull Rush",
        ];
        let first = names(&titles);
        let mut reversed = titles;
        reversed.reverse();
        assert_eq!(first, names(&reversed));
        assert_eq!(
            first,
            [
                "Improved Bull Rush",
                "Greater Bull Rush",
                "WEAPON FOCUS",
                "Weapon Focus, Mythic",
                "greater weapon focus",
                "Zeal",
            ]
        );
    }

    #[test]
    fn rows_of_one_title_keep_the_order_of_their_ids() {
        let rows = [
            ("Weapon Focus", "ref/b"),
            ("Greater Weapon Focus", "ref/z"),
            ("Weapon Focus", "ref/a"),
            ("Greater Weapon Focus", "ref/y"),
        ];
        let ids: Vec<&str> = list_order(&rows)
            .into_iter()
            .map(|placed| rows[placed.index].1)
            .collect();
        assert_eq!(ids, ["ref/a", "ref/b", "ref/y", "ref/z"]);
    }

    #[test]
    fn a_mythic_version_without_its_base_keeps_the_place_of_the_base() {
        // Mirrors the old feat key: `Dodge, Mythic` sits before `Dodge Anything`.
        assert_eq!(
            names(&["Acrobatic Spellcaster", "Acrobatic, Mythic", "Acrobatic"]),
            ["Acrobatic", "Acrobatic, Mythic", "Acrobatic Spellcaster"]
        );
        assert_eq!(
            names(&["Dodge Anything", "Dodge, Mythic"]),
            ["Dodge, Mythic", "Dodge Anything"]
        );
        // A title that only looks like a suffix keeps its own place.
        assert_eq!(
            names(&["Zed", ", Mythic", "Alpha"]),
            [", Mythic", "Alpha", "Zed"]
        );
    }

    #[test]
    fn a_large_list_orders_without_a_quadratic_scan() {
        let names: Vec<String> = (0..20_000)
            .flat_map(|i| [format!("Feat {i}"), format!("Greater Feat {i}")])
            .collect();
        let rows: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), n.as_str())).collect();
        let order = list_order(&rows);
        assert_eq!(order.len(), 40_000);
        // Each Greater title follows its base.
        let titles: Vec<&str> = order.iter().map(|p| rows[p.index].0).collect();
        let at = |title: &str| titles.iter().position(|t| *t == title).unwrap();
        assert_eq!(at("Greater Feat 1234"), at("Feat 1234") + 1);
        assert!(order.iter().all(|placed| placed.letter == 'F'));
    }
}
