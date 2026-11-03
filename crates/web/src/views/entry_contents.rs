//! Rule article contents and heading-aware search matching.

use crate::route::Route;
use crate::views::rich::{heading_id, plain_text};
use cast_iron_schema::{Block, EntryId};
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Heading {
    level: u8,
    title: String,
    anchor: String,
}

fn headings(body: &[Block]) -> Vec<Heading> {
    fn collect(blocks: &[Block], out: &mut Vec<Heading>) {
        for block in blocks {
            match block {
                Block::Heading {
                    level,
                    anchor,
                    content,
                } => {
                    if let Some(id) = heading_id(anchor.as_deref(), content) {
                        out.push(Heading {
                            level: *level,
                            title: plain_text(content),
                            anchor: id,
                        });
                    }
                }
                Block::List { items, .. } => {
                    for item in items {
                        collect(item, out);
                    }
                }
                Block::Quote { content } => collect(content, out),
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    collect(body, &mut found);
    // RichBody emits these IDs verbatim. Skip collisions so links never point ambiguously.
    found.retain(|heading| {
        headings_count(body, &heading.anchor) == 1 && !heading.title.trim().is_empty()
    });
    found
}

fn headings_count(body: &[Block], id: &str) -> usize {
    fn count(blocks: &[Block], id: &str) -> usize {
        blocks
            .iter()
            .map(|block| match block {
                Block::Heading {
                    anchor, content, ..
                } => usize::from(heading_id(anchor.as_deref(), content).as_deref() == Some(id)),
                Block::List { items, .. } => items.iter().map(|item| count(item, id)).sum(),
                Block::Quote { content } => count(content, id),
                _ => 0,
            })
            .sum()
    }
    count(body, id)
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Find the best matching, uniquely anchored heading in a typed rule body.
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
    let items = headings(&body);
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
                if current.get_untracked() == target {
                    event.prevent_default();
                    crate::route::scroll_to_anchor(&anchor);
                }
            }>{heading.title}</a></li> }
        })
        .collect_view();
    view! {
        <details class="article-contents">
            <summary>"On this page"</summary>
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
    fn fallback_uses_the_renderer_heading_id_and_rejects_duplicate_ids() {
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
        assert_eq!(matching_heading(&body, "duplicate"), None);
    }
}
