//! Block writer and parser for the Markdown subset in `docs/corpus-format.md`.

use crate::error::{Error, Result};
use crate::inline::{parse_inlines, write_inline_line, write_inlines};
use crate::tomlutil;
use cast_iron_schema::{
    Block, EntryId, Inline, ParagraphStyle, StatRow, Table, TableCell, TableRow,
};
use serde::Deserialize;
use std::path::Path;

/// One source line with its absolute 1-based number.
#[derive(Clone, Debug)]
struct Line {
    text: String,
    number: usize,
}

/// Renders blocks separated by one blank line, without a trailing newline.
pub fn write_blocks(blocks: &[Block]) -> String {
    let mut rendered = Vec::with_capacity(blocks.len());
    for block in blocks {
        rendered.push(write_block(block));
    }
    rendered.join("\n\n")
}

pub fn parse_blocks(path: &Path, lines: &[(usize, &str)]) -> Result<Vec<Block>> {
    let owned: Vec<Line> = lines
        .iter()
        .map(|(number, text)| Line {
            text: (*text).to_string(),
            number: *number,
        })
        .collect();
    parse_lines(path, &owned)
}

fn write_block(block: &Block) -> String {
    match block {
        Block::Heading {
            level,
            anchor,
            content,
        } => {
            let mut out = format!("{} ", "#".repeat(usize::from(*level)));
            // Inline text escapes `{` and `}`, so a literal `{#...}` suffix in
            // the heading text cannot be read back as an anchor.
            out.push_str(&write_inline_line(content));
            if let Some(anchor) = anchor {
                out.push_str(" {#");
                out.push_str(anchor);
                out.push('}');
            }
            out
        }
        Block::Paragraph { style, content } => match style {
            ParagraphStyle::Normal => write_inline_line(content),
            ParagraphStyle::StatLine => format!(
                "::: paragraph statline\n{}\n:::",
                write_inline_line(content)
            ),
            ParagraphStyle::Indented => {
                format!(
                    "::: paragraph indented\n{}\n:::",
                    write_inline_line(content)
                )
            }
        },
        Block::List { ordered, items } => write_list(*ordered, items),
        Block::Table(table) => write_table(table),
        Block::Quote { content } => write_quote(content),
        Block::StatBlock { title, rows } => write_stat_block(title.as_deref(), rows),
        Block::EntryLink { target, title } => format!(
            "::: entry-link\ntarget = {}\ntitle = {}\n:::",
            tomlutil::id(target),
            tomlutil::string(title)
        ),
    }
}

fn write_list(ordered: bool, items: &[Vec<Block>]) -> String {
    let marker = if ordered { "1." } else { "-" };
    let mut out = String::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(marker);
        let mut blocks = item.iter();
        if let Some(Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }) = blocks.next()
        {
            out.push(' ');
            out.push_str(&write_inline_line(content));
        } else {
            for block in item {
                out.push('\n');
                indent(&mut out, &write_block(block), 2);
            }
            continue;
        }
        for block in blocks {
            out.push_str("\n\n");
            indent(&mut out, &write_block(block), 2);
        }
    }
    out
}

fn indent(out: &mut String, text: &str, spaces: usize) {
    let prefix = " ".repeat(spaces);
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if !line.is_empty() {
            out.push_str(&prefix);
        }
        out.push_str(line);
    }
}

fn write_quote(content: &[Block]) -> String {
    let inner = write_blocks(content);
    let mut out = String::new();
    for (index, line) in inner.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if line.is_empty() {
            out.push('>');
        } else {
            out.push_str("> ");
            out.push_str(line);
        }
    }
    if out.is_empty() {
        out.push('>');
    }
    out
}

fn simple_table(table: &Table) -> bool {
    if !table.caption.is_empty() || !table.foot.is_empty() || table.head.len() != 1 {
        return false;
    }
    let rows: Vec<&TableRow> = table.head.iter().chain(&table.body).collect();
    if rows.is_empty() || rows.iter().any(|row| row.cells.is_empty()) {
        return false;
    }
    let columns = rows[0].cells.len();
    if rows.iter().any(|row| row.cells.len() != columns) {
        return false;
    }
    if table.head[0]
        .cells
        .iter()
        .any(|cell| !cell.header || cell.colspan != 1 || cell.rowspan != 1)
    {
        return false;
    }
    if table.body.iter().any(|row| {
        row.cells
            .iter()
            .any(|cell| cell.header || cell.colspan != 1 || cell.rowspan != 1)
    }) {
        return false;
    }
    true
}

