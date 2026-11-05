//! The inline subset. It maps one-to-one to [`Inline`].
//!
//! See `docs/corpus-format.md` for the user-facing grammar. The important
//! invariants: a backslash escapes the next character, `\n` is [`Inline::Break`],
//! `\N` is a literal newline, `\R` a literal carriage return, `\0` is a
//! text-node boundary (needed only where the schema holds adjacent `Text`
//! nodes), and `\e` is an empty `Text` node.

use cast_iron_schema::{EntryId, Inline};

const SPECIAL: &[char] = &[
    '\\', '*', '_', '^', '~', '[', ']', '#', '>', '|', '{', '}', '<',
];

/// Escapes plain text. Parentheses and underscores are not syntax in this
/// subset, so they stay readable. Newlines and carriage returns use escapes so
/// one inline run is always one source line.
pub fn escape_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\N"),
            '\r' => out.push_str("\\R"),
            ch if SPECIAL.contains(&ch) => {
                out.push('\\');
                out.push(ch);
            }
            ch => out.push(ch),
        }
    }
    out
}

/// Writes an inline sequence. Adjacent `Text` nodes are separated with `\0` so
/// the structural shape survives.
pub fn write_inlines(inlines: &[Inline], out: &mut String) {
    write_inlines_depth(inlines, 0, out);
}

fn write_inlines_depth(inlines: &[Inline], em_depth: usize, out: &mut String) {
    for (index, inline) in inlines.iter().enumerate() {
        match inline {
            Inline::Text { v } => {
                if v.is_empty() {
                    out.push_str("\\e");
                } else {
                    out.push_str(&escape_text(v));
                }
                let next_is_text = matches!(inlines.get(index + 1), Some(Inline::Text { .. }));
                if next_is_text {
                    out.push_str("\\0");
                }
            }
            Inline::Em { c } => {
                // Alternate the delimiter so an emphasis span can nest inside
                // another one without closing the outer span early.
                let delimiter = if em_depth.is_multiple_of(2) { '_' } else { '*' };
                out.push(delimiter);
                write_inlines_depth(c, em_depth + 1, out);
                out.push(delimiter);
            }
            Inline::Strong { c } => {
                out.push_str("**");
                write_inlines_depth(c, em_depth, out);
                out.push_str("**");
            }
            Inline::Sup { c } => {
                out.push('^');
                write_inlines_depth(c, em_depth, out);
                out.push('^');
            }
            Inline::Sub { c } => {
                out.push('~');
                write_inlines_depth(c, em_depth, out);
                out.push('~');
            }
            Inline::Break => out.push_str("\\n"),
            Inline::Link { target, anchor, c } => {
                out.push('[');
                write_inlines_depth(c, em_depth, out);
                out.push_str("](");
                out.push_str(target.as_str());
                if let Some(anchor) = anchor {
                    out.push('#');
                    out.push_str(anchor);
                }
                out.push(')');
            }
        }
    }
}

/// Renders a single paragraph's inline run and protects a leading block
/// marker. Used by paragraphs, headings, and fences where the content starts a
/// line.
pub fn write_inline_line(inlines: &[Inline]) -> String {
    let mut out = String::new();
    write_inlines(inlines, &mut out);
    guard_start(out)
}

fn guard_start(value: String) -> String {
    let first = value.as_bytes().first().copied();
    let blocked = match first {
        Some(b'-') | Some(b'+') => value.len() == 1 || value.as_bytes().get(1) == Some(&b' '),
        Some(b':') => value.starts_with(":::"),
        Some(b'0'..=b'9') => {
            let digits = value.bytes().take_while(u8::is_ascii_digit).count();
            let rest = &value[digits..];
            rest == "." || rest.starts_with(". ")
        }
        _ => false,
    };
    if blocked {
        let mut escaped = String::with_capacity(value.len() + 1);
        escaped.push('\\');
        escaped.push_str(&value);
        escaped
    } else {
        value
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    None,
    Em(char),
    Strong,
    Sup,
    Sub,
    LinkLabel,
}

/// Parses an inline run. The error text has no line number; the block parser
/// attaches the source line.
pub fn parse_inlines(text: &str) -> Result<Vec<Inline>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut position = 0;
    let inlines = parse_seq(&chars, &mut position, Stop::None)?;
    if position != chars.len() {
        return Err(format!(
            "unexpected {:?} at the end of inline text",
            chars[position]
        ));
    }
    Ok(inlines)
}

