//! Search: query box, live ranked results, kind/source/facet filters. The route holds the whole
//! state, so a search is deep-linkable and back works. Typing replaces the history entry after
//! a short pause; toggling a filter or leaving Home with a query pushes one.

use crate::js;
use crate::library::use_library;
use crate::route::{Route, navigate, replace, use_route};
use crate::views::util::{kind_name, set_title};
use cast_iron_schema::{ChallengeRating, EntryId, EntryKind, SourceInfo};
use cast_iron_search::{FilterCounts, Filters, Hit, HitSummary, Query, SearchIndex};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use wasm_bindgen::JsCast;

const PAGE: usize = 50;
const DEBOUNCE_MS: u32 = 80;
/// Publications shown before the reader asks for the rest.
const PUBLICATION_TOP: usize = 12;

// ---- pure helpers -----------------------------------------------------------------------

/// Splits `text` into `(start, end, highlighted)` pieces from byte ranges. Ranges that are out
/// of bounds or not on character boundaries are ignored; overlapping ranges merge.
pub fn highlight_segments(text: &str, ranges: &[(usize, usize)]) -> Vec<(usize, usize, bool)> {
    let mut valid: Vec<(usize, usize)> = ranges
        .iter()
        .copied()
        .filter(|&(a, b)| {
            a < b && b <= text.len() && text.is_char_boundary(a) && text.is_char_boundary(b)
        })
        .collect();
    valid.sort();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (a, b) in valid {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    let mut out = Vec::new();
    let mut at = 0;
    for (a, b) in merged {
        if a > at {
            out.push((at, a, false));
        }
        out.push((a, b, true));
        at = b;
    }
    if at < text.len() {
        out.push((at, text.len(), false));
    }
    out
}

/// Facet fields and their labels for a kind.
pub fn facet_fields_of(kind: EntryKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        EntryKind::Spell => &[
            ("school", "School"),
            ("class", "Class"),
            ("spell_level", "Spell level"),
        ],
        EntryKind::Feat => &[("feat_type", "Feat type")],
        EntryKind::Creature => &[
            ("creature_type", "Creature type"),
            ("cr", "CR"),
            ("size", "Size"),
        ],
        EntryKind::Item => &[("item_category", "Category"), ("slot", "Slot")],
        EntryKind::Class | EntryKind::PrestigeClass => &[("class_type", "Class type")],
        EntryKind::Race => &[("race_group", "Race group")],
        EntryKind::Trait => &[("trait_category", "Trait category")],
        EntryKind::Archetype => &[("archetype_class", "Class")],
        _ => &[],
    }
}

/// Facet fields to offer for the selected kinds, without repeats.
pub fn facet_fields_for(kinds: &[EntryKind]) -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    for kind in kinds {
        for field in facet_fields_of(*kind) {
            if !out.contains(field) {
                out.push(*field);
            }
        }
    }
    out
}

/// Drops facets whose field the selected kinds do not offer.
pub fn prune_facets(facets: &[(String, String)], kinds: &[EntryKind]) -> Vec<(String, String)> {
    let allowed = facet_fields_for(kinds);
    facets
        .iter()
        .filter(|(f, _)| allowed.iter().any(|(name, _)| name == f))
        .cloned()
        .collect()
}

pub fn toggled<T: PartialEq + Clone>(list: &[T], item: &T) -> Vec<T> {
    if list.contains(item) {
        list.iter().filter(|x| *x != item).cloned().collect()
    } else {
        let mut next = list.to_vec();
        next.push(item.clone());
        next
    }
}

/// Orders facet values for display: levels numerically, CR by value, the rest alphabetically.
pub fn sort_facet_values(field: &str, values: &mut [String]) {
    values.sort_by_cached_key(|v| facet_sort_key(field, v));
}

/// The same order for `(value, count)` pairs.
pub fn sort_facet_pairs(field: &str, values: &mut [(String, u32)]) {
    values.sort_by_cached_key(|(v, _)| facet_sort_key(field, v));
}

fn facet_sort_key(field: &str, value: &str) -> (u32, String) {
    match field {
        "spell_level" => (value.parse::<u32>().unwrap_or(u32::MAX), String::new()),
        "cr" => (
            ChallengeRating::parse(value).map_or(u32::MAX, |c| c.twenty_fourths),
            String::new(),
        ),
        _ => (0, value.to_lowercase()),
    }
}

/// The text of a facet value on screen. The value itself stays what the index holds, because
/// routes and filters use it. Most values read well as they are; the ones below do not.
pub fn facet_label(field: &str, value: &str) -> String {
    match (field, value) {
        ("race_group", "standard_race") => "Core Rulebook races".into(),
        ("race_group", "core_race") => "Core races (Advanced Race Guide)".into(),
        ("race_group", "featured_race") => "Featured races".into(),
        ("race_group", "uncommon_race") => "Uncommon races".into(),
        ("race_group", "monster_race") => "Monster races".into(),
        ("class_type", "npc") => "NPC".into(),
        ("class_type" | "trait_category" | "archetype_class" | "school", _) => {
            capitalize_words(value)
        }
        _ => value.to_string(),
    }
}

/// Connective words that stay lowercase inside a facet label, except as the first word.
const SHORT_CONNECTIVES: [&str; 4] = ["or", "and", "of", "the"];

