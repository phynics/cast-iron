//! Which parent a list row names: the race of a racial trait, the class of an archetype, the
//! group of a class option, the creature of an affliction.
//!
//! The lookups are pure over typed content, so the rules are tested without a browser. The list
//! page loads the maps it needs (`relations.json`, `option-groups.json`) and the rows read them
//! once they have arrived.

use crate::graph::Relations;
use crate::views::source_variants::Parent;
use cast_iron_schema::{EntryId, EntryKind, EntryStub, OptionGroupIndex, RelationKind};
use std::collections::HashMap;

/// The parent of each entry that has one. The map holds the position of the parent in a list, so
/// it is the same `HashMap<EntryId, usize>` the list page already uses, not a new one.
#[derive(Debug, Default)]
pub struct ParentMap {
    index: HashMap<EntryId, usize>,
    parents: Vec<Parent>,
}

impl ParentMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &EntryId) -> Option<&Parent> {
        self.index.get(id).map(|&position| &self.parents[position])
    }

    pub fn is_empty(&self) -> bool {
        self.parents.is_empty()
    }

    /// Records the parent of `id`; the first one recorded for an entry stays.
    fn add(&mut self, id: &EntryId, parent: Parent) {
        if !self.index.contains_key(id) {
            self.index.insert(id.clone(), self.parents.len());
            self.parents.push(parent);
        }
    }
}

/// The search facet whose menu narrows a list of `kind` by its parent. A list narrowed by it
/// names the parent once, in the menu, so its rows leave the parent out.
pub fn parent_facet(kind: EntryKind) -> Option<&'static str> {
    match kind {
        EntryKind::ClassOption => Some("group"),
        EntryKind::RacialTrait => Some("racial_trait_race"),
        EntryKind::Archetype => Some("archetype_class"),
        _ => None,
    }
}

/// True when the rows of `kind` name a parent first on their muted line.
pub fn names_parent(kind: EntryKind) -> bool {
    parent_facet(kind).is_some() || kind == EntryKind::Affliction
}

/// True for an alternate racial trait. Its muted line starts with `Alternate`, so a standard
/// and an alternate trait of one race read apart; the id says which it is, with no lookup.
pub fn is_alternate(stub: &EntryStub) -> bool {
    stub.kind == EntryKind::RacialTrait && crate::graph::is_alternate_racial_trait(&stub.id)
}

/// The muted line of a row: `{Alternate} · {Parent} · {Book}`, each part only where it applies.
/// A list that does not span books has no book, an entry with no parent (or a list that its
/// parent's menu narrows) has none, and only an alternate racial trait has `Alternate`.
pub fn line(alternate: bool, parent: Option<&Parent>, book: Option<&str>) -> Option<String> {
    let parts: Vec<&str> = alternate
        .then_some("Alternate")
        .into_iter()
        .chain(parent.map(|parent| parent.title.as_str()))
        .chain(book)
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// A parent made of the targets of an entry, which are named by `title_of`. A target the catalog
/// does not know is left out. Several targets join into one parent.
fn parent_of_targets(
    targets: &[&EntryId],
    title_of: &dyn Fn(&EntryId) -> Option<String>,
) -> Option<Parent> {
    let mut named: Vec<(String, &EntryId)> = targets
        .iter()
        .filter_map(|target| Some((title_of(target)?, *target)))
        .collect();
    cast_iron_sort::sort_by(&mut named, |a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)));
    if named.is_empty() {
        return None;
    }
    let ids: Vec<&str> = named.iter().map(|(_, id)| id.as_str()).collect();
    let titles: Vec<&str> = named.iter().map(|(title, _)| title.as_str()).collect();
    Some(Parent {
        key: ids.join(","),
        title: titles.join(", "),
    })
}

/// The race of each racial trait (its `part` edge) and the class of each archetype (its
/// `archetype` edge), for the entries of `kind` in `stubs`. Any other kind has none.
pub fn from_relations(
    relations: &Relations,
    kind: EntryKind,
    stubs: &[EntryStub],
    title_of: &dyn Fn(&EntryId) -> Option<String>,
) -> ParentMap {
    let relation = match kind {
        EntryKind::RacialTrait => RelationKind::Part,
        EntryKind::Archetype => RelationKind::Archetype,
        _ => return ParentMap::new(),
    };
    let mut parents = ParentMap::new();
    for stub in stubs {
        let targets = relations.targets_of(&stub.id, relation);
        if let Some(parent) = parent_of_targets(&targets, title_of) {
            parents.add(&stub.id, parent);
        }
    }
    parents
}

