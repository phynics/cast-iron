//! Legal and attribution, and the coverage report. Both render the files of the active library.
//!
//! The legal notices are shown in full and are never collapsed. The coverage report is large
//! (thousands of upstream paths), so its overview is a set of counts and every full table is
//! built only when its disclosure opens.

use crate::library::use_library;
use crate::route::Route;
use crate::views::util::{Shared, local_resource};
use cast_iron_schema::{
    CoverageReport, EntryId, Exclusion, LegalNotices, Rejection, RelationCoverage,
};
use leptos::prelude::*;
use std::collections::BTreeMap;
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

/// A collapsed disclosure whose content is built only when it first opens. Coverage detail can
/// run to thousands of rows, so a closed disclosure must not put any of them in the DOM.
fn lazy_details<F>(kind: &'static str, summary: String, detail: F) -> impl IntoView
where
    F: Fn() -> AnyView + Send + Sync + 'static,
{
    let opened = RwSignal::new(false);
    let on_toggle = move |event: leptos::ev::Event| {
        let open = event_target::<web_sys::HtmlDetailsElement>(&event).open();
        // The signal sets the attribute too; only the reader's own toggle changes it here.
        if open != opened.get_untracked() {
            opened.set(open);
        }
    };
    view! {
        <details class="coverage-more" data-detail=kind prop:open=move || opened.get() on:toggle=on_toggle>
            <summary>{summary}</summary>
            {move || opened.get().then(&detail)}
        </details>
    }
}

/// One row of an exclusion or rejection summary, grouped by the reason it gives.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReasonSummary {
    reason: String,
    paths: usize,
    records: u32,
}

/// The stable part of a reason. Converter and rights rows carry a per-record detail after the
/// group name; the detail stays in the full table below, and the summary counts the group.
fn reason_group(reason: &str) -> String {
    for group in ["converter derived exclusion", "converter native exclusion"] {
        if reason.starts_with(group) {
            return group.to_string();
        }
    }
    const PENDING: &str = "no publication with a Section 15 notice";
    if reason.starts_with(PENDING) {
        return "no publication with a Section 15 notice or a notice-pending decision".to_string();
    }
    if reason.starts_with("converter coverage [")
        && let Some(end) = reason.find(']')
    {
        return reason[..=end].to_string();
    }
    const RIGHTS: &str = "rights basis `";
    if reason.starts_with(RIGHTS)
        && let Some(end) = reason[RIGHTS.len()..].find('`')
    {
        return reason[..=RIGHTS.len() + end].to_string();
    }
    reason.to_string()
}

fn summarize<'a>(rows: impl Iterator<Item = (&'a str, u32)>) -> Vec<ReasonSummary> {
    let mut counts: BTreeMap<String, (usize, u32)> = BTreeMap::new();
    for (reason, records) in rows {
        let entry = counts.entry(reason_group(reason)).or_default();
        entry.0 += 1;
        entry.1 = entry.1.saturating_add(records);
    }
    let mut summary: Vec<ReasonSummary> = counts
        .into_iter()
        .map(|(reason, (paths, records))| ReasonSummary {
            reason,
            paths,
            records,
        })
        .collect();
    summary.sort_by(|a, b| {
        b.records
            .cmp(&a.records)
            .then_with(|| b.paths.cmp(&a.paths))
            .then_with(|| a.reason.cmp(&b.reason))
    });
    summary
}

fn exclusion_summary(rows: &[Exclusion]) -> Vec<ReasonSummary> {
    summarize(rows.iter().map(|row| (row.reason.as_str(), row.records)))
}

fn rejection_summary(rows: &[Rejection]) -> Vec<ReasonSummary> {
    summarize(rows.iter().map(|row| (row.reason.as_str(), 1)))
}