fn capitalize_words(text: &str) -> String {
    text.split(' ')
        .enumerate()
        .map(|(i, word)| {
            if i > 0 {
                let lower = word.to_lowercase();
                if SHORT_CONNECTIVES.contains(&lower.as_str()) {
                    return lower;
                }
            }
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The count a chip shows: the latest result's, or `total` until the first result arrives.
fn chip_count(
    counts: RwSignal<Option<FilterCounts>>,
    total: usize,
    pick: impl Fn(&FilterCounts) -> u32,
) -> usize {
    counts.with(|c| c.as_ref().map_or(total, |c| pick(c) as usize))
}

/// The accessible name of a chip with its count.
fn chip_name(label: &str, count: usize) -> String {
    match count {
        1 => format!("{label}, 1 entry"),
        n => format!("{label}, {n} entries"),
    }
}

/// The publications a reader can see, and the counts for the controls around them.
///
/// A publication with no match is absent unless it is already selected, so it can be cleared.
/// At most [`PUBLICATION_TOP`] publications with results show until the reader asks for all of
/// them or narrows the list by name. Selected publications come first, then results by count.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PublicationPage {
    ids: Vec<String>,
    /// Publications with at least one match for the current query and filters.
    with_results: usize,
    /// Visible publications that have at least one match.
    shown_with_results: usize,
    /// Publications with results that are not visible, while the reader has not asked for all.
    more: usize,
    /// Publications with results whose title matches the name box.
    matches: usize,
}

fn publication_page(
    sources: &[SourceInfo],
    selected: &[String],
    count_of: impl Fn(&str) -> usize,
    filter: &str,
    show_all: bool,
) -> PublicationPage {
    let filter = filter.trim().to_lowercase();
    let selected_set: BTreeSet<&str> = selected.iter().map(String::as_str).collect();
    let mut matching: Vec<(&SourceInfo, usize)> = sources
        .iter()
        .filter_map(|source| {
            let count = count_of(&source.id);
            (count > 0 && (filter.is_empty() || source.title.to_lowercase().contains(&filter)))
                .then_some((source, count))
        })
        .collect();
    matching.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()))
            .then_with(|| a.0.id.cmp(&b.0.id))
    });
    let with_results = sources.iter().filter(|s| count_of(&s.id) > 0).count();
    // A name box is an explicit request to see every match. So is "Show all publications".
    let limit = if show_all || !filter.is_empty() {
        matching.len()
    } else {
        PUBLICATION_TOP.min(matching.len())
    };
    let mut ids: Vec<String> = Vec::new();
    for id in selected {
        if let Some(source) = sources.iter().find(|source| &source.id == id)
            && !ids.contains(&source.id)
        {
            ids.push(source.id.clone());
        }
    }
    let mut shown_with_results = ids.iter().filter(|id| count_of(id) > 0).count();
    for (source, count) in matching.iter().take(limit) {
        if selected_set.contains(source.id.as_str()) {
            continue;
        }
        ids.push(source.id.clone());
        if *count > 0 {
            shown_with_results += 1;
        }
    }
    let more = if filter.is_empty() && !show_all {
        with_results.saturating_sub(shown_with_results)
    } else {
        0
    };
    PublicationPage {
        ids,
        with_results,
        shown_with_results,
        more,
        matches: matching.len(),
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct SearchState {
    q: String,
    kinds: Vec<EntryKind>,
    sources: Vec<String>,
    facets: Vec<(String, String)>,
}

impl SearchState {
    fn from_route(route: &Route) -> Self {
        match route {
            Route::Search {
                q,
                kinds,
                sources,
                facets,
            } => Self {
                q: q.clone(),
                kinds: kinds.clone(),
                sources: sources.clone(),
                facets: facets.clone(),
            },
            _ => Self::default(),
        }
    }

    fn route(&self) -> Route {
        Route::Search {
            q: self.q.clone(),
            kinds: self.kinds.clone(),
            sources: self.sources.clone(),
            facets: self.facets.clone(),
        }
    }

    fn query(&self, limit: usize) -> Query {
        Query {
            text: self.q.clone(),
            filters: Filters {
                kinds: self.kinds.iter().map(|k| k.slug().to_string()).collect(),
                sources: self.sources.clone(),
                facets: self.facets.clone(),
            },
            limit,
        }
    }

    fn filter_count(&self) -> usize {
        self.kinds.len() + self.sources.len() + self.facets.len()
    }
}

/// Keep the best ranked record as the group position, retaining every publication choice.
fn grouped_hits(hits: Vec<HitSummary>) -> Vec<SearchHit> {
    let mut positions = std::collections::BTreeMap::<(String, String), usize>::new();
    let mut groups: Vec<SearchHit> = Vec::new();
    for summary in hits {
        let key = (
            summary.kind.clone(),
            crate::views::source_variants::normalized_title(&summary.title),
        );
        if let Some(&position) = positions.get(&key) {
            groups[position].variants.push((summary.into_hit(), None));
        } else {
            positions.insert(key, groups.len());
            groups.push(SearchHit {
                hit: summary.into_hit(),
                anchor: None,
                variants: Vec::new(),
            });
        }
    }
    groups
}

#[derive(Clone)]
enum Phase {
    Idle,
    LoadingIndex,
    Done {
        hits: Vec<SearchHit>,
        total: usize,
        groups: usize,
    },
    Failed(String),
}

#[derive(Clone)]
struct SearchHit {
    hit: Hit,
    anchor: Option<String>,
    variants: Vec<(Hit, Option<String>)>,
}

#[derive(Clone, PartialEq)]
struct FacetGroup {
    field: &'static str,
    label: &'static str,
    values: Vec<String>,
}

fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or_else(js::now_ms, |p| p.now())
}

fn pointer_is_fine() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(pointer: fine)").ok().flatten())
        .is_some_and(|m| m.matches())
}

fn element_by_id(id: &str) -> Option<web_sys::HtmlElement> {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into().ok())
}

/// True when the event target is a text field or sits inside a contenteditable region, where
/// `/` is a character rather than a shortcut.
fn is_text_entry(target: &web_sys::EventTarget) -> bool {
    let Some(node) = target.dyn_ref::<web_sys::Node>() else {
        return false;
    };
    let mut current = Some(node.clone());
    while let Some(node) = current {
        if let Some(element) = node.dyn_ref::<web_sys::HtmlElement>()
            && (matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || element.is_content_editable())
        {
            return true;
        }
        current = node.parent_node();
    }
    false
}

/// `/` focuses the search box from anywhere outside a text field, or opens Search.
pub fn install_slash_shortcut() {
    let handle = window_event_listener(leptos::ev::keydown, move |ev| {
        // Match the produced character, not Shift: some layouts (German, French) need Shift for `/`.
        if ev.is_composing() || ev.key() != "/" || ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        if ev.target().is_some_and(|target| is_text_entry(&target)) {
            return;
        }
        ev.prevent_default();
        if let Some(input) = element_by_id("search-input") {
            let _ = input.focus();
            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                input.select();
            }
        } else {
            navigate(&Route::search(""));
            spawn_local(async {
                js::sleep(60).await;
                if let Some(input) = element_by_id("search-input") {
                    let _ = input.focus();
                }
            });
        }
    });
    on_cleanup(move || handle.remove());
}

// ---- view -------------------------------------------------------------------------------

