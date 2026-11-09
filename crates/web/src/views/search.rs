//! Search: query box, live ranked results, kind/source/facet filters. The route holds the whole
//! state, so a search is deep-linkable and back works. Typing replaces the history entry after
//! a short pause; toggling a filter or leaving Home with a query pushes one.

use crate::fmt;
use crate::js;
use crate::library::use_library;
use crate::revisions::RevisionView;
use crate::route::{Route, navigate, replace, use_route};
use crate::views::book_tier::{Tier, preferred_source};
use crate::views::icons;
use crate::views::quick_search::{QuickSearch, STARTERS, focus_results_field};
use crate::views::scope::{Scope, ScopePill};
use crate::views::util::{addon_name, kind_name, local_resource, set_title};
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

/// The facets a kind's list page applies from its URL: those the kind offers, one value for
/// each field. A list page narrows by one value per field, so a hand-made URL with two values
/// for one field uses the first.
pub fn list_facets(facets: &[(String, String)], kind: EntryKind) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (field, value) in prune_facets(facets, &[kind]) {
        if !out.iter().any(|(earlier, _)| *earlier == field) {
            out.push((field, value));
        }
    }
    // A class option's group is a list filter, not a search facet: `option-groups.json` holds
    // the groups, and the search index has no `group` field to scope a query with.
    if kind == EntryKind::ClassOption
        && !out.iter().any(|(field, _)| field == "group")
        && let Some((_, value)) = facets.iter().find(|(field, _)| field == "group")
    {
        out.push(("group".to_string(), value.clone()));
    }
    out
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
        // The stored value stays `0`; the reader sees the word that creature spell lists use.
        ("spell_level", "0") => fmt::spell_level(0),
        ("race_group", "standard_race") => "Core Rulebook races".into(),
        ("race_group", "core_race") => "Core races (Advanced Race Guide)".into(),
        ("race_group", "featured_race") => "Featured races".into(),
        ("race_group", "uncommon_race") => "Uncommon races".into(),
        ("race_group", "monster_race") => "Monster races".into(),
        ("class_type", "npc") => "NPC".into(),
        ("creature_type", "magicalBeast") => "Magical Beast".into(),
        ("creature_type", "monstrousHumanoid") => "Monstrous Humanoid".into(),
        ("creature_type", _)
        | ("class_type" | "trait_category" | "archetype_class" | "school", _) => {
            capitalize_words(value)
        }
        // The class-option list prints the group's own label; this is only the fallback for a
        // screen that has the slug without the index (an old shared link, for example).
        ("group", _) => capitalize_words(&value.replace('-', " ")),
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
    format!("{label}, {}", fmt::pluralize(count, "entry", "entries"))
}

/// The publications a reader can see, grouped by product-line tier, and the counts for the
/// controls around them.
///
/// A publication with no match is absent unless it is already selected, so it can be cleared.
/// Groups follow [`Tier::ALL`]: Rulebooks first. A tier with no result and no selection is
/// absent; every other tier is present, even when none of its chips fit the budget, so its
/// heading can say how many publications it holds.
///
/// The first page spends a budget of [`PUBLICATION_TOP`] chips in group order. Selected
/// publications are always listed and spend the budget first; the rest are listed group by
/// group, so a Rulebook is never hidden while a lower-tier publication that nobody chose shows.
/// A tier that is collapsed by default (Adventures) takes no budget: its chips would sit in a
/// closed disclosure. Every tier but Rulebooks is a disclosure. A reader who opens one, or asks
/// for all publications, or types in the name box, gets all of that tier's publications; those
/// do not spend the budget. A group's count is the publications with results that it holds.
/// Inside a group the order is selected first, then results by count, then title.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PublicationPage {
    groups: Vec<PublicationGroup>,
    /// Publications with at least one match for the current query and filters.
    with_results: usize,
    /// Listed publications with at least one match inside a group that is open. A chip in a
    /// closed disclosure is not visible, so it is not counted.
    shown_with_results: usize,
    /// Publications with results that are not visible, while the reader has not asked for all.
    more: usize,
    /// Publications with results whose title matches the name box.
    matches: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicationGroup {
    tier: Tier,
    /// The chips to mount, in tab order.
    ids: Vec<String>,
    /// Publications in this tier with at least one match: the count the heading shows.
    with_results: usize,
    /// How many of those are in [`PublicationGroup::ids`].
    listed_with_results: usize,
    /// The group holds a selected publication, so it must be open.
    holds_selected: bool,
    /// Whether the chips are visible: always for Rulebooks, otherwise the disclosure's state.
    open: bool,
}

impl PublicationGroup {
    /// Publications with results that a reader can see in this group right now.
    fn shown_with_results(&self) -> usize {
        if self.open {
            self.listed_with_results
        } else {
            0
        }
    }

    /// The heading: the tier and how many publications with results it holds. An open group
    /// that lists only some of them says so, so the number never claims chips that are absent.
    fn heading(&self) -> String {
        let label = self.tier.label();
        if self.open && self.listed_with_results < self.with_results {
            format!(
                "{label} ({} of {})",
                self.listed_with_results, self.with_results
            )
        } else {
            format!("{label} ({})", self.with_results)
        }
    }
}

impl PublicationPage {
    /// Every mounted publication id in the order a reader tabs through them.
    #[cfg(test)]
    fn ids(&self) -> Vec<&str> {
        self.groups
            .iter()
            .flat_map(|group| group.ids.iter().map(String::as_str))
            .collect()
    }

    fn group(&self, tier: Tier) -> Option<&PublicationGroup> {
        self.groups.iter().find(|group| group.tier == tier)
    }
}

/// What a disclosure's toggle event does with the reader's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToggleAction {
    /// The tier is forced open by a selection or the name box: reopen it, record nothing.
    ForceOpen,
    /// Store the reader's answer.
    Record(bool),
    /// The event agrees with the page already; nothing to store.
    Ignore,
}

/// A selection or a non-empty name box forces its tier open, so a close click on such a tier is
/// not the reader's choice: it is answered by reopening the tier and recording nothing. This
/// keeps the answer the reader gave before the force, which clearing the box restores.
fn publication_toggle_action(
    holds_selected: bool,
    name_box: bool,
    open_now: bool,
    open_after_click: bool,
) -> ToggleAction {
    if (holds_selected || name_box) && !open_after_click {
        ToggleAction::ForceOpen
    } else if open_after_click != open_now {
        ToggleAction::Record(open_after_click)
    } else {
        ToggleAction::Ignore
    }
}

