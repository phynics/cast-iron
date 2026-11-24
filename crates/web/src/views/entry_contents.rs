//! Rule article contents and heading-aware search matching.

use crate::library::use_library;
use crate::route::Route;
use crate::views::footnotes;
use crate::views::icons;
use crate::views::rich::{keyed_headings, plain_text};
use cast_iron_schema::headings::feature_heads;
use cast_iron_schema::{Block, EntryId, RuleEntry};
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Heading {
    level: u8,
    title: String,
    anchor: String,
}

fn headings(body: &[Block]) -> Vec<Heading> {
    keyed_headings(body)
        .into_iter()
        .filter_map(|(block, key)| {
            let Block::Heading { level, content, .. } = block else {
                return None;
            };
            // A footnote is not a section: the page shows it in its Notes group.
            if footnotes::is_note_heading(block) {
                return None;
            }
            let key = key?;
            let title = plain_text(content);
            (!title.trim().is_empty()).then_some(Heading {
                level: *level,
                title,
                anchor: key,
            })
        })
        .collect()
}

/// A heading that names a publication repeats the entry's attribution, not a section.
fn is_publication_heading(heading: &Heading, publication_titles: &[String]) -> bool {
    let title = heading.title.trim();
    publication_titles
        .iter()
        .any(|publication| publication.trim().eq_ignore_ascii_case(title))
}