/// Text with `<mark>` around the given byte ranges.
pub fn marked(text: &str, ranges: &[(usize, usize)]) -> Vec<AnyView> {
    highlight_segments(text, ranges)
        .into_iter()
        .map(|(a, b, on)| {
            let piece = text[a..b].to_string();
            if on {
                view! { <mark>{piece}</mark> }.into_any()
            } else {
                piece.into_any()
            }
        })
        .collect()
}

/// The discovery snapshot for one selection revision. Search asks for it from the search
/// effect, both chip-total memos, and the idle panel; rebuilding it each time rescanned the
/// whole catalog once per caller.
fn discovery_for(
    library: &crate::library::Library,
    cache: StoredValue<Option<(u64, Rc<crate::discovery::Discovery>)>, LocalStorage>,
    revision: u64,
) -> Rc<crate::discovery::Discovery> {
    if let Some((cached, discovery)) = cache.get_value()
        && cached == revision
    {
        return discovery;
    }
    let discovery = Rc::new(library.discovery());
    cache.set_value(Some((revision, discovery.clone())));
    discovery
}

#[component]
pub fn Search() -> impl IntoView {
    let library = use_library();
    let route = use_route();
    set_title("Search");

    let state = Memo::new(move |_| SearchState::from_route(&route.get()));
    let text = RwSignal::new(state.get_untracked().q);
    // The last text that this screen wrote to the route. A route change with another text came
    // from outside (back or forward) and replaces the box.
    let written = StoredValue::new(text.get_untracked());
    let typing_gen = RwSignal::new(0u64);
    let pages = RwSignal::new(1usize);
    let phase = RwSignal::new(Phase::Idle);
    let search_ms = RwSignal::new(None::<f64>);
    let index_ms = RwSignal::new(None::<f64>);
    let index = RwSignal::new_local(None::<Rc<SearchIndex>>);
    let eligible_facets = RwSignal::new(Vec::<(String, Vec<String>)>::new());
    // Facet values do not depend on the query or the active filters: only on the library and
    // the active-book selection. Cache them per revision so typing and toggling chips do not
    // rescan every indexed document for every field on every search.
    let facet_values_cache = StoredValue::new(None::<(u64, BTreeMap<String, Vec<String>>)>);
    let discovery_cache = StoredValue::new_local(None::<(u64, Rc<crate::discovery::Discovery>)>);
    // Counts for the chips. They change only when a result arrives, so a debounced or running
    // search keeps showing the previous numbers. `None` until the first result: chips then show
    // the totals for the active books, which is what an empty search counts anyway.
    let counts = RwSignal::new(None::<FilterCounts>);
    let seq = StoredValue::new(0u64);
    let filters_open = RwSignal::new(false);
    let publication_filter = RwSignal::new(String::new());
    let publications_open = RwSignal::new(false);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    Effect::new(move |_| {
        let q = state.with(|s| s.q.clone());
        if q != written.get_value() && q != text.get_untracked() {
            written.set_value(q.clone());
            text.set(q);
        }
    });

    Effect::new(move |_| {
        if let Some(input) = input_ref.get()
            && pointer_is_fine()
        {
            let _ = input.focus();
        }
    });

    // The search itself. Stale answers are dropped by sequence number.
    {
        let library = library.clone();
        Effect::new(move |previous: Option<SearchState>| {
            let revision = library.selection_revision().get();
            let discovery = discovery_for(&library, discovery_cache, revision);
            let current = state.get();
            let previous_value = current.clone();
            if previous.as_ref() != Some(&current) {
                pages.update_untracked(|p| *p = 1);
            }
            let limit = pages.get() * PAGE;
            let id = seq.get_value() + 1;
            seq.set_value(id);
            let library = library.clone();
            let query = current.query(usize::MAX);
            // The requested page is sliced only after eligible hits have been ranked and counted.
            let page_limit = limit;
            let runs = !current.q.trim().is_empty() || current.filter_count() > 0;
            spawn_local(async move {
                // A search that outlives the screen must not read its signals: the owner is
                // disposed when the reader leaves, and reading a disposed value panics. Finish
                // the run only while it is still the current one.
                let alive = || {
                    seq.try_get_value() == Some(id)
                        && library.selection_revision().get_untracked() == revision
                };
                if !runs {
                    phase.set(Phase::Idle);
                    search_ms.set(None);
                    // Warm the index so the first keystroke does not wait for it.
                    ensure_index(&library, index, index_ms, phase, false).await;
                    if let Ok(fresh) = discovery.filter_counts("", &Filters::default()).await
                        && alive()
                    {
                        counts.set(Some(fresh));
                    }
                    return;
                }
                let loaded = ensure_index(&library, index, index_ms, phase, true).await;
                if !alive() {
                    return;
                }
                let Some(idx) = loaded else { return };
                let started = now();
                let all_hits = match discovery.search_summaries(&query).await {
                    Ok(hits) => hits,
                    Err(error) => {
                        if alive() {
                            phase.set(Phase::Failed(error.to_string()));
                        }
                        return;
                    }
                };
                if !alive() {
                    return;
                }
                let total = all_hits.len();
                let grouped = grouped_hits(all_hits);
                let groups = grouped.len();
                let mut hits = grouped.into_iter().take(page_limit).collect::<Vec<_>>();
                // Only the rendered rows scan their document text for highlights.
                let highlighter = idx.highlighter(&query);
                for result in &mut hits {
                    idx.materialize_hit(&highlighter, &mut result.hit);
                    for (hit, _) in &mut result.variants {
                        idx.materialize_hit(&highlighter, hit);
                    }
                }
                let engine_ms = now() - started;
                let fresh_counts = discovery
                    .filter_counts(&current.q, &query.filters)
                    .await
                    .ok();
                let mut facet_data = Vec::new();
                for (field, _) in facet_fields_for(&current.kinds) {
                    if !alive() {
                        return;
                    }
                    let cached = facet_values_cache.with_value(|slot| match slot {
                        Some((cached, values)) if *cached == revision => values.get(field).cloned(),
                        _ => None,
                    });
                    let values = match cached {
                        Some(values) => values,
                        None => match discovery.facet_values(field).await {
                            Ok(pairs) => {
                                let mut values =
                                    pairs.into_iter().map(|(v, _)| v).collect::<Vec<_>>();
                                sort_facet_values(field, &mut values);
                                let for_cache = values.clone();
                                facet_values_cache.update_value(|slot| {
                                    let fresh =
                                        !matches!(slot, Some((cached, _)) if *cached == revision);
                                    if fresh {
                                        *slot = Some((revision, BTreeMap::new()));
                                    }
                                    if let Some((_, values)) = slot {
                                        values.insert(field.to_owned(), for_cache);
                                    }
                                });
                                values
                            }
                            Err(_) => continue,
                        },
                    };
                    facet_data.push((field.to_owned(), values));
                }
                // Rule headings come from typed, cached rule entries only. Non-rule results
                // never cause shard reads.
                let query_text = current.q.clone();
                for result in &mut hits {
                    for (hit, anchor) in std::iter::once((&result.hit, &mut result.anchor)).chain(
                        result
                            .variants
                            .iter_mut()
                            .map(|(hit, anchor)| (&*hit, anchor)),
                    ) {
                        if hit.kind != EntryKind::Rule.slug() {
                            continue;
                        }
                        if let Ok(id) = EntryId::parse(hit.id.clone())
                            && let Ok(entry) = library.entry(&id).await
                        {
                            *anchor = crate::views::entry_contents::matching_heading(
                                &entry.body,
                                &query_text,
                            );
                        }
                    }
                }
                if !alive() {
                    return;
                }
                eligible_facets.set(facet_data);
                if let Some(fresh) = fresh_counts {
                    counts.set(Some(fresh));
                }
                search_ms.set(Some(engine_ms));
                phase.set(Phase::Done {
                    hits,
                    total,
                    groups,
                });
            });
            previous_value
        });
    }

    let write = move |next: SearchState, push: bool| {
        written.set_value(next.q.clone());
        let route = next.route();
        if push {
            navigate(&route);
        } else {
            replace(&route);
        }
    };
    let flush = move || {
        let mut next = state.get_untracked();
        let typed = text.get_untracked();
        if next.q != typed {
            next.q = typed;
            write(next, false);
        }
    };
    let schedule = move || {
        let id = typing_gen.get_untracked() + 1;
        typing_gen.set(id);
        spawn_local(async move {
            js::sleep(DEBOUNCE_MS).await;
            if typing_gen.try_get_untracked() == Some(id) {
                flush();
            }
        });
    };
    let change_filters = move |edit: &dyn Fn(&mut SearchState)| {
        typing_gen.update_untracked(|g| *g += 1);
        let mut next = state.get_untracked();
        next.q = text.get_untracked();
        edit(&mut next);
        next.facets = prune_facets(&next.facets, &next.kinds);
        write(next, true);
    };

    let facet_library = library.clone();
    let facet_groups = Memo::new(move |_: Option<&Vec<FacetGroup>>| {
        let _revision = facet_library.selection_revision().get();
        let kinds = state.with(|s| s.kinds.clone());
        let groups: Vec<FacetGroup> = eligible_facets.with(|facets| {
            facet_fields_for(&kinds)
                .into_iter()
                .map(|(field, label)| {
                    let values = facets
                        .iter()
                        .find(|(name, _)| name == field)
                        .map(|(_, values)| values.clone())
                        .unwrap_or_default();
                    FacetGroup {
                        field,
                        label,
                        values,
                    }
                })
                .filter(|g| !g.values.is_empty())
                .collect()
        });
        groups
    });

    let count_library = library.clone();
    let kind_counts = Memo::new(move |_: Option<&Vec<(EntryKind, usize)>>| {
        let revision = count_library.selection_revision().get();
        discovery_for(&count_library, discovery_cache, revision).kind_counts()
    });
    let kind_counts = kind_counts.get();
    // One pass over the eligible entries for every publication chip. Computing this inside
    // each chip's count closure rebuilt the whole discovery set once per source, which is
    // seconds of work on a library with hundreds of publications.
    let source_totals_library = library.clone();
    let source_totals = Memo::new(move |_| {
        let revision = source_totals_library.selection_revision().get();
        let mut totals: BTreeMap<String, usize> = BTreeMap::new();
        for stub in discovery_for(&source_totals_library, discovery_cache, revision).entries() {
            *totals.entry(stub.source.clone()).or_default() += 1;
        }
        totals
    });
    let kind_chips = kind_counts
        .into_iter()
        .map(|(kind, total)| {
            let checked = move || state.with(|s| s.kinds.contains(&kind));
            let slug = kind.slug();
            let n = move || chip_count(counts, total, |c| c.kind(slug));
            view! {
                <label class="chip" class:chip-empty=move || n() == 0 && !checked()>
                    <input
                        type="checkbox"
                        prop:checked=checked
                        aria-label=move || chip_name(kind.label(), n())
                        on:change=move |_| change_filters(&|s| s.kinds = toggled(&s.kinds, &kind))
                    />
                    <span>{kind.label()}<small class="chip-count">{n}</small></span>
                </label>
            }
        })
        .collect_view();
    let quick_scopes = [
        ("All entries", None),
        ("Rules text", Some(EntryKind::Rule)),
        ("Spells", Some(EntryKind::Spell)),
        ("Feats", Some(EntryKind::Feat)),
    ]
    .into_iter()
    .map(|(label, kind)| {
        let selected = move || {
            state.with(|s| match kind {
                Some(kind) => s.kinds.as_slice() == [kind],
                None => s.kinds.is_empty(),
            })
        };
        view! {
            <button class="scope-choice" type="button" aria-pressed=move || selected().to_string()
                on:click=move |_| change_filters(&|s| s.kinds = kind.into_iter().collect())>
                {label}
            </button>
        }
    })
    .collect_view();
    // Every publication chip is kept out of the DOM until its publication matches. The list is
    // rendered in the same order a reader sees, so keyboard focus follows the page.
    let publication_sources = library.sources().to_vec();
    let publication_titles: BTreeMap<String, String> = publication_sources
        .iter()
        .map(|source| (source.id.clone(), source.title.clone()))
        .collect();
    let publication_view = Memo::new(move |_| {
        let selected = state.with(|s| s.sources.clone());
        let filter = publication_filter.get();
        let show_all = publications_open.get();
        counts.with(|counts| {
            source_totals.with(|totals| {
                publication_page(
                    &publication_sources,
                    &selected,
                    |id| {
                        counts.as_ref().map_or_else(
                            || totals.get(id).copied().unwrap_or(0),
                            |counts| counts.source(id) as usize,
                        )
                    },
                    &filter,
                    show_all,
                )
            })
        })
    });
    let publication_chips = {
        let titles = publication_titles;
        move |id: String| {
            let title = titles.get(&id).cloned().unwrap_or_else(|| id.clone());
            let count_id = id.clone();
            let count_id2 = id.clone();
            let change_id = id.clone();
            // The active-book total is the chip count before any search; the search's own
            // counts replace it when a result arrives.
            let total = Signal::derive(move || {
                source_totals.with(|totals| totals.get(&count_id).copied().unwrap_or(0))
            });
            let n =
                Signal::derive(move || chip_count(counts, total.get(), |c| c.source(&count_id2)));
            let checked = Signal::derive(move || state.with(|s| s.sources.contains(&id)));
            let name_title = title.clone();
            view! {
                <label class="chip" class:chip-empty=move || n.get() == 0 && !checked.get()>
                    <input
                        type="checkbox"
                        prop:checked=checked
                        aria-label=move || chip_name(&name_title, n.get())
                        on:change=move |_| change_filters(&|s| s.sources = toggled(&s.sources, &change_id))
                    />
                    <span>{title}<small class="chip-count">{n}</small></span>
                </label>
            }
        }
    };
    let publication_summary = move || {
        let page = publication_view.get();
        let filtered = publication_filter.with(|filter| !filter.trim().is_empty());
        if filtered {
            format!(
                "{} of {} publications with results match “{}”.",
                page.shown_with_results,
                page.matches,
                publication_filter.with(|filter| filter.trim().to_string())
            )
        } else if publications_open.get() {
            format!(
                "Showing all {} publications with results.",
                page.with_results
            )
        } else {
            format!(
                "Showing {} of {} publications with results.",
                page.shown_with_results, page.with_results
            )
        }
    };
    let publication_tools = view! {
        <label class="publication-filter">
            <span class="sr-only">"Filter publications by name"</span>
            <input
                type="search"
                placeholder="Find a publication"
                autocomplete="off"
                spellcheck="false"
                prop:value=move || publication_filter.get()
                aria-controls="publication-chips"
                aria-describedby="publication-summary"
                on:input=move |ev| publication_filter.set(event_target_value(&ev))
            />
        </label>
        <p class="publication-summary" id="publication-summary">{publication_summary}</p>
        <div id="publication-chips" class="chips">
            <For
                each=move || publication_view.with(|page| page.ids.clone())
                key=|id| id.clone()
                children=publication_chips
            />
        </div>
        <Show when=move || {
            publication_view.with(|page| page.more > 0)
                || (publications_open.get() && publication_filter.with(|filter| filter.trim().is_empty()))
        }>
            <button
                class="btn publication-more"
                type="button"
                aria-expanded=move || publications_open.get().to_string()
                aria-controls="publication-chips"
                on:click=move |_| publications_open.update(|open| *open = !*open)
            >
                {move || {
                    if publications_open.get() {
                        "Show fewer publications".to_string()
                    } else {
                        format!("Show all publications ({} more)", publication_view.with(|page| page.more))
                    }
                }}
            </button>
        </Show>
    };

    let facet_view = move || {
        let groups = facet_groups.get();
        let kinds_empty = state.with(|s| s.kinds.is_empty());
        if groups.is_empty() {
            return kinds_empty
                .then(|| {
                    view! { <p class="hint">"Choose a kind to filter by school, level, CR, and more."</p> }
                })
                .into_any();
        }
        groups
            .into_iter()
            .map(|group| {
                let field = group.field;
                view! {
                    <fieldset class="filter-group">
                        <legend>{group.label}</legend>
                        <div class="chips chips-scroll">
                            {group.values.into_iter().map(|value| {
                                let label = facet_label(field, &value);
                                let pair = (field.to_string(), value.clone());
                                let pair2 = pair.clone();
                                let checked = Signal::derive(move || state.with(|s| s.facets.contains(&pair)));
                                let name_label = label.clone();
                                let value_for_count = value.clone();
                                // Without a result yet there is no count to show.
                                let n = Signal::derive(move || counts.with(|c| c.as_ref().map(|c| c.facet(field, &value_for_count))));
                                view! {
                                    <label class="chip" class:chip-empty=move || n.get() == Some(0) && !checked.get()>
                                        <input
                                            type="checkbox"
                                            prop:checked=checked
                                            aria-label=move || match n.get() {
                                                Some(n) => chip_name(&name_label, n as usize),
                                                None => name_label.clone(),
                                            }
                                            on:change=move |_| change_filters(&|s| s.facets = toggled(&s.facets, &pair2))
                                        />
                                        <span>{label}<small class="chip-count">{n}</small></span>
                                    </label>
                                }
                            }).collect_view()}
                        </div>
                    </fieldset>
                }
            })
            .collect_view()
            .into_any()
    };

    let lib_for_hits = library.clone();
    let results = move || match phase.get() {
        Phase::Idle => {
            let revision = lib_for_hits.selection_revision().get();
            let total = discovery_for(&lib_for_hits, discovery_cache, revision)
                .entries()
                .count();
            view! {
                <div class="empty">
                    <p>{format!("Search {total} entries by name or by any word in the text.")}</p>
                    <p class="muted">"Try one of these:"</p>
                    <p class="examples">
                        {["fireball", "grapple", "cure light", "dragon"].into_iter().map(|q| {
                            view! { <a class="jump" href=Route::search(q).href()>{q}</a> }
                        }).collect_view()}
                    </p>
                </div>
            }
            .into_any()
        }
        Phase::LoadingIndex => view! { <p class="empty">"Loading search index…"</p> }.into_any(),
        Phase::Failed(message) => view! {
            <div class="error" role="alert">
                <p>{message}</p>
                <p><a href=Route::Settings.href()>"Open Settings"</a></p>
            </div>
        }
        .into_any(),
        Phase::Done { hits, .. } if hits.is_empty() => view! {
            <div class="empty">
                <p>"No entries match."</p>
                <p class="muted">"Check the spelling, use fewer words, or clear a filter."</p>
            </div>
        }
        .into_any(),
        Phase::Done { hits, groups, .. } => {
            let full = hits.len() < groups;
            let items = hits
                .iter()
                .map(|result| {
                    if result.variants.is_empty() {
                        return hit_view(&lib_for_hits, &result.hit, result.anchor.as_deref()).into_any();
                    }
                    let title = marked(&result.hit.title, &result.hit.title_matches);
                    let count = result.variants.len() + 1;
                    let variants = std::iter::once((&result.hit, &result.anchor)).chain(result.variants.iter().map(|(hit, anchor)| (hit, anchor)))
                        .map(|(hit, anchor)| hit_view(&lib_for_hits, hit, anchor.as_deref())).collect_view();
                    view! { <li class="hit hit-group"><details class="source-variants">
                        <summary><span class="hit-title">{title}</span><span class="hit-meta">{format!("{} · {count} publication entries", EntryKind::from_slug(&result.hit.kind).map_or("", kind_name))}</span></summary>
                        <ul>{variants}</ul>
                    </details></li> }.into_any()
                })
                .collect_view();
            view! {
                <ol class="results">{items}</ol>
                {full.then(|| view! {
                    <p><button class="btn" type="button" on:click=move |_| pages.update(|p| *p += 1)>
                        "Show more results"
                    </button></p>
                })}
            }
            .into_any()
        }
    };

    let count_text = move || match phase.get() {
        Phase::LoadingIndex => "Loading search index…".to_string(),
        Phase::Done { total, groups, .. } => {
            let n = groups;
            let limit = pages.get() * PAGE;
            let text = match (n, n > limit) {
                (0, _) => "No results".to_string(),
                (1, _) => "1 result".to_string(),
                (_, true) => format!("Top {} of {n} results", limit.min(n)),
                _ => format!("{n} results"),
            };
            if total == groups {
                text
            } else {
                format!("{text} · {total} publication entries")
            }
        }
        _ => String::new(),
    };

    let clear_filters = move |_| {
        change_filters(&|s| {
            s.kinds.clear();
            s.sources.clear();
            s.facets.clear();
        })
    };

    view! {
        <div class="search-page">
            <h1 class="sr-only">"Search"</h1>
            <form class="search-form" role="search" on:submit=move |ev| {
                ev.prevent_default();
                typing_gen.update_untracked(|g| *g += 1);
                flush();
                if !pointer_is_fine()
                    && let Some(input) = input_ref.get()
                {
                    let _ = input.blur();
                }
            }>
                <label class="sr-only" for="search-input">"Search the rules"</label>
                <input
                    id="search-input"
                    node_ref=input_ref
                    type="search"
                    placeholder="Search spells, feats, rules…"
                    autocomplete="off"
                    autocapitalize="off"
                    spellcheck="false"
                    enterkeyhint="search"
                    prop:value=move || text.get()
                    on:input=move |ev| { text.set(event_target_value(&ev)); schedule(); }
                />
                <button class="btn btn-primary" type="submit">"Search"</button>
            </form>
            <div class="search-scopes" role="group" aria-label="Quick search categories">{quick_scopes}</div>
            <div class="search-layout">
                <aside class="filters" aria-label="Filters">
                    <button
                        class="btn filters-toggle"
                        type="button"
                        aria-expanded=move || filters_open.get().to_string()
                        aria-controls="filters-panel"
                        on:click=move |_| filters_open.update(|o| *o = !*o)
                    >
                        {move || {
                            let n = state.with(|s| s.filter_count());
                            if n == 0 { "Filters".to_string() } else { format!("Filters ({n})") }
                        }}
                    </button>
                    <div id="filters-panel" class="filters-panel" class:open=move || filters_open.get()>
                        <fieldset class="filter-group">
                            <legend>"Kind"</legend>
                            <div class="chips">{kind_chips}</div>
                        </fieldset>
                        <fieldset class="filter-group">
                            <legend>"Publication"</legend>
                            {publication_tools}
                        </fieldset>
                        {facet_view}
                        <Show when=move || state.with(|s| s.filter_count() > 0)>
                            <button class="btn" type="button" on:click=clear_filters>"Clear filters"</button>
                        </Show>
                    </div>
                </aside>
                <section
                    class="results-region"
                    aria-label="Results"
                    data-search-ms=move || search_ms.get().map(|ms| format!("{ms:.2}"))
                    data-index-ms=move || index_ms.get().map(|ms| format!("{ms:.1}"))
                    data-query=move || state.with(|s| s.q.clone())
                >
                    <p class="result-count" role="status" aria-live="polite">
                        <span>{count_text}</span>
                        {move || search_ms.get().filter(|_| matches!(phase.get(), Phase::Done { .. })).map(|ms| view! {
                            <span class="ms" aria-hidden="true">{format!(" · {ms:.1} ms")}</span>
                        })}
                    </p>
                    {results}
                </section>
            </div>
        </div>
    }
}

