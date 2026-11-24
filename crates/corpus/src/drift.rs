//! Drift between the typed `[fields]` and the body text that states the same value.
//!
//! An entry keeps some values twice: once in `[fields]`, where search and facets read
//! them, and once in the body, where the reader sees them (a spell's `Range` row, a
//! creature's `AC` row, a feat's `**Prerequisites:**` line). The corpus keeps both
//! copies. This check compares each pair that both state and reports every
//! disagreement, naming both places, so a hand edit to one copy is noticed.

use crate::check::is_item_slot;
use crate::load::Stored;
use cast_iron_schema::headings::plain_text;
use cast_iron_schema::{
    Block, ChallengeRating, Inline, RuleEntry, StatRow, TradeOp, TypedFields, printed_trades,
    stat_sets::first_stat_set_region,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// One pair of values that disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disagreement {
    pub id: String,
    pub path: PathBuf,
    /// The entry kind slug.
    pub kind: String,
    pub field: &'static str,
    pub field_line: usize,
    pub field_value: String,
    /// `stat block row `Range`` or ``**Prerequisites:**` line`.
    pub source: String,
    pub source_line: usize,
    pub source_value: String,
    /// The field has no statement of its label in the normal part at all.
    pub missing: bool,
}

impl Disagreement {
    /// `path:line: [fields] range = "A" disagrees with the stat block row `Range`: "B" (line 36)`
    pub fn describe(&self) -> String {
        if self.missing {
            return format!(
                "{}:{}: [fields] {} = {:?} has no {} statement in the normal part",
                self.path.display(),
                self.field_line,
                self.field,
                self.field_value,
                self.source
            );
        }
        format!(
            "{}:{}: [fields] {} = {:?} disagrees with the {} = {:?} (line {})",
            self.path.display(),
            self.field_line,
            self.field,
            self.field_value,
            self.source,
            self.source_value,
            self.source_line
        )
    }
}

/// The typed fields keep the upstream Markdown (`*emphasis*`, `[label](target)`); the body
/// holds the same words as typed nodes. Reduce both to their words.
fn strip_markup(text: &str) -> String {
    let decoded = text
        .replace("&times;", "×")
        .replace("&dagger;", "†")
        .replace("&amp;", "&");
    let text = decoded.as_str();
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '*' => index += 1,
            '[' => {
                // `[label](target)` keeps the label.
                let close = chars[index..].iter().position(|ch| *ch == ']');
                match close {
                    Some(offset) if chars.get(index + offset + 1) == Some(&'(') => {
                        let label: String = chars[index + 1..index + offset].iter().collect();
                        out.push_str(&label);
                        let end = chars[index + offset..]
                            .iter()
                            .position(|ch| *ch == ')')
                            .map_or(chars.len(), |end| index + offset + end + 1);
                        index = end;
                    }
                    _ => {
                        out.push('[');
                        index += 1;
                    }
                }
            }
            ch => {
                out.push(ch);
                index += 1;
            }
        }
    }
    out
}

/// Lowercase, one space between words, no end punctuation, no Markdown markers,
/// no footnote markers.
fn normalize(text: &str) -> String {
    strip_footnotes(&strip_markup(text))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .trim_end_matches(['.', ';'])
        .to_string()
}

/// Whether a `creature_type` field agrees with the `Type` row of its stat block.
///
/// The field uses the corpus spelling (`magicalBeast`), while the body keeps the printed spelling
/// (`Magical Beast`), so case, spaces, and punctuation do not decide the comparison. The
/// Technology Guide prints `Type: Intelligence` for its sample AIs, but the book states that an
/// artificial intelligence counts as a construct for every effect that targets creature type; the
/// corpus records the type that the rules use, so `construct` agrees with that row.
fn creature_type_agrees(field: &str, source: &str) -> bool {
    let field = creature_type_key(field);
    let source = creature_type_key(source);
    field == source || (field == "construct" && source == "intelligence")
}

/// A creature type reduced to letters and digits, so `Magical Beast` and `magicalBeast` match.
fn creature_type_key(text: &str) -> String {
    normalize(text)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// A superscript is a footnote marker when it holds only a short number: `^1^`, or the
/// `^[1](…)` link the reader sees. Real superscripts such as `10^2^` keep their text.
fn is_footnote_sup(items: &[Inline]) -> bool {
    let text = plain_text(items);
    !text.is_empty() && text.len() <= 3 && text.chars().all(|ch| ch.is_ascii_digit())
}

/// Plain text with footnote superscripts dropped. A footnote marker is not part of the
/// rule text, and the field side spells it differently, so it must not decide a comparison.
fn comparison_text(items: &[Inline]) -> String {
    let mut out = String::new();
    push_comparison_text(items, &mut out);
    out
}

fn push_comparison_text(items: &[Inline], out: &mut String) {
    for item in items {
        match item {
            Inline::Text { v } => out.push_str(v),
            Inline::Break => out.push(' '),
            Inline::Sup { c } if is_footnote_sup(c) => {}
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                push_comparison_text(c, out);
            }
            Inline::Link { c, .. } => push_comparison_text(c, out),
        }
    }
}

