//! Drift between the typed `[fields]` and the body text that states the same value.
//!
//! An entry keeps some values twice: once in `[fields]`, where search and facets read
//! them, and once in the body, where the reader sees them (a spell's `Range` row, a
//! creature's `AC` row, a feat's `**Prerequisites:**` line). The corpus keeps both
//! copies. This check compares each pair that both state and reports every
//! disagreement, naming both places, so a hand edit to one copy is noticed.

use crate::load::Stored;
use cast_iron_schema::headings::plain_text;
use cast_iron_schema::{Block, ChallengeRating, Inline, RuleEntry, StatRow, TypedFields};
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
fn is_sources_line(block: &Block) -> bool {
    let Block::Paragraph { content, .. } = block else {
        return false;
    };
    matches!(
        content.first(),
        Some(Inline::Strong { c }) if plain_text(c).trim() == "Sources:"
    )
}

/// The normal feat's part of the body: everything before the first block that starts a
/// second source. A source block starts at a level-3 heading that carries its own
/// `**Sources:**` line, or at any later bare `**Sources:**` line (an Elephant-in-the-Room
/// tail, a second printing). Those blocks hold another publication's version of the feat
/// (a `### Mythic` or `### Stamina` block, an Elephant-in-the-Room replacement, …). A base
/// feat's `[fields]` describe the normal feat, so those blocks are never compared.
///
/// A `###` heading without its own `**Sources:**` line organizes the normal feat's own
/// text (options, examples, a table) and stays in the comparison.
fn normal_part(body: &[Block]) -> &[Block] {
    let mut seen_sources = false;
    for (index, block) in body.iter().enumerate() {
        if is_sources_line(block) {
            // The entry's first `**Sources:**` line opens the normal part. A later bare
            // one starts another source's block.
            if seen_sources {
                return &body[..index];
            }
            seen_sources = true;
            continue;
        }
        let is_subsection = matches!(block, Block::Heading { level: 3, .. });
        if is_subsection && body.get(index + 1).is_some_and(is_sources_line) {
            return &body[..index];
        }
    }
    body
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
    text.lines()
        .position(|line| line.starts_with(&prefix))
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
) -> Option<Pair> {
    let stated = labelled_texts(body, label);
    let source_value = stated.first()?.clone();
    let marker = format!("**{label}:**");
    // A normal feat may state its label more than once in its own part (a variant
    // statement), so the field agrees when any of those statements matches.
    let agree = stated
        .iter()
        .any(|statement| normalize(value) == normalize(statement));
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
        .any(|statement| match ChallengeRating::parse(statement) {
            Some(parsed) => parsed == value,
            None => normalize(&value.display()) == normalize(statement),
        });
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

/// The six ability scores of an `Ability Scores` row: `Str 3, Dex 16, ...`.
fn parse_abilities(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| {
            part.split_whitespace()
                .last()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

fn creature_pairs(entry: &RuleEntry, text: &str, rows: &[&StatRow]) -> Vec<Pair> {
    let TypedFields::Creature(fields) = &entry.fields else {
        return vec![];
    };
    let mut pairs = Vec::new();
    if let Some(cr) = fields.cr {
        pairs.extend(row_pair(text, rows, "cr", &cr.display(), &["CR"]));
        pairs.extend(challenge_rating_pair(text, &entry.body, "cr", cr));
    }
    for (field, value, labels) in [
        ("size", &fields.size, &["Size"][..]),
        ("alignment", &fields.alignment, &["Alignment"][..]),
        ("ac", &fields.ac, &["AC"][..]),
        ("hp", &fields.hp, &["hp"][..]),
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
    if let Some(kind) = &fields.creature_type {
        // The row may add the subtypes in parentheses.
        let with_subtypes = if fields.subtypes.is_empty() {
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
            source_line: body_line(text, |line| line.starts_with("Ability Scores:")),
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

fn item_pairs(entry: &RuleEntry, text: &str, rows: &[&StatRow]) -> Vec<Pair> {
    let TypedFields::Item(fields) = &entry.fields else {
        return vec![];
    };
    let mut pairs = Vec::new();
    for (field, value, labels) in [
        ("price", &fields.price, &["Price"][..]),
        ("weight", &fields.weight, &["Weight"][..]),
        ("slot", &fields.slot, &["Slot"][..]),
        ("aura", &fields.aura, &["Aura"][..]),
        ("caster_level", &fields.caster_level, &["CL"][..]),
    ] {
        if let Some(value) = value {
            pairs.extend(row_pair(text, rows, field, value, labels));
        }
    }
    pairs
}

fn feat_pairs(entry: &RuleEntry, text: &str) -> Vec<Pair> {
    let TypedFields::Feat(fields) = &entry.fields else {
        return vec![];
    };
    // A base feat's fields describe the normal feat, never a source subsection.
    let body = normal_part(&entry.body);
    let mut pairs = Vec::new();
    for (field, value, label) in [
        ("prerequisites", &fields.prerequisites, "Prerequisites"),
        ("benefit", &fields.benefit, "Benefit"),
        ("normal", &fields.normal, "Normal"),
        ("special", &fields.special, "Special"),
    ] {
        if let Some(value) = value {
            match text_pair(text, body, field, value, label) {
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
    let body = normal_part(&entry.body);
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

/// Every pair in the entry that states a value in both places and disagrees.
pub fn find_drift(stored: &Stored) -> Vec<Disagreement> {
    let entry = &stored.entry;
    let rows = stat_rows(&entry.body);
    let pairs = match &entry.fields {
        TypedFields::Spell(_) => spell_pairs(entry, &stored.text, &rows),
        TypedFields::Creature(_) => creature_pairs(entry, &stored.text, &rows),
        TypedFields::Item(_) => item_pairs(entry, &stored.text, &rows),
        TypedFields::Feat(_) => feat_pairs(entry, &stored.text),
        TypedFields::Archetype(fields) => {
            replaces_alters_pairs(entry, &stored.text, &fields.replaces, &fields.alters)
        }
        TypedFields::ClassOption(fields) => {
            replaces_alters_pairs(entry, &stored.text, &fields.replaces, &fields.alters)
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
            section: vec![],
            parent: None,
            body,
            fields,
            tags: vec![],
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

    fn archetype(replaces: &[&str], alters: &[&str]) -> TypedFields {
        TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
            replaces: replaces.iter().map(|value| value.to_string()).collect(),
            alters: alters.iter().map(|value| value.to_string()).collect(),
        })
    }

    fn class_option(replaces: &[&str], alters: &[&str]) -> TypedFields {
        TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
            group: "paladin-oath".into(),
            replaces: replaces.iter().map(|value| value.to_string()).collect(),
            alters: alters.iter().map(|value| value.to_string()).collect(),
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
}