/// The outline stops: body headings minus publication names. The filtered list keeps the
/// renderer's keys, so every remaining link still reaches its heading.
fn contents(body: &[Block], publication_titles: &[String]) -> Vec<Heading> {
    headings(body)
        .into_iter()
        .filter(|heading| !is_publication_heading(heading, publication_titles))
        .collect()
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Find the best matching heading key in a typed rule body.
/// Exact heading matches win, then whole-word matches, then headings containing every query token.
pub fn matching_heading(body: &[Block], query: &str) -> Option<String> {
    let query = query.trim();
    let query_words = words(query);
    if query_words.is_empty() {
        return None;
    }
    let all = headings(body);
    let exact = all
        .iter()
        .find(|heading| heading.title.trim().eq_ignore_ascii_case(query));
    if let Some(heading) = exact {
        return Some(heading.anchor.clone());
    }
    let whole = all.iter().find(|heading| {
        let title_words = words(&heading.title);
        query_words
            .iter()
            .all(|word| title_words.iter().any(|title| title == word))
    });
    whole
        .or_else(|| {
            all.iter().find(|heading| {
                let title_words = words(&heading.title);
                query_words
                    .iter()
                    .all(|word| title_words.iter().any(|title| title.contains(word)))
            })
        })
        .map(|heading| heading.anchor.clone())
}

/// The anchor of the class feature that a search alias names (#540): the feature of `entry`
/// whose printed name, without its ability tag, is `alias`. `None` when the entry lists no such
/// feature, so the result opens the entry at the top.
pub fn feature_anchor(entry: &RuleEntry, alias: &str) -> Option<String> {
    feature_heads(&entry.body, entry.fields.features())
        .into_iter()
        .find(|head| head.name == alias)
        .map(|head| head.anchor)
}

/// The article lists its sections from this many headings on. A listed feature is a heading of
/// the body, so it counts like any other (#540).
const CONTENTS_MIN: usize = 4;

/// Whether the page lists its sections under `On this page`: enough headings after the
/// publication names. A link that jumps within the page adds value on such a page only.
pub fn lists_contents(body: &[Block], publication_titles: &[String]) -> bool {
    contents(body, publication_titles).len() >= CONTENTS_MIN
}

/// A link to an anchor of the entry that is open. A plain click on the page's own address scrolls
/// without navigating; any other click, and a click with a modifier, follows the link.
#[component]
pub fn JumpLink(
    id: EntryId,
    anchor: String,
    book: Option<String>,
    children: Children,
) -> impl IntoView {
    let current = crate::route::use_route();
    let target = Route::Entry {
        id,
        anchor: Some(anchor.clone()),
        book,
        section: None,
        arch: Default::default(),
    };
    let href = target.href();
    view! { <a href=href on:click=move |event: leptos::ev::MouseEvent| {
        if event.button() == 0 && !event.ctrl_key() && !event.meta_key() && !event.shift_key() && !event.alt_key() && current.get_untracked() == target {
            event.prevent_default();
            crate::route::scroll_to_anchor(&anchor);
        }
    }>{children()}</a> }
}

/// Show a compact disclosure only when the article has enough sections to benefit from it.
#[component]
pub fn ArticleContents(body: Vec<Block>, id: EntryId) -> impl IntoView {
    let library = use_library();
    let publication_titles: Vec<String> = library
        .sources()
        .iter()
        .map(|source| source.title.clone())
        .collect();
    let items = contents(&body, &publication_titles);
    if items.len() < CONTENTS_MIN {
        return ().into_any();
    }
    let current = crate::route::use_route();
    let book = match current.get_untracked() {
        Route::Entry { book, .. } => book,
        _ => None,
    };
    let links = items
        .into_iter()
        .map(|heading| {
            let class = format!("contents-level-{}", heading.level.clamp(2, 6));
            view! { <li class=class><JumpLink id=id.clone() anchor=heading.anchor book=book.clone()>{heading.title}</JumpLink></li> }
        })
        .collect_view();
    view! {
        <details class="article-contents">
            <summary>{icons::chevron_right("disclosure-chevron")}"On this page"</summary>
            <nav aria-label="Article contents"><ol>{links}</ol></nav>
        </details>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::Inline;

    fn h(level: u8, title: &str, anchor: &str) -> Block {
        Block::Heading {
            level,
            anchor: Some(anchor.into()),
            content: vec![Inline::text(title)],
        }
    }

    #[test]
    fn matching_prefers_exact_then_whole_words_and_skips_empty_or_missing() {
        let body = vec![
            h(2, "Greater Grapple", "greater"),
            h(3, "Grapple", "grapple"),
        ];
        assert_eq!(
            matching_heading(&body, "grapple").as_deref(),
            Some("grapple")
        );
        assert_eq!(
            matching_heading(&body, "greater grapple").as_deref(),
            Some("greater")
        );
        assert_eq!(matching_heading(&body, "   "), None);
        assert_eq!(matching_heading(&body, "unrelated"), None);
    }

    #[test]
    fn fallback_uses_the_renderer_heading_key_and_deduplicates_anchors() {
        let body = vec![
            Block::Heading {
                level: 2,
                anchor: None,
                content: vec![Inline::text("Grapple")],
            },
            h(2, "Other", "other"),
            h(2, "Duplicate", "other"),
        ];
        assert_eq!(
            matching_heading(&body, "grapple").as_deref(),
            Some("sec-grapple")
        );
        assert_eq!(
            matching_heading(&body, "duplicate").as_deref(),
            Some("other-2")
        );
    }

    #[test]
    fn publication_titles_do_not_become_article_contents() {
        let body = vec![
            h(2, "Advanced Player's Guide", "apg"),
            h(2, "Class Features", "features"),
            h(3, "Blood of Shadows", "blood"),
            h(2, "Archetypes", "archetypes"),
        ];
        let titles = [
            "Advanced Player's Guide".to_string(),
            "Blood of Shadows".to_string(),
        ];
        let found = contents(&body, &titles);
        assert_eq!(
            found
                .iter()
                .map(|heading| heading.title.as_str())
                .collect::<Vec<_>>(),
            ["Class Features", "Archetypes"]
        );
        // Matching ignores case and surrounding spaces.
        assert!(contents(&[h(2, "  blood of shadows ", "b")], &titles).is_empty());
        // With no publication list the outline is unchanged.
        assert_eq!(contents(&body, &[]).len(), 4);
    }

    #[test]
    fn listed_features_count_as_headings_of_the_contents_list() {
        // Two sections and two features: four headings list the page, and a feature keeps its
        // anchor, its printed name and its place (#540).
        let body = vec![
            h(2, "Class Skills", "skills"),
            h(3, "Shattering Strike (Ex)", "shattering-strike"),
            h(3, "Piledriver (Ex)", "piledriver"),
            h(2, "Alternate Racial Traits", "traits"),
        ];
        assert!(lists_contents(&body, &[]));
        let found = contents(&body, &[]);
        assert_eq!(found[1].anchor, "shattering-strike");
        assert_eq!(found[1].title, "Shattering Strike (Ex)");
        // Three headings stay below the rule: the list appears at four or more.
        assert!(!lists_contents(&body[..3], &[]));
    }

    #[test]
    fn a_feature_alias_opens_the_entry_at_the_feature_that_prints_the_name() {
        use cast_iron_schema::{ArchetypeFields, EntryKind, FeatureFact, Provenance, TypedFields};
        let entry = RuleEntry {
            id: EntryId::parse("ref/arc-fighter/two-handed-fighter").unwrap(),
            kind: EntryKind::Archetype,
            title: "Two-Handed Fighter".into(),
            aon: None,
            source_ids: vec![],
            sources: vec![],
            expanded_in: vec![],
            stat_sets: vec![],
            section: vec![],
            parent: None,
            body: vec![
                h(2, "Shattering Strike (Ex)", "shattering-strike"),
                h(2, "Piledriver (Ex)", "piledriver"),
            ],
            fields: TypedFields::Archetype(ArchetypeFields {
                features: vec![FeatureFact {
                    anchor: "shattering-strike".into(),
                    level: Some(2),
                }],
                ..Default::default()
            }),
            tags: vec![],
            race_group: None,
            links: vec![],
            provenance: Vec::<Provenance>::new(),
            license_refs: vec![],
            relations: vec![],
        };
        assert_eq!(
            feature_anchor(&entry, "Shattering Strike").as_deref(),
            Some("shattering-strike")
        );
        // A heading that the data does not list is not a feature, and a printed name with its tag
        // is not the alias.
        assert_eq!(feature_anchor(&entry, "Piledriver"), None);
        assert_eq!(feature_anchor(&entry, "Shattering Strike (Ex)"), None);
    }

    #[test]
    fn duplicate_headings_both_appear_in_the_contents_with_distinct_keys() {
        let body = vec![
            Block::Heading {
                level: 2,
                anchor: None,
                content: vec![Inline::text("Grapple")],
            },
            h(2, "Other", "other"),
            Block::Heading {
                level: 3,
                anchor: None,
                content: vec![Inline::text("Grapple")],
            },
        ];
        let found = headings(&body);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].anchor, "sec-grapple");
        assert_eq!(found[2].anchor, "sec-grapple-2");
        assert_eq!(found[2].title, "Grapple");
    }
}
