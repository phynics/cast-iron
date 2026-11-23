//! The other printings of an entry: the `Other printings:` line of an entry page (#650).
//!
//! A rule printed in several books is several entries, one per printing. The line links the
//! others of the set that the browse folds already use: the same kind, the same parent and the
//! same normalized title ([`fold_key`]). Three pairs stay out: creatures (one name, different
//! stat blocks), a pair that a `revision` edge joins in either direction (the revision notice
//! already links it), and a pair whose primary book is the same (a duplicate inside one book).
//!
//! Everything here is pure over the catalog stubs, so each rule has a native test. The page
//! finds its candidates by comparing titles without building any, then loads the parent source
//! (`relations.json`, `option-groups.json`) only when a candidate exists.

use crate::graph::Relations;
use crate::row_parents::{self, ParentMap};
use crate::views::book_tier::PrintingOrder;
use crate::views::source_variants::{
    SourceVariant, fold_key, printing_cmp, same_normalized_title, variant_labels,
};
use cast_iron_schema::{EntryId, EntryKind, EntryStub, OptionGroupIndex};

/// One link of the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printing {
    pub id: EntryId,
    /// The book that the link names. Two links that read the same carry a last `(n)`.
    pub label: String,
    /// False when every book that holds the printing is turned off: the link stays, dimmed.
    pub active: bool,
}

/// What the catalog says about a printing besides its stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Described {
    /// The display title of the primary book.
    pub book: String,
    pub order: PrintingOrder,
    pub active: bool,
}

/// True for a kind whose same-name entries are never printings of one rule. A creature of one
/// name prints different stat blocks in different books, so the line would name the wrong page.
pub fn applies(kind: EntryKind) -> bool {
    kind != EntryKind::Creature
}

/// The entries of the page's kind that share its normalized title, other than the page itself.
/// This is the cheap pass: it builds no string for a title that differs, so the scan of a kind
/// of about 8,000 entries costs only the comparisons. It says nothing about the parent yet.
pub fn candidates<'a>(
    of_kind: impl Iterator<Item = &'a EntryStub>,
    entry: &EntryStub,
) -> Vec<&'a EntryStub> {
    if !applies(entry.kind) {
        return Vec::new();
    }
    of_kind
        .filter(|other| {
            other.kind == entry.kind
                && other.id != entry.id
                && same_normalized_title(&other.title, &entry.title)
        })
        .collect()
}

/// The parents of the page's entry and of its candidates, from the index that the kind keeps
/// them in. `None` when that index is missing, because two rules that share a name could then
/// not be told apart and the line stays off. A kind with no parent gives an empty map.
pub fn parents_of(
    entry: &EntryStub,
    found: &[&EntryStub],
    relations: &Relations,
    groups: Option<&OptionGroupIndex>,
    title_of: &dyn Fn(&EntryId) -> Option<String>,
) -> Option<ParentMap> {
    let stubs: Vec<EntryStub> = std::iter::once(entry)
        .chain(found.iter().copied())
        .cloned()
        .collect();
    match entry.kind {
        EntryKind::RacialTrait | EntryKind::Archetype => Some(row_parents::from_relations(
            relations, entry.kind, &stubs, title_of,
        )),
        EntryKind::ClassOption => groups.map(row_parents::from_option_groups),
        EntryKind::Affliction => Some(row_parents::from_stub_parents(&stubs, title_of)),
        _ => Some(ParentMap::new()),
    }
}

/// The printings to link from the page of `entry`, newest first. `found` are its
/// [`candidates`]; the rest of the rule is decided here. `joined` says whether a `revision`
/// edge joins two entries in either direction, and `describe` gives a printing its book, its
/// place in the order and whether it is active.
pub fn others(
    entry: &EntryStub,
    found: &[&EntryStub],
    parents: &ParentMap,
    joined: &dyn Fn(&EntryId, &EntryId) -> bool,
    describe: &dyn Fn(&EntryStub) -> Described,
) -> Vec<Printing> {
    if !applies(entry.kind) {
        return Vec::new();
    }
    let key = fold_key(entry.kind, parents.get(&entry.id), &entry.title);
    let mut kept: Vec<(SourceVariant, bool)> = found
        .iter()
        .copied()
        .filter(|other| {
            fold_key(other.kind, parents.get(&other.id), &other.title) == key
                && other.source != entry.source
                && !joined(&entry.id, &other.id)
        })
        .map(|other| {
            let described = describe(other);
            (
                SourceVariant {
                    id: other.id.clone(),
                    title: other.title.clone(),
                    source: described.book,
                    kind: other.kind,
                    parent: None,
                    order: described.order,
                },
                described.active,
            )
        })
        .collect();
    cast_iron_sort::sort_by(&mut kept, |a, b| printing_cmp(&a.0, &b.0));
    let variants: Vec<SourceVariant> = kept.iter().map(|(variant, _)| variant.clone()).collect();
    variant_labels(&variants, false)
        .into_iter()
        .zip(kept)
        .map(|(label, (variant, active))| Printing {
            id: variant.id,
            label,
            active,
        })
        .collect()
}

