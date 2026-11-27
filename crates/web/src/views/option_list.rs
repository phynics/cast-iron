//! The Options section of a class page (#739; DESIGN.md, "Options section"): a chip for each
//! option group of the class, and under the chips the group's options as rows, with a name field,
//! a book menu and (where the printed options carry a level) a level menu. The rules are in
//! [`crate::option_list`]; this file draws them.
//!
//! The choices are the route's (`?s=options&group=<slug>&find=<text>&source=<book>&level=<n>`):
//! a chip or a menu choice is one more history entry, typing replaces the current one, and the
//! page is not rebuilt for any of it, so Back undoes a choice and a shared link opens the same
//! list. The options of a group are read from the one or two shards that hold them when the group
//! is shown, and kept for the page's life.

use crate::class_options;
use crate::class_sections::Section;
use crate::fmt;
use crate::library::Library;
use crate::option_list::{self, OptionRow};
use crate::route::{ArchFilter, OptionChoice, Route, push, replace, use_route};
use crate::views::filter_menu::{
    FilterMenuButton, FilterMenuPanel, MenuOption, MenuSpec, MenuState, dismiss_on_change,
    focus_by_id,
};
use crate::views::icons;
use crate::views::util::{Shared, sort_key};
use cast_iron_schema::{EntryId, OptionGroup, OptionGroupIndex, TypedFields};
use leptos::prelude::*;
use std::collections::BTreeMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;

const BOOK: MenuSpec = MenuSpec {
    field: "source",
    group: "Book",
    all_label: "All books",
    find_label: "Find a book",
    list_label: "Book",
    singular: "book",
    plural: "books",
};

const LEVEL: MenuSpec = MenuSpec {
    field: "level",
    group: "Level",
    all_label: "All levels",
    find_label: "Find a level",
    list_label: "Level",
    singular: "level",
    plural: "levels",
};

/// The id of the name field, for the focus that `Show all` returns to.
const FIND_ID: &str = "opt-find";

/// The page position that a late list must not move.
///
/// The rows of a group arrive after the page has rendered, and the section grows from one
/// `Loading` line to the whole list. When a jump (a stop, an `?a=` anchor, a first load at
/// `?s=archetypes`) has put something below the section at the top of the view, that growth would
/// push it down. The rule: when the rows arrive and the section lies wholly above the top of the
/// view (below the sticky bar), the element after the section is held where it was by scrolling
/// the page by the distance it moved. The reader's own place is kept the same way, so a reader who
/// has scrolled since the jump is not moved back to its target; a reader who is at or above the
/// section is not moved at all. The browser's scroll anchoring is not relied on: the measure is
/// taken after a forced layout, so it counts only what anchoring left over.
pub struct Hold {
    below: web_sys::HtmlElement,
    /// Where the element sits in the view, in pixels from its top.
    in_view: f64,
}

/// The top of an element in the document, from the offsets of its parents. (`getBoundingClientRect`
/// needs a `web-sys` feature that the crate does not enable, and the offsets read the same layout.)
fn document_top(element: &web_sys::HtmlElement) -> f64 {
    let mut top = 0.0;
    let mut current = Some(element.clone());
    while let Some(element) = current {
        top += f64::from(element.offset_top());
        current = element
            .offset_parent()
            .and_then(|parent| parent.dyn_into::<web_sys::HtmlElement>().ok());
    }
    top
}

fn scrolled() -> f64 {
    web_sys::window()
        .and_then(|window| window.scroll_y().ok())
        .unwrap_or(0.0)
}

/// How far down the page the sticky bar of a class page covers the view: the masthead and the
/// switcher.
fn covered_to() -> f64 {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return 0.0;
    };
    ["header.app-header", ".section-switcher"]
        .iter()
        .filter_map(|selector| document.query_selector(selector).ok().flatten())
        .filter_map(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        .map(|element| f64::from(element.offset_height()))
        .sum()
}

impl Hold {
    /// The element after the section and its place in the view, when the section is above it.
    fn take(section: NodeRef<leptos::html::Section>) -> Option<Hold> {
        let section = section.get_untracked()?;
        let bottom = document_top(&section) + f64::from(section.offset_height());
        if bottom > scrolled() + covered_to() {
            return None;
        }
        let below = section
            .next_element_sibling()?
            .dyn_into::<web_sys::HtmlElement>()
            .ok()?;
        let in_view = document_top(&below) - scrolled();
        Some(Hold { below, in_view })
    }