/// Drop footnote markers: `[^P]` in a typed field, `^P^` and `^1^` as written text, and
/// the flattened form where a superscript's link has become a bare number glued to the
/// word before it (`ranks.1`, `ranks,1`).
fn strip_footnotes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        // `[^P]`, `[^X]`
        if chars[index] == '['
            && chars.get(index + 1) == Some(&'^')
            && let Some(offset) = chars[index..].iter().position(|ch| *ch == ']')
        {
            index += offset + 1;
            continue;
        }
        // `^1^`, `^P^`
        if chars[index] == '^'
            && let Some(offset) = chars[index + 1..].iter().position(|ch| *ch == '^')
        {
            let marker: String = chars[index + 1..index + 1 + offset].iter().collect();
            if !marker.is_empty() && marker.len() <= 3 && !marker.contains(char::is_whitespace) {
                index += offset + 2;
                continue;
            }
        }
        // `ranks.1`, `ranks,1`: a flattened marker glued to the word before it. A comma or
        // period inside a number (`1,000`) follows a digit, so it is left alone.
        if matches!(chars[index], '.' | ',') && index > 0 && chars[index - 1].is_alphabetic() {
            let mut end = index + 1;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            let ends_marker = chars
                .get(end)
                .is_none_or(|ch| ch.is_whitespace() || matches!(ch, '.' | ',' | ';' | ')' | ']'));
            if end > index + 1 && ends_marker {
                index = end;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

fn split_on_break(inlines: &[Inline]) -> Vec<&[Inline]> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (index, inline) in inlines.iter().enumerate() {
        if matches!(inline, Inline::Break) {
            parts.push(&inlines[start..index]);
            start = index + 1;
        }
    }
    parts.push(&inlines[start..]);
    parts
}

fn stat_rows(body: &[Block]) -> Vec<&StatRow> {
    body.iter()
        .filter_map(|block| match block {
            Block::StatBlock { rows, .. } => Some(rows),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The text after each bold `**Label:**` in the top-level paragraphs, up to the next break,
/// in document order. The normal part of a feat may state the same label more than once,
/// and a sidebar or variant list may carry the statement inside a quote.
fn labelled_texts(body: &[Block], label: &str) -> Vec<String> {
    let wanted = format!("{label}:");
    let mut found = Vec::new();
    collect_labelled(body, &wanted, &mut found);
    found
}

fn collect_labelled(body: &[Block], wanted: &str, found: &mut Vec<String>) {
    for block in body {
        if let Block::Quote { content } = block {
            collect_labelled(content, wanted, found);
            continue;
        }
        let Block::Paragraph { content, .. } = block else {
            continue;
        };
        for (start, inline) in content.iter().enumerate() {
            if !matches!(inline, Inline::Strong { c } if plain_text(c).trim() == wanted) {
                continue;
            }
            let rest = &content[start + 1..];
            let end = rest
                .iter()
                .position(|inline| matches!(inline, Inline::Break))
                .unwrap_or(rest.len());
            found.push(comparison_text(&rest[..end]).trim().to_string());
        }
    }
}

/// Whether `block` is a paragraph whose first inline is the bold `**Sources:**` label.
pub(crate) fn is_sources_line(block: &Block) -> bool {
    let Block::Paragraph { content, .. } = block else {
        return false;
    };
    matches!(
        content.first(),
        Some(Inline::Strong { c }) if plain_text(c).trim() == "Sources:"
    )
}

/// The normal part of an entry's body: everything before the first block that starts another
/// publication's version of the entry. A base entry's `[fields]` describe the normal part, so the
/// blocks after it (a `### Mythic` or `### Stamina` block, an Elephant-in-the-Room replacement, a
/// subdomain or wildblooded mutation printed in a later book) are never compared.
///
/// The entry's own citation is not in the body: the `[[sources]]` rows carry it. Two signs start
/// another source's block:
///
/// 1. A `**Sources:**` paragraph, unless it is the first block. A first block names the entry's own
///    citation (a legacy line, or a Paizo Blog citation that no row carries); a later one names
///    another publication. A `###` heading directly above such a paragraph starts the block at
///    the heading.
/// 2. A `###` heading, when the entry prints in more than one publication (`source_ids`; every
///    `[[sources]]` row names one of them). A second publication can print its own `###` section,
///    and the data no longer keeps a `**Sources:**` paragraph under it, because the rows already
///    cite the page. An entry of one publication has no other source, so its `###` headings
///    organize the normal part (options, examples, a table) and stay in the comparison.
fn normal_part(entry: &RuleEntry) -> &[Block] {
    split_normal_part(entry).0
}

/// Whether the normal part of `entry` ends at a `###` heading that only the entry's second
/// publication marks: no `**Sources:**` paragraph, before the heading or directly below it, says
/// that another source's block starts there (the second branch of [`normal_part`]). Lines below
/// such a heading are not compared, so a `###` that organizes the entry's own rules text would
/// hide them; the check lists these entries as information (#642). Only the kinds that compare
/// the normal part (a feat, an archetype, a class option) are asked: the others read the whole
/// body. Returns the 1-based line of the heading. The result of [`find_drift`] does not depend
/// on it.
pub fn unmarked_heading_cut(stored: &Stored) -> Option<usize> {
    let entry = &stored.entry;
    if !matches!(
        entry.fields,
        TypedFields::Feat(_) | TypedFields::Archetype(_) | TypedFields::ClassOption(_)
    ) {
        return None;
    }
    let (normal, unmarked) = split_normal_part(entry);
    unmarked.then(|| stored.block_lines.get(normal.len()).copied().unwrap_or(1))
}

/// The normal part, and whether it ends at a heading that only `source_ids` marks.
fn split_normal_part(entry: &RuleEntry) -> (&[Block], bool) {
    let body = &entry.body;
    let other_sources = entry.source_ids.len() > 1;
    for (index, block) in body.iter().enumerate() {
        if index > 0 && is_sources_line(block) {
            return (&body[..index], false);
        }
        if matches!(block, Block::Heading { level: 3, .. }) {
            if body.get(index + 1).is_some_and(is_sources_line) {
                return (&body[..index], false);
            }
            if other_sources {
                return (&body[..index], true);
            }
        }
    }
    (body, false)
}

/// The 1-based line of the first line after the front matter that satisfies `matches`.
fn body_line(text: &str, matches: impl Fn(&str) -> bool) -> usize {
    let mut lines = text.lines().enumerate();
    // Skip the front matter: two `+++` lines.
    let mut fences = 0;
    for (index, line) in lines.by_ref() {
        if line.trim_end() == "+++" {
            fences += 1;
            if fences == 2 {
                let _ = index;
                break;
            }
        }
    }
    for (index, line) in lines {
        if matches(line) {
            return index + 1;
        }
    }
    1
}

fn field_line(text: &str, field: &str) -> usize {
    let prefix = format!("{field} = ");
    // A corpus-format-2 entry holds `replaces` and `alters` as keys of the rows of one `trades` list.
    let row = format!("  {{ {field} = ");
    text.lines()
        .position(|line| line.starts_with(&prefix) || line.starts_with(&row))
        .map_or(1, |index| index + 1)
}

/// One field-to-row pair with the text each side states.
struct Pair {
    field: &'static str,
    field_value: String,
    source: String,
    source_line: usize,
    source_value: String,
    agree: bool,
    missing: bool,
}

fn row_pair(
    text: &str,
    rows: &[&StatRow],
    field: &'static str,
    value: &str,
    labels: &[&str],
) -> Option<Pair> {
    let row = rows.iter().find(|row| {
        labels
            .iter()
            .any(|label| row.label.eq_ignore_ascii_case(label))
    })?;
    let source_value = plain_text(&row.value).trim().to_string();
    let label = row.label.clone();
    let prefix = format!("{}:", label.to_lowercase());
    Some(Pair {
        field,
        field_value: value.to_string(),
        source: format!("stat block row `{label}`"),
        source_line: body_line(text, |line| line.to_lowercase().starts_with(&prefix)),
        agree: normalize(value) == normalize(&source_value),
        source_value,
        missing: false,
    })
}

fn text_pair(
    text: &str,
    body: &[Block],
    field: &'static str,
    value: &str,
    label: &str,
    first_only: bool,
) -> Option<Pair> {
    let stated = labelled_texts(body, label);
    let source_value = stated.first()?.clone();
    let marker = format!("**{label}:**");
    // A normal feat may state its label more than once in its own part (a variant
    // statement), so the field agrees when any of those statements matches. With
    // `first_only` only the first statement counts: a feat has one prerequisites line
    // of its own, and a later one belongs to a sub-option.
    let agree = if first_only {
        normalize(value) == normalize(&source_value)
    } else {
        stated
            .iter()
            .any(|statement| normalize(value) == normalize(statement))
    };
    Some(Pair {
        field,
        field_value: value.to_string(),
        source: format!("`**{label}:**` line"),
        source_line: body_line(text, |line| line.contains(&marker)),
        agree,
        source_value,
        missing: false,
    })
}

/// A creature's `cr` field against its `**Challenge rating:**` line. Most creatures
/// state the rating only this way, without a stat block `CR` row. Fractions compare
/// by parsed value, so `1/2` agrees with `1/2` in either form.
fn challenge_rating_pair(
    text: &str,
    body: &[Block],
    field: &'static str,
    value: ChallengeRating,
) -> Option<Pair> {
    let stated = labelled_texts(body, "Challenge rating");
    let source_value = stated.first()?.clone();
    let marker = "**Challenge rating:**";
    let agree = stated
        .iter()
        .any(|statement| challenge_rating_agrees(statement, value));
    Some(Pair {
        field,
        field_value: value.display(),
        source: "`**Challenge rating:**` line".into(),
        source_line: body_line(text, |line| line.contains(marker)),
        source_value,
        agree,
        missing: false,
    })
}

fn joined(values: &[String]) -> String {
    values.join(", ")
}

/// `(class, level)` pairs of a spell's `Level` row: lines like `2: Arcanist, Druid`.
fn parse_levels(value: &[Inline]) -> BTreeSet<(String, u8)> {
    let mut found = BTreeSet::new();
    for line in split_on_break(value) {
        let line = plain_text(line);
        let Some((level, classes)) = line.split_once(':') else {
            continue;
        };
        let Ok(level) = level.trim().parse::<u8>() else {
            continue;
        };
        for class in classes.split(',') {
            let class = normalize(class);
            if !class.is_empty() {
                found.insert((class, level));
            }
        }
    }
    found
}

fn describe_levels(levels: &BTreeSet<(String, u8)>) -> String {
    let mut by_level: std::collections::BTreeMap<u8, Vec<&str>> = Default::default();
    for (class, level) in levels {
        by_level.entry(*level).or_default().push(class);
    }
    by_level
        .iter()
        .map(|(level, classes)| format!("{level}: {}", classes.join(", ")))
        .collect::<Vec<_>>()
        .join("; ")
}

const SPELL_OTHER_ROWS: &[&str] = &[
    "School",
    "Subschool",
    "Descriptors",
    "Level",
    "Casting time",
    "Components",
    "Range",
    "Duration",
    "Saving throw",
    "Spell resistance",
];

fn spell_pairs(entry: &RuleEntry, text: &str, rows: &[&StatRow]) -> Vec<Pair> {
    let TypedFields::Spell(fields) = &entry.fields else {
        return vec![];
    };
    let mut pairs = Vec::new();
    if let Some(mut pair) = row_pair(text, rows, "school", &fields.school, &["School"]) {
        // The row adds the subschool and descriptors: `conjuration (calling) [curse]`.
        let school = normalize(&fields.school);
        let stated = normalize(&pair.source_value);
        pair.agree = pair.agree
            || stated.starts_with(&format!("{school} ("))
            || stated.starts_with(&format!("{school} ["));
        pairs.push(pair);
    }
    if !fields.subschools.is_empty() {
        pairs.extend(row_pair(
            text,
            rows,
            "subschools",
            &joined(&fields.subschools),
            &["Subschool"],
        ));
    }
    if !fields.descriptors.is_empty() {
        pairs.extend(row_pair(
            text,
            rows,
            "descriptors",
            &joined(&fields.descriptors),
            &["Descriptors"],
        ));
    }
    if let Some(row) = rows
        .iter()
        .find(|row| row.label.eq_ignore_ascii_case("Level"))
        && !fields.levels.is_empty()
    {
        let stated = parse_levels(&row.value);
        let typed: BTreeSet<(String, u8)> = fields
            .levels
            .iter()
            .map(|level| (normalize(&level.class), level.level))
            .collect();
        pairs.push(Pair {
            field: "levels",
            field_value: describe_levels(&typed),
            source: "stat block row `Level`".into(),
            source_line: body_line(text, |line| line.to_lowercase().starts_with("level:")),
            source_value: describe_levels(&stated),
            agree: typed == stated,
            missing: false,
        });
    }
    for (field, value, labels) in [
        ("casting_time", &fields.casting_time, &["Casting time"][..]),
        ("components", &fields.components, &["Components"][..]),
        ("range", &fields.range, &["Range"][..]),
        ("duration", &fields.duration, &["Duration"][..]),
        ("saving_throw", &fields.saving_throw, &["Saving throw"][..]),
        (
            "spell_resistance",
            &fields.spell_resistance,
            &["Spell resistance"][..],
        ),
    ] {
        if let Some(value) = value {
            pairs.extend(row_pair(text, rows, field, value, labels));
        }
    }
    if !fields.effects.is_empty() {
        let stated: BTreeSet<(String, String)> = rows
            .iter()
            .filter(|row| {
                !SPELL_OTHER_ROWS
                    .iter()
                    .any(|other| row.label.eq_ignore_ascii_case(other))
            })
            .map(|row| (normalize(&row.label), normalize(&plain_text(&row.value))))
            .collect();
        if !stated.is_empty() {
            let typed: BTreeSet<(String, String)> = fields
                .effects
                .iter()
                .map(|(label, value)| (normalize(label), normalize(value)))
                .collect();
            let show = |set: &BTreeSet<(String, String)>| {
                set.iter()
                    .map(|(label, value)| format!("{label}: {value}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            let first = rows
                .iter()
                .find(|row| {
                    !SPELL_OTHER_ROWS
                        .iter()
                        .any(|other| row.label.eq_ignore_ascii_case(other))
                })
                .map(|row| row.label.to_lowercase())
                .unwrap_or_default();
            pairs.push(Pair {
                field: "effects",
                field_value: show(&typed),
                source: "stat block rows `Target`, `Effect`, `Area`".into(),
                source_line: body_line(text, |line| {
                    line.to_lowercase().starts_with(&format!("{first}:"))
                }),
                source_value: show(&stated),
                agree: typed == stated,
                missing: false,
            });
        }
    }
    pairs
}

/// The six ability scores of an `Ability Scores` row: `Str 3, Dex 16, ...`. Each item is what
/// follows the ability name, so `Str - (22 while corporeal)` gives `- (22 while corporeal)`, the
/// value that the typed field keeps.
fn parse_abilities(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| {
            let part = part.trim();
            let (name, rest) = part.split_once(char::is_whitespace).unwrap_or((part, ""));
            if ABILITY_NAMES
                .iter()
                .any(|ability| name.eq_ignore_ascii_case(ability))
            {
                rest.trim().to_string()
            } else {
                part.to_string()
            }
        })
        .collect()
}

const ABILITY_NAMES: [&str; 6] = ["Str", "Dex", "Con", "Int", "Wis", "Cha"];

/// Whether a printed challenge rating states `value`. Fractions compare by parsed value.
fn challenge_rating_agrees(statement: &str, value: ChallengeRating) -> bool {
    match ChallengeRating::parse(statement) {
        Some(parsed) => parsed == value,
        None => normalize(&value.display()) == normalize(statement),
    }
}

/// The text of the first stat block row with `label`, if any.
fn row_text(rows: &[&StatRow], label: &str) -> Option<String> {
    rows.iter()
        .find(|row| row.label.eq_ignore_ascii_case(label))
        .map(|row| plain_text(&row.value).trim().to_string())
}

/// The `Subtype` row of a stat block against `[fields] subtypes`, as sets of words: the row
/// is `Chaotic, Demon, Evil, Extraplanar` and the field `chaotic, demon, evil, extraplanar`.
/// Pf Data prints the type alone in `Type` and the subtypes in this row.
fn subtype_pair(text: &str, rows: &[&StatRow], subtypes: &[String]) -> Option<Pair> {
    let stated = row_text(rows, "Subtype")?;
    if subtypes.is_empty() {
        return None;
    }
    let set = |items: &[&str]| -> BTreeSet<String> {
        items
            .iter()
            .flat_map(|item| item.split(','))
            .map(|item| creature_type_key(item.trim().trim_start_matches("or ")))
            .filter(|item| !item.is_empty())
            .collect()
    };
    let typed: Vec<&str> = subtypes.iter().map(String::as_str).collect();
    Some(Pair {
        field: "subtypes",
        field_value: joined(subtypes),
        source: "stat block row `Subtype`".into(),
        source_line: body_line(text, |line| line.to_lowercase().starts_with("subtype:")),
        agree: set(&typed) == set(&[stated.as_str()]),
        source_value: stated,
        missing: false,
    })
}

/// `[fields] ac` (`20/10/19`) against the `Armor class`, `Touch` and `Flat-footed` rows.
fn armor_class_pair(text: &str, rows: &[&StatRow], ac: &str) -> Option<Pair> {
    let (armor, touch, flat) = (
        row_text(rows, "Armor class")?,
        row_text(rows, "Touch")?,
        row_text(rows, "Flat-footed")?,
    );
    let stated = format!("{armor}/{touch}/{flat}");
    Some(Pair {
        field: "ac",
        field_value: ac.to_string(),
        source: "stat block rows `Armor class`, `Touch`, `Flat-footed`".into(),
        source_line: body_line(text, |line| line.to_lowercase().starts_with("armor class:")),
        agree: normalize(ac) == normalize(&stated),
        source_value: stated,
        missing: false,
    })
}

/// `[fields] hp` against the `hp` or `Hit points` row. The field writes `103~9d12+45` where
/// print writes `103 (9d12+45)`.
fn hit_points_pair(text: &str, rows: &[&StatRow], hp: &str) -> Option<Pair> {
    let printed = match hp.split_once('~') {
        Some((total, dice)) => format!("{total} ({dice})"),
        None => hp.to_string(),
    };
    let mut pair = row_pair(text, rows, "hp", &printed, &["hp", "Hit points"])?;
    pair.field_value = hp.to_string();
    pair.agree = pair.agree || normalize(hp) == normalize(&pair.source_value);
    Some(pair)
}

/// `body` is the part that `[fields]` describe: the whole body, or with several stat sets the
/// blocks before the second set's heading (see [`first_stat_set_region`]). A later set prints
/// other values by design and never drifts against the entry's `[fields]`.
fn creature_pairs(entry: &RuleEntry, text: &str, body: &[Block], rows: &[&StatRow]) -> Vec<Pair> {
    let TypedFields::Creature(fields) = &entry.fields else {
        return vec![];
    };
    let mut pairs = Vec::new();
    if let Some(cr) = fields.cr {
        pairs.extend(row_pair(text, rows, "cr", &cr.display(), &["CR"]));
        // The Pf Data `Challenge rating` row compares by value, like the paragraph line.
        if let Some(mut pair) = row_pair(text, rows, "cr", &cr.display(), &["Challenge rating"]) {
            pair.agree = challenge_rating_agrees(&pair.source_value, cr);
            pairs.push(pair);
        }
        pairs.extend(challenge_rating_pair(text, body, "cr", cr));
    }
    if let Some(hp) = &fields.hp {
        pairs.extend(hit_points_pair(text, rows, hp));
    }
    for (field, value, labels) in [
        ("size", &fields.size, &["Size"][..]),
        ("alignment", &fields.alignment, &["Alignment"][..]),
        ("ac", &fields.ac, &["AC"][..]),
        ("fortitude", &fields.fortitude, &["Fort"][..]),
        ("reflex", &fields.reflex, &["Ref"][..]),
        ("will", &fields.will, &["Will"][..]),
        ("melee", &fields.melee, &["Melee"][..]),
        ("ranged", &fields.ranged, &["Ranged"][..]),
        (
            "special_attacks",
            &fields.special_attacks,
            &["Special Attacks"][..],
        ),
        (
            "special_qualities",
            &fields.special_qualities,
            &["SQ", "Special Qualities"][..],
        ),
    ] {
        if let Some(value) = value {
            pairs.extend(row_pair(text, rows, field, value, labels));
        }
    }
    if let Some(ac) = &fields.ac {
        pairs.extend(armor_class_pair(text, rows, ac));
    }
    pairs.extend(subtype_pair(text, rows, &fields.subtypes));
    if let Some(kind) = &fields.creature_type {
        // A stat block with a `Subtype` row prints the type alone in `Type`. Without that row,
        // the `Type` row may add the subtypes in parentheses.
        let with_subtypes = if fields.subtypes.is_empty() || row_text(rows, "Subtype").is_some() {
            kind.clone()
        } else {
            format!("{kind} ({})", joined(&fields.subtypes))
        };
        if let Some(mut pair) = row_pair(text, rows, "creature_type", &with_subtypes, &["Type"]) {
            pair.agree = pair.agree || creature_type_agrees(&with_subtypes, &pair.source_value);
            pairs.push(pair);
        }
    }
    if let Some(abilities) = &fields.abilities
        && let Some(row) = rows
            .iter()
            .find(|row| row.label.eq_ignore_ascii_case("Ability Scores"))
    {
        let stated_text = plain_text(&row.value);
        let stated = parse_abilities(&stated_text);
        let typed: Vec<String> = abilities.to_vec();
        pairs.push(Pair {
            field: "abilities",
            field_value: typed.join(", "),
            source: "stat block row `Ability Scores`".into(),
            source_line: body_line(text, |line| {
                line.to_lowercase().starts_with("ability scores:")
            }),
            source_value: stated_text.trim().to_string(),
            agree: typed
                .iter()
                .map(|value| normalize(value))
                .eq(stated.iter().map(|value| normalize(value))),
            missing: false,
        });
    }
    pairs
}

/// The variant spellings that print gives a slot and that the corpus maps to a closed `slot`
/// value in the field only (decision 2026-11-22, affliction types and item slots are closed
/// vocabularies). The body keeps the printed spelling, so the drift check maps it the same way.
const SLOT_SPELLINGS: [(&str, &str); 7] = [
    ("-", "none"),
    ("amulet", "neck"),
    ("cloak", "shoulders"),
    ("eye", "eyes"),
    ("held", "none"),
    ("shoulder", "shoulders"),
    ("waist", "belt"),
];

/// A printed `Slot` row reduced to the closed value it states: the text up to the first `;` (a
/// price or weight that the row repeats), without a trailing qualifier in parentheses (`(see
/// text)`, `(replaces hand)`, `(barding)`), with each variant spelling mapped. A value that this
/// leaves unchanged is returned as it is, so an open value compares by its own text.
fn slot_key(printed: &str) -> String {
    let printed = normalize(printed);
    let printed = printed.split(';').next().unwrap_or_default().trim();
    // A closed value that holds parentheses itself (`arms (cybernetic)`) is whole.
    let printed = match printed.rfind(" (") {
        Some(open) if printed.ends_with(')') && !is_item_slot(printed) => {
            printed[..open].trim_end()
        }
        _ => printed,
    };
    printed
        .split(" or ")
        .map(|part| {
            SLOT_SPELLINGS
                .iter()
                .find(|(spelling, _)| *spelling == part)
                .map_or(part, |(_, slot)| slot)
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

/// `[fields] slot` against the `Slot` row. The field holds the closed value and the row the
/// printed one, so the row agrees when it states the field itself or a variant spelling that
/// maps to it.
fn slot_pair(text: &str, rows: &[&StatRow], slot: &str) -> Option<Pair> {
    let mut pair = row_pair(text, rows, "slot", slot, &["Slot"])?;
    pair.agree = pair.agree || slot_key(&pair.source_value) == normalize(slot);
    Some(pair)
}

fn item_pairs(entry: &RuleEntry, text: &str, rows: &[&StatRow]) -> Vec<Pair> {
    let TypedFields::Item(fields) = &entry.fields else {
        return vec![];
    };
    let mut pairs = Vec::new();
    for (field, value, labels) in [
        ("price", &fields.price, &["Price"][..]),
        ("weight", &fields.weight, &["Weight"][..]),
        ("aura", &fields.aura, &["Aura"][..]),
        ("caster_level", &fields.caster_level, &["CL"][..]),
    ] {
        if let Some(value) = value {
            pairs.extend(row_pair(text, rows, field, value, labels));
        }
    }
    if let Some(slot) = &fields.slot {
        pairs.extend(slot_pair(text, rows, slot));
    }
    pairs
}

fn feat_pairs(entry: &RuleEntry, text: &str) -> Vec<Pair> {
    let TypedFields::Feat(fields) = &entry.fields else {
        return vec![];
    };
    // A base feat's fields describe the normal feat, never a source subsection.
    let body = normal_part(entry);
    let mut pairs = Vec::new();
    for (field, value, label) in [
        ("prerequisites", &fields.prerequisites, "Prerequisites"),
        ("benefit", &fields.benefit, "Benefit"),
        ("normal", &fields.normal, "Normal"),
        ("special", &fields.special, "Special"),
    ] {
        if let Some(value) = value {
            match text_pair(text, body, field, value, label, label == "Prerequisites") {
                Some(pair) => pairs.push(pair),
                // The normal part states no such label, so nothing compares the field.
                // Report it when another block in the body does state the label: the
                // field can only have come from there. A body that never uses the label
                // (the carried heading-and-statline style) is outside this check.
                None if !labelled_texts(&entry.body, label).is_empty() => pairs.push(Pair {
                    field,
                    field_value: value.to_string(),
                    source: format!("`**{label}:**`"),
                    source_line: 1,
                    source_value: String::new(),
                    agree: false,
                    missing: true,
                }),
                None => {}
            }
        }
    }
    pairs
}

/// The feat labels whose field is meant to hold a printed line but whose typed field is empty:
/// a normal part that prints `**Prerequisites:**` or `**Special:**` with no field for it.
/// `feat_pairs` compares only the fields that exist, so these entries escape the drift check.
/// Each item is the field name and the line of the printed label. It is a warning, not drift,
/// until the corpus has none (phynics/cast-iron-data#190).
pub fn unfielded_feat_labels(stored: &Stored) -> Vec<(&'static str, usize)> {
    let TypedFields::Feat(fields) = &stored.entry.fields else {
        return vec![];
    };
    let body = normal_part(&stored.entry);
    let mut found = Vec::new();
    for (field, value, label) in [
        ("prerequisites", &fields.prerequisites, "Prerequisites"),
        ("special", &fields.special, "Special"),
    ] {
        if value.is_some() || labelled_texts(body, label).is_empty() {
            continue;
        }
        let marker = format!("**{label}:**");
        found.push((
            field,
            body_line(&stored.text, |line| line.contains(&marker)),
        ));
    }
    found
}

/// The items of every printed `**Replaces:**` or `**Alters:**` line in the normal part, in
/// body order. One line may state several comma-separated items, and an entry may print
/// several lines (oaths and medium spirits trade one power at a time), so the typed list is
/// the concatenation of them all.
fn printed_items(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .flat_map(|line| line.split(','))
        // Each line may end in its own period or semicolon, so strip the punctuation
        // per item rather than from the joined text.
        .map(|item| item.trim().trim_end_matches(['.', ';']).trim_end())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// The printed `**Replaces:**` and `**Alters:**` lines against the typed item lists, in print
/// order. A typed list and its printed lines must state the same items; one side alone is
/// drift.
fn replaces_alters_pairs(
    entry: &RuleEntry,
    text: &str,
    replaces: &[String],
    alters: &[String],
) -> Vec<Pair> {
    let body = normal_part(entry);
    let mut pairs = Vec::new();
    for (field, values, label) in [
        ("replaces", replaces, "Replaces"),
        ("alters", alters, "Alters"),
    ] {
        let stated = printed_items(&labelled_texts(body, label));
        if values.is_empty() && stated.is_empty() {
            continue;
        }
        let field_value = joined(values);
        let source_value = joined(&stated);
        let agree = !values.is_empty()
            && !stated.is_empty()
            && normalize(&field_value) == normalize(&source_value);
        let marker = format!("**{label}:**");
        pairs.push(Pair {
            field,
            field_value,
            source: format!("`**{label}:**` line"),
            source_line: body_line(text, |line| line.contains(&marker)),
            source_value,
            agree,
            missing: values.is_empty(),
        });
    }
    pairs
}

/// The printed `**Replaces:**` and `**Alters:**` names of the normal part, in body order.
/// Each list is the concatenation of every such line in the entry.
pub fn printed_replaces_alters(entry: &RuleEntry) -> (Vec<String>, Vec<String>) {
    let body = normal_part(entry);
    (
        printed_items(&labelled_texts(body, "Replaces")),
        printed_items(&labelled_texts(body, "Alters")),
    )
}

/// The `**Replaces:**` and `**Alters:**` names printed anywhere in the body, variant and source
/// subsections included. [`printed_replaces_alters`] reads only the normal part, which is what
/// the typed trades are compared with; a domain page prints the lines of its subdomains below
/// the normal part, and this reads those too.
pub fn printed_replaces_alters_anywhere(entry: &RuleEntry) -> (Vec<String>, Vec<String>) {
    (
        printed_items(&labelled_texts(&entry.body, "Replaces")),
        printed_items(&labelled_texts(&entry.body, "Alters")),
    )
}

/// `replaces: bravery; alters: armor training`
fn describe_trades<'a>(items: impl Iterator<Item = (TradeOp, &'a str)>) -> String {
    let items: Vec<String> = items
        .map(|(op, name)| format!("{}: {name}", op.slug()))
        .collect();
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join("; ")
    }
}

/// A trade sentence is covered when the trades of its op, together, state the whole phrase.
fn sentence_uncovered(
    sentence: &crate::trade_sentences::TradeSentence,
    trades: &[&cast_iron_schema::Trade],
) -> bool {
    use crate::trade_sentences::{name_words, phrase_is_covered};
    let names: Vec<Vec<String>> = trades
        .iter()
        .filter(|trade| trade.op == sentence.op)
        .map(|trade| name_words(&trade.printed))
        .collect();
    !phrase_is_covered(&sentence.phrase, &names)
}

/// A trade is stated when a sentence of its op holds its name.
fn trade_stated(
    trade: &cast_iron_schema::Trade,
    sentences: &[crate::trade_sentences::TradeSentence],
) -> bool {
    use crate::trade_sentences::{name_in_phrase, name_words};
    let name = name_words(&trade.printed);
    sentences
        .iter()
        .any(|sentence| sentence.op == trade.op && name_in_phrase(&name, &sentence.phrase))
}

fn sentence_list(sentences: &[crate::trade_sentences::TradeSentence]) -> String {
    describe_trades(
        sentences
            .iter()
            .map(|sentence| (sentence.op, sentence.phrase.as_str())),
    )
}

fn trade_list(trades: &[&cast_iron_schema::Trade]) -> String {
    describe_trades(
        trades
            .iter()
            .map(|trade| (trade.op, trade.printed.as_str())),
    )
}

/// The trades against the trade sentences of one place (`blocks`): every sentence is covered by
/// `trades`, and every trade is stated by a sentence. `place` names the place for the report.
/// One pair says whether the sentences are covered; each unstated trade is a pair of its own.
fn sentence_pairs(
    trades: &[&cast_iron_schema::Trade],
    sentences: &[crate::trade_sentences::TradeSentence],
    place: &str,
    source_line: usize,
    needs_no_sentence: impl Fn(&cast_iron_schema::Trade) -> bool,
) -> Vec<Pair> {
    if trades.is_empty() && sentences.is_empty() {
        return Vec::new();
    }
    let uncovered = sentences
        .iter()
        .any(|sentence| sentence_uncovered(sentence, trades));
    let mut pairs = vec![Pair {
        field: "trades",
        field_value: trade_list(trades),
        source: format!("trade sentences of {place}"),
        source_line,
        source_value: sentence_list(sentences),
        agree: !uncovered,
        missing: false,
    }];
    for trade in trades {
        if !needs_no_sentence(trade) && !trade_stated(trade, sentences) {
            pairs.push(Pair {
                field: "trades",
                field_value: trade_list(&[trade]),
                source: format!("trade sentences of {place}"),
                source_line,
                source_value: sentence_list(sentences),
                agree: false,
                missing: false,
            });
        }
    }
    pairs
}

/// The trades of an entry with `features` (corpus format 4) against the trade sentences of the
/// feature sections that state them.
///
/// - Each feature with trade sentences or trades is one pair: every trade sentence of the section
///   is covered by the trades that name the feature in `by` (a trade may name one item of the
///   sentence or its whole phrase).
/// - Each trade is one more pair when no section it names prints it: the trade's name must appear
///   in a trade sentence, with the same op, of at least one feature that it lists in `by`. A
///   trade may list several features, so a trade that two features print is not read twice.
///   Exception (owner ruling of 2026-11-24, #540): a trade whose `by` names a listed feature
///   that prints no trade sentence at all is not drift. Print then only describes the changed
///   ability ("A druid archetype's Wild Shape ..."), and `by` names the feature that prints it.
///   Coverage is unchanged: every trade sentence that is printed must still be covered by trades.
/// - A trade without `by` is a trade that the book prints outside any feature: in the opening
///   text before the first feature, or (owner ruling 13 of 2026-11-24, #540) under a scaffold
///   heading such as "Bonus Feats" wherever it sits, unless a listed feature contains it. Its name
///   must appear in a trade sentence of one of those places, and every trade sentence of those
///   places is covered by those trades.
/// - A trade that names a feature that is not listed is its own pair, so it cannot hide from the
///   comparison.
///
/// The sections are read from the whole body, cut only where another publication's version starts
/// (see [`crate::check_features::feature_part`]). The `**Replaces:**` and `**Alters:**` rows are
/// not read: this form has none.
fn feature_trade_pairs(entry: &RuleEntry, text: &str) -> Vec<Pair> {
    use crate::check_features::{
        feature_part, proficiency_section_prints_no, sections, sentences_in, unbound_sentences,
    };
    use crate::trade_sentences::TradeSentence;
    let found = sections(feature_part(entry), &entry.body);
    let features = entry.fields.features();
    let trades = entry.fields.trades();
    let sentences_of = |anchor: &str| -> Vec<TradeSentence> {
        found
            .iter()
            .find(|section| section.key == anchor)
            .map(|section| match section.heading {
                Block::Heading { content, .. } => {
                    sentences_in(section.blocks, trades, Some(&plain_text(content)))
                }
                _ => sentences_in(section.blocks, trades, None),
            })
            .unwrap_or_default()
    };
    let listed = |anchor: &str| features.iter().any(|feature| feature.anchor == anchor);
    let mut pairs = Vec::new();
    for feature in features {
        let sentences = sentences_of(&feature.anchor);
        let mine: Vec<_> = trades
            .iter()
            .filter(|trade| trade.by.contains(&feature.anchor))
            .collect();
        if sentences.is_empty() && mine.is_empty() {
            continue;
        }
        let marker = format!("{{#{}}}", feature.anchor);
        let uncovered = sentences
            .iter()
            .any(|sentence| sentence_uncovered(sentence, &mine));
        pairs.push(Pair {
            field: "trades",
            field_value: trade_list(&mine),
            source: format!("trade sentences of the feature `{}`", feature.anchor),
            source_line: body_line(text, |line| line.contains(&marker)),
            source_value: sentence_list(&sentences),
            agree: !uncovered,
            missing: false,
        });
    }
    // Trades that the book prints outside any feature (opening text, scaffold sections).
    let outside_sentences = unbound_sentences(entry);
    let outside_trades: Vec<_> = trades.iter().filter(|trade| trade.by.is_empty()).collect();
    // Owner ruling 23 of 2026-11-24 (#540): an `alters` trade without `by` may rest on a
    // "Weapon and Armor Proficiency" section that prints no `alters` sentence. It is not drift
    // that print does not state it; the sentences still must be covered by the trades.
    let proficiency = proficiency_section_prints_no(entry, TradeOp::Alters);
    pairs.extend(sentence_pairs(
        &outside_trades,
        &outside_sentences,
        "the text outside any feature",
        field_line(text, "trades"),
        |trade| proficiency && trade.op == TradeOp::Alters,
    ));
    for trade in trades.iter().filter(|trade| !trade.by.is_empty()) {
        let named: Vec<&String> = trade.by.iter().filter(|by| listed(by)).collect();
        let unlisted: Vec<&String> = trade.by.iter().filter(|by| !listed(by)).collect();
        if !unlisted.is_empty() {
            pairs.push(Pair {
                field: "trades",
                field_value: trade_list(&[trade]),
                source: "`by` of the trade".to_string(),
                source_line: field_line(text, "trades"),
                source_value: format!(
                    "{}, not a feature",
                    unlisted
                        .iter()
                        .map(|by| format!("`{by}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                agree: false,
                missing: false,
            });
            continue;
        }
        let all: Vec<TradeSentence> = named.iter().flat_map(|by| sentences_of(by)).collect();
        // Ruling A, per operation (owner ruling 24 of 2026-11-24): a named feature that prints no
        // sentence of the trade's own op may carry the trade unstated.
        let prints_none = named.iter().any(|by| {
            sentences_of(by)
                .iter()
                .all(|sentence| sentence.op != trade.op)
        });
        if !trade_stated(trade, &all) && !prints_none {
            pairs.push(Pair {
                field: "trades",
                field_value: trade_list(&[trade]),
                source: format!(
                    "trade sentences of the feature{} {}",
                    if named.len() == 1 { "" } else { "s" },
                    named
                        .iter()
                        .map(|by| format!("`{by}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                source_line: body_line(text, |line| line.contains(&format!("{{#{}}}", named[0]))),
                source_value: sentence_list(&all),
                agree: false,
                missing: false,
            });
        }
    }
    pairs
}

/// The trades of an archetype or class option without `features`, when the normal part prints
/// them as trade sentences ("This ability replaces detect evil.") and holds no `**Replaces:**`
/// or `**Alters:**` row. Both forms are accepted while the corpus moves from rows to prose: the
/// caller uses the row comparison whenever a row is printed, or when no sentence is. The same
/// coverage rules as for features apply, to the sentences anywhere in the normal part.
fn printed_sentence_pairs(entry: &RuleEntry, text: &str) -> Option<Vec<Pair>> {
    use crate::check_features::sentences_in;
    let trades: Vec<_> = entry.fields.trades().iter().collect();
    if trades.is_empty() {
        return None;
    }
    let body = normal_part(entry);
    if !labelled_texts(body, "Replaces").is_empty() || !labelled_texts(body, "Alters").is_empty() {
        return None;
    }
    let sentences = sentences_in(body, entry.fields.trades(), None);
    if sentences.is_empty() {
        return None;
    }
    Some(sentence_pairs(
        &trades,
        &sentences,
        "the normal part",
        field_line(text, "trades"),
        |_| false,
    ))
}

/// Every pair in the entry that states a value in both places and disagrees.
pub fn find_drift(stored: &Stored) -> Vec<Disagreement> {
    let entry = &stored.entry;
    let rows = stat_rows(&entry.body);
    let pairs = match &entry.fields {
        TypedFields::Spell(_) => spell_pairs(entry, &stored.text, &rows),
        TypedFields::Creature(_) => {
            // `[fields]` describe the first stat set only.
            let region = first_stat_set_region(&entry.body);
            creature_pairs(entry, &stored.text, region, &stat_rows(region))
        }
        TypedFields::Item(_) => item_pairs(entry, &stored.text, &rows),
        TypedFields::Feat(_) => feat_pairs(entry, &stored.text),
        TypedFields::Archetype(_) | TypedFields::ClassOption(_)
            if !entry.fields.features().is_empty() =>
        {
            feature_trade_pairs(entry, &stored.text)
        }
        TypedFields::Archetype(_) | TypedFields::ClassOption(_) => {
            printed_sentence_pairs(entry, &stored.text).unwrap_or_else(|| {
                let trades = entry.fields.trades();
                replaces_alters_pairs(
                    entry,
                    &stored.text,
                    &printed_trades(trades, TradeOp::Replaces),
                    &printed_trades(trades, TradeOp::Alters),
                )
            })
        }
        _ => vec![],
    };
    pairs
        .into_iter()
        .filter(|pair| !pair.agree)
        .map(|pair| Disagreement {
            id: entry.id.to_string(),
            path: stored.path.clone(),
            kind: entry.kind.slug().to_string(),
            field: pair.field,
            field_line: field_line(&stored.text, pair.field),
            field_value: pair.field_value,
            source: pair.source,
            source_line: pair.source_line,
            source_value: pair.source_value,
            missing: pair.missing,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::entry::{entry_path, write_entry};
    use cast_iron_schema::{
        CreatureFields, EntryId, EntryKind, FeatFields, ItemFields, ParagraphStyle, SpellFields,
        SpellLevel,
    };

    fn stored(mut entry: RuleEntry) -> Stored {
        entry.links.clear();
        Stored {
            text: write_entry(&entry, &Config::default()),
            path: entry_path(&entry.id),
            block_lines: Vec::new(),
            entry,
        }
    }

    fn base(id: &str, kind: EntryKind, fields: TypedFields, body: Vec<Block>) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: "T".into(),
            aon: None,
            source_ids: vec!["core-rulebook".into()],
            sources: vec![],
            expanded_in: vec![],
            stat_sets: vec![],
            section: vec![],
            parent: None,
            body,
            fields,
            tags: vec![],
            race_group: None,
            links: vec![],
            provenance: vec![],
            license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
            relations: vec![],
        }
    }

    fn row(label: &str, value: &str) -> StatRow {
        StatRow {
            label: label.into(),
            value: vec![Inline::text(value)],
        }
    }

    fn statblock(rows: Vec<StatRow>) -> Block {
        Block::StatBlock { title: None, rows }
    }

    fn spell(range_field: &str, range_row: &str) -> Stored {
        stored(base(
            "ref/spell/test",
            EntryKind::Spell,
            TypedFields::Spell(SpellFields {
                school: "evocation".into(),
                levels: vec![
                    SpellLevel {
                        class: "Wizard".into(),
                        level: 3,
                    },
                    SpellLevel {
                        class: "Sorcerer".into(),
                        level: 3,
                    },
                ],
                range: Some(range_field.into()),
                effects: vec![("Target".into(), "one creature".into())],
                ..Default::default()
            }),
            vec![statblock(vec![
                row("School", "Evocation"),
                StatRow {
                    label: "Level".into(),
                    value: vec![Inline::text("3: Sorcerer, Wizard")],
                },
                row("Range", range_row),
                row("Target", "one creature."),
            ])],
        ))
    }

    #[test]
    fn matching_spell_values_do_not_drift() {
        assert!(find_drift(&spell("close", "Close")).is_empty());
    }

    #[test]
    fn a_changed_range_names_both_places() {
        let drift = find_drift(&spell("close (25 ft.)", "medium (100 ft.)"));
        assert_eq!(drift.len(), 1);
        let text = drift[0].describe();
        assert!(text.starts_with("entries/spell/test.md:"), "{text}");
        assert!(
            text.contains("[fields] range = \"close (25 ft.)\""),
            "{text}"
        );
        assert!(text.contains("stat block row `Range`"), "{text}");
        assert!(text.contains("\"medium (100 ft.)\""), "{text}");
        // The stat block row is further down the file than the field.
        assert!(drift[0].source_line > drift[0].field_line);
        assert_eq!(drift[0].kind, "spell");
    }

    #[test]
    fn spell_levels_compare_as_sets_across_break_lines() {
        let mut one = spell("close", "close");
        let Block::StatBlock { rows, .. } = &mut one.entry.body[0] else {
            panic!("stat block");
        };
        rows[1].value = vec![
            Inline::text("3: Sorcerer"),
            Inline::Break,
            Inline::text("4: Wizard"),
        ];
        let drift = find_drift(&one);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].field, "levels");
        assert!(
            drift[0].source_value.contains("4: wizard"),
            "{:?}",
            drift[0]
        );
    }

    #[test]
    fn creature_values_and_abilities_are_compared() {
        let entry = base(
            "ref/monster/test",
            EntryKind::Creature,
            TypedFields::Creature(Box::new(CreatureFields {
                ac: Some("15, touch 11".into()),
                hp: Some("10 (2d8)".into()),
                abilities: Some([
                    "10".into(),
                    "11".into(),
                    "12".into(),
                    "-".into(),
                    "8".into(),
                    "9".into(),
                ]),
                ..Default::default()
            })),
            vec![statblock(vec![
                row("AC", "16, touch 11"),
                row("hp", "10 (2d8)"),
                row(
                    "Ability Scores",
                    "Str 10, Dex 11, Con 12, Int -, Wis 8, Cha 9",
                ),
            ])],
        );
        let drift = find_drift(&stored(entry));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "ac");
    }

    #[test]
    fn an_artificial_intelligence_agrees_with_construct() {
        let entry = base(
            "ref/monster/ai",
            EntryKind::Creature,
            TypedFields::Creature(Box::new(CreatureFields {
                creature_type: Some("construct".into()),
                ..Default::default()
            })),
            vec![statblock(vec![row("Type", "Intelligence")])],
        );
        assert!(find_drift(&stored(entry)).is_empty());
    }

    #[test]
    fn a_creature_type_agrees_across_case_and_spaces() {
        let entry = base(
            "ref/monster/beast",
            EntryKind::Creature,
            TypedFields::Creature(Box::new(CreatureFields {
                creature_type: Some("monstrousHumanoid".into()),
                subtypes: vec!["mythic".into()],
                ..Default::default()
            })),
            vec![statblock(vec![row("Type", "Monstrous Humanoid (mythic)")])],
        );
        let drift = find_drift(&stored(entry));
        assert!(drift.is_empty(), "{drift:?}");
    }

    #[test]
    fn a_different_creature_type_still_drifts() {
        let entry = base(
            "ref/monster/wrong",
            EntryKind::Creature,
            TypedFields::Creature(Box::new(CreatureFields {
                creature_type: Some("dragon".into()),
                ..Default::default()
            })),
            vec![statblock(vec![row("Type", "Outsider")])],
        );
        let drift = find_drift(&stored(entry));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "creature_type");
    }

    fn creature_with_rating(cr: &str, line: &str) -> Stored {
        stored(base(
            "ref/monster/test",
            EntryKind::Creature,
            TypedFields::Creature(Box::new(CreatureFields {
                cr: Some(ChallengeRating::parse(cr).unwrap()),
                ..Default::default()
            })),
            vec![Block::Paragraph {
                style: ParagraphStyle::Normal,
                content: labelled("Challenge rating", line),
            }],
        ))
    }

    #[test]
    fn a_challenge_rating_line_agrees() {
        assert!(find_drift(&creature_with_rating("5", "5")).is_empty());
    }

    #[test]
    fn a_changed_challenge_rating_names_both_places() {
        let drift = find_drift(&creature_with_rating("5", "6"));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "cr");
        let text = drift[0].describe();
        assert!(text.contains("[fields] cr = \"5\""), "{text}");
        assert!(text.contains("`**Challenge rating:**` line"), "{text}");
        assert!(text.contains("\"6\""), "{text}");
    }

    #[test]
    fn challenge_rating_fractions_compare_by_value() {
        assert!(find_drift(&creature_with_rating("1/2", "1/2")).is_empty());
        let drift = find_drift(&creature_with_rating("1/2", "1/3"));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "cr");
        assert_eq!(drift[0].field_value, "1/2");
        assert_eq!(drift[0].source_value, "1/3");
    }

    fn creature(fields: CreatureFields, body: Vec<Block>) -> Stored {
        stored(base(
            "ref/monster/test",
            EntryKind::Creature,
            TypedFields::Creature(Box::new(fields)),
            body,
        ))
    }

    /// The same statements as bold-label paragraphs, the form of the entries before the
    /// stat-block pass.
    fn paragraphs(pairs: &[(&str, &str)]) -> Vec<Block> {
        pairs
            .iter()
            .map(|(label, text)| Block::Paragraph {
                style: ParagraphStyle::Normal,
                content: labelled(label, text),
            })
            .collect()
    }

    #[test]
    fn a_challenge_rating_row_compares_by_value() {
        let fields = |cr: &str| CreatureFields {
            cr: Some(ChallengeRating::parse(cr).unwrap()),
            ..Default::default()
        };
        let block = |text: &str| vec![statblock(vec![row("Challenge rating", text)])];
        assert!(find_drift(&creature(fields("1/2"), block("1/2"))).is_empty());
        assert!(find_drift(&creature(fields("8"), block("8"))).is_empty());
        let drift = find_drift(&creature(fields("1/2"), block("1/3")));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "cr");
        assert!(
            drift[0].source.contains("`Challenge rating`"),
            "{}",
            drift[0].source
        );
        // The paragraph form keeps comparing as before.
        let lines = paragraphs(&[("Challenge rating", "1/2")]);
        assert!(find_drift(&creature(fields("1/2"), lines)).is_empty());
        let lines = paragraphs(&[("Challenge rating", "1/3")]);
        assert_eq!(find_drift(&creature(fields("1/2"), lines)).len(), 1);
    }

    /// One `##` stat set: a heading and a stat block with the given rows.
    fn stat_set(title: &str, rows: &[(&str, &str)]) -> Vec<Block> {
        vec![
            Block::Heading {
                level: 2,
                anchor: None,
                content: vec![Inline::text(title)],
            },
            statblock(
                rows.iter()
                    .map(|(label, value)| row(label, value))
                    .collect(),
            ),
        ]
    }

    fn first_set_fields() -> CreatureFields {
        CreatureFields {
            cr: Some(ChallengeRating::parse("1").unwrap()),
            size: Some("small".into()),
            hp: Some("13~2d10+2".into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_later_stat_set_never_drifts_against_the_fields_of_the_first() {
        let mut body = stat_set(
            "Small",
            &[
                ("Challenge rating", "1"),
                ("Size", "small"),
                ("Hit points", "13 (2d10+2)"),
            ],
        );
        for (title, cr, size, hp) in [
            ("Medium", "3", "medium", "39 (4d10+16)"),
            ("Large", "5", "large", "68 (8d10+24)"),
            ("Elder", "11", "huge", "152 (16d10+64)"),
        ] {
            body.extend(stat_set(
                title,
                &[("Challenge rating", cr), ("Size", size), ("Hit points", hp)],
            ));
        }
        let drift = find_drift(&creature(first_set_fields(), body));
        assert!(drift.is_empty(), "{drift:?}");
    }

    #[test]
    fn the_first_stat_set_still_drifts_against_the_fields() {
        let mut body = stat_set("Small", &[("Challenge rating", "2"), ("Size", "small")]);
        body.extend(stat_set(
            "Large",
            &[("Challenge rating", "1"), ("Size", "small")],
        ));
        let drift = find_drift(&creature(first_set_fields(), body));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "cr");
        assert_eq!(drift[0].source_value, "2");
    }

    #[test]
    fn a_row_the_first_set_lacks_is_not_taken_from_a_later_set() {
        let mut body = stat_set("Small", &[("Challenge rating", "1")]);
        body.extend(stat_set(
            "Large",
            &[("Challenge rating", "5"), ("Size", "huge")],
        ));
        let drift = find_drift(&creature(first_set_fields(), body));
        assert!(drift.is_empty(), "{drift:?}");
    }

    #[test]
    fn the_paragraph_rating_of_a_later_set_is_not_read_either() {
        let mut body = paragraphs(&[("Challenge rating", "1")]);
        body.push(Block::Heading {
            level: 2,
            anchor: None,
            content: vec![Inline::text("Small")],
        });
        body.push(statblock(vec![row("Challenge rating", "1")]));
        body.extend(stat_set("Large", &[("Challenge rating", "5")]));
        body.extend(paragraphs(&[("Challenge rating", "5")]));
        let drift = find_drift(&creature(first_set_fields(), body));
        assert!(drift.is_empty(), "{drift:?}");
    }

    #[test]
    fn an_entry_with_one_stat_block_section_compares_all_of_it() {
        let body = stat_set("Ettercap", &[("Challenge rating", "5")]);
        let drift = find_drift(&creature(first_set_fields(), body));
        assert_eq!(drift.len(), 1, "{drift:?}");
    }

    #[test]
    fn the_type_row_holds_the_type_alone_beside_a_subtype_row() {
        let fields = |subtypes: &[&str]| CreatureFields {
            creature_type: Some("outsider".into()),
            subtypes: subtypes.iter().map(|item| item.to_string()).collect(),
            ..Default::default()
        };
        let block = |kind: &str, subtype: &str| {
            vec![statblock(vec![row("Type", kind), row("Subtype", subtype)])]
        };
        let both = fields(&["chaotic", "demon", "evil", "extraplanar"]);
        let ok = block("outsider", "Chaotic, Demon, Evil, Extraplanar");
        assert!(find_drift(&creature(both.clone(), ok)).is_empty());
        // The subtypes compare as a set, in any order.
        let shuffled = block("Outsider", "Evil, Extraplanar, Demon, Chaotic");
        assert!(find_drift(&creature(both.clone(), shuffled)).is_empty());
        // A subtype that differs is drift in `subtypes`, not in the type.
        let wrong = block("outsider", "Chaotic, Demon, Extraplanar");
        let drift = find_drift(&creature(both.clone(), wrong));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "subtypes");
        assert!(drift[0].source.contains("`Subtype`"), "{}", drift[0].source);
        // A different type is drift in `creature_type`.
        let wrong = block("dragon", "Chaotic, Demon, Evil, Extraplanar");
        let drift = find_drift(&creature(both, wrong));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "creature_type");
        // A field with no subtypes leaves the `Subtype` row uncompared.
        let none = block("outsider", "Native");
        assert!(find_drift(&creature(fields(&[]), none)).is_empty());
        // Without a `Subtype` row, the `Type` row still adds the subtypes in parentheses.
        let inline = vec![statblock(vec![row("Type", "Outsider (native)")])];
        assert!(find_drift(&creature(fields(&["native"]), inline)).is_empty());
        // The paragraph form is not compared, so it cannot drift.
        let lines = paragraphs(&[("Type", "outsider"), ("Subtype", "Native")]);
        assert!(find_drift(&creature(fields(&["native"]), lines)).is_empty());
    }

    #[test]
    fn armor_class_rows_compare_as_one_value() {
        let fields = |ac: &str| CreatureFields {
            ac: Some(ac.into()),
            ..Default::default()
        };
        let block = |armor: &str, touch: &str, flat: &str| {
            vec![statblock(vec![
                row("Armor class", armor),
                row("Touch", touch),
                row("Flat-footed", flat),
                row("AC modifiers", "+1 Dex, +10 natural, -1 size"),
            ])]
        };
        assert!(find_drift(&creature(fields("20/10/19"), block("20", "10", "19"))).is_empty());
        let drift = find_drift(&creature(fields("20/10/19"), block("20", "10", "18")));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "ac");
        assert_eq!(drift[0].source_value, "20/10/18");
        // One row alone says nothing about the whole value.
        let partial = vec![statblock(vec![row("Armor class", "21")])];
        assert!(find_drift(&creature(fields("20/10/19"), partial)).is_empty());
        // The paragraph form is not compared, so it cannot drift.
        let lines = paragraphs(&[
            ("Armor class", "21"),
            ("Touch", "10"),
            ("Flat-footed", "19"),
        ]);
        assert!(find_drift(&creature(fields("20/10/19"), lines)).is_empty());
    }

    #[test]
    fn hit_points_read_a_tilde_as_parentheses() {
        let fields = |hp: &str| CreatureFields {
            hp: Some(hp.into()),
            ..Default::default()
        };
        let block = |label: &str, text: &str| vec![statblock(vec![row(label, text)])];
        assert!(
            find_drift(&creature(
                fields("103~9d12+45"),
                block("Hit points", "103 (9d12+45)")
            ))
            .is_empty()
        );
        assert!(
            find_drift(&creature(
                fields("103~9d12+45"),
                block("hp", "103 (9d12+45)")
            ))
            .is_empty()
        );
        assert!(find_drift(&creature(fields("10"), block("Hit points", "10"))).is_empty());
        // Print holds a second tilde in the dice (`16~4`); only the first one is the separator.
        assert!(
            find_drift(&creature(
                fields("34~3d8+1d10+16~4"),
                block("Hit points", "34 (3d8+1d10+16~4)")
            ))
            .is_empty()
        );
        let drift = find_drift(&creature(
            fields("103~9d12+45"),
            block("Hit points", "104 (9d12+45)"),
        ));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "hp");
        assert_eq!(drift[0].field_value, "103~9d12+45");
        assert_eq!(drift[0].source_value, "104 (9d12+45)");
        // The paragraph form is not compared, so it cannot drift.
        let lines = paragraphs(&[("Hit points", "104 (9d12+45)")]);
        assert!(find_drift(&creature(fields("103~9d12+45"), lines)).is_empty());
    }

    #[test]
    fn an_ability_row_keeps_the_value_after_each_name() {
        let fields = |first: &str| CreatureFields {
            abilities: Some([
                first.into(),
                "16".into(),
                "-".into(),
                "11".into(),
                "13".into(),
                "22".into(),
            ]),
            ..Default::default()
        };
        let block = |text: &str| vec![statblock(vec![row("Ability scores", text)])];
        let printed = "Str - (22 while corporeal), Dex 16, Con -, Int 11, Wis 13, Cha 22";
        assert!(find_drift(&creature(fields("- (22 while corporeal)"), block(printed))).is_empty());
        let drift = find_drift(&creature(fields("-"), block(printed)));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "abilities");
        assert_eq!(
            parse_abilities("Str 3, Dex 16, Con -"),
            vec!["3", "16", "-"]
        );
        // The paragraph form is not compared, so it cannot drift.
        let lines = paragraphs(&[("Ability scores", printed)]);
        assert!(find_drift(&creature(fields("5"), lines)).is_empty());
    }

    fn item_with_slot(slot: &str, printed: &str) -> Stored {
        stored(base(
            "ref/item/test",
            EntryKind::Item,
            TypedFields::Item(ItemFields {
                slot: Some(slot.into()),
                ..Default::default()
            }),
            vec![statblock(vec![row("Slot", printed)])],
        ))
    }

    #[test]
    fn a_slot_row_agrees_with_the_closed_value_it_maps_to() {
        // The corpus maps the spellings in the field only; the row keeps the printed one.
        for (field, printed) in [
            ("neck", "neck"),
            ("neck", "amulet"),
            ("neck", "neck (but see text)"),
            ("none", "-"),
            ("none", "held"),
            ("none", "none (replaces hand)"),
            ("none", "none (see below)"),
            ("none", "none; price 10,000 gp; weight 1 lb."),
            ("shoulders", "shoulder"),
            ("shoulders", "cloak"),
            ("belt", "waist"),
            ("eyes", "eye"),
            ("eyes", "eye; Price 6,000 gp (Minor), 18,000 gp (Major)"),
            ("armor", "armor (barding)"),
            ("neck or none", "neck or none (see text)"),
            ("neck or none", "neck or -"),
            // A value that stays open keeps its own text.
            ("head and eyes", "head and eyes"),
            ("chest and neck (see text)", "chest and neck (see text)"),
            ("arms (cybernetic)", "arms (cybernetic)"),
            ("weapon", "weapon"),
        ] {
            let drift = find_drift(&item_with_slot(field, printed));
            assert!(drift.is_empty(), "{field} / {printed}: {drift:?}");
        }
    }

    #[test]
    fn a_slot_row_that_names_another_slot_still_drifts() {
        for (field, printed) in [
            ("neck", "ring"),
            ("neck", "amulet or ring"),
            ("none", "neck"),
            ("belt", "shoulder"),
            ("armor", "armor quality"),
            // A tech slot with parentheses is whole, not `arms` with a qualifier.
            ("arms", "arms (cybernetic)"),
            // `eye` maps to `eyes` whole, not by prefix.
            ("eyes", "eyewear"),
        ] {
            let drift = find_drift(&item_with_slot(field, printed));
            assert_eq!(drift.len(), 1, "{field} / {printed}: {drift:?}");
            assert_eq!(drift[0].field, "slot");
        }
        let text = find_drift(&item_with_slot("neck", "ring"))[0].describe();
        assert!(text.contains("stat block row `Slot`"), "{text}");
    }

    #[test]
    fn a_slot_in_a_paragraph_is_not_compared() {
        let body = paragraphs(&[("Slot", "amulet")]);
        let item = stored(base(
            "ref/item/test",
            EntryKind::Item,
            TypedFields::Item(ItemFields {
                slot: Some("ring".into()),
                ..Default::default()
            }),
            body,
        ));
        assert!(find_drift(&item).is_empty());
    }

    #[test]
    fn item_rows_are_compared() {
        let entry = base(
            "ref/item/test",
            EntryKind::Item,
            TypedFields::Item(ItemFields {
                price: Some("300 gp".into()),
                weight: Some("1 lb.".into()),
                ..Default::default()
            }),
            vec![statblock(vec![
                row("Price", "350 gp"),
                row("Weight", "1 lb"),
            ])],
        );
        let drift = find_drift(&stored(entry));
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].field, "price");
    }

    #[test]
    fn feat_prerequisites_and_benefit_come_from_the_bold_labels() {
        let paragraph = Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                Inline::text("Intro."),
                Inline::Break,
                Inline::Strong {
                    c: vec![Inline::text("Prerequisites:")],
                },
                Inline::text(" Str 13, base attack bonus +1."),
                Inline::Break,
                Inline::Strong {
                    c: vec![Inline::text("Benefit:")],
                },
                Inline::text(" You gain a bonus."),
            ],
        };
        let entry = base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                prerequisites: Some("Power Attack.".into()),
                benefit: Some("You gain a bonus.".into()),
                ..Default::default()
            }),
            vec![paragraph],
        );
        let drift = find_drift(&stored(entry));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "prerequisites");
        assert!(drift[0].describe().contains("`**Prerequisites:**` line"));
    }

    fn feat_with_statements(prerequisites: &str, statements: &[&str]) -> Stored {
        let mut content = Vec::new();
        for statement in statements {
            if !content.is_empty() {
                content.push(Inline::Break);
            }
            content.push(Inline::Strong {
                c: vec![Inline::text("Prerequisites:")],
            });
            content.push(Inline::text(format!(" {statement}")));
        }
        stored(base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                prerequisites: Some(prerequisites.into()),
                ..Default::default()
            }),
            vec![Block::Paragraph {
                style: ParagraphStyle::Normal,
                content,
            }],
        ))
    }

    #[test]
    fn a_prerequisites_field_matches_only_the_first_printed_statement() {
        let statements = ["Str 13.", "Dex 15."];
        assert!(find_drift(&feat_with_statements("Str 13.", &statements)).is_empty());
        let drift = find_drift(&feat_with_statements("Dex 15.", &statements));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "prerequisites");
        assert_eq!(drift[0].source_value, "Str 13.");
    }

    #[test]
    fn a_benefit_field_may_match_a_later_printed_statement() {
        let labelled = |text: &str| {
            [
                Inline::Strong {
                    c: vec![Inline::text("Benefit:")],
                },
                Inline::text(format!(" {text}")),
            ]
        };
        let mut content = Vec::new();
        content.extend(labelled("First."));
        content.push(Inline::Break);
        content.extend(labelled("Second."));
        let entry = base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                benefit: Some("Second.".into()),
                ..Default::default()
            }),
            vec![Block::Paragraph {
                style: ParagraphStyle::Normal,
                content,
            }],
        );
        assert!(find_drift(&stored(entry)).is_empty());
    }

    #[test]
    fn a_printed_label_without_its_field_is_listed() {
        let printed = |label: &str| Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                Inline::Strong {
                    c: vec![Inline::text(format!("{label}:"))],
                },
                Inline::text(" Printed."),
            ],
        };
        let feat = |fields: FeatFields| {
            stored(base(
                "ref/feat/test",
                EntryKind::Feat,
                TypedFields::Feat(fields),
                vec![printed("Prerequisites"), printed("Special")],
            ))
        };
        let both = unfielded_feat_labels(&feat(FeatFields::default()));
        assert_eq!(
            both.iter().map(|item| item.0).collect::<Vec<_>>(),
            ["prerequisites", "special"]
        );
        let none = unfielded_feat_labels(&feat(FeatFields {
            prerequisites: Some("Printed.".into()),
            special: Some("Printed.".into()),
            ..Default::default()
        }));
        assert!(none.is_empty(), "{none:?}");
    }

    #[test]
    fn a_value_stated_in_only_one_place_is_not_drift() {
        let entry = base(
            "ref/item/test",
            EntryKind::Item,
            TypedFields::Item(ItemFields {
                price: Some("300 gp".into()),
                ..Default::default()
            }),
            vec![statblock(vec![row("Weight", "1 lb.")])],
        );
        assert!(find_drift(&stored(entry)).is_empty());
    }

    fn labelled(label: &str, text: &str) -> Vec<Inline> {
        vec![
            Inline::Strong {
                c: vec![Inline::text(format!("{label}:"))],
            },
            Inline::text(format!(" {text}")),
        ]
    }

    /// A feat whose normal part states one `**Benefit:**` and whose `### Mythic Test`
    /// subsection, carrying its own `**Sources:**` line, states another.
    fn mythic_feat(benefit: &str) -> Stored {
        let mut subsection = vec![
            Inline::Strong {
                c: vec![Inline::text("Sources:")],
            },
            Inline::text(" Mythic Adventures pg 1"),
            Inline::Break,
        ];
        subsection.extend(labelled("Benefit", "Mythic *wording*."));
        stored(base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                benefit: Some(benefit.into()),
                ..Default::default()
            }),
            vec![
                Block::Paragraph {
                    style: ParagraphStyle::Normal,
                    content: labelled("Benefit", "Normal wording."),
                },
                Block::Heading {
                    level: 3,
                    anchor: None,
                    content: vec![Inline::text("Mythic Test")],
                },
                Block::Paragraph {
                    style: ParagraphStyle::Normal,
                    content: subsection,
                },
            ],
        ))
    }

    #[test]
    fn mythic_wording_in_a_base_field_is_drift() {
        let drift = find_drift(&mythic_feat("Mythic *wording*."));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "benefit");
        assert_eq!(drift[0].source_value, "Normal wording.");
    }

    #[test]
    fn a_field_that_matches_the_normal_benefit_is_not_drift() {
        assert!(find_drift(&mythic_feat("Normal wording.")).is_empty());
    }

    fn paragraph(content: Vec<Inline>) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }
    }

    fn sources(text: &str) -> Block {
        paragraph(vec![
            Inline::Strong {
                c: vec![Inline::text("Sources:")],
            },
            Inline::text(format!(" {text}")),
        ])
    }

    #[test]
    fn a_heading_without_its_own_sources_line_stays_compared() {
        // `### Options` organizes the normal feat's own text; it is not a source block.
        let entry = base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                benefit: Some("Variant wording.".into()),
                ..Default::default()
            }),
            vec![
                paragraph(labelled("Benefit", "Normal wording.")),
                Block::Heading {
                    level: 3,
                    anchor: None,
                    content: vec![Inline::text("Options")],
                },
                paragraph(labelled("Benefit", "Variant wording.")),
            ],
        );
        assert!(find_drift(&stored(entry)).is_empty());
    }

    #[test]
    fn an_eitr_tail_after_a_bare_sources_line_is_cut() {
        // A bare `**Sources:**` line starts another source's block, here the
        // Elephant-in-the-Room tail that follows the normal feat.
        let entry = base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                benefit: Some("Tail wording.".into()),
                ..Default::default()
            }),
            vec![
                sources("Core Rulebook pg 1"),
                paragraph(labelled("Benefit", "Normal wording.")),
                sources("The Elephant in the Room: Feat Taxes in Pathfinder"),
                paragraph(labelled("Benefit", "Tail wording.")),
            ],
        );
        let drift = find_drift(&stored(entry));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "benefit");
        assert_eq!(drift[0].source_value, "Normal wording.");
    }

    #[test]
    fn a_labelled_statement_inside_a_quote_still_counts() {
        let entry = base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                normal: Some("A normal choice.".into()),
                ..Default::default()
            }),
            vec![
                sources("Core Rulebook pg 1"),
                paragraph(labelled("Benefit", "Normal wording.")),
                Block::Quote {
                    content: vec![paragraph(labelled("Normal", "A normal choice."))],
                },
            ],
        );
        assert!(find_drift(&stored(entry)).is_empty());
    }

    #[test]
    fn a_field_with_no_statement_in_the_normal_part_is_drift() {
        // Prerequisites appear only in the mythic subsection, so the normal part has no
        // line to compare; the field must not pass silently.
        let entry = base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                prerequisites: Some("Mythic Test.".into()),
                ..Default::default()
            }),
            vec![
                sources("Core Rulebook pg 1"),
                paragraph(labelled("Benefit", "Normal wording.")),
                Block::Heading {
                    level: 3,
                    anchor: None,
                    content: vec![Inline::text("Mythic Test")],
                },
                sources("Mythic Adventures pg 1"),
                paragraph(labelled("Prerequisites", "Mythic Test.")),
            ],
        );
        let drift = find_drift(&stored(entry));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "prerequisites");
        assert!(drift[0].missing);
        assert!(
            drift[0]
                .describe()
                .contains("has no `**Prerequisites:**` statement in the normal part")
        );
    }

    /// The entry as two publications print it, with a `[[sources]]` row for each and no
    /// `**Sources:**` paragraph in the body.
    fn printed_twice(mut entry: RuleEntry) -> RuleEntry {
        entry.source_ids = vec!["core-rulebook".into(), "advanced-players-guide".into()];
        entry.sources = entry
            .source_ids
            .iter()
            .map(|publication| cast_iron_schema::Citation {
                publication: publication.clone(),
                page: Some("10".into()),
            })
            .collect();
        entry
    }

    fn subsection(title: &str) -> Block {
        Block::Heading {
            level: 3,
            anchor: None,
            content: vec![Inline::text(title)],
        }
    }

    /// A class option whose body prints one `**Replaces:**` line above a `###` subsection and
    /// one inside it, with `typed` as the typed list.
    fn domain_with_subdomain(typed: &[&str], head: Vec<Block>) -> RuleEntry {
        let mut body = vec![paragraph(labelled("Replaces", "Detect evil"))];
        body.extend(head);
        body.push(subsection("Alchemy Subdomain"));
        body.push(paragraph(labelled("Replaces", "Smite evil")));
        base(
            "ref/domain/test",
            EntryKind::ClassOption,
            class_option(typed, &[]),
            body,
        )
    }

    #[test]
    fn rows_without_a_body_sources_line_end_the_normal_part_at_a_heading() {
        // Two publications, rows, no `**Sources:**` paragraph: the `###` section is another
        // source's block, so the line inside it is not compared.
        let entry = printed_twice(domain_with_subdomain(&["Detect evil"], vec![]));
        assert!(find_drift(&stored(entry)).is_empty());
        // The line above the heading still is.
        let entry = printed_twice(domain_with_subdomain(&["Smite evil"], vec![]));
        let drift = find_drift(&stored(entry));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "replaces");
        assert_eq!(drift[0].source_value, "Detect evil");
    }

    #[test]
    fn only_an_unmarked_heading_of_a_multi_publication_entry_is_listed() {
        // Two publications and no `**Sources:**` paragraph: the heading is cut by `source_ids` alone.
        let twice = printed_twice(domain_with_subdomain(&["Detect evil"], vec![]));
        let before = find_drift(&stored(twice.clone()));
        // The heading is the second block, and its line comes from the parser's block lines.
        let mut located = stored(twice);
        located.block_lines = vec![5, 9, 11];
        assert_eq!(unmarked_heading_cut(&located), Some(9));
        // Listing the entry does not change the drift result.
        assert_eq!(before.len(), find_drift(&located).len());
        // One publication: the heading stays in the normal part.
        let once = domain_with_subdomain(&["Detect evil", "Smite evil"], vec![]);
        assert_eq!(unmarked_heading_cut(&stored(once)), None);
        // A `**Sources:**` paragraph below the heading marks it, so it is not listed.
        let mut marked = domain_with_subdomain(&["Detect evil"], vec![]);
        marked
            .body
            .insert(2, sources("Advanced Player's Guide pg 10"));
        assert_eq!(unmarked_heading_cut(&stored(printed_twice(marked))), None);
        // No heading, nothing cut.
        let mut plain = printed_twice(domain_with_subdomain(&["Detect evil"], vec![]));
        plain.body.truncate(1);
        assert_eq!(unmarked_heading_cut(&stored(plain)), None);
    }

    #[test]
    fn a_heading_of_a_single_publication_entry_stays_in_the_normal_part() {
        // One publication has no other source, so its `###` section organizes the normal part.
        let both = domain_with_subdomain(&["Detect evil", "Smite evil"], vec![]);
        assert!(find_drift(&stored(both)).is_empty());
        let first_only = domain_with_subdomain(&["Detect evil"], vec![]);
        let drift = find_drift(&stored(first_only));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].source_value, "Detect evil, Smite evil");
    }

    #[test]
    fn a_heading_and_sources_line_still_end_the_normal_part() {
        // The paragraph the data keeps for a blog or conflicting citation still marks the block,
        // with or without rows, and for an entry of one publication too.
        let mut entry = domain_with_subdomain(&["Detect evil"], vec![]);
        entry
            .body
            .insert(2, sources("Advanced Player's Guide pg 10"));
        assert!(find_drift(&stored(entry.clone())).is_empty());
        let drift = find_drift(&stored(printed_twice(entry)));
        assert!(drift.is_empty(), "{drift:?}");
        // A bare line with no heading ends it as well.
        let mut bare = domain_with_subdomain(&["Detect evil"], vec![]);
        bare.body.remove(1);
        bare.body
            .insert(1, sources("Advanced Player's Guide pg 10"));
        assert!(find_drift(&stored(bare)).is_empty());
    }

    #[test]
    fn a_leading_sources_line_does_not_end_the_normal_part() {
        // A first block names the entry's own citation (a blog post that no row carries).
        let mut entry = printed_twice(domain_with_subdomain(&["Detect evil"], vec![]));
        entry.body.insert(0, sources("Paizo Blog - Test"));
        assert!(find_drift(&stored(entry)).is_empty());
    }

    #[test]
    fn variant_paragraphs_after_the_normal_part_are_not_compared() {
        let feat = |benefit: &str| {
            printed_twice(base(
                "ref/feat/test",
                EntryKind::Feat,
                TypedFields::Feat(FeatFields {
                    benefit: Some(benefit.into()),
                    ..Default::default()
                }),
                vec![
                    paragraph(labelled("Benefit", "Normal wording.")),
                    subsection("Mythic Test"),
                    paragraph(labelled("Benefit", "Mythic wording.")),
                ],
            ))
        };
        assert!(find_drift(&stored(feat("Normal wording."))).is_empty());
        let drift = find_drift(&stored(feat("Mythic wording.")));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].source_value, "Normal wording.");
        // A label that only the variant states is reported, as with a body Sources line.
        let mut only_variant = feat("Normal wording.");
        let TypedFields::Feat(fields) = &mut only_variant.fields else {
            panic!("feat");
        };
        fields.prerequisites = Some("Mythic Test.".into());
        only_variant
            .body
            .push(paragraph(labelled("Prerequisites", "Mythic Test.")));
        let drift = find_drift(&stored(only_variant));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert!(drift[0].missing);
    }

    #[test]
    fn stat_blocks_neither_end_nor_mark_the_normal_part() {
        let titled = Block::StatBlock {
            title: Some("Advanced".into()),
            rows: vec![row("AC", "99")],
        };
        // A creature: the rows come from the first stat block, whether a later titled block
        // follows a `###` heading or not.
        let entry = printed_twice(base(
            "ref/monster/test",
            EntryKind::Creature,
            TypedFields::Creature(Box::new(CreatureFields {
                ac: Some("15".into()),
                ..Default::default()
            })),
            vec![
                statblock(vec![row("AC", "15")]),
                subsection("Advanced Test"),
                titled.clone(),
            ],
        ));
        assert!(find_drift(&stored(entry)).is_empty());
        // An archetype: an untitled and a titled stat block between the printed lines stay in
        // the normal part, and the part ends at the heading below them.
        let entry = printed_twice(base(
            "ref/arc-fighter/test",
            EntryKind::Archetype,
            archetype(&["Detect evil", "Smite evil"], &[]),
            vec![
                paragraph(labelled("Replaces", "Detect evil")),
                statblock(vec![row("Size", "Medium")]),
                titled.clone(),
                paragraph(labelled("Replaces", "Smite evil")),
                subsection("Variant"),
                paragraph(labelled("Replaces", "Channel positive energy")),
            ],
        ));
        assert!(find_drift(&stored(entry)).is_empty());
    }

    fn footnote_feat(field: &str, body: Vec<Inline>) -> Stored {
        let mut content = vec![Inline::Strong {
            c: vec![Inline::text("Prerequisites:")],
        }];
        content.extend(body);
        stored(base(
            "ref/feat/test",
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                prerequisites: Some(field.into()),
                ..Default::default()
            }),
            vec![Block::Paragraph {
                style: ParagraphStyle::Normal,
                content,
            }],
        ))
    }

    #[test]
    fn footnote_markers_are_ignored_when_comparing() {
        let sup = |text: &str| Inline::Sup {
            c: vec![Inline::text(text)],
        };
        // `[^P]` in the field; the body carries the marker as a superscript.
        assert!(
            find_drift(&footnote_feat(
                "Stealth 3 ranks, Survival 3 ranks.[^P]",
                vec![
                    Inline::text(" Stealth 3 ranks, Survival 3 ranks."),
                    sup("1"),
                ],
            ))
            .is_empty()
        );
        // The flattened form: the superscript's link is a bare number glued to the word.
        assert!(
            find_drift(&footnote_feat(
                "Stealth 3 ranks, Survival 3 ranks.[^P]",
                vec![Inline::text(" Stealth 3 ranks, Survival 3 ranks.1")],
            ))
            .is_empty()
        );
        // A marker that follows a space (`Play ^1^).`) still matches a field `[^X]`.
        assert!(
            find_drift(&footnote_feat(
                "Ability to attempt Day Job checks (see the Guide [^X]).",
                vec![
                    Inline::text(" Ability to attempt Day Job checks (see the Guide "),
                    sup("1"),
                    Inline::text(")."),
                ],
            ))
            .is_empty()
        );
    }

    #[test]
    fn markdown_markers_and_entities_in_a_field_are_not_drift() {
        let mut stored_spell = spell("close", "close");
        let TypedFields::Spell(fields) = &mut stored_spell.entry.fields else {
            panic!("spell");
        };
        fields.components = Some("V, S, [M](rule/m) &times; 2".into());
        let Block::StatBlock { rows, .. } = &mut stored_spell.entry.body[0] else {
            panic!("stat block");
        };
        rows.push(row("Components", "V, S, M × 2"));
        assert!(find_drift(&stored_spell).is_empty());
    }

    #[test]
    fn the_school_row_may_add_a_subschool_and_descriptors() {
        let set_school = |stored: &mut Stored, text: &str| {
            let Block::StatBlock { rows, .. } = &mut stored.entry.body[0] else {
                panic!("stat block");
            };
            rows[0] = row("School", text);
        };
        let mut stored_spell = spell("close", "close");
        set_school(&mut stored_spell, "evocation (force) [fire]");
        assert!(find_drift(&stored_spell).is_empty());
        set_school(&mut stored_spell, "necromancy [death]");
        assert_eq!(find_drift(&stored_spell).len(), 1);
    }

    fn attached(id: &str, kind: EntryKind, fields: TypedFields, labels: &[(&str, &str)]) -> Stored {
        let body = labels
            .iter()
            .map(|(label, text)| paragraph(labelled(label, text)))
            .collect();
        stored(base(id, kind, fields, body))
    }

    fn trades(replaces: &[&str], alters: &[&str]) -> Vec<cast_iron_schema::Trade> {
        let rows = |op, names: &[&str]| -> Vec<cast_iron_schema::Trade> {
            names
                .iter()
                .map(|name| cast_iron_schema::Trade::unresolved(op, *name))
                .collect()
        };
        let mut out = rows(TradeOp::Replaces, replaces);
        out.extend(rows(TradeOp::Alters, alters));
        out
    }

    fn archetype(replaces: &[&str], alters: &[&str]) -> TypedFields {
        TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
            trades: trades(replaces, alters),
            ..Default::default()
        })
    }

    fn class_option(replaces: &[&str], alters: &[&str]) -> TypedFields {
        TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
            group: "paladin-oath".into(),
            trades: trades(replaces, alters),
            ..Default::default()
        })
    }

    #[test]
    fn several_replaces_lines_concatenate_in_body_order() {
        let entry = attached(
            "ref/arc-fighter/test",
            EntryKind::Archetype,
            archetype(
                &["Detect evil", "Smite evil", "Channel positive energy"],
                &[],
            ),
            &[
                ("Replaces", "Detect evil"),
                ("Replaces", "Smite evil"),
                ("Replaces", "Channel positive energy"),
            ],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn several_replaces_lines_must_all_be_typed() {
        let entry = attached(
            "ref/arc-fighter/test",
            EntryKind::Archetype,
            archetype(&["Detect evil", "Smite evil"], &[]),
            &[
                ("Replaces", "Detect evil"),
                ("Replaces", "Smite evil"),
                ("Replaces", "Channel positive energy"),
            ],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "replaces");
        assert_eq!(drift[0].field_value, "Detect evil, Smite evil");
        assert_eq!(
            drift[0].source_value,
            "Detect evil, Smite evil, Channel positive energy"
        );
    }

    #[test]
    fn several_replaces_lines_compare_in_body_order() {
        let entry = attached(
            "ref/arc-fighter/test",
            EntryKind::Archetype,
            archetype(&["Smite evil", "Detect evil"], &[]),
            &[("Replaces", "Detect evil"), ("Replaces", "Smite evil")],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "replaces");
    }

    #[test]
    fn a_single_replaces_line_still_compares_as_one_statement() {
        let entry = attached(
            "ref/arc-fighter/test",
            EntryKind::Archetype,
            archetype(&["Trap sense"], &["Uncanny dodge"]),
            &[("Replaces", "Trap sense."), ("Alters", "Uncanny dodge.")],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn a_class_option_concatenates_its_replaces_lines() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&["Detect evil", "Smite evil"], &[]),
            &[("Replaces", "Detect evil"), ("Replaces", "Smite evil")],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn printed_periods_do_not_break_a_multi_line_replaces_list() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&["Detect evil", "Smite evil"], &[]),
            &[("Replaces", "Detect evil."), ("Replaces", "Smite evil.")],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn printed_periods_do_not_break_a_multi_line_alters_list() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&[], &["Divine bond", "Aura of resolve"]),
            &[("Alters", "Divine bond."), ("Alters", "Aura of resolve.")],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn class_option_alters_agree_with_the_printed_line() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&[], &["Divine bond"]),
            &[("Alters", "Divine bond")],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn a_printed_class_option_alters_line_must_be_typed() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&[], &[]),
            &[("Alters", "Divine bond")],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "alters");
        assert!(drift[0].missing);
    }

    #[test]
    fn a_typed_class_option_alters_item_must_be_printed() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&[], &["Divine bond", "Aura of resolve"]),
            &[("Alters", "Divine bond")],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "alters");
        assert_eq!(drift[0].source_value, "Divine bond");
    }

    #[test]
    fn one_comma_separated_line_splits_into_items() {
        let entry = attached(
            "ref/arc-fighter/test",
            EntryKind::Archetype,
            archetype(&["Trap sense", "Uncanny dodge"], &[]),
            &[("Replaces", "Trap sense, Uncanny dodge")],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn several_alters_lines_concatenate_in_body_order() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&[], &["Divine bond", "Aura of resolve"]),
            &[("Alters", "Divine bond"), ("Alters", "Aura of resolve")],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn several_alters_lines_compare_in_body_order() {
        let entry = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&[], &["Aura of resolve", "Divine bond"]),
            &[("Alters", "Divine bond"), ("Alters", "Aura of resolve")],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "alters");
    }

    #[test]
    fn an_entry_with_no_typed_and_no_printed_items_has_no_pair() {
        let stored = attached(
            "ref/oath/test",
            EntryKind::ClassOption,
            class_option(&[], &[]),
            &[],
        );
        assert!(replaces_alters_pairs(&stored.entry, &stored.text, &[], &[]).is_empty());
    }

    #[test]
    fn an_extra_typed_replaces_item_is_drift() {
        let entry = attached(
            "ref/arc-fighter/test",
            EntryKind::Archetype,
            archetype(&["Trap sense", "Uncanny dodge"], &[]),
            &[("Replaces", "Trap sense")],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "replaces");
        assert_eq!(drift[0].field_value, "Trap sense, Uncanny dodge");
        assert_eq!(drift[0].source_value, "Trap sense");
    }

    #[test]
    fn the_check_fails_on_drift_with_counts_and_a_report() {
        let drifting = spell("close (25 ft.)", "medium (100 ft.)");
        let report = crate::check(&crate::testkit::corpus(vec![drifting.entry]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].contains("stat block row `Range`"),
            "{}",
            report.problems[0]
        );
        assert_eq!(report.drift.len(), 1);
        assert_eq!(report.drift_by_kind().get("spell"), Some(&1));
        let text = report.drift_report();
        assert!(text.contains("| spell | 1 |"), "{text}");
        assert!(text.contains("| **total** | **1** |"), "{text}");
        assert!(text.contains("stat block row `Range`"), "{text}");
    }

    fn feature_heading_at(level: u8, anchor: &str, title: &str) -> Block {
        Block::Heading {
            level,
            anchor: Some(anchor.into()),
            content: vec![Inline::text(title)],
        }
    }

    fn text_block(text: &str) -> Block {
        paragraph(vec![Inline::text(text)])
    }

    fn trade_by(op: TradeOp, printed: &str, by: &[&str]) -> cast_iron_schema::Trade {
        cast_iron_schema::Trade {
            by: by.iter().map(|anchor| anchor.to_string()).collect(),
            ..cast_iron_schema::Trade::unresolved(op, printed)
        }
    }

    /// An archetype in the printed form: one headed feature per `(anchor, paragraph)` and the
    /// features and trades that state them.
    fn printed_archetype_at(
        level: u8,
        sections: &[(&str, &str)],
        trades: Vec<cast_iron_schema::Trade>,
    ) -> Stored {
        let mut body = Vec::new();
        for (anchor, paragraph) in sections {
            body.push(feature_heading_at(level, anchor, anchor));
            body.push(text_block(paragraph));
        }
        stored(base(
            "ref/arc-fighter/two-handed-fighter",
            EntryKind::Archetype,
            TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
                features: sections
                    .iter()
                    .map(|(anchor, _)| cast_iron_schema::FeatureFact {
                        anchor: (*anchor).into(),
                        level: Some(2),
                    })
                    .collect(),
                trades,
                ..Default::default()
            }),
            body,
        ))
    }

    fn printed_archetype(
        sections: &[(&str, &str)],
        trades: Vec<cast_iron_schema::Trade>,
    ) -> Stored {
        printed_archetype_at(2, sections, trades)
    }

    const SHATTERING: &str = "At 2nd level, a two-handed fighter gains a +1 bonus on damage rolls with two-handed weapons. This ability replaces bravery.";

    #[test]
    fn a_trade_that_its_feature_prints_is_not_drift() {
        let entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![trade_by(
                TradeOp::Replaces,
                "Bravery",
                &["shattering-strike"],
            )],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn a_trade_sentence_without_a_trade_is_drift() {
        let entry = printed_archetype(&[("shattering-strike", SHATTERING)], vec![]);
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "trades");
        assert_eq!(drift[0].field_value, "none");
        assert_eq!(drift[0].source_value, "replaces: bravery");
        assert!(drift[0].source.contains("shattering-strike"), "{drift:?}");
    }

    #[test]
    fn a_trade_that_its_feature_does_not_print_is_drift() {
        let entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Replaces, "Weapon training", &["shattering-strike"]),
            ],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        // The op counts: the sentence replaces, the trade alters. The sentence stays uncovered;
        // the alters trade is unstated, which the feature may carry (ruling 24, below).
        let entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![trade_by(TradeOp::Alters, "Bravery", &["shattering-strike"])],
        );
        assert_eq!(find_drift(&entry).len(), 1);
    }

    /// A druid archetype's feature describes the changed ability and prints no trade sentence.
    const WILD_SHAPE: &str = "At 4th level, an aquatic druid can use wild shape to assume the form of a Medium or Small fish. She gains the swim speed of the form.";

    #[test]
    fn a_trade_that_print_never_states_passes_when_by_names_the_feature() {
        // Owner ruling of 2026-11-24 (#540): `alters = "Wild shape"` names the feature that
        // prints the changed ability.
        let entry = printed_archetype(
            &[("wild-shape", WILD_SHAPE)],
            vec![trade_by(TradeOp::Alters, "Wild shape", &["wild-shape"])],
        );
        assert!(find_drift(&entry).is_empty());
        // The same trade without `by` has no sentence to state it: drift.
        let entry = printed_archetype(
            &[("wild-shape", WILD_SHAPE)],
            vec![trade_by(TradeOp::Alters, "Wild shape", &[])],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(
            drift[0].source,
            "trade sentences of the text outside any feature"
        );
        assert_eq!(drift[0].field_value, "alters: Wild shape");
    }

    #[test]
    fn a_silent_feature_does_not_excuse_a_printed_sentence_or_an_unlisted_by() {
        // A printed sentence stays covered by trades: this one is not.
        let entry = printed_archetype(
            &[
                ("wild-shape", WILD_SHAPE),
                ("shattering-strike", SHATTERING),
            ],
            vec![trade_by(TradeOp::Alters, "Wild shape", &["wild-shape"])],
        );
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert!(drift[0].source.contains("shattering-strike"), "{drift:?}");
        // A feature that prints another trade of the same op does not state this one.
        let entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Replaces, "Trap sense", &["shattering-strike"]),
            ],
        );
        assert_eq!(find_drift(&entry).len(), 1);
        // A `by` that is not a feature is still drift.
        let entry = printed_archetype(
            &[("wild-shape", WILD_SHAPE)],
            vec![trade_by(TradeOp::Alters, "Wild shape", &["missing"])],
        );
        assert_eq!(find_drift(&entry).len(), 1);
    }

    #[test]
    fn an_unstated_trade_passes_per_operation() {
        // Owner ruling 24 of 2026-11-24 (#540), Preacher: the feature prints a `replaces`
        // sentence and carries an `alters` trade that print only describes.
        let entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Alters, "Weapon training", &["shattering-strike"]),
            ],
        );
        assert!(find_drift(&entry).is_empty(), "{:?}", find_drift(&entry));
        // The other way: a feature that prints an `alters` sentence carries a `replaces` trade.
        let entry = printed_archetype(
            &[(
                "weapon-focus",
                "At 1st level, a fighter gains a bonus. This ability alters weapon training.",
            )],
            vec![
                trade_by(TradeOp::Alters, "Weapon training", &["weapon-focus"]),
                trade_by(TradeOp::Replaces, "Bravery", &["weapon-focus"]),
            ],
        );
        assert!(find_drift(&entry).is_empty(), "{:?}", find_drift(&entry));
        // Within the same op the guard stays: this `replaces` trade is not printed.
        let entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Replaces, "Armor training", &["shattering-strike"]),
            ],
        );
        assert_eq!(find_drift(&entry).len(), 1);
        // The printed sentence still has to be covered.
        let entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![trade_by(
                TradeOp::Alters,
                "Weapon training",
                &["shattering-strike"],
            )],
        );
        assert_eq!(find_drift(&entry).len(), 1);
    }

    /// A listed feature, then a heading with the given text and the given paragraphs under it.
    fn with_section_after_feature(
        heading_text: &str,
        paragraphs: &[&str],
        trades: Vec<cast_iron_schema::Trade>,
    ) -> Stored {
        let mut entry = printed_archetype(&[("shattering-strike", SHATTERING)], trades);
        entry
            .entry
            .body
            .push(feature_heading_at(2, "after", heading_text));
        for paragraph in paragraphs {
            entry.entry.body.push(text_block(paragraph));
        }
        entry
    }

    #[test]
    fn an_alters_trade_may_rest_on_a_proficiency_section_that_prints_no_sentence() {
        // Owner ruling 23 of 2026-11-24 (#540), True Primitive, Gladiator, Tactician.
        let trades = || {
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Alters, "Weapon and armor proficiency", &[]),
            ]
        };
        let prose = ["A gladiator is proficient with all martial weapons."];
        for name in [
            "Weapon and Armor Proficiency",
            "weapon and armor proficiency:",
        ] {
            let drift = find_drift(&with_section_after_feature(name, &prose, trades()));
            assert!(drift.is_empty(), "{name}: {drift:?}");
        }
        // Only for `alters`.
        let replaces = vec![
            trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
            trade_by(TradeOp::Replaces, "Weapon and armor proficiency", &[]),
        ];
        let drift = find_drift(&with_section_after_feature(
            "Weapon and Armor Proficiency",
            &prose,
            replaces,
        ));
        assert_eq!(drift.len(), 1, "{drift:?}");
        // Only under that heading.
        let drift = find_drift(&with_section_after_feature(
            "Monk Training",
            &prose,
            trades(),
        ));
        assert_eq!(drift.len(), 1, "{drift:?}");
        // Without any such section the trade stays unstated.
        let drift = find_drift(&with_opening("A tyrant follows a code.", trades()));
        assert_eq!(drift.len(), 1, "{drift:?}");
        // A section that prints an `alters` sentence states its own trade: the unstated trade
        // does not hide behind it.
        let printed = ["This ability alters the armor proficiency."];
        let drift = find_drift(&with_section_after_feature(
            "Weapon and Armor Proficiency",
            &printed,
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Alters, "Armor proficiency", &[]),
                trade_by(TradeOp::Alters, "Weapon proficiency", &[]),
            ],
        ));
        assert_eq!(drift.len(), 1, "{drift:?}");
        // A proficiency subsection of a listed feature belongs to the feature.
        let mut inner = printed_archetype(&[("shattering-strike", SHATTERING)], trades());
        inner.entry.body.push(feature_heading_at(
            3,
            "inner",
            "Weapon and Armor Proficiency",
        ));
        inner.entry.body.push(text_block(prose[0]));
        assert_eq!(find_drift(&inner).len(), 1);
    }

    #[test]
    fn the_pilot_wordings_are_not_drift() {
        // Advanced Player's Guide: "This performance replaces ...", "replaces both X and Y".
        let entry = printed_archetype(
            &[
                (
                    "inspire-courage",
                    "At 1st level, a court bard inspires. This performance replaces inspire courage.",
                ),
                (
                    "ki-sacrifice",
                    "At 11th level, a monk can bring a person back to life. This ability replaces both diamond body and quivering palm.",
                ),
            ],
            vec![
                trade_by(TradeOp::Replaces, "Inspire courage", &["inspire-courage"]),
                trade_by(TradeOp::Replaces, "Diamond body", &["ki-sacrifice"]),
                trade_by(TradeOp::Replaces, "Quivering palm", &["ki-sacrifice"]),
            ],
        );
        assert!(find_drift(&entry).is_empty(), "{:?}", find_drift(&entry));
    }

    #[test]
    fn a_trade_is_read_in_the_features_it_names_only() {
        let entry = printed_archetype(
            &[
                ("shattering-strike", SHATTERING),
                (
                    "overhand-chop",
                    "At 3rd level, the fighter chops. This ability replaces armor training.",
                ),
            ],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["overhand-chop"]),
                trade_by(TradeOp::Replaces, "Armor training", &["overhand-chop"]),
            ],
        );
        let drift = find_drift(&entry);
        // `shattering-strike` prints a trade that no trade names, and `Bravery` is not printed
        // by `overhand-chop`.
        assert_eq!(drift.len(), 2, "{drift:?}");
    }

    #[test]
    fn one_trade_may_name_every_feature_that_prints_it() {
        let two = [
            ("a", "At 1st level, a. This ability replaces bravery."),
            ("b", "At 3rd level, b. This ability replaces bravery."),
        ];
        let entry = printed_archetype(
            &two,
            vec![trade_by(TradeOp::Replaces, "Bravery", &["a", "b"])],
        );
        assert!(find_drift(&entry).is_empty());
        // Naming only one leaves the other sentence uncovered.
        let entry = printed_archetype(&two, vec![trade_by(TradeOp::Replaces, "Bravery", &["a"])]);
        assert_eq!(find_drift(&entry).len(), 1);
        // The name must appear in at least one named section.
        let entry = printed_archetype(
            &two,
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["a", "b"]),
                trade_by(TradeOp::Replaces, "Armor training", &["a", "b"]),
            ],
        );
        assert_eq!(find_drift(&entry).len(), 1);
    }

    #[test]
    fn a_trade_with_an_unlisted_by_or_no_sentence_cannot_hide() {
        let mut entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![trade_by(
                TradeOp::Replaces,
                "Bravery",
                &["shattering-strike"],
            )],
        );
        if let TypedFields::Archetype(fields) = &mut entry.entry.fields {
            // No `by`: the opening text must state it, and it prints nothing.
            fields.trades.push(cast_iron_schema::Trade::unresolved(
                TradeOp::Replaces,
                "Trap sense",
            ));
            fields.trades.push(trade_by(
                TradeOp::Replaces,
                "Uncanny dodge",
                &["shattering-strike", "missing"],
            ));
        }
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 2, "{drift:?}");
        assert!(
            drift
                .iter()
                .any(|d| d.source == "trade sentences of the text outside any feature"),
            "{drift:?}"
        );
        assert!(
            drift.iter().any(|d| d.source_value.contains("`missing`")),
            "{drift:?}"
        );
    }

    /// An archetype whose opening text prints a trade, then a headed feature.
    fn with_opening(opening: &str, trades: Vec<cast_iron_schema::Trade>) -> Stored {
        let mut entry = printed_archetype(&[("shattering-strike", SHATTERING)], trades);
        entry.entry.body.insert(0, text_block(opening));
        entry
    }

    #[test]
    fn a_trade_without_by_is_stated_by_the_opening_text() {
        let both = || {
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Replaces, "Code of conduct", &[]),
            ]
        };
        let opening = "A tyrant follows a code. This ability replaces the code of conduct.";
        assert!(find_drift(&with_opening(opening, both())).is_empty());
        // The opening sentence without its trade is uncovered.
        let only_by = vec![trade_by(
            TradeOp::Replaces,
            "Bravery",
            &["shattering-strike"],
        )];
        assert_eq!(find_drift(&with_opening(opening, only_by)).len(), 1);
        // A trade without `by` that the opening text does not state.
        assert_eq!(
            find_drift(&with_opening("A tyrant follows a code.", both())).len(),
            1
        );
        // A sentence inside a feature does not state a trade of the opening text.
        let drift = find_drift(&printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Replaces, "Bravery", &[]),
            ],
        ));
        assert_eq!(drift.len(), 1, "{drift:?}");
    }

    /// A listed feature, then a heading with the given text and a trade sentence under it.
    fn with_heading_after_feature(
        heading_text: &str,
        trades: Vec<cast_iron_schema::Trade>,
    ) -> Stored {
        let mut entry = printed_archetype(&[("shattering-strike", SHATTERING)], trades);
        entry
            .entry
            .body
            .push(feature_heading_at(2, "after", heading_text));
        entry
            .entry
            .body
            .push(text_block("This ability replaces the monk's bonus feats."));
        entry
    }

    #[test]
    fn a_scaffold_heading_after_the_first_feature_prints_trades_without_by() {
        // Owner ruling 13 of 2026-11-24 (#540): "Bonus Feats" after a listed feature.
        let both = || {
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Replaces, "Bonus feats", &[]),
            ]
        };
        let drift = find_drift(&with_heading_after_feature("Bonus Feats", both()));
        assert!(drift.is_empty(), "{drift:?}");
        // The sentence needs a trade without `by`: with a `by` that names a feature that does
        // not print it, or with none at all, it is uncovered.
        let only_by = vec![trade_by(
            TradeOp::Replaces,
            "Bravery",
            &["shattering-strike"],
        )];
        assert_eq!(
            find_drift(&with_heading_after_feature("Bonus Feats", only_by)).len(),
            1
        );
        // A trade without `by` that is stated nowhere is drift.
        let nowhere = vec![
            trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
            trade_by(TradeOp::Replaces, "Trap sense", &[]),
            trade_by(TradeOp::Replaces, "Bonus feats", &[]),
        ];
        let drift = find_drift(&with_heading_after_feature("Bonus Feats", nowhere));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field_value, "replaces: Trap sense");
    }

    #[test]
    fn a_scaffold_subsection_of_a_listed_feature_belongs_to_the_feature() {
        // `### Bonus Feats` inside a listed feature: its sentence is the feature's, so a trade
        // that names the feature covers it, and a trade without `by` does not.
        let mut entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
                trade_by(TradeOp::Replaces, "Bonus feats", &["shattering-strike"]),
            ],
        );
        entry
            .entry
            .body
            .push(feature_heading_at(3, "inner", "Bonus Feats"));
        entry
            .entry
            .body
            .push(text_block("This ability replaces the monk's bonus feats."));
        let drift = find_drift(&entry);
        assert!(drift.is_empty(), "{drift:?}");
        if let TypedFields::Archetype(fields) = &mut entry.entry.fields {
            fields.trades[1].by.clear();
        }
        assert!(!find_drift(&entry).is_empty());
    }

    #[test]
    fn another_unlisted_heading_does_not_state_trades_without_by() {
        // The sentence under a heading that is no scaffold heading is not an unbound sentence,
        // so the trade without `by` is not stated (the check reports the heading).
        let both = vec![
            trade_by(TradeOp::Replaces, "Bravery", &["shattering-strike"]),
            trade_by(TradeOp::Replaces, "Bonus feats", &[]),
        ];
        let drift = find_drift(&with_heading_after_feature("Monk Training", both));
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(
            drift[0].source,
            "trade sentences of the text outside any feature"
        );
    }

    fn single_power(body: Vec<Block>, trades: Vec<cast_iron_schema::Trade>) -> Stored {
        stored(base(
            "ref/domain/test",
            EntryKind::ClassOption,
            TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
                group: "domain".into(),
                trades,
                ..Default::default()
            }),
            body,
        ))
    }

    #[test]
    fn a_single_power_class_option_may_print_rows_or_sentences() {
        let trades = || {
            vec![
                cast_iron_schema::Trade::unresolved(TradeOp::Replaces, "Detect evil"),
                cast_iron_schema::Trade::unresolved(TradeOp::Alters, "Smite evil"),
            ]
        };
        // Today's form: rows.
        let rows = single_power(
            vec![
                paragraph(labelled("Replaces", "Detect evil")),
                paragraph(labelled("Alters", "Smite evil")),
            ],
            trades(),
        );
        assert!(find_drift(&rows).is_empty());
        // The printed form: no heading, no rows, the sentences in the paragraphs.
        let prose = single_power(
            vec![text_block(
                "You gain a bonus on Sense Motive checks. This ability replaces detect evil and alters smite evil.",
            )],
            trades(),
        );
        assert!(find_drift(&prose).is_empty());
        // The same coverage rules: a sentence name without a trade, and a trade without a sentence.
        let extra_sentence = single_power(
            vec![text_block(
                "This ability replaces detect evil and smite evil.",
            )],
            vec![cast_iron_schema::Trade::unresolved(
                TradeOp::Replaces,
                "Detect evil",
            )],
        );
        assert_eq!(find_drift(&extra_sentence).len(), 1);
        let unstated = single_power(
            vec![text_block("This ability replaces detect evil.")],
            trades(),
        );
        assert_eq!(find_drift(&unstated).len(), 1);
        // Trades with neither rows nor sentences are still drift, as they were.
        let neither = single_power(vec![text_block("You gain a bonus.")], trades());
        let drift = find_drift(&neither);
        assert_eq!(drift.len(), 2, "{drift:?}");
        assert!(drift.iter().all(|d| d.source_value.is_empty()), "{drift:?}");
        // No trades and no rows is not drift, with or without prose that mentions the verb.
        let none = single_power(
            vec![text_block("This ability replaces detect evil.")],
            vec![],
        );
        assert!(find_drift(&none).is_empty());
        // A printed row wins over a sentence: the row comparison decides.
        let both = single_power(
            vec![
                paragraph(labelled("Replaces", "Detect evil")),
                text_block("This ability replaces smite evil."),
            ],
            vec![cast_iron_schema::Trade::unresolved(
                TradeOp::Replaces,
                "Detect evil",
            )],
        );
        assert!(find_drift(&both).is_empty());
    }

    #[test]
    fn the_stored_trade_names_read_against_printed_step_lists_and_extra_words() {
        // The trades as the corpus stores them: one per step, a bare name.
        let steps = |paragraph: &str, names: &[&str]| {
            printed_archetype(
                &[("f", paragraph)],
                names
                    .iter()
                    .map(|name| trade_by(TradeOp::Replaces, name, &["f"]))
                    .collect(),
            )
        };
        assert!(
            find_drift(&steps(
                "At 1st level, x. This ability replaces armor training 1, 2, 3, and 4.",
                &[
                    "Armor training 1",
                    "Armor training 2",
                    "Armor training 3",
                    "Armor training 4",
                ]
            ))
            .is_empty()
        );
        assert!(
            find_drift(&steps(
                "At 1st level, x. This ability replaces weapon training 1 and 4.",
                &["Weapon training 1", "Weapon training 4"]
            ))
            .is_empty()
        );
        assert!(
            find_drift(&steps(
                "At 1st level, x. This ability replaces the paladin's spells class feature.",
                &["Spells"]
            ))
            .is_empty()
        );
        assert!(
            find_drift(&steps(
                "At 1st level, x. This ability replaces the nature sense ability.",
                &["Nature sense (Ex)"]
            ))
            .is_empty()
        );
    }

    #[test]
    fn the_printed_wordings_of_a_trade_sentence_agree() {
        let case = |paragraph: &str, trades: Vec<cast_iron_schema::Trade>| {
            let entry = printed_archetype(&[("f", paragraph)], trades);
            find_drift(&entry)
        };
        // Two names in one sentence: one trade each.
        assert!(
            case(
                "At 2nd level, x. This ability replaces armor training and bravery.",
                vec![
                    trade_by(TradeOp::Replaces, "Armor training", &["f"]),
                    trade_by(TradeOp::Replaces, "Bravery (Ex)", &["f"]),
                ]
            )
            .is_empty()
        );
        // Two names, one trade: the second name is a sentence part that no trade covers.
        assert_eq!(
            case(
                "At 2nd level, x. This ability replaces armor training and bravery.",
                vec![trade_by(TradeOp::Replaces, "Armor training", &["f"])]
            )
            .len(),
            1
        );
        // A hyphen-suspended list, as one trade per level.
        assert!(
            case(
                "At 2nd level, x. This ability replaces the 1st-, 2nd- and 3rd-level bonus feats.",
                vec![
                    trade_by(TradeOp::Replaces, "1st-level bonus feat", &["f"]),
                    trade_by(TradeOp::Replaces, "2nd-level bonus feat", &["f"]),
                    trade_by(TradeOp::Replaces, "3rd-level bonus feat", &["f"]),
                ]
            )
            .is_empty()
        );
        // The same list as one trade.
        assert!(
            case(
                "At 2nd level, x. This replaces the 1st-, 2nd- and 3rd-level bonus feats.",
                vec![trade_by(
                    TradeOp::Replaces,
                    "1st-, 2nd- and 3rd-level bonus feats",
                    &["f"]
                )]
            )
            .is_empty()
        );
        // Replaces and alters in one sentence.
        assert!(
            case(
                "At 2nd level, x. This ability replaces bravery and alters armor training.",
                vec![
                    trade_by(TradeOp::Replaces, "Bravery", &["f"]),
                    trade_by(TradeOp::Alters, "Armor training", &["f"]),
                ]
            )
            .is_empty()
        );
        // A feature that trades nothing and prints no trade sentence is fine.
        assert!(case("At 2nd level, x gains a bonus.", vec![]).is_empty());
    }

    #[test]
    fn numbered_steps_read_as_one_trade_per_step_or_one_trade() {
        let paragraph = "At 2nd level, x. This ability replaces armor training 1, 2, 3, and 4.";
        let case = |trades: Vec<cast_iron_schema::Trade>| {
            find_drift(&printed_archetype(&[("f", paragraph)], trades))
        };
        let step = |n: u8| trade_by(TradeOp::Replaces, &format!("Armor training {n}"), &["f"]);
        assert!(case(vec![step(1), step(2), step(3), step(4)]).is_empty());
        assert!(
            case(vec![trade_by(
                TradeOp::Replaces,
                "Armor training 1, 2, 3, and 4",
                &["f"]
            )])
            .is_empty()
        );
        // A missing step is uncovered.
        assert_eq!(case(vec![step(1), step(2), step(3)]).len(), 1);
        // Two steps with "and".
        let two = find_drift(&printed_archetype(
            &[(
                "f",
                "At 5th level, x. This ability alters weapon training 2 and 3.",
            )],
            vec![
                trade_by(TradeOp::Alters, "Weapon training 2", &["f"]),
                trade_by(TradeOp::Alters, "Weapon training 3", &["f"]),
            ],
        ));
        assert!(two.is_empty(), "{two:?}");
    }

    #[test]
    fn a_possessive_before_the_name_is_read_through() {
        let entry = printed_archetype(
            &[(
                "f",
                "At 1st level, x. This ability replaces the rogue's trapfinding.",
            )],
            vec![trade_by(TradeOp::Replaces, "Trapfinding", &["f"])],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn a_bare_pronoun_sentence_counts_only_when_it_names_a_stored_trade() {
        // Savage Warrior: "These replace the normal attack roll bonus and AC penalty for
        // charging." is prose about charging, not a trade sentence.
        let prose = "At 1st level, x. These replace the normal attack roll bonus and AC penalty for charging.";
        let entry = printed_archetype(&[("f", prose)], vec![]);
        assert!(find_drift(&entry).is_empty());
        // A stored trade that the pronoun sentence names makes it a trade sentence.
        let named = "At 1st level, x. This replaces bravery.";
        let covered = printed_archetype(
            &[("f", named)],
            vec![trade_by(TradeOp::Replaces, "Bravery", &["f"])],
        );
        assert!(find_drift(&covered).is_empty());
        // ... and then it must be covered like any other.
        let wider = "At 1st level, x. This replaces bravery and armor training.";
        let short = printed_archetype(
            &[("f", wider)],
            vec![trade_by(TradeOp::Replaces, "Bravery", &["f"])],
        );
        assert_eq!(find_drift(&short).len(), 1);
    }

    #[test]
    fn a_class_name_before_the_feature_and_a_shared_tail_read_against_stored_trades() {
        let alchemist = "At 1st level, x. This ability otherwise functions as and replaces the standard alchemist bomb class feature.";
        let entry = printed_archetype(
            &[("f", alchemist)],
            vec![trade_by(TradeOp::Replaces, "Bombs", &["f"])],
        );
        assert!(find_drift(&entry).is_empty());
        let mercies =
            "At 3rd level, x. This ability replaces the paladin's 3rd-level and 9th-level mercies.";
        let entry = printed_archetype(
            &[("f", mercies)],
            vec![
                trade_by(TradeOp::Replaces, "3rd-level mercy", &["f"]),
                trade_by(TradeOp::Replaces, "9th-level mercy", &["f"]),
            ],
        );
        assert!(find_drift(&entry).is_empty());
        let hexes = "At 1st level, x. This ability replaces hexes.";
        let entry = printed_archetype(
            &[("f", hexes)],
            vec![trade_by(TradeOp::Replaces, "Hex", &["f"])],
        );
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn a_negated_or_other_subject_sentence_is_not_a_trade() {
        for paragraph in [
            "At 2nd level, x. This ability does not replace bravery.",
            "At 2nd level, x. This ability never replaces bravery.",
            "At 2nd level, x. This bonus replaces the usual bonus.",
            "At 2nd level, x. The ability replaces bravery.",
        ] {
            let entry = printed_archetype(&[("f", paragraph)], vec![]);
            assert!(find_drift(&entry).is_empty(), "{paragraph}");
        }
    }

    #[test]
    fn feature_heads_after_a_multi_publication_cut_are_still_read() {
        // Two publications and `###` heads: the row form ends the normal part at the first
        // `###`, but the feature form reads every head.
        let mut entry = printed_archetype_at(
            3,
            &[
                ("a", "At 1st level, a. This ability replaces bravery."),
                ("b", "At 3rd level, b. This ability alters armor training."),
            ],
            vec![
                trade_by(TradeOp::Replaces, "Bravery", &["a"]),
                trade_by(TradeOp::Alters, "Armor training", &["b"]),
            ],
        );
        entry.entry.source_ids = vec!["core-rulebook".into(), "other-book".into()];
        assert!(find_drift(&entry).is_empty());
        // The second feature's trade is checked: drop it and the sentence is uncovered.
        if let TypedFields::Archetype(fields) = &mut entry.entry.fields {
            fields.trades.truncate(1);
        }
        assert_eq!(find_drift(&entry).len(), 1);
        // Another source's block (a `**Sources:**` paragraph) still ends the part.
        entry.entry.body.push(sources("Another Book pg 3"));
        entry
            .entry
            .body
            .push(text_block("This ability replaces something else."));
        assert_eq!(find_drift(&entry).len(), 1);
    }

    #[test]
    fn an_entry_with_features_does_not_read_the_rows() {
        // A `**Replaces:**` row beside features is not compared: the printed sentence is.
        let mut entry = printed_archetype(
            &[("shattering-strike", SHATTERING)],
            vec![trade_by(
                TradeOp::Replaces,
                "Bravery",
                &["shattering-strike"],
            )],
        );
        entry
            .entry
            .body
            .push(paragraph(labelled("Replaces", "Something else")));
        assert!(find_drift(&entry).is_empty());
    }

    #[test]
    fn an_entry_without_features_keeps_the_row_comparison() {
        let entry = printed_archetype(&[], vec![]);
        let entry = Stored {
            entry: {
                let mut e = entry.entry;
                e.body = vec![paragraph(labelled("Replaces", "Bravery"))];
                e
            },
            ..entry
        };
        let drift = find_drift(&entry);
        assert_eq!(drift.len(), 1, "{drift:?}");
        assert_eq!(drift[0].field, "replaces");
    }
}
