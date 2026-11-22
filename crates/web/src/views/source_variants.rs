//! Presentation grouping for records that are printings of one rule: the same kind, normalized
//! title and parent (race, class, option group, creature).
//!
//! Records remain independent: every variant keeps its permanent entry id and publication.
use crate::route::Route;
use crate::views::icons;
use cast_iron_schema::{EntryId, EntryKind, EntryStub};
use leptos::prelude::*;

/// What an entry belongs to: the race of a racial trait, the class of an archetype, the group of
/// a class option, the creature of an affliction. The key tells two parents apart; the title is
/// what a row prints. An entry with several parents has the keys and the titles joined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parent {
    pub key: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceVariant {
    pub id: EntryId,
    pub title: String,
    pub source: String,
    pub kind: EntryKind,
    pub parent: Option<Parent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceGroup {
    pub title: String,
    pub kind: EntryKind,
    pub variants: Vec<SourceVariant>,
}

impl SourceGroup {
    /// The parent the printings share. The fold key includes the parent, so every variant has
    /// the same one.
    pub fn parent(&self) -> Option<&Parent> {
        self.variants.first().and_then(|v| v.parent.as_ref())
    }
}

pub fn normalized_title(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Groups identical normalized titles only within the same kind and parent: a fold joins the
/// printings of one rule, never two rules that share a name. Input should already be filtered
/// to active publications; the IDs and source labels are never collapsed.
pub fn group_variants(entries: Vec<SourceVariant>) -> Vec<SourceGroup> {
    let mut groups: Vec<SourceGroup> = Vec::new();
    let mut positions = std::collections::BTreeMap::<(String, String, String), usize>::new();
    for entry in entries {
        let key = (
            entry.kind.slug().to_string(),
            normalized_title(&entry.title),
            entry
                .parent
                .as_ref()
                .map(|parent| parent.key.clone())
                .unwrap_or_default(),
        );
        if let Some(&position) = positions.get(&key) {
            groups[position].variants.push(entry);
        } else {
            positions.insert(key, groups.len());
            groups.push(SourceGroup {
                title: entry.title.clone(),
                kind: entry.kind,
                variants: vec![entry],
            });
        }
    }
    for group in &mut groups {
        cast_iron_sort::sort_by(&mut group.variants, |a, b| {
            a.source
                .to_lowercase()
                .cmp(&b.source.to_lowercase())
                .then(a.id.cmp(&b.id))
        });
    }
    groups
}

pub fn variants_for_stubs(
    stubs: &[EntryStub],
    source_title: impl Fn(&str) -> String,
) -> Vec<SourceGroup> {
    group_variants(
        stubs
            .iter()
            .map(|s| SourceVariant {
                id: s.id.clone(),
                title: s.title.clone(),
                source: source_title(&s.source),
                kind: s.kind,
                parent: None,
            })
            .collect(),
    )
}

/// The text of each link in a fold, in order. A link names its book: the list already names
/// the kind. If two links would still read the same they add the parent title, and a last `(n)`
/// keeps any two apart. `with_kind` keeps `, {Kind}` for a panel that is not a list of one kind.
pub fn variant_labels(variants: &[SourceVariant], with_kind: bool) -> Vec<String> {
    let mut labels: Vec<String> = variants
        .iter()
        .map(|v| {
            if with_kind {
                format!("{}, {}", v.source, v.kind.label())
            } else {
                v.source.clone()
            }
        })
        .collect();
    let clashes = |labels: &[String]| -> Vec<bool> {
        labels
            .iter()
            .map(|label| labels.iter().filter(|other| *other == label).count() > 1)
            .collect()
    };
    let clashing = clashes(&labels);
    for (label, (variant, clash)) in labels.iter_mut().zip(variants.iter().zip(clashing)) {
        if clash && let Some(parent) = &variant.parent {
            *label = format!("{label}, {}", parent.title);
        }
    }
    let clashing = clashes(&labels);
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for (label, clash) in labels.iter_mut().zip(clashing) {
        if clash {
            let n = seen.entry(label.clone()).or_default();
            *n += 1;
            *label = format!("{label} ({n})");
        }
    }
    labels
}

/// A fold of printings of one rule. `with_kind` keeps the kind in each link, for a panel that
/// is not a list of one kind.
#[component]
pub fn SourceVariants(
    group: SourceGroup,
    #[prop(optional)] with_kind: bool,
    /// The muted line beside the count: what names the rule when other folds share its title,
    /// such as the race. A list that its parent's menu narrows has none.
    #[prop(optional)]
    line: String,
) -> impl IntoView {
    let count = group.variants.len();
    let items = variant_labels(&group.variants, with_kind)
        .into_iter()
        .zip(&group.variants)
        .map(|(label, variant)| {
            let href = Route::entry(variant.id.clone()).href();
            view! { <li><a href=href>{label}</a></li> }
        })
        .collect_view();
    view! {
        <details class="source-variants">
            <summary>
                {icons::chevron_right("disclosure-chevron")}{group.title}
                <span class="source-variant-count">{format!(" {}", crate::fmt::pluralize(count, "publication entry", "publication entries"))}</span>
                {(!line.is_empty()).then(|| view! { <span class="muted row-source">{line}</span> })}
            </summary>
            <ul>{items}</ul>
        </details>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(s: &str) -> EntryId {
        EntryId::parse(s.to_string()).unwrap()
    }

    fn variant(entry: &str, title: &str, source: &str, kind: EntryKind) -> SourceVariant {
        SourceVariant {
            id: id(entry),
            title: title.into(),
            source: source.into(),
            kind,
            parent: None,
        }
    }

    fn of(parent: &str, mut v: SourceVariant) -> SourceVariant {
        v.parent = Some(Parent {
            key: format!("ref/race/{}", parent.to_lowercase()),
            title: parent.into(),
        });
        v
    }

    #[test]
    fn groups_same_title_and_kind_but_never_cross_kind() {
        let groups = group_variants(vec![
            variant(
                "trait/a",
                "Anatomist",
                "Advanced Player's Guide",
                EntryKind::Trait,
            ),
            variant(
                "trait/b",
                " anatomist ",
                "Ultimate Campaign",
                EntryKind::Trait,
            ),
            variant("feat/c", "Anatomist", "Other", EntryKind::Feat),
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(
            groups
                .iter()
                .find(|g| g.kind == EntryKind::Trait)
                .unwrap()
                .variants
                .len(),
            2
        );
    }

    #[test]
    fn the_fold_joins_printings_of_one_parent_and_keeps_other_parents_apart() {
        let kind = EntryKind::RacialTrait;
        let groups = group_variants(vec![
            of(
                "Dwarf",
                variant(
                    "racial-trait/dwarf/attributes",
                    "Attributes",
                    "Core Rulebook",
                    kind,
                ),
            ),
            of(
                "Elf",
                variant(
                    "racial-trait/elf/attributes",
                    "Attributes",
                    "Core Rulebook",
                    kind,
                ),
            ),
            of(
                "Dwarf",
                variant(
                    "racial-trait/dwarf/attributes-2",
                    "Attributes",
                    "Advanced Race Guide",
                    kind,
                ),
            ),
            variant("racial-trait/x/attributes", "Attributes", "Other", kind),
        ]);
        assert_eq!(groups.len(), 3, "dwarf, elf, and the entry with no parent");
        let dwarf = &groups[0];
        assert_eq!(dwarf.variants.len(), 2);
        assert_eq!(dwarf.parent().map(|p| p.title.as_str()), Some("Dwarf"));
        assert_eq!(groups[1].parent().map(|p| p.title.as_str()), Some("Elf"));
        assert!(groups[2].parent().is_none());
    }

    #[test]
    fn an_entry_with_no_parent_still_folds_by_title() {
        let kind = EntryKind::Creature;
        let groups = group_variants(vec![
            variant("monster/a/acolyte", "Acolyte", "Bestiary", kind),
            variant("monster/b/acolyte", "Acolyte", "NPC Codex", kind),
        ]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].variants.len(), 2);
    }

    #[test]
    fn a_link_names_its_book_alone_and_the_kind_only_when_asked() {
        let kind = EntryKind::RacialTrait;
        let both = [
            variant("a/x", "Hardy", "Advanced Race Guide", kind),
            variant("a/y", "Hardy", "Core Rulebook", kind),
        ];
        assert_eq!(
            variant_labels(&both, false),
            ["Advanced Race Guide", "Core Rulebook"]
        );
        assert_eq!(
            variant_labels(&both, true),
            [
                "Advanced Race Guide, Racial traits",
                "Core Rulebook, Racial traits"
            ]
        );
    }

    #[test]
    fn a_link_that_still_reads_the_same_adds_its_parent_and_then_a_number() {
        let kind = EntryKind::RacialTrait;
        let fold = [
            of("Dwarf", variant("a/x", "Hardy", "Core Rulebook", kind)),
            of("Elf", variant("a/y", "Hardy", "Core Rulebook", kind)),
            of("Elf", variant("a/z", "Hardy", "Advanced Race Guide", kind)),
        ];
        assert_eq!(
            variant_labels(&fold, false),
            [
                "Core Rulebook, Dwarf",
                "Core Rulebook, Elf",
                "Advanced Race Guide"
            ]
        );
        let same = [
            of("Dwarf", variant("a/x", "Hardy", "Core Rulebook", kind)),
            of("Dwarf", variant("a/y", "Hardy", "Core Rulebook", kind)),
        ];
        assert_eq!(
            variant_labels(&same, false),
            ["Core Rulebook, Dwarf (1)", "Core Rulebook, Dwarf (2)"]
        );
        let bare = [
            variant("a/x", "Hardy", "Core Rulebook", EntryKind::Creature),
            variant("a/y", "Hardy", "Core Rulebook", EntryKind::Creature),
        ];
        assert_eq!(
            variant_labels(&bare, false),
            ["Core Rulebook (1)", "Core Rulebook (2)"]
        );
    }
}