    /// Scrolls the page by what the held element has moved in the view since [`Self::take`]. The
    /// read forces layout, so the browser's own anchoring has been applied and only what it left is
    /// counted.
    fn settle(self) {
        if !self.below.is_connected() {
            return;
        }
        let moved = document_top(&self.below) - scrolled() - self.in_view;
        if moved.abs() > 1.5
            && let Some(window) = web_sys::window()
        {
            window.scroll_by_with_x_and_y(0.0, moved);
        }
    }
}

/// The option groups of `class`, each with only the options that the active books keep (its
/// `entries`), and without a group that has none left.
pub fn groups_of(library: &Library, index: &OptionGroupIndex, class: &EntryId) -> Vec<OptionGroup> {
    let discovery = library.discovery();
    let eligible = |id: &EntryId| discovery.is_eligible(id.as_str());
    class_options::groups_for_class(index, class, eligible)
        .into_iter()
        .map(|(group, _)| OptionGroup {
            entries: group
                .entries
                .iter()
                .filter(|id| eligible(id))
                .cloned()
                .collect(),
            ..group.clone()
        })
        .collect()
}

/// The address of the Options section, at `choice`. Any choice of the archetype list stays.
fn options_route(
    class: &EntryId,
    book: &Option<String>,
    arch: ArchFilter,
    options: OptionChoice,
) -> Route {
    Route::Entry {
        id: class.clone(),
        anchor: None,
        book: book.clone(),
        section: Some(Section::Options),
        arch: ArchFilter {
            options: Box::new(options),
            ..arch
        },
    }
}

/// The address that opens the Options section at `group`, for the row under the feature that
/// grants a choice from it.
pub fn group_route(class: &EntryId, book: &Option<String>, arch: ArchFilter, group: &str) -> Route {
    options_route(
        class,
        book,
        arch,
        OptionChoice {
            group: Some(group.to_string()),
            ..Default::default()
        },
    )
}

/// The row of one group's options, read from the shards that hold them: the title, the facts of
/// the printed text and the book the reader's active books give it.
async fn load(library: &Library, ids: &[EntryId]) -> Result<Vec<OptionRow>, String> {
    let read = library
        .read_entries(ids, |entry| {
            let ability_type = match &entry.fields {
                TypedFields::ClassOption(fields) => fields.ability_type.as_deref(),
                _ => None,
            };
            option_list::facts_with_type(&entry.title, &entry.body, ability_type)
        })
        .await
        .map_err(|error| error.to_string())?;
    let mut rows: Vec<OptionRow> = read
        .into_iter()
        .filter_map(|(id, facts)| {
            let stub = library.stub(&id)?;
            let source = library.row_book(stub).to_string();
            let book = library
                .source(&source)
                .map_or_else(|| source.clone(), |info| info.title.clone());
            Some(OptionRow {
                title: stub.title.clone(),
                id,
                facts,
                source,
                book,
            })
        })
        .collect();
    cast_iron_sort::sort_by_key(&mut rows, |row| (sort_key(&row.title), row.id.clone()));
    Ok(rows)
}

fn row_view(row: &OptionRow) -> impl IntoView + use<> {
    let tag = row
        .facts
        .tag
        .clone()
        .map(|tag| view! { <em class="opt-tag">{tag}</em> });
    let aside = row
        .aside()
        .map(|text| view! { <span class="opt-aside">{text}</span> });
    let line = match row.prerequisite_line() {
        Some(prerequisite) => format!("{} · {prerequisite}", row.book),
        None => row.book.clone(),
    };
    view! {
        <li class="opt-row">
            <span class="opt-title">
                <a href=Route::entry(row.id.clone()).href()>
                    <span class="opt-name">{row.title.clone()}</span>
                </a>
                {tag}
            </span>
            {aside}
            <p class="opt-meta muted">{line}</p>
        </li>
    }
}