fn parse_seq(chars: &[char], position: &mut usize, stop: Stop) -> Result<Vec<Inline>, String> {
    let mut out = Vec::new();
    let mut buffer = String::new();

    let flush = |buffer: &mut String, out: &mut Vec<Inline>| {
        if !buffer.is_empty() {
            out.push(Inline::text(std::mem::take(buffer)));
        }
    };

    while *position < chars.len() {
        let ch = chars[*position];
        match ch {
            '\\' => {
                let next = chars
                    .get(*position + 1)
                    .copied()
                    .ok_or_else(|| "dangling backslash at the end of an inline run".to_string())?;
                *position += 2;
                match next {
                    'n' => {
                        flush(&mut buffer, &mut out);
                        out.push(Inline::Break);
                    }
                    'N' => {
                        buffer.push('\n');
                    }
                    'R' => {
                        buffer.push('\r');
                    }
                    '0' => {
                        flush(&mut buffer, &mut out);
                    }
                    'e' => {
                        flush(&mut buffer, &mut out);
                        out.push(Inline::text(String::new()));
                    }
                    other => buffer.push(other),
                }
            }
            '*' if matches!(stop, Stop::Strong) && chars.get(*position + 1) == Some(&'*') => {
                *position += 2;
                flush(&mut buffer, &mut out);
                return Ok(out);
            }
            '*' if chars.get(*position + 1) == Some(&'*') => {
                flush(&mut buffer, &mut out);
                *position += 2;
                let c = parse_seq(chars, position, Stop::Strong)?;
                out.push(Inline::Strong { c });
            }
            '*' | '_' => {
                if matches!(stop, Stop::Em(delimiter) if delimiter == ch) {
                    *position += 1;
                    flush(&mut buffer, &mut out);
                    return Ok(out);
                }
                flush(&mut buffer, &mut out);
                *position += 1;
                let c = parse_seq(chars, position, Stop::Em(ch))?;
                out.push(Inline::Em { c });
            }
            '^' => {
                if matches!(stop, Stop::Sup) {
                    *position += 1;
                    flush(&mut buffer, &mut out);
                    return Ok(out);
                }
                flush(&mut buffer, &mut out);
                *position += 1;
                let c = parse_seq(chars, position, Stop::Sup)?;
                out.push(Inline::Sup { c });
            }
            '~' => {
                if matches!(stop, Stop::Sub) {
                    *position += 1;
                    flush(&mut buffer, &mut out);
                    return Ok(out);
                }
                flush(&mut buffer, &mut out);
                *position += 1;
                let c = parse_seq(chars, position, Stop::Sub)?;
                out.push(Inline::Sub { c });
            }
            '[' => {
                flush(&mut buffer, &mut out);
                *position += 1;
                let c = parse_seq(chars, position, Stop::LinkLabel)?;
                if chars.get(*position) != Some(&']') {
                    return Err("a link label is not closed with `]`".into());
                }
                *position += 1;
                if chars.get(*position) != Some(&'(') {
                    return Err("a link label must be followed by `(target)`".into());
                }
                *position += 1;
                let mut target = String::new();
                while let Some(&ch) = chars.get(*position) {
                    if ch == ')' {
                        break;
                    }
                    target.push(ch);
                    *position += 1;
                }
                if chars.get(*position) != Some(&')') {
                    return Err("a link target is not closed with `)`".into());
                }
                *position += 1;
                let (id, anchor) = match target.split_once('#') {
                    Some((id, anchor)) => (id, Some(anchor.to_string())),
                    None => (target.as_str(), None),
                };
                let target = EntryId::parse(id)
                    .map_err(|e| format!("a link target is not a canonical id: {e}"))?;
                if anchor.as_deref().is_some_and(str::is_empty) {
                    return Err("a link anchor is empty after `#`".into());
                }
                out.push(Inline::Link { target, anchor, c });
            }
            ']' if matches!(stop, Stop::LinkLabel) => {
                flush(&mut buffer, &mut out);
                return Ok(out);
            }
            ']' => return Err("stray `]`; escape it as `\\]`".into()),
            ch => {
                buffer.push(ch);
                *position += 1;
            }
        }
    }

    if !matches!(stop, Stop::None) {
        let name = match stop {
            Stop::Em('_') => "`_`",
            Stop::Em('*') => "`*`",
            Stop::Em(_) => "emphasis",
            Stop::Strong => "`**`",
            Stop::Sup => "`^`",
            Stop::Sub => "`~`",
            Stop::LinkLabel => "`]`",
            Stop::None => "",
        };
        return Err(format!("unclosed inline marker {name}"));
    }
    flush(&mut buffer, &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(inlines: Vec<Inline>) {
        let mut text = String::new();
        write_inlines(&inlines, &mut text);
        let parsed = parse_inlines(&text).unwrap();
        assert_eq!(parsed, inlines, "source was {text:?}");
    }

    #[test]
    fn text_with_every_special_character_round_trips() {
        round_trip(vec![Inline::text(
            "lead #tag *star* ^sup^ ~sub~ [bracket] (paren) |pipe| {brace} <tag> > quote \\ slash",
        )]);
    }

    #[test]
    fn literal_newline_and_carriage_return_use_their_own_escape() {
        round_trip(vec![Inline::text("a\nb")]);
        round_trip(vec![Inline::text("a\rb")]);
    }

    #[test]
    fn adjacent_and_empty_text_nodes_keep_their_boundary() {
        round_trip(vec![Inline::text("a"), Inline::text("b")]);
        round_trip(vec![Inline::text(""), Inline::text("b")]);
        round_trip(vec![Inline::text("a"), Inline::text("")]);
        round_trip(vec![Inline::text(""), Inline::text("")]);
    }

    #[test]
    fn every_node_kind_round_trips() {
        round_trip(vec![
            Inline::text("A "),
            Inline::Em {
                c: vec![Inline::text("em")],
            },
            Inline::text(" "),
            Inline::Strong {
                c: vec![Inline::text("strong")],
            },
            Inline::text(" "),
            Inline::Sup {
                c: vec![Inline::text("1")],
            },
            Inline::text(" "),
            Inline::Sub {
                c: vec![Inline::text("2")],
            },
            Inline::Break,
            Inline::Link {
                target: EntryId::parse("ref/spell/fog-cloud").unwrap(),
                anchor: Some("effects".into()),
                c: vec![Inline::text("fog cloud")],
            },
        ]);
    }

    #[test]
    fn nested_emphasis_round_trips() {
        round_trip(vec![Inline::Em {
            c: vec![
                Inline::text("a"),
                Inline::Strong {
                    c: vec![Inline::text("b")],
                },
            ],
        }]);
        round_trip(vec![Inline::Strong {
            c: vec![
                Inline::text("a"),
                Inline::Em {
                    c: vec![Inline::text("b")],
                },
            ],
        }]);
    }

    #[test]
    fn dangling_escape_and_unclosed_markers_are_errors() {
        assert!(parse_inlines("a\\").is_err());
        assert!(parse_inlines("*open").is_err());
        assert!(parse_inlines("**open").is_err());
        assert!(parse_inlines("^open").is_err());
        assert!(parse_inlines("~open").is_err());
        assert!(parse_inlines("[open").is_err());
        assert!(parse_inlines("stray ]").is_err());
    }
}
