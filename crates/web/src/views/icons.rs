//! The inline SVG icon set. Every icon uses the same 24 px grid, a 1.8 px round stroke and
//! `currentColor`, so it follows the theme in light and dark. Icons are always `aria-hidden`:
//! the control around them carries the accessible name. There is no remote asset and no icon
//! font (AGENTS.md rule 1), and no raw HTML (rule 2).
//!
//! The star reuses the path already drawn on an entry and in the Home list. The chevrons match
//! the filter menu and the book rows.

use leptos::prelude::*;

/// An outlined star: nothing is bookmarked.
pub fn star() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
            stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9z"/>
        </svg>
    }
}

/// A filled star: the entry is bookmarked.
pub fn star_filled() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor" stroke="currentColor"
            stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9z"/>
        </svg>
    }
}

/// A down chevron for a filter toggle. Its open state rotates it in CSS.
pub fn chevron_down(class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" width="20" height="20" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="m6 9 6 6 6-6"/>
        </svg>
    }
}

/// A right chevron for a disclosure that opens downwards. The open state rotates it in CSS.
pub fn chevron_right(class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" width="16" height="16" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="m9 6 6 6-6 6"/>
        </svg>
    }
}

/// A right chevron for a page link: the larger size, with no class of its own.
pub fn chevron_right_large() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
            stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m9 6 6 6-6 6"/>
        </svg>
    }
}

/// The chevron at the end of a book row. CSS sizes it.
pub fn book_chevron() -> impl IntoView {
    view! {
        <svg class="book-chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m9 6 6 6-6 6"/>
        </svg>
    }
}

/// The chevron of a book tier heading. Its open state rotates it in CSS.
pub fn tier_chevron() -> impl IntoView {
    view! {
        <svg class="tier-chevron" viewBox="0 0 24 24" width="16" height="16" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="m9 6 6 6-6 6"/>
        </svg>
    }
}

/// A left arrow, for going back.
pub fn arrow_left(class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" width="16" height="16" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="M19 12H5"/><path d="m11 6-6 6 6 6"/>
        </svg>
    }
}

/// A right arrow, for going forward.
pub fn arrow_right(class: &'static str) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" width="16" height="16" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="M5 12h14"/><path d="m13 6 6 6-6 6"/>
        </svg>
    }
}

/// A check mark, for a finished step.
pub fn check() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
            stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m5 12.5 4.5 4.5L19 7.5"/>
        </svg>
    }
}

/// The check mark of the preset that matches the active set.
pub fn preset_check() -> impl IntoView {
    view! {
        <svg class="preset-check" viewBox="0 0 24 24" width="20" height="20" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="m5 12.5 4.5 4.5L19 7.5"/>
        </svg>
    }
}

/// The check mark of the current filter choice.
pub fn choice_check() -> impl IntoView {
    view! {
        <svg class="choice-check" viewBox="0 0 24 24" width="20" height="20" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <path d="m5 12.5 4.5 4.5L19 7.5"/>
        </svg>
    }
}

/// A magnifier, for opening search. CSS sizes it.
pub fn search() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
            stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/>
        </svg>
    }
}

/// A magnifier that sits inside a search field.
pub fn search_field() -> impl IntoView {
    view! {
        <svg class="search-field-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/>
        </svg>
    }
}

/// A close cross, for clearing a field or removing a value. CSS sizes it.
pub fn close() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
            stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m7 7 10 10M17 7 7 17"/>
        </svg>
    }
}

/// The chain marker: a root joined to two feats. The title is for screen readers; the icon is
/// for the rest.
pub fn chain_mark(suffix: &str) -> impl IntoView + use<> {
    let title = suffix.trim_start_matches(", ").to_string();
    view! {
        <svg class="chain-mark" viewBox="0 0 24 24" width="16" height="16" fill="none"
            stroke="currentColor" stroke-width="1.8" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true">
            <title>{title}</title>
            <circle cx="5.5" cy="12" r="2.2"/><circle cx="18.5" cy="6" r="2.2"/>
            <circle cx="18.5" cy="18" r="2.2"/><path d="M7.7 12H11l3.4-4.5M11 12l3.4 4.5"/>
        </svg>
    }
}

/// The Share icon of the Add to Home Screen guide.
pub fn install_share() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
            stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M12 15V4"/><path d="m8 8 4-4 4 4"/><path d="M7 11H5.5v9h13v-9H17"/>
        </svg>
    }
}

/// The Add to Home Screen icon of the guide.
pub fn install_add_home() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
            stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <rect x="4" y="4" width="16" height="16" rx="3"/><path d="M12 8v8M8 12h8"/>
        </svg>
    }
}

/// The quiet header action icon: install when `update` is false, an update when it is true.
pub fn header_action(update: bool) -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {if update {
                view! {
                    <path d="M20 12a8 8 0 1 1-2.6-5.9"/><path d="M20 4v6h-6"/>
                }.into_any()
            } else {
                view! {
                    <path d="M12 4v10"/><path d="m8 10 4 4 4-4"/><path d="M5 19h14"/>
                }.into_any()
            }}
        </svg>
    }
}

/// The shell navigation icon for `label`.
pub fn nav(label: &'static str) -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
            stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {match label {
                "Home" => view! {
                    <path d="M3 10.5 12 3l9 7.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 19.5z"/>
                    <path d="M9 21v-7h6v7"/>
                }.into_any(),
                "Books" => view! {
                    <path d="M12 6c-2-2-5-2.5-9-2v15c4-.5 7 0 9 2m0-15c2-2 5-2.5 9-2v15c-4-.5-7 0-9 2m0-15v15"/>
                }.into_any(),
                _ => view! {
                    <circle cx="12" cy="12" r="3"/>
                    <path d="M19.4 13.5a7.7 7.7 0 0 0 0-3l2-1.5-2-3.5-2.4 1a8 8 0 0 0-2.6-1.5L14 2h-4l-.4 3a8 8 0 0 0-2.6 1.5l-2.4-1-2 3.5 2 1.5a7.7 7.7 0 0 0 0 3l-2 1.5 2 3.5 2.4-1a8 8 0 0 0 2.6 1.5l.4 3h4l.4-3a8 8 0 0 0 2.6-1.5l2.4 1 2-3.5z"/>
                }.into_any(),
            }}
        </svg>
    }
}
