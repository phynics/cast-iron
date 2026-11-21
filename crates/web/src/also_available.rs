//! The `Also available:` line of a class page (issue #170, part E).
//!
//! A class's Alternate Capstones section ends with a paragraph such as
//! `Also available: Walking Library, Arch-Familiar`. Those capstones are printed once, in the
//! shared Alternate Capstones entry, and the corpus text names them without a link. This module
//! links a name only when it is one of the names that the shared entry itself prints, read from
//! its typed body, so a name that does not resolve stays plain text. Nothing is guessed: a name
//! is never matched loosely (only a curly apostrophe equals a straight one), and a comma inside a
//! name (`Old Dog, New Tricks`) is not a separator when the shared entry prints that name.
//!
//! Everything here is pure and has native tests.

use cast_iron_schema::headings::plain_text;
use cast_iron_schema::power::{candidates, split_ability_tag, straight_apostrophe};
use cast_iron_schema::{Block, EntryId, Inline};

/// The shared entry that prints the capstones every class may take.
pub const CAPSTONES: &str = "ref/ability/capstones";

/// The label that opens the line, as the corpus writes it.
const LABEL: &str = "Also available:";

/// The id of the shared capstones entry.
pub fn capstones_id() -> EntryId {
    EntryId::parse(CAPSTONES).expect("the capstones id is well formed")
}

/// The printed names of the capstones in the shared entry's `body`, without their ability tag
/// (`Walking Library (Ex)` is `Walking Library`). They are the power names that the shared
/// classifier finds, so a block that is not a capstone is never a name.
pub fn names_in(body: &[Block]) -> Vec<String> {
    candidates(body)
        .into_iter()
        .map(|candidate| split_ability_tag(&candidate.name).0.to_string())
        .collect()
}

/// Whether `block` is the `Also available:` paragraph.
fn is_line(block: &Block) -> bool {
    line_parts(block).is_some()
}

/// The text after the label of an `Also available:` paragraph, when the paragraph is the bold
/// label followed by plain text only. Any other shape (a link already, emphasis) is left alone.
fn line_parts(block: &Block) -> Option<String> {
    let Block::Paragraph { content, .. } = block else {
        return None;
    };
    let (Inline::Strong { c }, rest) = content.split_first()? else {
        return None;
    };
    if plain_text(c).trim() != LABEL {
        return None;
    }
    let mut text = String::new();
    for inline in rest {
        let Inline::Text { v } = inline else {
            return None;
        };
        text.push_str(v);
    }
    Some(text)
}

/// Whether the body has an `Also available:` paragraph, so a page that has none loads nothing.
pub fn has_line(body: &[Block]) -> bool {
    body.iter().any(is_line)
}

/// One piece of the names list: a known name or the text between names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Name(String),
    Text(String),
}

/// What the corpus writes between two names, longest first so that `, and ` is not read as `, `.
const SEPARATORS: [&str; 5] = [", and ", ", or ", " and ", " or ", ", "];

/// The length of the separator that `text` starts with.
fn separator_len(text: &str) -> Option<usize> {
    SEPARATORS
        .iter()
        .find(|separator| text.starts_with(**separator))
        .map(|separator| separator.len())
}

/// How many bytes of `text` the `name` takes when `text` starts with it. The two match character
/// for character, case-sensitively, except that a curly apostrophe equals the straight one.
fn name_len(text: &str, name: &str) -> Option<usize> {
    let mut chars = text.chars();
    let mut used = 0;
    for expected in name.chars() {
        let found = chars.next()?;
        if straight_apostrophe(found) != straight_apostrophe(expected) {
            return None;
        }
        used += found.len_utf8();
    }
    Some(used)
}

/// What may follow a name: the end of the text, a separator, or a closing `.`, `;` or `)` with
/// nothing after it but white space.
fn ends_name(after: &str) -> bool {
    after.trim().is_empty()
        || separator_len(after).is_some()
        || after
            .strip_prefix(['.', ';', ')'])
            .is_some_and(|rest| rest.trim().is_empty())
}

