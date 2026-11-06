//! The inline subset. It maps one-to-one to [`Inline`].
//!
//! See `docs/corpus-format.md` for the user-facing grammar. The important
//! invariants: a backslash escapes the next character; a backslash at the end
//! of a physical line is [`Inline::Break`]; an ordinary line end inside a
//! paragraph is a space; `\N` is a literal newline and `\R` a literal carriage
//! return. Adjacent and empty `Text` nodes have no spelling: the writer merges
//! them, and the compiler never produces them.

use cast_iron_schema::{EntryId, Inline};

const SPECIAL: &[char] = &[
    '\\', '*', '_', '^', '~', '[', ']', '#', '>', '|', '{', '}', '<',
];

/// Default paragraph width in characters.
pub const WRAP_WIDTH: usize = 100;

fn escape_char(ch: char, out: &mut String) {
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

/// How a multi-line run is laid out.
#[derive(Clone, Copy, Debug)]
pub struct Flow {
    /// Greedy wrap column. `usize::MAX` never wraps.
    pub width: usize,
    /// Escape a line start that would read as a block marker (paragraphs).
    pub guard: bool,
}

/// Writes an inline sequence on one physical line. [`Inline::Break`] becomes the
/// `\n` escape, because a heading, a pipe cell, and a TOML string cannot hold a
/// second line. Adjacent `Text` nodes merge and empty ones vanish.
pub fn write_inlines(inlines: &[Inline], out: &mut String) {
    let mut writer = Writer::new(false);
    writer.emit(inlines, 0);
    out.push_str(&writer.finish_single());
}

/// Writes inline text for a place the reader trims, a heading or a pipe table
/// cell: the blanks at either end are escaped so they survive the trim.
pub fn write_trimmed(inlines: &[Inline]) -> String {
    let mut text = String::new();
    write_inlines(inlines, &mut text);
    let leading = text.chars().take_while(|ch| is_blank_char(*ch)).count();
    let rest = &text[leading..];
    let trailing = rest
        .chars()
        .rev()
        .take_while(|ch| is_blank_char(*ch))
        .count();
    let keep = rest.len() - trailing;
    let mut out = String::with_capacity(text.len() + leading + trailing);
    for ch in text.chars().take(leading) {
        out.push('\\');
        out.push(ch);
    }
    out.push_str(&rest[..keep]);
    for ch in rest[keep..].chars() {
        out.push('\\');
        out.push(ch);
    }
    out
}

/// Parses text that the writer escaped with [`write_trimmed`]: the blanks at
/// either end are layout, not content.
pub fn parse_trimmed(text: &str) -> Result<Vec<Inline>, String> {
    parse_inlines(trim_line_end(text.trim_start_matches([' ', '\t'])))
}

/// Writes a paragraph-like run over several physical lines: each
/// [`Inline::Break`] ends a line with a backslash, and text wraps at
/// `flow.width`. Never breaks inside a link, an escape, or at a space that the
/// reader would not give back.
pub fn write_flow(inlines: &[Inline], flow: Flow) -> String {
    let mut writer = Writer::new(true);
    writer.emit(inlines, 0);
    writer.finish_flow(flow)
}

struct Writer {
    flow: bool,
    link_depth: usize,
    line: String,
    /// Byte offsets in `line` of spaces that came from text outside a link.
    breakable: Vec<usize>,
    done: Vec<(String, Vec<usize>)>,
}

impl Writer {
    fn new(flow: bool) -> Self {
        Self {
            flow,
            link_depth: 0,
            line: String::new(),
            breakable: Vec::new(),
            done: Vec::new(),
        }
    }

    fn text(&mut self, value: &str) {
        for ch in value.chars() {
            if ch == ' ' && self.flow && self.link_depth == 0 {
                self.breakable.push(self.line.len());
            }
            escape_char(ch, &mut self.line);
        }
    }

    fn emit(&mut self, inlines: &[Inline], em_depth: usize) {
        let mut pending = String::new();
        for inline in inlines {
            if let Inline::Text { v } = inline {
                pending.push_str(v);
                continue;
            }
            self.text(&pending);
            pending.clear();
            match inline {
                Inline::Text { .. } => {}
                Inline::Em { c } => {
                    // Alternate the delimiter so an emphasis span can nest
                    // inside another one without closing the outer span early.
                    let delimiter = if em_depth.is_multiple_of(2) { '_' } else { '*' };
                    self.line.push(delimiter);
                    self.emit(c, em_depth + 1);
                    self.line.push(delimiter);
                }
                Inline::Strong { c } => {
                    self.line.push_str("**");
                    self.emit(c, em_depth);
                    self.line.push_str("**");
                }
                Inline::Sup { c } => {
                    self.line.push('^');
                    self.emit(c, em_depth);
                    self.line.push('^');
                }
                Inline::Sub { c } => {
                    self.line.push('~');
                    self.emit(c, em_depth);
                    self.line.push('~');
                }
                Inline::Break => {
                    if self.flow {
                        let line = std::mem::take(&mut self.line);
                        let breakable = std::mem::take(&mut self.breakable);
                        self.done.push((line, breakable));
                    } else {
                        self.line.push_str("\\n");
                    }
                }
                Inline::Link { target, anchor, c } => {
                    self.line.push('[');
                    self.link_depth += 1;
                    self.emit(c, em_depth);
                    self.link_depth -= 1;
                    self.line.push_str("](");
                    self.line.push_str(target.as_str());
                    if let Some(anchor) = anchor {
                        self.line.push('#');
                        self.line.push_str(anchor);
                    }
                    self.line.push(')');
                }
            }
        }
        self.text(&pending);
    }

    fn finish_single(self) -> String {
        self.line
    }

    fn finish_flow(mut self, flow: Flow) -> String {
        let line = std::mem::take(&mut self.line);
        let breakable = std::mem::take(&mut self.breakable);
        self.done.push((line, breakable));
        let total = self.done.len();
        let mut out: Vec<String> = Vec::new();
        for (index, (text, breakable)) in self.done.into_iter().enumerate() {
            let last = index + 1 == total;
            let mut pieces = layout_line(text, &breakable, last, flow);
            if !last {
                if let Some(piece) = pieces.last_mut() {
                    piece.push('\\');
                }
            } else if pieces.last().is_some_and(String::is_empty) {
                // An empty final line is the end of the paragraph, not a line.
                pieces.pop();
            }
            out.extend(pieces);
        }
        out.join("\n")
    }
}

fn is_blank_char(ch: char) -> bool {
    ch == ' ' || ch == '\t'
}

/// Escapes the whitespace the reader would trim, protects a block marker at
/// the line start, then wraps.
fn layout_line(text: String, breakable: &[usize], last: bool, flow: Flow) -> Vec<String> {
    let bytes = text.as_bytes();
    let blank = |index: usize| matches!(bytes[index], b' ' | b'\t');
    let valid: Vec<usize> = breakable
        .iter()
        .copied()
        .filter(|&index| {
            index > 0 && index + 1 < bytes.len() && !blank(index - 1) && !blank(index + 1)
        })
        .collect();

    let leading = text.chars().take_while(|ch| is_blank_char(*ch)).count();
    let mut escaped = String::with_capacity(text.len() + leading + 1);
    for ch in text.chars().take(leading) {
        escaped.push('\\');
        escaped.push(ch);
    }
    let mut shift = leading;
    let rest = &text[leading..];
    let trailing = if last {
        rest.chars()
            .rev()
            .take_while(|ch| is_blank_char(*ch))
            .count()
    } else {
        0
    };
    let keep = rest.len() - trailing;
    escaped.push_str(&rest[..keep]);
    for ch in rest[keep..].chars() {
        escaped.push('\\');
        escaped.push(ch);
    }
    if flow.guard && leading == 0 && starts_block(&escaped) {
        escaped.insert(0, '\\');
        shift += 1;
    }
    let valid: Vec<usize> = valid
        .into_iter()
        .map(|index| index + shift)
        .filter(|&index| !starts_block(&escaped[index + 1..]))
        .collect();
    wrap(&escaped, &valid, flow.width)
}

fn columns(text: &str) -> usize {
    text.chars().count()
}

/// Greedy wrap at the given space offsets. A line without a usable space stays
/// long rather than break inside a token.
fn wrap(text: &str, candidates: &[usize], width: usize) -> Vec<String> {
    if width == usize::MAX || candidates.is_empty() {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let mut best: Option<usize> = None;
    'candidates: for &candidate in candidates {
        while columns(&text[start..candidate]) > width {
            if let Some(at) = best.take() {
                lines.push(text[start..at].to_string());
                start = at + 1;
            } else {
                lines.push(text[start..candidate].to_string());
                start = candidate + 1;
                continue 'candidates;
            }
        }
        best = Some(candidate);
    }
    while columns(&text[start..]) > width {
        let Some(at) = best.take() else { break };
        lines.push(text[start..at].to_string());
        start = at + 1;
    }
    lines.push(text[start..].to_string());
    lines
}

/// True when a line that starts with `value` would read as another block.
pub fn starts_block(value: &str) -> bool {
    let first = value.as_bytes().first().copied();
    match first {
        Some(b'-') | Some(b'+') => value.len() == 1 || value.as_bytes().get(1) == Some(&b' '),
        Some(b':') => value.starts_with(":::"),
        Some(b'0'..=b'9') => {
            let digits = value.bytes().take_while(u8::is_ascii_digit).count();
            let rest = &value[digits..];
            rest == "." || rest.starts_with(". ")
        }
        _ => false,
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

/// Parses an inline run that sits on one physical line. The error text has no
/// line number; the block parser attaches the source line.
pub fn parse_inlines(text: &str) -> Result<Vec<Inline>, String> {
    parse_with(text, false)
}

/// Parses the physical lines of one paragraph. Each line is trimmed of the
/// whitespace around it, a line end is a space, and a backslash at the end of a
/// line is an [`Inline::Break`].
pub fn parse_flow(lines: &[&str]) -> Result<Vec<Inline>, String> {
    let trimmed: Vec<&str> = lines
        .iter()
        .map(|line| trim_line_end(line.trim_start_matches([' ', '\t'])))
        .collect();
    parse_with(&trimmed.join("\n"), true)
}

/// Trims blank characters from the end of a line, keeping one that a backslash
/// escapes.
pub fn trim_line_end(line: &str) -> &str {
    let trimmed = line.trim_end_matches([' ', '\t']);
    if trimmed.len() == line.len() {
        return line;
    }
    let slashes = trimmed
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count();
    if slashes % 2 == 1 {
        &line[..trimmed.len() + 1]
    } else {
        trimmed
    }
}

/// True when the line ends with an unescaped backslash, a hard break.
pub fn ends_with_break(line: &str) -> bool {
    line.bytes().rev().take_while(|byte| *byte == b'\\').count() % 2 == 1
}

fn parse_with(text: &str, trailing_break: bool) -> Result<Vec<Inline>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut position = 0;
    let inlines = parse_seq(&chars, &mut position, Stop::None, trailing_break)?;
    if position != chars.len() {
        return Err(format!(
            "unexpected {:?} at the end of inline text",
            chars[position]
        ));
    }
    Ok(inlines)
}

fn parse_seq(
    chars: &[char],
    position: &mut usize,
    stop: Stop,
    trailing_break: bool,
) -> Result<Vec<Inline>, String> {
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
            '\n' => {
                // A line end inside a paragraph is a space.
                buffer.push(' ');
                *position += 1;
            }
            '\\' => {
                let Some(next) = chars.get(*position + 1).copied() else {
                    if !trailing_break {
                        return Err("dangling backslash at the end of an inline run".to_string());
                    }
                    *position += 1;
                    flush(&mut buffer, &mut out);
                    out.push(Inline::Break);
                    continue;
                };
                *position += 2;
                match next {
                    // A backslash at the end of a line. `\n` also reads as a
                    // break: it is how a break is written where a second line
                    // cannot go (a heading or a table cell).
                    '\n' | 'n' => {
                        flush(&mut buffer, &mut out);
                        out.push(Inline::Break);
                    }
                    'N' => buffer.push('\n'),
                    'R' => buffer.push('\r'),
                    other if other.is_ascii_punctuation() || other.is_ascii_digit() => {
                        buffer.push(other)
                    }
                    ' ' | '\t' => buffer.push(next),
                    other => {
                        return Err(format!(
                            "unknown escape `\\{other}`; a backslash escapes punctuation, a digit, or a space"
                        ));
                    }
                }
            }
            '<' => {
                // The writer escapes every `<`, so an unescaped one that opens a
                // tag or a comment is raw HTML, which the corpus never accepts.
                match chars.get(*position + 1) {
                    Some('!') if chars.get(*position + 2) == Some(&'-') => {
                        return Err(
                            "an HTML comment is not allowed in the corpus; remove it, or write `\\<` for a literal `<`"
                                .to_string(),
                        );
                    }
                    Some(next) if next.is_ascii_alphabetic() || matches!(next, '/' | '!' | '?') => {
                        return Err(format!(
                            "raw HTML (`<{next}...`) is not allowed in the corpus; write `\\<` for a literal `<`"
                        ));
                    }
                    _ => {
                        buffer.push('<');
                        *position += 1;
                    }
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
                let c = parse_seq(chars, position, Stop::Strong, trailing_break)?;
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
                let c = parse_seq(chars, position, Stop::Em(ch), trailing_break)?;
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
                let c = parse_seq(chars, position, Stop::Sup, trailing_break)?;
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
                let c = parse_seq(chars, position, Stop::Sub, trailing_break)?;
                out.push(Inline::Sub { c });
            }
            '[' => {
                flush(&mut buffer, &mut out);
                *position += 1;
                let c = parse_seq(chars, position, Stop::LinkLabel, trailing_break)?;
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
    fn adjacent_text_merges_and_empty_text_vanishes() {
        let mut text = String::new();
        write_inlines(
            &[Inline::text("a"), Inline::text(""), Inline::text("b")],
            &mut text,
        );
        assert_eq!(text, "ab");
        assert_eq!(parse_inlines(&text).unwrap(), vec![Inline::text("ab")]);
        assert!(!text.contains("\\0") && !text.contains("\\e"));
    }

    #[test]
    fn old_text_node_escapes_are_errors() {
        for text in ["a\\eb", "\\e", "\\q", "\\é"] {
            let error = parse_inlines(text).unwrap_err();
            assert!(error.contains("unknown escape"), "{text}: {error}");
        }
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

    fn flow(inlines: &[Inline], width: usize) -> String {
        write_flow(inlines, Flow { width, guard: true })
    }

    fn flow_round_trip(inlines: Vec<Inline>, width: usize) -> String {
        let text = flow(&inlines, width);
        let lines: Vec<&str> = text.split('\n').collect();
        let parsed = parse_flow(&lines).unwrap();
        assert_eq!(parsed, inlines, "source was {text:?}");
        text
    }

    #[test]
    fn a_break_is_a_trailing_backslash_and_starts_a_new_line() {
        let text = flow_round_trip(
            vec![Inline::text("one"), Inline::Break, Inline::text("two")],
            100,
        );
        assert_eq!(text, "one\\\ntwo");
        assert_eq!(
            flow_round_trip(vec![Inline::Break], 100),
            "\\",
            "a lone break"
        );
        flow_round_trip(
            vec![
                Inline::text("a"),
                Inline::Break,
                Inline::Break,
                Inline::text("b"),
            ],
            100,
        );
        flow_round_trip(vec![Inline::text("a"), Inline::Break], 100);
        flow_round_trip(vec![Inline::Break, Inline::text("a")], 100);
    }

    #[test]
    fn a_literal_backslash_before_a_break_stays_distinct() {
        let text = flow_round_trip(
            vec![Inline::text("a\\"), Inline::Break, Inline::text("b")],
            100,
        );
        assert_eq!(text, "a\\\\\\\nb");
        flow_round_trip(vec![Inline::text("ends with a backslash \\")], 100);
    }

    #[test]
    fn long_text_wraps_at_single_spaces_and_joins_back_with_a_space() {
        let words: Vec<String> = (0..60).map(|n| format!("word{n}")).collect();
        let inlines = vec![Inline::text(words.join(" "))];
        let text = flow_round_trip(inlines, 40);
        assert!(text.lines().count() > 5);
        for line in text.lines() {
            assert!(line.chars().count() <= 40, "{line:?}");
            assert!(!line.ends_with(' ') && !line.starts_with(' '));
        }
    }

    #[test]
    fn wrapping_never_splits_a_link_or_an_escape() {
        let link = Inline::Link {
            target: EntryId::parse("ref/spell/fog-cloud").unwrap(),
            anchor: Some("effects".into()),
            c: vec![Inline::text("a long link label with many words in it")],
        };
        let text = flow_round_trip(
            vec![
                Inline::text("lead text "),
                link,
                Inline::text(" trailing words that follow the link \\ and \n"),
            ],
            30,
        );
        let link_line = text
            .lines()
            .find(|line| line.contains("a long link label"))
            .unwrap();
        assert!(
            link_line.contains("](ref/spell/fog-cloud#effects)"),
            "{text}"
        );
    }

    #[test]
    fn wrapping_does_not_start_a_line_with_a_block_marker() {
        let inlines = vec![Inline::text(
            "aaaaaaaaaa bbbbbbbbbb - cccccccccc 12. dddddddddd ::: eeeeeeeeee 1 ffffffffff",
        )];
        for width in 10..40 {
            let text = flow_round_trip(inlines.clone(), width);
            for line in text.lines().skip(1) {
                assert!(!starts_block(line), "{line:?} at width {width}");
            }
        }
    }

    #[test]
    fn runs_of_spaces_survive_wrapping_and_edges() {
        for text in [
            "double  space and a lot of words to wrap around the line end here",
            "  leading spaces",
            "trailing spaces  ",
            " ",
            "\t tab \t",
            "a \t b",
        ] {
            for width in [8, 20, 100] {
                flow_round_trip(vec![Inline::text(text)], width);
            }
        }
        flow_round_trip(
            vec![
                Inline::text("a "),
                Inline::Break,
                Inline::text(" b"),
                Inline::Break,
                Inline::text("  "),
            ],
            100,
        );
    }

    #[test]
    fn a_break_inside_emphasis_round_trips() {
        flow_round_trip(
            vec![Inline::Em {
                c: vec![Inline::text("one"), Inline::Break, Inline::text("two")],
            }],
            100,
        );
    }

    #[test]
    fn a_line_end_in_a_paragraph_is_a_space_and_edge_blanks_are_trimmed() {
        let parsed = parse_flow(&["one  ", "   two", "three"]).unwrap();
        assert_eq!(parsed, vec![Inline::text("one two three")]);
        // An escaped trailing space is content.
        let parsed = parse_flow(&["one\\ ", "two"]).unwrap();
        assert_eq!(parsed, vec![Inline::text("one  two")]);
    }

    #[test]
    fn a_backslash_at_the_end_of_one_line_is_a_dangling_error() {
        assert!(parse_inlines("a\\").is_err());
        assert_eq!(
            parse_flow(&["a\\"]).unwrap(),
            vec![Inline::text("a"), Inline::Break]
        );
    }

    #[test]
    fn a_single_line_break_uses_the_n_escape() {
        let mut text = String::new();
        write_inlines(
            &[Inline::text("a"), Inline::Break, Inline::text("b")],
            &mut text,
        );
        assert_eq!(text, "a\\nb");
        assert_eq!(parse_inlines(&text).unwrap().len(), 3);
    }

    #[test]
    fn random_text_round_trips_at_several_widths() {
        let atoms = [
            "word", "a", " ", "  ", "- ", "1. ", ":::", "\\", "*", "_", "#", "|", "[x]", "~", "^",
            "é", "\t", "(", ")", "{#a}", "<b>",
        ];
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..400 {
            let mut inlines = Vec::new();
            let mut text = String::new();
            for _ in 0..(next() % 30) {
                match next() % 12 {
                    0 => {
                        if !text.is_empty() {
                            inlines.push(Inline::text(std::mem::take(&mut text)));
                        }
                        inlines.push(Inline::Break);
                    }
                    1 => {
                        if !text.is_empty() {
                            inlines.push(Inline::text(std::mem::take(&mut text)));
                        }
                        inlines.push(Inline::Strong {
                            c: vec![Inline::text("two words")],
                        });
                    }
                    n => text.push_str(atoms[(n as usize * 7 + next() as usize) % atoms.len()]),
                }
            }
            if !text.is_empty() {
                inlines.push(Inline::text(text));
            }
            for width in [6, 17, 100] {
                flow_round_trip(inlines.clone(), width);
            }
        }
    }

    #[test]
    fn raw_html_and_comments_are_rejected() {
        for text in [
            "<b>bold</b>",
            "a <!-- note --> b",
            "x </i>",
            "<?php",
            "<!DOCTYPE>",
        ] {
            let error = parse_inlines(text).unwrap_err();
            assert!(error.contains("HTML"), "{text}: {error}");
        }
        assert!(parse_inlines("<!-- c -->").unwrap_err().contains("comment"));
        // An escaped bracket and a bare one that opens nothing are text.
        assert_eq!(
            parse_inlines("a \\<b> c").unwrap(),
            vec![Inline::text("a <b> c")]
        );
        assert_eq!(parse_inlines("1 < 2").unwrap(), vec![Inline::text("1 < 2")]);
    }

    #[test]
    fn trimmed_text_keeps_escaped_edge_blanks() {
        for text in [" lead", "trail ", "  both  ", " ", "a b"] {
            let inlines = vec![Inline::text(text)];
            let written = write_trimmed(&inlines);
            assert_eq!(parse_trimmed(&written).unwrap(), inlines, "{written:?}");
            // Extra blanks around the written form are layout.
            let padded = format!("  {written}   ");
            assert_eq!(parse_trimmed(&padded).unwrap(), inlines, "{padded:?}");
        }
    }
}
