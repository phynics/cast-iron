//! The "Referenced by" panel: entries whose body links point at this one.
//!
//! The reverse index is optional. A library without it, or without a row for this target,
//! renders nothing; a read error is logged and also renders nothing, so the entry itself
//! stays readable.

use crate::fmt;
use crate::library::Library;
use crate::route::Route;
use crate::views::icons;
use crate::views::rich::plain_text;
use crate::views::util::{kind_name, local_resource, sort_key};
use cast_iron_schema::{BacklinkSource, Block, EntryId, EntryKind, RuleEntry};
use leptos::prelude::*;
use std::collections::BTreeMap;
use wasm_bindgen::JsCast;

/// Rows shown before a group offers "Show all".
pub const GROUP_PREVIEW: usize = 8;
/// Groups with this many rows or fewer open by default.
pub const OPEN_GROUP_UP_TO: usize = 3;

/// One source entry that links here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacklinkRow {
    pub source: EntryId,
    pub title: String,
    pub kind: EntryKind,
    /// The publication that owns the source, used as the group heading.
    pub book: String,
    /// The heading in the current entry that the source linked to, when one source reaches
    /// this target through exactly one anchor.
    pub anchor: Option<String>,
}

/// Rows under one publication heading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacklinkGroup {
    pub heading: String,
    pub rows: Vec<BacklinkRow>,
}

/// One row per source. The compiler already collapses a source's several anchors; this
/// repeats the rule for a library that an older compiler wrote.
pub fn collapse(sources: &[BacklinkSource]) -> Vec<(EntryId, Option<String>)> {
    let mut by_source: BTreeMap<EntryId, Option<String>> = BTreeMap::new();
    for BacklinkSource(source, anchor) in sources {
        match by_source.get_mut(source) {
            Some(existing) => {
                if existing.as_deref() != anchor.as_deref() {
                    *existing = None;
                }
            }
            None => {
                by_source.insert(source.clone(), anchor.clone());
            }
        }
    }
    by_source.into_iter().collect()
}

/// Joins the index rows to catalog stubs and sorts them by title.
pub fn rows(library: &Library, sources: &[BacklinkSource]) -> Vec<BacklinkRow> {
    let mut out = Vec::new();
    for (source, anchor) in collapse(sources) {
        let Some(stub) = library.stub(&source) else {
            continue;
        };
        let book = library
            .source(&stub.source)
            .map_or_else(|| "Other".to_string(), |source| source.title.clone());
        out.push(BacklinkRow {
            source: stub.id.clone(),
            title: stub.title.clone(),
            kind: stub.kind,
            book,
            anchor,
        });
    }
    out.sort_by(|a, b| (sort_key(&a.title), &a.source).cmp(&(sort_key(&b.title), &b.source)));
    out
}

/// Groups rows by publication, headings in name order and rows in title order.
pub fn group(rows: Vec<BacklinkRow>) -> Vec<BacklinkGroup> {
    let mut groups: BTreeMap<String, Vec<BacklinkRow>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.book.clone()).or_default().push(row);
    }
    let mut out: Vec<BacklinkGroup> = groups
        .into_iter()
        .map(|(heading, mut rows)| {
            rows.sort_by(|a, b| {
                (sort_key(&a.title), &a.source).cmp(&(sort_key(&b.title), &b.source))
            });
            BacklinkGroup { heading, rows }
        })
        .collect();
    out.sort_by_cached_key(|a| sort_key(&a.heading));
    out
}

/// The first `GROUP_PREVIEW` rows and the rest, for the group's disclosure.
pub fn split_preview(rows: &[BacklinkRow]) -> (&[BacklinkRow], &[BacklinkRow]) {
    let at = rows.len().min(GROUP_PREVIEW);
    rows.split_at(at)
}