/// Split `text` (`" Walking Library, Arch-Familiar"`) into known names and the text between and
/// around them. At each position the longest known name that is followed by the end of the text,
/// a separator (see [`SEPARATORS`]) or a closing `.`, `;` or `)` wins; otherwise the text up to
/// the next separator is plain. A name keeps the characters of the line, so a curly apostrophe
/// stays curly.
pub fn split_names(text: &str, known: &[String]) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    let push_text = |pieces: &mut Vec<Piece>, value: &str| {
        if value.is_empty() {
            return;
        }
        match pieces.last_mut() {
            Some(Piece::Text(last)) => last.push_str(value),
            _ => pieces.push(Piece::Text(value.to_string())),
        }
    };
    let mut rest = text.trim_start();
    push_text(&mut pieces, &text[..text.len() - rest.len()]);
    while !rest.is_empty() {
        let longest = known
            .iter()
            .filter(|name| !name.is_empty())
            .filter_map(|name| name_len(rest, name))
            .filter(|len| ends_name(&rest[*len..]))
            .max();
        let len = match longest {
            Some(len) => {
                pieces.push(Piece::Name(rest[..len].to_string()));
                len
            }
            None => {
                let end = rest
                    .char_indices()
                    .map(|(at, _)| at)
                    .find(|at| separator_len(&rest[*at..]).is_some())
                    .unwrap_or(rest.len());
                push_text(&mut pieces, &rest[..end]);
                end
            }
        };
        rest = &rest[len..];
        match separator_len(rest) {
            Some(separator) => {
                push_text(&mut pieces, &rest[..separator]);
                rest = &rest[separator..];
                let next = rest.trim_start();
                push_text(&mut pieces, &rest[..rest.len() - next.len()]);
                rest = next;
            }
            None => {
                push_text(&mut pieces, rest);
                rest = "";
            }
        }
    }
    pieces
}