fn coverage_view(report: &Rc<CoverageReport>) -> impl IntoView + use<> {
    let shared = Shared::from_rc(report.clone());
    let publication_count = report.by_publication.len();
    let upstream_records: u64 = report
        .by_publication
        .iter()
        .map(|p| u64::from(p.upstream_records))
        .sum();
    let included_entries: u64 = report
        .by_publication
        .iter()
        .map(|p| u64::from(p.included_entries))
        .sum();
    let excluded_paths = report.excluded.len();
    let excluded_records: u64 = report.excluded.iter().map(|e| u64::from(e.records)).sum();
    let exclusions = exclusion_summary(&report.excluded);
    let rejections = rejection_summary(&report.rejected);
    let labels = report
        .by_kind
        .iter()
        .map(|(kind, n)| {
            let label = cast_iron_schema::EntryKind::from_slug(kind)
                .map(crate::views::util::kind_name)
                .map(str::to_string)
                .unwrap_or_else(|| kind.clone());
            view! { <tr><th scope="row">{label}</th><td class="num">{*n}</td></tr> }
        })
        .collect_view();
    let exclusion_rows = exclusions
        .iter()
        .map(|row| {
            view! {
                <tr>
                    <th scope="row">{row.reason.clone()}</th>
                    <td class="num">{row.paths}</td>
                    <td class="num">{row.records}</td>
                </tr>
            }
        })
        .collect_view();
    let rejection_section = (!report.rejected.is_empty()).then(|| {
        let rows = rejections
            .iter()
            .map(|row| {
                view! {
                    <tr>
                        <th scope="row">{row.reason.clone()}</th>
                        <td class="num">{row.records}</td>
                    </tr>
                }
            })
            .collect_view();
        view! {
            <section aria-labelledby="coverage-rejected">
                <h3 id="coverage-rejected">"Rejected records"</h3>
                <p>
                    {format!(
                        "{} selected records were dropped before the library was built. Every record and its reason remains in the coverage report.",
                        report.rejected.len()
                    )}
                </p>
                <table class="data-table">
                    <caption>"Rejected records by reason"</caption>
                    <thead><tr><th scope="col">"Reason"</th><th scope="col" class="num">"Records"</th></tr></thead>
                    <tbody>{rows}</tbody>
                </table>
                {lazy_details("rejected", format!("All rejected records ({})", report.rejected.len()), {
                    let report = shared.clone();
                    move || rejected_table(&report)
                })}
            </section>
        }
    });
    let stripped = (!report.stripped.is_empty()).then(|| {
        let rows = report
            .stripped
            .iter()
            .map(|(what, n)| view! { <tr><th scope="row"><code>{what.clone()}</code></th><td class="num">{*n}</td></tr> })
            .collect_view();
        view! {
            <section aria-labelledby="coverage-stripped">
                <h3 id="coverage-stripped">"Upstream content removed on purpose"</h3>
                <table class="data-table">
                    <caption>"Removed content by kind"</caption>
                    <thead><tr><th scope="col">"Kind"</th><th scope="col" class="num">"Count"</th></tr></thead>
                    <tbody>{rows}</tbody>
                </table>
            </section>
        }
    });
    let links = report.links.clone();
    let publications_detail = {
        let report = shared.clone();
        move || publications_table(&report)
    };
    let excluded_detail = {
        let report = shared.clone();
        move || excluded_table(&report)
    };
    let relationships = relationship_coverage(&report.relations, shared.clone());
    view! {
        <div class="coverage-summary" data-coverage-summary>
            <p>"This library holds a selection of the published rules. The counts below say what is in it and what was left out; every full list opens on demand."</p>
            <section aria-labelledby="coverage-publications">
                <h3 id="coverage-publications">"Publications"</h3>
                <p>{format!(
                    "{publication_count} publications, {upstream_records} upstream records, and {included_entries} included publication entries."
                )}</p>
                {lazy_details("publications", format!("All publications ({publication_count})"), publications_detail)}
            </section>
            <section aria-labelledby="coverage-categories">
                <h3 id="coverage-categories">"Entries by category"</h3>
                <table class="data-table">
                    <caption>"By category"</caption>
                    <thead><tr><th scope="col">"Category"</th><th scope="col" class="num">"Entries"</th></tr></thead>
                    <tbody>{labels}</tbody>
                </table>
            </section>
            <section aria-labelledby="coverage-excluded">
                <h3 id="coverage-excluded">"Excluded from this library"</h3>
                <p>{format!(
                    "{excluded_paths} upstream paths and {excluded_records} records are excluded, grouped by reason. The full path and reason for every row remains in the coverage report."
                )}</p>
                <table class="data-table">
                    <caption>"Exclusions by reason"</caption>
                    <thead>
                        <tr><th scope="col">"Reason"</th><th scope="col" class="num">"Paths"</th><th scope="col" class="num">"Records"</th></tr>
                    </thead>
                    <tbody>{exclusion_rows}</tbody>
                </table>
                {lazy_details("excluded", format!("All excluded paths ({excluded_paths})"), excluded_detail)}
            </section>
            {rejection_section}
            <section aria-labelledby="coverage-links">
                <h3 id="coverage-links">"Links between entries"</h3>
                <table class="data-table">
                    <caption>"Link coverage"</caption>
                    <tbody>
                        <tr><th scope="row">"Resolved"</th><td class="num">{links.resolved}</td></tr>
                        <tr><th scope="row">"Target outside this selection"</th><td class="num">{links.out_of_selection}</td></tr>
                        <tr><th scope="row">"Broken upstream"</th><td class="num">{links.broken}</td></tr>
                    </tbody>
                </table>
            </section>
            {stripped}
            {relationships}
        </div>
    }
}