/// Heading anchor to heading text, so an anchor row shows words rather than a slug.
pub fn heading_text(body: &[Block]) -> BTreeMap<String, String> {
    fn walk(blocks: &[Block], out: &mut BTreeMap<String, String>) {
        for block in blocks {
            match block {
                Block::Heading {
                    anchor: Some(anchor),
                    content,
                    ..
                } => {
                    let text = plain_text(content);
                    if !text.is_empty() {
                        out.insert(anchor.clone(), text);
                    }
                }
                Block::List { items, .. } => {
                    for item in items {
                        walk(item, out);
                    }
                }
                Block::Quote { content } => walk(content, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(body, &mut out);
    out
}

#[component]
pub fn Backlinks(library: Library, entry: RuleEntry) -> impl IntoView {
    let target = entry.id.clone();
    let current = entry.id.clone();
    let headings = heading_text(&entry.body);
    let fetch_library = library.clone();
    let data = local_resource(async move { fetch_library.backlinks(&target).await });

    view! {
        {move || data.with(|state| match state {
            // Render nothing until the index answers. A heading here flashes and then
            // disappears for entries with no rows (#116).
            None => ().into_any(),
            Some(Err(error)) => {
                leptos::logging::warn!("backlinks are off: {error}");
                ().into_any()
            }
            Some(Ok(None)) => ().into_any(),
            Some(Ok(Some(sources))) => {
                let rows = rows(&library, sources);
                if rows.is_empty() {
                    return ().into_any();
                }
                let groups = group(rows);
                let total: usize = groups.iter().map(|group| group.rows.len()).sum();
                let groups_view = groups
                    .into_iter()
                    .map(|group| {
                        group_view(&library, &current, &headings, group)
                    })
                    .collect_view();
                view! {
                    <section class="ref-block entry-backlinks" id="entry-backlinks" aria-labelledby="backlinks-heading">
                        <h2 id="backlinks-heading">"Referenced by"</h2>
                        <p class="muted backlink-count">{format!(
                            "{} {} here.",
                            fmt::pluralize(total, "entry", "entries"),
                            if total == 1 { "links" } else { "link" }
                        )}</p>
                        {groups_view}
                    </section>
                }.into_any()
            }
        })}
    }
}

fn group_view(
    library: &Library,
    current: &EntryId,
    headings: &BTreeMap<String, String>,
    group: BacklinkGroup,
) -> impl IntoView + use<> {
    let count = group.rows.len();
    let open = count <= OPEN_GROUP_UP_TO;
    let (preview, rest) = split_preview(&group.rows);
    let preview_rows = preview
        .iter()
        .map(|row| row_view(library, current, headings, row.clone()))
        .collect_view();
    // The rest waits until "Show all" opens, so a heavily linked entry does not build every
    // row inside a closed disclosure. The closure owns what it needs, so it can run after the
    // first render.
    let rest: Vec<BacklinkRow> = rest.to_vec();
    let lazy_library = library.clone();
    let lazy_current = current.clone();
    let lazy_headings = headings.clone();
    let show_all = RwSignal::new(false);
    let heading = group.heading;
    view! {
        <details class="rel-group backlink-group" open=open>
            <summary>
                {icons::chevron_right("disclosure-chevron")}
                <span class="rel-group-name">{heading}</span>
                <span class="rel-count">{fmt::count(count)}</span>
            </summary>
            <ul class="ref-list">{preview_rows}</ul>
            {(!rest.is_empty()).then(|| view! {
                <details class="backlink-more" on:toggle=move |event| {
                    show_all.set(
                        event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlDetailsElement>().ok())
                            .is_some_and(|details| details.open()),
                    );
                }>
                    <summary>{icons::chevron_right("disclosure-chevron")}{format!("Show all {}", fmt::count(count))}</summary>
                    {move || show_all.get().then(|| view! {
                        <ul class="ref-list">
                            {rest.iter()
                                .map(|row| {
                                    row_view(&lazy_library, &lazy_current, &lazy_headings, row.clone())
                                })
                                .collect_view()}
                        </ul>
                    })}
                </details>
            })}
        </details>
    }
}

fn row_view(
    library: &Library,
    current: &EntryId,
    headings: &BTreeMap<String, String>,
    row: BacklinkRow,
) -> impl IntoView + use<> {
    let href = Route::entry(row.source.clone()).href();
    let kind = kind_name(row.kind);
    let title = row.title;
    let anchor = row.anchor.map(|anchor| {
        let text = headings
            .get(&anchor)
            .cloned()
            .unwrap_or_else(|| anchor.clone());
        let href = Route::Entry {
            id: current.clone(),
            anchor: Some(anchor),
        }
        .href();
        view! { <a class="muted backlink-anchor" href=href>{format!("at {text}")}</a> }
    });
    let _ = library;
    view! {
        <li>
            <a href=href>{title}</a>
            <span class="muted">" · "{kind}</span>
            {anchor}
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, anchor: Option<&str>) -> BacklinkSource {
        BacklinkSource(EntryId::parse(id).unwrap(), anchor.map(str::to_string))
    }

    #[test]
    fn one_row_per_source_and_several_anchors_drop_the_anchor() {
        let collapsed = collapse(&[
            source("ref/rule/a", Some("one")),
            source("ref/rule/a", Some("two")),
            source("ref/rule/b", Some("one")),
            source("ref/rule/b", Some("one")),
        ]);
        assert_eq!(
            collapsed,
            vec![
                (EntryId::parse("ref/rule/a").unwrap(), None),
                (
                    EntryId::parse("ref/rule/b").unwrap(),
                    Some("one".to_string())
                ),
            ]
        );
    }

    #[test]
    fn groups_are_named_sorted_and_split_at_the_preview() {
        let row = |id: &str, title: &str, book: &str| BacklinkRow {
            source: EntryId::parse(id).unwrap(),
            title: title.into(),
            kind: EntryKind::Rule,
            book: book.into(),
            anchor: None,
        };
        let groups = group(vec![
            row("ref/rule/b", "Beta", "Zed"),
            row("ref/rule/a", "Alpha", "Able"),
            row("ref/rule/c", "Gamma", "Able"),
        ]);
        assert_eq!(
            groups
                .iter()
                .map(|group| group.heading.as_str())
                .collect::<Vec<_>>(),
            vec!["Able", "Zed"]
        );
        assert_eq!(
            groups[0]
                .rows
                .iter()
                .map(|row| row.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Gamma"]
        );
    }

    #[test]
    fn a_long_group_keeps_the_first_eight_rows_in_the_preview() {
        let rows: Vec<BacklinkRow> = (0..20)
            .map(|i| BacklinkRow {
                source: EntryId::parse(format!("ref/rule/{i}")).unwrap(),
                title: format!("Entry {i}"),
                kind: EntryKind::Rule,
                book: "Book".into(),
                anchor: None,
            })
            .collect();
        let (preview, rest) = split_preview(&rows);
        assert_eq!(preview.len(), GROUP_PREVIEW);
        assert_eq!(rest.len(), 12);
    }

    #[test]
    fn a_group_that_fits_the_preview_has_no_remaining_rows() {
        for size in 0..=GROUP_PREVIEW {
            let rows: Vec<BacklinkRow> = (0..size)
                .map(|i| BacklinkRow {
                    source: EntryId::parse(format!("ref/rule/{i}")).unwrap(),
                    title: format!("Entry {i}"),
                    kind: EntryKind::Rule,
                    book: "Book".into(),
                    anchor: None,
                })
                .collect();
            let (preview, rest) = split_preview(&rows);
            assert_eq!(preview.len(), size);
            assert!(rest.is_empty(), "no rest for a group of {size}");
        }
    }

    #[test]
    fn heading_text_maps_anchor_slugs_to_their_words() {
        let body = vec![
            Block::Heading {
                level: 2,
                anchor: Some("casting".into()),
                content: vec![cast_iron_schema::Inline::text("Casting Spells")],
            },
            Block::Quote {
                content: vec![Block::Heading {
                    level: 3,
                    anchor: Some("nested".into()),
                    content: vec![cast_iron_schema::Inline::text("Nested")],
                }],
            },
        ];
        let headings = heading_text(&body);
        assert_eq!(headings["casting"], "Casting Spells");
        assert_eq!(headings["nested"], "Nested");
    }
}
