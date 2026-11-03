//! Legal and attribution, and the coverage report. Both render the files of the active library.

use crate::library::use_library;
use crate::views::util::local_resource;
use cast_iron_schema::{CoverageReport, LegalNotices};
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
    }
}
