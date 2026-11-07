//! The filter menu: one button that opens an in-flow panel of choices, for a filter group that
//! is too large for a row of pills (DESIGN.md, "Filter menu").
//!
//! The panel is not a dialog. It sits in the page under the filter row, so the page keeps its
//! scroll and the phone's keyboard cannot cover it, and it needs no focus trap. Every choice is
//! a link to the filtered list, so the route stays the only state: choosing navigates, Back
//! undoes it. The rules for the labels and the find field are plain functions, tested natively.

use crate::fmt;
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

fn button_id(spec: &MenuSpec) -> String {
    format!("filter-btn-{}", spec.field)
}

fn panel_id(spec: &MenuSpec) -> String {
    format!("filter-panel-{}", spec.field)
}

fn find_id(spec: &MenuSpec) -> String {
    format!("filter-find-{}", spec.field)
}

fn focus_by_id(id: &str) {
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
    open: RwSignal<bool>,
    query: RwSignal<String>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            query: RwSignal::new(String::new()),
        }
    }

    /// Closes the panel and puts focus back on its button.
    fn dismiss(self, spec: &MenuSpec) {
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

fn chevron() -> impl IntoView {
    view! {
        <svg class="filter-btn-chevron" viewBox="0 0 24 24" width="20" height="20" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true">
            <path d="m6 9 6 6 6-6"/>
        </svg>
    }
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
        Effect::new(move |done: Option<bool>| {
            if done == Some(true) {
                return true;
            }
            let ready = text.with(Option::is_some);
            if ready {
                focus_by_id_later(button_id(&spec));
            }
            ready
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
            {chevron()}
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
    Effect::new(move |_| {
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
    let check = || {
        view! {
            <svg class="choice-check" viewBox="0 0 24 24" width="20" height="20" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round"
                stroke-linejoin="round" aria-hidden="true">
                <path d="m5 12.5 4.5 4.5L19 7.5"/>
            </svg>
        }
    };
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
            {find.then(|| view! {
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
                    <p class="filter-find-status" role="status">{status}</p>
                    <Show when=none_match>
                        <button type="button" class="filter-find-clear"
                            on:click=move |_| {
                                state.query.set(String::new());
                                focus_by_id(&find_id(&spec));
                            }>"Clear"</button>
                    </Show>
                </div>
            })}
            <ul class="choice-list" aria-label=spec.list_label>
                {all_row}
                {rows}
            </ul>
        </div>
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
    fn the_status_counts_the_choices_or_says_none_match() {
        assert_eq!(find_status(&TYPE, 38, ""), "38 types");
        assert_eq!(find_status(&TYPE, 1, "crit"), "1 type");
        assert_eq!(find_status(&TYPE, 1234, ""), "1,234 types");
        assert_eq!(find_status(&TYPE, 0, " xyz "), "No types match \"xyz\".");
    }
}
