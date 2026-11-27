//! The filter menu: one button that opens an in-flow panel of choices, for a filter group that
//! is too large for a row of pills (DESIGN.md, "Filter menu").
//!
//! The panel is not a dialog. It sits in the page under the filter row, so the page keeps its
//! scroll and the phone's keyboard cannot cover it, and it needs no focus trap. Every choice is
//! a link to the filtered list, so the route stays the only state: choosing navigates, Back
//! undoes it. The rules for the labels and the find field are plain functions, tested natively.
//!
//! A multi-select menu ([`MultiFilterMenuPanel`]) is the same button and panel with checkbox rows.
//! The route is still the only state, but a choice is one more history entry that the page reads
//! without being rebuilt, so the panel stays open while the reader ticks several rows.

use crate::fmt;
use crate::views::icons;
use leptos::prelude::*;
use std::cell::Cell;
use wasm_bindgen::JsCast;

/// A group with more options than this gets a find field.
pub const FIND_THRESHOLD: usize = 10;

/// What a menu says about itself, in the words of its group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuSpec {
    /// The facet field the menu filters, such as `feat_type`.
    pub field: &'static str,
    /// The word before the value on the button: `Type`.
    pub group: &'static str,
    /// The first choice, and the button's value while nothing is chosen: `All types`.
    pub all_label: &'static str,
    /// The find field's name and placeholder: `Find a type`.
    pub find_label: &'static str,
    /// The accessible name of the list of choices: `Feat type`.
    pub list_label: &'static str,
    /// How the find status counts the choices: `type`, `types`.
    pub singular: &'static str,
    pub plural: &'static str,
}

/// One choice. `href` is the filtered list; `count` is how many entries it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuOption {
    pub value: String,
    pub label: String,
    pub count: u32,
    pub href: String,
}

pub fn needs_find(options: usize) -> bool {
    options > FIND_THRESHOLD
}

/// The option for the route's value. The index compares values without regard to case, so a
/// shared link with `combat` finds `Combat`.
pub fn current_option<'a>(
    options: &'a [MenuOption],
    value: Option<&str>,
) -> Option<&'a MenuOption> {
    let value = value?;
    options
        .iter()
        .find(|option| option.value.eq_ignore_ascii_case(value))
}

/// The button's text: `Type: All types`, or `Type: Combat` once a choice narrows the list.
pub fn trigger_text(spec: &MenuSpec, current: Option<&MenuOption>) -> String {
    format!(
        "{}: {}",
        spec.group,
        current.map_or(spec.all_label, |option| option.label.as_str())
    )
}

/// The options whose label contains `query`, without regard to case, in their given order.
pub fn matching<'a>(options: &'a [MenuOption], query: &str) -> Vec<&'a MenuOption> {
    let query = query.trim().to_lowercase();
    options
        .iter()
        .filter(|option| option.label.to_lowercase().contains(&query))
        .collect()
}

/// The find field's status line: `38 types`, or what to do when nothing matches.
pub fn find_status(spec: &MenuSpec, shown: usize, query: &str) -> String {
    if shown == 0 {
        format!("No {} match \"{}\".", spec.plural, query.trim())
    } else {
        format!(
            "{} {}",
            fmt::count(shown),
            if shown == 1 {
                spec.singular
            } else {
                spec.plural
            }
        )
    }
}

/// One row of a multi-select menu: the value the route carries, its label and its count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiOption {
    pub value: String,
    pub label: String,
    pub count: usize,
}

/// The button's text of a multi-select menu: `School: All schools`, `School: Evocation` for one
/// choice, `School: 2 chosen` for more. `labels` are the labels of the chosen rows.
pub fn multi_trigger_text(spec: &MenuSpec, labels: &[String]) -> String {
    match labels {
        [] => format!("{}: {}", spec.group, spec.all_label),
        [one] => format!("{}: {one}", spec.group),
        many => format!("{}: {} chosen", spec.group, fmt::count(many.len())),
    }
}

