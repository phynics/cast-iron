//! Spell lists: every class with its levels (the hub), and the list of one class, whole or by
//! level. The data is `lists.json`, an index of `SpellFields::levels`.

use crate::fmt;
use crate::graph::{ListedSpell, filter_list, find_list, level_counts, list_schools, list_total};
use crate::library::{Library, LibraryError, use_library};
use crate::route::Route;
use crate::views::book_tier::Tier;
use crate::views::search::{facet_label, sort_facet_pairs};
use crate::views::util::{Shared, local_resource, set_title};
use cast_iron_schema::{EntryKind, SpellListClass, SpellListDetail, SpellLists};
use leptos::prelude::*;

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
    rest.sort_by_key(|l| l.name.to_lowercase());
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
    if n == 1 {
        "1 spell".into()
    } else {
        format!("{} spells", fmt::count(n))
    }
}

fn level_label(level: u8) -> String {
    format!("Level {level}")
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
    let href = Route::SpellList {
        class: list.name.clone(),
        level: None,
    }
    .href();
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

/// `#/spells`: the way into every spell list.
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
    let schools = {
        let library = library.clone();
        let revision = library.selection_revision();
        local_resource(async move {
            let _ = revision.get();
            let discovery = library.discovery();
            discovery.facet_values("school").await.map(|mut values| {
                sort_facet_pairs("school", &mut values);
                values
            })
        })
    };
    let body = {
        move || {
            lists.with(|state| match state {
                None => {
                    view! { <p class="status" role="status">"Loading spell lists…"</p> }.into_any()
                }
                Some(Err(error)) => load_failed(error).into_any(),
                Some(Ok(None)) => missing_lists().into_any(),
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
                        <section aria-labelledby="class-lists">
                            <h2 id="class-lists" class="sub-head">"Choose a class"</h2>
                            <ul class="jump-list class-picker" aria-labelledby="class-lists">
                                {common_jumps}
                            </ul>
                            {more}
                        </section>
                    }
                    .into_any()
                }
            })
        }
    };
    view! {
        <div class="wide">
            {spell_crumbs(None)}
            <h1>"Spell lists"</h1>
            {body}
            <section class="lens" aria-labelledby="by-school">
                <h2 id="by-school" class="sub-head">"By school"</h2>
                {move || schools.with(|state| match state {
                    Some(Ok(values)) => {
                        let chips = values.iter().map(|(value, n)| {
                            let href = Route::Search {
                                q: String::new(),
                                kinds: vec![EntryKind::Spell],
                                sources: Vec::new(),
                                facets: vec![("school".into(), value.clone())],
                            }
                            .href();
                            view! {
                                <li>
                                    <a class="lens-chip" href=href>
                                        {facet_label("school", value)}<small>{*n}</small>
                                    </a>
                                </li>
                            }
                        }).collect_view();
                        view! { <ul class="lens-chips">{chips}</ul> }.into_any()
                    }
                    Some(Err(_)) => ().into_any(),
                    None => view! { <p class="hint lens-wait" role="status">"Loading schools…"</p> }.into_any(),
                })}
                <p><a href=Route::browse_kind(EntryKind::Spell).href()>"All spells, A to Z"</a></p>
            </section>
        </div>
    }
}

