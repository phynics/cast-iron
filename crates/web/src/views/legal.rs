//! Legal and attribution, and the coverage report. Both render the files of the active library.

use crate::library::use_library;
use crate::route::Route;
use crate::views::util::local_resource;
use cast_iron_schema::{CoverageReport, EntryId, LegalNotices, RelationCoverage};
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn Legal() -> impl IntoView {
    let library = use_library();
    let legal = local_resource({
        let library = library.clone();
        async move { library.legal().await }
    });
    let coverage = local_resource(async move { library.coverage().await });
    view! {
        <h1>"Legal and attribution"</h1>
        <p>
            <a href="https://github.com/phynics/cast-iron/issues">"Contact the maintainer"</a>
            " through GitHub Issues for questions, corrections, or licensing concerns."
        </p>
        {move || legal.with(|state| match state {
            None => view! { <p>"Loading..."</p> }.into_any(),
            Some(Err(error)) => view! { <p class="error" role="alert">{error.to_string()}</p> }.into_any(),
            Some(Ok(legal)) => legal_view(legal).into_any(),
        })}
        <h2 id="coverage">"Coverage"</h2>
        {move || coverage.with(|state| match state {
            None => view! { <p>"Loading..."</p> }.into_any(),
            Some(Err(error)) => view! { <p class="error" role="alert">{error.to_string()}</p> }.into_any(),
            Some(Ok(report)) => coverage_view(report).into_any(),
        })}
    }
}

fn legal_view(legal: &Rc<LegalNotices>) -> impl IntoView + use<> {
    let licenses = legal
        .licenses
        .iter()
        .map(|license| {
            view! {
                <details class="legal-license" open>
                    <summary>{license.title.clone()}</summary>
                    <pre class="legal-text">{license.text.clone()}</pre>
                </details>
            }
        })
        .collect_view();
    let section_15 = legal
        .section_15
        .iter()
        .map(|notice| {
            view! {
                <li>
                    <strong>{notice.title.clone()}</strong>
                    <span class="legal-notice">{format!(" {}", notice.notice)}</span>
                </li>
            }
        })
        .collect_view();
    let notice_pending = legal
        .notice_pending
        .iter()
        .map(|title| {
            view! {
                <li>
                    <strong>{title.clone()}</strong>
                </li>
            }
        })
        .collect_view();
    let notice_pending_section = (!legal.notice_pending.is_empty()).then(|| {
        view! {
            <section aria-labelledby="notice-pending">
                <h2 id="notice-pending">"Section 15 notice pending"</h2>
                <p>
                    "These publications have no Section 15 notice in any pinned source. Their records ship as assumed Open Game Content under the Open Game License; the project tracks their notice coverage as open work."
                </p>
                <ul class="legal-list">{notice_pending}</ul>
            </section>
        }
    });
    let datasets = legal
        .datasets
        .iter()
        .map(|dataset| {
            view! {
                <li>
                    <strong>{dataset.id.clone()}</strong>
                    {format!(" ({}, commit {}). {}", dataset.repository, dataset.commit, dataset.note)}
                </li>
            }
        })
        .collect_view();
    view! {
        <section aria-labelledby="community-use">
            <h2 id="community-use">"Community Use notice"</h2>
            <pre class="legal-text">{legal.community_use_notice.clone()}</pre>
        </section>
        <section aria-labelledby="product-identity">
            <h2 id="product-identity">"Product Identity declaration"</h2>
            <pre class="legal-text">{legal.product_identity.clone()}</pre>
        </section>
        <section aria-labelledby="licenses">
            <h2 id="licenses">"Open Game License"</h2>
            {licenses}
        </section>
        <section aria-labelledby="section-15">
            <h2 id="section-15">"Section 15: copyright notices"</h2>
            <ul class="legal-list">{section_15}</ul>
        </section>
        {notice_pending_section}
        <section aria-labelledby="dataset">
            <h2 id="dataset">"Dataset"</h2>
            <ul class="legal-list">{datasets}</ul>
        </section>
    }
}

