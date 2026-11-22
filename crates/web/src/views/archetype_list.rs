//! The archetype list of a class page and its compatibility filter (#170, part D; DESIGN.md,
//! "Archetype list"). The rules are in [`crate::archetypes`]; this file draws them.
//!
//! One row per archetype: the name as a link to the archetype's own page, the book, and what it
//! replaces and alters. Two filter menus narrow the list, the shared ones of DESIGN.md's "Filter
//! menu": `Keep` (features of the class, several at once) and `Combine with` (one archetype).
//! The choices are the route's (`?keep=` and `?with=`): a choice is one more history entry that
//! this page reads without being rebuilt, so Back undoes it and a shared link opens filtered.

use crate::archetypes::{self, FeatureOption, Row};
use crate::class_sections::Section;
use crate::fmt;
use crate::graph::Relations;
use crate::library::Library;
use crate::route::{ArchFilter, Route, push, use_route};
use crate::views::filter_menu::{
    FilterMenuButton, FilterMenuPanel, MenuOption, MenuSpec, MenuState, MultiFilterMenuPanel,
    MultiOption, button_id, focus_by_id, multi_trigger_text,
};
use crate::views::related::source_title;
use crate::views::util::{Shared, sort_key};
use cast_iron_schema::{EntryId, TradeIndex};
use leptos::prelude::*;

const KEEP: MenuSpec = MenuSpec {
    field: "keep",
    group: "Keep",
    all_label: "Any feature",
    find_label: "Find a feature",
    list_label: "Features to keep",
    singular: "feature",
    plural: "features",
};

const WITH: MenuSpec = MenuSpec {
    field: "with",
    group: "Combine with",
    all_label: "Any archetype",
    find_label: "Find an archetype",
    list_label: "Archetype to combine with",
    singular: "archetype",
    plural: "archetypes",
};

/// The rows of a class: its archetypes in the active books, A to Z.
pub fn rows_of(
    library: &Library,
    class: &EntryId,
    relations: &Relations,
    trades: Option<&TradeIndex>,
) -> Vec<Row> {
    let mut rows: Vec<Row> = relations
        .sources_of(class, cast_iron_schema::RelationKind::Archetype)
        .into_iter()
        .map(|id| {
            let title = library
                .stub(id)
                .map_or_else(|| id.to_string(), |stub| stub.title.clone());
            Row::new(
                id.clone(),
                title,
                source_title(library, id),
                trades.and_then(|index| index.row(id)),
            )
        })
        .collect();
    cast_iron_sort::sort_by_key(&mut rows, |row| (sort_key(&row.title), row.id.clone()));
    rows
}

fn row_view(row: &Row) -> impl IntoView + use<> {
    let meta: Vec<String> = [row.replaces.clone(), row.alters.clone()]
        .into_iter()
        .flatten()
        .collect();
    view! {
        <li class="arch-row">
            <a href=Route::entry(row.id.clone()).href()>{row.title.clone()}</a>
            <span class="row-source muted">{row.book.clone()}</span>
            {(!meta.is_empty()).then(|| view! { <p class="arch-meta muted">{meta.join(" · ")}</p> })}
        </li>
    }
}

/// Why nothing can be checked against the chosen archetype, or `None` when it can be.
fn blocked_note(rows: &[Row], with: &Option<EntryId>) -> Option<String> {
    let id = with.as_ref()?;
    match rows.iter().find(|row| &row.id == id) {
        None => Some(
            "That archetype is not in this list, so no archetype can be checked against it."
                .to_string(),
        ),
        Some(row) if row.is_checked() => None,
        Some(row) if !row.has_data() => Some(format!(
            "The data does not say which class features {} changes, so the filter cannot check any archetype against it.",
            row.title
        )),
        Some(row) => Some(format!(
            "Some trades of {} name no class feature, so the filter cannot check any archetype against it.",
            row.title
        )),
    }
}

/// The hint under the `Not checked` rows: what the data lacks for them.
fn unchecked_hint(rows: &[Row], indexes: &[usize]) -> String {
    let no_data = indexes.iter().any(|&index| !rows[index].has_data());
    let partial = indexes.iter().any(|&index| rows[index].has_data());
    let mut hint = Vec::new();
    if no_data {
        hint.push("The data does not say which class features these change, so the filter cannot check them.");
    }
    if partial {
        hint.push("Some trades of these name no class feature, so the filter cannot say whether they fit.");
    }
    hint.join(" ")
}

