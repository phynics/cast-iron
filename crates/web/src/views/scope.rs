//! The scope of a search: the kind, and optionally the facet values, that a list page hands to
//! Quick search and that the results page shows again.
//!
//! One scope type serves both places. Quick search derives it from the page that was open when
//! the layer opened; the results page derives it from the route it displays. Either place shows
//! it as a removable pill ([`ScopePill`]), and removing it searches everything. No browser calls
//! here, so the rules are tested natively.

use crate::route::Route;
use crate::views::search::facet_label;
use cast_iron_schema::EntryKind;
use cast_iron_search::Filters;
use leptos::prelude::*;

/// What a search is limited to. Always one kind; a list page that filters by facet adds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub kind: EntryKind,
    pub facets: Vec<(String, String)>,
}

/// `combat` becomes `Combat`.
fn upper_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

impl Scope {
    /// The scope that Quick search takes from the page that is open: a kind's list page limits
    /// the search to that kind. Every other page has none. Rules are read through Books, so they
    /// have no list page to scope from.
    pub fn of_page(route: &Route) -> Option<Self> {
        match route {
            Route::BrowseKind { kind, .. } if *kind != EntryKind::Rule => Some(Self {
                kind: *kind,
                facets: Vec::new(),
            }),
            _ => None,
        }
    }

    /// The scope that a results page shows: a search limited to exactly one kind. Several kinds
    /// are a choice the Kind filter already shows.
    pub fn of_search(kinds: &[EntryKind], facets: &[(String, String)]) -> Option<Self> {
        match kinds {
            [kind] => Some(Self {
                kind: *kind,
                facets: facets.to_vec(),
            }),
            _ => None,
        }
    }

    /// `feats`, or `Combat feats` when a facet narrows it. The facet comes first and keeps its
    /// capital, so the phrase reads after "Search".
    fn phrase(&self) -> String {
        let noun = self.kind.label().to_lowercase();
        let facets: Vec<String> = self
            .facets
            .iter()
            .map(|(field, value)| upper_first(&facet_label(field, value)))
            .collect();
        if facets.is_empty() {
            noun
        } else {
            format!("{} {noun}", facets.join(" "))
        }
    }

    /// The field's placeholder and accessible name: `Search feats`, `Search Combat feats`.
    pub fn placeholder(&self) -> String {
        format!("Search {}", self.phrase())
    }

    /// The text of the pill: `Feats`, `Combat feats`.
    pub fn label(&self) -> String {
        upper_first(&self.phrase())
    }

    /// The accessible name of the pill's remove control. It contains the visible text.
    pub fn remove_label(&self) -> String {
        format!("Remove scope: {}", self.label())
    }

    /// The engine filters that carry the scope.
    pub fn filters(&self) -> Filters {
        Filters {
            kinds: vec![self.kind.slug().to_string()],
            sources: Vec::new(),
            facets: self.facets.clone(),
        }
    }

    /// The results page for `q` with this scope applied.
    pub fn route(&self, q: impl Into<String>) -> Route {
        Route::Search {
            q: q.into(),
            kinds: vec![self.kind],
            sources: Vec::new(),
            facets: self.facets.clone(),
        }
    }
}

/// The scope as a Filter-role pill with a 44 px remove target. The whole pill is the button, so
/// the visible text and the cross are one target and one accessible name.
#[component]
pub fn ScopePill(scope: Scope, on_remove: Callback<()>) -> impl IntoView {
    let label = scope.label();
    let name = scope.remove_label();
    view! {
        <button class="scope-pill" type="button" aria-label=name on:click=move |_| on_remove.run(())>
            <span>{label}</span>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
                stroke-linecap="round" aria-hidden="true">
                <path d="m7 7 10 10M17 7 7 17"/>
            </svg>
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feats() -> Scope {
        Scope {
            kind: EntryKind::Feat,
            facets: Vec::new(),
        }
    }

    fn combat_feats() -> Scope {
        Scope {
            kind: EntryKind::Feat,
            facets: vec![("feat_type".into(), "combat".into())],
        }
    }

    #[test]
    fn a_kind_page_scopes_the_search_to_its_kind_and_no_other_page_does() {
        assert_eq!(
            Scope::of_page(&Route::browse_kind(EntryKind::Feat)),
            Some(feats())
        );
        // A letter group is still the same list.
        assert_eq!(
            Scope::of_page(&Route::browse_kind_letter(EntryKind::Feat, 'B')),
            Some(feats())
        );
        for route in [
            Route::Home,
            Route::Books,
            Route::Spells,
            Route::FeatChains,
            Route::Settings,
            Route::search("fire"),
            Route::browse_kind(EntryKind::Rule),
        ] {
            assert_eq!(Scope::of_page(&route), None, "{route:?}");
        }
    }

    #[test]
    fn the_field_names_the_scope() {
        assert_eq!(feats().placeholder(), "Search feats");
        assert_eq!(combat_feats().placeholder(), "Search Combat feats");
        let prestige = Scope::of_page(&Route::browse_kind(EntryKind::PrestigeClass)).unwrap();
        assert_eq!(prestige.placeholder(), "Search prestige classes");
    }

    #[test]
    fn the_pill_names_the_scope_and_its_remove_control_repeats_that_name() {
        assert_eq!(feats().label(), "Feats");
        assert_eq!(combat_feats().label(), "Combat feats");
        assert_eq!(feats().remove_label(), "Remove scope: Feats");
        assert_eq!(combat_feats().remove_label(), "Remove scope: Combat feats");
        // The visible text is part of the accessible name (WCAG 2.5.3).
        assert!(
            combat_feats()
                .remove_label()
                .contains(&combat_feats().label())
        );
    }

    #[test]
    fn the_scope_becomes_engine_filters_and_a_results_route() {
        let filters = combat_feats().filters();
        assert_eq!(filters.kinds, ["feat"]);
        assert_eq!(filters.facets, [("feat_type".into(), "combat".into())]);
        assert!(filters.sources.is_empty());
        assert_eq!(
            feats().route("grapple"),
            Route::Search {
                q: "grapple".into(),
                kinds: vec![EntryKind::Feat],
                sources: Vec::new(),
                facets: Vec::new(),
            }
        );
        assert_eq!(
            combat_feats().route("grapple").href(),
            "#/search?q=grapple&kind=feat&facet=feat_type:combat"
        );
    }

    #[test]
    fn a_results_page_shows_a_scope_only_for_exactly_one_kind() {
        assert_eq!(Scope::of_search(&[EntryKind::Feat], &[]), Some(feats()));
        assert_eq!(
            Scope::of_search(&[EntryKind::Feat], &[("feat_type".into(), "combat".into())]),
            Some(combat_feats())
        );
        assert_eq!(Scope::of_search(&[], &[]), None);
        assert_eq!(
            Scope::of_search(&[EntryKind::Feat, EntryKind::Spell], &[]),
            None
        );
    }

    #[test]
    fn the_route_a_scope_makes_gives_the_same_scope_back() {
        let route = combat_feats().route("x");
        let Route::Search { kinds, facets, .. } = Route::parse(&route.href()) else {
            panic!("not a search route");
        };
        assert_eq!(Scope::of_search(&kinds, &facets), Some(combat_feats()));
    }
}
