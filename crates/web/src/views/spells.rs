//! Spell lists: every class with its levels (the hub), and the list of one class, whole or by
//! level. The data is `lists.json`, an index of `SpellFields::levels`.

use crate::app::use_app;
use crate::density;
use crate::fmt;
use crate::graph::{ListedSpell, filter_list, find_list, level_counts, list_schools, list_total};
use crate::library::{Library, LibraryError, use_library};
use crate::route::{Route, push, replace, use_route};
use crate::views::book_tier::Tier;
use crate::views::categories;
use crate::views::filter_menu::{
    FilterMenuButton, MenuState, MultiFilterMenuPanel, MultiOption, multi_trigger_text,
};
use crate::views::fresh::IntoFresh as _;
use crate::views::icons;
use crate::views::util::{Shared, local_resource, set_title};
use cast_iron_schema::{EntryKind, SpellListClass, SpellListDetail, SpellLists};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// The classes that most players look for, in the order that the picker lists them. Names
/// compare without regard to case.
const COMMON_CLASSES: [&str; 7] = [
    "wizard", "sorcerer", "cleric", "druid", "bard", "ranger", "paladin",
];

/// The class picker's two groups: the common classes that the library lists, in
/// [`COMMON_CLASSES`] order, then every other list by name.
pub fn picker_groups(lists: &SpellLists) -> (Vec<&SpellListClass>, Vec<&SpellListClass>) {
    let common: Vec<&SpellListClass> = COMMON_CLASSES
        .iter()
        .filter_map(|name| {
            lists
                .classes
                .iter()
                .find(|l| l.name.to_lowercase() == *name)
        })
        .collect();
    let mut rest: Vec<&SpellListClass> = lists
        .classes
        .iter()
        .filter(|l| !COMMON_CLASSES.contains(&l.name.to_lowercase().as_str()))
        .collect();
    cast_iron_sort::sort_by_key(&mut rest, |l| l.name.to_lowercase());
    (common, rest)
}

/// `evocation` as `Evocation`.
fn school_label(school: &str) -> String {
    let mut chars = school.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// What a part of the second line is, which decides how it gives way on a narrow screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Casting time or saving throw: short, and always whole.
    Fact,
    /// The first sentence of the description: it ends in an ellipsis when space runs out.
    Gist,
    /// The book: whole up to a share of the line.
    Book,
}

impl Part {
    fn class(self) -> &'static str {
        match self {
            Part::Fact => "spell-fact",
            Part::Gist => "spell-fact spell-gist",
            Part::Book => "spell-fact spell-book",
        }
    }
}

/// The parts of the second line, in reading order: casting time, saving throw, the gist of the
/// description, the book. A missing part is left out.
pub fn meta_parts(detail: Option<&SpellListDetail>, book: Option<&str>) -> Vec<(Part, String)> {
    let mut parts: Vec<(Part, String)> = Vec::new();
    if let Some(d) = detail {
        for (part, text) in [
            (Part::Fact, &d.casting_time),
            (Part::Fact, &d.saving_throw),
            (Part::Gist, &d.gist),
        ] {
            if !text.is_empty() {
                parts.push((part, text.clone()));
            }
        }
    }
    if let Some(book) = book.filter(|b| !b.is_empty()) {
        parts.push((Part::Book, book.to_string()));
    }
    parts
}

/// Keeps the order inside each tier and puts Rulebooks first, then the other tiers in
/// [`Tier::ALL`] order. A tier with no spell is left out.
pub fn by_tier<T>(rows: Vec<(Tier, T)>) -> Vec<(Tier, Vec<T>)> {
    let mut groups: Vec<(Tier, Vec<T>)> = Tier::ALL.iter().map(|&t| (t, Vec::new())).collect();
    for (tier, row) in rows {
        if let Some((_, rows)) = groups.iter_mut().find(|(t, _)| *t == tier) {
            rows.push(row);
        }
    }
    groups.retain(|(_, rows)| !rows.is_empty());
    groups
}

/// `1 spell` or `22 spells`.
pub fn spells_noun(n: usize) -> String {
    fmt::pluralize(n, "spell", "spells")
}

fn level_label(level: u8) -> String {
    fmt::spell_level(level)
}

