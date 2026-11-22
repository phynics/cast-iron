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

/// The text of each link in a fold, in order. A link names its book: the page and the list
/// already name the kind. Two printings in one book add `p. {page}` (the first citation row of
/// the entry, once `pages` has it); if two links would still read the same they add the parent
/// title; once the pages are read (`Some`), a last `(n)` keeps any two links apart. `with_kind`
/// keeps `, {Kind}` for a panel that is not a list of one kind.
pub fn variant_labels(
    variants: &[SourceVariant],
    pages: Option<&std::collections::HashMap<EntryId, String>>,
    with_kind: bool,
) -> Vec<String> {
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
    let repeated =
        |labels: &[String], i: usize| labels.iter().filter(|l| **l == labels[i]).count() > 1;
    let clashing: Vec<bool> = (0..labels.len()).map(|i| repeated(&labels, i)).collect();
    for (i, variant) in variants.iter().enumerate() {
        if clashing[i]
            && let Some(page) = pages.and_then(|pages| pages.get(&variant.id))
        {
            labels[i] = format!("{}, p. {page}", labels[i]);
        }
    }
    let clashing: Vec<bool> = (0..labels.len()).map(|i| repeated(&labels, i)).collect();
    for (i, variant) in variants.iter().enumerate() {
        if clashing[i]
            && let Some(parent) = &variant.parent
        {
            labels[i] = format!("{}, {}", labels[i], parent.title);
        }
    }
    let mut seen = std::collections::HashMap::<String, usize>::new();
    let clashing: Vec<bool> = (0..labels.len()).map(|i| repeated(&labels, i)).collect();
    for (i, label) in labels.iter_mut().enumerate() {
        if pages.is_some() && clashing[i] {
            let n = seen.entry(label.clone()).or_default();
            *n += 1;
            *label = format!("{label} ({n})");
        }
    }
    labels
}

/// A fold of printings of one rule. `with_kind` keeps the kind in each link, for a panel that
/// is not a list of one kind. When two printings share a book, the first open reads their
/// citation rows so each link can say its page.
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
    let shared_book = group.variants.iter().any(|a| {
        group
            .variants
            .iter()
            .filter(|b| b.source == a.source)
            .count()
            > 1
    });
    let pages = RwSignal::new(None::<std::collections::HashMap<EntryId, String>>);
    let opened = RwSignal::new(false);
    if shared_book {
        let library = crate::library::use_library();
        let ids: Vec<EntryId> = group.variants.iter().map(|v| v.id.clone()).collect();
        Effect::new(move |_| {
            if !opened.get() {
                return;
            }
            let (library, ids) = (library.clone(), ids.clone());
            leptos::task::spawn_local(async move {
                let mut found = std::collections::HashMap::new();
                for id in ids {
                    if let Ok(entry) = library.entry(&id).await
                        && let Some(page) = entry.sources.first().and_then(|c| c.page.clone())
                    {
                        found.insert(id, page);
                    }
                }
                pages.set(Some(found));
            });
        });
    }
    let on_toggle = move |event: leptos::ev::Event| {
        if !opened.get_untracked() && event_target::<web_sys::HtmlDetailsElement>(&event).open() {
            opened.set(true);
        }
    };
    let variants = group.variants;
    let items = {
        let variants = variants.clone();
        move || {
            let labels = pages.with(|pages| variant_labels(&variants, pages.as_ref(), with_kind));
            variants
                .iter()
                .zip(labels)
                .map(|(variant, label)| {
                    let href = Route::entry(variant.id.clone()).href();
                    view! { <li><a href=href>{label}</a></li> }
                })
                .collect_view()
        }
    };
    view! {
        <details class="source-variants" on:toggle=on_toggle>
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
    use std::collections::HashMap;

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

    fn labels(variants: &[SourceVariant], pages: Option<&[(&str, &str)]>) -> Vec<String> {
        let pages: Option<HashMap<EntryId, String>> = pages.map(|rows| {
            rows.iter()
                .map(|(entry, page)| (id(entry), page.to_string()))
                .collect()
        });
        variant_labels(variants, pages.as_ref(), false)
    }

    #[test]
    fn a_link_names_its_book_alone_and_the_kind_only_when_asked() {
        let kind = EntryKind::RacialTrait;
        let both = [
            variant("a/x", "Hardy", "Advanced Race Guide", kind),
            variant("a/y", "Hardy", "Core Rulebook", kind),
        ];
        assert_eq!(
            labels(&both, None),
            ["Advanced Race Guide", "Core Rulebook"]
        );
        assert_eq!(
            variant_labels(&both, None, true),
            [
                "Advanced Race Guide, Racial traits",
                "Core Rulebook, Racial traits"
            ]
        );
    }

    #[test]
    fn two_printings_in_one_book_add_their_page_and_only_those() {
        let kind = EntryKind::ClassOption;
        let fold = [
            variant("a/x", "Quick Study", "Advanced Class Guide", kind),
            variant("a/y", "Quick Study", "Advanced Class Guide", kind),
            variant("a/z", "Quick Study", "Core Rulebook", kind),
        ];
        assert_eq!(
            labels(&fold, Some(&[("a/x", "12"), ("a/y", "40"), ("a/z", "9")])),
            [
                "Advanced Class Guide, p. 12",
                "Advanced Class Guide, p. 40",
                "Core Rulebook"
            ]
        );
    }

    #[test]
    fn a_link_that_still_reads_the_same_adds_its_parent_and_then_a_number() {
        let kind = EntryKind::RacialTrait;
        let fold = [
            of("Dwarf", variant("a/x", "Hardy", "Core Rulebook", kind)),
            of("Elf", variant("a/y", "Hardy", "Core Rulebook", kind)),
        ];
        assert_eq!(
            labels(&fold, Some(&[])),
            ["Core Rulebook, Dwarf", "Core Rulebook, Elf"]
        );
        let same = [
            of("Dwarf", variant("a/x", "Hardy", "Core Rulebook", kind)),
            of("Dwarf", variant("a/y", "Hardy", "Core Rulebook", kind)),
        ];
        // The page is read but neither entry has one: the number keeps the links apart.
        assert_eq!(
            labels(&same, Some(&[])),
            ["Core Rulebook, Dwarf (1)", "Core Rulebook, Dwarf (2)"]
        );
        // Until the pages are read the clash stays unnumbered, so the label does not flicker.
        assert_eq!(
            labels(&same, None),
            ["Core Rulebook, Dwarf", "Core Rulebook, Dwarf"]
        );
    }
}
