//! The archetype list of a class page and its compatibility filter (#170, part D; DESIGN.md,
//! "Class pages"). The rules are in [`crate::archetypes`]; this file draws them.
//!
//! One row per archetype: the name as a link to the archetype's own page, the book, and what it
//! replaces and alters. Two filter menus narrow the list, in the form of DESIGN.md's filter menu:
//! `Keep` (features of the class, several at once) and `Combine with` (one archetype). The choice
//! is the state of this page's view and lives in the component, not in the route: the entry route
//! carries the entry, its anchor, its book and its section, and a filter that Back would have to
//! walk is not worth a new field on every one of those addresses.

use crate::archetypes::{self, Choice, FeatureOption, Row};
use crate::fmt;
use crate::graph::Relations;
use crate::library::Library;
use crate::route::Route;
use crate::views::filter_menu::{
    FilterMenuButton, MenuSpec, MenuState, find_id, focus_by_id, multi_trigger_text, needs_find,
    panel_id, toggled_choice,
};
use crate::views::icons;
use crate::views::related::source_title;
use crate::views::util::{Shared, sort_key};
use cast_iron_schema::{EntryId, RelationKind, TradeIndex};
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
        .sources_of(class, RelationKind::Archetype)
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

/// The `All` row of a menu: a button, ticked while nothing is chosen. It clears the choice.
fn any_row(
    spec: MenuSpec,
    state: MenuState,
    none: impl Fn() -> bool + Send + Sync + 'static,
    clear: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let none = Shared::new(none);
    let ticked = none.clone();
    view! {
        <li>
            <button
                type="button"
                class="choice-row"
                id=format!("{}-all", panel_id(&spec))
                aria-pressed=move || none().to_string()
                on:click=move |_| {
                    clear();
                    state.dismiss(&spec);
                }
            >
                <span class="choice-box" aria-hidden="true">
                    {move || ticked().then(icons::choice_check)}
                </span>
                <span class="choice-label">{spec.all_label}</span>
            </button>
        </li>
    }
}

