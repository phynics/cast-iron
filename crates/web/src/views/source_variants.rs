//! Presentation grouping for records with the same normalized title and kind.
//!
//! Records remain independent: every variant keeps its permanent entry id and publication.
use crate::route::Route;
use cast_iron_schema::{EntryId, EntryKind, EntryStub};
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceVariant {
    pub id: EntryId,
    pub title: String,
    pub source: String,
    pub kind: EntryKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceGroup {
    pub title: String,
    pub kind: EntryKind,
    pub variants: Vec<SourceVariant>,
}

pub fn normalized_title(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Groups identical normalized titles only within the same kind. Input should already be
/// filtered to active publications; the IDs and source labels are never collapsed.
pub fn group_variants(entries: Vec<SourceVariant>) -> Vec<SourceGroup> {
    let mut groups: Vec<SourceGroup> = Vec::new();
    let mut positions = std::collections::BTreeMap::<(String, String), usize>::new();
    for entry in entries {
        let key = (
            entry.kind.slug().to_string(),
            normalized_title(&entry.title),
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
        group.variants.sort_by(|a, b| {
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
            })
            .collect(),
    )
}

#[component]
pub fn SourceVariants(group: SourceGroup) -> impl IntoView {
    let count = group.variants.len();
    let items = group
        .variants
        .into_iter()
        .map(|variant| {
            let href = Route::entry(variant.id).href();
            let label = format!("{}, {}", variant.source, variant.kind.label());
            view! { <li><a href=href>{label}</a></li> }
        })
        .collect_view();
    view! {
        <details class="source-variants">
            <summary>{group.title}<span class="source-variant-count">{format!(" {count} publication entries")}</span></summary>
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

    #[test]
    fn groups_same_title_and_kind_but_never_cross_kind() {
        let groups = group_variants(vec![
            SourceVariant {
                id: id("trait/a"),
                title: "Anatomist".into(),
                source: "Advanced Player's Guide".into(),
                kind: EntryKind::Trait,
            },
            SourceVariant {
                id: id("trait/b"),
                title: " anatomist ".into(),
                source: "Ultimate Campaign".into(),
                kind: EntryKind::Trait,
            },
            SourceVariant {
                id: id("feat/c"),
                title: "Anatomist".into(),
                source: "Other".into(),
                kind: EntryKind::Feat,
            },
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
}