fn coverage_view(report: &Rc<CoverageReport>) -> impl IntoView + use<> {
    let publications = report
        .by_publication
        .iter()
        .map(|p| {
            view! {
                <tr>
                    <th scope="row">{p.title.clone()}</th>
                    <td class="num">{p.upstream_records}</td>
                    <td class="num">{p.included_entries}</td>
                </tr>
            }
        })
        .collect_view();
    let kinds = report
        .by_kind
        .iter()
        .map(|(kind, n)| view! { <tr><th scope="row">{kind.clone()}</th><td class="num">{*n}</td></tr> })
        .collect_view();
    let excluded = report
        .excluded
        .iter()
        .map(|e| {
            view! {
                <tr>
                    <th scope="row"><code>{e.path.clone()}</code></th>
                    <td class="num">{e.records}</td>
                    <td>{e.reason.clone()}</td>
                </tr>
            }
        })
        .collect_view();
    let rejected = if report.rejected.is_empty() {
        view! { <p>"No selected record was rejected."</p> }.into_any()
    } else {
        let rows = report
            .rejected
            .iter()
            .map(|r| view! { <tr><th scope="row"><code>{r.path.clone()}</code></th><td>{r.reason.clone()}</td></tr> })
            .collect_view();
        view! {
            <table class="data-table">
                <caption>"Rejected records"</caption>
                <thead><tr><th scope="col">"Record"</th><th scope="col">"Reason"</th></tr></thead>
                <tbody>{rows}</tbody>
            </table>
        }
        .into_any()
    };
    let stripped = report
        .stripped
        .iter()
        .map(|(what, n)| view! { <tr><th scope="row"><code>{what.clone()}</code></th><td class="num">{*n}</td></tr> })
        .collect_view();
    let links = report.links.clone();
    let relationships = relationship_coverage(&report.relations);
    view! {
        <p>"This library holds a selection of the published rules. Everything left out is listed here."</p>
        <table class="data-table">
            <caption>"By publication"</caption>
            <thead>
                <tr><th scope="col">"Publication"</th><th scope="col" class="num">"Upstream records"</th><th scope="col" class="num">"Included entries"</th></tr>
            </thead>
            <tbody>{publications}</tbody>
        </table>
        <table class="data-table">
            <caption>"By category"</caption>
            <thead><tr><th scope="col">"Category"</th><th scope="col" class="num">"Entries"</th></tr></thead>
            <tbody>{kinds}</tbody>
        </table>
        <table class="data-table">
            <caption>"Excluded from this library"</caption>
            <thead>
                <tr><th scope="col">"Upstream path"</th><th scope="col" class="num">"Records"</th><th scope="col">"Reason"</th></tr>
            </thead>
            <tbody>{excluded}</tbody>
        </table>
        {rejected}
        <table class="data-table">
            <caption>"Link coverage"</caption>
            <tbody>
                <tr><th scope="row">"Resolved"</th><td class="num">{links.resolved}</td></tr>
                <tr><th scope="row">"Target outside this selection"</th><td class="num">{links.out_of_selection}</td></tr>
                <tr><th scope="row">"Broken upstream"</th><td class="num">{links.broken}</td></tr>
            </tbody>
        </table>
        <table class="data-table">
            <caption>"Upstream content removed on purpose"</caption>
            <thead><tr><th scope="col">"Kind"</th><th scope="col" class="num">"Count"</th></tr></thead>
            <tbody>{stripped}</tbody>
        </table>
        {relationships}
    }
}

/// A code-styled entry id that links to the entry when the id is valid.
fn entry_link(id: &str) -> AnyView {
    match EntryId::parse(id) {
        Ok(parsed) => {
            let href = Route::entry(parsed).href();
            view! { <a href=href><code>{id.to_string()}</code></a> }.into_any()
        }
        Err(_) => view! { <code>{id.to_string()}</code> }.into_any(),
    }
}