/// `#/spells/<class>` and `#/spells/<class>/<level>`.
#[component]
pub fn SpellListView(class: String, level: Option<u8>) -> impl IntoView {
    let library = use_library();
    set_title(&match level {
        Some(level) => format!("{class} spells, level {level}"),
        None => format!("{class} spells"),
    });
    let lists = {
        let library = library.clone();
        let revision = library.selection_revision();
        local_resource(async move {
            let _ = revision.get();
            library.discovery().spell_lists().await
        })
    };
    let filter = RwSignal::new(String::new());
    let schools = RwSignal::new(Vec::<String>::new());
    move || {
        let (class, library) = (class.clone(), library.clone());
        lists.with(|state| match state {
            None => view! {
                <div class="wide">
                    {spell_crumbs(None)}
                    <h1>{format!("{class} spells")}</h1>
                    <p class="status" role="status">"Loading spell list…"</p>
                </div>
            }
            .into_any(),
            Some(Err(error)) => view! {
                <div class="wide">{spell_crumbs(None)}<h1>"Spell list"</h1>{load_failed(error)}</div>
            }
            .into_any(),
            Some(Ok(None)) => view! {
                <div class="wide">{spell_crumbs(None)}<h1>"Spell list"</h1>{missing_lists()}</div>
            }
            .into_any(),
            Some(Ok(Some(lists))) => {
                view! { <ListPage lists=Shared::from_rc(lists.clone()) class level filter schools library /> }.into_any()
            }
        })
    }
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
            .canonical_source_for_entry(spell.id)
            .and_then(|id| library.source(&id).cloned());
        let tier = Tier::from_category(source.as_ref().and_then(|s| s.category.as_deref()));
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

#[component]
fn ListPage(
    lists: Shared<SpellLists>,
    class: String,
    level: Option<u8>,
    filter: RwSignal<String>,
    schools: RwSignal<Vec<String>>,
    library: Library,
) -> impl IntoView {
    let Some(list) = find_list(&lists, &class) else {
        return view! {
            <div class="wide">
                {spell_crumbs(Some(("Spell lists", Route::Spells)))}
                <h1>"No such spell list"</h1>
                <p>{format!("No spell is listed for the class {class:?} in this library.")}</p>
                <p><a href=Route::Spells.href()>"See all spell lists"</a></p>
            </div>
        }
        .into_any();
    };
    let name = list.name.clone();
    let counts = level_counts(list);
    let known_level = level.is_none_or(|l| counts.iter().any(|(c, _)| *c == l));
    let class_href = list
        .class
        .as_ref()
        .map(|id| Route::entry(id.clone()).href());
    let total = list_total(list);

    let level_links = {
        let all_href = Route::SpellList {
            class: name.clone(),
            level: None,
        }
        .href();
        let items = counts
            .iter()
            .map(|&(l, n)| {
                let href = Route::SpellList {
                    class: name.clone(),
                    level: Some(l),
                }
                .href();
                let label = format!("{}, {}", level_label(l), spells_noun(n));
                view! {
                    <li>
                        <a class="lvl" href=href aria-label=label
                            aria-current=(level == Some(l)).then_some("page")>
                            <span class="lvl-n">{l}</span>
                            <span class="lvl-count">{n}</span>
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
                <ol class="lvls">{items}</ol>
            </nav>
        }
    };

    let heading = match level {
        Some(l) => format!("{name} spells, level {l}"),
        None => format!("{name} spells"),
    };
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
            return view! {
                <p class="empty" role="status">
                    {if text.trim().is_empty() && !narrowed {
                        "No spells at this level.".to_string()
                    } else if text.trim().is_empty() {
                        "No spell on this list is in the chosen schools.".to_string()
                    } else {
                        format!("No spell on this list matches {:?}.", text.trim())
                    }}
                </p>
            }
            .into_any();
        }
        let shown: usize = groups.iter().map(|(_, s)| s.len()).sum();
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
        let several_tiers = tiered
            .iter()
            .flat_map(|(_, rows)| rows.iter().map(|(tier, _)| *tier))
            .collect::<std::collections::BTreeSet<_>>()
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
                    let href = Route::SpellList {
                        class: list_name.clone(),
                        level: Some(l),
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
        // `data-density` is the reader's choice of row layout. "wide" is the only value for
        // now: casting time, save and book on one line and the gist under them on a phone.
        view! {
            <p class="muted" role="status" data-testid="list-count">{spells_noun(shown)}</p>
            <div class="spell-list" data-density="wide">{sections}</div>
        }
        .into_any()
    };

    // Counts follow the page: on a level page they are that level's. A school that is still
    // chosen stays listed, with its count, so it can be cleared.
    let mut school_counts = list_schools(&lists, list, level);
    for chosen in schools.get_untracked() {
        if !school_counts.iter().any(|(s, _)| *s == chosen) {
            school_counts.push((chosen, 0));
        }
    }
    school_counts.sort();
    let school_filter = (school_counts.len() > 1).then(|| {
        let chips = school_counts
            .into_iter()
            .map(|(school, n)| {
                let label = school_label(&school);
                let value = school.clone();
                let checked = Signal::derive({
                    let school = school.clone();
                    move || schools.with(|chosen| chosen.contains(&school))
                });
                let toggle = move |_| {
                    schools.update(|chosen| match chosen.iter().position(|s| *s == value) {
                        Some(i) => {
                            chosen.remove(i);
                        }
                        None => chosen.push(value.clone()),
                    })
                };
                let aria = format!("{label}, {}", spells_noun(n));
                view! {
                    <label class="chip">
                        <input type="checkbox" prop:checked=checked aria-label=aria on:change=toggle />
                        <span>{label}<small class="chip-count">{n}</small></span>
                    </label>
                }
            })
            .collect_view();
        view! {
            <details class="lens-more school-filter">
                <summary>
                    "Filter by school"
                    {move || {
                        let n = schools.with(Vec::len);
                        (n > 0).then(|| view! { <span class="muted">{format!("{n} chosen")}</span> })
                    }}
                </summary>
                <div class="chips">{chips}</div>
            </details>
        }
    });

    view! {
        <div class="wide">
            {spell_crumbs(Some(("Spell lists", Route::Spells)))}
            <h1>{heading}</h1>
            <p class="muted">{format!("{} on this list", spells_noun(total))}</p>
            {class_href.map(|href| view! {
                <p>
                    <a class="jump class-page" href=href aria-label=format!("{name} class page")>
                        "Class page"
                    </a>
                </p>
            })}
            {level_links}
            {(!known_level).then(|| view! {
                <p class="notice">
                    {format!("{name} has no spells at level {}. The levels it has are listed above.", level.unwrap_or(0))}
                </p>
            })}
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
                    on:input=move |ev| filter.set(event_target_value(&ev))
                />
            </form>
            {school_filter}
            {body}
        </div>
    }
    .into_any()
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
}