/// Loads the index once. With `show`, flags the loading state while waiting.
async fn ensure_index(
    library: &crate::library::Library,
    index: RwSignal<Option<Rc<SearchIndex>>, LocalStorage>,
    index_ms: RwSignal<Option<f64>>,
    phase: RwSignal<Phase>,
    show: bool,
) -> Option<Rc<SearchIndex>> {
    if let Some(idx) = index.get_untracked() {
        return Some(idx);
    }
    if show {
        phase.set(Phase::LoadingIndex);
        // Let the browser paint the message before the index decodes on this thread.
        js::sleep(0).await;
    }
    let started = now();
    match library.search_index().await {
        Ok(idx) => {
            if index_ms.get_untracked().is_none() {
                index_ms.set(Some(now() - started));
            }
            index.set(Some(idx.clone()));
            Some(idx)
        }
        Err(error) => {
            if show {
                phase.set(Phase::Failed(error.to_string()));
            }
            None
        }
    }
}

fn hit_view(
    library: &crate::library::Library,
    hit: &Hit,
    anchor: Option<&str>,
) -> impl IntoView + use<> {
    let href = EntryId::parse(hit.id.clone())
        .map(|id| {
            Route::Entry {
                id,
                anchor: anchor.map(str::to_owned),
            }
            .href()
        })
        .unwrap_or_else(|_| Route::Home.href());
    let kind = EntryKind::from_slug(&hit.kind).map_or("", kind_name);
    let source = library
        .source(&hit.source)
        .map_or_else(|| hit.source.clone(), |s| s.title.clone());
    let title = marked(&hit.title, &hit.title_matches);
    let snippet = marked(&hit.snippet.text, &hit.snippet.matches);
    let has_snippet = !hit.snippet.text.is_empty();
    view! {
        <li class="hit">
            <a class="hit-link" href=href>
                <span class="hit-title">{title}</span>
                <span class="hit-meta">{kind}" · "{source}</span>
                {has_snippet.then(|| view! { <span class="hit-snippet">{snippet}</span> })}
            </a>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_splits_on_ranges() {
        let text = "Fire the fireball";
        let segs = highlight_segments(text, &[(0, 4), (9, 13)]);
        assert_eq!(
            segs,
            vec![(0, 4, true), (4, 9, false), (9, 13, true), (13, 17, false)]
        );
        let joined: String = segs.iter().map(|&(a, b, _)| &text[a..b]).collect();
        assert_eq!(joined, text);
    }

    #[test]
    fn highlight_ignores_bad_ranges_and_merges_overlaps() {
        let text = "héllo wörld";
        // (1,2) splits the two-byte é; (50,60) is out of bounds; (3,3) is empty.
        let segs = highlight_segments(text, &[(1, 2), (50, 60), (3, 3), (0, 3), (3, 6)]);
        assert_eq!(segs[0], (0, 6, true));
        assert_eq!(highlight_segments("", &[(0, 1)]), vec![]);
        assert_eq!(highlight_segments("abc", &[]), vec![(0, 3, false)]);
    }

    #[test]
    fn facets_follow_the_selected_kinds() {
        assert!(facet_fields_for(&[]).is_empty());
        let spell = facet_fields_for(&[EntryKind::Spell]);
        assert_eq!(
            spell.iter().map(|f| f.0).collect::<Vec<_>>(),
            ["school", "class", "spell_level"]
        );
        let both = facet_fields_for(&[EntryKind::Spell, EntryKind::Feat, EntryKind::Spell]);
        assert_eq!(both.len(), 4);
        let facets = vec![
            ("school".to_string(), "evocation".to_string()),
            ("cr".to_string(), "5".to_string()),
        ];
        assert_eq!(
            prune_facets(&facets, &[EntryKind::Spell]),
            vec![("school".to_string(), "evocation".to_string())]
        );
        assert!(prune_facets(&facets, &[]).is_empty());
    }

    #[test]
    fn new_kinds_offer_their_facets() {
        for (kind, field) in [
            (EntryKind::Race, "race_group"),
            (EntryKind::Trait, "trait_category"),
            (EntryKind::Archetype, "archetype_class"),
        ] {
            assert_eq!(facet_fields_for(&[kind])[0].0, field);
        }
    }

    #[test]
    fn facet_labels_read_well_and_leave_values_alone() {
        assert_eq!(facet_label("race_group", "monster_race"), "Monster races");
        assert_eq!(facet_label("class_type", "npc"), "NPC");
        assert_eq!(facet_label("class_type", "base"), "Base");
        assert_eq!(facet_label("trait_category", "combat"), "Combat");
        assert_eq!(
            facet_label("archetype_class", "arcane archer"),
            "Arcane Archer"
        );
        assert_eq!(
            facet_label("archetype_class", "knight of the road"),
            "Knight of the Road"
        );
        assert_eq!(
            facet_label("archetype_class", "Knight Of The Road"),
            "Knight of the Road"
        );
        assert_eq!(facet_label("archetype_class", "the road"), "The Road");
        // Fields that already read well are not touched.
        assert_eq!(facet_label("school", "evocation"), "Evocation");
        assert_eq!(facet_label("cr", "1/2"), "1/2");
        assert_eq!(facet_label("race_group", "unheard_of"), "unheard_of");
    }

    #[test]
    fn facet_pairs_sort_with_their_counts() {
        let mut pairs: Vec<(String, u32)> =
            vec![("10".into(), 1), ("2".into(), 5), ("1/2".into(), 7)];
        sort_facet_pairs("cr", &mut pairs);
        assert_eq!(pairs[0], ("1/2".to_string(), 7));
        assert_eq!(pairs[2], ("10".to_string(), 1));
    }

    /// Counts for a search state over a small index, as the Search view asks for them.
    fn counts_for(state: &SearchState, active: &[&str]) -> FilterCounts {
        use cast_iron_search::{DocInput, IndexBuilder};
        let doc = |id: &str, title: &str, kind: &str, source: &str, school: &str| DocInput {
            id: id.into(),
            title: title.into(),
            kind: kind.into(),
            source: source.into(),
            section: "S".into(),
            facets: vec![("school".into(), school.into())],
            text: "flame".into(),
            ..DocInput::default()
        };
        let mut builder = IndexBuilder::new();
        for d in [
            doc("spell:a", "Fire Bolt", "spell", "core", "evocation"),
            doc("spell:b", "Fire Wall", "spell", "apg", "evocation"),
            doc("spell:c", "Fire Shield", "spell", "core", "abjuration"),
            doc("feat:a", "Fire Mastery", "feat", "core", "none"),
        ] {
            builder.add(d);
        }
        let index = builder.build();
        index.filter_counts_eligible(&state.q, &state.query(0).filters, |id| active.contains(&id))
    }

    #[test]
    fn counts_of_an_empty_search_are_the_active_book_totals() {
        let all = ["spell:a", "spell:b", "spell:c", "feat:a"];
        let c = counts_for(&SearchState::default(), &all);
        assert_eq!((c.kind("spell"), c.kind("feat")), (3, 1));
        assert_eq!((c.source("core"), c.source("apg")), (3, 1));
        assert_eq!(c.facet("school", "evocation"), 2);
    }

    #[test]
    fn a_kind_filter_leaves_kind_counts_and_narrows_source_and_facet_counts() {
        let all = ["spell:a", "spell:b", "spell:c", "feat:a"];
        let state = SearchState {
            q: "fire".into(),
            kinds: vec![EntryKind::Spell],
            ..SearchState::default()
        };
        let c = counts_for(&state, &all);
        assert_eq!((c.kind("spell"), c.kind("feat")), (3, 1));
        assert_eq!((c.source("core"), c.source("apg")), (2, 1));
        assert_eq!(c.facet("school", "none"), 0);
        assert_eq!(c.total, 3);
    }

    #[test]
    fn a_source_filter_narrows_kind_and_facet_counts() {
        let all = ["spell:a", "spell:b", "spell:c", "feat:a"];
        let state = SearchState {
            q: "fire".into(),
            sources: vec!["apg".into()],
            ..SearchState::default()
        };
        let c = counts_for(&state, &all);
        assert_eq!((c.kind("spell"), c.kind("feat")), (1, 0));
        assert_eq!((c.source("core"), c.source("apg")), (3, 1));
        assert_eq!(c.facet("school", "evocation"), 1);
        assert_eq!(c.facet("school", "abjuration"), 0);
    }

    #[test]
    fn an_active_book_change_changes_the_counts() {
        let state = SearchState {
            q: "fire".into(),
            ..SearchState::default()
        };
        let both = counts_for(&state, &["spell:a", "spell:b", "spell:c", "feat:a"]);
        let core = counts_for(&state, &["spell:a", "spell:c", "feat:a"]);
        assert_eq!((both.kind("spell"), core.kind("spell")), (3, 2));
        assert_eq!((both.source("apg"), core.source("apg")), (1, 0));
        assert_eq!((both.total, core.total), (4, 3));
    }

    #[test]
    fn chip_names_carry_their_counts() {
        assert_eq!(chip_name("Spells", 12), "Spells, 12 entries");
        assert_eq!(chip_name("Feats", 1), "Feats, 1 entry");
        assert_eq!(chip_name("Rules", 0), "Rules, 0 entries");
    }

    fn source(id: &str, title: &str) -> SourceInfo {
        SourceInfo {
            id: id.into(),
            title: title.into(),
            entry_count: 0,
            copyright: None,
        }
    }

    fn page_of(
        sources: &[SourceInfo],
        counts: &[(&str, usize)],
        selected: &[&str],
        filter: &str,
        show_all: bool,
    ) -> PublicationPage {
        let selected: Vec<String> = selected.iter().map(|id| (*id).to_string()).collect();
        publication_page(
            sources,
            &selected,
            |id| {
                counts
                    .iter()
                    .find(|(source, _)| *source == id)
                    .map_or(0, |(_, count)| *count)
            },
            filter,
            show_all,
        )
    }

    #[test]
    fn publications_with_no_matches_are_not_listed() {
        let sources = [
            source("a", "Alpha"),
            source("b", "Beta"),
            source("c", "Gamma"),
        ];
        let page = page_of(&sources, &[("a", 5), ("b", 0), ("c", 2)], &[], "", false);
        assert_eq!(page.ids, ["a", "c"]);
        assert_eq!(page.with_results, 2);
        assert_eq!(page.shown_with_results, 2);
        assert_eq!(page.more, 0);
    }

    #[test]
    fn the_first_page_keeps_the_twelve_largest_publications() {
        let sources: Vec<SourceInfo> = (0..20)
            .map(|n| source(&format!("s{n:02}"), &format!("Source {n:02}")))
            .collect();
        let counts: Vec<(&str, usize)> = sources
            .iter()
            .enumerate()
            .map(|(n, source)| (source.id.as_str(), n + 1))
            .collect();
        let page = page_of(&sources, &counts, &[], "", false);
        assert_eq!(page.ids.len(), PUBLICATION_TOP);
        assert_eq!(page.ids[0], "s19");
        assert_eq!(page.ids[PUBLICATION_TOP - 1], "s08");
        assert_eq!(page.with_results, 20);
        assert_eq!(page.shown_with_results, PUBLICATION_TOP);
        assert_eq!(page.more, 20 - PUBLICATION_TOP);
    }

    #[test]
    fn a_selected_publication_stays_visible_with_no_matches() {
        let sources = [
            source("a", "Alpha"),
            source("b", "Beta"),
            source("c", "Gamma"),
        ];
        let page = page_of(&sources, &[("a", 5), ("b", 0), ("c", 2)], &["b"], "", false);
        assert_eq!(page.ids[0], "b");
        assert_eq!(page.ids, ["b", "a", "c"]);
        assert_eq!(page.shown_with_results, 2);
        assert_eq!(page.more, 0);
    }

    #[test]
    fn a_name_filter_shows_every_match_and_no_zero_count_row() {
        let sources = [
            source("core", "Core Rulebook"),
            source("apg", "Advanced Player's Guide"),
            source("bestiary", "Bestiary"),
        ];
        let counts = [("core", 5), ("apg", 3), ("bestiary", 0)];
        let page = page_of(&sources, &counts, &[], "player", false);
        assert_eq!(page.ids, ["apg"]);
        assert_eq!(page.matches, 1);
        assert_eq!(page.more, 0);
        let none = page_of(&sources, &counts, &[], "bestiary", false);
        assert!(none.ids.is_empty(), "a zero-count publication stays hidden");
    }

    #[test]
    fn show_all_lists_every_publication_with_results() {
        let sources: Vec<SourceInfo> = (0..20)
            .map(|n| source(&format!("s{n:02}"), &format!("Source {n:02}")))
            .collect();
        let counts: Vec<(&str, usize)> = sources
            .iter()
            .enumerate()
            .map(|(n, source)| (source.id.as_str(), n + 1))
            .collect();
        let page = page_of(&sources, &counts, &[], "", true);
        assert_eq!(page.ids.len(), 20);
        assert_eq!(page.shown_with_results, 20);
        assert_eq!(page.more, 0);
    }

    #[test]
    fn equal_counts_sort_by_title_and_the_selected_set_is_not_repeated() {
        let sources = [
            source("z", "Zebra"),
            source("a", "Alpha"),
            source("b", "Beta"),
        ];
        let counts = [("z", 2), ("a", 2), ("b", 0)];
        let page = page_of(&sources, &counts, &["z"], "", false);
        assert_eq!(page.ids, ["z", "a"]);
        assert_eq!(page.with_results, 2);
    }

    #[test]
    fn toggled_adds_and_removes() {
        let a = toggled(&[1, 2], &3);
        assert_eq!(a, vec![1, 2, 3]);
        assert_eq!(toggled(&a, &2), vec![1, 3]);
    }

    #[test]
    fn facet_values_sort_by_meaning() {
        let mut levels: Vec<String> = ["10", "2", "0", "9"].map(String::from).to_vec();
        sort_facet_values("spell_level", &mut levels);
        assert_eq!(levels, ["0", "2", "9", "10"]);
        let mut cr: Vec<String> = ["10", "1/2", "1", "1/8"].map(String::from).to_vec();
        sort_facet_values("cr", &mut cr);
        assert_eq!(cr, ["1/8", "1/2", "1", "10"]);
        let mut names: Vec<String> = ["fire", "Acid", "cold"].map(String::from).to_vec();
        sort_facet_values("school", &mut names);
        assert_eq!(names, ["Acid", "cold", "fire"]);
    }
}
