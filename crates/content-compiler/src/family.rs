//! Upstream record families: which directory and `type` value become which entry kind.
//!
//! The directory name is the upstream `type` value for most families. The exceptions are the
//! reason this table exists: `class_archetype` becomes [`EntryKind::Archetype`], and the
//! upstream `drawback` family becomes a trait with the category `Drawback`.

use cast_iron_schema::EntryKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub kind: EntryKind,
    /// The `type` value that every record of the directory must carry.
    pub upstream_type: &'static str,
}

/// The family of a typed directory name such as `class_archetype`.
pub fn for_dir(dir: &str) -> Option<Family> {
    let (kind, upstream_type) = match dir {
        "spell" => (EntryKind::Spell, "spell"),
        "feat" => (EntryKind::Feat, "feat"),
        "creature" => (EntryKind::Creature, "creature"),
        "item" => (EntryKind::Item, "item"),
        "class" => (EntryKind::Class, "class"),
        "race" => (EntryKind::Race, "race"),
        "skill" => (EntryKind::Skill, "skill"),
        "affliction" => (EntryKind::Affliction, "affliction"),
        "trap" => (EntryKind::Trap, "trap"),
        "trait" => (EntryKind::Trait, "trait"),
        "drawback" => (EntryKind::Trait, "drawback"),
        "racial_trait" => (EntryKind::RacialTrait, "racial_trait"),
        "class_archetype" => (EntryKind::Archetype, "class_archetype"),
        _ => return None,
    };
    Some(Family {
        kind,
        upstream_type,
    })
}

/// A typed record that sits only inside a rules tree is promoted to its own entry when its
/// `type` names one of these older families. The newer families always come from their own
/// directories, so a rules tree cannot add entries of those kinds by accident.
pub fn promotable_from_tree(upstream_type: &str) -> Option<EntryKind> {
    match upstream_type {
        "spell" | "feat" | "creature" | "item" | "class" | "race" | "skill" | "affliction"
        | "trap" => for_dir(upstream_type).map(|f| f.kind),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_names_map_to_kinds() {
        assert_eq!(
            for_dir("class_archetype").unwrap().kind,
            EntryKind::Archetype
        );
        let drawback = for_dir("drawback").unwrap();
        assert_eq!(
            (drawback.kind, drawback.upstream_type),
            (EntryKind::Trait, "drawback")
        );
        assert!(for_dir("vehicle").is_none());
    }

    #[test]
    fn trees_promote_only_the_older_families() {
        assert_eq!(promotable_from_tree("class"), Some(EntryKind::Class));
        assert_eq!(promotable_from_tree("trait"), None);
        assert_eq!(promotable_from_tree("racial_trait"), None);
        assert_eq!(promotable_from_tree("class_archetype"), None);
        assert_eq!(promotable_from_tree("section"), None);
    }
}