#[component]
pub fn ArchetypeList(
    class: EntryId,
    book: Option<String>,
    /// The stop the choices keep in the address, when the page has the Archetypes stop.
    section: Option<Section>,
    rows: Vec<Row>,
    features: Vec<FeatureOption>,
) -> impl IntoView {
    let route = use_route();
    let rows = Shared::new(rows);
    let features = Shared::new(features);
    // The choices of the address, less any `?keep=` that is not a feature of this class: the
    // menu, the count and the list never show it, and the next choice drops it from the address.
    let choice = {
        let features = features.clone();
        Memo::new(move |_| {
            let mut arch = route.with(Route::arch);
            arch.keep
                .retain(|anchor| features.iter().any(|feature| &feature.anchor == anchor));
            arch
        })
    };
    let keep = Memo::new(move |_| choice.with(|choice| choice.keep.clone()));
    let with = Memo::new(move |_| choice.with(|choice| choice.with.clone()));
    let route_for = {
        let (class, book) = (class.clone(), book.clone());
        Shared::new(
            move |keep: Vec<String>, with: Option<EntryId>| Route::Entry {
                id: class.clone(),
                anchor: None,
                book: book.clone(),
                section,
                arch: ArchFilter { keep, with },
            },
        )
    };
    let shown = {
        let rows = rows.clone();
        Memo::new(move |_| choice.with(|choice| archetypes::narrow(&rows, choice)))
    };
    let nothing_chosen = move || choice.with(ArchFilter::is_empty);
    let has_features = !features.is_empty();
    let keep_menu = MenuState::new();
    let with_menu = MenuState::new();

    // The buttons.
    let keep_text = {
        let features = features.clone();
        Signal::derive(move || {
            let labels: Vec<String> = keep.with(|chosen| {
                chosen
                    .iter()
                    .map(|anchor| {
                        features
                            .iter()
                            .find(|feature| &feature.anchor == anchor)
                            .map_or_else(|| archetypes::anchor_label(anchor), |f| f.label.clone())
                    })
                    .collect()
            });
            Some(multi_trigger_text(&KEEP, &labels))
        })
    };
    let with_text = {
        let rows = rows.clone();
        Signal::derive(move || {
            let title = with.with(|id| {
                id.as_ref().map(|id| {
                    rows.iter()
                        .find(|row| &row.id == id)
                        .map_or_else(|| "not in this list".to_string(), |row| row.title.clone())
                })
            });
            Some(format!(
                "{}: {}",
                WITH.group,
                title.as_deref().unwrap_or(WITH.all_label)
            ))
        })
    };
    let keep_narrowed = Signal::derive(move || keep.with(|chosen| !chosen.is_empty()));
    let with_narrowed = Signal::derive(move || with.with(Option::is_some));

    // A choice of archetype closes its panel and puts focus back on the button, as a choice
    // in a single-choice menu does on a list page. Back and Forward change the choice with the
    // panel shut, and leave focus where it is.
    Effect::new(move |previous: Option<Option<EntryId>>| {
        let now = with.get();
        if previous.is_some_and(|before| before != now) {
            // The panel's link asked the next list page to focus the button; this page has no
            // rebuild to read the request, so it is read here.
            crate::views::filter_menu::take_focus_request();
            if with_menu.open.get_untracked() {
                with_menu.dismiss(&WITH);
            }
        }
        now
    });

    // The features to keep. The panel reads the choices from the route, and its `Any feature`
    // link follows the archetype that is chosen.
    let keep_panel = {
        let (features, route_for) = (features.clone(), route_for.clone());
        move || {
            let options: Vec<MultiOption> = features
                .iter()
                .map(|feature| MultiOption {
                    value: feature.anchor.clone(),
                    label: feature.label.clone(),
                    count: feature.count,
                })
                .collect();
            let all_href = Signal::derive({
                let route_for = route_for.clone();
                move || route_for(Vec::new(), with.get()).href()
            });
            let on_change = {
                let route_for = route_for.clone();
                Callback::new(move |next: Vec<String>| push(&route_for(next, with.get_untracked())))
            };
            let chosen = Signal::derive(move || keep.get());
            view! {
                <MultiFilterMenuPanel spec=KEEP state=keep_menu options chosen all_href
                    count_noun=("archetype", "archetypes") on_change />
            }
        }
    };

    // The archetype to combine with: a link per archetype, with how many others it fits.
    // How many others each archetype fits does not depend on the choices: counted once.
    let fits: Vec<u32> = rows
        .iter()
        .map(|row| archetypes::compatible_with(&rows, row) as u32)
        .collect();
    let with_panel = {
        let (rows, route_for) = (rows.clone(), route_for.clone());
        move || {
            let (keep_now, with_now) = (keep.get(), with.get());
            let options: Vec<MenuOption> = rows
                .iter()
                .zip(&fits)
                .map(|(row, &count)| MenuOption {
                    value: row.id.to_string(),
                    label: row.title.clone(),
                    count,
                    href: route_for(keep_now.clone(), Some(row.id.clone())).href(),
                })
                .collect();
            let all_href = route_for(keep_now, None).href();
            let current = with_now.map(|id| id.to_string());
            view! {
                <FilterMenuPanel spec=WITH state=with_menu options current all_href />
            }
        }
    };

    // The list, the count line and the rows the data cannot place.
    let list = {
        let rows = rows.clone();
        move || {
            shown.with(|s| {
                s.compatible
                    .iter()
                    .map(|&index| row_view(&rows[index]))
                    .collect_view()
            })
        }
    };
    let blocked = {
        let rows = rows.clone();
        move || with.with(|with| blocked_note(&rows, with))
    };
    let unchecked = {
        let (rows, blocked) = (rows.clone(), blocked.clone());
        move || {
            shown.with(|s| {
                (!s.unchecked.is_empty()).then(|| {
                    let hint = blocked().unwrap_or_else(|| unchecked_hint(&rows, &s.unchecked));
                    let items = s
                        .unchecked
                        .iter()
                        .map(|&index| row_view(&rows[index]))
                        .collect_view();
                    view! {
                        <h4 class="arch-unchecked">"Not checked"</h4>
                        <p class="hint">{hint}</p>
                        <ul class="entry-list arch-rows">{items}</ul>
                    }
                })
            })
        }
    };
    let count_line = {
        let rows = rows.clone();
        move || {
            let n = shown.with(|s| s.compatible.len());
            let total = choice.with(|choice| archetypes::others(&rows, choice));
            let noun = fmt::pluralize(total, "archetype", "archetypes");
            if nothing_chosen() {
                noun
            } else {
                format!("{} of {noun}", fmt::count(n))
            }
        }
    };
    let none_fit = move || !nothing_chosen() && shown.with(|s| s.compatible.is_empty());
    let show_all = {
        let route_for = route_for.clone();
        move |_| {
            push(&route_for(Vec::new(), None));
            // The button that was pressed goes away with the choice: focus moves to the first menu.
            let target = button_id(if has_features { &KEEP } else { &WITH });
            request_animation_frame(move || focus_by_id(&target));
        }
    };

    view! {
        <div class="arch-list">
            <div class="filter-row">
                {has_features.then(|| view! {
                    <div class="filter-menu">
                        <FilterMenuButton spec=KEEP state=keep_menu text=keep_text narrowed=keep_narrowed focus=false />
                    </div>
                })}
                <div class="filter-menu">
                    <FilterMenuButton spec=WITH state=with_menu text=with_text narrowed=with_narrowed focus=false />
                </div>
            </div>
            {has_features.then(keep_panel)}
            {with_panel}
            <p class="list-count" role="status">
                {count_line}
                {move || (!nothing_chosen()).then(|| view! {
                    " · "
                    <button type="button" class="count-action" on:click=show_all.clone()>"Show all"</button>
                })}
            </p>
            {move || (!nothing_chosen()).then(|| view! {
                <p class="hint">"Worked out from the recorded trades only, not from the text of each archetype."</p>
            })}
            {move || none_fit().then(|| view! {
                <p class="hint">"No archetype is known to fit that choice."</p>
            })}
            <ul class="entry-list arch-rows">{list}</ul>
            {unchecked}
        </div>
    }
}
