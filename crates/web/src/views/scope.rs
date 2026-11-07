//! The scope of a search: the kind, and optionally the facet values, that a list page hands to
//! Quick search and that the results page shows again.
//!
//! One scope type serves both places. Quick search derives it from the page that was open when
//! the layer opened; the results page derives it from the route it displays. Either place shows
//! it as a removable pill ([`ScopePill`]), and removing it searches everything. No browser calls
//! here, so the rules are tested natively.

use crate::route::Route;
use crate::views::categories::applied_facets;
use crate::views::search::facet_label;
use cast_iron_schema::EntryKind;
use cast_iron_search::Filters;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// What a search is limited to. Always one kind; a list page that filters by facet adds them,
/// and a book-filtered list adds the publication. `source_title` is the book's display name; it
/// is stored with a recent search so the pill still names the book after a reload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub kind: EntryKind,
    pub facets: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_title: Option<String>,
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
    /// the search to that kind, to the facet value that narrows the list, and to the book that a
    /// book page chose. Every other page has none. Rules are read through Books, so they have no
    /// list page to scope from.
    pub fn of_page(route: &Route) -> Option<Self> {
        match route {
            Route::BrowseKind {
                kind,
                facets,
                source,
                ..
            } if *kind != EntryKind::Rule => Some(Self {
                kind: *kind,
                facets: applied_facets(*kind, facets),
                source: source.clone(),
                source_title: None,
            }),
            _ => None,
        }
    }

    /// The scope that a results page shows: a search limited to exactly one kind and, at most,
    /// one publication. Several kinds are a choice the Kind filter already shows.
    pub fn of_search(
        kinds: &[EntryKind],
        facets: &[(String, String)],
        sources: &[String],
    ) -> Option<Self> {
        match (kinds, sources) {
            ([kind], []) => Some(Self {
                kind: *kind,
                facets: facets.to_vec(),
                source: None,
                source_title: None,
            }),
            ([kind], [source]) => Some(Self {
                kind: *kind,
                facets: facets.to_vec(),
                source: Some(source.clone()),
                source_title: None,
            }),
            _ => None,
        }
    }

    /// The same scope with the book's display name, which Quick search reads from the library.
    pub fn with_source_title(mut self, title: Option<String>) -> Self {
        self.source_title = title;
        self
    }

    /// `feats`, or `Combat feats` when a facet narrows it. The facet comes first and keeps its
    /// capital, so the phrase reads after "Search". An item category is already a plural noun
    /// (`Wondrous Items`, `Rods`), so it stands without the kind: `Search Rods`. A book filter
    /// appends the book's name: `feats in Core Rulebook`.
    fn phrase(&self) -> String {
        let noun = self.kind.label().to_lowercase();
        let facets: Vec<String> = self
            .facets
            .iter()
            .map(|(field, value)| upper_first(&facet_label(field, value)))
            .collect();
        let mut phrase = if facets.is_empty() {
            noun
        } else if self.kind == EntryKind::Item
            && self
                .facets
                .iter()
                .any(|(field, _)| field == "item_category")
        {
            facets.join(" ")
        } else {
            format!("{} {noun}", facets.join(" "))
        };
        if let Some(title) = &self.source_title {
            phrase = format!("{phrase} in {title}");
        }
        phrase
    }

    /// The field's placeholder and accessible name: `Search feats`, `Search Combat feats`.
    pub fn placeholder(&self) -> String {
        format!("Search {}", self.phrase())
    }

    /// The text of the pill: `Feats`, `Combat feats`.
    pub fn label(&self) -> String {
        upper_first(&self.phrase())
    }

    /// The results page's count line with the scope named: `12 results in Combat feats`,
    /// `18 results in Feats`.
    pub fn results_line(&self, results: &str) -> String {
        format!("{results} in {}", self.label())
    }

    /// The control that widens a facet scope back to the whole kind: `Show all feats`.
    pub fn show_all_label(&self) -> String {
        format!("Show all {}", self.kind.label().to_lowercase())
    }

    /// The accessible name of the pill's remove control. It contains the visible text.
    pub fn remove_label(&self) -> String {
        format!("Remove scope: {}", self.label())
    }

    /// The engine filters that carry the scope.
    pub fn filters(&self) -> Filters {
        Filters {
            kinds: vec![self.kind.slug().to_string()],
            sources: self.source.iter().cloned().collect(),
            facets: self.facets.clone(),
        }
    }

    /// The results page for `q` with this scope applied.
    pub fn route(&self, q: impl Into<String>) -> Route {
        Route::Search {
            q: q.into(),
            kinds: vec![self.kind],
            sources: self.source.iter().cloned().collect(),
            facets: self.facets.clone(),
        }
    }
}

/// The scope as a Filter-role pill with a 44 px remove target. The whole pill is the button, so
/// the visible text and the cross are one target and one accessible name.
#[component]
pub fn ScopePill(scope: Scope, on_remove: Callback<()>) -> impl IntoView {
    view! {
        <FilterPill label=scope.label() name=scope.remove_label() on_remove=on_remove />
    }
}