fn write_table(table: &Table) -> String {
    if simple_table(table) {
        let mut out = String::new();
        write_pipe_row(&mut out, &table.head[0]);
        out.push('\n');
        let columns = table.head[0].cells.len();
        out.push('|');
        for _ in 0..columns {
            out.push_str(" --- |");
        }
        for row in &table.body {
            out.push('\n');
            write_pipe_row(&mut out, row);
        }
        return out;
    }
    write_fenced_table(table)
}

fn write_pipe_row(out: &mut String, row: &TableRow) {
    out.push('|');
    for cell in &row.cells {
        let mut text = String::new();
        write_inlines(&cell.content, &mut text);
        out.push_str(&text);
        out.push('|');
    }
}

fn write_fenced_table(table: &Table) -> String {
    let mut out = String::from("::: table\n");
    if !table.caption.is_empty() {
        let mut text = String::new();
        write_inlines(&table.caption, &mut text);
        out.push_str("caption = ");
        out.push_str(&tomlutil::string(&text));
        out.push('\n');
    }
    for (name, rows) in [
        ("head", &table.head),
        ("body", &table.body),
        ("foot", &table.foot),
    ] {
        if rows.is_empty() {
            continue;
        }
        out.push_str(name);
        out.push_str(" = [\n");
        for row in rows {
            out.push_str("  [\n");
            for cell in &row.cells {
                let mut text = String::new();
                write_inlines(&cell.content, &mut text);
                out.push_str("    { text = ");
                out.push_str(&tomlutil::string(&text));
                if cell.header {
                    out.push_str(", header = true");
                }
                if cell.colspan != 1 {
                    out.push_str(&format!(", colspan = {}", cell.colspan));
                }
                if cell.rowspan != 1 {
                    out.push_str(&format!(", rowspan = {}", cell.rowspan));
                }
                out.push_str(" },\n");
            }
            out.push_str("  ],\n");
        }
        out.push_str("]\n");
    }
    out.push_str(":::");
    out
}

fn write_stat_block(title: Option<&str>, rows: &[StatRow]) -> String {
    let mut out = String::from("::: statblock\n");
    if let Some(title) = title {
        out.push_str("title = ");
        out.push_str(&tomlutil::string(title));
        out.push('\n');
    }
    for row in rows {
        out.push_str(&escape_label(&row.label));
        out.push_str(": ");
        let mut value = String::new();
        write_inlines(&row.value, &mut value);
        out.push_str(&value);
        out.push('\n');
    }
    out.push_str(":::");
    out
}

fn escape_label(label: &str) -> String {
    let mut out = String::new();
    for ch in label.chars() {
        match ch {
            '\\' | ':' => {
                out.push('\\');
                out.push(ch);
            }
            ch => out.push(ch),
        }
    }
    out
}

fn parse_lines(path: &Path, lines: &[Line]) -> Result<Vec<Block>> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if is_blank(&lines[index].text) {
            index += 1;
            continue;
        }
        let line = &lines[index];
        let text = line.text.as_str();
        if text == ":::" || text.starts_with("::: ") || text.starts_with(":::") {
            let (block, next) = parse_fence(path, lines, index)?;
            blocks.push(block);
            index = next;
        } else if text.starts_with('#') {
            let (block, next) = parse_heading(path, lines, index)?;
            blocks.push(block);
            index = next;
        } else if text == ">" || text.starts_with("> ") {
            let (block, next) = parse_quote(path, lines, index)?;
            blocks.push(block);
            index = next;
        } else if is_unordered_item(text) || is_ordered_item(text) {
            let (block, next) = parse_list(path, lines, index)?;
            blocks.push(block);
            index = next;
        } else if text.starts_with('|') {
            let (block, next) = parse_pipe_table(path, lines, index)?;
            blocks.push(block);
            index = next;
        } else {
            let (block, next) = parse_paragraph(path, lines, index)?;
            blocks.push(block);
            index = next;
        }
    }
    Ok(blocks)
}

fn is_blank(text: &str) -> bool {
    // Only the ASCII blank characters separate blocks. Unicode spaces such as a
    // non-breaking space are content and must survive as a paragraph.
    text.chars().all(|ch| ch == ' ' || ch == '\t')
}

