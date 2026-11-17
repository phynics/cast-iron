//! The class summary card: the facts a reader checks first, in the spell statistics card's
//! visual language (`.stat-block`, with its left accent rule).
//!
//! The values come from `class_summary`. A row without a reliable value is not drawn.

use crate::class_summary::ClassSummary;
use crate::fmt;
use crate::views::icons;
use leptos::prelude::*;

/// The class's spell list: its name for the link text and the list page's address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellListLink {
    pub name: String,
    pub href: String,
}

fn row(label: &'static str, value: impl IntoView + 'static) -> impl IntoView {
    view! {
        <div class="stat-row">
            <dt>{label}</dt>
            <dd>{value}</dd>
        </div>
    }
}

/// A row whose value is one 44 px control, so the label centres on it.
fn action_row(label: &'static str, value: impl IntoView + 'static) -> impl IntoView {
    view! {
        <div class="stat-row stat-row-action">
            <dt>{label}</dt>
            <dd>{value}</dd>
        </div>
    }
}

/// `None` when the card would have no row.
pub fn class_card(summary: &ClassSummary, list: Option<SpellListLink>) -> Option<AnyView> {
    if summary.is_empty() && list.is_none() {
        return None;
    }
    let hit_die = summary.hit_die.clone().map(|die| row("Hit Die", die));
    let attack = summary.attack.map(|rate| {
        row(
            "Base attack",
            view! { {rate.label()}" "<span class="muted">{format!("· {}", rate.per_level())}</span> },
        )
    });
    let saves = summary.good_saves.as_ref().map(|saves| {
        let text = if saves.is_empty() {
            "None".to_string()
        } else {
            saves.join(", ")
        };
        row("Good saves", text)
    });
    let ranks = summary.skill_ranks.clone().map(|ranks| {
        row(
            "Skill ranks",
            view! { {ranks}" "<span class="muted">"per level"</span> },
        )
    });
    let skills = (!summary.skills.is_empty()).then(|| {
        let groups = summary
            .skills
            .iter()
            .map(|group| {
                let ability = (!group.ability.is_empty())
                    .then(|| view! { <span class="muted">{group.ability.clone()}" "</span> });
                view! { <li>{ability}{group.skills.join(", ")}</li> }
            })
            .collect_view();
        action_row(
            "Class skills",
            view! {
                <details class="class-skills">
                    <summary>
                        {icons::chevron_right("disclosure-chevron")}
                        {fmt::pluralize(summary.skill_count(), "skill", "skills")}
                    </summary>
                    <ul class="class-skill-list">{groups}</ul>
                </details>
            },
        )
    });
    let alignment = summary
        .alignment
        .clone()
        .map(|value| row("Alignment", value));
    let wealth = summary
        .wealth
        .clone()
        .map(|value| row("Starting wealth", value));
    let casting = (!summary.casting.is_empty() || list.is_some()).then(|| {
        let line =
            (!summary.casting.is_empty()).then(|| view! { <span>{summary.casting.line()}</span> });
        let link = list.map(|list| {
            view! {
                <a class="page-link" href=list.href>
                    <span>{format!("{} spell list", list.name)}</span>
                    {icons::chevron_right_large()}
                </a>
            }
        });
        // Without printed text the row is only the link, so the label centres on it.
        match line.is_some() {
            true => row("Spellcasting", view! { {line}{link} }).into_any(),
            false => action_row("Spellcasting", link).into_any(),
        }
    });
    Some(
        view! {
            <section class="stat-block class-card" aria-label="Class summary">
                <dl>{hit_die}{attack}{saves}{ranks}{skills}{alignment}{wealth}{casting}</dl>
            </section>
        }
        .into_any(),
    )
}