/// A row's accessible name: the group, the label, and the count when there is one: `CR 10, 4
/// creatures`, `School Evocation, 12 spells`. The group comes first so a reader of a screen reader
/// hears which menu a bare value such as `10` belongs to. `count` is `None` while counts load.
pub fn row_name(spec: &MenuSpec, label: &str, count: Option<usize>, noun: (&str, &str)) -> String {
    match count {
        Some(n) => format!(
            "{} {label}, {}",
            spec.group,
            fmt::pluralize(n, noun.0, noun.1)
        ),
        None => format!("{} {label}", spec.group),
    }
}

/// The chosen values with `value` ticked or unticked. A new choice goes last, so the route lists
/// the choices in the order they were made; values compare without regard to case, and a new
/// choice keeps the case it was given (a facet value such as `CR 1/2` is not a slug).
pub fn toggled_choice(chosen: &[String], value: &str) -> Vec<String> {
    if chosen.iter().any(|c| c.eq_ignore_ascii_case(value)) {
        chosen
            .iter()
            .filter(|c| !c.eq_ignore_ascii_case(value))
            .cloned()
            .collect()
    } else {
        let mut next = chosen.to_vec();
        next.push(value.to_string());
        next
    }
}

// ---- focus after a choice ---------------------------------------------------------------

thread_local! {
    /// Set when a choice is made, so the list page that the choice opens puts focus on that
    /// menu's button. The field names the menu: a page with two menus must focus only the one
    /// that changed. The page is rebuilt for the new route, so the request has to outlive it.
    static FOCUS_BUTTON_NEXT: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// Reads and clears the field of the menu whose choice opened the page. Call it once, when the
/// list page is built, so a request that nothing used cannot reach a later page.
pub fn take_focus_request() -> Option<&'static str> {
    FOCUS_BUTTON_NEXT.with(Cell::take)
}

fn request_focus(field: &'static str) {
    FOCUS_BUTTON_NEXT.with(|slot| slot.set(Some(field)));
}

pub(crate) fn button_id(spec: &MenuSpec) -> String {
    format!("filter-btn-{}", spec.field)
}

pub(crate) fn panel_id(spec: &MenuSpec) -> String {
    format!("filter-panel-{}", spec.field)
}

pub(crate) fn find_id(spec: &MenuSpec) -> String {
    format!("filter-find-{}", spec.field)
}

pub(crate) fn focus_by_id(id: &str) {
    let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(id))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    let _ = element.focus_with_options(&options);
}

/// Focuses `id` after the browser has painted twice. The route frame moves focus to the page
/// in the first frame after a route change, and the button must be focused after that.
fn focus_by_id_later(id: String) {
    request_animation_frame(move || {
        request_animation_frame(move || focus_by_id(&id));
    });
}

/// Whether the panel is open, and the text in its find field. Local to the list page: a new
/// route builds a new page, which starts closed.
#[derive(Clone, Copy)]
pub struct MenuState {
    pub(crate) open: RwSignal<bool>,
    pub(crate) query: RwSignal<String>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            query: RwSignal::new(String::new()),
        }
    }

    /// Closes the panel and puts focus back on its button.
    pub(crate) fn dismiss(self, spec: &MenuSpec) {
        self.open.set(false);
        self.query.set(String::new());
        focus_by_id(&button_id(spec));
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}

/// Closes `menu` when `pick`, the route's choice for it, changes under it (a choice, Back or
/// Forward), and drops the focus request that the choice's link left. A list page that a choice
/// does not rebuild never reads that request, so it is read here. The first run only reads `pick`.
pub fn dismiss_on_change(menu: MenuState, spec: MenuSpec, pick: Box<dyn Fn() -> Option<String>>) {
    Effect::new(move |previous: Option<Option<String>>| {
        let now = pick();
        if previous.is_some_and(|before| before != now) {
            take_focus_request();
            if menu.open.get_untracked() {
                menu.dismiss(&spec);
            }
        }
        now
    });
}