/// The whole line, for tests: `Other printings: Ultimate Equipment; Core Rulebook · Inactive`.
pub fn line_text(printings: &[Printing]) -> String {
    if printings.is_empty() {
        return String::new();
    }
    let links: Vec<String> = printings
        .iter()
        .map(|printing| {
            if printing.active {
                printing.label.clone()
            } else {
                format!("{} · Inactive", printing.label)
            }
        })
        .collect();
    format!("Other printings: {}", links.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row_parents::from_stub_parents;
    use cast_iron_schema::{RelationEdge, RelationIndex, RelationKind};
    use std::collections::HashMap;

    fn id(s: &str) -> EntryId {
        EntryId::parse(s.to_string()).unwrap()
    }

    fn stub(entry: &str, kind: EntryKind, title: &str, book: &str) -> EntryStub {
        EntryStub {
            id: id(entry),
            kind,
            title: title.into(),
            source: book.into(),
            books: vec![],
            parent: None,
            shard: "entries/x.json".into(),
        }
    }

    /// A catalog of books: each has a title, a place in the order and an active flag.
    fn describe(
        books: &'static [(&'static str, &'static str, u8, u32, bool)],
    ) -> impl Fn(&EntryStub) -> Described {
        let table: HashMap<&str, Described> = books
            .iter()
            .map(|(book, title, rank, newest, active)| {
                (
                    *book,
                    Described {
                        book: (*title).into(),
                        order: PrintingOrder {
                            rank: *rank,
                            newest: *newest,
                        },
                        active: *active,
                    },
                )
            })
            .collect();
        move |stub| table[stub.source.as_str()].clone()
    }

    const BOOKS: &[(&str, &str, u8, u32, bool)] = &[
        ("core", "Core Rulebook", 0, 1, true),
        ("apg", "Advanced Player's Guide", 0, 2, true),
        ("ue", "Ultimate Equipment", 0, 3, true),
        ("off", "Dusty Tome", 0, 0, false),
        ("adv", "Some Adventure", 3, 9, true),
    ];

    fn no_edges(_: &EntryId, _: &EntryId) -> bool {
        false
    }

    fn run(entry: &EntryStub, all: &[EntryStub]) -> Vec<Printing> {
        let found = candidates(all.iter(), entry);
        others(
            entry,
            &found,
            &ParentMap::new(),
            &no_edges,
            &describe(BOOKS),
        )
    }

    #[test]
    fn the_scan_ignores_case_and_spacing_and_skips_the_page_itself() {
        let tent = stub("ref/item/tent-core", EntryKind::Item, "Tent", "core");
        let all = [
            tent.clone(),
            stub("ref/item/tent-ue", EntryKind::Item, "  TENT ", "ue"),
            stub("ref/item/tentacle", EntryKind::Item, "Tentacle", "ue"),
            stub("ref/feat/tent", EntryKind::Feat, "Tent", "ue"),
        ];
        let found = candidates(all.iter(), &tent);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id("ref/item/tent-ue"));
    }

    #[test]
    fn the_newest_printing_comes_first_and_the_line_names_books_only() {
        let all = [
            stub("ref/item/tent-core", EntryKind::Item, "Tent", "core"),
            stub("ref/item/tent-ue", EntryKind::Item, "Tent", "ue"),
            stub("ref/item/tent-apg", EntryKind::Item, "Tent", "apg"),
        ];
        let from_core = run(&all[0], &all);
        assert_eq!(
            line_text(&from_core),
            "Other printings: Ultimate Equipment; Advanced Player's Guide"
        );
        // The newest printing lists the older ones, newest of those first.
        let from_ue = run(&all[1], &all);
        assert_eq!(
            line_text(&from_ue),
            "Other printings: Advanced Player's Guide; Core Rulebook"
        );
        assert!(!line_text(&from_ue).contains("Latest"));
    }

    #[test]
    fn product_line_beats_recency_as_in_search() {
        let all = [
            stub("ref/item/rope-core", EntryKind::Item, "Rope", "core"),
            stub("ref/item/rope-adv", EntryKind::Item, "Rope", "adv"),
            stub("ref/item/rope-apg", EntryKind::Item, "Rope", "apg"),
        ];
        let from_core = run(&all[0], &all);
        let books: Vec<&str> = from_core.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(books, ["Advanced Player's Guide", "Some Adventure"]);
    }

    #[test]
    fn a_creature_never_lists_a_same_name_creature() {
        let acolyte = stub(
            "ref/monster/gmg/acolyte",
            EntryKind::Creature,
            "Acolyte",
            "core",
        );
        let all = [
            acolyte.clone(),
            stub(
                "ref/monster/npc/acolyte",
                EntryKind::Creature,
                "Acolyte",
                "ue",
            ),
        ];
        assert!(candidates(all.iter(), &acolyte).is_empty());
        assert!(run(&acolyte, &all).is_empty());
        assert!(!applies(EntryKind::Creature));
        assert!(applies(EntryKind::Feat));
    }

    #[test]
    fn a_pair_joined_by_a_revision_edge_is_left_out_in_either_direction() {
        let base = stub("ref/feat/toughness", EntryKind::Feat, "Toughness", "core");
        let revision = stub(
            "ref/feat/toughness-elephant",
            EntryKind::Feat,
            "Toughness",
            "ue",
        );
        let other = stub(
            "ref/feat/toughness-apg",
            EntryKind::Feat,
            "Toughness",
            "apg",
        );
        let all = [base.clone(), revision.clone(), other.clone()];
        let relations = Relations::new(RelationIndex {
            edges: vec![RelationEdge(
                revision.id.clone(),
                RelationKind::Revision,
                base.id.clone(),
                None,
            )],
            ..Default::default()
        });
        let joined = |a: &EntryId, b: &EntryId| {
            relations
                .revision_base(a)
                .is_some_and(|(base, _)| base == b)
                || relations
                    .revision_base(b)
                    .is_some_and(|(base, _)| base == a)
        };
        let describe = describe(BOOKS);
        let parents = ParentMap::new();
        let from_base = others(
            &base,
            &candidates(all.iter(), &base),
            &parents,
            &joined,
            &describe,
        );
        assert_eq!(
            from_base.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["ref/feat/toughness-apg"]
        );
        let from_revision = others(
            &revision,
            &candidates(all.iter(), &revision),
            &parents,
            &joined,
            &describe,
        );
        assert_eq!(
            from_revision
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            ["ref/feat/toughness-apg"]
        );
    }

    #[test]
    fn a_same_book_duplicate_is_left_out() {
        let all = [
            stub("ref/item/a", EntryKind::Item, "Scabbard", "core"),
            stub("ref/item/b", EntryKind::Item, "Scabbard", "core"),
            stub("ref/item/c", EntryKind::Item, "Scabbard", "ue"),
        ];
        let from_a = run(&all[0], &all);
        assert_eq!(
            from_a.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["ref/item/c"]
        );
        // Two duplicates in the one other book read apart by a number.
        let from_c = run(&all[2], &all);
        assert_eq!(
            line_text(&from_c),
            "Other printings: Core Rulebook (1); Core Rulebook (2)"
        );
        assert!(run(&stub("ref/item/z", EntryKind::Item, "Alone", "core"), &all).is_empty());
    }

    #[test]
    fn a_printing_in_a_turned_off_book_stays_and_says_so() {
        let all = [
            stub("ref/item/a", EntryKind::Item, "Lamp", "core"),
            stub("ref/item/b", EntryKind::Item, "Lamp", "off"),
        ];
        let printings = run(&all[0], &all);
        assert_eq!(printings.len(), 1);
        assert!(!printings[0].active);
        assert_eq!(
            line_text(&printings),
            "Other printings: Dusty Tome · Inactive"
        );
    }

    #[test]
    fn the_parent_keeps_two_rules_that_share_a_name_apart() {
        let mut dwarf_core = stub(
            "ref/racial-trait/dwarf/hardy",
            EntryKind::RacialTrait,
            "Hardy",
            "core",
        );
        dwarf_core.parent = Some(id("ref/race/dwarf"));
        let mut dwarf_apg = stub(
            "ref/racial-trait/dwarf/hardy-2",
            EntryKind::RacialTrait,
            "Hardy",
            "apg",
        );
        dwarf_apg.parent = Some(id("ref/race/dwarf"));
        let mut orc_ue = stub(
            "ref/racial-trait/orc/hardy",
            EntryKind::RacialTrait,
            "Hardy",
            "ue",
        );
        orc_ue.parent = Some(id("ref/race/orc"));
        let all = [dwarf_core.clone(), dwarf_apg, orc_ue];
        let title_of =
            |target: &EntryId| Some(target.as_str().rsplit('/').next().unwrap().to_string());
        let found = candidates(all.iter(), &dwarf_core);
        assert_eq!(found.len(), 2, "the scan alone cannot tell the races apart");
        let stubs: Vec<EntryStub> = all.to_vec();
        let parents = from_stub_parents(&stubs, &title_of);
        let printings = others(&dwarf_core, &found, &parents, &no_edges, &describe(BOOKS));
        assert_eq!(
            printings.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["ref/racial-trait/dwarf/hardy-2"]
        );
    }

    #[test]
    fn a_class_option_with_no_group_index_gives_no_parents_and_no_line() {
        let option = stub(
            "ref/class-option/a",
            EntryKind::ClassOption,
            "Surprise Strike",
            "core",
        );
        let relations = Relations::default();
        assert!(parents_of(&option, &[], &relations, None, &|_| None).is_none());
        let feat = stub("ref/feat/a", EntryKind::Feat, "Dodge", "core");
        assert!(
            parents_of(&feat, &[], &relations, None, &|_| None)
                .unwrap()
                .is_empty()
        );
    }
}
