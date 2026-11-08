//! Rule article contents and heading-aware search matching.

use crate::library::use_library;
use crate::route::Route;
use crate::views::icons;
use crate::views::rich::{keyed_headings, plain_text};
use cast_iron_schema::{Block, EntryId};
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
    if items.len() < 4 {
        return ().into_any();
    }
    let current = crate::route::use_route();
    let links = items
        .into_iter()
        .map(|heading| {
            let class = format!("contents-level-{}", heading.level.clamp(2, 6));
            let anchor = heading.anchor.clone();
            let target = Route::Entry {
                id: id.clone(),
                anchor: Some(heading.anchor),
            };
            let href = target.href();
            view! { <li class=class><a href=href on:click=move |event: leptos::ev::MouseEvent| {
                if event.button() == 0 && !event.ctrl_key() && !event.meta_key() && !event.shift_key() && !event.alt_key() && current.get_untracked() == target {
                    event.prevent_default();
                    crate::route::scroll_to_anchor(&anchor);
                }
            }>{heading.title}</a></li> }
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
