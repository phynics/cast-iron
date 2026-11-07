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
