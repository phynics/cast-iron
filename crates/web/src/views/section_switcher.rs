//! The class page's section switcher: a sticky bar of links under the summary card
//! (#170, part A). `class_sections` says which stops a class has and where they start.
//!
//! It is a row of anchored jumps, not ARIA tabs. Every section stays in the document, so Find in
//! page and a screen reader's heading list reach all of it; a link moves the page to the start
//! of its section and the address records it (`?s=archetypes`), so Back returns to the section
//! before. The current stop is the last one chosen, shown with `aria-current`.

use crate::class_sections::Section;
use crate::route::{self, ArchFilter, Route, use_route};
use cast_iron_schema::EntryId;
use leptos::prelude::*;

/// `stops` are the sections after Overview that this class has, in switcher order.
#[component]
pub fn SectionSwitcher(id: EntryId, book: Option<String>, stops: Vec<Section>) -> impl IntoView {
    let current = use_route();
    let items = std::iter::once(None).chain(stops.iter().copied().map(Some));
    let links = items
        .map(|section| {
            // A stop keeps the archetype choices of the address, so a jump to Features and back
            // does not drop the filter.
            let target = {
                let (id, book) = (id.clone(), book.clone());
                move |arch: ArchFilter| Route::Entry {
                    id: id.clone(),
                    anchor: None,
                    book: book.clone(),
                    section,
                    arch,
                }
            };
            let href = {
                let target = target.clone();
                move || target(current.with(Route::arch)).href()
            };
            let label = section.map_or("Overview", Section::label);
            // An address that names a stop this class lacks (`?s=spells` on a fighter) opens at
            // the top, so Overview is the current one.
            let stops = stops.clone();
            let is_current = move || {
                current
                    .with(|route| route.section().filter(|named| stops.contains(named)) == section)
            };
            view! {
                <li>
                    <a
                        href=href
                        aria-current=move || is_current().then_some("true")
                        on:click=move |event: leptos::ev::MouseEvent| {
                            // The address already names this stop, so the link would change
                            // nothing and the page would stay where the reader scrolled it.
                            let plain = event.button() == 0
                                && !event.ctrl_key()
                                && !event.meta_key()
                                && !event.shift_key()
                                && !event.alt_key();
                            if plain && current.get_untracked() == target(current.get_untracked().arch()) {
                                event.prevent_default();
                                match section {
                                    Some(section) => {
                                        route::scroll_to_section(section);
                                    }
                                    None => route::scroll_to_top(),
                                }
                            }
                        }
                    >{label}</a>
                </li>
            }
        })
        .collect_view();
    view! {
        <nav class="section-switcher" aria-label="Sections">
            <ul>{links}</ul>
        </nav>
    }
}