/// The button. Its visible text is its accessible name. `text` is `None` while the choices
/// load, which disables it.
#[component]
pub fn FilterMenuButton(
    spec: MenuSpec,
    state: MenuState,
    #[prop(into)] text: Signal<Option<String>>,
    narrowed: Signal<bool>,
    /// True when a choice just opened this page: take focus, so the reader keeps their place.
    focus: bool,
) -> impl IntoView {
    if focus {
        // The button is disabled while the choices load, and a disabled button cannot take focus.
        let mut done = false;
        crate::views::util::effect(move || {
            if done {
                return;
            }
            if text.with(Option::is_some) {
                focus_by_id_later(button_id(&spec));
                done = true;
            }
        });
    }
    view! {
        <button
            id=button_id(&spec)
            type="button"
            class="filter-btn"
            class:filter-btn-on=move || narrowed.get()
            aria-expanded=move || state.open.get().to_string()
            aria-controls=panel_id(&spec)
            disabled=move || text.with(Option::is_none)
            on:click=move |_| {
                if state.open.get_untracked() {
                    state.open.set(false);
                    state.query.set(String::new());
                } else {
                    state.open.set(true);
                }
            }
            on:keydown=move |event| {
                if event.key() == "Escape" && state.open.get_untracked() {
                    event.prevent_default();
                    state.dismiss(&spec);
                }
            }
        >
            <span class="filter-btn-text">
                {move || text.get().unwrap_or_else(|| format!("{}: Loading…", spec.group))}
            </span>
            {icons::chevron_down("filter-btn-chevron")}
        </button>
    }
}

/// The panel under the filter row: the find field above ten options, then the choices.
#[component]
pub fn FilterMenuPanel(
    spec: MenuSpec,
    state: MenuState,
    options: Vec<MenuOption>,
    /// The route's value, if the list is narrowed.
    current: Option<String>,
    /// The unfiltered list.
    all_href: String,
) -> impl IntoView {
    let find = needs_find(options.len());
    let current_value = current
        .as_deref()
        .and_then(|value| current_option(&options, Some(value)))
        .map(|option| option.value.clone());
    let options = crate::views::util::Shared::new(options);

    // Opening puts focus in the find field, or on the current choice when there is no field.
    crate::views::util::effect(move || {
        if state.open.get() {
            let target = if find {
                find_id(&spec)
            } else {
                format!("{}-current", panel_id(&spec))
            };
            request_animation_frame(move || focus_by_id(&target));
        }
    });

    // A value the library lacks leaves no row current: the list is empty, not "all".
    let all_current = current.is_none();
    let check = icons::choice_check;
    let row = {
        let current_value = current_value.clone();
        move |option: &MenuOption| {
            let is_current = current_value.as_deref() == Some(option.value.as_str());
            let name = format!("{}, {}", option.label, fmt::count(option.count));
            view! {
                <li>
                    <a
                        class="choice-row"
                        href=option.href.clone()
                        id=is_current.then(|| format!("{}-current", panel_id(&spec)))
                        aria-current=is_current.then_some("true")
                        aria-label=name
                        on:click=move |event| choose(event, is_current, state, &spec)
                    >
                        <span class="choice-label">{option.label.clone()}</span>
                        <span class="choice-count">{fmt::count(option.count)}</span>
                        {is_current.then(check)}
                    </a>
                </li>
            }
        }
    };
    let rows = {
        let options = options.clone();
        move || {
            let query = state.query.get();
            let shown = matching(&options, &query);
            shown.into_iter().map(&row).collect_view()
        }
    };
    let status = {
        let options = options.clone();
        move || {
            let query = state.query.get();
            find_status(&spec, matching(&options, &query).len(), &query)
        }
    };
    let none_match = {
        let options = options.clone();
        move || matching(&options, &state.query.get()).is_empty()
    };
    let all_row = move || {
        state.query.with(|query| query.trim().is_empty()).then(|| {
            let id = all_current.then(|| format!("{}-current", panel_id(&spec)));
            view! {
                <li>
                    <a
                        class="choice-row"
                        href=all_href.clone()
                        id=id
                        aria-current=all_current.then_some("true")
                        on:click=move |event| choose(event, all_current, state, &spec)
                    >
                        <span class="choice-label">{spec.all_label}</span>
                        {all_current.then(check)}
                    </a>
                </li>
            }
        })
    };

    let find = find.then(|| FindProps {
        status: Signal::derive(status),
        none_match: Signal::derive(none_match),
    });
    panel_shell(spec, state, find, view! { {all_row} {rows} }.into_any())
}

