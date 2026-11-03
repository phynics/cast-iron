//! Spell lists: every class with its levels (the hub), and the list of one class, whole or by
//! level. The data is `lists.json`, an index of `SpellFields::levels`.

use crate::graph::{filter_list, find_list, level_counts, list_total};
use crate::library::{Library, LibraryError, use_library};
use crate::route::Route;
use crate::views::search::{facet_label, sort_facet_pairs};
use crate::views::util::{Shared, local_resource, set_title};
use cast_iron_schema::{EntryKind, SpellListClass, SpellLists};
use leptos::prelude::*;

/// Lists whose class has an entry first, then the others, each group in name order.
pub fn split_lists(lists: &SpellLists) -> (Vec<&SpellListClass>, Vec<&SpellListClass>) {
    let mut with_class: Vec<&SpellListClass> = Vec::new();
    let mut without: Vec<&SpellListClass> = Vec::new();
    for list in &lists.classes {
        if list.class.is_some() {
            with_class.push(list);
        } else {
            without.push(list);
        }
    }
    with_class.sort_by_key(|l| l.name.to_lowercase());
    without.sort_by_key(|l| l.name.to_lowercase());
    (with_class, without)
}

/// `1 spell` or `22 spells`.
pub fn spells_noun(n: usize) -> String {
    if n == 1 {
        "1 spell".into()
    } else {
        format!("{n} spells")
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
                <li><a href=Route::Browse.href()>"Browse"</a></li>
                <li><a href=Route::BrowseKind(EntryKind::Spell).href()>"Spells"</a></li>
                {last.map(|(label, route)| view! { <li><a href=route.href()>{label}</a></li> })}
            </ol>
        </nav>
    }
}

fn class_card(list: &SpellListClass, library: &Library) -> impl IntoView + use<> {
    let class_href = list
        .class
        .as_ref()
        .map(|id| Route::entry(id.clone()).href());
    let list_href = Route::SpellList {
        class: list.name.clone(),
        level: None,
    }
    .href();
    let levels = level_counts(list)
        .into_iter()
        .map(|(level, n)| {
            let href = Route::SpellList {
                class: list.name.clone(),
                level: Some(level),
            }
            .href();
            let label = format!("{} level {level}, {}", list.name, spells_noun(n));
            view! {
                <li>
                    <a class="lvl" href=href aria-label=label>
                        <span class="lvl-n">{level}</span>
                        <span class="lvl-count">{n}</span>
                    </a>
                </li>
            }
        })
        .collect_view();
    let _ = library;
    let total = list_total(list);
    let name = list.name.clone();
    view! {
        <li class="spell-class">
            <div class="spell-class-head">
                <h3><a href=list_href>{name}</a></h3>
                <span class="muted">{spells_noun(total)}</span>
                {class_href.map(|href| view! { <a class="spell-class-page" href=href>"Class page"</a> })}
            </div>
            <ol class="lvls" aria-label=format!("{} levels", list.name)>{levels}</ol>
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
        let library = library.clone();
        move || {
            lists.with(|state| match state {
                None => view! { <p class="status" role="status">"Loading spell lists…"</p> }.into_any(),
                Some(Err(error)) => load_failed(error).into_any(),
                Some(Ok(None)) => missing_lists().into_any(),
                Some(Ok(Some(lists))) => {
                    let (with_class, without) = split_lists(lists);
                    let cards = with_class
                        .iter()
                        .map(|l| class_card(l, &library))
                        .collect_view();
                    let others = (!without.is_empty()).then(|| {
                        let cards = without.iter().map(|l| class_card(l, &library)).collect_view();
                        view! {
                            <section aria-labelledby="other-lists">
                                <h2 id="other-lists" class="sub-head">"Lists without a class page"</h2>
                                <p class="muted measure">
                                    "These class pages are unavailable or belong to disabled books. "
                                    "Their lists include spells from your active books."
                                </p>
                                <ul class="spell-classes">{cards}</ul>
                            </section>
                        }
                    });
                    view! {
                        <section aria-labelledby="class-lists">
                            <h2 id="class-lists" class="sub-head">"By class and level"</h2>
                            <p class="muted measure">
                                "Choose a class for its whole list, or a number for one spell level. "
                                "The small number under a level is how many spells it holds."
                            </p>
                            <ul class="spell-classes">{cards}</ul>
                        </section>
                        {others}
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
                                    <a class="chip-link lens-chip" href=href>
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
                <p><a href=Route::BrowseKind(EntryKind::Spell).href()>"All spells, A to Z"</a></p>
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
                view! { <ListPage lists=Shared::from_rc(lists.clone()) class level filter library /> }.into_any()
            }
        })
    }
}

#[component]
fn ListPage(
    lists: Shared<SpellLists>,
    class: String,
    level: Option<u8>,
    filter: RwSignal<String>,
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
                <a class="pill level-all" href=all_href aria-current=level.is_none().then_some("page")>
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
    let body = move || {
        let Some(list) = find_list(&lists_for_body, &list_name) else {
            return ().into_any();
        };
        let groups = filter_list(&lists_for_body, list, level, &filter.get());
        if groups.is_empty() {
            let text = filter.get();
            return view! {
                <p class="empty" role="status">
                    {if text.trim().is_empty() {
                        "No spells at this level.".to_string()
                    } else {
                        format!("No spell on this list matches {:?}.", text.trim())
                    }}
                </p>
            }
            .into_any();
        }
        let shown: usize = groups.iter().map(|(_, s)| s.len()).sum();
        let sections = groups
            .into_iter()
            .map(|(l, spells)| {
                let n = spells.len();
                let rows = spells
                    .into_iter()
                    .map(|spell| {
                        view! { <li><a href=Route::entry(spell.0.clone()).href()>{spell.1.clone()}</a></li> }
                    })
                    .collect_view();
                let head = level.is_none().then(|| {
                    let href = Route::SpellList { class: list_name.clone(), level: Some(l) }.href();
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
                        <ul class="entry-list">{rows}</ul>
                    </section>
                }
            })
            .collect_view();
        view! {
            <p class="muted" role="status" data-testid="list-count">{spells_noun(shown)}</p>
            {sections}
        }
        .into_any()
    };

    let _ = library;
    view! {
        <div class="wide">
            {spell_crumbs(Some(("Spell lists", Route::Spells)))}
            <h1>{heading}</h1>
            <p class="muted">
                {format!("{} on this list", spells_noun(total))}
                {class_href.map(|href| view! { " · " <a href=href>{format!("{name} class page")}</a> })}
            </p>
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
    fn lists_split_by_whether_their_class_has_a_page() {
        let lists = SpellLists {
            spells: vec![],
            classes: vec![
                list("Wizard", Some("c/wizard")),
                list("Hunter", None),
                list("bard", Some("c/bard")),
                list("Arcanist", None),
            ],
        };
        let (with, without) = split_lists(&lists);
        assert_eq!(
            with.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
            ["bard", "Wizard"]
        );
        assert_eq!(
            without.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
            ["Arcanist", "Hunter"]
        );
    }

    #[test]
    fn counts_read_as_words() {
        assert_eq!(spells_noun(1), "1 spell");
        assert_eq!(spells_noun(0), "0 spells");
        assert_eq!(spells_noun(131), "131 spells");
    }
}