#[component]
pub fn OptionsSection(
    class: EntryId,
    book: Option<String>,
    groups: Vec<OptionGroup>,
) -> impl IntoView {
    let route = use_route();
    let library = crate::library::use_library();
    let groups = Shared::new(groups);
    let choice = Memo::new(move |_| route.with(|route| *route.arch().options));
    // The route's group, or the class's first when it names none that the class has.
    let shown = {
        let groups = groups.clone();
        Memo::new(move |_| {
            choice.with(|choice| {
                option_list::shown_group(&groups, |group| &group.slug, choice.group.as_deref())
                    .map_or(0, |group| {
                        groups
                            .iter()
                            .position(|g| g.slug == group.slug)
                            .unwrap_or(0)
                    })
            })
        })
    };
    let route_for = {
        let (class, book) = (class.clone(), book.clone());
        Shared::new(move |options: OptionChoice| {
            options_route(&class, &book, route.with(Route::arch), options)
        })
    };
    // The same choices with the shown group's slug filled in.
    let with_group = {
        let (groups, route_for) = (groups.clone(), route_for.clone());
        Shared::new(move |edit: &dyn Fn(&mut OptionChoice)| {
            let mut next = choice.get_untracked();
            next.group = Some(groups[shown.get_untracked()].slug.clone());
            edit(&mut next);
            route_for(next)
        })
    };

    let section_ref = NodeRef::<leptos::html::Section>::new();
    // The rows of the shown group: read once per group, kept for the page's life.
    let (rows, set_rows) = signal_local(None::<Result<Rc<Vec<OptionRow>>, String>>);
    let cache: StoredValue<BTreeMap<String, Rc<Vec<OptionRow>>>, LocalStorage> =
        StoredValue::new_local(BTreeMap::new());
    {
        let groups = groups.clone();
        let library = library.clone();
        crate::views::util::effect(move || {
            let index = shown.get();
            let (slug, ids) = (groups[index].slug.clone(), groups[index].entries.clone());
            if let Some(found) = cache.with_value(|cache| cache.get(&slug).cloned()) {
                set_rows.set(Some(Ok(found)));
                return;
            }
            set_rows.set(None);
            let library = library.clone();
            leptos::task::spawn_local(async move {
                let result = load(&library, &ids).await.map(Rc::new);
                if let Ok(found) = &result {
                    cache.update_value(|cache| {
                        cache.insert(slug.clone(), found.clone());
                    });
                }
                // A newer group has been chosen since: its own read answers for it.
                if shown.try_get_untracked() == Some(index) {
                    let held = Hold::take(section_ref);
                    set_rows.set(Some(result));
                    request_animation_frame(move || {
                        if let Some(held) = held {
                            held.settle();
                        }
                    });
                }
            });
        });
    }

    let singular = {
        let groups = groups.clone();
        move || groups[shown.get()].singular.to_lowercase()
    };
    let plural = {
        let groups = groups.clone();
        move || groups[shown.get()].label.to_lowercase()
    };

    // Chips.
    let chips = groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let (slug, route_for) = (group.slug.clone(), route_for.clone());
            let href = move || {
                route_for(OptionChoice {
                    group: Some(slug.clone()),
                    ..Default::default()
                })
                .href()
            };
            view! {
                <li>
                    <a
                        class="jump"
                        href=href
                        aria-current=move || (shown.get() == index).then_some("true")
                    >
                        {group.label.clone()}
                        <span class="jump-count">{fmt::count(group.entries.len())}</span>
                    </a>
                </li>
            }
        })
        .collect_view();

    // The rows that the choices keep, as indexes into the loaded rows.
    let kept = Memo::new(move |_| {
        rows.with(|rows| match rows {
            Some(Ok(rows)) => choice.with(|choice| option_list::narrow(rows, choice)),
            _ => Vec::new(),
        })
    });
    let narrowed = move || choice.with(OptionChoice::narrows);

    // The name field. Typing replaces the history entry, written on the next animation frame from
    // the route as it is then, so a fast typist rebuilds the list once per frame and a stale field
    // cannot write into another page.
    let typed: StoredValue<Option<String>> = StoredValue::new(None);
    let on_find = {
        let (with_group, class) = (with_group.clone(), class.clone());
        move |text: String| {
            let waiting = typed.with_value(Option::is_some);
            typed.set_value(Some(text));
            if waiting {
                return;
            }
            let (with_group, class) = (with_group.clone(), class.clone());
            request_animation_frame(move || {
                let Some(text) = typed.try_update_value(Option::take).flatten() else {
                    return;
                };
                if matches!(route.get_untracked(), Route::Entry { id, .. } if id == class) {
                    replace(&with_group(&|next| next.find = text.clone()));
                }
            });
        }
    };

    // The two menus. A menu choice closes its panel and puts focus back on the button, as in the
    // archetype list; this page has no rebuild to read the focus request, so it is read here.
    let book_menu = MenuState::new();
    let level_menu = MenuState::new();
    for (menu, spec, pick) in [
        (
            book_menu,
            BOOK,
            Box::new(move || choice.with(|choice| choice.source.clone())) as Box<dyn Fn() -> _>,
        ),
        (
            level_menu,
            LEVEL,
            Box::new(move || choice.with(|choice| choice.level.map(|level| level.to_string()))),
        ),
    ] {
        dismiss_on_change(menu, spec, pick);
    }

    let book_text = Signal::derive(move || {
        rows.with(|rows| {
            let Some(Ok(rows)) = rows else {
                return None;
            };
            let title = choice.with(|choice| {
                choice.source.as_ref().map(|source| {
                    rows.iter()
                        .find(|row| &row.source == source)
                        .map_or_else(|| "not in this list".to_string(), |row| row.book.clone())
                })
            });
            Some(format!(
                "{}: {}",
                BOOK.group,
                title.as_deref().unwrap_or(BOOK.all_label)
            ))
        })
    });
    let book_on = Signal::derive(move || choice.with(|choice| choice.source.is_some()));
    let book_panel = {
        let with_group = with_group.clone();
        move || {
            rows.with(|rows| {
                let Some(Ok(rows)) = rows else {
                    return ().into_any();
                };
                let current = choice.get();
                let options: Vec<MenuOption> = option_list::book_counts(rows, &current)
                    .into_iter()
                    .map(|(source, title, count)| MenuOption {
                        href: with_group(&|next| next.source = Some(source.clone())).href(),
                        value: source,
                        label: title,
                        count: count as u32,
                    })
                    .collect();
                let all_href = with_group(&|next| next.source = None).href();
                view! { <FilterMenuPanel spec=BOOK state=book_menu options current=current.source all_href /> }
                    .into_any()
            })
        }
    };

    // Only a group whose printed options carry a level has a level menu.
    let has_levels = Memo::new(move |_| {
        rows.with(|rows| match rows {
            Some(Ok(rows)) => option_list::level_counts(rows, &OptionChoice::default()).len() > 1,
            _ => false,
        })
    });
    let level_text = Signal::derive(move || {
        rows.with(|rows| {
            rows.as_ref().and_then(|rows| rows.as_ref().ok()).map(|_| {
                let label = choice.with(|choice| choice.level.map(|level| level.to_string()));
                format!(
                    "{}: {}",
                    LEVEL.group,
                    label.as_deref().unwrap_or(LEVEL.all_label)
                )
            })
        })
    });
    let level_on = Signal::derive(move || choice.with(|choice| choice.level.is_some()));
    let level_panel = {
        let with_group = with_group.clone();
        move || {
            rows.with(|rows| {
                let Some(Ok(rows)) = rows else {
                    return ().into_any();
                };
                let current = choice.get();
                let options: Vec<MenuOption> = option_list::level_counts(rows, &current)
                    .into_iter()
                    .map(|(level, count)| MenuOption {
                        href: with_group(&|next| next.level = Some(level)).href(),
                        value: level.to_string(),
                        label: level.to_string(),
                        count: count as u32,
                    })
                    .collect();
                let all_href = with_group(&|next| next.level = None).href();
                let level = current.level.map(|level| level.to_string());
                view! { <FilterMenuPanel spec=LEVEL state=level_menu options current=level all_href /> }
                    .into_any()
            })
        }
    };

    // The count line, and the empty and loading states.
    let count_line = {
        let (singular, plural) = (singular.clone(), plural.clone());
        move || {
            rows.with(|rows| match rows {
                None => format!("Loading {}…", plural()),
                Some(Err(_)) => String::new(),
                Some(Ok(rows)) => {
                    let noun = fmt::pluralize(rows.len(), &singular(), &plural());
                    let shown = kept.with(Vec::len);
                    if narrowed() {
                        format!("{} of {noun}", fmt::count(shown))
                    } else {
                        noun
                    }
                }
            })
        }
    };
    let show_all = {
        let with_group = with_group.clone();
        move |_| {
            push(&with_group(&|next| {
                next.find = String::new();
                next.source = None;
                next.level = None;
            }));
            request_animation_frame(|| focus_by_id(FIND_ID));
        }
    };
    let list = move || {
        rows.with(|rows| match rows {
            Some(Ok(rows)) => kept
                .with(|kept| {
                    kept.iter()
                        .map(|&index| row_view(&rows[index]).into_any())
                        .collect_view()
                })
                .into_any(),
            _ => ().into_any(),
        })
    };
    let notice = {
        let plural = plural.clone();
        move || {
            rows.with(|rows| match rows {
                Some(Err(_)) => {
                    Some("The options could not be loaded. Search still works.".to_string())
                }
                Some(Ok(_)) if kept.with(Vec::is_empty) => {
                    Some(match choice.with(|choice| choice.find.trim().to_string()) {
                        text if !text.is_empty()
                            && choice.with(|c| c.source.is_none() && c.level.is_none()) =>
                        {
                            format!("No {} match \"{text}\".", plural())
                        }
                        _ => format!("No {} match those choices.", plural()),
                    })
                }
                _ => None,
            })
        }
    };

    view! {
        <section class="opt-section" aria-labelledby="opt-section" node_ref=section_ref>
            <h2 id="opt-section" data-class-section=Section::Options.slug() data-printed-level="3" tabindex="-1">
                "Options"
            </h2>
            <nav aria-label="Option groups">
                <ul class="opt-chips">{chips}</ul>
            </nav>
            <div class="opt-list">
                <form class="search-form list-filter" role="search" on:submit=|event| event.prevent_default()>
                    <label class="sr-only" for=FIND_ID>"Filter this list by name"</label>
                    <input
                        id=FIND_ID
                        type="search"
                        placeholder="Filter this list by name"
                        autocomplete="off"
                        autocapitalize="off"
                        spellcheck="false"
                        prop:value=move || choice.with(|choice| choice.find.clone())
                        on:input=move |event| on_find(event_target_value(&event))
                    />
                </form>
                <div class="filter-row">
                    <div class="filter-menu">
                        <FilterMenuButton spec=BOOK state=book_menu text=book_text narrowed=book_on focus=false />
                    </div>
                    <Show when=move || has_levels.get()>
                        <div class="filter-menu">
                            <FilterMenuButton spec=LEVEL state=level_menu text=level_text narrowed=level_on focus=false />
                        </div>
                    </Show>
                </div>
                {book_panel}
                <Show when=move || has_levels.get()>{level_panel.clone()}</Show>
                <p class="list-count" role="status">
                    {count_line}
                    {move || narrowed().then(|| view! {
                        " · "
                        <button type="button" class="count-action" on:click=show_all.clone()>"Show all"</button>
                    })}
                </p>
                {move || notice().map(|text| view! { <p class="hint">{text}</p> })}
                <ul class="entry-list opt-rows">{list}</ul>
            </div>
        </section>
    }
}