/// What the find field of a panel shows: its status line, and whether no choice matches.
struct FindProps {
    status: Signal<String>,
    none_match: Signal<bool>,
}

/// The panel that both menus share: the `div` that hides while the menu is shut and closes it on
/// Escape, the find field (`find` is `None` for a short list), and the list of choices.
fn panel_shell(
    spec: MenuSpec,
    state: MenuState,
    find: Option<FindProps>,
    items: AnyView,
) -> AnyView {
    view! {
        <div
            id=panel_id(&spec)
            class="filter-panel"
            hidden=move || !state.open.get()
            on:keydown=move |event| {
                if event.key() == "Escape" {
                    event.prevent_default();
                    state.dismiss(&spec);
                }
            }
        >
            {find.map(|FindProps { status, none_match }| view! {
                <div class="filter-find">
                    <input
                        id=find_id(&spec)
                        type="search"
                        class="filter-find-input"
                        placeholder=spec.find_label
                        aria-label=spec.find_label
                        autocomplete="off"
                        autocapitalize="off"
                        spellcheck="false"
                        enterkeyhint="search"
                        prop:value=move || state.query.get()
                        on:input=move |event| state.query.set(event_target_value(&event))
                    />
                    <p class="filter-find-status" role="status">{move || status.get()}</p>
                    <Show when=move || none_match.get()>
                        <button type="button" class="filter-find-clear"
                            on:click=move |_| {
                                state.query.set(String::new());
                                focus_by_id(&find_id(&spec));
                            }>"Clear"</button>
                    </Show>
                </div>
            })}
            <ul class="choice-list" aria-label=spec.list_label>{items}</ul>
        </div>
    }
    .into_any()
}

