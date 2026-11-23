//! The list of stat sets on the entry page of a creature that prints several (#166).
//!
//! The compiler derives the sets from the body (`cast_iron_schema::stat_sets`). The page shows
//! them as one row of jump links above the body: `Small Air Elemental` with `CR 1 · Small`.
//! A link scrolls to the set's `##` heading. The rating text is plain, never colour alone.

use crate::route::{self, Route};
use cast_iron_schema::{EntryId, RuleEntry, StatSet, TypedFields};
use leptos::prelude::*;

/// The muted detail after a set's title: `CR 1/2 · Small`. Empty when the set has neither.
pub fn detail(set: &StatSet) -> String {
    let mut parts = Vec::new();
    if let Some(cr) = set.cr {
        parts.push(format!("CR {}", cr.display()));
    }
    if let Some(size) = set.size.as_deref().filter(|size| !size.is_empty()) {
        let mut chars = size.chars();
        let first = chars.next().map(|c| c.to_uppercase().collect::<String>());
        parts.push(first.into_iter().chain(chars.map(String::from)).collect());
    }
    parts.join(" · ")
}

/// The accessible name of a link: the title, a comma and the detail. Two flex items of the link
/// would read as `Small Air Elemental CR 1`, or with a hidden separator as `Small Air Elemental
/// , CR 1`, so the name is set whole. The visible text starts with the title.
pub fn link_name(set: &StatSet) -> String {
    match detail(set).as_str() {
        "" => set.title.clone(),
        detail => format!("{}, {detail}", set.title),
    }
}

/// Whether the page lists stat sets: a creature with two or more. The compiler derives them
/// for creatures only; the kind check keeps a stale or foreign list from showing on another
/// kind of entry.
pub fn listed(entry: &RuleEntry) -> bool {
    matches!(entry.fields, TypedFields::Creature(_)) && entry.stat_sets.len() > 1
}

/// Renders nothing for fewer than two sets: a single set is the entry itself.
#[component]
pub fn StatSetList(sets: Vec<StatSet>, id: EntryId) -> impl IntoView {
    if sets.len() < 2 {
        return ().into_any();
    }
    let current = route::use_route();
    let book = match current.get_untracked() {
        Route::Entry { book, .. } => book,
        _ => None,
    };
    let items = sets
        .into_iter()
        .map(|set| {
            let anchor = set.anchor.clone();
            let target = Route::Entry {
                id: id.clone(),
                anchor: Some(set.anchor.clone()),
                book: book.clone(),
                section: None,
                arch: Default::default(),
            };
            let href = target.href();
            let name = link_name(&set);
            let detail = detail(&set);
            view! {
                <li>
                    <a class="jump" href=href aria-label=name on:click=move |event: leptos::ev::MouseEvent| {
                        if event.button() == 0 && !event.ctrl_key() && !event.meta_key() && !event.shift_key() && !event.alt_key() && current.get_untracked() == target {
                            event.prevent_default();
                            route::scroll_to_anchor(&anchor);
                        }
                    }>
                        {set.title}
                        {(!detail.is_empty()).then(|| view! { <span class="jump-count">{detail}</span> })}
                    </a>
                </li>
            }
        })
        .collect_view();
    view! {
        <nav class="stat-sets" aria-labelledby="stat-sets-heading">
            <h2 id="stat-sets-heading" class="sub-head">"Stat sets"</h2>
            <ul class="jump-list">{items}</ul>
        </nav>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::ChallengeRating;

    fn set(cr: Option<&str>, size: Option<&str>) -> StatSet {
        StatSet {
            title: "Small Air Elemental".into(),
            anchor: "stat-small".into(),
            cr: cr.and_then(ChallengeRating::parse),
            size: size.map(str::to_string),
        }
    }

    #[test]
    fn the_link_name_adds_the_detail_after_a_comma() {
        assert_eq!(
            link_name(&set(Some("1"), Some("small"))),
            "Small Air Elemental, CR 1 · Small"
        );
        assert_eq!(link_name(&set(None, None)), "Small Air Elemental");
    }

    #[test]
    fn only_a_creature_with_two_sets_is_listed() {
        let mut entry = RuleEntry {
            id: EntryId::parse("ref/monster/x").unwrap(),
            kind: cast_iron_schema::EntryKind::Creature,
            title: "X".into(),
            aon: None,
            source_ids: vec![],
            sources: vec![],
            expanded_in: vec![],
            stat_sets: vec![],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::None,
            tags: vec![],
            race_group: None,
            links: vec![],
            provenance: vec![],
            license_refs: vec![],
            relations: vec![],
        };
        entry.stat_sets = vec![set(Some("1"), None), set(Some("5"), None)];
        entry.fields = TypedFields::None;
        assert!(!listed(&entry));
        entry.fields = TypedFields::Creature(Default::default());
        assert!(listed(&entry));
        entry.stat_sets.pop();
        assert!(!listed(&entry));
    }

    #[test]
    fn the_detail_names_the_rating_and_a_capitalised_size() {
        assert_eq!(detail(&set(Some("1/2"), Some("small"))), "CR 1/2 · Small");
        assert_eq!(detail(&set(Some("10"), None)), "CR 10");
        assert_eq!(detail(&set(None, Some("Huge"))), "Huge");
        assert_eq!(detail(&set(None, None)), "");
    }
}