/// The row under the class feature that grants a choice from `group`: `Rogue talents · 234 ›`,
/// which opens the Options section at that group.
#[component]
pub fn OptionsLink(
    class: EntryId,
    book: Option<String>,
    slug: String,
    label: String,
    count: usize,
) -> impl IntoView {
    let route = use_route();
    let target = {
        let (class, book, slug) = (class, book, slug);
        move || group_route(&class, &book, route.with(Route::arch), &slug)
    };
    let href = {
        let target = target.clone();
        move || target().href()
    };
    let name = format!(
        "{label}, {} in your books. Open the Options section.",
        fmt::pluralize(count, "option", "options")
    );
    view! {
        <p class="opt-link">
            <a
                class="page-link"
                href=href
                aria-label=name
                on:click=move |event: leptos::ev::MouseEvent| {
                    // The move changes the choices of the section, and the frame leaves scroll
                    // alone for a choice (the reader is looking at the list): scroll to the
                    // section's top here, from wherever the row is.
                    let plain = event.button() == 0
                        && !event.ctrl_key()
                        && !event.meta_key()
                        && !event.shift_key()
                        && !event.alt_key();
                    if plain {
                        event.prevent_default();
                        push(&target());
                        request_animation_frame(|| {
                            crate::route::scroll_to_section(Section::Options);
                        });
                    }
                }
            >
                <span>{format!("{label} · {}", fmt::count(count))}</span>
                {icons::chevron_right_large()}
            </a>
        </p>
    }
}