/// The body with each known name of an `Also available:` paragraph linked to `target`. A page
/// whose paragraph names nothing known, or that has no such paragraph, comes back unchanged.
pub fn link_names(body: Vec<Block>, known: &[String], target: &EntryId) -> Vec<Block> {
    if known.is_empty() {
        return body;
    }
    body.into_iter()
        .map(|block| {
            let Some(text) = line_parts(&block) else {
                return block;
            };
            let pieces = split_names(&text, known);
            if !pieces.iter().any(|piece| matches!(piece, Piece::Name(_))) {
                return block;
            }
            let Block::Paragraph { style, mut content } = block else {
                return block;
            };
            content.truncate(1);
            content.extend(pieces.into_iter().map(|piece| match piece {
                Piece::Text(text) => Inline::text(text),
                Piece::Name(name) => Inline::Link {
                    target: target.clone(),
                    anchor: None,
                    c: vec![Inline::text(name)],
                },
            }));
            Block::Paragraph { style, content }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::ParagraphStyle;

    fn text(value: &str) -> Inline {
        Inline::text(value)
    }

    fn strong(value: &str) -> Inline {
        Inline::Strong {
            c: vec![text(value)],
        }
    }

    fn para(content: Vec<Inline>) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }
    }

    fn line(names: &str) -> Block {
        para(vec![strong("Also available:"), text(&format!(" {names}"))])
    }

    fn known() -> Vec<String> {
        [
            "Arch-Familiar",
            "Old Dog, New Tricks",
            "Old Dog",
            "Walking Library",
            "Great Beast",
        ]
        .map(String::from)
        .to_vec()
    }

    fn target() -> EntryId {
        capstones_id()
    }

    fn names(pieces: &[Piece]) -> Vec<&str> {
        pieces
            .iter()
            .filter_map(|piece| match piece {
                Piece::Name(name) => Some(name.as_str()),
                Piece::Text(_) => None,
            })
            .collect()
    }

    #[test]
    fn the_names_are_the_capstone_powers_without_their_tag() {
        let body = vec![
            para(vec![text("Intro.")]),
            para(vec![strong("Walking Library (Ex)")]),
            para(vec![
                strong("At 20th Level:"),
                text(" The character becomes a small athenaeum."),
            ]),
            para(vec![strong("Prereq:"), text(" Prepares spells")]),
            para(vec![strong("The Boss (Ex)")]),
            para(vec![strong("At 20th Level:"), text(" A leader.")]),
        ];
        assert_eq!(names_in(&body), ["Walking Library", "The Boss"]);
        assert!(names_in(&[para(vec![text("None")])]).is_empty());
    }

    #[test]
    fn a_comma_inside_a_known_name_is_not_a_separator() {
        let pieces = split_names(" Great Beast, Old Dog, New Tricks", &known());
        assert_eq!(names(&pieces), ["Great Beast", "Old Dog, New Tricks"]);
        // The shorter `Old Dog` does not win over the longer name.
        let pieces = split_names(" Old Dog, New Tricks", &known());
        assert_eq!(names(&pieces), ["Old Dog, New Tricks"]);
    }

    #[test]
    fn a_name_that_does_not_resolve_stays_plain_text() {
        let pieces = split_names(
            " Walking Library, Made Up Capstone, Arch-Familiar",
            &known(),
        );
        assert_eq!(names(&pieces), ["Walking Library", "Arch-Familiar"]);
        let plain: String = pieces
            .iter()
            .map(|piece| match piece {
                Piece::Name(name) | Piece::Text(name) => name.as_str(),
            })
            .collect();
        assert_eq!(plain, " Walking Library, Made Up Capstone, Arch-Familiar");
        // A prefix of a name is not the name.
        assert!(names(&split_names(" Walking Libraryx", &known())).is_empty());
        assert!(names(&split_names(" walking library", &known())).is_empty());
    }

    fn joined(pieces: &[Piece]) -> String {
        pieces
            .iter()
            .map(|piece| match piece {
                Piece::Name(text) | Piece::Text(text) => text.as_str(),
            })
            .collect()
    }

    #[test]
    fn and_and_or_join_names_like_a_comma() {
        for (text, expected) in [
            (" Walking Library and Great Beast", 2),
            (" Walking Library or Great Beast", 2),
            (" Walking Library, Great Beast and Arch-Familiar", 3),
            (" Walking Library, Great Beast, and Arch-Familiar", 3),
            (" Walking Library, Great Beast, or Arch-Familiar", 3),
            (" Great Beast, Old Dog, New Tricks and Walking Library", 3),
        ] {
            let pieces = split_names(text, &known());
            assert_eq!(names(&pieces).len(), expected, "{text}");
            assert_eq!(joined(&pieces), text, "text is kept: {text}");
        }
        // An unknown name between known ones stays plain, and `and` in it does not split a name.
        let pieces = split_names(" Great Beast and Made Up and Walking Library", &known());
        assert_eq!(names(&pieces), ["Great Beast", "Walking Library"]);
    }

    #[test]
    fn a_closing_mark_or_the_end_follows_the_last_name() {
        for text in [
            " Walking Library.",
            " Walking Library;",
            " Walking Library)",
            " Walking Library. ",
        ] {
            let pieces = split_names(text, &known());
            assert_eq!(names(&pieces), ["Walking Library"], "{text}");
            assert_eq!(joined(&pieces), text);
        }
        // A mark that is followed by more text does not end a name.
        assert!(names(&split_names(" Walking Library.x", &known())).is_empty());
        assert!(names(&split_names(" Walking Library's", &known())).is_empty());
    }

    #[test]
    fn white_space_after_a_separator_is_skipped() {
        let text = " Walking Library,  Arch-Familiar";
        let pieces = split_names(text, &known());
        assert_eq!(names(&pieces), ["Walking Library", "Arch-Familiar"]);
        assert_eq!(joined(&pieces), text);
    }

    #[test]
    fn a_curly_apostrophe_matches_a_straight_one_and_case_still_counts() {
        let known = vec!["Won't Stay Dead".to_string(), "Great Beast".to_string()];
        let curly = " Great Beast, Won\u{2019}t Stay Dead";
        let pieces = split_names(curly, &known);
        assert_eq!(names(&pieces), ["Great Beast", "Won\u{2019}t Stay Dead"]);
        // The line keeps its own character.
        assert_eq!(joined(&pieces), curly);
        // A straight name in the list also matches a straight line, and the reverse.
        assert_eq!(names(&split_names(" Won't Stay Dead", &known)).len(), 1);
        let curly_known = vec!["Won\u{2019}t Stay Dead".to_string()];
        assert_eq!(
            names(&split_names(" Won't Stay Dead", &curly_known)).len(),
            1
        );
        // Only the apostrophe is folded.
        assert!(names(&split_names(" won\u{2019}t stay dead", &known)).is_empty());
        assert!(names(&split_names(" Wont Stay Dead", &known)).is_empty());
    }

    #[test]
    fn the_names_of_a_real_capstones_body_come_through_the_classifier() {
        // The shape of `ref/ability/capstones`: introduction paragraphs, then each capstone as a
        // bold name, a bold `At 20th Level:` lead and optional `Prereq:` or `Special:` rows.
        let body = vec![
            para(vec![text(
                "When a character reaches the 20th level of a class...",
            )]),
            para(vec![text(
                "The following does not contain class-specific capstones.",
            )]),
            para(vec![strong("Old Dog, New Tricks (Ex)")]),
            para(vec![
                strong("At 20th Level:"),
                text(" The character gains four combat feats."),
            ]),
            para(vec![
                strong("Prereq:"),
                text(" Is of a class that gains bonus combat feats"),
            ]),
            para(vec![strong("Perfect Body, Flawless Mind (Ex)")]),
            para(vec![
                strong("At 20th Level:"),
                text(" Ability scores increase by 8."),
            ]),
            para(vec![strong("The Boss (Ex)")]),
            para(vec![strong("At 20th Level:"), text(" A leader.")]),
            para(vec![text("The character gains the Leadership feat.")]),
            para(vec![strong("Won\u{2019}t Stay Dead (Ex)")]),
            para(vec![
                strong("At 20th Level:"),
                text(" A paragon of resilience."),
            ]),
        ];
        let found = names_in(&body);
        assert_eq!(
            found,
            [
                "Old Dog, New Tricks",
                "Perfect Body, Flawless Mind",
                "The Boss",
                "Won\u{2019}t Stay Dead"
            ]
        );
        // The line of a class resolves against them, commas inside names included.
        let pieces = split_names(
            " Perfect Body, Flawless Mind, Old Dog, New Tricks and Won't Stay Dead.",
            &found,
        );
        assert_eq!(
            names(&pieces),
            [
                "Perfect Body, Flawless Mind",
                "Old Dog, New Tricks",
                "Won't Stay Dead"
            ]
        );
    }

    #[test]
    fn the_paragraph_links_each_known_name_and_keeps_every_other_character() {
        let body = vec![
            para(vec![text("Other text.")]),
            line("Walking Library, Arch-Familiar"),
        ];
        let linked = link_names(body, &known(), &target());
        assert_eq!(linked[0], para(vec![text("Other text.")]));
        let Block::Paragraph { content, .. } = &linked[1] else {
            panic!("a paragraph stays a paragraph");
        };
        assert_eq!(content[0], strong("Also available:"));
        let links: Vec<&EntryId> = content
            .iter()
            .filter_map(|inline| match inline {
                Inline::Link { target, anchor, .. } => {
                    assert!(anchor.is_none());
                    Some(target)
                }
                _ => None,
            })
            .collect();
        assert_eq!(links, [&target(), &target()]);
        assert_eq!(
            plain_text(content),
            "Also available: Walking Library, Arch-Familiar"
        );
    }

    #[test]
    fn a_line_without_a_known_name_or_a_library_without_names_is_unchanged() {
        let body = vec![line("Made Up, Other")];
        assert_eq!(link_names(body.clone(), &known(), &target()), body);
        let body = vec![line("Walking Library")];
        assert_eq!(link_names(body.clone(), &[], &target()), body);
    }

    #[test]
    fn only_the_plain_bold_label_line_is_read() {
        assert!(has_line(&[line("Great Beast")]));
        assert!(!has_line(&[para(vec![text(
            "Also available: Great Beast"
        )])]));
        assert!(!has_line(&[para(vec![
            strong("Also available:"),
            Inline::Em {
                c: vec![text("Great Beast")]
            },
        ])]));
        let body = vec![para(vec![
            strong("Also available:"),
            Inline::Em {
                c: vec![text("Great Beast")],
            },
        ])];
        assert_eq!(link_names(body.clone(), &known(), &target()), body);
    }
}