fn publications_table(report: &CoverageReport) -> AnyView {
    let rows = report
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
    view! {
        <table class="data-table">
            <caption>"By publication"</caption>
            <thead>
                <tr><th scope="col">"Publication"</th><th scope="col" class="num">"Upstream records"</th><th scope="col" class="num">"Included entries"</th></tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
    }
    .into_any()
}

fn excluded_table(report: &CoverageReport) -> AnyView {
    let rows = report
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
    view! {
        <table class="data-table coverage-detail">
            <caption>"Excluded from this library"</caption>
            <thead>
                <tr><th scope="col">"Upstream path"</th><th scope="col" class="num">"Records"</th><th scope="col">"Reason"</th></tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
    }
    .into_any()
}

fn rejected_table(report: &CoverageReport) -> AnyView {
    let rows = report
        .rejected
        .iter()
        .map(|r| view! { <tr><th scope="row"><code>{r.path.clone()}</code></th><td>{r.reason.clone()}</td></tr> })
        .collect_view();
    view! {
        <table class="data-table coverage-detail">
            <caption>"Rejected records"</caption>
            <thead><tr><th scope="col">"Record"</th><th scope="col">"Reason"</th></tr></thead>
            <tbody>{rows}</tbody>
        </table>
    }
    .into_any()
}

fn unread_table(report: &CoverageReport) -> AnyView {
    let rows = report
        .relations
        .prerequisites
        .unread
        .iter()
        .map(|u| view! { <tr><th scope="row">{entry_link(u.feat.as_str())}</th><td>{u.text.clone()}</td></tr> })
        .collect_view();
    view! {
        <table class="data-table">
            <caption>"Prerequisites left as printed text"</caption>
            <thead><tr><th scope="col">"Feat"</th><th scope="col">"Wording"</th></tr></thead>
            <tbody>{rows}</tbody>
        </table>
    }
    .into_any()
}

fn unplaced_parts_table(report: &CoverageReport) -> AnyView {
    let rows = report
        .relations
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
}

fn unpromoted_table(report: &CoverageReport) -> AnyView {
    let rows = report
        .relations
        .unpromoted
        .iter()
        .map(|u| view! { <tr><th scope="row"><code>{u.path.clone()}</code></th><td>{u.reason.clone()}</td></tr> })
        .collect_view();
    view! {
        <table class="data-table">
            <caption>"Records that stay ordinary rules text"</caption>
            <thead><tr><th scope="col">"Record"</th><th scope="col">"Reason"</th></tr></thead>
            <tbody>{rows}</tbody>
        </table>
    }
    .into_any()
}

fn book_placements_table(report: &CoverageReport) -> AnyView {
    let rows = report
        .relations
        .book_order
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
    view! {
        <table class="data-table">
            <caption>"Entries placed in books"</caption>
            <thead>
                <tr><th scope="col">"Publication"</th><th scope="col" class="num">"Placed"</th><th scope="col" class="num">"Not placed"</th></tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
    }
    .into_any()
}