/// Relations, spell lists, prerequisites, and book order: what was linked, and what was not.
/// Reports of schema 1 and 2 have none of it, and then nothing shows.
fn relationship_coverage(r: &RelationCoverage) -> Option<AnyView> {
    if *r == RelationCoverage::default() {
        return None;
    }
    let edges = r
        .edges
        .iter()
        .map(|(kind, n)| view! { <tr><th scope="row">{kind.clone()}</th><td class="num">{*n}</td></tr> })
        .collect_view();
    let p = &r.prerequisites;
    let readings = p
        .by_reading
        .iter()
        .map(|(reading, n)| view! { <tr><th scope="row">{reading.replace('_', " ")}</th><td class="num">{*n}</td></tr> })
        .collect_view();
    let unread_count = p.unread.len();
    let unread = p
        .unread
        .iter()
        .map(|u| view! { <tr><th scope="row">{entry_link(u.feat.as_str())}</th><td>{u.text.clone()}</td></tr> })
        .collect_view();
    let lists = &r.spell_lists;
    let unmapped = lists
        .unmapped
        .iter()
        .map(|c| view! { <tr><th scope="row">{c.name.clone()}</th><td class="num">{c.spells}</td></tr> })
        .collect_view();
    let unplaced_parts = if r.unplaced_parts.is_empty() {
        view! { <p>"Every racial trait and archetype has a race or class."</p> }.into_any()
    } else {
        let rows = r
            .unplaced_parts
            .iter()
            .map(|u| view! { <tr><th scope="row">{entry_link(&u.entry)}</th><td>{u.reason.clone()}</td></tr> })
            .collect_view();
        view! {
            <table class="data-table">
                <caption>"Racial traits and archetypes without a parent"</caption>
                <thead><tr><th scope="col">"Entry"</th><th scope="col">"Reason"</th></tr></thead>
                <tbody>{rows}</tbody>
            </table>
        }
        .into_any()
    };
    let unpromoted = (!r.unpromoted.is_empty()).then(|| {
        let rows = r
            .unpromoted
            .iter()
            .map(|u| view! { <tr><th scope="row"><code>{u.path.clone()}</code></th><td>{u.reason.clone()}</td></tr> })
            .collect_view();
        view! {
            <table class="data-table">
                <caption>"Records of a typed family that stay ordinary rules text"</caption>
                <thead><tr><th scope="col">"Record"</th><th scope="col">"Reason"</th></tr></thead>
                <tbody>{rows}</tbody>
            </table>
        }
    });
    let order = &r.book_order;
    let unplaced = if order.unplaced.is_empty() {
        view! { <p>{format!("All {} entries have a place in a book.", order.placed)}</p> }
            .into_any()
    } else {
        let rows = order
            .unplaced
            .iter()
            .map(|u| view! { <tr><th scope="row">{entry_link(&u.entry)}</th><td>{u.reason.clone()}</td></tr> })
            .collect_view();
        view! {
            <table class="data-table">
                <caption>"Entries without a place in a book"</caption>
                <thead><tr><th scope="col">"Entry"</th><th scope="col">"Reason"</th></tr></thead>
                <tbody>{rows}</tbody>
            </table>
        }
        .into_any()
    };
    let placements = order
        .by_publication
        .iter()
        .map(|b| {
            view! {
                <tr>
                    <th scope="row">{b.source_id.clone()}</th>
                    <td class="num">{b.placed}</td>
                    <td class="num">{b.unplaced}</td>
                </tr>
            }
        })
        .collect_view();
    Some(
        view! {
            <h3 id="coverage-relationships">"Relationships, spell lists, and book order"</h3>
            <p>
                "These tables say what the links between entries rest on, and what they could not cover. "
                "The chapter lists behind the book view are drafts; see the note on each book."
            </p>
            <table class="data-table">
                <caption>"Links between entries"</caption>
                <thead><tr><th scope="col">"Kind"</th><th scope="col" class="num">"Links"</th></tr></thead>
                <tbody>{edges}</tbody>
            </table>
            {unplaced_parts}
            {unpromoted}
            <table class="data-table">
                <caption>"Feat prerequisites, read from their wording"</caption>
                <thead><tr><th scope="col">"Reading"</th><th scope="col" class="num">"Requirements"</th></tr></thead>
                <tbody>
                    <tr><th scope="row">"Feats"</th><td class="num">{p.feats}</td></tr>
                    <tr><th scope="row">"With prerequisite text"</th><td class="num">{p.feats_with_text}</td></tr>
                    <tr><th scope="row">"Naming at least one feat that exists"</th><td class="num">{p.feats_with_resolved_feat}</td></tr>
                    <tr><th scope="row">"Names that match more than one feat"</th><td class="num">{p.ambiguous}</td></tr>
                    {readings}
                </tbody>
            </table>
            <details class="coverage-more">
                <summary>{format!("Prerequisites not read as a feat or a number ({unread_count})")}</summary>
                <table class="data-table">
                    <caption>"Left as printed text"</caption>
                    <thead><tr><th scope="col">"Feat"</th><th scope="col">"Wording"</th></tr></thead>
                    <tbody>{unread}</tbody>
                </table>
            </details>
            <table class="data-table">
                <caption>"Spell lists whose class has no class entry"</caption>
                <thead><tr><th scope="col">"Class name in the spells"</th><th scope="col" class="num">"Spells"</th></tr></thead>
                <tbody>{unmapped}</tbody>
            </table>
            <table class="data-table">
                <caption>"Entries placed in books"</caption>
                <thead>
                    <tr><th scope="col">"Publication"</th><th scope="col" class="num">"Placed"</th><th scope="col" class="num">"Not placed"</th></tr>
                </thead>
                <tbody>{placements}</tbody>
            </table>
            {unplaced}
        }
        .into_any(),
    )
}