/// `choices` holds the reader's own open (`true`) or closed (`false`) answer per tier. A
/// selected publication forces its tier open, so a chosen filter is never hidden, and so does a
/// non-empty name box, so its matches are seen. Otherwise the reader's answer applies, then the
/// default: open when the tier has chips and is not collapsed by default. "Show all" opens a
/// collapsed-by-default tier too, because it lists everything. A page whose only results are in
/// one collapsed tier opens that tier and spends the budget on it, because a page of closed
/// headings looks empty (#142). Adventures and third-party books are the collapsed tiers.
fn publication_page(
    sources: &[SourceInfo],
    selected: &[String],
    count_of: impl Fn(&str) -> usize,
    filter: &str,
    show_all: bool,
    choices: &BTreeMap<Tier, bool>,
) -> PublicationPage {
    let filter = filter.trim().to_lowercase();
    let selected_set: BTreeSet<&str> = selected.iter().map(String::as_str).collect();
    let name_matches =
        |source: &SourceInfo| filter.is_empty() || source.title.to_lowercase().contains(&filter);
    let with_results = sources.iter().filter(|s| count_of(&s.id) > 0).count();
    let tiers_with_results: BTreeSet<Tier> = sources
        .iter()
        .filter(|s| count_of(&s.id) > 0)
        .map(Tier::of)
        .collect();
    let only_tier = if tiers_with_results.len() == 1 {
        tiers_with_results.iter().next().copied()
    } else {
        None
    };
    let matches = sources
        .iter()
        .filter(|s| count_of(&s.id) > 0 && name_matches(s))
        .count();
    let name_box = !filter.is_empty();
    // A name box is an explicit request to see every match. So is "Show all publications".
    let unlimited = show_all || name_box;
    // A tier lists every publication when asked: by the reader opening its disclosure, or by
    // one of the page-wide requests above. Rulebooks have no disclosure.
    let expanded = |tier: Tier| {
        unlimited || (tier.is_disclosure() && choices.get(&tier).copied() == Some(true))
    };
    // A single collapsed tier with results opens and spends the budget, unless the reader
    // closed it.
    let one_tier_default = only_tier
        .filter(|tier| tier.collapsed_by_default())
        .filter(|tier| choices.get(tier).copied() != Some(false));
    let takes_budget = |tier: Tier| !tier.collapsed_by_default() || Some(tier) == one_tier_default;

    // Candidates by tier: pinned selections, plus every publication with results that the name
    // box matches. Selected first, then count, then title.
    let mut candidates: Vec<(Tier, &SourceInfo, usize, bool)> = sources
        .iter()
        .filter_map(|source| {
            let count = count_of(&source.id);
            let pinned = selected_set.contains(source.id.as_str());
            (pinned || (count > 0 && name_matches(source))).then(|| {
                let tier = Tier::of(source);
                (tier, source, count, pinned)
            })
        })
        .collect();
    candidates.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| b.3.cmp(&a.3))
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.1.title.to_lowercase().cmp(&b.1.title.to_lowercase()))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });

    // Selected publications with results spend the budget first, unless their tier lists
    // everything anyway. Then the rest are taken in group order, which is what keeps a Rulebook
    // ahead of every lower tier.
    let mut budget = PUBLICATION_TOP.saturating_sub(
        candidates
            .iter()
            .filter(|(tier, _, count, pinned)| *pinned && *count > 0 && !expanded(*tier))
            .count(),
    );
    let mut groups: Vec<PublicationGroup> = Vec::new();
    for (tier, source, count, pinned) in candidates {
        if groups.last().is_none_or(|group| group.tier != tier) {
            groups.push(PublicationGroup {
                tier,
                ids: Vec::new(),
                with_results: 0,
                listed_with_results: 0,
                holds_selected: false,
                open: false,
            });
        }
        let group = groups.last_mut().expect("a group was just pushed");
        let counted = count > 0 && name_matches(source);
        let listed = if pinned || expanded(tier) {
            true
        } else if takes_budget(tier) && budget > 0 {
            budget -= 1;
            true
        } else {
            false
        };
        group.with_results += usize::from(counted);
        group.holds_selected |= pinned;
        if listed {
            group.ids.push(source.id.clone());
            group.listed_with_results += usize::from(counted);
        }
    }
    for group in &mut groups {
        let tier = group.tier;
        group.open = !tier.is_disclosure()
            || group.holds_selected
            || name_box
            || choices
                .get(&tier)
                .copied()
                .unwrap_or(!group.ids.is_empty() && (show_all || takes_budget(tier)));
    }
    let shown_with_results = groups
        .iter()
        .map(PublicationGroup::shown_with_results)
        .sum();
    let more = if !name_box && !show_all {
        with_results.saturating_sub(shown_with_results)
    } else {
        0
    };
    PublicationPage {
        groups,
        with_results,
        shown_with_results,
        more,
        matches,
    }
}