fn parse_heading(path: &Path, lines: &[Line], index: usize) -> Result<(Block, usize)> {
    let text = &lines[index].text;
    let hashes = text.chars().take_while(|ch| *ch == '#').count();
    if !(2..=6).contains(&hashes) || text.chars().nth(hashes) != Some(' ') {
        return Err(Error::new(
            path,
            lines[index].number,
            "a heading must start with 2 to 6 `#` and a space",
        ));
    }
    let mut rest = &text[hashes + 1..];
    let mut anchor = None;
    if rest.ends_with('}')
        && let Some(start) = rest.rfind(" {#")
    {
        let candidate = &rest[start + 3..rest.len() - 1];
        if candidate.is_empty()
            || !candidate
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(Error::new(
                path,
                lines[index].number,
                format!("invalid heading anchor `{{{candidate}}}`"),
            ));
        }
        anchor = Some(candidate.to_string());
        rest = rest[..start].trim_end();
    }
    let content = parse_inlines(rest)
        .map_err(|e| Error::new(path, lines[index].number, format!("heading: {e}")))?;
    Ok((
        Block::Heading {
            level: u8::try_from(hashes).unwrap_or(6),
            anchor,
            content,
        },
        index + 1,
    ))
}

fn parse_paragraph(path: &Path, lines: &[Line], index: usize) -> Result<(Block, usize)> {
    let mut end = index;
    while end < lines.len() && !is_blank(&lines[end].text) {
        // A block marker only starts a new block after a blank line. This keeps
        // a paragraph from silently swallowing a heading the writer never
        // produced; the writer always separates blocks with a blank line.
        if end > index {
            let text = lines[end].text.as_str();
            if text.starts_with(":::")
                || text.starts_with('#')
                || text == ">"
                || text.starts_with("> ")
                || is_unordered_item(text)
                || is_ordered_item(text)
                || text.starts_with('|')
            {
                break;
            }
        }
        end += 1;
    }
    let joined = lines[index..end]
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let content = parse_inlines(&joined)
        .map_err(|e| Error::new(path, lines[index].number, format!("paragraph: {e}")))?;
    Ok((
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        },
        end,
    ))
}

fn is_unordered_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn is_ordered_item(text: &str) -> bool {
    let digits = text.chars().take_while(char::is_ascii_digit).count();
    let rest = &text[digits..];
    digits > 0 && (rest == "." || rest.starts_with(". "))
}

fn parse_list(path: &Path, lines: &[Line], index: usize) -> Result<(Block, usize)> {
    let ordered = is_ordered_item(&lines[index].text) && !is_unordered_item(&lines[index].text);
    let mut items = Vec::new();
    let mut cursor = index;
    while cursor < lines.len() {
        let text = lines[cursor].text.as_str();
        let marker_len = if is_unordered_item(text) {
            if text == "-" { 1 } else { 2 }
        } else if is_ordered_item(text) {
            text.chars().take_while(char::is_ascii_digit).count() + 2
        } else {
            break;
        };
        let this_ordered = is_ordered_item(text) && !is_unordered_item(text);
        if this_ordered != ordered {
            break;
        }
        let rest = if text.len() > marker_len {
            &text[marker_len..]
        } else {
            ""
        };
        let mut content = Vec::new();
        if !rest.is_empty() {
            content.push(Line {
                text: rest.to_string(),
                number: lines[cursor].number,
            });
        }
        cursor += 1;
        while cursor < lines.len() {
            if is_blank(&lines[cursor].text) {
                let next = cursor + 1;
                // A blank line ends the list unless the next line is indented
                // as item content. The writer never separates two items with a
                // blank line, so this keeps adjacent lists as separate blocks.
                if next < lines.len() && indent_of(&lines[next].text) >= 2 {
                    content.push(Line {
                        text: String::new(),
                        number: lines[cursor].number,
                    });
                    cursor += 1;
                } else {
                    break;
                }
            } else if indent_of(&lines[cursor].text) >= 2 {
                content.push(Line {
                    text: lines[cursor].text[2..].to_string(),
                    number: lines[cursor].number,
                });
                cursor += 1;
            } else {
                break;
            }
        }
        items.push(parse_lines(path, &content)?);
    }
    Ok((Block::List { ordered, items }, cursor))
}