/// One Filter-role pill. `label` is what the reader sees and `name` is the accessible name, which
/// must contain it. The list page's book filter uses it with `Remove filter: {Book}`; search's
/// scope uses [`ScopePill`].
#[component]
pub fn FilterPill(label: String, name: String, on_remove: Callback<()>) -> impl IntoView {
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

    fn scope(kind: EntryKind, facets: Vec<(String, String)>) -> Scope {
        Scope {
            kind,
            facets,
            source: None,
            source_title: None,
        }
    }

    fn feats() -> Scope {
        scope(EntryKind::Feat, Vec::new())
    }

    fn combat_feats() -> Scope {
        scope(EntryKind::Feat, vec![("feat_type".into(), "combat".into())])
    }

    fn combat_feats_cased() -> Scope {
        scope(EntryKind::Feat, vec![("feat_type".into(), "Combat".into())])
    }

    fn core_feats() -> Scope {
        let mut scope = scope(EntryKind::Feat, Vec::new());
        scope.source = Some("core-rulebook".into());
        scope.source_title = Some("Core Rulebook".into());
        scope
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
        // A facet that narrows the list narrows the search, and a letter does not.
        assert_eq!(
            Scope::of_page(&Route::browse_kind_facets(
                EntryKind::Feat,
                vec![("feat_type".into(), "Combat".into())]
            )),
            Some(combat_feats_cased())
        );
        assert_eq!(
            Scope::of_page(
                &Route::browse_kind_facets(
                    EntryKind::Feat,
                    vec![("feat_type".into(), "Combat".into())]
                )
                .with_letter('C')
            ),
            Some(combat_feats_cased())
        );
        // A facet the kind does not have is not a scope.
        assert_eq!(
            Scope::of_page(&Route::browse_kind_facets(
                EntryKind::Feat,
                vec![("school".into(), "evocation".into())]
            )),
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
    fn a_menu_value_reads_after_search_for_every_kind() {
        let scope = |kind, field: &str, value: &str| {
            self::scope(kind, vec![(field.to_string(), value.to_string())])
        };
        assert_eq!(
            scope(EntryKind::Spell, "school", "evocation").placeholder(),
            "Search Evocation spells"
        );
        assert_eq!(
            scope(EntryKind::Trait, "trait_category", "Combat").placeholder(),
            "Search Combat traits"
        );
        assert_eq!(
            scope(EntryKind::Archetype, "archetype_class", "wizard").placeholder(),
            "Search Wizard archetypes"
        );
        // A category names the items itself, so the kind is not said twice.
        let rods = scope(EntryKind::Item, "item_category", "Rods");
        assert_eq!(rods.placeholder(), "Search Rods");
        assert_eq!(rods.label(), "Rods");
        assert_eq!(rods.show_all_label(), "Show all items");
        assert_eq!(rods.results_line("4 results"), "4 results in Rods");
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
    fn the_results_line_names_the_scope_and_the_widening_control_names_the_kind() {
        assert_eq!(
            combat_feats().results_line("12 results"),
            "12 results in Combat feats"
        );
        assert_eq!(feats().results_line("18 results"), "18 results in Feats");
        assert_eq!(feats().results_line("No results"), "No results in Feats");
        assert_eq!(combat_feats().show_all_label(), "Show all feats");
        let prestige = Scope::of_page(&Route::browse_kind(EntryKind::PrestigeClass)).unwrap();
        assert_eq!(prestige.show_all_label(), "Show all prestige classes");
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
        assert_eq!(
            Scope::of_search(&[EntryKind::Feat], &[], &[]),
            Some(feats())
        );
        assert_eq!(
            Scope::of_search(
                &[EntryKind::Feat],
                &[("feat_type".into(), "combat".into())],
                &[]
            ),
            Some(combat_feats())
        );
        assert_eq!(Scope::of_search(&[], &[], &[]), None);
        assert_eq!(
            Scope::of_search(&[EntryKind::Feat, EntryKind::Spell], &[], &[]),
            None
        );
        // One publication narrows the scope too; several do not.
        let mut book = feats();
        book.source = Some("core-rulebook".into());
        assert_eq!(
            Scope::of_search(&[EntryKind::Feat], &[], &["core-rulebook".into()]),
            Some(book)
        );
        assert_eq!(
            Scope::of_search(
                &[EntryKind::Feat],
                &[],
                &["core-rulebook".into(), "bestiary".into()]
            ),
            None
        );
    }

    #[test]
    fn the_route_a_scope_makes_gives_the_same_scope_back() {
        let route = combat_feats().route("x");
        let Route::Search {
            kinds,
            facets,
            sources,
            ..
        } = Route::parse(&route.href())
        else {
            panic!("not a search route");
        };
        assert_eq!(
            Scope::of_search(&kinds, &facets, &sources),
            Some(combat_feats())
        );
    }

    #[test]
    fn a_book_filter_scopes_quick_search_to_the_kind_and_the_book() {
        let route = Route::browse_kind(EntryKind::Feat).with_source(Some("core-rulebook".into()));
        let page = Scope::of_page(&route).unwrap();
        assert_eq!(page.source.as_deref(), Some("core-rulebook"));
        // Quick search fills the title from the library; the pill then names the book.
        let named = page.with_source_title(Some("Core Rulebook".into()));
        assert_eq!(named, core_feats());
        assert_eq!(named.label(), "Feats in Core Rulebook");
        assert_eq!(named.placeholder(), "Search feats in Core Rulebook");
        assert_eq!(named.remove_label(), "Remove scope: Feats in Core Rulebook");
        assert_eq!(named.filters().sources, ["core-rulebook"]);
        assert_eq!(
            named.route("grapple").href(),
            "#/search?q=grapple&kind=feat&source=core-rulebook"
        );
    }
}