/// The line above the publication chips. A pinned publication that the name box does not match
/// is not counted, so the numbers never read like "1 of 0". A chip inside a closed disclosure
/// is not counted as shown either, so the line never claims a chip the reader cannot see.
fn publication_summary_line(page: &PublicationPage, filter: &str, show_all: bool) -> String {
    let filter = filter.trim();
    if !filter.is_empty() {
        return match page.matches {
            0 => format!("No publication matches “{filter}”."),
            1 => "Showing the one matching publication with results.".to_string(),
            n => format!(
                "Showing all {} matching publications with results.",
                fmt::count(n)
            ),
        };
    }
    if show_all && page.shown_with_results == page.with_results {
        format!(
            "Showing all {} with results.",
            fmt::pluralize(page.with_results, "publication", "publications")
        )
    } else {
        format!(
            "Showing {} of {} with results.",
            fmt::count(page.shown_with_results),
            fmt::pluralize(page.with_results, "publication", "publications")
        )
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

/// Matches the query found in inactive books: how many, and the book that holds the most.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InactiveMatches {
    count: usize,
    source: String,
    title: String,
}

/// Counts the query's hits that discovery removed because their entry's book is inactive,
/// grouped by that book. Pure data, so a native test covers the counting without a browser.
fn inactive_match_counts(
    hits: &[HitSummary],
    is_eligible: impl Fn(&str) -> bool,
    source_of: impl Fn(&HitSummary) -> String,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for hit in hits {
        if !is_eligible(&hit.id) {
            *counts.entry(source_of(hit)).or_default() += 1;
        }
    }
    counts
}

/// The hidden count and the book to name: the one with the most hidden matches, then the
/// higher-priority `Tier`, then the source id ascending, so a tie is deterministic.
/// `None` when nothing is hidden.
fn inactive_matches(
    hits: &[HitSummary],
    is_eligible: impl Fn(&str) -> bool,
    source_of: impl Fn(&HitSummary) -> String,
    tier_of: impl Fn(&str) -> Tier,
) -> Option<(usize, String)> {
    let counts = inactive_match_counts(hits, is_eligible, source_of);
    let (source, _) = counts.iter().max_by(|a, b| {
        a.1.cmp(b.1)
            .then_with(|| tier_of(b.0).cmp(&tier_of(a.0)))
            .then_with(|| b.0.cmp(a.0))
    })?;
    Some((counts.values().sum(), source.clone()))
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

/// `/` focuses the search box from anywhere outside a text field: the field of the results page
/// when it is open, otherwise Quick search.
pub fn install_slash_shortcut(quick: QuickSearch) {
    let handle = window_event_listener(leptos::ev::keydown, move |ev| {
        // Match the produced character, not Shift: some layouts (German, French) need Shift for `/`.
        if ev.is_composing() || ev.key() != "/" || ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        if quick.is_open() || ev.target().is_some_and(|target| is_text_entry(&target)) {
            return;
        }
        ev.prevent_default();
        if !focus_results_field() {
            quick.open();
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
pub(crate) fn discovery_for(
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
    // The add-on labels of result rows. The whole graph, so a mythic row keeps its label while
    // its base's book is inactive; a hit is only shown when its own book is active.
    let addon_labels = {
        let library = library.clone();
        local_resource(async move { library.relations().await.ok().flatten() })
    };
    // A class-option row names its group's singular label, which lives in `option-groups.json`.
    let option_groups = {
        let library = library.clone();
        local_resource(async move { library.option_groups().await.ok().flatten() })
    };
    // Counts for the chips. They change only when a result arrives, so a debounced or running
    // search keeps showing the previous numbers. `None` until the first result: chips then show
    // the totals for the active books, which is what an empty search counts anyway.
    let counts = RwSignal::new(None::<FilterCounts>);
    // Matches that the active-book filter removed, with the book to name. `None` while every
    // book is active or nothing is hidden.
    let inactive = RwSignal::new(None::<InactiveMatches>);
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
            // WebKit puts the caret at the start of a value that was set before the focus, such
            // as the text that Quick search handed over. Typing continues at the end.
            let end = u32::try_from(input.value().encode_utf16().count()).unwrap_or(0);
            let _ = input.set_selection_range(end, end);
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
                    inactive.set(None);
                    // Warm the index so the first keystroke does not wait for it.
                    ensure_index(&library, index, index_ms, phase, false, &alive).await;
                    if let Ok(fresh) = discovery.filter_counts("", &Filters::default()).await
                        && alive()
                    {
                        counts.set(Some(fresh));
                    }
                    return;
                }
                let loaded = ensure_index(&library, index, index_ms, phase, true, &alive).await;
                if !alive() {
                    return;
                }
                let Some(idx) = loaded else { return };
                let started = now();
                let all_hits = match discovery.search_summaries(&query).await {
                    Ok(hits) => hits,
                    Err(error) => {
                        if alive() {
                            inactive.set(None);
                            phase.set(Phase::Failed(error.to_string()));
                        }
                        return;
                    }
                };
                // A revision is a second indexed document with the same title. Rewrite it to
                // the original id and keep one row, so a search never shows the pair.
                let all_hits = match library.relations().await {
                    Ok(Some(relations)) => {
                        let view = RevisionView::build(
                            &relations,
                            |id| library.is_entry_active(id),
                            |id| library.stub(id).map(|stub| stub.source.clone()),
                        );
                        crate::revisions::effective_hits(all_hits, &view, |id| {
                            discovery.is_eligible(id)
                        })
                    }
                    _ => all_hits,
                };
                if !alive() {
                    return;
                }
                // The index holds every book; eligibility is applied per query. Search the
                // unfiltered index once more to say what the active-book filter removed.
                let hidden = if library.has_inactive_books() {
                    let every_hit = idx.search_summaries(&query);
                    inactive_matches(
                        &every_hit,
                        |id| discovery.is_eligible(id),
                        |hit| {
                            EntryId::parse(hit.id.clone())
                                .ok()
                                .and_then(|id| library.canonical_source_for_entry(&id))
                                .unwrap_or_else(|| hit.source.clone())
                        },
                        |id| library.source(id).map_or(Tier::Other, Tier::of),
                    )
                } else {
                    None
                };
                if !alive() {
                    return;
                }
                inactive.set(hidden.map(|(count, source)| {
                    InactiveMatches {
                        count,
                        title: library
                            .source(&source)
                            .map_or_else(|| source.clone(), |s| s.title.clone()),
                        source,
                    }
                }));
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
                                if !alive() {
                                    return;
                                }
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
        // A debounce can outlive the screen: the column is gone after the reader leaves Search.
        let Some(mut next) = state.try_get_untracked() else {
            return;
        };
        let Some(typed) = text.try_get_untracked() else {
            return;
        };
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
                    <span>{kind.label()}<small class="chip-count">{move || fmt::count(n())}</small></span>
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
                id=kind.is_none().then_some("scope-all")
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
    // The reader's open or closed answer per tier. It lives outside the keyed group list, so it
    // survives a result refresh, and a group that drops out and returns keeps it.
    let group_choice = RwSignal::new(BTreeMap::<Tier, bool>::new());
    let publication_view = Memo::new(move |_| {
        let selected = state.with(|s| s.sources.clone());
        let filter = publication_filter.get();
        let show_all = publications_open.get();
        group_choice.with(|choices| {
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
                        choices,
                    )
                })
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
                    <span>{title}<small class="chip-count">{move || fmt::count(n.get())}</small></span>
                </label>
            }
        }
    };
    // One product-line group. The keyed lists keep every chip and group mounted while counts
    // refresh, so focus and an open group survive a new result. Every tier but Rulebooks is a
    // native disclosure whose open state and chip list come from `publication_page`, which reads
    // signals that outlive the group.
    let group_open = move |tier: Tier| {
        publication_view.with(|page| page.group(tier).is_some_and(|group| group.open))
    };
    let group_heading = move |tier: Tier| {
        publication_view.with(|page| {
            page.group(tier)
                .map_or_else(|| tier.label().to_string(), PublicationGroup::heading)
        })
    };
    let group_ids = move |tier: Tier| {
        publication_view.with(|page| page.group(tier).map(|g| g.ids.clone()).unwrap_or_default())
    };
    let publication_group = move |tier: Tier| {
        let label = tier.label();
        let title_id = format!(
            "publication-group-{}",
            label.to_lowercase().replace(' ', "-")
        );
        let chips = view! {
            <For each=move || group_ids(tier) key=|id| id.clone() children=publication_chips.clone() />
        };
        if !tier.is_disclosure() {
            return view! {
                <div class="publication-group" role="group" aria-labelledby=title_id.clone() data-tier=label>
                    <p class="publication-group-title" id=title_id.clone()>{move || group_heading(tier)}</p>
                    <div class="chips">{chips}</div>
                </div>
            }
            .into_any();
        }
        let on_toggle = move |ev: web_sys::Event| {
            let details = event_target::<web_sys::HtmlDetailsElement>(&ev);
            let (holds_selected, open_now) = publication_view.with_untracked(|page| {
                page.group(tier)
                    .map_or((false, false), |group| (group.holds_selected, group.open))
            });
            let name_box = publication_filter.with_untracked(|filter| !filter.trim().is_empty());
            match publication_toggle_action(holds_selected, name_box, open_now, details.open()) {
                ToggleAction::ForceOpen => details.set_open(true),
                ToggleAction::Record(open) => {
                    group_choice.update(|choices| {
                        choices.insert(tier, open);
                    });
                }
                ToggleAction::Ignore => {}
            }
        };
        view! {
            <details class="publication-group publication-details" data-tier=label
                prop:open=move || group_open(tier) on:toggle=on_toggle>
                <summary class="publication-group-summary">
                    {icons::chevron_right("disclosure-chevron")}
                    <span class="publication-group-title" id=title_id.clone()>
                        {move || group_heading(tier)}
                    </span>
                </summary>
                <div class="chips" role="group" aria-labelledby=title_id>{chips}</div>
            </details>
        }
        .into_any()
    };
    let publication_summary = move || {
        let page = publication_view.get();
        let filter = publication_filter.with(|filter| filter.clone());
        publication_summary_line(&page, &filter, publications_open.get())
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
        <div id="publication-chips" class="publication-groups">
            <For
                each=move || publication_view.with(|page| page.groups.iter().map(|group| group.tier).collect::<Vec<_>>())
                key=|tier| *tier
                children=move |tier: Tier| publication_group(tier)
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
                                        <span>{label}<small class="chip-count">{move || n.get().map(fmt::count)}</small></span>
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
                    <p>{format!("Search {} by name or by any word in the text.", fmt::pluralize(total, "entry", "entries"))}</p>
                    <p class="muted">"Try one of these:"</p>
                    <p class="examples">
                        {STARTERS.into_iter().map(|q| {
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
            let all_relations = addon_labels.with(|state| state.clone().flatten());
            let all_groups = option_groups.with(|state| state.clone().flatten());
            let addon_of = |hit: &Hit| -> Option<String> {
                let id = EntryId::parse(hit.id.clone()).ok()?;
                all_relations.as_ref()?.addon_label(&id).map(str::to_owned)
            };
            let group_of = |hit: &Hit| -> Option<String> {
                let id = EntryId::parse(hit.id.clone()).ok()?;
                crate::class_options::group_of(all_groups.as_ref()?, &id)
                    .map(|group| group.singular.clone())
            };
            let items = hits
                .iter()
                .map(|result| {
                    if result.variants.is_empty() {
                        let addon = addon_of(&result.hit);
                        let group = group_of(&result.hit);
                        return hit_view(&lib_for_hits, &result.hit, result.anchor.as_deref(), addon.as_deref(), group.as_deref()).into_any();
                    }
                    let title = marked(&result.hit.title, &result.hit.title_matches);
                    let count = result.variants.len() + 1;
                    let variants = std::iter::once((&result.hit, &result.anchor)).chain(result.variants.iter().map(|(hit, anchor)| (hit, anchor)))
                        .map(|(hit, anchor)| {
                            let addon = addon_of(hit);
                            let group = group_of(hit);
                            hit_view(&lib_for_hits, hit, anchor.as_deref(), addon.as_deref(), group.as_deref())
                        }).collect_view();
                    let summary_kind = group_of(&result.hit).unwrap_or_else(|| {
                        EntryKind::from_slug(&result.hit.kind).map_or("", kind_name).to_string()
                    });
                    view! { <li class="hit hit-group"><details class="source-variants">
                        <summary>
                            {icons::chevron_right("disclosure-chevron")}
                            <span class="hit-text">
                                <span class="hit-title">{title}</span>
                                <span class="hit-meta">{format!("{summary_kind} · {}", fmt::pluralize(count, "publication entry", "publication entries"))}</span>
                            </span>
                        </summary>
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

    // The scope the results are limited to, if exactly one kind is: the count line names it.
    let page_scope =
        Memo::new(move |_| state.with(|s| Scope::of_search(&s.kinds, &s.facets, &s.sources)));
    let count_text = move || match phase.get() {
        Phase::LoadingIndex => "Loading search index…".to_string(),
        Phase::Done { total, groups, .. } => {
            let n = groups;
            let limit = pages.get() * PAGE;
            let text = match (n, n > limit) {
                (0, _) => "No results".to_string(),
                (1, _) => "1 result".to_string(),
                (_, true) => format!(
                    "Top {} of {} results",
                    fmt::count(limit.min(n)),
                    fmt::count(n)
                ),
                _ => format!("{} results", fmt::count(n)),
            };
            let text = match page_scope.get() {
                Some(scope) => scope.results_line(&text),
                None => text,
            };
            if total == groups {
                text
            } else {
                format!(
                    "{text} · {}",
                    fmt::pluralize(total, "publication entry", "publication entries")
                )
            }
        }
        _ => String::new(),
    };
    // A facet narrows the scope below its kind: `Show all feats` widens it back to the kind and
    // keeps the text. A scope of the kind alone has nothing to widen; "All entries" does that.
    let show_all = move || {
        page_scope
            .get()
            .filter(|scope| !scope.facets.is_empty())
            .map(|scope| {
                view! {
                    <span aria-hidden="true">" · "</span>
                    <button type="button" class="count-action"
                        on:click=move |_| change_filters(&|s| {
                            s.facets.clear();
                            s.sources.clear();
                        })>
                        {scope.show_all_label()}
                    </button>
                }
            })
    };

    // Matches that only an inactive book holds. The count is real, but no row shows it, so the
    // hint names the book with the most hidden matches and includes it in one tap. Including a
    // book changes the stored active set only because the reader asked for it here.
    let inactive_hint = move || {
        inactive.get().map(|hidden| {
            let title = hidden.title.clone();
            let source = hidden.source.clone();
            let library = library.clone();
            view! {
                <p class="inactive-books-hint" role="status">
                    <span>{format!(
                        "{} {} in inactive books ({}).",
                        fmt::count(hidden.count),
                        if hidden.count == 1 { "more match" } else { "more matches" },
                        title
                    )}</span>
                    <button class="btn btn-quiet" type="button"
                        on:click=move |_| {
                            let library = library.clone();
                            let source = source.clone();
                            // Keep focus in the page while the hint unmounts.
                            focus_result_count();
                            spawn_local(async move {
                                let _ = library.set_book_active(&source, true).await;
                                js::sleep(0).await;
                                // The results have re-rendered. Keep focus on the status line
                                // instead of dropping it to <body>.
                                focus_result_count();
                            });
                        }>
                        {format!("Turn on {title}")}
                    </button>
                </p>
            }
        })
    };

    // Removing the scope searches everything again. Focus moves to "All entries", which now
    // reads as pressed; the field is left alone so a phone's keyboard does not rise.
    let remove_scope = move |()| {
        change_filters(&|s| {
            s.kinds.clear();
            s.facets.clear();
            s.sources.clear();
        });
        if let Some(button) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("scope-all"))
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = button.focus();
        }
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
                let submitted = text.get_untracked();
                // A search in a scope is remembered with it: the same words mean another list.
                let scope = state.with_untracked(|s| {
                    Scope::of_search(&s.kinds, &s.facets, &s.sources)
                });
                spawn_local(async move {
                    crate::views::recent_searches::record(&submitted, scope).await;
                });
                if !pointer_is_fine()
                    && let Some(input) = input_ref.get()
                {
                    let _ = input.blur();
                }
            }>
                <label class="sr-only" for="search-input">"Search the rules"</label>
                <div class="search-field">
                    {icons::search_field()}
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
                    <Show when=move || text.with(|text| !text.is_empty())>
                        <button class="search-clear" type="button" aria-label="Clear search"
                            on:click=move |_| {
                                text.set(String::new());
                                typing_gen.update_untracked(|g| *g += 1);
                                flush();
                                if let Some(input) = input_ref.get() {
                                    let _ = input.focus();
                                }
                            }>
                            {icons::close()}
                        </button>
                    </Show>
                </div>
            </form>
            // The scope that Quick search applied, or that a kind choice made: one tap removes it.
            {move || state.with(|s| Scope::of_search(&s.kinds, &s.facets, &s.sources)).map(|scope| view! {
                <div class="search-scope" role="group" aria-label="Search scope">
                    <span class="quick-search-scope-label">"Searching in"</span>
                    <ScopePill scope on_remove=Callback::new(remove_scope) />
                </div>
            })}
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
                        {icons::chevron_down("filters-toggle-icon")}
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
                    <p class="result-count" role="status" aria-live="polite" tabindex="-1">
                        <span>{count_text}</span>
                        {show_all}
                        {move || search_ms.get().filter(|_| matches!(phase.get(), Phase::Done { .. })).map(|ms| view! {
                            <span class="ms" aria-hidden="true">{format!(" · {ms:.1} ms")}</span>
                        })}
                    </p>
                    {inactive_hint}
                    {results}
                </section>
            </div>
        </div>
    }
}

/// Loads the index once. With `show`, flags the loading state while waiting. `alive` is checked
/// after each wait, so a task whose screen was disposed stops before it touches a signal.
async fn ensure_index(
    library: &crate::library::Library,
    index: RwSignal<Option<Rc<SearchIndex>>, LocalStorage>,
    index_ms: RwSignal<Option<f64>>,
    phase: RwSignal<Phase>,
    show: bool,
    alive: impl Fn() -> bool,
) -> Option<Rc<SearchIndex>> {
    if let Some(idx) = index.get_untracked() {
        return Some(idx);
    }
    if show {
        phase.set(Phase::LoadingIndex);
        // Let the browser paint the message before the index decodes on this thread.
        js::sleep(0).await;
        if !alive() {
            return None;
        }
    }
    let started = now();
    let loaded = library.search_index().await;
    if !alive() {
        return None;
    }
    match loaded {
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

/// Focus the results status line. The inactive-book hint unmounts after its action, so the
/// next focus target has to be found after the render.
fn focus_result_count() {
    if let Some(status) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector(".result-count").ok().flatten())
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = status.focus();
    }
}

fn hit_view(
    library: &crate::library::Library,
    hit: &Hit,
    anchor: Option<&str>,
    addon: Option<&str>,
    group: Option<&str>,
) -> impl IntoView + use<> {
    let entry_id = EntryId::parse(hit.id.clone()).ok();
    let href = entry_id
        .as_ref()
        .map(|id| {
            Route::Entry {
                id: id.clone(),
                anchor: anchor.map(str::to_owned),
            }
            .href()
        })
        .unwrap_or_else(|| Route::Home.href());
    let kind = EntryKind::from_slug(&hit.kind).map_or("", kind_name);
    // An add-on row names its label where the kind would stand: `Mythic · Mythic Adventures`.
    // A class option names its group: `Arcane discovery · Ultimate Magic`.
    let row_kind = match (addon, group) {
        (Some(label), _) => addon_name(label).to_string(),
        (None, Some(group)) => group.to_string(),
        (None, None) => kind.to_string(),
    };
    // The index records one attribution source. When it is inactive, the entry's book placement
    // is the source that kept the entry eligible, so it is the one the reader should see.
    // A search row only has the catalog stub, which carries one source, so its alternative set
    // is the canonical placement. The entry header uses the same `preferred_source` rule over
    // every `source_ids` entry because it has the full entry. The rule is one; the data differs.
    // See docs/decisions.md, "Inactive books in search".
    let canonical = entry_id
        .as_ref()
        .and_then(|id| library.canonical_source_for_entry(id));
    let source_id = preferred_source(
        &hit.source,
        canonical.as_deref(),
        |id| library.book_active(id),
        |id| library.source(id).map_or(Tier::Other, Tier::of),
    );
    let source = library
        .source(source_id)
        .map_or_else(|| source_id.to_string(), |s| s.title.clone());
    let title = marked(&hit.title, &hit.title_matches);
    let snippet = marked(&hit.snippet.text, &hit.snippet.matches);
    let has_snippet = !hit.snippet.text.is_empty();
    view! {
        <li class="hit">
            <a class="hit-link" href=href>
                <span class="hit-title">{title}</span>
                <span class="hit-meta">{row_kind}" · "{source}</span>
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
    fn a_list_page_keeps_the_facets_its_kind_offers_one_value_per_field() {
        let facet = |f: &str, v: &str| (f.to_string(), v.to_string());
        let url = [
            facet("feat_type", "Combat"),
            facet("school", "evocation"),
            facet("feat_type", "General"),
        ];
        assert_eq!(
            list_facets(&url, EntryKind::Feat),
            [facet("feat_type", "Combat")]
        );
        assert_eq!(
            list_facets(&url, EntryKind::Spell),
            [facet("school", "evocation")]
        );
        assert!(list_facets(&url, EntryKind::Trap).is_empty());
    }

    #[test]
    fn facet_labels_read_well_and_leave_values_alone() {
        assert_eq!(facet_label("race_group", "monster_race"), "Monster races");
        assert_eq!(facet_label("class_type", "npc"), "NPC");
        assert_eq!(
            facet_label("creature_type", "magicalBeast"),
            "Magical Beast"
        );
        assert_eq!(
            facet_label("creature_type", "monstrousHumanoid"),
            "Monstrous Humanoid"
        );
        // Every lowercase creature type reads as a title, not as its index value.
        for (value, label) in [
            ("aberration", "Aberration"),
            ("animal", "Animal"),
            ("construct", "Construct"),
            ("dragon", "Dragon"),
            ("fey", "Fey"),
            ("humanoid", "Humanoid"),
            ("ooze", "Ooze"),
            ("outsider", "Outsider"),
            ("plant", "Plant"),
            ("undead", "Undead"),
            ("vermin", "Vermin"),
        ] {
            assert_eq!(facet_label("creature_type", value), label);
        }
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
        // Spell level 0 reads as the word the rest of the app uses; other values stay numeric.
        assert_eq!(facet_label("spell_level", "0"), "Cantrips");
        assert_eq!(facet_label("spell_level", "5"), "5");
    }

    #[test]
    fn facet_pairs_sort_with_their_counts() {
        let mut pairs: Vec<(String, u32)> =
            vec![("10".into(), 1), ("2".into(), 5), ("1/2".into(), 7)];
        sort_facet_pairs("cr", &mut pairs);
        assert_eq!(pairs[0], ("1/2".to_string(), 7));
        assert_eq!(pairs[2], ("10".to_string(), 1));
    }

    #[test]
    fn challenge_ratings_sort_numerically_from_eighths_to_thirty() {
        // The corpus order is alphabetical (1/2, 1/3, 1/4, 1/6, 1/8, 1, 10, 2), which would
        // put a tenth above a second and every fraction before the whole numbers.
        let mut pairs: Vec<(String, u32)> = [
            "30", "3", "1/8", "20", "1/2", "10", "1/6", "2", "1/4", "1", "1/3", "12",
        ]
        .into_iter()
        .map(|value| (value.to_string(), 1))
        .collect();
        sort_facet_pairs("cr", &mut pairs);
        let order: Vec<&str> = pairs.iter().map(|(value, _)| value.as_str()).collect();
        assert_eq!(
            order,
            [
                "1/8", "1/6", "1/4", "1/3", "1/2", "1", "2", "3", "10", "12", "20", "30"
            ]
        );
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
        assert_eq!(chip_name("Feats", 1324), "Feats, 1,324 entries");
    }

    #[test]
    fn a_search_row_names_the_canonical_book_when_the_indexed_source_is_inactive() {
        let tier = |id: &str| match id {
            "core" => Tier::Rulebooks,
            _ => Tier::Other,
        };
        let active = |id: &str| id == "core";
        assert_eq!(
            preferred_source("inactive", Some("core"), active, tier),
            "core"
        );
        // Without a canonical alternative the row keeps the indexed source.
        assert_eq!(preferred_source("inactive", None, active, tier), "inactive");
    }

    #[test]
    fn hidden_matches_count_only_inactive_entries_by_book() {
        let hits = [
            summary("a", "core"),
            summary("b", "bestiary"),
            summary("c", "bestiary"),
            summary("d", "um"),
        ];
        let counts = inactive_match_counts(&hits, |id| id == "a", |hit| hit.source.clone());
        assert_eq!(counts.get("bestiary"), Some(&2));
        assert_eq!(counts.get("um"), Some(&1));
        assert_eq!(
            inactive_matches(
                &hits,
                |id| id == "a",
                |hit| hit.source.clone(),
                |_| Tier::Other
            ),
            Some((3, "bestiary".to_string()))
        );
        // With every entry eligible there is nothing to say.
        assert_eq!(
            inactive_matches(&hits, |_| true, |hit| hit.source.clone(), |_| Tier::Other),
            None
        );
    }

    #[test]
    fn an_inactive_tie_prefers_the_higher_tier_then_the_source_id() {
        // “core” (Rulebooks) ties “ap” (Adventures); the higher tier wins over the id order.
        let hits = [summary("a", "ap"), summary("b", "core")];
        let hidden = |_: &str| false;
        let tier = |id: &str| match id {
            "core" => Tier::Rulebooks,
            "ap" => Tier::Adventures,
            _ => Tier::Other,
        };
        assert_eq!(
            inactive_matches(&hits, hidden, |hit| hit.source.clone(), tier),
            Some((2, "core".to_string()))
        );
        // In one tier the source id ascending decides.
        let hits = [summary("a", "zeta"), summary("b", "alpha")];
        assert_eq!(
            inactive_matches(&hits, hidden, |hit| hit.source.clone(), |_| Tier::Other),
            Some((2, "alpha".to_string()))
        );
    }

    fn summary(id: &str, source: &str) -> HitSummary {
        HitSummary {
            ord: 0,
            id: id.into(),
            title: String::new(),
            kind: "rule".into(),
            source: source.into(),
            score: 0.0,
        }
    }

    fn source(id: &str, title: &str) -> SourceInfo {
        SourceInfo {
            id: id.into(),
            title: title.into(),
            entry_count: 0,
            copyright: None,
            category: None,
            publication_date: None,
            third_party: false,
        }
    }

    fn page_of(
        sources: &[SourceInfo],
        counts: &[(&str, usize)],
        selected: &[&str],
        filter: &str,
        show_all: bool,
    ) -> PublicationPage {
        page_choosing(sources, counts, selected, filter, show_all, &[])
    }

    /// A page where the reader has opened (`true`) or closed (`false`) some tiers.
    fn page_choosing(
        sources: &[SourceInfo],
        counts: &[(&str, usize)],
        selected: &[&str],
        filter: &str,
        show_all: bool,
        choices: &[(Tier, bool)],
    ) -> PublicationPage {
        let selected: Vec<String> = selected.iter().map(|id| (*id).to_string()).collect();
        let choices: BTreeMap<Tier, bool> = choices.iter().copied().collect();
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
            &choices,
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
        assert_eq!(page.ids(), ["a", "c"]);
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
        assert_eq!(page.ids().len(), PUBLICATION_TOP);
        assert_eq!(page.ids()[0], "s19");
        assert_eq!(page.ids()[PUBLICATION_TOP - 1], "s08");
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
        assert_eq!(page.ids()[0], "b");
        assert_eq!(page.ids(), ["b", "a", "c"]);
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
        assert_eq!(page.ids(), ["apg"]);
        assert_eq!(page.matches, 1);
        assert_eq!(page.more, 0);
        let none = page_of(&sources, &counts, &[], "bestiary", false);
        assert!(
            none.ids().is_empty(),
            "a zero-count publication stays hidden"
        );
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
        assert_eq!(page.ids().len(), 20);
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
        assert_eq!(page.ids(), ["z", "a"]);
        assert_eq!(page.with_results, 2);
    }

    #[test]
    fn a_pinned_publication_outside_the_name_box_is_not_counted() {
        let sources = [source("a", "Alpha"), source("b", "Beta")];
        let counts = [("a", 5), ("b", 0)];
        let page = page_of(&sources, &counts, &["b"], "alpha", false);
        // Beta stays visible so it can be cleared, but the name box has one match, not zero.
        assert_eq!(page.ids(), ["b", "a"]);
        assert_eq!(page.matches, 1);
        assert_eq!(
            publication_summary_line(&page, "alpha", false),
            "Showing the one matching publication with results."
        );
        let none = page_of(&sources, &counts, &["b"], "gamma", false);
        assert_eq!(none.matches, 0);
        assert_eq!(
            publication_summary_line(&none, "gamma", false),
            "No publication matches “gamma”."
        );
    }

    #[test]
    fn the_summary_counts_what_is_visible() {
        let sources: Vec<SourceInfo> = (0..20)
            .map(|n| source(&format!("s{n:02}"), &format!("Source {n:02}")))
            .collect();
        let counts: Vec<(&str, usize)> = sources
            .iter()
            .enumerate()
            .map(|(n, source)| (source.id.as_str(), n + 1))
            .collect();
        let page = page_of(&sources, &counts, &[], "", false);
        assert_eq!(
            publication_summary_line(&page, "", false),
            "Showing 12 of 20 publications with results."
        );
        // The wording follows the page it describes: a page with chips left out never says "all".
        let all = page_of(&sources, &counts, &[], "", true);
        assert_eq!(
            publication_summary_line(&all, "", true),
            "Showing all 20 publications with results."
        );
    }

    fn in_tier(id: &str, title: &str, category: &str) -> SourceInfo {
        SourceInfo {
            category: Some(category.into()),
            ..source(id, title)
        }
    }

    fn group_ids(page: &PublicationPage, tier: Tier) -> Vec<&str> {
        page.group(tier)
            .map(|group| group.ids.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    #[test]
    fn groups_follow_the_tier_order_and_skip_empty_ones() {
        let sources = [
            in_tier("ap", "Adventure", "Adventure Path"),
            in_tier("misc", "Odds and Ends", "Miscellaneous"),
            in_tier("setting", "Setting", "Campaign Setting"),
            in_tier("core", "Core", "RPG"),
            in_tier("pc", "Companion", "Player Companion"),
            source("none", "No category"),
            in_tier("quiet", "Quiet Rulebook", "RPG"),
        ];
        let counts = [
            ("ap", 9),
            ("misc", 1),
            ("setting", 2),
            ("core", 3),
            ("pc", 4),
            ("none", 6),
            ("quiet", 0),
        ];
        let page = page_of(&sources, &counts, &[], "", false);
        let tiers: Vec<Tier> = page.groups.iter().map(|group| group.tier).collect();
        assert_eq!(
            tiers,
            [
                Tier::Rulebooks,
                Tier::PlayerCompanions,
                Tier::CampaignSetting,
                Tier::Adventures,
                Tier::Other
            ]
        );
        // The publication with no category is Other, after Miscellaneous only by count.
        assert_eq!(group_ids(&page, Tier::Other), ["none", "misc"]);
        // Adventures are closed by default and take no budget, but the group is present.
        assert_eq!(page.ids(), ["core", "pc", "setting", "none", "misc"]);
        assert!(group_ids(&page, Tier::Adventures).is_empty());
        assert_eq!(page.group(Tier::Adventures).unwrap().with_results, 1);
        let only_rulebooks = page_of(&sources, &[("core", 3)], &[], "", false);
        assert_eq!(only_rulebooks.groups.len(), 1, "empty groups are absent");
    }

    #[test]
    fn a_marked_book_is_its_own_group_after_every_paizo_tier() {
        let sources = [
            in_tier("core", "Core", "RPG"),
            SourceInfo {
                id: "indie".into(),
                title: "Indie".into(),
                entry_count: 0,
                copyright: None,
                // The marker wins over a Paizo category.
                category: Some("RPG".into()),
                publication_date: None,
                third_party: true,
            },
        ];
        let counts = [("core", 3), ("indie", 2)];
        let page = page_of(&sources, &counts, &[], "", true);
        let tiers: Vec<Tier> = page.groups.iter().map(|group| group.tier).collect();
        assert_eq!(tiers, [Tier::Rulebooks, Tier::ThirdParty]);
        assert_eq!(group_ids(&page, Tier::ThirdParty), ["indie"]);
        assert_eq!(
            page.group(Tier::ThirdParty).unwrap().heading(),
            "Third party (1)"
        );
    }

    #[test]
    fn a_selected_publication_leads_its_own_tier_only() {
        let sources = [
            in_tier("big", "Big Rulebook", "RPG"),
            in_tier("small", "Small Rulebook", "RPG"),
            in_tier("pc", "Companion", "Player Companion"),
        ];
        let counts = [("big", 9), ("small", 1), ("pc", 5)];
        let page = page_of(&sources, &counts, &["small"], "", false);
        assert_eq!(group_ids(&page, Tier::Rulebooks), ["small", "big"]);
        assert_eq!(group_ids(&page, Tier::PlayerCompanions), ["pc"]);
        // A selection at zero is pinned inside its tier, and it does not count as a result.
        let pinned = page_of(&sources, &[("big", 9)], &["pc"], "", false);
        assert_eq!(group_ids(&pinned, Tier::PlayerCompanions), ["pc"]);
        assert_eq!(
            pinned.group(Tier::PlayerCompanions).unwrap().with_results,
            0
        );
        assert_eq!(pinned.shown_with_results, 1);
    }

    fn many(prefix: &str, category: &str, n: usize) -> Vec<SourceInfo> {
        (0..n)
            .map(|i| {
                in_tier(
                    &format!("{prefix}{i:02}"),
                    &format!("{prefix} {i:02}"),
                    category,
                )
            })
            .collect()
    }

    fn ranked_counts(sources: &[SourceInfo], base: usize) -> Vec<(&str, usize)> {
        sources
            .iter()
            .enumerate()
            .map(|(i, source)| (source.id.as_str(), base + i))
            .collect()
    }

    #[test]
    fn the_limit_is_spent_in_group_order_so_a_rulebook_is_never_hidden_for_a_lower_tier() {
        // Fifteen rulebooks with few results and ten companions with many: counts alone
        // would put the companions first.
        let mut sources = many("rb", "RPG", 15);
        sources.extend(many("pc", "Player Companion", 10));
        let mut counts = ranked_counts(&sources[..15], 1);
        counts.extend(ranked_counts(&sources[15..], 500));
        let page = page_of(&sources, &counts, &[], "", false);
        assert_eq!(page.ids().len(), PUBLICATION_TOP);
        assert_eq!(group_ids(&page, Tier::Rulebooks).len(), PUBLICATION_TOP);
        // The companions have no chip, but their group is there to say how many they hold.
        let companions = page.group(Tier::PlayerCompanions).expect("a heading");
        assert!(companions.ids.is_empty());
        assert_eq!(companions.with_results, 10);
        assert!(!companions.open, "a group with no chip starts closed");
        assert_eq!(companions.heading(), "Player Companions (10)");
        assert_eq!(page.more, 25 - PUBLICATION_TOP);
        // The largest rulebooks come first inside the group.
        assert_eq!(group_ids(&page, Tier::Rulebooks)[0], "rb14");
        // With room left over, the next tier fills the rest of the budget.
        let mut few = ranked_counts(&sources[..5], 1);
        few.extend(ranked_counts(&sources[15..], 500));
        let page = page_of(&sources, &few, &[], "", false);
        assert_eq!(group_ids(&page, Tier::Rulebooks).len(), 5);
        assert_eq!(group_ids(&page, Tier::PlayerCompanions).len(), 7);
        assert_eq!(page.more, 3);
        // Open, with seven of ten listed, the heading says so.
        let companions = page.group(Tier::PlayerCompanions).unwrap();
        assert!(companions.open);
        assert_eq!(companions.heading(), "Player Companions (7 of 10)");
    }

    #[test]
    fn adventures_take_no_budget_and_the_tier_after_them_uses_it() {
        let mut sources = many("rb", "RPG", 4);
        sources.extend(many("ap", "Adventure Path", 10));
        sources.extend(many("mod", "Module", 5));
        sources.extend(many("misc", "Miscellaneous", 12));
        let counts = ranked_counts(&sources, 1);
        let page = page_of(&sources, &counts, &[], "", false);
        assert_eq!(group_ids(&page, Tier::Rulebooks).len(), 4);
        // A closed disclosure would hide the chips, so the budget goes to the next tier.
        let adventures = page.group(Tier::Adventures).expect("adventures group");
        assert!(adventures.ids.is_empty());
        assert_eq!(adventures.with_results, 15);
        assert_eq!(adventures.heading(), "Adventures (15)");
        assert_eq!(group_ids(&page, Tier::Other).len(), PUBLICATION_TOP - 4);
        assert_eq!(page.more, 31 - PUBLICATION_TOP);
        // "Show all" lists them and opens the group, so nothing it lists is hidden.
        let all = page_of(&sources, &counts, &[], "", true);
        let adventures = all.group(Tier::Adventures).unwrap();
        assert_eq!(adventures.ids.len(), 15);
        assert!(adventures.open);
        assert_eq!(all.shown_with_results, 31);
        assert_eq!(all.more, 0);
    }

    #[test]
    fn a_selected_publication_is_shown_even_when_it_is_beyond_the_limit() {
        let mut sources = many("rb", "RPG", 15);
        sources.extend(many("pc", "Player Companion", 3));
        let counts = ranked_counts(&sources, 1);
        // pc02 is the largest companion, rb14 the largest rulebook.
        let page = page_of(&sources, &counts, &["pc00"], "", false);
        let rulebooks = group_ids(&page, Tier::Rulebooks);
        // The selection spends one slot of the budget, so eleven rulebooks remain.
        assert_eq!(rulebooks.len(), PUBLICATION_TOP - 1);
        assert_eq!(group_ids(&page, Tier::PlayerCompanions), ["pc00"]);
        assert_eq!(page.ids().len(), PUBLICATION_TOP);
    }

    /// Fifteen rulebooks that spend the whole budget, then companions, a setting and
    /// adventures, none of which gets a chip.
    fn broad_query() -> (Vec<SourceInfo>, Vec<(String, usize)>) {
        let mut sources = many("rb", "RPG", 15);
        sources.extend(many("pc", "Player Companion", 6));
        sources.extend(many("cs", "Campaign Setting", 3));
        sources.extend(many("ap", "Adventure Path", 8));
        sources.extend(many("misc", "Miscellaneous", 2));
        let counts = sources
            .iter()
            .enumerate()
            .map(|(i, source)| (source.id.clone(), i + 1))
            .collect();
        (sources, counts)
    }

    fn broad_page(
        selected: &[&str],
        filter: &str,
        show_all: bool,
        choices: &[(Tier, bool)],
    ) -> PublicationPage {
        let (sources, counts) = broad_query();
        let counts: Vec<(&str, usize)> = counts.iter().map(|(id, n)| (id.as_str(), *n)).collect();
        page_choosing(&sources, &counts, selected, filter, show_all, choices)
    }

    #[test]
    fn a_broad_query_shows_rulebook_chips_and_a_closed_heading_for_every_other_tier() {
        let page = broad_page(&[], "", false, &[]);
        let tiers: Vec<Tier> = page.groups.iter().map(|group| group.tier).collect();
        // This library has no third-party book, so that group is absent.
        assert_eq!(tiers, Tier::ALL[..5].to_vec());
        let headings: Vec<String> = page.groups.iter().map(PublicationGroup::heading).collect();
        assert_eq!(
            headings,
            [
                "Rulebooks (12 of 15)",
                "Player Companions (6)",
                "Campaign Setting (3)",
                "Adventures (8)",
                "Other (2)",
            ]
        );
        assert_eq!(group_ids(&page, Tier::Rulebooks).len(), PUBLICATION_TOP);
        for tier in &Tier::ALL[1..5] {
            let group = page.group(*tier).unwrap();
            assert!(group.ids.is_empty(), "{tier:?} has no chip");
            assert!(!group.open, "{tier:?} starts closed");
        }
        // The rule holds: nothing from a lower tier shows while a Rulebook is hidden.
        assert_eq!(page.ids().len(), PUBLICATION_TOP);
        assert!(page.ids().iter().all(|id| id.starts_with("rb")));
        // The numbers reconcile: shown plus the rest is every publication with results.
        assert_eq!(page.with_results, 34);
        assert_eq!(page.shown_with_results, PUBLICATION_TOP);
        assert_eq!(page.more, 34 - PUBLICATION_TOP);
        let held: usize = page.groups.iter().map(|group| group.with_results).sum();
        assert_eq!(held, page.with_results);
        assert_eq!(
            publication_summary_line(&page, "", false),
            "Showing 12 of 34 publications with results."
        );
    }

    #[test]
    fn a_tier_with_no_result_stays_hidden() {
        let (sources, mut counts) = broad_query();
        for (id, n) in &mut counts {
            if id.starts_with("cs") || id.starts_with("misc") {
                *n = 0;
            }
        }
        let counts: Vec<(&str, usize)> = counts.iter().map(|(id, n)| (id.as_str(), *n)).collect();
        let page = page_of(&sources, &counts, &[], "", false);
        let tiers: Vec<Tier> = page.groups.iter().map(|group| group.tier).collect();
        assert_eq!(
            tiers,
            [Tier::Rulebooks, Tier::PlayerCompanions, Tier::Adventures]
        );
    }

    #[test]
    fn opening_a_tier_lists_all_of_its_publications_without_show_all() {
        let page = broad_page(&[], "", false, &[(Tier::PlayerCompanions, true)]);
        let companions = page.group(Tier::PlayerCompanions).unwrap();
        assert!(companions.open);
        assert_eq!(companions.ids.len(), 6);
        assert_eq!(companions.heading(), "Player Companions (6)");
        // The other tiers and the Rulebook budget are untouched.
        assert_eq!(group_ids(&page, Tier::Rulebooks).len(), PUBLICATION_TOP);
        assert!(page.group(Tier::Adventures).unwrap().ids.is_empty());
        assert_eq!(page.shown_with_results, PUBLICATION_TOP + 6);
        assert_eq!(page.more, 34 - PUBLICATION_TOP - 6);
        assert_eq!(
            publication_summary_line(&page, "", false),
            "Showing 18 of 34 publications with results."
        );
        // Closing it again hides the chips from the count.
        let closed = broad_page(&[], "", false, &[(Tier::PlayerCompanions, false)]);
        assert_eq!(closed.shown_with_results, PUBLICATION_TOP);
        // Rulebooks have no disclosure: a stored answer does not change them.
        let stale = broad_page(&[], "", false, &[(Tier::Rulebooks, true)]);
        assert_eq!(group_ids(&stale, Tier::Rulebooks).len(), PUBLICATION_TOP);
    }

    #[test]
    fn chips_in_a_closed_disclosure_are_not_counted_as_shown() {
        let sources = [
            in_tier("core", "Core", "RPG"),
            in_tier("pc", "Companion", "Player Companion"),
            in_tier("pc2", "Another", "Player Companion"),
        ];
        let counts = [("core", 3), ("pc", 2), ("pc2", 1)];
        let open = page_of(&sources, &counts, &[], "", false);
        assert_eq!(open.shown_with_results, 3);
        assert_eq!(open.more, 0);
        // The reader closes the companions: their chips stay mounted but are not shown.
        let closed = page_choosing(
            &sources,
            &counts,
            &[],
            "",
            false,
            &[(Tier::PlayerCompanions, false)],
        );
        assert_eq!(group_ids(&closed, Tier::PlayerCompanions).len(), 2);
        assert_eq!(closed.shown_with_results, 1);
        assert_eq!(closed.more, 2);
        assert_eq!(
            closed.group(Tier::PlayerCompanions).unwrap().heading(),
            "Player Companions (2)"
        );
        assert_eq!(
            publication_summary_line(&closed, "", false),
            "Showing 1 of 3 publications with results."
        );
        // "Show all" does not claim everything while a group the reader closed hides chips.
        let all_closed = page_choosing(
            &sources,
            &counts,
            &[],
            "",
            true,
            &[(Tier::PlayerCompanions, false)],
        );
        assert_eq!(
            publication_summary_line(&all_closed, "", true),
            "Showing 1 of 3 publications with results."
        );
        assert_eq!(all_closed.more, 0, "the button reads 'Show fewer'");
    }

    #[test]
    fn show_all_lists_and_opens_every_tier() {
        let page = broad_page(&[], "", true, &[]);
        assert!(page.groups.iter().all(|group| group.open));
        assert_eq!(page.shown_with_results, 34);
        assert_eq!(page.more, 0);
        assert_eq!(
            publication_summary_line(&page, "", true),
            "Showing all 34 publications with results."
        );
        let headings: Vec<String> = page.groups.iter().map(PublicationGroup::heading).collect();
        assert_eq!(headings[0], "Rulebooks (15)");
        // A tier's own answer still wins.
        let reader = broad_page(&[], "", true, &[(Tier::Adventures, false)]);
        assert!(!reader.group(Tier::Adventures).unwrap().open);
    }

    #[test]
    fn a_selected_publication_forces_its_tier_open() {
        let page = broad_page(&["ap03"], "", false, &[(Tier::Adventures, false)]);
        let adventures = page.group(Tier::Adventures).unwrap();
        assert!(adventures.holds_selected);
        assert!(adventures.open, "a selection overrides the reader's answer");
        assert_eq!(adventures.ids, ["ap03"]);
        assert_eq!(adventures.heading(), "Adventures (1 of 8)");
        // The selection spends one slot of the budget, as before.
        assert_eq!(group_ids(&page, Tier::Rulebooks).len(), PUBLICATION_TOP - 1);
        // Opened by the reader, the tier lists everything and the selection spends no slot.
        let opened = broad_page(&["ap03"], "", false, &[(Tier::Adventures, true)]);
        assert_eq!(opened.group(Tier::Adventures).unwrap().ids.len(), 8);
        assert_eq!(group_ids(&opened, Tier::Rulebooks).len(), PUBLICATION_TOP);
        // A selection at zero still holds the group open.
        let sources = [
            in_tier("core", "Core", "RPG"),
            in_tier("ap", "Adventure", "Adventure Path"),
        ];
        let zero = page_of(&sources, &[("core", 3)], &["ap"], "", false);
        let adventures = zero.group(Tier::Adventures).unwrap();
        assert_eq!(adventures.with_results, 0);
        assert!(adventures.open);
    }

    #[test]
    fn the_name_box_opens_every_tier_it_matches() {
        let page = broad_page(&[], "ap 0", false, &[(Tier::Adventures, false)]);
        let adventures = page.group(Tier::Adventures).unwrap();
        assert!(adventures.open);
        assert_eq!(adventures.ids.len(), 8);
        assert!(page.group(Tier::PlayerCompanions).is_none());
        assert_eq!(page.more, 0);
    }

    #[test]
    fn the_name_box_lifts_the_limit_in_every_group() {
        let mut sources = many("rb", "RPG", 20);
        sources.extend(many("pc", "Player Companion", 20));
        let counts = ranked_counts(&sources, 1);
        let page = page_of(&sources, &counts, &[], "0", false);
        // Titles containing "0": rb 00-09, rb 10, 20.. and so on; the point is no limit.
        assert!(page.ids().len() > PUBLICATION_TOP);
        assert_eq!(page.more, 0);
        assert_eq!(page.ids().len(), page.matches);
    }

    #[test]
    fn a_close_click_while_the_name_box_forces_a_tier_open_records_no_choice() {
        // The steps of #141: the reader opens a tier, then the name box forces it open, then a
        // close click is answered by reopening and must not replace the earlier answer.
        let action = publication_toggle_action(false, true, true, false);
        assert_eq!(action, ToggleAction::ForceOpen);
        let sources = [
            in_tier("core", "Core", "RPG"),
            in_tier("pc", "Companion", "Player Companion"),
        ];
        let counts = [("core", 3), ("pc", 2)];
        let opened = page_choosing(
            &sources,
            &counts,
            &[],
            "",
            false,
            &[(Tier::PlayerCompanions, true)],
        );
        assert!(opened.group(Tier::PlayerCompanions).unwrap().open);
        // The box that forced it open is gone; the stored open answer still applies.
        let restored = page_choosing(
            &sources,
            &counts,
            &[],
            "",
            false,
            &[(Tier::PlayerCompanions, true)],
        );
        assert!(restored.group(Tier::PlayerCompanions).unwrap().open);
        // A close click with no force is the reader's choice and is recorded.
        assert_eq!(
            publication_toggle_action(false, false, true, false),
            ToggleAction::Record(false)
        );
        // A selection behaves the same way as the name box.
        assert_eq!(
            publication_toggle_action(true, false, true, false),
            ToggleAction::ForceOpen
        );
    }

    #[test]
    fn an_adventures_only_page_opens_adventures_and_spends_the_budget() {
        let sources = many("ap", "Adventure Path", 15);
        let counts = ranked_counts(&sources, 1);
        let page = page_of(&sources, &counts, &[], "", false);
        let adventures = page.group(Tier::Adventures).expect("an adventures group");
        assert!(adventures.open, "the only tier with results opens");
        assert_eq!(adventures.ids.len(), PUBLICATION_TOP);
        assert_eq!(adventures.heading(), "Adventures (12 of 15)");
        assert_eq!(page.shown_with_results, PUBLICATION_TOP);
        assert_eq!(page.more, 15 - PUBLICATION_TOP);
        assert_eq!(
            publication_summary_line(&page, "", false),
            "Showing 12 of 15 publications with results."
        );
    }

    #[test]
    fn a_reader_who_closed_adventures_keeps_them_closed_on_an_adventures_only_page() {
        let sources = many("ap", "Adventure Path", 15);
        let counts = ranked_counts(&sources, 1);
        let page = page_choosing(
            &sources,
            &counts,
            &[],
            "",
            false,
            &[(Tier::Adventures, false)],
        );
        let adventures = page.group(Tier::Adventures).unwrap();
        assert!(!adventures.open);
        assert!(adventures.ids.is_empty());
        assert_eq!(page.shown_with_results, 0);
        assert_eq!(page.more, 15);
    }

    #[test]
    fn a_third_party_only_page_opens_third_party_and_spends_the_budget() {
        let mut sources = many("tp", "RPG", 15);
        for source in &mut sources {
            source.third_party = true;
        }
        let counts = ranked_counts(&sources, 1);
        let page = page_of(&sources, &counts, &[], "", false);
        let third_party = page.group(Tier::ThirdParty).expect("a third-party group");
        assert!(third_party.open, "the only tier with results opens");
        assert_eq!(third_party.ids.len(), PUBLICATION_TOP);
        assert_eq!(third_party.heading(), "Third party (12 of 15)");
        assert_eq!(page.shown_with_results, PUBLICATION_TOP);
        // A reader who closed the group keeps it closed.
        let page = page_choosing(
            &sources,
            &counts,
            &[],
            "",
            false,
            &[(Tier::ThirdParty, false)],
        );
        let third_party = page.group(Tier::ThirdParty).unwrap();
        assert!(!third_party.open);
        assert!(third_party.ids.is_empty());
    }

    #[test]
    fn adventures_stay_collapsed_and_take_no_budget_when_another_tier_has_results() {
        let mut sources = many("rb", "RPG", 3);
        sources.extend(many("ap", "Adventure Path", 10));
        let counts = ranked_counts(&sources, 1);
        let page = page_of(&sources, &counts, &[], "", false);
        let adventures = page.group(Tier::Adventures).unwrap();
        assert!(!adventures.open);
        assert!(adventures.ids.is_empty(), "no chip sits in a closed tier");
        // The rulebooks keep the whole budget, as before.
        assert_eq!(group_ids(&page, Tier::Rulebooks).len(), 3);
        assert_eq!(page.more, 10);
    }

    #[test]
    fn the_summary_line_counts_what_the_groups_show() {
        let mut sources = many("rb", "RPG", 9);
        sources.extend(many("misc", "Miscellaneous", 11));
        let counts = ranked_counts(&sources, 1);
        let page = page_of(&sources, &counts, &[], "", false);
        // Nine rulebooks and three others, of twenty.
        assert_eq!(page.shown_with_results, PUBLICATION_TOP);
        assert_eq!(page.with_results, 20);
        assert_eq!(page.more, 20 - PUBLICATION_TOP);
        assert_eq!(
            publication_summary_line(&page, "", false),
            "Showing 12 of 20 publications with results."
        );
        let all = page_of(&sources, &counts, &[], "", true);
        assert_eq!(
            publication_summary_line(&all, "", true),
            "Showing all 20 publications with results."
        );
        // A pinned selection at zero is not counted, so the line never reads "1 of 0".
        let pinned = page_of(&sources, &[], &["rb00"], "", false);
        assert_eq!(pinned.shown_with_results, 0);
        assert_eq!(pinned.with_results, 0);
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