#[component]
pub fn ArchetypeList(rows: Vec<Row>, features: Vec<FeatureOption>) -> impl IntoView {
    let total = rows.len();
    let rows = Shared::new(rows);
    let features = Shared::new(features);
    let keep: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let with: RwSignal<Option<EntryId>> = RwSignal::new(None);
    let keep_menu = MenuState::new();
    let with_menu = MenuState::new();
    let shown = {
        let rows = rows.clone();
        Memo::new(move |_| {
            archetypes::narrow(
                &rows,
                &Choice {
                    keep: keep.get(),
                    with: with.get(),
                },
            )
        })
    };
    let nothing_chosen = move || keep.with(Vec::is_empty) && with.with(Option::is_none);
    let show_all = move || {
        keep.set(Vec::new());
        with.set(None);
    };
    let has_features = !features.is_empty();

    // The buttons.
    let keep_text = {
        let features = features.clone();
        Signal::derive(move || {
            let labels: Vec<String> = keep.with(|chosen| {
                chosen
                    .iter()
                    .filter_map(|anchor| {
                        features
                            .iter()
                            .find(|feature| &feature.anchor == anchor)
                            .map(|feature| feature.label.clone())
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
                id.as_ref()
                    .and_then(|id| rows.iter().find(|row| &row.id == id))
                    .map(|row| row.title.clone())
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

    // Opening a panel puts focus in its find field, or on its first row.
    Effect::new(move |_| {
        if keep_menu.open.get() {
            let target = format!("{}-all", panel_id(&KEEP));
            request_animation_frame(move || focus_by_id(&target));
        }
    });
    let with_find = needs_find(total);
    Effect::new(move |_| {
        if with_menu.open.get() {
            let target = if with_find {
                find_id(&WITH)
            } else {
                format!("{}-all", panel_id(&WITH))
            };
            request_animation_frame(move || focus_by_id(&target));
        }
    });

    // The features to keep: one checkbox row each, with how many archetypes touch it.
    let keep_rows = features
        .iter()
        .map(|feature| {
            let (anchor, for_checked) = (feature.anchor.clone(), feature.anchor.clone());
            let name = format!(
                "{}, {}",
                feature.label,
                fmt::pluralize(feature.count, "archetype", "archetypes")
            );
            view! {
                <li>
                    <label class="choice-row">
                        <input
                            type="checkbox"
                            aria-label=name
                            prop:checked=move || keep.with(|chosen| chosen.contains(&for_checked))
                            on:change=move |_| {
                                keep.set(toggled_choice(&keep.get_untracked(), &anchor));
                            }
                        />
                        <span class="choice-label">{feature.label.clone()}</span>
                        <span class="choice-count">{fmt::count(feature.count)}</span>
                    </label>
                </li>
            }
        })
        .collect_view();

    // The archetype to combine with: one button row each, filtered by the find field.
    let with_rows = {
        let rows = rows.clone();
        move || {
            let query = with_menu.query.get().trim().to_lowercase();
            rows.iter()
                .filter(|row| row.title.to_lowercase().contains(&query))
                .map(|row| {
                    let (id, chosen_id) = (row.id.clone(), row.id.clone());
                    let chosen = move || with.with(|current| current.as_ref() == Some(&chosen_id));
                    let ticked = chosen.clone();
                    view! {
                        <li>
                            <button
                                type="button"
                                class="choice-row"
                                aria-pressed=move || chosen().to_string()
                                on:click=move |_| {
                                    with.set(Some(id.clone()));
                                    with_menu.dismiss(&WITH);
                                }
                            >
                                <span class="choice-label">{row.title.clone()}</span>
                                {move || ticked().then(icons::choice_check)}
                            </button>
                        </li>
                    }
                })
                .collect_view()
        }
    };
    let with_status = {
        let rows = rows.clone();
        move || {
            let query = with_menu.query.get();
            let n = rows
                .iter()
                .filter(|row| {
                    row.title
                        .to_lowercase()
                        .contains(&query.trim().to_lowercase())
                })
                .count();
            crate::views::filter_menu::find_status(&WITH, n, &query)
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
    let unchecked = {
        let rows = rows.clone();
        move || {
            shown.with(|s| {
                (!s.unchecked.is_empty()).then(|| {
                    let items = s
                        .unchecked
                        .iter()
                        .map(|&index| row_view(&rows[index]))
                        .collect_view();
                    view! {
                        <h3 class="arch-unchecked">"Not checked"</h3>
                        <p class="hint">
                            "Some trades of these are not matched to a class feature, so the filter cannot say whether they fit."
                        </p>
                        <ul class="entry-list arch-rows">{items}</ul>
                    }
                })
            })
        }
    };
    let count_line = move || {
        let n = shown.with(|s| s.compatible.len());
        if nothing_chosen() {
            fmt::pluralize(total, "archetype", "archetypes")
        } else {
            format!(
                "{} of {}",
                fmt::count(n),
                fmt::pluralize(total, "archetype", "archetypes")
            )
        }
    };
    let none_fit = move || {
        !nothing_chosen() && shown.with(|s| s.compatible.is_empty() && s.unchecked.is_empty())
    };
    let with_note = {
        let rows = rows.clone();
        move || {
            with.with(|id| {
                id.as_ref()
                    .and_then(|id| rows.iter().find(|row| &row.id == id))
                    .filter(|row| !row.is_checked())
                    .map(|row| {
                        format!(
                            "Some trades of {} name no class feature, so they are not compared.",
                            row.title
                        )
                    })
            })
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
            {has_features.then(|| view! {
                <div
                    id=panel_id(&KEEP)
                    class="filter-panel"
                    hidden=move || !keep_menu.open.get()
                    on:keydown=move |event| {
                        if event.key() == "Escape" {
                            event.prevent_default();
                            keep_menu.dismiss(&KEEP);
                        }
                    }
                >
                    <ul class="choice-list" aria-label=KEEP.list_label>
                        {any_row(
                            KEEP,
                            keep_menu,
                            move || keep.with(Vec::is_empty),
                            move || keep.set(Vec::new()),
                        )}
                        {keep_rows}
                    </ul>
                </div>
            })}
            <div
                id=panel_id(&WITH)
                class="filter-panel"
                hidden=move || !with_menu.open.get()
                on:keydown=move |event| {
                    if event.key() == "Escape" {
                        event.prevent_default();
                        with_menu.dismiss(&WITH);
                    }
                }
            >
                {with_find.then(|| view! {
                    <div class="filter-find">
                        <input
                            id=find_id(&WITH)
                            type="search"
                            class="filter-find-input"
                            placeholder=WITH.find_label
                            aria-label=WITH.find_label
                            autocomplete="off"
                            autocapitalize="off"
                            spellcheck="false"
                            enterkeyhint="search"
                            prop:value=move || with_menu.query.get()
                            on:input=move |event| with_menu.query.set(event_target_value(&event))
                        />
                        <p class="filter-find-status" role="status">{with_status}</p>
                    </div>
                })}
                <ul class="choice-list" aria-label=WITH.list_label>
                    {any_row(
                        WITH,
                        with_menu,
                        move || with.with(Option::is_none),
                        move || with.set(None),
                    )}
                    {with_rows}
                </ul>
            </div>
            <p class="list-count" role="status">
                {count_line}
                {move || (!nothing_chosen()).then(|| view! {
                    " · "
                    <button type="button" class="count-action" on:click=move |_| show_all()>"Show all"</button>
                })}
            </p>
            {move || with_note().map(|note| view! { <p class="hint">{note}</p> })}
            {move || none_fit().then(|| view! {
                <p class="hint">"No archetype fits that choice."</p>
            })}
            <ul class="entry-list arch-rows">{list}</ul>
            {unchecked}
        </div>
    }
}