/// The compact text inside a level-number tab on a spell page: `Cantrips`, or the numeral.
fn level_tab(level: u8) -> String {
    if level == 0 {
        fmt::spell_level(level)
    } else {
        level.to_string()
    }
}

/// The heading and document title of a class list: `Wizard spells`, `Wizard cantrips` for level
/// 0, `Wizard spells, level 5` otherwise.
fn list_heading(name: &str, level: Option<u8>) -> String {
    match level {
        Some(0) => format!("{name} cantrips"),
        Some(level) => format!("{name} spells, level {level}"),
        None => format!("{name} spells"),
    }
}

fn missing_lists() -> impl IntoView {
    view! {
        <p class="notice">
            "This library has no spell lists. It was installed before they existed. "
            "Open Settings and check for a library update."
        </p>
        <p><a href=Route::Settings.href()>"Open Settings"</a></p>
    }
}

fn load_failed(error: &LibraryError) -> impl IntoView + use<> {
    view! {
        <div class="error" role="alert">
            <p>{error.to_string()}</p>
            <p><a href=Route::Settings.href()>"Open Settings"</a></p>
        </div>
    }
}

fn spell_crumbs(last: Option<(&'static str, Route)>) -> impl IntoView {
    view! {
        <nav class="crumbs" aria-label="Breadcrumb">
            <ol>
                <li><a href=Route::browse_kind(EntryKind::Spell).href()>"Spells"</a></li>
                {last.map(|(label, route)| view! { <li><a href=route.href()>{label}</a></li> })}
            </ol>
        </nav>
    }
}

fn class_jump(list: &SpellListClass) -> impl IntoView + use<> {
    let href = Route::spell_list(list.name.clone(), None).href();
    let total = list_total(list);
    let label = format!("{} spells, {}", list.name, spells_noun(total));
    view! {
        <li>
            <a class="jump class-jump" href=href aria-label=label>
                <span>{list.name.clone()}</span>
                <small class="muted">{fmt::count(total)}</small>
            </a>
        </li>
    }
}

/// `#/spells`: the way into every spell list. A list page's head: the crumb, the title and the
/// count line, then the class picker. The picker's classes are navigation, not a filter, so the
/// page has no filter row; a school is chosen on a list, or on the Spells page.
#[component]
pub fn SpellsHub() -> impl IntoView {
    set_title("Spell lists");
    let library = use_library();
    let lists = {
        let library = library.clone();
        let revision = library.selection_revision();
        local_resource(async move {
            let _ = revision.get();
            library.discovery().spell_lists().await
        })
    };
    let body = {
        move || {
            lists.with(|state| match state {
                None => {
                    view! { <p class="muted list-count" role="status" aria-live="polite">"Loading spell lists…"</p> }.into_fresh()
                }
                Some(Err(error)) => load_failed(error).into_fresh(),
                Some(Ok(None)) => missing_lists().into_fresh(),
                Some(Ok(Some(lists))) => {
                    let (common, rest) = picker_groups(lists);
                    let common_jumps = common.iter().map(|l| class_jump(l)).collect_view();
                    let rest_jumps = rest.iter().map(|l| class_jump(l)).collect_view();
                    let more = (!rest.is_empty()).then(|| {
                        view! {
                            <h3 id="other-classes" class="sub-head">"More classes"</h3>
                            <ul class="jump-list class-picker" aria-labelledby="other-classes">
                                {rest_jumps}
                            </ul>
                        }
                    });
                    view! {
                        <p class="muted list-count" role="status" aria-live="polite">
                            {fmt::pluralize(lists.classes.len(), "spell list", "spell lists")}
                        </p>
                        <section aria-labelledby="class-lists">
                            <h2 id="class-lists" class="sub-head">"Choose a class"</h2>
                            <ul class="jump-list class-picker" aria-labelledby="class-lists">
                                {common_jumps}
                            </ul>
                            {more}
                        </section>
                    }
                    .into_fresh()
                }
            })
        }
    };
    view! {
        <div class="wide list-page">
            {spell_crumbs(None)}
            <h1 tabindex="-1">"Spell lists"</h1>
            {body}
        </div>
    }
}

/// `#/spells/<class>` and `#/spells/<class>/<level>`, with the schools of the query.
#[component]
pub fn SpellListView(class: String, level: Option<u8>) -> impl IntoView {
    let library = use_library();
    set_title(&list_heading(&class, level));
    let lists = {
        let library = library.clone();
        let revision = library.selection_revision();
        local_resource(async move {
            let _ = revision.get();
            library.discovery().spell_lists().await
        })
    };
    move || {
        let (class, library) = (class.clone(), library.clone());
        lists.with(|state| match state {
            None => view! {
                <div class="wide list-page">
                    {spell_crumbs(None)}
                    <h1 tabindex="-1">{format!("{class} spells")}</h1>
                    <p class="muted list-count" role="status" aria-live="polite">"Loading spell list…"</p>
                </div>
            }
            .into_fresh(),
            Some(Err(error)) => view! {
                <div class="wide list-page">{spell_crumbs(None)}<h1 tabindex="-1">"Spell list"</h1>{load_failed(error)}</div>
            }
            .into_fresh(),
            Some(Ok(None)) => view! {
                <div class="wide list-page">{spell_crumbs(None)}<h1 tabindex="-1">"Spell list"</h1>{missing_lists()}</div>
            }
            .into_fresh(),
            Some(Ok(Some(lists))) => {
                view! { <ListPage lists=Shared::from_rc(lists.clone()) class level library /> }.into_fresh()
            }
        })
    }
}

/// True when `next` is the list that `previous` already shows, with other schools chosen. The
/// frame then leaves scroll and focus alone: the reader is looking at it.
pub fn same_spell_list(previous: &Route, next: &Route) -> bool {
    matches!(
        (previous, next),
        (
            Route::SpellList { class: a, level: la, .. },
            Route::SpellList { class: b, level: lb, .. },
        ) if a == b && la == lb
    )
}

/// One row of a list: the spell, what its second line says, and the tier of its book.
struct SpellRow {
    id: cast_iron_schema::EntryId,
    title: String,
    school: String,
    parts: Vec<(Part, String)>,
    tier: Tier,
}

impl SpellRow {
    fn new(library: &Library, spell: ListedSpell<'_>) -> Self {
        let source = library
            .entry_row_book(spell.id)
            .and_then(|id| library.source(id).cloned());
        let tier = source.as_ref().map_or(Tier::Other, Tier::of);
        let book = source.as_ref().map(|s| s.title.as_str());
        Self {
            id: spell.id.clone(),
            title: spell.title.to_string(),
            school: spell
                .detail
                .map(|d| school_label(&d.school))
                .unwrap_or_default(),
            parts: meta_parts(spell.detail, book),
            tier,
        }
    }
}

/// A two-line row. The link wraps the name only, so its accessible name stays the title; a
/// stretched overlay makes the whole row the tap target.
fn spell_row(row: SpellRow) -> impl IntoView {
    let SpellRow {
        id,
        title,
        school,
        parts,
        ..
    } = row;
    // Separators are drawn by the stylesheet, so the narrow layout can move the gist to its own
    // line and keep the dots right. `sep` is a dot before the part on the one-line layout;
    // `sep-narrow` is a dot on the narrow one, where the gist stands apart.
    let mut facts_seen = 0usize;
    let mut facts = Vec::new();
    let mut gist = None;
    for (seen, (part, text)) in parts.into_iter().enumerate() {
        let mut class = part.class().to_string();
        if seen > 0 {
            class.push_str(" sep");
        }
        if part != Part::Gist && facts_seen > 0 {
            class.push_str(" sep-narrow");
        }
        if part == Part::Fact {
            facts_seen += 1;
        }
        let item = view! { <span class=class>{text}</span> };
        if part == Part::Gist {
            gist = Some(item);
        } else {
            facts.push(item);
        }
    }
    view! {
        <li class="spell-row">
            <div class="spell-row-head">
                <a class="spell-name" href=Route::entry(id).href()>{title}</a>
                {(!school.is_empty()).then(|| view! { <span class="tag spell-school">{school}</span> })}
            </div>
            <p class="spell-row-meta"><span class="spell-facts">{facts}</span>{gist}</p>
        </li>
    }
}

/// The Compact / Wide choice, as one quiet text button: a `.text-btn` named for the layout that a
/// press switches to. The state lives in the app context, so Settings and every spell list show
/// the same choice. The button is a plain button with a changing name, not a pressed toggle,
/// because its name already states the action; `said` is the live line that names the layout
/// that is now shown, which the new name of the button does not.
fn density_toggle(said: RwSignal<&'static str>) -> impl IntoView {
    let app = use_app();
    view! {
        <li class="lvl-density">
            <button
                type="button"
                class="text-btn"
                on:click=move |_| {
                    let next = app.density.get_untracked().other();
                    app.density.set(next);
                    said.set(next.rows_label());
                    spawn_local(density::set(next));
                }
            >
                {move || app.density.get().other().rows_label()}
            </button>
        </li>
    }
}

/// The count line of a list: `199 spells`, `1 cantrip`, or `12 of 199 spells` once a school or the
/// name filter narrows it. `total` is what the page lists with nothing chosen.
fn count_line(level: Option<u8>, shown: usize, total: usize, narrowed: bool) -> String {
    let noun = |n: usize| {
        if level == Some(0) {
            fmt::pluralize(n, "cantrip", "cantrips")
        } else {
            spells_noun(n)
        }
    };
    if narrowed {
        format!("{} of {}", fmt::count(shown), noun(total))
    } else {
        noun(total)
    }
}

#[component]
fn ListPage(
    lists: Shared<SpellLists>,
    class: String,
    level: Option<u8>,
    library: Library,
) -> impl IntoView {
    let app = use_app();
    let Some(list) = find_list(&lists, &class) else {
        return view! {
            <div class="wide list-page">
                {spell_crumbs(Some(("Spell lists", Route::Spells)))}
                <h1 tabindex="-1">"No such spell list"</h1>
                <p>{format!("No spell is listed for the class {class:?} in this library.")}</p>
                <p><a href=Route::Spells.href()>"See all spell lists"</a></p>
            </div>
        }
        .into_fresh();
    };
    // The schools and the name are the route's: a school is one more history entry and the name
    // replaces the current one, and this page reads both without being rebuilt.
    let route = use_route();
    let schools = Memo::new(move |_| match route.get() {
        Route::SpellList { schools, .. } => schools,
        _ => Vec::new(),
    });
    let filter = Memo::new(move |_| match route.get() {
        Route::SpellList { name, .. } => name,
        _ => String::new(),
    });
    let name = list.name.clone();
    let counts = level_counts(list);
    let known_level = level.is_none_or(|l| counts.iter().any(|(c, _)| *c == l));
    let class_href = list
        .class
        .as_ref()
        .map(|id| Route::entry(id.clone()).href());
    // What the page lists with nothing chosen: the whole list, or its one level.
    let total = match level {
        Some(wanted) => counts
            .iter()
            .find(|(l, _)| *l == wanted)
            .map_or(0, |(_, n)| *n),
        None => list_total(list),
    };
    let route_class = class.clone();
    let route_with = {
        let route_class = route_class.clone();
        move |schools: Vec<String>, name: String| Route::SpellList {
            class: route_class.clone(),
            level,
            schools,
            name,
        }
    };

    let said = RwSignal::new("");
    let level_links = {
        let all_href = {
            let route_class = route_class.clone();
            move || {
                Route::SpellList {
                    class: route_class.clone(),
                    level: None,
                    schools: schools.get(),
                    name: filter.get(),
                }
                .href()
            }
        };
        let items = counts
            .iter()
            .map(|&(l, n)| {
                let route_class = route_class.clone();
                let href = move || {
                    Route::SpellList {
                        class: route_class.clone(),
                        level: Some(l),
                        schools: schools.get(),
                        name: filter.get(),
                    }
                    .href()
                };
                let label = format!("{}, {}", level_label(l), spells_noun(n));
                view! {
                    <li>
                        <a class="lvl" href=href aria-label=label
                            aria-current=(level == Some(l)).then_some("page")>
                            <span class="lvl-n">{level_tab(l)}</span>
                            <span class="lvl-count">{fmt::count(n)}</span>
                        </a>
                    </li>
                }
            })
            .collect_view();
        view! {
            <nav class="level-nav" aria-label=format!("{name} spell levels")>
                <a class="jump level-all" href=all_href aria-current=level.is_none().then_some("page")>
                    "All levels"
                </a>
                <ol class="lvls">{items}{density_toggle(said)}</ol>
            </nav>
        }
    };

    let heading = list_heading(&name, level);
    let lists_for_body = lists.clone();
    let list_name = name.clone();
    let library_for_body = library.clone();
    let body = move || {
        let Some(list) = find_list(&lists_for_body, &list_name) else {
            return ().into_any();
        };
        let chosen = schools.get();
        let groups = filter_list(&lists_for_body, list, level, &filter.get(), &chosen);
        if groups.is_empty() {
            let text = filter.get();
            let narrowed = !chosen.is_empty();
            // A level page names its own category: a list of cantrips has no "spells" on it.
            let empty_level = if level == Some(0) {
                "No cantrips on this list.".to_string()
            } else {
                "No spells at this level.".to_string()
            };
            let noun = if level == Some(0) { "cantrip" } else { "spell" };
            return view! {
                <p class="empty" role="status">
                    {if text.trim().is_empty() && !narrowed {
                        empty_level
                    } else if text.trim().is_empty() {
                        format!("No {noun} on this list is in the chosen schools.")
                    } else {
                        format!("No {noun} on this list matches {:?}.", text.trim())
                    }}
                </p>
            }
            .into_any();
        }
        // Tier headings appear once the list spans more than one tier.
        let tiered: Vec<(u8, Vec<(Tier, SpellRow)>)> = groups
            .into_iter()
            .map(|(l, spells)| {
                let rows = spells
                    .into_iter()
                    .map(|spell| {
                        let row = SpellRow::new(&library_for_body, spell);
                        (row.tier, row)
                    })
                    .collect();
                (l, rows)
            })
            .collect();
        let several_tiers = cast_iron_sort::set_of(
            tiered
                .iter()
                .flat_map(|(_, rows)| rows.iter().map(|(tier, _)| *tier)),
        )
        .len()
            > 1;
        let sections = tiered
            .into_iter()
            .map(|(l, rows)| {
                let n = rows.len();
                let tiers = by_tier(rows)
                    .into_iter()
                    .map(|(tier, rows)| {
                        let head = several_tiers.then(|| {
                            if level.is_none() {
                                view! { <h3 class="tier-head">{tier.label()}</h3> }.into_any()
                            } else {
                                view! { <h2 class="tier-head">{tier.label()}</h2> }.into_any()
                            }
                        });
                        let items = rows.into_iter().map(spell_row).collect_view();
                        view! { {head}<ul class="spell-rows">{items}</ul> }
                    })
                    .collect_view();
                let head = level.is_none().then(|| {
                    // A level link keeps the chosen schools, like the level bar's links.
                    let href = Route::SpellList {
                        class: list_name.clone(),
                        level: Some(l),
                        schools: chosen.clone(),
                        name: filter.get(),
                    }
                    .href();
                    view! {
                        <h2 class="level-head">
                            <a href=href>{level_label(l)}</a>
                            <span class="muted">{spells_noun(n)}</span>
                        </h2>
                    }
                });
                view! {
                    <section class="level-group" data-level=l>
                        {head}
                        {tiers}
                    </section>
                }
            })
            .collect_view();
        // `data-density` is the reader's choice of row layout: "wide" keeps three lines on a
        // phone, "compact" always shows two. The CSS under each value selects the layout.
        view! {
            <div class="spell-list" data-density=move || app.density.get().slug()>{sections}</div>
        }
        .into_any()
    };

    // The count line follows the page: what the list shows now, out of what it holds.
    let shown = {
        let (lists, list_name) = (lists.clone(), name.clone());
        Memo::new(move |_| {
            find_list(&lists, &list_name).map_or(0, |list| {
                filter_list(&lists, list, level, &filter.get(), &schools.get())
                    .iter()
                    .map(|(_, spells)| spells.len())
                    .sum()
            })
        })
    };
    // Typing replaces the history entry, so no key makes an entry of its own, and the write waits
    // for the next animation frame, so a fast typist rebuilds the list once per frame, not once
    // per key. The field keeps focus and caret: the frame writes the text it already shows.
    let typed: StoredValue<Option<String>> = StoredValue::new(None);
    let on_name = move |text: String| {
        let waiting = typed.with_value(Option::is_some);
        typed.set_value(Some(text));
        if waiting {
            return;
        }
        request_animation_frame(move || {
            let Some(text) = typed.try_update_value(Option::take).flatten() else {
                return;
            };
            // Read the route as it is now, not as it was when this field rendered: a level link
            // pushes the address before the list remounts, so a stale field can type into the
            // new list. The reader may also have left the list in this frame.
            if let Route::SpellList {
                class,
                level,
                schools,
                ..
            } = route.get_untracked()
            {
                replace(&Route::SpellList {
                    class,
                    level,
                    schools,
                    name: text,
                });
            }
        });
    };
    let show_all_href = route_with(Vec::new(), String::new()).href();
    let count_view = move || {
        let narrowed = schools.with(|s| !s.is_empty()) || !filter.with(|f| f.trim().is_empty());
        let show_all = narrowed.then(|| {
            view! {
                " · "
                <a class="count-action" href=show_all_href.clone()
                    aria-label=if level == Some(0) { "Show all cantrips" } else { "Show all spells" }>
                    "Show all"
                </a>
            }
        });
        view! { {count_line(level, shown.get(), total, narrowed)}{show_all} }
    };

    // Counts follow the page: on a level page they are that level's. A school that is chosen
    // stays listed, with its count, so it can be cleared.
    let mut school_counts = list_schools(&lists, list, level);
    for chosen in schools.get_untracked() {
        if !school_counts.iter().any(|(s, _)| *s == chosen) {
            school_counts.push((chosen, 0));
        }
    }
    cast_iron_sort::sort(&mut school_counts);
    let options: Vec<MultiOption> = school_counts
        .into_iter()
        .map(|(value, count)| MultiOption {
            label: school_label(&value),
            value,
            count,
        })
        .collect();
    let school_menu = (options.len() > 1).then(|| {
        let spec = categories::SCHOOL_MENU;
        let state = MenuState::new();
        let labels: Vec<(String, String)> = options
            .iter()
            .map(|o| (o.value.clone(), o.label.clone()))
            .collect();
        let text = Signal::derive(move || {
            let chosen = schools.get();
            let names: Vec<String> = chosen
                .iter()
                .map(|value| {
                    labels
                        .iter()
                        .find(|(v, _)| v.eq_ignore_ascii_case(value))
                        .map_or_else(|| school_label(value), |(_, label)| label.clone())
                })
                .collect();
            Some(multi_trigger_text(&spec, &names))
        });
        let narrowed = Signal::derive(move || schools.with(|s| !s.is_empty()));
        let chosen = Signal::derive(move || schools.get());
        let on_change = {
            let route_with = route_with.clone();
            Callback::new(move |next: Vec<String>| {
                push(&route_with(next, filter.get_untracked()));
            })
        };
        let button = view! {
            <div class="filter-menu">
                <FilterMenuButton spec state text narrowed focus=false />
            </div>
        };
        // "All schools" keeps the name that is typed now.
        let all_href = Signal::derive({
            let route_with = route_with.clone();
            move || route_with(Vec::new(), filter.get()).href()
        });
        let panel = view! {
            <MultiFilterMenuPanel spec state options chosen all_href count_noun=("spell", "spells")
                on_change />
        };
        (button, panel)
    });
    let (school_button, school_panel) = match school_menu {
        Some((button, panel)) => (Some(button), Some(panel)),
        None => (None, None),
    };
    let class_page = class_href.map(|href| {
        view! {
            <a class="page-link" href=href aria-label=format!("{name} class page")>
                <span>"Class page"</span>
                {icons::chevron_right_large()}
            </a>
        }
    });
    let filter_row = (school_button.is_some() || class_page.is_some())
        .then(|| view! { <div class="filter-row">{school_button}{class_page}</div> });

    view! {
        <div class="wide list-page">
            {spell_crumbs(Some(("Spell lists", Route::Spells)))}
            <h1 tabindex="-1">{heading}</h1>
            <p class="muted list-count" role="status" aria-live="polite">{count_view}</p>
            {filter_row}
            {school_panel}
            <form class="search-form list-filter" role="search" on:submit=|ev| ev.prevent_default()>
                <label class="sr-only" for="list-filter">"Filter this list by name"</label>
                <input
                    id="list-filter"
                    type="search"
                    placeholder="Filter this list by name"
                    autocomplete="off"
                    autocapitalize="off"
                    spellcheck="false"
                    prop:value=move || filter.get()
                    on:input=move |ev| on_name(event_target_value(&ev))
                />
            </form>
            {level_links}
            <p class="sr-only" role="status">{move || said.get()}</p>
            {(!known_level).then(|| view! {
                <p class="notice">
                    {match level {
                        Some(0) => format!("{name} has no cantrips. The levels it has are listed above."),
                        Some(l) => format!("{name} has no spells at level {l}. The levels it has are listed above."),
                        None => format!("{name} has no spells at this level. The levels it has are listed above."),
                    }}
                </p>
            })}
            {body}
        </div>
    }
    .into_fresh()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::EntryId;

    fn list(name: &str, class: Option<&str>) -> SpellListClass {
        SpellListClass {
            name: name.into(),
            class: class.map(|c| EntryId::parse(c).unwrap()),
            levels: vec![],
        }
    }

    #[test]
    fn the_picker_leads_with_the_common_classes_in_their_own_order() {
        let lists = SpellLists {
            classes: ["Witch", "Wizard", "Hunter", "bard", "Cleric", "Arcanist"]
                .iter()
                .map(|n| list(n, None))
                .collect(),
            ..Default::default()
        };
        let (common, rest) = picker_groups(&lists);
        let names = |lists: &[&SpellListClass]| -> Vec<String> {
            lists.iter().map(|l| l.name.clone()).collect()
        };
        assert_eq!(names(&common), ["Wizard", "Cleric", "bard"]);
        assert_eq!(names(&rest), ["Arcanist", "Hunter", "Witch"]);
    }

    #[test]
    fn a_row_leaves_out_what_it_does_not_have() {
        let detail = SpellListDetail {
            school: "evocation".into(),
            casting_time: "1 standard action".into(),
            saving_throw: String::new(),
            gist: "Fire leaps from your hand".into(),
        };
        assert_eq!(
            meta_parts(Some(&detail), Some("Core Rulebook")),
            [
                (Part::Fact, "1 standard action".to_string()),
                (Part::Gist, "Fire leaps from your hand".to_string()),
                (Part::Book, "Core Rulebook".to_string()),
            ]
        );
        assert_eq!(
            meta_parts(None, Some("Core Rulebook")),
            [(Part::Book, "Core Rulebook".to_string())]
        );
        assert!(meta_parts(None, None).is_empty());
        assert!(meta_parts(Some(&SpellListDetail::default()), Some("")).is_empty());
    }

    #[test]
    fn tiers_put_rulebooks_first_and_keep_the_title_order_inside_a_tier() {
        let rows = vec![
            (Tier::Adventures, "Aid"),
            (Tier::Rulebooks, "Bless"),
            (Tier::Other, "Calm"),
            (Tier::Rulebooks, "Daze"),
        ];
        assert_eq!(
            by_tier(rows),
            [
                (Tier::Rulebooks, vec!["Bless", "Daze"]),
                (Tier::Adventures, vec!["Aid"]),
                (Tier::Other, vec!["Calm"]),
            ]
        );
    }

    #[test]
    fn schools_read_with_a_capital() {
        assert_eq!(school_label("evocation"), "Evocation");
        assert_eq!(school_label(""), "");
    }

    #[test]
    fn counts_read_as_words() {
        assert_eq!(spells_noun(1), "1 spell");
        assert_eq!(spells_noun(0), "0 spells");
        assert_eq!(spells_noun(131), "131 spells");
    }

    #[test]
    fn level_zero_is_cantrips_in_headings_tabs_and_labels() {
        assert_eq!(level_label(0), "Cantrips");
        assert_eq!(level_label(5), "Level 5");
        assert_eq!(level_tab(0), "Cantrips");
        assert_eq!(level_tab(5), "5");
        // The heading names the page's own noun, and level 0 needs no "level" qualifier.
        assert_eq!(list_heading("Wizard", None), "Wizard spells");
        assert_eq!(list_heading("Wizard", Some(0)), "Wizard cantrips");
        assert_eq!(list_heading("Wizard", Some(5)), "Wizard spells, level 5");
    }
}
