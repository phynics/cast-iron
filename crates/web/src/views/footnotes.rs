//! Footnotes (#507): the pure side of the "Notes" group at the end of an entry.
//!
//! The corpus has no footnote block. A note is a level-six heading with a `footnote-` anchor,
//! and the paragraph after it is the note (`docs/corpus-format.md`, "Footnotes"). Nothing in the
//! schema marks the pair, so the client classifies it here from the typed nodes it already has;
//! the serialized body does not change, so `SCHEMA_VERSION` does not either.
//!
//! Only that one paragraph belongs to the note. A monster's printed note sits inside its stat
//! block, so the blocks after the paragraph (`**Ecology**`, `**Environment:** ...`) are stat
//! block text and stay where they are.

use cast_iron_schema::headings::plain_text;
use cast_iron_schema::{Block, EntryId, Inline};

/// The anchor prefix of a footnote heading.
pub const ANCHOR_PREFIX: &str = "footnote-";

/// Whether a block is the heading of a footnote: level six, with a `footnote-` anchor. The
/// level alone is not enough, because some deeply nested skill sections also use level six.
pub fn is_note_heading(block: &Block) -> bool {
    matches!(
        block,
        Block::Heading { level: 6, anchor: Some(anchor), .. } if anchor.starts_with(ANCHOR_PREFIX)
    )
}

/// One footnote: its heading, its paragraph and where it stood in the body.
#[derive(Debug, Clone, Copy)]
pub struct Note<'a> {
    pub heading: &'a Block,
    pub text: &'a Block,
    /// The printed marker: `1`, `B`. Falls back to the anchor's suffix.
    pub label: &'a str,
    /// How many main blocks come before the note. A block at main index `i` comes before the
    /// note when `i < at`.
    pub at: usize,
}

/// A body with its notes taken out.
#[derive(Debug, Default)]
pub struct Split<'a> {
    /// Every block that is not a note, in order.
    pub main: Vec<&'a Block>,
    pub notes: Vec<Note<'a>>,
}

/// Takes each footnote (heading plus the paragraph after it) out of the top level of a body.
/// A heading with no paragraph after it stays where it is, so nothing is dropped.
pub fn split(body: &[Block]) -> Split<'_> {
    let mut out = Split::default();
    let mut index = 0;
    while index < body.len() {
        let block = &body[index];
        if is_note_heading(block)
            && let Some(text @ Block::Paragraph { .. }) = body.get(index + 1)
            && let Block::Heading {
                anchor: Some(anchor),
                content,
                ..
            } = block
        {
            out.notes.push(Note {
                heading: block,
                text,
                label: label_of(content, anchor),
                at: out.main.len(),
            });
            index += 2;
            continue;
        }
        out.main.push(block);
        index += 1;
    }
    out
}

/// The label of a note. A heading that is only a superscript marker has no text of its own
/// (`^1^` is a `Sup`), so the label is the plain text inside it.
fn label_of<'a>(content: &'a [Inline], anchor: &'a str) -> &'a str {
    match content {
        [Inline::Text { v }] if !v.trim().is_empty() => v.trim(),
        [Inline::Sup { c }] | [Inline::Strong { c }] => match c.as_slice() {
            [Inline::Text { v }] if !v.trim().is_empty() => v.trim(),
            _ => suffix(anchor),
        },
        _ => suffix(anchor),
    }
}

fn suffix(anchor: &str) -> &str {
    anchor.strip_prefix(ANCHOR_PREFIX).unwrap_or(anchor)
}

/// The label of an unlinked marker such as `^B^` or `^12^`: a short run of text with no space.
/// A digit marker straight after a digit is a real exponent (`10^2^`), not a marker.
pub fn marker_label(previous: Option<&Inline>, content: &[Inline]) -> Option<String> {
    if content
        .iter()
        .any(|item| matches!(item, Inline::Link { .. }))
    {
        return None;
    }
    let text = plain_text(content);
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 3 || text.contains(char::is_whitespace) {
        return None;
    }
    let digits = text.chars().all(|ch| ch.is_ascii_digit());
    if digits
        && let Some(Inline::Text { v }) = previous
        && v.chars().next_back().is_some_and(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some(text.to_string())
}

/// The anchor of a linked marker, `^[5](ref/ability/improved-familiars#footnote-h)^`, when the
/// link points into this entry. The link target is the entry's own id.
pub fn linked_anchor<'a>(content: &'a [Inline], entry: &EntryId) -> Option<&'a str> {
    match content {
        [
            Inline::Link {
                target,
                anchor: Some(anchor),
                ..
            },
        ] if target == entry && anchor.starts_with(ANCHOR_PREFIX) => Some(anchor),
        _ => None,
    }
}