/// The multi-select panel under the filter row: an `All` link, then one checkbox row per option,
/// each with its count. A tick calls `on_change` with the new set of values; the page turns it
/// into a history entry. Above ten options a find field narrows the rows by label (the nine
/// schools of a spell list have none).
#[component]
pub fn MultiFilterMenuPanel(
    spec: MenuSpec,
    state: MenuState,
    options: Vec<MultiOption>,
    /// The values the route chooses now.
    chosen: Signal<Vec<String>>,
    /// The list with nothing chosen. A signal, because the page's other state, such as a name
    /// filter, changes under the open panel.
    all_href: Signal<String>,
    /// What a row's count counts, for its accessible name: `spell`, `spells`.
    count_noun: (&'static str, &'static str),
    on_change: Callback<Vec<String>>,
    /// Live counts by value, for a page whose counts follow its results. The rows are not
    /// rebuilt when a count changes, so a ticked row keeps focus. `None` for a value says the
    /// count is not known yet: the row shows no count. Without it the rows use `options`.
    #[prop(optional)]
    count_of: Option<Callback<String, Option<usize>>>,
) -> impl IntoView {
    let all_id = format!("{}-all", panel_id(&spec));
    // Above ten options the panel has a find field, as a single-choice menu does.
    let find = needs_find(options.len());
    // Opening puts focus in the find field, on the first ticked row, or on `All`.
    crate::views::util::effect({
        let all_id = all_id.clone();
        move || {
            if state.open.get() {
                let (panel, all_id) = (panel_id(&spec), all_id.clone());
                if find {
                    request_animation_frame(move || focus_by_id(&find_id(&spec)));
                } else {
                    request_animation_frame(move || focus_first_checked(&panel, &all_id));
                }
            }
        }
    });
    let options = crate::views::util::Shared::new(options);
    let matches = {
        let options = options.clone();
        move || -> Vec<MultiOption> {
            let query = state.query.get().trim().to_lowercase();
            options
                .iter()
                .filter(|option| option.label.to_lowercase().contains(&query))
                .cloned()
                .collect()
        }
    };
    let rows = {
        let matches = matches.clone();
        move || {
            matches()
                .into_iter()
                .map(|option| {
                    let value = option.value.clone();
                    let for_checked = option.value.clone();
                    let for_count = option.value.clone();
                    let (label, fallback) = (option.label.clone(), option.count);
                    // The count the row shows: the live one, or the one it was built with.
                    let count = Signal::derive(move || match count_of {
                        Some(live) => live.run(for_count.clone()),
                        None => Some(fallback),
                    });
                    let name = {
                        let label = label.clone();
                        move || row_name(&spec, &label, count.get(), count_noun)
                    };
                    view! {
                        <li>
                            <label class="choice-row" class:choice-empty=move || count.get() == Some(0)>
                                <input
                                    type="checkbox"
                                    aria-label=name
                                    prop:checked=move || {
                                        chosen.with(|c| c.iter().any(|v| v.eq_ignore_ascii_case(&for_checked)))
                                    }
                                    on:change=move |_| on_change.run(toggled_choice(&chosen.get_untracked(), &value))
                                />
                                <span class="choice-label">{label}</span>
                                <span class="choice-count">{move || count.get().map(fmt::count)}</span>
                            </label>
                        </li>
                    }
                })
                .collect_view()
        }
    };
    let status = {
        let matches = matches.clone();
        move || find_status(&spec, matches().len(), &state.query.get())
    };
    let none_match = move || matches().is_empty();
    let none_chosen = move || chosen.with(Vec::is_empty);
    let find = find.then(|| FindProps {
        status: Signal::derive(status),
        none_match: Signal::derive(none_match),
    });
    let items = view! {
        <li>
            <a
                class="choice-row"
                id=all_id
                href=move || all_href.get()
                aria-current=move || none_chosen().then_some("true")
                on:click=move |event| {
                    // The current choice only closes the panel, as in a single-choice menu.
                    if none_chosen() {
                        event.prevent_default();
                        state.dismiss(&spec);
                    }
                }
            >
                // The same box as the rows, drawn: it is ticked while nothing is chosen.
                // It is a link, so it carries no checkbox role; `aria-current` says it.
                <span class="choice-box" aria-hidden="true">
                    {move || none_chosen().then(icons::choice_check)}
                </span>
                <span class="choice-label">{spec.all_label}</span>
            </a>
        </li>
        {rows}
    };
    panel_shell(spec, state, find, items.into_any())
}

/// Focuses the first ticked checkbox in the panel, or the element `fallback` when none is.
fn focus_first_checked(panel: &str, fallback: &str) {
    let ticked = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| {
            document
                .query_selector(&format!("#{panel} input:checked"))
                .ok()
                .flatten()
        })
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
    match ticked {
        Some(element) => {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = element.focus_with_options(&options);
        }
        None => focus_by_id(fallback),
    }
}