fn book_unplaced_table(report: &CoverageReport) -> AnyView {
    let rows = report
        .relations
        .book_order
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
}

fn spell_unmapped_table(report: &CoverageReport) -> AnyView {
    let rows = report
        .relations
        .spell_lists
        .unmapped
        .iter()
        .map(|c| view! { <tr><th scope="row">{c.name.clone()}</th><td class="num">{c.spells}</td></tr> })
        .collect_view();
    view! {
        <table class="data-table">
            <caption>"Spell lists whose class has no class entry"</caption>
            <thead><tr><th scope="col">"Class name in the spells"</th><th scope="col" class="num">"Spells"</th></tr></thead>
            <tbody>{rows}</tbody>
        </table>
    }
    .into_any()
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
fn relationship_coverage(r: &RelationCoverage, report: Shared<CoverageReport>) -> Option<AnyView> {
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
    let unread_detail = (unread_count > 0).then(|| {
        lazy_details(
            "prerequisites-unread",
            format!("Prerequisites left as printed text ({unread_count})"),
            {
                let report = report.clone();
                move || unread_table(&report)
            },
        )
        .into_any()
    });
    let lists = &r.spell_lists;
    let unmapped_count = lists.unmapped.len();
    let unmapped_detail = (unmapped_count > 0).then(|| {
        lazy_details(
            "spell-unmapped",
            format!("Spell lists whose class has no class entry ({unmapped_count})"),
            {
                let report = report.clone();
                move || spell_unmapped_table(&report)
            },
        )
        .into_any()
    });
    let unplaced_parts_count = r.unplaced_parts.len();
    let unplaced_parts = if unplaced_parts_count == 0 {
        view! { <p class="hint">"Every racial trait and archetype has a race or class."</p> }
            .into_any()
    } else {
        view! {
            <p>{format!("{unplaced_parts_count} racial traits and archetypes have no parent.")}</p>
            {lazy_details(
                "unplaced-parts",
                format!("Racial traits and archetypes without a parent ({unplaced_parts_count})"),
                {
                    let report = report.clone();
                    move || unplaced_parts_table(&report)
                },
            )}
        }
        .into_any()
    };
    let unpromoted_count = r.unpromoted.len();
    let unpromoted = (unpromoted_count > 0).then(|| {
        view! {
            <p>{format!("{unpromoted_count} typed-family records stay ordinary rules text.")}</p>
            {lazy_details(
                "unpromoted",
                format!("Records that stay ordinary rules text ({unpromoted_count})"),
                {
                    let report = report.clone();
                    move || unpromoted_table(&report)
                },
            )}
        }
        .into_any()
    });
    let order = &r.book_order;
    let book_unplaced_count = order.unplaced.len();
    let book_unplaced = (book_unplaced_count > 0).then(|| {
        lazy_details(
            "book-unplaced",
            format!("Entries without a place in a book ({book_unplaced_count})"),
            {
                let report = report.clone();
                move || book_unplaced_table(&report)
            },
        )
        .into_any()
    });
    let placement_count = order.by_publication.len();
    let placements = lazy_details(
        "book-placements",
        format!("Book placements by publication ({placement_count})"),
        {
            let report = report.clone();
            move || book_placements_table(&report)
        },
    );
    Some(
        view! {
            <section aria-labelledby="coverage-relationships">
                <h3 id="coverage-relationships">"Relationships, spell lists, and book order"</h3>
                <p>
                    "These counts say what the links between entries rest on, and what they could not cover. "
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
                {unread_detail}
                <table class="data-table">
                    <caption>"Spell lists"</caption>
                    <tbody>
                        <tr><th scope="row">"Spells with a class list"</th><td class="num">{lists.spells}</td></tr>
                        <tr><th scope="row">"Classes named by the lists"</th><td class="num">{lists.classes}</td></tr>
                        <tr><th scope="row">"Class and level pairs upstream"</th><td class="num">{lists.pairs_upstream}</td></tr>
                        <tr><th scope="row">"Distinct class and level pairs"</th><td class="num">{lists.pairs_distinct}</td></tr>
                        <tr><th scope="row">"Classes with a class entry"</th><td class="num">{lists.classes_with_entry.len()}</td></tr>
                    </tbody>
                </table>
                {unmapped_detail}
                <table class="data-table">
                    <caption>"Book order"</caption>
                    <tbody>
                        <tr><th scope="row">"Entries placed in a book"</th><td class="num">{order.placed}</td></tr>
                        <tr><th scope="row">"Entries without a place"</th><td class="num">{book_unplaced_count}</td></tr>
                        <tr><th scope="row">"Publications with placements"</th><td class="num">{placement_count}</td></tr>
                    </tbody>
                </table>
                {book_unplaced}
                {placements}
            </section>
        }
        .into_any(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exclusion(reason: &str, records: u32) -> Exclusion {
        Exclusion {
            path: format!("path/{reason}"),
            records,
            reason: reason.into(),
        }
    }

    fn rejection(reason: &str) -> Rejection {
        Rejection {
            path: format!("record/{reason}"),
            reason: reason.into(),
        }
    }

    #[test]
    fn converter_reasons_group_by_category() {
        assert_eq!(
            reason_group("converter coverage [jump-list-metadata] heading_jump_list: jl"),
            "converter coverage [jump-list-metadata]"
        );
        assert_eq!(
            reason_group("converter derived exclusion: ::aff block has no name"),
            "converter derived exclusion"
        );
        assert_eq!(
            reason_group("converter native exclusion (ref/rule/x): unsupported"),
            "converter native exclusion"
        );
        assert_eq!(
            reason_group(
                "no publication with a Section 15 notice or a notice-pending decision (comic); redistribution cannot be established"
            ),
            "no publication with a Section 15 notice or a notice-pending decision"
        );
        assert_eq!(
            reason_group("rights basis `excluded`: art"),
            "rights basis `excluded`"
        );
    }

    #[test]
    fn unrelated_reasons_stay_whole() {
        let reason = "out of scope: Comic publication. Comics are out of scope permanently.";
        assert_eq!(reason_group(reason), reason);
    }

    #[test]
    fn summaries_count_paths_and_records_per_group() {
        let rows = vec![
            exclusion("converter coverage [a] x: one", 2),
            exclusion("converter coverage [a] y: two", 3),
            exclusion("converter coverage [b] z: three", 1),
            exclusion("out of scope: comic", 4),
        ];
        let summary = exclusion_summary(&rows);
        assert_eq!(summary.len(), 3);
        assert_eq!(
            summary[0],
            ReasonSummary {
                reason: "converter coverage [a]".into(),
                paths: 2,
                records: 5,
            }
        );
        assert_eq!(
            summary[1],
            ReasonSummary {
                reason: "out of scope: comic".into(),
                paths: 1,
                records: 4,
            }
        );
        assert_eq!(
            summary[2],
            ReasonSummary {
                reason: "converter coverage [b]".into(),
                paths: 1,
                records: 1,
            }
        );
    }

    #[test]
    fn rejection_summaries_count_one_record_per_row_and_sort_by_count() {
        let rows = vec![
            rejection("converter native exclusion (a): x"),
            rejection("converter native exclusion (b): y"),
            rejection("converter derived exclusion: z"),
        ];
        let summary = rejection_summary(&rows);
        assert_eq!(summary.len(), 2);
        assert_eq!(summary[0].reason, "converter native exclusion");
        assert_eq!(summary[0].records, 2);
        assert_eq!(summary[1].reason, "converter derived exclusion");
        assert_eq!(summary[1].records, 1);
    }

    #[test]
    fn summary_totals_match_the_rows() {
        let rows = vec![
            exclusion("converter coverage [a] x: one", 7),
            exclusion("converter coverage [a] y: two", 0),
            exclusion("converter coverage [b] z: three", 11),
        ];
        let summary = exclusion_summary(&rows);
        assert_eq!(
            summary.iter().map(|row| row.paths).sum::<usize>(),
            rows.len()
        );
        assert_eq!(
            summary.iter().map(|row| row.records).sum::<u32>(),
            rows.iter().map(|row| row.records).sum::<u32>()
        );
    }
}