/// The note an unlinked marker cites. Notes follow their passage, so a marker cites the first
/// note with its label that comes after it; with none after, the last one before. That keeps the
/// notes of a second stat block with the same labels (`B`) apart from the first.
pub fn resolve<'a>(
    notes: impl IntoIterator<Item = (&'a str, usize)>,
    label: &str,
    position: usize,
) -> Option<usize> {
    let mut last = None;
    for (index, (note_label, at)) in notes.into_iter().enumerate() {
        if note_label != label {
            continue;
        }
        if position < at {
            return Some(index);
        }
        last = Some(index);
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_id(value: &str) -> EntryId {
        EntryId::try_from(value.to_string()).unwrap()
    }

    fn text(value: &str) -> Inline {
        Inline::text(value)
    }

    fn sup(value: &str) -> Inline {
        Inline::Sup {
            c: vec![text(value)],
        }
    }

    fn note_heading(label: Vec<Inline>, anchor: &str) -> Block {
        Block::Heading {
            level: 6,
            anchor: Some(anchor.to_string()),
            content: label,
        }
    }

    fn para(value: &str) -> Block {
        Block::Paragraph {
            style: Default::default(),
            content: vec![text(value)],
        }
    }

    #[test]
    fn a_note_is_its_heading_and_the_one_paragraph_after_it() {
        let body = vec![
            para("Rules."),
            note_heading(vec![sup("1")], "footnote-1"),
            para("The note."),
            para("**Ecology**"),
        ];
        let split = split(&body);
        assert_eq!(split.main.len(), 2);
        assert_eq!(split.notes.len(), 1);
        assert_eq!(split.notes[0].label, "1");
        assert_eq!(split.notes[0].at, 1);
        assert_eq!(split.main[1], &body[3]);
    }

    #[test]
    fn a_level_six_section_without_the_prefix_is_not_a_note() {
        let body = vec![
            Block::Heading {
                level: 6,
                anchor: Some("sec-scale".into()),
                content: vec![text("Scale")],
            },
            para("Text."),
        ];
        assert!(split(&body).notes.is_empty());
        let body = vec![
            Block::Heading {
                level: 5,
                anchor: Some("footnote-1".into()),
                content: vec![text("1")],
            },
            para("Text."),
        ];
        assert!(split(&body).notes.is_empty());
    }

    #[test]
    fn a_note_heading_without_a_paragraph_stays_in_the_body() {
        let body = vec![para("Rules."), note_heading(vec![text("B")], "footnote-b")];
        let split = split(&body);
        assert!(split.notes.is_empty());
        assert_eq!(split.main.len(), 2);
    }

    #[test]
    fn the_label_is_plain_text_inside_the_marker() {
        let body = vec![
            note_heading(vec![text("D")], "footnote-d"),
            para("Domain spell"),
            note_heading(vec![sup("12")], "footnote-12"),
            para("Twelve."),
            note_heading(Vec::new(), "footnote-x"),
            para("Empty."),
        ];
        let labels: Vec<&str> = split(&body).notes.iter().map(|note| note.label).collect();
        assert_eq!(labels, ["D", "12", "x"]);
    }

    #[test]
    fn a_marker_is_short_text_and_an_exponent_is_not() {
        let marker =
            |previous: Option<&Inline>, value: &str| marker_label(previous, &[text(value)]);
        assert_eq!(marker(None, "1").as_deref(), Some("1"));
        assert_eq!(marker(None, "B").as_deref(), Some("B"));
        assert_eq!(marker(Some(&text("bite")), "2").as_deref(), Some("2"));
        assert_eq!(marker(Some(&text("10")), "2"), None);
        assert_eq!(marker(Some(&text("10")), "B").as_deref(), Some("B"));
        assert_eq!(marker(None, "see note"), None);
        assert_eq!(marker(None, "1234"), None);
        assert_eq!(marker(None, ""), None);
    }

    #[test]
    fn a_marker_that_holds_a_link_is_not_an_unlinked_marker() {
        let id = entry_id("ref/ability/improved-familiars");
        let link = Inline::Link {
            target: id.clone(),
            anchor: Some("footnote-h".into()),
            c: vec![text("5")],
        };
        assert_eq!(marker_label(None, std::slice::from_ref(&link)), None);
        assert_eq!(
            linked_anchor(std::slice::from_ref(&link), &id),
            Some("footnote-h")
        );
        assert_eq!(
            linked_anchor(std::slice::from_ref(&link), &entry_id("ref/ability/other")),
            None
        );
    }

    #[test]
    fn a_marker_cites_the_next_note_with_its_label() {
        let body = vec![
            para("first stat block^B^"),
            note_heading(vec![text("B")], "footnote-b"),
            para("Note one."),
            para("second stat block^B^"),
            note_heading(vec![text("B")], "footnote-b"),
            para("Note two."),
            para("trailing"),
        ];
        let split = split(&body);
        assert_eq!(split.main.len(), 3);
        // Main block 0 comes before note 0; block 1 before note 1; block 2 after both.
        let found = |label: &str, position: usize| {
            resolve(
                split.notes.iter().map(|note| (note.label, note.at)),
                label,
                position,
            )
        };
        assert_eq!(found("B", 0), Some(0));
        assert_eq!(found("B", 1), Some(1));
        assert_eq!(found("B", 2), Some(1));
        assert_eq!(found("C", 0), None);
    }
}