/// A click on a choice. The current choice only closes the panel (its link would not change the
/// route). Any other choice navigates, and asks the new page to focus the button.
fn choose(event: web_sys::MouseEvent, is_current: bool, state: MenuState, spec: &MenuSpec) {
    if is_current {
        event.prevent_default();
        state.dismiss(spec);
    } else {
        request_focus(spec.field);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPE: MenuSpec = MenuSpec {
        field: "feat_type",
        group: "Type",
        all_label: "All types",
        find_label: "Find a type",
        list_label: "Feat type",
        singular: "type",
        plural: "types",
    };

    fn option(value: &str, count: u32) -> MenuOption {
        MenuOption {
            value: value.into(),
            label: value.into(),
            count,
            href: format!("#/browse/kind/feat?facet=feat_type:{value}"),
        }
    }

    fn options() -> Vec<MenuOption> {
        vec![
            option("Combat", 1324),
            option("Critical", 28),
            option("Combination", 7),
            option("General", 600),
        ]
    }

    #[test]
    fn a_find_field_appears_above_ten_options() {
        assert!(!needs_find(0));
        assert!(!needs_find(10));
        assert!(needs_find(11));
    }

    #[test]
    fn the_button_says_the_group_and_the_value() {
        let options = options();
        assert_eq!(trigger_text(&TYPE, None), "Type: All types");
        let combat = current_option(&options, Some("Combat"));
        assert_eq!(trigger_text(&TYPE, combat), "Type: Combat");
    }

    #[test]
    fn a_shared_link_finds_the_option_whatever_its_case() {
        let options = options();
        assert_eq!(
            current_option(&options, Some("combat")).map(|o| o.value.as_str()),
            Some("Combat")
        );
        assert_eq!(current_option(&options, Some("Missing")), None);
        assert_eq!(current_option(&options, None), None);
    }

    #[test]
    fn typing_narrows_the_options_by_label_and_keeps_their_order() {
        let options = options();
        let labels = |query: &str| -> Vec<&str> {
            matching(&options, query)
                .into_iter()
                .map(|o| o.label.as_str())
                .collect()
        };
        assert_eq!(labels(""), ["Combat", "Critical", "Combination", "General"]);
        assert_eq!(labels("comb"), ["Combat", "Combination"]);
        assert_eq!(labels("  CRIT "), ["Critical"]);
        assert_eq!(labels("ral"), ["General"]);
        assert!(labels("xyz").is_empty());
    }

    #[test]
    fn a_multi_select_button_names_one_choice_and_counts_several() {
        let schools = MenuSpec {
            field: "school",
            group: "School",
            all_label: "All schools",
            find_label: "Find a school",
            list_label: "Spell school",
            singular: "school",
            plural: "schools",
        };
        let one = |label: &str| vec![label.to_string()];
        assert_eq!(multi_trigger_text(&schools, &[]), "School: All schools");
        assert_eq!(
            multi_trigger_text(&schools, &one("Evocation")),
            "School: Evocation"
        );
        let two = ["Evocation".to_string(), "Illusion".to_string()];
        assert_eq!(multi_trigger_text(&schools, &two), "School: 2 chosen");
    }

    #[test]
    fn ticking_a_row_adds_it_last_and_ticking_it_again_removes_it() {
        let chosen = vec!["evocation".to_string()];
        assert_eq!(
            toggled_choice(&chosen, "illusion"),
            ["evocation", "illusion"]
        );
        assert_eq!(toggled_choice(&chosen, "Evocation"), Vec::<String>::new());
        // A new choice keeps its case: a value such as `CR 1/2` is not a slug.
        assert_eq!(toggled_choice(&[], "Illusion"), ["Illusion"]);
        assert_eq!(
            toggled_choice(&["Combat".to_string()], "combat"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_row_is_named_with_its_group_label_and_count() {
        let cr = MenuSpec {
            field: "cr",
            group: "CR",
            all_label: "All CRs",
            find_label: "Find a CR",
            list_label: "Challenge rating",
            singular: "rating",
            plural: "ratings",
        };
        let noun = ("creature", "creatures");
        assert_eq!(row_name(&cr, "10", Some(4), noun), "CR 10, 4 creatures");
        assert_eq!(row_name(&cr, "1/2", Some(1), noun), "CR 1/2, 1 creature");
        assert_eq!(
            row_name(&cr, "10", Some(1234), noun),
            "CR 10, 1,234 creatures"
        );
        assert_eq!(row_name(&cr, "10", Some(0), noun), "CR 10, 0 creatures");
        // While counts load the row has a name and no count.
        assert_eq!(row_name(&cr, "10", None, noun), "CR 10");
    }

    #[test]
    fn the_status_counts_the_choices_or_says_none_match() {
        assert_eq!(find_status(&TYPE, 38, ""), "38 types");
        assert_eq!(find_status(&TYPE, 1, "crit"), "1 type");
        assert_eq!(find_status(&TYPE, 1234, ""), "1,234 types");
        assert_eq!(find_status(&TYPE, 0, " xyz "), "No types match \"xyz\".");
    }
}