fn indent_of(text: &str) -> usize {
    text.chars().take_while(|ch| *ch == ' ').count()
}

fn parse_quote(path: &Path, lines: &[Line], index: usize) -> Result<(Block, usize)> {
    let mut content = Vec::new();
    let mut cursor = index;
    while cursor < lines.len() {
        let text = lines[cursor].text.as_str();
        let stripped = if text == ">" {
            ""
        } else if let Some(rest) = text.strip_prefix("> ") {
            rest
        } else {
            break;
        };
        content.push(Line {
            text: stripped.to_string(),
            number: lines[cursor].number,
        });
        cursor += 1;
    }
    let inner = parse_lines(path, &content)?;
    Ok((Block::Quote { content: inner }, cursor))
}

fn parse_pipe_table(path: &Path, lines: &[Line], index: usize) -> Result<(Block, usize)> {
    let header_line = &lines[index];
    let header = split_pipe_row(path, header_line)?;
    if index + 1 >= lines.len() || !lines[index + 1].text.starts_with('|') {
        return Err(Error::new(
            path,
            header_line.number,
            "a pipe table needs a `| --- |` delimiter row",
        ));
    }
    let delimiter = split_pipe_row(path, &lines[index + 1])?;
    if delimiter.len() != header.len() || delimiter.iter().any(|cell| cell.trim() != "---") {
        return Err(Error::new(
            path,
            lines[index + 1].number,
            "the delimiter row must hold one `---` per column",
        ));
    }
    let header_cells = header
        .iter()
        .map(|cell| {
            let content = parse_inlines(cell)
                .map_err(|e| Error::new(path, header_line.number, format!("table header: {e}")))?;
            Ok(TableCell {
                header: true,
                colspan: 1,
                rowspan: 1,
                content,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut body = Vec::new();
    let mut cursor = index + 2;
    while cursor < lines.len() && lines[cursor].text.starts_with('|') {
        let row_line = &lines[cursor];
        let cells = split_pipe_row(path, row_line)?;
        if cells.len() != header.len() {
            return Err(Error::new(
                path,
                row_line.number,
                format!(
                    "a pipe table row has {} cells; the header has {}",
                    cells.len(),
                    header.len()
                ),
            ));
        }
        let cells = cells
            .iter()
            .map(|cell| {
                let content = parse_inlines(cell)
                    .map_err(|e| Error::new(path, row_line.number, format!("table cell: {e}")))?;
                Ok(TableCell {
                    header: false,
                    colspan: 1,
                    rowspan: 1,
                    content,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        body.push(TableRow { cells });
        cursor += 1;
    }
    Ok((
        Block::Table(Table {
            head: vec![TableRow {
                cells: header_cells,
            }],
            body,
            ..Table::default()
        }),
        cursor,
    ))
}

fn split_pipe_row(path: &Path, line: &Line) -> Result<Vec<String>> {
    let text = line.text.as_str();
    if !text.starts_with('|') || !text.ends_with('|') || text.len() < 2 {
        return Err(Error::new(
            path,
            line.number,
            "a pipe table row must start and end with `|`",
        ));
    }
    let inner = &text[1..text.len() - 1];
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                cell.push('\\');
                if let Some(next) = chars.next() {
                    cell.push(next);
                }
            }
            '|' => {
                cells.push(std::mem::take(&mut cell));
            }
            ch => cell.push(ch),
        }
    }
    cells.push(cell);
    Ok(cells)
}

fn parse_fence(path: &Path, lines: &[Line], index: usize) -> Result<(Block, usize)> {
    let opener = lines[index].text.trim();
    let kind = opener
        .strip_prefix("::: ")
        .ok_or_else(|| Error::new(path, lines[index].number, "unknown fence"))?
        .trim();
    let mut end = index + 1;
    while end < lines.len() && lines[end].text.trim() != ":::" {
        end += 1;
    }
    if end >= lines.len() {
        return Err(Error::new(
            path,
            lines[index].number,
            "a fenced block is not closed with `:::`",
        ));
    }
    let content = &lines[index + 1..end];
    let block = match kind {
        "paragraph statline" => Block::Paragraph {
            style: ParagraphStyle::StatLine,
            content: parse_fence_inlines(path, content, opener_line(lines, index))?,
        },
        "paragraph indented" => Block::Paragraph {
            style: ParagraphStyle::Indented,
            content: parse_fence_inlines(path, content, opener_line(lines, index))?,
        },
        "table" => Block::Table(parse_fenced_table(
            path,
            content,
            opener_line(lines, index),
        )?),
        "statblock" => {
            let (title, rows) = parse_stat_block(path, content, opener_line(lines, index))?;
            Block::StatBlock { title, rows }
        }
        "entry-link" => {
            let text = join_lines_newline(content);
            let raw: RawEntryLink = toml::from_str(&text).map_err(|error| {
                Error::new(
                    path,
                    opener_line(lines, index) + toml_error_line(&error, &text),
                    format!("entry-link: {error}"),
                )
            })?;
            Block::EntryLink {
                target: EntryId::parse(raw.target)
                    .map_err(|e| Error::new(path, opener_line(lines, index), e.to_string()))?,
                title: raw.title,
            }
        }
        other => {
            return Err(Error::new(
                path,
                lines[index].number,
                format!("unknown fenced block `{other}`"),
            ));
        }
    };
    Ok((block, end + 1))
}

fn opener_line(lines: &[Line], index: usize) -> usize {
    lines[index].number + 1
}

fn parse_fence_inlines(path: &Path, content: &[Line], line: usize) -> Result<Vec<Inline>> {
    let text = join_lines(content);
    parse_inlines(&text).map_err(|e| Error::new(path, line, format!("paragraph: {e}")))
}

fn join_lines(content: &[Line]) -> String {
    content
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn join_lines_newline(content: &[Line]) -> String {
    content
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntryLink {
    target: String,
    title: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTable {
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    head: Vec<Vec<RawCell>>,
    #[serde(default)]
    body: Vec<Vec<RawCell>>,
    #[serde(default)]
    foot: Vec<Vec<RawCell>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCell {
    text: String,
    #[serde(default)]
    header: bool,
    #[serde(default = "default_span")]
    colspan: u16,
    #[serde(default = "default_span")]
    rowspan: u16,
}

fn default_span() -> u16 {
    1
}

fn parse_fenced_table(path: &Path, content: &[Line], line: usize) -> Result<Table> {
    let text = join_lines_newline(content);
    if text.trim().is_empty() {
        return Ok(Table::default());
    }
    let raw: RawTable = toml::from_str(&text).map_err(|error| {
        Error::new(
            path,
            line + toml_error_line(&error, &text),
            format!("table: {error}"),
        )
    })?;
    Ok(Table {
        caption: inline_from_raw(path, raw.caption.as_deref(), line)?,
        head: raw
            .head
            .iter()
            .map(|row| raw_row(path, row, line))
            .collect::<Result<Vec<_>>>()?,
        body: raw
            .body
            .iter()
            .map(|row| raw_row(path, row, line))
            .collect::<Result<Vec<_>>>()?,
        foot: raw
            .foot
            .iter()
            .map(|row| raw_row(path, row, line))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn inline_from_raw(path: &Path, raw: Option<&str>, line: usize) -> Result<Vec<Inline>> {
    match raw {
        Some(text) => {
            parse_inlines(text).map_err(|e| Error::new(path, line, format!("table caption: {e}")))
        }
        None => Ok(Vec::new()),
    }
}

fn raw_row(path: &Path, row: &[RawCell], line: usize) -> Result<TableRow> {
    let mut cells = Vec::with_capacity(row.len());
    for cell in row {
        if cell.colspan == 0 || cell.rowspan == 0 {
            return Err(Error::new(
                path,
                line,
                format!(
                    "table cell spans must be at least 1; found colspan={} rowspan={}",
                    cell.colspan, cell.rowspan
                ),
            ));
        }
        let content = parse_inlines(&cell.text)
            .map_err(|e| Error::new(path, line, format!("table cell: {e}")))?;
        cells.push(TableCell {
            header: cell.header,
            colspan: cell.colspan,
            rowspan: cell.rowspan,
            content,
        });
    }
    Ok(TableRow { cells })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStatTitle {
    title: String,
}

fn parse_stat_block(
    path: &Path,
    content: &[Line],
    line: usize,
) -> Result<(Option<String>, Vec<StatRow>)> {
    let mut title = None;
    let mut rows = Vec::new();
    for item in content {
        if is_blank(&item.text) {
            continue;
        }
        if let Some(rest) = item.text.strip_prefix("title = ") {
            let raw: RawStatTitle =
                toml::from_str(&format!("title = {rest}")).map_err(|error| {
                    Error::new(path, item.number, format!("statblock title: {error}"))
                })?;
            title = Some(raw.title);
            continue;
        }
        let (label, value) = split_stat_row(path, item)?;
        let value = parse_inlines(&value)
            .map_err(|e| Error::new(path, item.number, format!("statblock row: {e}")))?;
        rows.push(StatRow {
            label: unescape_label(&label),
            value,
        });
    }
    let _ = line;
    Ok((title, rows))
}

fn split_stat_row(path: &Path, line: &Line) -> Result<(String, String)> {
    let mut escaped = false;
    for (index, ch) in line.text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            ':' => {
                let mut label = line.text[..index].to_string();
                let mut value = line.text[index + 1..].to_string();
                if let Some(rest) = value.strip_prefix(' ') {
                    value = rest.to_string();
                }
                label.shrink_to_fit();
                return Ok((label, value));
            }
            _ => {}
        }
    }
    Err(Error::new(
        path,
        line.number,
        "a statblock row needs `Label: value`",
    ))
}

fn unescape_label(label: &str) -> String {
    let mut out = String::new();
    let mut chars = label.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Converts a TOML error span into a zero-based line offset from `text`.
fn toml_error_line(error: &toml::de::Error, text: &str) -> usize {
    let Some(span) = error.span() else {
        return 0;
    };
    let end = span.start.min(text.len());
    text[..end].bytes().filter(|byte| *byte == b'\n').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(blocks: Vec<Block>) {
        let text = write_blocks(&blocks);
        let lines: Vec<&str> = text.split('\n').collect();
        let numbered: Vec<(usize, &str)> = lines
            .iter()
            .enumerate()
            .map(|(index, line)| (index + 1, *line))
            .collect();
        let parsed = parse_blocks(Path::new("entries/test.md"), &numbered).unwrap();
        assert_eq!(parsed, blocks, "source:\n{text}");
    }

    fn paragraph(text: &str) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![Inline::text(text)],
        }
    }

    #[test]
    fn every_paragraph_style_round_trips() {
        round_trip(vec![
            paragraph("normal"),
            Block::Paragraph {
                style: ParagraphStyle::StatLine,
                content: vec![Inline::text("**School** conjuration")],
            },
            Block::Paragraph {
                style: ParagraphStyle::Indented,
                content: vec![Inline::text("indented")],
            },
        ]);
    }

    #[test]
    fn a_heading_text_that_looks_like_an_anchor_stays_text() {
        round_trip(vec![Block::Heading {
            level: 2,
            anchor: None,
            content: vec![Inline::text("Effects {#not-an-anchor}")],
        }]);
    }

    #[test]
    fn a_non_breaking_space_paragraph_line_is_not_blank() {
        round_trip(vec![paragraph("\u{a0}")]);
    }

    #[test]
    fn headings_with_and_without_anchors_round_trip() {
        round_trip(vec![
            Block::Heading {
                level: 2,
                anchor: Some("extraordinary-abilities".into()),
                content: vec![Inline::text("Extraordinary Abilities")],
            },
            Block::Heading {
                level: 6,
                anchor: None,
                content: vec![Inline::text("Small")],
            },
        ]);
    }

    #[test]
    fn ordered_and_unordered_lists_with_nested_blocks_round_trip() {
        round_trip(vec![Block::List {
            ordered: true,
            items: vec![
                vec![paragraph("one")],
                vec![
                    paragraph("two"),
                    Block::List {
                        ordered: false,
                        items: vec![vec![paragraph("nested a")], vec![paragraph("nested b")]],
                    },
                    Block::Quote {
                        content: vec![paragraph("quoted in item")],
                    },
                ],
            ],
        }]);
    }

    #[test]
    fn adjacent_lists_stay_separate() {
        round_trip(vec![
            Block::List {
                ordered: false,
                items: vec![vec![paragraph("a")]],
            },
            Block::List {
                ordered: false,
                items: vec![vec![paragraph("b")]],
            },
        ]);
    }

    #[test]
    fn multi_block_quotes_round_trip() {
        round_trip(vec![Block::Quote {
            content: vec![
                paragraph("first"),
                Block::List {
                    ordered: false,
                    items: vec![vec![paragraph("inner")]],
                },
            ],
        }]);
    }

    #[test]
    fn simple_pipe_tables_round_trip() {
        let mut table = Table::default();
        table.head.push(TableRow {
            cells: vec![
                TableCell {
                    header: true,
                    colspan: 1,
                    rowspan: 1,
                    content: vec![Inline::text("Lever (d8)")],
                },
                TableCell {
                    header: true,
                    colspan: 1,
                    rowspan: 1,
                    content: vec![Inline::text("Function")],
                },
            ],
        });
        table.body.push(TableRow {
            cells: vec![
                TableCell {
                    header: false,
                    colspan: 1,
                    rowspan: 1,
                    content: vec![Inline::text("1")],
                },
                TableCell {
                    header: false,
                    colspan: 1,
                    rowspan: 1,
                    content: vec![Inline::text("a | b")],
                },
            ],
        });
        round_trip(vec![Block::Table(table)]);
    }

    #[test]
    fn fenced_tables_keep_caption_foot_header_cells_and_spans() {
        round_trip(vec![Block::Table(Table {
            caption: vec![Inline::text("Table: Special Ability Types")],
            head: vec![TableRow {
                cells: vec![TableCell {
                    header: true,
                    colspan: 2,
                    rowspan: 1,
                    content: vec![Inline::text("Header")],
                }],
            }],
            body: vec![TableRow {
                cells: vec![
                    TableCell {
                        header: true,
                        colspan: 1,
                        rowspan: 2,
                        content: vec![Inline::text("Body header")],
                    },
                    TableCell {
                        header: false,
                        colspan: 1,
                        rowspan: 1,
                        content: vec![],
                    },
                ],
            }],
            foot: vec![TableRow {
                cells: vec![TableCell {
                    header: false,
                    colspan: 2,
                    rowspan: 1,
                    content: vec![Inline::text("foot")],
                }],
            }],
        })]);
    }

    #[test]
    fn stat_blocks_with_and_without_a_title_round_trip() {
        round_trip(vec![
            Block::StatBlock {
                title: Some("Spell statistics".into()),
                rows: vec![StatRow {
                    label: "Level".into(),
                    value: vec![
                        Inline::text("2: Arcanist, Druid"),
                        Inline::Break,
                        Inline::text("3: Cleric"),
                    ],
                }],
            },
            Block::StatBlock {
                title: None,
                rows: vec![],
            },
        ]);
    }

    #[test]
    fn entry_links_round_trip() {
        round_trip(vec![Block::EntryLink {
            target: EntryId::parse("ref/spell/light").unwrap(),
            title: "Light".into(),
        }]);
    }

    #[test]
    fn leading_block_markers_inside_paragraphs_are_escaped() {
        for text in [
            "# not a heading",
            "- not a list",
            "1. not a list",
            "::: not a fence",
            "> not a quote",
            "|not a table|",
            "-",
            "+",
            "1.",
            "12.",
        ] {
            round_trip(vec![paragraph(text)]);
        }
    }

    #[test]
    fn malformed_blocks_name_a_line() {
        let parse = |source: &str| {
            let lines: Vec<(usize, &str)> = source
                .split('\n')
                .enumerate()
                .map(|(index, line)| (index + 1, line))
                .collect();
            parse_blocks(Path::new("entries/test.md"), &lines)
        };
        assert!(parse("## heading\n# bad").is_err());
        assert!(parse("::: unknown\nx\n:::").is_err());
        assert!(parse("::: table\ncaption = [\n:::").is_err());
        assert!(parse("::: table\nbody = [[{ text = \"x\", colspan = 0 }]]\n:::").is_err());
        assert!(parse("|a|b|\n| --- |\n|c|d|").is_err());
        assert!(parse("::: statblock\nno colon\n:::").is_err());
        assert!(parse("::: entry-link\ntarget = \"bad id\"\ntitle = \"x\"\n:::").is_err());
        let error = parse("::: paragraph statline\n*open\n:::").unwrap_err();
        assert!(error.line >= 1);
        let error = parse("## Effects {#bad anchor}").unwrap_err();
        assert!(error.message.contains("invalid heading anchor"), "{error}");
        assert_eq!(error.line, 1);
    }
}