/// The group of each class option, named by its singular label (`Rage power`).
pub fn from_option_groups(index: &OptionGroupIndex) -> ParentMap {
    let mut parents = ParentMap::new();
    for group in &index.groups {
        for id in &group.entries {
            parents.add(
                id,
                Parent {
                    key: group.slug.clone(),
                    title: group.singular.clone(),
                },
            );
        }
    }
    parents
}

/// The parent that the catalog stores on the stub itself: the creature, template or rule that an
/// affliction belongs to.
pub fn from_stub_parents(
    stubs: &[EntryStub],
    title_of: &dyn Fn(&EntryId) -> Option<String>,
) -> ParentMap {
    let mut parents = ParentMap::new();
    for stub in stubs {
        let targets: Vec<&EntryId> = stub.parent.iter().collect();
        if let Some(parent) = parent_of_targets(&targets, title_of) {
            parents.add(&stub.id, parent);
        }
    }
    parents
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{OptionGroup, RelationEdge, RelationIndex};

    fn id(s: &str) -> EntryId {
        EntryId::parse(s.to_string()).unwrap()
    }

    fn stub(s: &str, kind: EntryKind, parent: Option<&str>) -> EntryStub {
        EntryStub {
            id: id(s),
            kind,
            title: s.rsplit('/').next().unwrap().to_string(),
            source: "core-rulebook".into(),
            books: vec![],
            parent: parent.map(id),
            shard: "entries/x.json".into(),
        }
    }

    fn titles(target: &EntryId) -> Option<String> {
        match target.as_str() {
            "ref/race/dwarf" => Some("Dwarf".into()),
            "ref/race/gnome" => Some("Gnome".into()),
            "ref/class/rogue" => Some("Rogue".into()),
            "ref/monster/giant-ant" => Some("Giant Ant".into()),
            _ => None,
        }
    }

    #[test]
    fn only_these_kinds_name_a_parent_and_three_menus_narrow_by_it() {
        assert_eq!(
            parent_facet(EntryKind::RacialTrait),
            Some("racial_trait_race")
        );
        assert_eq!(parent_facet(EntryKind::Archetype), Some("archetype_class"));
        assert_eq!(parent_facet(EntryKind::ClassOption), Some("group"));
        assert_eq!(parent_facet(EntryKind::Affliction), None);
        assert!(names_parent(EntryKind::Affliction));
        assert!(!names_parent(EntryKind::Spell));
        assert!(!names_parent(EntryKind::Creature));
    }

    #[test]
    fn the_line_puts_the_parent_before_the_book_and_drops_what_is_absent() {
        let dwarf = Parent {
            key: "ref/race/dwarf".into(),
            title: "Dwarf".into(),
        };
        assert_eq!(
            line(false, Some(&dwarf), Some("Advanced Race Guide")).as_deref(),
            Some("Dwarf · Advanced Race Guide")
        );
        assert_eq!(line(false, Some(&dwarf), None).as_deref(), Some("Dwarf"));
        assert_eq!(
            line(false, None, Some("Core Rulebook")).as_deref(),
            Some("Core Rulebook")
        );
        assert_eq!(line(false, None, None), None);
    }

    #[test]
    fn an_alternate_trait_starts_its_line_with_alternate_whatever_else_it_names() {
        let dwarf = Parent {
            key: "ref/race/dwarf".into(),
            title: "Dwarf".into(),
        };
        assert_eq!(
            line(true, Some(&dwarf), Some("Advanced Race Guide")).as_deref(),
            Some("Alternate · Dwarf · Advanced Race Guide")
        );
        // The Race menu names the race, so a narrowed list keeps only the marker and the book.
        assert_eq!(
            line(true, None, Some("Advanced Race Guide")).as_deref(),
            Some("Alternate · Advanced Race Guide")
        );
        assert_eq!(line(true, None, None).as_deref(), Some("Alternate"));
    }

    #[test]
    fn only_a_racial_trait_with_an_alternate_id_is_alternate() {
        let alternate = stub(
            "ref/racial-trait/dwarf/alternate-racial-traits/stubborn",
            EntryKind::RacialTrait,
            None,
        );
        let standard = stub(
            "ref/racial-trait/dwarf/attributes",
            EntryKind::RacialTrait,
            None,
        );
        assert!(is_alternate(&alternate));
        assert!(!is_alternate(&standard));
        let mut other = alternate.clone();
        other.kind = EntryKind::Archetype;
        assert!(!is_alternate(&other));
    }

    #[test]
    fn a_racial_trait_names_its_race_and_an_archetype_its_class() {
        let relations = Relations::new(RelationIndex {
            edges: vec![
                RelationEdge(
                    id("ref/racial-trait/dwarf/attributes"),
                    RelationKind::Part,
                    id("ref/race/dwarf"),
                    None,
                ),
                RelationEdge(
                    id("ref/arc-rogue/acrobat"),
                    RelationKind::Archetype,
                    id("ref/class/rogue"),
                    None,
                ),
            ],
            ..Default::default()
        });
        let traits = [
            stub(
                "ref/racial-trait/dwarf/attributes",
                EntryKind::RacialTrait,
                None,
            ),
            stub("ref/racial-trait/orc/loose", EntryKind::RacialTrait, None),
        ];
        let map = from_relations(&relations, EntryKind::RacialTrait, &traits, &titles);
        assert_eq!(map.get(&traits[0].id).unwrap().title, "Dwarf");
        assert_eq!(map.get(&traits[0].id).unwrap().key, "ref/race/dwarf");
        assert!(map.get(&traits[1].id).is_none(), "no edge, no parent");
        let archetypes = [stub("ref/arc-rogue/acrobat", EntryKind::Archetype, None)];
        let map = from_relations(&relations, EntryKind::Archetype, &archetypes, &titles);
        assert_eq!(map.get(&archetypes[0].id).unwrap().title, "Rogue");
        assert!(from_relations(&relations, EntryKind::Spell, &archetypes, &titles).is_empty());
    }

    #[test]
    fn a_trait_of_several_races_names_them_all_in_title_order() {
        let relations = Relations::new(RelationIndex {
            edges: vec![
                RelationEdge(
                    id("ref/racial-trait/x/shared"),
                    RelationKind::Part,
                    id("ref/race/gnome"),
                    None,
                ),
                RelationEdge(
                    id("ref/racial-trait/x/shared"),
                    RelationKind::Part,
                    id("ref/race/dwarf"),
                    None,
                ),
            ],
            ..Default::default()
        });
        let traits = [stub(
            "ref/racial-trait/x/shared",
            EntryKind::RacialTrait,
            None,
        )];
        let map = from_relations(&relations, EntryKind::RacialTrait, &traits, &titles);
        let parent = map.get(&traits[0].id).unwrap();
        assert_eq!(parent.title, "Dwarf, Gnome");
        assert_eq!(parent.key, "ref/race/dwarf,ref/race/gnome");
    }

    #[test]
    fn a_class_option_names_its_group_by_the_singular_label() {
        let index = OptionGroupIndex {
            groups: vec![OptionGroup {
                slug: "rage-power".into(),
                label: "Rage powers".into(),
                singular: "Rage power".into(),
                classes: vec![],
                entries: vec![id("ref/ragepower/lesser-spirit")],
            }],
            ..Default::default()
        };
        let map = from_option_groups(&index);
        let parent = map.get(&id("ref/ragepower/lesser-spirit")).unwrap();
        assert_eq!(
            (parent.key.as_str(), parent.title.as_str()),
            ("rage-power", "Rage power")
        );
        assert!(map.get(&id("ref/ragepower/other")).is_none());
    }

    #[test]
    fn an_affliction_names_the_parent_its_stub_stores() {
        let poison = stub(
            "ref/affliction/giant-ant/poison-ex",
            EntryKind::Affliction,
            Some("ref/monster/giant-ant"),
        );
        let loose = stub("ref/affliction/x/poison", EntryKind::Affliction, None);
        let map = from_stub_parents(&[poison.clone(), loose.clone()], &titles);
        assert_eq!(map.get(&poison.id).unwrap().title, "Giant Ant");
        assert!(map.get(&loose.id).is_none());
    }
}
