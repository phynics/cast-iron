//! Cross-file checks over a loaded corpus.

use crate::books::{
    BOOKS_DIR, BookLayout, PAGE_ORDER_TITLE, UNPAGED_TITLE, is_clean_page, is_reserved_chapter,
};
use crate::config::SPLIT_BOOKS_FORMAT;
use crate::drift::{Disagreement, find_drift, printed_replaces_alters};
use crate::entry::entry_path;
use crate::load::Corpus;
use cast_iron_schema::headings::{headings_in_order, keyed_headings, plain_text, slugify};
use cast_iron_schema::{
    Block, BookNode, BookTree, EntryId, EntryKind, Inline, Relation, RelationKind, RelationLabel,
    RuleEntry, TypedFields,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The creature types of Pathfinder First Edition, in the corpus spelling.
///
/// The client shows a `creature_type` facet value as written, so one type must have exactly one
/// spelling. Case variants such as `Humanoid` would otherwise appear as separate filter values.
pub const CREATURE_TYPES: [&str; 13] = [
    "aberration",
    "animal",
    "construct",
    "dragon",
    "fey",
    "humanoid",
    "magicalBeast",
    "monstrousHumanoid",
    "ooze",
    "outsider",
    "plant",
    "undead",
    "vermin",
];

/// What a check found. Every problem fails the run; warnings do not.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub problems: Vec<String>,
    /// Suspicious content that needs review but does not fail the run. Third-party sharing is
    /// a warning until the corpus has no unclassified case.
    pub warnings: Vec<String>,
    /// Values that `[fields]` and the body both state and that disagree. Each one is also
    /// listed in `problems`.
    pub drift: Vec<Disagreement>,
}

impl Report {
    /// Disagreements per entry kind, by kind slug.
    pub fn drift_by_kind(&self) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for item in &self.drift {
            *counts.entry(item.kind.as_str()).or_default() += 1;
        }
        counts
    }

    /// The drift list as a report file: counts per kind, then one line per disagreement.
    pub fn drift_report(&self) -> String {
        let mut out = String::from("# Corpus drift report\n\n");
        out.push_str(
            "Values that `[fields]` and the body both state and that disagree. Each line names both places.\n\n",
        );
        out.push_str("| Kind | Disagreements |\n|---|---|\n");
        for (kind, count) in self.drift_by_kind() {
            out.push_str(&format!("| {kind} | {count} |\n"));
        }
        out.push_str(&format!("| **total** | **{}** |\n\n", self.drift.len()));
        let mut sorted: Vec<&Disagreement> = self.drift.iter().collect();
        sorted.sort_by(|a, b| (&a.kind, &a.id, a.field).cmp(&(&b.kind, &b.id, b.field)));
        for item in sorted {
            out.push_str(&item.describe());
            out.push('\n');
        }
        out
    }
}

/// The 1-based line of the first line of `text` that holds `needle`, or 1.
pub fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map_or(1, |index| index + 1)
}

/// Every 1-based line of `text` that holds `needle`, in file order.
fn lines_of(text: &str, needle: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains(needle))
        .map(|(index, _)| index + 1)
        .collect()
}

/// Every heading of an entry with its URL anchor key and plain text.
pub fn heading_keys(entry: &RuleEntry) -> Vec<(String, String)> {
    keyed_headings(&entry.body)
        .into_iter()
        .filter_map(|(block, key)| {
            let key = key?;
            let Block::Heading { content, .. } = block else {
                return None;
            };
            Some((key, cast_iron_schema::headings::plain_text(content)))
        })
        .collect()
}

/// The body part of an entry file, with the 1-based line number it starts on.
pub(crate) fn body_text(text: &str) -> (usize, &str) {
    let mut offset = 0;
    let mut fences = 0;
    for line in text.split_inclusive('\n') {
        offset += line.len();
        if line.trim_end() == "+++" {
            fences += 1;
            if fences == 2 {
                break;
            }
        }
    }
    let start = text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
    (start, &text[offset..])
}

/// Upstream inline tokens the retired import left in body text: `&`, one or more ASCII
/// uppercase letters, and `&`, so `&L&` and `&FN&` are caught. Lowercase HTML-style entities
/// (`&times&`, `&quot&`, ...) and mixed-case markers (`&Next1&`) are deliberately not matched.
fn upstream_markup_tokens(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'&' => {
                let start = i;
                let mut end = i + 1;
                while end < bytes.len() && bytes[end].is_ascii_uppercase() {
                    end += 1;
                }
                if end > i + 1 && end < bytes.len() && bytes[end] == b'&' {
                    out.push((line, text[start..=end].to_string()));
                    i = end + 1;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Well-formed HTML entity references in a front-matter string: `&mdash;`, `&times;`, `&#39;`,
/// `&#x2014;`. The retired import left them in typed fields, where a reader or a filter would see
/// the literal characters. Broken fragments (`&times&`) are not matched.
fn entity_references(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            i += 1;
            continue;
        }
        let rest = &text[i + 1..];
        let (name, hex) = match rest.strip_prefix('#') {
            Some(number) => match number.strip_prefix(['x', 'X']) {
                Some(digits) => (digits, true),
                None => (number, false),
            },
            None => (rest, false),
        };
        let len = name
            .bytes()
            .take_while(|b| match (rest.starts_with('#'), hex) {
                (true, true) => b.is_ascii_hexdigit(),
                (true, false) => b.is_ascii_digit(),
                (false, _) => b.is_ascii_alphanumeric(),
            })
            .count();
        let valid_start =
            rest.starts_with('#') || name.starts_with(|c: char| c.is_ascii_alphabetic());
        if len > 0 && valid_start && name[len..].starts_with(';') {
            let prefix = rest.len() - name.len();
            let end = i + 1 + prefix + len + 1;
            out.push(text[i..end].to_string());
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// Every inline run of `blocks`, in document order. Footnote headings are skipped: one holds its
/// own label, which is not a marker. Other headings count, because a marker can sit in a
/// section title (`### Removed Feats^[1](…#footnote-1)^`).
fn body_inlines<'a>(blocks: &'a [Block], out: &mut Vec<&'a [Inline]>) {
    for block in blocks {
        match block {
            Block::Heading {
                level: 6,
                anchor: Some(anchor),
                ..
            } if anchor.starts_with("footnote-") => {}
            Block::Heading { content, .. } => out.push(content),
            Block::EntryLink { .. } => {}
            Block::Paragraph { content, .. } => out.push(content),
            Block::List { items, .. } => {
                for item in items {
                    body_inlines(item, out);
                }
            }
            Block::Quote { content } => body_inlines(content, out),
            Block::StatBlock { rows, .. } => out.extend(rows.iter().map(|row| &row.value[..])),
            Block::Table(table) => {
                out.push(&table.caption);
                for row in table.head.iter().chain(&table.body).chain(&table.foot) {
                    out.extend(row.cells.iter().map(|cell| &cell.content[..]));
                }
            }
        }
    }
}

/// Collects what can mark a footnote: the text of each superscript, and the anchor of each link.
fn footnote_markers(
    items: &[Inline],
    labels: &mut BTreeSet<String>,
    anchors: &mut BTreeSet<String>,
) {
    for item in items {
        match item {
            Inline::Sup { c } => {
                labels.insert(plain_text(c).trim().to_string());
                footnote_markers(c, labels, anchors);
            }
            Inline::Link { anchor, c, .. } => {
                if let Some(anchor) = anchor {
                    anchors.insert(anchor.clone());
                }
                footnote_markers(c, labels, anchors);
            }
            Inline::Em { c } | Inline::Strong { c } | Inline::Sub { c } => {
                footnote_markers(c, labels, anchors)
            }
            Inline::Text { .. } | Inline::Break => {}
        }
    }
}

/// Report a footnote block whose marker is not in the entry body (an orphan) as a problem.
///
/// A footnote is a level-six heading with a `footnote-` anchor. Its marker is a superscript with
/// the heading label (`^B^`) or a link to the anchor (`^[1](id#footnote-1)^`). The audit in
/// `docs/research/editorial-footnotes-audit.md` found 140 orphans.
fn check_orphan_footnotes(stored: &crate::load::Stored, report: &mut Report) {
    let entry = &stored.entry;
    let footnotes: Vec<(&str, String)> = headings_in_order(&entry.body)
        .into_iter()
        .filter_map(|block| match block {
            Block::Heading {
                level: 6,
                anchor: Some(anchor),
                content,
            } if anchor.starts_with("footnote-") => {
                Some((anchor.as_str(), plain_text(content).trim().to_string()))
            }
            _ => None,
        })
        .collect();
    if footnotes.is_empty() {
        return;
    }
    let mut runs = Vec::new();
    body_inlines(&entry.body, &mut runs);
    let (mut labels, mut anchors) = (BTreeSet::new(), BTreeSet::new());
    for run in runs {
        footnote_markers(run, &mut labels, &mut anchors);
    }
    for (anchor, label) in footnotes {
        if anchors.contains(anchor) || labels.contains(&label) {
            continue;
        }
        let line = line_of(&stored.text, &format!("{{#{anchor}}}"));
        report.problems.push(format!(
            "{}:{line}: footnote `{label}` (`#{anchor}`) has no marker in the entry body: add `^{label}^` or a link to `#{anchor}`, or remove the note",
            stored.path.display()
        ));
    }
}

/// Converter residue such as `\|X~` in `text`, with its 1-based line: a pipe after a backslash,
/// one to three letters or digits, and a tilde. The importer left it where it did not split a
/// multi-note `::mfn`. The writer escapes the tilde, so a backslash may sit before it.
fn footnote_residue(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut line = 1;
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            line += 1;
            continue;
        }
        if *byte != b'|' || i == 0 || bytes[i - 1] != b'\\' {
            continue;
        }
        let mut end = i + 1;
        while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
            end += 1;
        }
        if !(1..=3).contains(&(end - i - 1)) {
            continue;
        }
        if bytes.get(end) == Some(&b'\\') {
            end += 1;
        }
        if bytes.get(end) == Some(&b'~') {
            out.push((line, text[i - 1..=end].to_string()));
        }
    }
    out
}

/// The 1-based lines of `text` that hold a left-over `p!` import marker.
///
/// The upstream import wrote a literal `p!` where a paragraph or list break had been, so the
/// marker now reads as text: `At 6th level: p!These bonuses +2.` A marker sits at a word
/// boundary and directly before a capital letter. The same sequence inside a word, as in `Help!`
/// or `35,150 gp!`, is real text and stays out.
fn p_markers(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut out: Vec<usize> = Vec::new();
    let mut line = 1;
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            line += 1;
            continue;
        }
        if *byte != b'p' || bytes.get(i + 1) != Some(&b'!') {
            continue;
        }
        if i > 0 {
            let before = bytes[i - 1];
            if before.is_ascii_alphanumeric() || before == b'_' {
                continue;
            }
        }
        if bytes.get(i + 2).is_some_and(u8::is_ascii_uppercase) && out.last() != Some(&line) {
            out.push(line);
        }
    }
    out
}

/// Every string that `[fields]` shows to a reader, paired with the key that holds it.
///
/// Handles and numbers are not display text: `prerequisite_feats`, `levels[].level`, `cr`,
/// and `alternate` stay out. Every remaining string is scanned, including pair labels and
/// values and the `levels[].class` names.
fn field_display_texts(fields: &TypedFields) -> Vec<(&'static str, &str)> {
    fn optional<'a>(
        out: &mut Vec<(&'static str, &'a str)>,
        key: &'static str,
        value: &'a Option<String>,
    ) {
        if let Some(value) = value {
            out.push((key, value.as_str()));
        }
    }
    fn list<'a>(out: &mut Vec<(&'static str, &'a str)>, key: &'static str, values: &'a [String]) {
        out.extend(values.iter().map(|value| (key, value.as_str())));
    }
    fn pairs<'a>(
        out: &mut Vec<(&'static str, &'a str)>,
        key: &'static str,
        values: &'a [(String, String)],
    ) {
        for (label, value) in values {
            out.push((key, label.as_str()));
            out.push((key, value.as_str()));
        }
    }

    let mut out = Vec::new();
    match fields {
        TypedFields::Spell(fields) => {
            out.push(("school", fields.school.as_str()));
            list(&mut out, "subschools", &fields.subschools);
            list(&mut out, "descriptors", &fields.descriptors);
            out.extend(
                fields
                    .levels
                    .iter()
                    .map(|level| ("levels", level.class.as_str())),
            );
            optional(&mut out, "casting_time", &fields.casting_time);
            optional(&mut out, "components", &fields.components);
            optional(&mut out, "range", &fields.range);
            pairs(&mut out, "effects", &fields.effects);
            optional(&mut out, "duration", &fields.duration);
            optional(&mut out, "saving_throw", &fields.saving_throw);
            optional(&mut out, "spell_resistance", &fields.spell_resistance);
        }
        TypedFields::Feat(fields) => {
            list(&mut out, "feat_types", &fields.feat_types);
            optional(&mut out, "prerequisites", &fields.prerequisites);
            optional(&mut out, "benefit", &fields.benefit);
            optional(&mut out, "normal", &fields.normal);
            optional(&mut out, "special", &fields.special);
        }
        TypedFields::Creature(fields) => {
            optional(&mut out, "creature_type", &fields.creature_type);
            list(&mut out, "subtypes", &fields.subtypes);
            optional(&mut out, "size", &fields.size);
            optional(&mut out, "alignment", &fields.alignment);
            optional(&mut out, "ac", &fields.ac);
            optional(&mut out, "hp", &fields.hp);
            optional(&mut out, "fortitude", &fields.fortitude);
            optional(&mut out, "reflex", &fields.reflex);
            optional(&mut out, "will", &fields.will);
            optional(&mut out, "melee", &fields.melee);
            optional(&mut out, "ranged", &fields.ranged);
            optional(&mut out, "special_attacks", &fields.special_attacks);
            optional(&mut out, "special_qualities", &fields.special_qualities);
            if let Some(abilities) = &fields.abilities {
                out.extend(abilities.iter().map(|value| ("abilities", value.as_str())));
            }
        }
        TypedFields::Item(fields) => {
            optional(&mut out, "category", &fields.category);
            optional(&mut out, "price", &fields.price);
            optional(&mut out, "weight", &fields.weight);
            optional(&mut out, "slot", &fields.slot);
            optional(&mut out, "aura", &fields.aura);
            optional(&mut out, "caster_level", &fields.caster_level);
            pairs(&mut out, "attributes", &fields.attributes);
        }
        TypedFields::Class(fields) => {
            optional(&mut out, "class_type", &fields.class_type);
            optional(&mut out, "hit_die", &fields.hit_die);
            optional(&mut out, "skill_ranks", &fields.skill_ranks);
            optional(&mut out, "class_skills", &fields.class_skills);
        }
        TypedFields::Trait(fields) => {
            out.push(("category", fields.category.as_str()));
            optional(&mut out, "detail", &fields.detail);
        }
        TypedFields::Archetype(_) | TypedFields::ClassOption(_) => {
            out.extend(
                fields
                    .trades()
                    .iter()
                    .map(|trade| (trade.op.slug(), trade.printed.as_str())),
            );
        }
        TypedFields::RacialTrait(_) | TypedFields::None => {}
    }
    out
}

/// A display string in an entry and the place in the file that holds it.
enum DisplayText<'a> {
    Title(&'a str),
    Tag(&'a str),
    Field(&'static str, &'a str),
}

impl<'a> DisplayText<'a> {
    fn value(&self) -> &'a str {
        match self {
            DisplayText::Title(value) | DisplayText::Tag(value) => value,
            DisplayText::Field(_, value) => value,
        }
    }

    /// What the finding calls the place, so a title or tag does not read as `[fields]`.
    fn place(&self) -> String {
        match self {
            DisplayText::Title(_) => "the entry title".into(),
            DisplayText::Tag(_) => "the entry tags".into(),
            DisplayText::Field(key, _) => format!("`[fields]` key `{key}`"),
        }
    }
}

/// Every string in an entry that a reader sees, with the place that holds it. Title, tags,
/// and typed fields share the rule; handles and numbers stay out.
fn entry_display_texts(entry: &RuleEntry) -> Vec<DisplayText<'_>> {
    let mut out = vec![DisplayText::Title(&entry.title)];
    out.extend(entry.tags.iter().map(|tag| DisplayText::Tag(tag)));
    out.extend(
        field_display_texts(&entry.fields)
            .into_iter()
            .map(|(key, value)| DisplayText::Field(key, value)),
    );
    out
}

pub fn fields_kind(fields: &TypedFields) -> &'static str {
    match fields {
        TypedFields::Spell(_) => "spell",
        TypedFields::Feat(_) => "feat",
        TypedFields::Creature(_) => "creature",
        TypedFields::Item(_) => "item",
        TypedFields::Class(_) => "class",
        TypedFields::Trait(_) => "trait",
        TypedFields::RacialTrait(_) => "racial_trait",
        TypedFields::Archetype(_) => "archetype",
        TypedFields::ClassOption(_) => "class_option",
        TypedFields::None => "none",
    }
}

pub fn fields_match(kind: EntryKind, fields: &TypedFields) -> bool {
    match fields {
        TypedFields::Spell(_) => kind == EntryKind::Spell,
        TypedFields::Feat(_) => kind == EntryKind::Feat,
        TypedFields::Creature(_) => kind == EntryKind::Creature,
        TypedFields::Item(_) => kind == EntryKind::Item,
        TypedFields::Class(_) => matches!(kind, EntryKind::Class | EntryKind::PrestigeClass),
        TypedFields::Trait(_) => kind == EntryKind::Trait,
        TypedFields::RacialTrait(_) => kind == EntryKind::RacialTrait,
        TypedFields::Archetype(_) => kind == EntryKind::Archetype,
        TypedFields::ClassOption(_) => kind == EntryKind::ClassOption,
        TypedFields::None => matches!(
            kind,
            EntryKind::Rule
                | EntryKind::Race
                | EntryKind::Skill
                | EntryKind::Affliction
                | EntryKind::Trap
                | EntryKind::Archetype
        ),
    }
}

/// Validates an entry's or a publication's optional `aon` path.
///
/// The value is a path relative to `https://aonprd.com/`. An entry names a `*Display.aspx`
/// page with `ItemName`, such as `FeatDisplay.aspx?ItemName=Power%20Attack`. A publication
/// names `SourceDisplay.aspx` with `FixedSource`, as in
/// `SourceDisplay.aspx?FixedSource=Core%20Rulebook`. A scheme, host, leading slash, fragment,
/// or another page type is rejected.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AonKind {
    Entry,
    Publication,
}

fn aon_problem(value: &str, kind: AonKind) -> Option<String> {
    if value.is_empty() {
        return Some("is empty".into());
    }
    if value.starts_with("//")
        || value.starts_with('/')
        || value.contains("://")
        || value.starts_with("\\\\")
    {
        return Some(
            "must be a path relative to https://aonprd.com/, with no scheme or host".into(),
        );
    }
    if value.contains('\\') {
        return Some("must use `/` separators, not `\\`".into());
    }
    if value.contains('#') {
        return Some("must not carry a fragment".into());
    }
    if value.contains(|ch: char| ch.is_control() || ch.is_whitespace()) {
        return Some("must not hold spaces or control characters; percent-encode them".into());
    }
    let Some((page, query)) = value.split_once('?') else {
        return Some(
            "must name a `*Display.aspx` page and a query, such as `FeatDisplay.aspx?ItemName=Power%20Attack`"
                .into(),
        );
    };
    if query.is_empty() || query.contains('?') {
        return Some("must carry exactly one non-empty query string".into());
    }
    let Some(stem) = page.strip_suffix("Display.aspx") else {
        return Some("must name a `*Display.aspx` page, such as `FeatDisplay.aspx`".into());
    };
    if stem.is_empty() || !stem.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Some("must name a `*Display.aspx` page, such as `FeatDisplay.aspx`".into());
    }
    if kind == AonKind::Publication && page != "SourceDisplay.aspx" {
        return Some("must name `SourceDisplay.aspx` for a publication".into());
    }
    let mut seen = BTreeSet::new();
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            return Some(format!("has a query part `{pair}` that is not `key=value`"));
        };
        // AoN names an entry differently by page type: feats, spells and most records use
        // `ItemName`, archetypes use `FixedName`, and some item pages use `FinalName`.
        let allowed: &[&str] = if kind == AonKind::Publication {
            &["FixedSource"]
        } else {
            &["ItemName", "FixedName", "FinalName"]
        };
        if !allowed.contains(&key) {
            return Some(format!(
                "uses query key `{key}`; a {kind_name} must use {expected}",
                kind_name = if kind == AonKind::Publication {
                    "publication"
                } else {
                    "entry"
                },
                expected = allowed
                    .iter()
                    .map(|key| format!("`{key}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !seen.insert(key) {
            return Some(format!("repeats the query key `{key}`"));
        }
        if !valid_aon_query_value(value) {
            return Some(format!(
                "has a value for `{key}` that must be non-empty and percent-encoded"
            ));
        }
    }
    None
}

/// A query value holds unreserved and sub-delimiter characters, and every `%` starts a
/// two-digit escape. A raw space or non-ASCII byte is not valid.
fn valid_aon_query_value(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
            continue;
        }
        let allowed = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b':'
                    | b'@'
                    | b'/'
            );
        if !allowed {
            return false;
        }
        index += 1;
    }
    true
}

/// A publication's optional `aon` source page. The entry check covers the entry field.
fn check_publication_aon(corpus: &Corpus, report: &mut Report) {
    let text = std::fs::read_to_string(corpus.dir.join("publications.toml")).unwrap_or_default();
    for publication in &corpus.publications {
        let Some(aon) = &publication.aon else {
            continue;
        };
        let at = format!(
            "publications.toml:{}",
            line_of(&text, &format!("aon = {}", crate::tomlutil::string(aon)))
        );
        if let Some(problem) = aon_problem(aon, AonKind::Publication) {
            report.problems.push(format!(
                "{at}: publication `{}` has `aon` that {problem}",
                publication.id
            ));
        }
    }
}

/// Every `resolved` row of `exclusions.toml` names entries that exist: a superseded exclusion
/// must point at the records that now ship. (The reader has already refused a row that matches
/// no `excluded` or `rejected` row of the file, one that matches several, a repeated row, and
/// an empty one.)
fn check_resolved_exclusions(corpus: &Corpus, ids: &BTreeSet<&str>, report: &mut Report) {
    if corpus.exclusions.resolved.is_empty() {
        return;
    }
    let text = std::fs::read_to_string(corpus.dir.join("exclusions.toml")).unwrap_or_default();
    for row in &corpus.exclusions.resolved {
        for entry in &row.entries {
            if !ids.contains(entry.as_str()) {
                report.problems.push(format!(
                    "exclusions.toml:{}: resolved row for `{}` names entry `{entry}`, which is not in the corpus",
                    line_of(&text, &crate::tomlutil::string(entry)),
                    row.path
                ));
            }
        }
    }
}

/// The licensing gate: every publication states a copyright and its legal basis is valid.
/// OGL publications must have a Section 15 notice or a reviewed `notice_pending` entry.
///
/// A publication is covered when the text of some `section_15` notice in `legal.toml` is part of
/// its `copyright`, which is how the import joined them, or when its title is in
/// `notice_pending`. A source without a publication license row keeps the legacy OGL default.
/// A problem names the source file, line, and publication id.
fn check_publication_notices(corpus: &Corpus, report: &mut Report) {
    let text = std::fs::read_to_string(corpus.dir.join("publications.toml")).unwrap_or_default();
    let mut license_ids: BTreeSet<&str> = corpus
        .legal
        .licenses
        .iter()
        .map(|license| license.id.as_str())
        .collect();
    if !corpus.legal.community_use_notice.is_empty() {
        license_ids.insert("legal#community-use");
    }
    let mut declared_publications = BTreeSet::new();
    for basis in &corpus.legal.publication_licenses {
        if !declared_publications.insert(basis.publication_id.as_str()) {
            report.problems.push(format!(
                "legal.toml: publication `{}` has more than one license basis",
                basis.publication_id
            ));
        }
        match corpus
            .publications
            .iter()
            .find(|publication| publication.id == basis.publication_id)
        {
            None => report.problems.push(format!(
                "legal.toml: publication license basis names unknown publication `{}`",
                basis.publication_id
            )),
            Some(publication) if publication.title != basis.title => report.problems.push(format!(
                "legal.toml: publication license basis title `{}` does not match `{}` for `{}`",
                basis.title, publication.title, basis.publication_id
            )),
            Some(_) => {}
        }
        if basis.license_refs.is_empty() {
            report.problems.push(format!(
                "legal.toml: publication `{}` has an empty license basis",
                basis.publication_id
            ));
        }
        for license in &basis.license_refs {
            if !license_ids.contains(license.as_str()) {
                report.problems.push(format!(
                    "legal.toml: publication `{}` has unknown licence `{license}`",
                    basis.publication_id
                ));
            }
        }
    }
    for publication in &corpus.publications {
        let at = format!(
            "publications.toml:{}",
            line_of(&text, &format!("id = \"{}\"", publication.id))
        );
        let copyright = publication.copyright.as_deref().map_or("", str::trim);
        if copyright.is_empty() {
            report.problems.push(format!(
                "{at}: publication `{}` has no copyright",
                publication.id
            ));
            continue;
        }
        // Contradiction check only: a third-party marker beside a copyright line that names
        // Paizo cannot be right. The corpus still has Paizo books whose notice is pending
        // (#34) and whose copyright does not name Paizo, so a missing marker is not a
        // problem: requiring one is a later change, after those notices are settled.
        if publication.third_party && copyright.to_lowercase().contains("paizo") {
            report.problems.push(format!(
                "{at}: publication `{}` is marked `third_party = true` but its copyright names Paizo",
                publication.id
            ));
        }
        let pending = corpus.legal.notice_pending.contains(&publication.title);
        let noticed = corpus.legal.section_15.iter().any(|notice| {
            let notice = notice.notice.trim();
            !notice.is_empty() && copyright.contains(notice)
        });
        let basis = corpus
            .legal
            .publication_licenses
            .iter()
            .find(|basis| basis.publication_id == publication.id);
        let uses_ogl = basis.is_none_or(|basis| {
            basis
                .license_refs
                .iter()
                .any(|license| license == "ogl-1.0a")
        });
        if uses_ogl && !pending && !noticed {
            report.problems.push(format!(
                "{at}: publication `{}` ({}) is covered by no section_15 notice in legal.toml and is not in notice_pending",
                publication.id, publication.title
            ));
        }
    }
}

/// The optional series fields (#159): one of three kinds, and the fields each kind needs.
///
/// Adventure Paths carry the path name and the volume within it. Modules are ordered by
/// publication date and carry no series fields. Society scenarios carry a season and, when it
/// is present, a scenario number of the form `<season>-NN`. Fields outside `series_kind` are
/// rejected so a row cannot half-join a grouping.
fn check_publication_series(corpus: &Corpus, report: &mut Report) {
    let text = std::fs::read_to_string(corpus.dir.join("publications.toml")).unwrap_or_default();
    for publication in &corpus.publications {
        let at = format!(
            "publications.toml:{}",
            line_of(&text, &format!("id = \"{}\"", publication.id))
        );
        let series = publication.series.as_deref().map(str::trim).unwrap_or("");
        let number = publication
            .series_number
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
        let Some(kind) = publication.series_kind else {
            if !series.is_empty() || !number.is_empty() || publication.season.is_some() {
                report.problems.push(format!(
                    "{at}: publication `{}` has series fields but no `series_kind`",
                    publication.id
                ));
            }
            continue;
        };
        match kind {
            cast_iron_schema::SeriesKind::AdventurePath => {
                if series.is_empty() {
                    report.problems.push(format!(
                        "{at}: publication `{}` is `series_kind = \"adventure-path\"` but has no `series`",
                        publication.id
                    ));
                }
                if number.is_empty() {
                    report.problems.push(format!(
                        "{at}: publication `{}` is `series_kind = \"adventure-path\"` but has no `series_number`",
                        publication.id
                    ));
                } else if number.parse::<u32>().map_or(true, |volume| volume == 0) {
                    report.problems.push(format!(
                        "{at}: publication `{}` has `series_number = {number:?}`, which is not a positive Adventure Path volume",
                        publication.id
                    ));
                }
                if publication.season.is_some() {
                    report.problems.push(format!(
                        "{at}: publication `{}` is `series_kind = \"adventure-path\"` but carries `season`",
                        publication.id
                    ));
                }
            }
            cast_iron_schema::SeriesKind::Module => {
                if !series.is_empty() || !number.is_empty() || publication.season.is_some() {
                    report.problems.push(format!(
                        "{at}: publication `{}` is `series_kind = \"module\"` but carries `series`, `series_number` or `season`; modules are ordered by publication_date",
                        publication.id
                    ));
                }
            }
            cast_iron_schema::SeriesKind::Society => {
                match publication.season {
                    None => report.problems.push(format!(
                        "{at}: publication `{}` is `series_kind = \"society\"` but has no `season`",
                        publication.id
                    )),
                    Some(0) => report.problems.push(format!(
                        "{at}: publication `{}` has `season = 0`, which is not a Pathfinder Society season",
                        publication.id
                    )),
                    Some(_) => {}
                }
                if !series.is_empty() {
                    report.problems.push(format!(
                        "{at}: publication `{}` is `series_kind = \"society\"` but carries `series`; a season names the grouping",
                        publication.id
                    ));
                }
                if !number.is_empty() {
                    let matches = publication.season.is_some_and(|season| {
                        number
                            .strip_prefix(&format!("{season}-"))
                            .is_some_and(|rest| {
                                rest.len() == 2 && rest.bytes().all(|b| b.is_ascii_digit())
                            })
                    });
                    if !matches {
                        report.problems.push(format!(
                            "{at}: publication `{}` has `series_number = {number:?}`, which is not `<season>-NN` for its season",
                            publication.id
                        ));
                    }
                }
            }
        }
    }
}

/// Runs every cross-file check.
pub fn check(corpus: &Corpus) -> Report {
    let mut report = Report::default();
    let publication_ids: BTreeSet<&str> = corpus
        .publications
        .iter()
        .map(|publication| publication.id.as_str())
        .collect();
    let mut license_ids: BTreeSet<&str> = corpus
        .legal
        .licenses
        .iter()
        .map(|license| license.id.as_str())
        .collect();
    if !corpus.legal.community_use_notice.is_empty() {
        license_ids.insert("legal#community-use");
    }
    let ids: BTreeSet<&str> = corpus.entries.keys().map(String::as_str).collect();
    let keys: BTreeMap<&str, Vec<(String, String)>> = corpus
        .entries
        .iter()
        .map(|(id, stored)| (id.as_str(), heading_keys(&stored.entry)))
        .collect();

    // `race_group` arrived after the pinned corpus, so a library that has no value anywhere is
    // an older corpus and passes. Once one race carries it, every entry of kind race must,
    // whatever its id: a race without a group is missing from every Races list facet.
    let races_have_groups = corpus
        .entries
        .values()
        .any(|stored| stored.entry.kind == EntryKind::Race && stored.entry.race_group.is_some());

    check_publication_notices(corpus, &mut report);
    check_publication_series(corpus, &mut report);
    check_publication_aon(corpus, &mut report);
    check_resolved_exclusions(corpus, &ids, &mut report);

    for (id, stored) in &corpus.entries {
        let entry = &stored.entry;
        let path = entry_path(&entry.id);
        if stored.path != path {
            report.problems.push(format!(
                "{}: entry id `{}` belongs at `{}`, not `{}`",
                stored.path.display(),
                entry.id,
                path.display(),
                stored.path.display()
            ));
        }
        let at = |needle: &str| format!("{}:{}", path.display(), line_of(&stored.text, needle));
        if let Some(aon) = &entry.aon
            && let Some(problem) = aon_problem(aon, AonKind::Entry)
        {
            report.problems.push(format!(
                "{}: `aon` {problem}",
                at(&format!("aon = {}", crate::tomlutil::string(aon)))
            ));
        }
        if !fields_match(entry.kind, &entry.fields) {
            report.problems.push(format!(
                "{}: entry kind `{}` does not match `[fields]` kind `{}`",
                at("kind"),
                entry.kind.slug(),
                fields_kind(&entry.fields)
            ));
        }
        let (body_start, body) = body_text(&stored.text);
        for line in p_markers(body) {
            report.problems.push(format!(
                "{}:{}: left-over import marker `p!`. Remove it and restore the printed wording.",
                path.display(),
                body_start + line - 1
            ));
        }
        for (line, residue) in footnote_residue(body) {
            report.problems.push(format!(
                "{}:{}: footnote converter residue `{residue}`: split it into one note per marker",
                path.display(),
                body_start + line - 1
            ));
        }
        check_orphan_footnotes(stored, &mut report);
        for (line, token) in upstream_markup_tokens(body) {
            report.problems.push(format!(
                "{}:{}: left-over upstream token `{token}`. Convert it to plain text or a typed link.",
                path.display(),
                body_start + line - 1
            ));
        }
        // The writer puts each display string on one line. When the same value appears more
        // than once, consume the next matching line so every occurrence reports its own
        // place instead of the first one over and over.
        let mut matched: BTreeMap<String, usize> = BTreeMap::new();
        for text in entry_display_texts(entry) {
            let value = text.value();
            let tokens = upstream_markup_tokens(value);
            let entities = entity_references(value);
            if tokens.is_empty() && entities.is_empty() {
                continue;
            }
            let needle = crate::tomlutil::string(value);
            let lines = lines_of(&stored.text, &needle);
            let next = matched.entry(needle.clone()).or_default();
            let line = lines
                .get(*next)
                .copied()
                .unwrap_or_else(|| line_of(&stored.text, &needle));
            *next += 1;
            let place = text.place();
            for (_, token) in tokens {
                report.problems.push(format!(
                    "{}:{line}: left-over upstream token `{token}` in {place}. Convert it to plain text or a typed link.",
                    path.display()
                ));
            }
            for entity in entities {
                report.problems.push(format!(
                    "{}:{line}: HTML entity reference `{entity}` in {place}. Write the character, or omit the key when it means no value.",
                    path.display()
                ));
            }
        }
        // A race without a group is missing from the Races list facets, so it looks as if it
        // does not exist. A carried chapter that repeats a typed race also shows up twice.
        if races_have_groups && entry.kind == EntryKind::Race && entry.race_group.is_none() {
            report.problems.push(format!(
                "{}: a race needs `race_group` ({})",
                path.display(),
                cast_iron_schema::RaceGroup::ALL
                    .iter()
                    .map(|group| group.slug())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if entry.kind != EntryKind::Race
            && let Some(group) = entry.race_group
        {
            report.problems.push(format!(
                "{}: `race_group = \"{}\"` is only valid on a race",
                path.display(),
                group.slug()
            ));
        }
        if let TypedFields::Creature(fields) = &entry.fields
            && let Some(creature_type) = &fields.creature_type
            && !CREATURE_TYPES.contains(&creature_type.as_str())
        {
            report.problems.push(format!(
                "{}: creature_type `{creature_type}` is not one of the PF1e creature types: {}",
                at(&format!("creature_type = \"{creature_type}\"")),
                CREATURE_TYPES.join(", ")
            ));
        }
        for source in &entry.source_ids {
            if !publication_ids.contains(source.as_str()) {
                report
                    .problems
                    .push(format!("{}: unknown publication `{source}`", at(source)));
            }
        }
        for citation in &entry.sources {
            let needle = format!("publication = \"{}\"", citation.publication);
            if !publication_ids.contains(citation.publication.as_str()) {
                report.problems.push(format!(
                    "{}: `[[sources]]`: unknown publication `{}`",
                    at(&needle),
                    citation.publication
                ));
            }
            if !entry
                .source_ids
                .iter()
                .any(|source| source == &citation.publication)
            {
                report.problems.push(format!(
                    "{}: `[[sources]]`: publication `{}` is not listed in `source_ids`",
                    at(&needle),
                    citation.publication
                ));
            }
        }
        // `[[expanded_in]]` credits a book that expands the entry without printing it, so the
        // book must exist, must not be a member book, and any `via` entry must itself be
        // printed there.
        for (index, expansion) in entry.expanded_in.iter().enumerate() {
            let needle = format!("publication = \"{}\"", expansion.publication);
            if entry.expanded_in[..index].contains(expansion) {
                report.problems.push(format!(
                    "{}: `[[expanded_in]]`: duplicate row for publication `{}`; drop the repeated row",
                    at(&needle),
                    expansion.publication
                ));
            }
            if !publication_ids.contains(expansion.publication.as_str()) {
                report.problems.push(format!(
                    "{}: `[[expanded_in]]`: unknown publication `{}`",
                    at(&needle),
                    expansion.publication
                ));
            }
            if entry
                .source_ids
                .iter()
                .any(|source| source == &expansion.publication)
            {
                report.problems.push(format!(
                    "{}: `[[expanded_in]]`: publication `{}` is listed in `source_ids`; a book that prints the entry is a member, so drop the `[[expanded_in]]` row",
                    at(&needle),
                    expansion.publication
                ));
            }
            if let Some(via) = &expansion.via {
                let via_needle = format!("via = \"{via}\"");
                match corpus.entries.get(via.as_str()) {
                    None => report.problems.push(format!(
                        "{}: `[[expanded_in]]`: `via` entry `{via}` does not exist",
                        at(&via_needle)
                    )),
                    Some(target) => {
                        if !target
                            .entry
                            .source_ids
                            .iter()
                            .any(|source| source == &expansion.publication)
                        {
                            report.problems.push(format!(
                                "{}: `[[expanded_in]]`: `via` entry `{via}` is not in publication `{}`; its `source_ids` must include it",
                                at(&via_needle),
                                expansion.publication
                            ));
                        }
                    }
                }
            }
        }
        for license in &entry.license_refs {
            if !license_ids.contains(license.as_str()) {
                report
                    .problems
                    .push(format!("{}: unknown licence `{license}`", at(license)));
            }
        }
        if let Some(parent) = &entry.parent
            && !ids.contains(parent.as_str())
        {
            report.problems.push(format!(
                "{}: unknown parent `{parent}`",
                at(parent.as_str())
            ));
        }
        for link in &entry.links {
            let target = link.target.as_str();
            if !ids.contains(target) {
                report
                    .problems
                    .push(format!("{}: unknown link target `{target}`", at(target)));
                continue;
            }
            let Some(anchor) = &link.anchor else { continue };
            let available = &keys[target];
            if available.iter().any(|(key, _)| key == anchor) {
                continue;
            }
            let needle = format!("]({target}#{anchor})");
            report.problems.push(format!(
                "{}: the link to `{target}#{anchor}` points at a heading that does not exist. {}",
                at(&needle),
                anchor_fix(&corpus.entries[target].path, anchor, available)
            ));
        }
        if let TypedFields::Feat(fields) = &entry.fields {
            for feat in &fields.prerequisite_feats {
                if !ids.contains(feat.as_str()) {
                    report.problems.push(format!(
                        "{}: unknown prerequisite feat `{feat}`",
                        at(feat.as_str())
                    ));
                }
            }
        }
        for relation in &entry.relations {
            let target = relation.target.as_str();
            if !ids.contains(target) {
                report.problems.push(format!(
                    "{}: unknown relation target `{target}`",
                    at(target)
                ));
            }
        }
        let _ = id;
        for item in find_drift(stored) {
            report.problems.push(item.describe());
            report.drift.push(item);
        }
    }

    check_relations(corpus, &mut report);
    check_third_party_sharing(corpus, &mut report);
    check_option_groups(corpus, &ids, &mut report);
    check_replaces_features(corpus, &keys, &mut report);
    crate::check_trades::check_trades(corpus, &keys, &mut report);
    crate::check_powers::check_class_option_powers(corpus, &mut report);
    check_publication_suffixes(corpus, &ids, &mut report);

    let publication_ids: BTreeSet<&str> = corpus
        .publications
        .iter()
        .map(|publication| publication.id.as_str())
        .collect();
    check_book_layout(corpus, &mut report);
    for tree in &corpus.books.books {
        let file = BookFile::of(corpus, &tree.source);
        if !publication_ids.contains(tree.source.as_str()) {
            report.problems.push(format!(
                "{}:{}: book `{}` has no row in publications.toml. A book is a printed publication, so add its row with a copyright and a Section 15 notice, or delete the book",
                file.label,
                book_line(file.text, &tree.source),
                tree.source
            ));
        }
        if corpus.layout == BookLayout::Split {
            for chapter in &tree.chapters {
                if is_reserved_chapter(&chapter.title) {
                    report.problems.push(format!(
                        "{}:{}: a curated book may not hold a `{}` chapter; the compiler builds `{PAGE_ORDER_TITLE}`, `{UNPAGED_TITLE}` and `{}` from the [[sources]] pages. Delete the chapter",
                        file.label,
                        line_of(file.text, &format!("title = \"{}\"", chapter.title)),
                        chapter.title,
                        cast_iron_schema::books::CATCH_ALL_TITLE
                    ));
                }
            }
        }
        let mut placed = BTreeSet::new();
        for chapter in &tree.chapters {
            check_book_node(
                corpus,
                &file,
                tree,
                &chapter.title,
                chapter,
                &ids,
                &keys,
                &mut placed,
                &mut report,
            );
        }
    }
    if let Some(aliases) = &corpus.aliases {
        for entry in &aliases.entries {
            if !ids.contains(entry.target.as_str()) {
                report.problems.push(format!(
                    "aliases.toml:1: alias target `{}` is not an entry",
                    entry.target
                ));
            }
        }
    }
    report
}

/// The fix to suggest for a link anchor that no heading in the target has.
fn anchor_fix(target_path: &Path, anchor: &str, available: &[(String, String)]) -> String {
    if available.is_empty() {
        return format!(
            "`{}` has no headings. Remove `#{anchor}` from the link, or add a heading `## Title {{#{anchor}}}` to that file.",
            target_path.display()
        );
    }
    let shown: Vec<String> = available
        .iter()
        .take(8)
        .map(|(key, text)| format!("`{key}` (\"{text}\")"))
        .collect();
    let more = available.len().saturating_sub(8);
    format!(
        "`{}` has these headings: {}{}. Change the link to one of them, or add `{{#{anchor}}}` to the heading it should reach.",
        target_path.display(),
        shown.join(", "),
        if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        }
    )
}

/// Warn when third-party content can escape its book.
///
/// The corpus has no per-entry statement of which paragraph came from which book. Two reliable
/// proxies stand in for one:
///
/// - an entry with no third-party source must not link an entry whose sources are all
///   third-party, because turning the third-party book off would hide the link target;
/// - an entry that mixes third-party and Paizo sources warns. The split form is an `addon` or
///   `revision` entry with only the third-party source, beside an unchanged Paizo base. A mixed
///   entry is either a missed split or a shared-printing claim that a reviewer must confirm.
fn check_third_party_sharing(corpus: &Corpus, report: &mut Report) {
    let third_party: BTreeSet<&str> = corpus
        .publications
        .iter()
        .filter(|publication| publication.third_party)
        .map(|publication| publication.id.as_str())
        .collect();
    if third_party.is_empty() {
        return;
    }
    let is_third_party = |source: &String| third_party.contains(source.as_str());
    let third_party_only = |entry: &RuleEntry| {
        !entry.source_ids.is_empty() && entry.source_ids.iter().all(is_third_party)
    };

    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let has_third_party = entry.source_ids.iter().any(is_third_party);
        let has_paizo = entry
            .source_ids
            .iter()
            .any(|source| !is_third_party(source));

        if !has_third_party {
            for link in &entry.links {
                let Some(target) = corpus.entries.get(link.target.as_str()) else {
                    continue;
                };
                if !third_party_only(&target.entry) {
                    continue;
                }
                let line = line_of(&stored.text, &format!("]({}", link.target));
                report.warnings.push(format!(
                    "{}:{line}: body link to `{}` is third-party only ({}); a non-third-party entry must not link content that its book setting can hide",
                    stored.path.display(),
                    link.target,
                    target.entry.source_ids.join(", ")
                ));
            }
        }

        if has_third_party && has_paizo {
            let line = line_of(&stored.text, "source_ids");
            report.warnings.push(format!(
                "{}:{line}: entry mixes third-party and Paizo sources ({}); move the third-party text into an `addon` or `revision` entry, or confirm a shared printing",
                stored.path.display(),
                entry.source_ids.join(", ")
            ));
        }
    }
}

fn check_relations(corpus: &Corpus, report: &mut Report) {
    let mut kinds: BTreeMap<&str, EntryKind> = BTreeMap::new();
    let mut variant_edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut revision_keys: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut revision_ids: BTreeSet<&str> = BTreeSet::new();

    for stored in corpus.entries.values() {
        kinds.insert(stored.entry.id.as_str(), stored.entry.kind);
        if stored
            .entry
            .relations
            .iter()
            .any(|relation| relation.kind == RelationKind::Revision)
        {
            revision_ids.insert(stored.entry.id.as_str());
        }
    }

    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let at = |target: &str| {
            format!(
                "{}:{}",
                stored.path.display(),
                line_of(&stored.text, target)
            )
        };
        let mut variants = 0usize;
        for relation in &entry.relations {
            let line = at(relation.target.as_str());
            if let Some(problem) = relation_label_problem(relation) {
                report.problems.push(format!("{line}: {problem}"));
            }
            let Some(target_kind) = kinds.get(relation.target.as_str()).copied() else {
                continue;
            };
            match relation.kind {
                RelationKind::Variant => {
                    variants += 1;
                    // TODO(slice 5, #143): error when a spell body holds another spell's title
                    // as a heading.
                    if entry.kind != EntryKind::Spell || target_kind != EntryKind::Spell {
                        report.problems.push(format!(
                            "{line}: a `variant` relation must go from a spell to a spell, but `{}` is a {} and `{}` is a {}",
                            entry.id,
                            entry.kind.slug(),
                            relation.target,
                            target_kind.slug()
                        ));
                    }
                    variant_edges
                        .entry(entry.id.to_string())
                        .or_default()
                        .push(relation.target.to_string());
                }
                RelationKind::Revision => {
                    let target = &corpus.entries[relation.target.as_str()];
                    if target
                        .entry
                        .relations
                        .iter()
                        .any(|row| row.kind == RelationKind::Revision)
                    {
                        report.problems.push(format!(
                            "{line}: a `revision` cannot point at `{}`, which is itself a revision",
                            relation.target
                        ));
                    }
                    if target_kind != entry.kind {
                        report.problems.push(format!(
                            "{line}: a `revision` must keep the target's kind, but `{}` is a {} and `{}` is a {}",
                            entry.id,
                            entry.kind.slug(),
                            relation.target,
                            target_kind.slug()
                        ));
                    }
                    if entry
                        .source_ids
                        .iter()
                        .any(|source| target.entry.source_ids.contains(source))
                    {
                        report.problems.push(format!(
                            "{line}: a `revision` of `{}` shares a book with it; a revision must come from another publication",
                            relation.target
                        ));
                    }
                    let label = relation.label;
                    if let Some(label @ (RelationLabel::Removed | RelationLabel::Merged)) = label
                        && !fields_are_empty(&entry.fields)
                    {
                        report.problems.push(format!(
                            "{line}: a `{}` revision must have empty `[fields]`",
                            label.slug()
                        ));
                    }
                    if label == Some(RelationLabel::Merged)
                        && !entry
                            .links
                            .iter()
                            .any(|link| kinds.get(link.target.as_str()) == Some(&EntryKind::Feat))
                    {
                        report.problems.push(format!(
                            "{line}: a `merged` revision must link the feat it merges into"
                        ));
                    }
                    if entry.source_ids.len() == 1 {
                        revision_keys
                            .entry((relation.target.to_string(), entry.source_ids[0].to_string()))
                            .or_default()
                            .push(entry.id.to_string());
                    }
                }
                RelationKind::Addon => {
                    if relation.label == Some(RelationLabel::Mythic) {
                        // The rule is about feats only. A spell or rule holder (the mythic-spell
                        // add-ons of slice 3) is not required to carry Mythic in `feat_types`.
                        if entry.kind == EntryKind::Feat {
                            let mythic = matches!(
                                &entry.fields,
                                TypedFields::Feat(fields)
                                    if fields.feat_types.iter().any(|kind| kind == "Mythic")
                            );
                            if !mythic {
                                report.problems.push(format!(
                                    "{line}: a `mythic` add-on must be a feat whose `feat_types` include `Mythic`"
                                ));
                            }
                        }
                        // TODO(slice 2, #178): error when a base still holds a `Mythic <Title>`
                        // heading while an add-on targets it.
                    }
                }
                RelationKind::Part | RelationKind::Archetype | RelationKind::Prerequisite => {}
            }
        }
        if variants > 1 {
            report.problems.push(format!(
                "{}: entry `{}` holds {variants} `variant` relations; at most one is allowed",
                stored.path.display(),
                entry.id
            ));
        }
        for link in &entry.links {
            if revision_ids.contains(link.target.as_str()) {
                report.problems.push(format!(
                    "{}: the body links `{}`, which is a revision",
                    stored.path.display(),
                    link.target
                ));
            }
        }
        if let TypedFields::Feat(fields) = &entry.fields {
            for feat in &fields.prerequisite_feats {
                if revision_ids.contains(feat.as_str()) {
                    report.problems.push(format!(
                        "{}: `prerequisite_feats` names `{feat}`, which is a revision",
                        stored.path.display()
                    ));
                }
            }
        }
    }

    for (target, entries) in &revision_keys {
        for extra in entries.iter().skip(1) {
            let path = EntryId::parse(extra.clone())
                .map(|id| entry_path(&id))
                .unwrap_or_else(|_| std::path::PathBuf::from(extra));
            report.problems.push(format!(
                "{}: `{}` is a second revision of `{}` in publication `{}`",
                path.display(),
                extra,
                target.0,
                target.1
            ));
        }
    }

    check_variant_cycles(&variant_edges, report);
}

/// The label vocabulary is closed: a relation may carry only the labels its kind declares.
fn relation_label_problem(relation: &Relation) -> Option<String> {
    let label = relation.label;
    if relation.kind.allows(label) {
        return None;
    }
    if relation.kind == RelationKind::Addon {
        return Some(
            "an `addon` relation needs exactly one of `mythic`, `stamina` or `changes`".into(),
        );
    }
    Some(match label {
        Some(label) => format!(
            "`{}` is not a valid label on a `{}` relation",
            label.slug(),
            relation.kind.slug()
        ),
        None => format!("a `{}` relation needs a label", relation.kind.slug()),
    })
}

fn fields_are_empty(fields: &TypedFields) -> bool {
    match fields {
        TypedFields::Spell(fields) => {
            fields.school.is_empty()
                && fields.subschools.is_empty()
                && fields.descriptors.is_empty()
                && fields.levels.is_empty()
                && fields.casting_time.is_none()
                && fields.components.is_none()
                && fields.range.is_none()
                && fields.effects.is_empty()
                && fields.duration.is_none()
                && fields.saving_throw.is_none()
                && fields.spell_resistance.is_none()
        }
        TypedFields::Feat(fields) => {
            fields.feat_types.is_empty()
                && fields.prerequisites.is_none()
                && fields.prerequisite_feats.is_empty()
                && fields.benefit.is_none()
                && fields.normal.is_none()
                && fields.special.is_none()
        }
        TypedFields::Creature(fields) => {
            fields.cr.is_none()
                && fields.creature_type.is_none()
                && fields.subtypes.is_empty()
                && fields.size.is_none()
                && fields.alignment.is_none()
                && fields.ac.is_none()
                && fields.hp.is_none()
                && fields.fortitude.is_none()
                && fields.reflex.is_none()
                && fields.will.is_none()
                && fields.melee.is_none()
                && fields.ranged.is_none()
                && fields.special_attacks.is_none()
                && fields.special_qualities.is_none()
                && fields.abilities.is_none()
        }
        TypedFields::Item(fields) => {
            fields.category.is_none()
                && fields.price.is_none()
                && fields.weight.is_none()
                && fields.slot.is_none()
                && fields.aura.is_none()
                && fields.caster_level.is_none()
                && fields.attributes.is_empty()
        }
        TypedFields::Class(fields) => {
            fields.class_type.is_none()
                && fields.hit_die.is_none()
                && fields.skill_ranks.is_none()
                && fields.class_skills.is_none()
        }
        TypedFields::Trait(fields) => fields.category.is_empty() && fields.detail.is_none(),
        TypedFields::RacialTrait(fields) => !fields.alternate,
        TypedFields::Archetype(fields) => fields.trades.is_empty(),
        TypedFields::ClassOption(fields) => fields.group.is_empty() && fields.trades.is_empty(),
        TypedFields::None => true,
    }
}

fn check_variant_cycles(edges: &BTreeMap<String, Vec<String>>, report: &mut Report) {
    fn visit<'a>(
        node: &'a str,
        edges: &'a BTreeMap<String, Vec<String>>,
        state: &mut BTreeMap<&'a str, bool>,
        path: &mut Vec<&'a str>,
        report: &mut Report,
    ) {
        match state.get(node) {
            Some(true) => {
                // The node is already on the current path: the path from its first occurrence
                // back to itself is the cycle, in order.
                let start = path.iter().position(|id| *id == node).unwrap_or(0);
                let mut cycle: Vec<&str> = path[start..].to_vec();
                cycle.push(node);
                let shown: Vec<String> = cycle.iter().map(|id| format!("`{id}`")).collect();
                report.problems.push(format!(
                    "`variant` relations form a cycle: {}; remove one variant edge",
                    shown.join(" → ")
                ));
                return;
            }
            Some(false) => return,
            None => {}
        }
        state.insert(node, true);
        path.push(node);
        for target in edges.get(node).map(Vec::as_slice).unwrap_or_default() {
            visit(target, edges, state, path, report);
        }
        path.pop();
        state.insert(node, false);
    }
    let mut state = BTreeMap::new();
    let mut path = Vec::new();
    for start in edges.keys() {
        visit(start, edges, &mut state, &mut path, report);
    }
}

fn check_option_groups(corpus: &Corpus, ids: &BTreeSet<&str>, report: &mut Report) {
    let mut used: BTreeMap<&str, usize> = BTreeMap::new();
    for stored in corpus.entries.values() {
        let TypedFields::ClassOption(fields) = &stored.entry.fields else {
            continue;
        };
        let at = format!(
            "{}:{}",
            stored.path.display(),
            line_of(&stored.text, &format!("group = \"{}\"", fields.group))
        );
        if !corpus.option_groups.groups.contains_key(&fields.group) {
            report.problems.push(format!(
                "{at}: class-option group `{}` is not declared in option-groups.toml",
                fields.group
            ));
        }
        *used.entry(fields.group.as_str()).or_default() += 1;
    }
    let text = std::fs::read_to_string(corpus.dir.join("option-groups.toml")).unwrap_or_default();
    for (slug, group) in &corpus.option_groups.groups {
        let line = text
            .lines()
            .position(|line| line.trim() == format!("[groups.{slug}]"))
            .map_or(1, |index| index + 1);
        for class in &group.classes {
            if !ids.contains(class.as_str()) {
                report.problems.push(format!(
                    "option-groups.toml:{line}: group `{slug}` names unknown class `{class}`"
                ));
            }
        }
        if !used.contains_key(slug.as_str()) {
            report.problems.push(format!(
                "option-groups.toml:{line}: group `{slug}` holds no class_option entries"
            ));
        }
    }
}

/// The base class entries for the `replaces_features` / `alters_features` anchors. An
/// archetype names its class through the `archetype` relation; a class option names the
/// classes of its group in `option-groups.toml`.
pub fn base_class_ids<'a>(corpus: &'a Corpus, entry: &'a RuleEntry) -> Vec<&'a str> {
    match &entry.fields {
        TypedFields::Archetype(_) => entry
            .relations
            .iter()
            .filter(|relation| relation.kind == RelationKind::Archetype)
            .map(|relation| relation.target.as_str())
            .collect(),
        TypedFields::ClassOption(fields) => corpus
            .option_groups
            .groups
            .get(&fields.group)
            .map(|group| group.classes.iter().map(String::as_str).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// A printed feature name without its ability tag: `Bravery (Ex)` becomes `Bravery`, and
/// `Arcane Bond (Ex or Sp)` becomes `Arcane Bond`.
pub(crate) fn strip_ability_tag(name: &str) -> &str {
    let name = name.trim();
    let Some(open) = name.rfind('(') else {
        return name;
    };
    if !name.ends_with(')') {
        return name;
    }
    let inner = &name[open + 1..name.len() - 1];
    let mut saw_tag = false;
    for token in inner.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '/' | ';')) {
        let token = token.trim().to_ascii_lowercase();
        if token.is_empty() {
            continue;
        }
        if !matches!(token.as_str(), "ex" | "su" | "sp" | "or" | "and") {
            return name;
        }
        saw_tag = true;
    }
    if saw_tag {
        name[..open].trim_end_matches(['/', ' ']).trim_end()
    } else {
        name
    }
}

/// The heading anchor a printed feature name would have, before checking that the base
/// class entry actually has it.
pub(crate) fn feature_key(name: &str) -> String {
    slugify(strip_ability_tag(name))
}

/// The first few anchors of `anchors`, for an error message.
fn anchor_hint(anchors: &BTreeSet<&str>) -> String {
    let shown: Vec<&str> = anchors.iter().copied().take(8).collect();
    format!(
        "{}{}",
        shown.join(", "),
        if anchors.len() > shown.len() {
            " and more"
        } else {
            ""
        }
    )
}

/// Check the typed trades of archetypes and class options against the base class entries.
///
/// A trade that names a feature must name a heading anchor of a base class entry, and a `class`
/// on it must be one of the base classes and hold the anchor. A printed name that confidently
/// resolves to another anchor than the trade's is a warning, because a printed name that no
/// heading matches is left unresolved instead of guessed. A corpus-format-1 list that did not line
/// up with its printed list keeps its anchors; they are checked and compared as they were before
/// trades. The compiler (`content-compiler`) separately settles which class an anchor belongs to
/// when several base classes hold it.
fn check_replaces_features(
    corpus: &Corpus,
    keys: &BTreeMap<&str, Vec<(String, String)>>,
    report: &mut Report,
) {
    let format_1 = corpus.config.format == 1;
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let (trades, legacy) = match &entry.fields {
            TypedFields::Archetype(fields) => (&fields.trades, &fields.legacy_features),
            TypedFields::ClassOption(fields) => (&fields.trades, &fields.legacy_features),
            _ => continue,
        };
        if trades.is_empty() && legacy.is_empty() {
            continue;
        }
        let path = stored.path.display();
        if format_1 && crate::trades::needs_format_2(trades) {
            report.problems.push(format!(
                "{path}:{}: [fields] this entry has a trade with `class`, `levels` or `power`, a feature that is not the slug of its printed name, or trades that interleave replaces and alters, which corpus format 1 cannot write; migrate the corpus to format 2",
                line_of(&stored.text, "[fields]")
            ));
        }
        let classes = base_class_ids(corpus, entry);
        let anchors_in = |class: &str| -> BTreeSet<&str> {
            keys.get(class)
                .map(|headings| headings.iter().map(|(key, _)| key.as_str()).collect())
                .unwrap_or_default()
        };
        let mut anchors: BTreeSet<&str> = BTreeSet::new();
        for class in &classes {
            anchors.extend(anchors_in(class));
        }
        let class_list = if classes.is_empty() {
            "no base class entry".to_string()
        } else {
            classes
                .iter()
                .map(|class| format!("`{class}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let has_features = trades.iter().any(|trade| trade.feature.is_some()) || !legacy.is_empty();
        if has_features && classes.is_empty() {
            report.problems.push(format!(
                "{path}:{}: [fields] a trade names a feature, but this entry has no base class entry",
                line_of(&stored.text, "[fields]")
            ));
            continue;
        }
        for trade in trades {
            let Some(feature) = &trade.feature else {
                continue;
            };
            let at = format!(
                "{path}:{}",
                line_of(&stored.text, &format!("\"{}\"", trade.printed))
            );
            let label = trade.op.slug();
            if let Some(class) = &feature.class {
                if !classes.contains(&class.as_str()) {
                    report.problems.push(format!(
                        "{at}: [fields] the {label} trade `{}` names class `{class}`, which is not a base class of this entry; the base classes are {class_list}",
                        trade.printed
                    ));
                    continue;
                }
                if !anchors_in(class.as_str()).contains(feature.anchor.as_str()) {
                    report.problems.push(format!(
                        "{at}: [fields] the {label} trade `{}` has feature `{}`, which is not a heading anchor in `{class}`. Use one of: {}",
                        trade.printed,
                        feature.anchor,
                        anchor_hint(&anchors_in(class.as_str()))
                    ));
                    continue;
                }
            } else if !anchors.contains(feature.anchor.as_str()) {
                report.problems.push(format!(
                    "{at}: [fields] the {label} trade `{}` has feature `{}`, which is not a heading anchor in {class_list}. Use one of: {}",
                    trade.printed,
                    feature.anchor,
                    anchor_hint(&anchors)
                ));
                continue;
            }
            let resolved = feature_key(&trade.printed);
            if feature.power.is_none()
                && feature.levels.is_empty()
                && anchors.contains(resolved.as_str())
                && resolved != feature.anchor
            {
                report.warnings.push(format!(
                    "{at}: [fields] the {label} trade `{}` has feature `{}`, but the printed name resolves to `{resolved}`",
                    trade.printed, feature.anchor
                ));
            }
        }
        // Anchors that a format-1 list kept because it did not line up with the printed list.
        let (printed_replaces, printed_alters) = printed_replaces_alters(entry);
        for (field, label, kept, printed) in [
            (
                "replaces_features",
                "Replaces",
                &legacy.replaces,
                printed_replaces,
            ),
            ("alters_features", "Alters", &legacy.alters, printed_alters),
        ] {
            if kept.is_empty() {
                continue;
            }
            let at = format!("{path}:{}", line_of(&stored.text, &format!("{field} = ")));
            for key in kept {
                if !anchors.contains(key.as_str()) {
                    report.problems.push(format!(
                        "{at}: [fields] {field} key `{key}` is not a heading anchor in {class_list}. Use one of: {}",
                        anchor_hint(&anchors)
                    ));
                }
            }
            let resolved: Vec<String> = printed
                .iter()
                .map(|name| feature_key(name))
                .filter(|key| anchors.contains(key.as_str()))
                .collect();
            if kept != &resolved {
                report.warnings.push(format!(
                    "{at}: [fields] {field} = {kept:?} disagrees with the `**{label}:**` line; the confident matches in print order are {resolved:?}"
                ));
            }
        }
    }
}

/// A valid publication suffix is one id segment.
fn valid_suffix(suffix: &str) -> bool {
    !suffix.contains('/') && EntryId::parse(suffix).is_ok()
}

fn publication_suffix<'a>(corpus: &'a Corpus, entry: &RuleEntry) -> Result<&'a str, String> {
    if entry.source_ids.len() != 1 {
        return Err(format!(
            "`{}` needs exactly one publication to take its id suffix",
            entry.id
        ));
    }
    let source = entry.source_ids[0].as_str();
    corpus
        .publications
        .iter()
        .find(|publication| publication.id == source)
        .and_then(|publication| publication.suffix.as_deref())
        .ok_or_else(|| {
            format!(
                "publication `{source}` has no `suffix` in publications.toml, so `{}` cannot take one",
                entry.id
            )
        })
}

fn check_publication_suffixes(corpus: &Corpus, ids: &BTreeSet<&str>, report: &mut Report) {
    let text = std::fs::read_to_string(corpus.dir.join("publications.toml")).unwrap_or_default();
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    let mut known: BTreeSet<String> = ["mythic".to_string(), "stamina".to_string()]
        .into_iter()
        .collect();
    for publication in &corpus.publications {
        let Some(suffix) = publication.suffix.as_deref() else {
            continue;
        };
        let line = text
            .lines()
            .position(|row| row.trim() == format!("suffix = \"{suffix}\""))
            .map_or(1, |index| index + 1);
        if !valid_suffix(suffix) {
            report.problems.push(format!(
                "publications.toml:{line}: publication `{}` has suffix `{suffix}`, which is not one id segment",
                publication.id
            ));
        }
        if let Some(earlier) = seen.insert(suffix, &publication.id) {
            report.problems.push(format!(
                "publications.toml:{line}: publications `{earlier}` and `{}` share the suffix `{suffix}`",
                publication.id
            ));
        }
        known.insert(suffix.to_string());
    }

    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let line = |target: &str| {
            format!(
                "{}:{}",
                stored.path.display(),
                line_of(&stored.text, target)
            )
        };
        for relation in &entry.relations {
            let expected = match (relation.kind, relation.label) {
                (RelationKind::Addon, Some(RelationLabel::Mythic)) => {
                    // The published carried twins keep their ids and still become the mythic
                    // entries, so they are exempt from nesting under the base.
                    if entry.id.as_str().starts_with("ref/carried/") {
                        continue;
                    }
                    Some("mythic")
                }
                (RelationKind::Addon, Some(RelationLabel::Stamina)) => Some("stamina"),
                (RelationKind::Addon, Some(RelationLabel::Changes))
                | (RelationKind::Revision, _) => match publication_suffix(corpus, entry) {
                    Ok(suffix) => Some(suffix),
                    Err(problem) => {
                        report
                            .problems
                            .push(format!("{}: {problem}", line(relation.target.as_str())));
                        None
                    }
                },
                _ => None,
            };
            let Some(suffix) = expected else { continue };
            let expected_id = format!("{}/{suffix}", relation.target);
            if entry.id.as_str() != expected_id {
                report.problems.push(format!(
                    "{}: a `{}` relation to `{}` needs the id `{expected_id}`, not `{}`",
                    line(relation.target.as_str()),
                    relation.kind.slug(),
                    relation.target,
                    entry.id
                ));
            }
        }
    }

    // The reverse rule: an id that ends in a known suffix and sits under an existing entry must
    // hold the matching edge. A carried id is checked like any other; new carried ids do not
    // end in a suffix.
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        let Some((parent, tail)) = entry.id.as_str().rsplit_once('/') else {
            continue;
        };
        if !ids.contains(parent) || !known.contains(tail) {
            continue;
        }
        let matches = entry.relations.iter().any(|relation| {
            if relation.target.as_str() != parent {
                return false;
            }
            match tail {
                "mythic" => {
                    relation.kind == RelationKind::Addon
                        && relation.label == Some(RelationLabel::Mythic)
                }
                "stamina" => {
                    relation.kind == RelationKind::Addon
                        && relation.label == Some(RelationLabel::Stamina)
                }
                _ => {
                    (relation.kind == RelationKind::Addon
                        && relation.label == Some(RelationLabel::Changes))
                        || relation.kind == RelationKind::Revision
                }
            }
        });
        if !matches {
            report.problems.push(format!(
                "{}: id ends in `/{tail}` under `{parent}`, so it must hold the matching `addon` or `revision` relation",
                stored.path.display()
            ));
        }
    }
}

/// Where a stored book lives: its path for messages and its text for line numbers.
struct BookFile<'a> {
    label: String,
    text: &'a str,
}

impl<'a> BookFile<'a> {
    fn of(corpus: &'a Corpus, source: &str) -> Self {
        match corpus.layout {
            BookLayout::Single => BookFile {
                label: "books.toml".to_string(),
                text: &corpus.books_text,
            },
            BookLayout::Split => BookFile {
                label: format!("{BOOKS_DIR}/{source}.toml"),
                text: corpus.book_texts.get(source).map_or("", String::as_str),
            },
        }
    }
}

/// The rules that only the split layout has: the page grammar the derivation relies on, and the
/// state of the format number. In the single layout a stray page is only a warning, because the
/// stored lists do not read pages.
fn check_book_layout(corpus: &Corpus, report: &mut Report) {
    let split = corpus.layout == BookLayout::Split;
    if split && corpus.config.format < SPLIT_BOOKS_FORMAT {
        report.warnings.push(format!(
            "corpus.toml:1: the corpus is format {} but holds the split `{BOOKS_DIR}/` layout; run `{}` to finish its migration",
            corpus.config.format,
            crate::config::MIGRATE_COMMAND
        ));
    }
    for stored in corpus.entries.values() {
        for row in &stored.entry.sources {
            let Some(page) = row.page.as_deref() else {
                continue;
            };
            if is_clean_page(page) {
                continue;
            }
            let message = format!(
                "{}:{}: `[[sources]]` page {page:?} for `{}` is not a page (`12`, `12-14`, `12, 14` or `inside cover`){}",
                stored.path.display(),
                line_of(&stored.text, &format!("page = \"{page}\"")),
                row.publication,
                if split {
                    "; the book order reads the leading number, so a stray citation puts the entry on a wrong page"
                } else {
                    ""
                }
            );
            if split {
                report.problems.push(message);
            } else {
                report.warnings.push(message);
            }
        }
    }
}

/// The 1-based line of the `[[books]]` block of `source` in `books.toml`, or 1.
fn book_line(text: &str, source: &str) -> usize {
    line_of(text, &format!("source = \"{source}\""))
}

/// The line of `entry`'s node inside the block of book `source`, or the block's own line.
fn placement_line(text: &str, source: &str, entry: &str) -> usize {
    let start = book_line(text, source);
    let needle = format!("\"{entry}\"");
    text.lines()
        .enumerate()
        .skip(start)
        .take_while(|(_, line)| !line.starts_with("[[books]]"))
        .find(|(_, line)| line.contains(&needle))
        .map_or(start, |(index, _)| index + 1)
}

/// `chapter` is the title of the top-level chapter that holds `node`.
#[allow(clippy::too_many_arguments)]
fn check_book_node(
    corpus: &Corpus,
    file: &BookFile<'_>,
    tree: &BookTree,
    chapter: &str,
    node: &BookNode,
    ids: &BTreeSet<&str>,
    keys: &BTreeMap<&str, Vec<(String, String)>>,
    placed: &mut BTreeSet<String>,
    report: &mut Report,
) {
    if let Some(entry) = &node.entry {
        let line = line_of(file.text, &format!("\"{entry}\""));
        let label = &file.label;
        if !ids.contains(entry.as_str()) {
            report.problems.push(format!(
                "{label}:{line}: book node `{}` names unknown entry `{entry}`",
                node.title
            ));
        } else {
            // A placement says the book prints the entry. The entry's `source_ids` are the
            // record of which books print it, so a placement in any other book is a claim
            // with no evidence behind it. This holds for every chapter kind, including
            // catch-all and listing chapters.
            let stored = &corpus.entries[entry.as_str()].entry;
            if !stored.source_ids.contains(&tree.source) {
                report.problems.push(format!(
                    "{label}:{}: book `{}` places `{entry}` in `{}` (chapter `{chapter}`), but the entry's `source_ids` ({}) do not name that book. Remove the placement, or add the book to `source_ids` in {} when it prints the entry",
                    placement_line(file.text, &tree.source, entry.as_str()),
                    tree.source,
                    node.title,
                    if stored.source_ids.is_empty() {
                        "empty".to_string()
                    } else {
                        stored.source_ids.join(", ")
                    },
                    corpus.entries[entry.as_str()].path.display()
                ));
            }
            // A marker is a promise that the hub page lists what the row hides. The automatic
            // rule already refuses a hub with an unlinked child; the authored marker needs the
            // same warning because it applies at every level and overrides the chapter default.
            if node.collapse
                && cast_iron_schema::books::listed_under(node, &|_| false) > 1
                && !cast_iron_schema::books::entry_lists_children(
                    node,
                    &corpus.entries[entry.as_str()].entry,
                )
            {
                report.warnings.push(format!(
                    "{label}:{line}: marked node `{}` collapses, but `{entry}` does not link every child entry in its body; collapsing would hide entries",
                    node.title
                ));
            }
            match &node.anchor {
                None => {
                    if !placed.insert(entry.to_string()) {
                        report
                            .problems
                            .push(format!("{label}:{line}: book places `{entry}` twice"));
                    }
                }
                Some(anchor) => {
                    let available = &keys[entry.as_str()];
                    if !available.iter().any(|(key, _)| key == anchor) {
                        report.problems.push(format!(
                            "{label}:{line}: book node `{}` points at `{entry}#{anchor}`, a heading that does not exist. {}",
                            node.title,
                            anchor_fix(&corpus.entries[entry.as_str()].path, anchor, available)
                        ));
                    }
                }
            }
        }
    }
    for child in &node.children {
        check_book_node(
            corpus, file, tree, chapter, child, ids, keys, placed, report,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus, corpus_in_format, heading, link, paragraph, rule};
    use cast_iron_schema::{
        BookIndex, BookNode, Citation, CreatureFields, EntryId, FeatFields, FeatureRef, Inline,
        SeriesKind, Trade, TradeOp,
    };

    fn creature(id: &str, creature_type: &str) -> RuleEntry {
        let mut entry = rule(id, vec![]);
        entry.kind = EntryKind::Creature;
        entry.fields = TypedFields::Creature(Box::new(CreatureFields {
            creature_type: Some(creature_type.into()),
            ..Default::default()
        }));
        entry
    }

    #[test]
    fn creature_type_must_be_a_pf1e_type() {
        let report = check(&corpus(vec![creature("ref/creature/ok", "magicalBeast")]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);

        let report = check(&corpus(vec![creature("ref/creature/bad", "Magical Beast")]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("entries/creature/bad.md:"), "{problem}");
        assert!(
            problem.contains("creature_type `Magical Beast`"),
            "{problem}"
        );
        assert!(problem.contains("magicalBeast"), "{problem}");
    }

    #[test]
    fn a_publication_needs_a_copyright_and_a_notice_or_a_pending_decision() {
        let mut c = corpus(vec![]);
        c.publications[0].copyright = None;
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("publications.toml:1: publication `core-rulebook` has no copyright"),
            "{text}"
        );

        let mut c = corpus(vec![]);
        c.legal.section_15.clear();
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "publication `core-rulebook` (Core Rulebook) is covered by no section_15"
            ),
            "{text}"
        );

        c.legal.notice_pending = vec!["Core Rulebook".into()];
        assert!(check(&c).problems.is_empty());
    }

    #[test]
    fn a_cup_only_publication_needs_no_ogl_section_15_notice() {
        let mut c = corpus(vec![]);
        c.legal.section_15.clear();
        c.legal.publication_licenses = vec![cast_iron_schema::PublicationLicense {
            publication_id: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            license_refs: vec!["legal#community-use".into()],
        }];
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);

        c.legal.publication_licenses[0]
            .license_refs
            .push("ogl-1.0a".into());
        assert!(
            check(&c)
                .problems
                .iter()
                .any(|problem| problem.contains("covered by no section_15 notice"))
        );
    }

    #[test]
    fn a_publication_license_basis_must_match_a_known_publication_and_license() {
        let mut c = corpus(vec![]);
        c.legal.publication_licenses = vec![cast_iron_schema::PublicationLicense {
            publication_id: "missing-publication".into(),
            title: "Missing publication".into(),
            license_refs: vec!["missing-license".into()],
        }];
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("names unknown publication `missing-publication`"),
            "{text}"
        );
        assert!(
            text.contains("has unknown licence `missing-license`"),
            "{text}"
        );
    }

    #[test]
    fn a_third_party_marker_is_rejected_beside_a_paizo_copyright() {
        let mut c = corpus(vec![]);
        c.publications[0].third_party = true;
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "publication `core-rulebook` is marked `third_party = true` but its copyright names Paizo"
            ),
            "{text}"
        );

        // The name is matched regardless of case.
        let mut c = corpus(vec![]);
        c.publications[0].third_party = true;
        c.publications[0].copyright = Some("Core Rulebook, (c) PAIZO Inc.".into());
        c.legal.section_15[0].notice = "Core Rulebook, (c) PAIZO Inc.".into();
        let text = check(&c).problems.join("\n");
        assert!(text.contains("but its copyright names Paizo"), "{text}");

        // A non-Paizo copyright with the marker is clean.
        let mut c = corpus(vec![]);
        c.publications[0].third_party = true;
        c.publications[0].copyright = Some("Necromancer Games, (c) 2011".into());
        c.legal.section_15[0].notice = "Necromancer Games, (c) 2011".into();
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);

        // A copyright that does not name Paizo is not a problem without the marker: some
        // Paizo books still have a pending notice.
        let mut c = corpus(vec![]);
        c.publications[0].copyright = Some("Necromancer Games, (c) 2011".into());
        c.legal.section_15[0].notice = "Necromancer Games, (c) 2011".into();
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);
    }

    fn footnote_heading(anchor: &str, label: &str) -> Block {
        Block::Heading {
            level: 6,
            anchor: Some(anchor.into()),
            content: vec![Inline::Sup {
                c: vec![Inline::text(label)],
            }],
        }
    }

    /// The problems of a one-entry corpus; footnote findings fail the check.
    fn footnote_problems(body: Vec<Block>) -> Vec<String> {
        let report = check(&corpus(vec![rule("ref/rule/note", body)]));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        report.problems
    }

    #[test]
    fn a_footnote_without_a_marker_in_the_body_fails_the_check() {
        let problems = footnote_problems(vec![
            paragraph(vec![Inline::text("Text with no marker.")]),
            footnote_heading("footnote-b", "B"),
            paragraph(vec![Inline::text("The note.")]),
        ]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("footnote `B` (`#footnote-b`) has no marker"),
            "{problems:?}"
        );
    }

    #[test]
    fn a_footnote_with_a_superscript_or_linked_marker_passes() {
        let note = |anchor: &str, label: &str| {
            [
                footnote_heading(anchor, label),
                paragraph(vec![Inline::text("The note.")]),
            ]
        };
        // A plain superscript with the heading label.
        let mut body = vec![paragraph(vec![
            Inline::text("Bonus"),
            Inline::Sup {
                c: vec![Inline::text("B")],
            },
        ])];
        body.extend(note("footnote-b", "B"));
        assert!(footnote_problems(body).is_empty());

        // A superscript that links to the anchor, whatever its label.
        let mut body = vec![paragraph(vec![Inline::Sup {
            c: vec![link("ref/rule/note", Some("footnote-1"), "1")],
        }])];
        body.extend(note("footnote-1", "1"));
        assert!(footnote_problems(body).is_empty());

        // A marker in a section title counts (`### Removed Feats^[1](…#footnote-1)^`), but a
        // footnote heading's own label does not mark itself.
        let mut body = vec![Block::Heading {
            level: 3,
            anchor: None,
            content: vec![
                Inline::text("Removed Feats"),
                Inline::Sup {
                    c: vec![link("ref/rule/note", Some("footnote-1"), "1")],
                },
            ],
        }];
        body.extend(note("footnote-1", "1"));
        assert!(footnote_problems(body).is_empty());
        assert_eq!(footnote_problems(note("footnote-2", "2").to_vec()).len(), 1);

        // A heading level other than six, or an anchor without the prefix, is not a footnote.
        let body = vec![
            heading(Some("footnote-x"), "Title"),
            Block::Heading {
                level: 6,
                anchor: Some("section".into()),
                content: vec![Inline::text("Section")],
            },
        ];
        assert!(footnote_problems(body).is_empty());
    }

    #[test]
    fn converter_residue_in_a_body_fails_the_check() {
        let problems = footnote_problems(vec![paragraph(vec![Inline::text(
            "ability.\\|M~mythic spells",
        )])]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("footnote converter residue"),
            "{problems:?}"
        );

        // A pipe or a tilde alone is ordinary text.
        let problems = footnote_problems(vec![paragraph(vec![Inline::text(
            "about 5~10 | fine, a|b and x~y",
        )])]);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_paizo_entry_linking_third_party_only_content_is_a_warning() {
        let target = in_publication(rule("ref/rule/tp", vec![]), "third-party");
        let source = rule(
            "ref/rule/base",
            vec![paragraph(vec![link("ref/rule/tp", None, "third party")])],
        );
        let mut c = corpus(vec![source, target]);
        add_third_party(&mut c, "third-party", None);

        let report = check(&c);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(
            report.warnings[0].contains("body link to `ref/rule/tp` is third-party only"),
            "{:?}",
            report.warnings
        );
    }

    #[test]
    fn mixed_sources_warn_but_a_third_party_only_entry_does_not() {
        let mut mixed = in_publication(rule("ref/rule/mixed", vec![]), "core-rulebook");
        mixed.source_ids.push("third-party".into());
        let mut c = corpus(vec![mixed]);
        add_third_party(&mut c, "third-party", None);

        let report = check(&c);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(
            report.warnings[0].contains("entry mixes third-party and Paizo sources"),
            "{:?}",
            report.warnings
        );

        // An entry whose sources are all third-party is the split form and does not warn.
        let only = in_publication(rule("ref/rule/only", vec![]), "third-party");
        let mut c = corpus(vec![only]);
        add_third_party(&mut c, "third-party", None);
        let report = check(&c);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    #[test]
    fn a_series_row_needs_the_fields_of_its_kind() {
        // An Adventure Path needs a name and a positive volume, and no season.
        let mut c = corpus(vec![]);
        c.publications[0].series_kind = Some(SeriesKind::AdventurePath);
        let text = check(&c).problems.join("\n");
        assert!(text.contains("has no `series`"), "{text}");
        assert!(text.contains("has no `series_number`"), "{text}");

        let mut c = corpus(vec![]);
        c.publications[0].series_kind = Some(SeriesKind::AdventurePath);
        c.publications[0].series = Some("Iron Gods".into());
        c.publications[0].series_number = Some("0".into());
        c.publications[0].season = Some(3);
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("not a positive Adventure Path volume"),
            "{text}"
        );
        assert!(text.contains("carries `season`"), "{text}");

        let mut c = corpus(vec![]);
        c.publications[0].series_kind = Some(SeriesKind::AdventurePath);
        c.publications[0].series = Some("Iron Gods".into());
        c.publications[0].series_number = Some("1".into());
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);

        // A Module is ordered by date and carries no series fields.
        let mut c = corpus(vec![]);
        c.publications[0].series_kind = Some(SeriesKind::Module);
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);
        c.publications[0].series = Some("Module".into());
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("modules are ordered by publication_date"),
            "{text}"
        );

        // Society needs a season; a scenario number must match it.
        let mut c = corpus(vec![]);
        c.publications[0].series_kind = Some(SeriesKind::Society);
        let text = check(&c).problems.join("\n");
        assert!(text.contains("has no `season`"), "{text}");

        let mut c = corpus(vec![]);
        c.publications[0].series_kind = Some(SeriesKind::Society);
        c.publications[0].season = Some(3);
        c.publications[0].series_number = Some("3-01".into());
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);

        c.publications[0].series_number = Some("2-01".into());
        let text = check(&c).problems.join("\n");
        assert!(text.contains("not `<season>-NN`"), "{text}");

        // Series fields without a kind cannot half-join a grouping.
        let mut c = corpus(vec![]);
        c.publications[0].series = Some("Rise of the Runelords".into());
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("series fields but no `series_kind`"),
            "{text}"
        );
    }

    #[test]
    fn left_over_upstream_tokens_fail_and_html_entities_do_not() {
        let dirty = rule(
            "ref/rule/a",
            vec![paragraph(vec![Inline::text("&L&Title&FN& and &Q&EM")])],
        );
        let text = check(&corpus(vec![dirty])).problems.join("\n");
        assert!(text.contains("left-over upstream token `&L&`"), "{text}");
        assert!(text.contains("`&FN&`"), "{text}");
        assert!(text.contains("`&Q&`"), "{text}");

        let clean = rule(
            "ref/rule/b",
            vec![paragraph(vec![Inline::text(
                "&times& &quot& &mdash &S &Next1&",
            )])],
        );
        assert!(check(&corpus(vec![clean])).problems.is_empty());
    }

    #[test]
    fn a_left_over_import_marker_fails() {
        let dirty = rule(
            "ref/rule/p-marker",
            vec![paragraph(vec![Inline::text(
                "At 6th level: p!These bonuses increase to +2.",
            )])],
        );
        let report = check(&corpus(vec![dirty]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(
            problem.starts_with("entries/rule/p-marker.md:"),
            "{problem}"
        );
        assert!(
            problem.contains("left-over import marker `p!`"),
            "{problem}"
        );
    }

    #[test]
    fn p_before_an_exclamation_inside_a_word_is_not_a_marker() {
        let clean = rule(
            "ref/rule/p-real-text",
            vec![paragraph(vec![Inline::text(
                "A buoy with the word \"Help!\" scratched on it, worth 35,150 gp!",
            )])],
        );
        let report = check(&corpus(vec![clean]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn a_marker_at_the_start_of_a_body_fails() {
        let dirty = rule(
            "ref/rule/p-first",
            vec![paragraph(vec![Inline::text(
                "p!These bonuses increase to +2.",
            )])],
        );
        let report = check(&corpus(vec![dirty]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].contains("left-over import marker `p!`"),
            "{}",
            report.problems[0]
        );
    }

    #[test]
    fn a_left_over_upstream_token_in_a_typed_field_fails() {
        let mut feat = rule("ref/feat/a", vec![]);
        feat.kind = EntryKind::Feat;
        feat.fields = TypedFields::Feat(cast_iron_schema::FeatFields {
            benefit: Some("&FN&gain a bonus".into()),
            ..Default::default()
        });
        let report = check(&corpus(vec![feat]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("entries/feat/a.md:9:"), "{problem}");
        assert!(
            problem.contains("left-over upstream token `&FN&`"),
            "{problem}"
        );
        assert!(problem.contains("`[fields]` key `benefit`"), "{problem}");
    }

    #[test]
    fn lowercase_entities_in_a_typed_field_do_not_fail() {
        let mut feat = rule("ref/feat/b", vec![]);
        feat.kind = EntryKind::Feat;
        feat.fields = TypedFields::Feat(cast_iron_schema::FeatFields {
            benefit: Some("&times& &quot&".into()),
            ..Default::default()
        });
        assert!(check(&corpus(vec![feat])).problems.is_empty());
    }

    #[test]
    fn an_entity_reference_in_a_typed_field_fails_the_check() {
        let mut feat = rule("ref/feat/entity", vec![]);
        feat.kind = EntryKind::Feat;
        feat.fields = TypedFields::Feat(cast_iron_schema::FeatFields {
            benefit: Some("deal &times;2 damage&#8212;or &#x2020;".into()),
            ..Default::default()
        });
        let report = check(&corpus(vec![feat]));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let text = report.problems.join("\n");
        for entity in ["&times;", "&#8212;", "&#x2020;"] {
            assert!(
                text.contains(&format!("HTML entity reference `{entity}`")),
                "{text}"
            );
        }
        assert!(text.contains("`[fields]` key `benefit`"), "{text}");
    }

    #[test]
    fn broken_fragments_and_bare_ampersands_do_not_warn_as_entities() {
        assert_eq!(
            entity_references("Swords & Sorcery &times& 2 &Q; &;"),
            vec!["&Q;"]
        );
        assert_eq!(
            entity_references("&mdash; &#39; &#;"),
            vec!["&mdash;", "&#39;"]
        );
    }

    #[test]
    fn a_token_in_a_title_or_tag_fails() {
        let mut titled = rule("ref/rule/titled", vec![]);
        titled.title = "&FN&Title".into();
        let text = check(&corpus(vec![titled])).problems.join("\n");
        assert!(text.contains("left-over upstream token `&FN&`"), "{text}");
        assert!(text.contains("in the entry title"), "{text}");

        let mut tagged = rule("ref/rule/tagged", vec![]);
        tagged.tags = vec!["&Q&tag".into()];
        let text = check(&corpus(vec![tagged])).problems.join("\n");
        assert!(text.contains("left-over upstream token `&Q&`"), "{text}");
        assert!(text.contains("in the entry tags"), "{text}");

        let mut clean = rule("ref/rule/clean", vec![]);
        clean.title = "&times& Title".into();
        clean.tags = vec!["&quot& tag".into()];
        assert!(check(&corpus(vec![clean])).problems.is_empty());
    }

    #[test]
    fn a_repeated_token_reports_each_occurrence_line() {
        let mut feat = rule("ref/feat/twice", vec![]);
        feat.kind = EntryKind::Feat;
        feat.fields = TypedFields::Feat(FeatFields {
            benefit: Some("&FN& twice".into()),
            special: Some("&FN& twice".into()),
            ..Default::default()
        });
        let c = corpus(vec![feat]);
        let text = &c.entries["ref/feat/twice"].text;
        let expected = [
            line_of(text, "benefit = \"&FN& twice\""),
            line_of(text, "special = \"&FN& twice\""),
        ];
        assert_ne!(
            expected[0], expected[1],
            "the two fields are on separate lines"
        );

        let report = check(&c);
        let reported: Vec<usize> = report
            .problems
            .iter()
            .map(|problem| {
                problem
                    .split(':')
                    .nth(1)
                    .and_then(|line| line.parse().ok())
                    .unwrap_or(0)
            })
            .collect();
        assert_eq!(reported.len(), 2, "{:?}", report.problems);
        assert_eq!(reported, expected, "{:?}", report.problems);
    }

    #[test]
    fn a_clean_corpus_has_no_problems() {
        let a = rule(
            "ref/rule/a",
            vec![paragraph(vec![link("ref/rule/b", Some("sec-deep"), "b")])],
        );
        let b = rule("ref/rule/b", vec![heading(None, "Deep")]);
        let report = check(&corpus(vec![a, b]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn an_aon_entry_path_must_be_a_relative_display_page() {
        for value in [
            "FeatDisplay.aspx?ItemName=Power%20Attack",
            "ArchetypeDisplay.aspx?FixedName=Alchemist%20Aerochemist",
            "MagicWondrousDisplay.aspx?FinalName=Wayfinder",
        ] {
            let mut entry = rule("ref/feat/power-attack", vec![]);
            entry.aon = Some(value.into());
            let report = check(&corpus(vec![entry]));
            assert!(report.problems.is_empty(), "{value}: {report:?}");
        }

        let bad = [
            "https://aonprd.com/FeatDisplay.aspx?ItemName=Power%20Attack",
            "aonprd.com/FeatDisplay.aspx?ItemName=Power%20Attack",
            "/FeatDisplay.aspx?ItemName=Power%20Attack",
            "//aonprd.com/FeatDisplay.aspx?ItemName=Power%20Attack",
            "FeatDisplay.aspx",
            "FeatDisplay.aspx?ItemName=Power Attack",
            "FeatDisplay.aspx?ItemName=Power%2",
            "FeatDisplay.aspx?ItemName=Power%20Attack#top",
            "Rules.aspx?ID=1",
            "FeatDisplay.aspx?Name=Power%20Attack",
            "FeatDisplay.aspx?ItemName=Power%20Attack&ItemName=Other",
        ];
        for value in bad {
            let mut entry = rule("ref/feat/power-attack", vec![]);
            entry.aon = Some(value.into());
            let report = check(&corpus(vec![entry]));
            assert_eq!(report.problems.len(), 1, "{value}: {report:?}");
            assert!(
                report.problems[0].contains("`aon`"),
                "{value}: {:?}",
                report.problems
            );
        }
    }

    #[test]
    fn a_publication_aon_path_must_use_source_display() {
        let mut clean = corpus(vec![]);
        clean.publications[0].aon = Some("SourceDisplay.aspx?FixedSource=Core%20Rulebook".into());
        assert!(
            check(&clean).problems.is_empty(),
            "{:?}",
            check(&clean).problems
        );

        let mut entry_page = corpus(vec![]);
        entry_page.publications[0].aon = Some("FeatDisplay.aspx?ItemName=Power%20Attack".into());
        let text = check(&entry_page).problems.join("\n");
        assert!(text.contains("must name `SourceDisplay.aspx`"), "{text}");

        let mut wrong_key = corpus(vec![]);
        wrong_key.publications[0].aon = Some("SourceDisplay.aspx?ItemName=Core%20Rulebook".into());
        let text = check(&wrong_key).problems.join("\n");
        assert!(text.contains("must use `FixedSource`"), "{text}");
    }

    #[test]
    fn unknown_sources_licences_and_targets_name_the_file() {
        let mut a = rule(
            "ref/rule/a",
            vec![paragraph(vec![link("ref/rule/missing", None, "x")])],
        );
        a.source_ids = vec!["no-such-book".into()];
        a.license_refs = vec!["no-such-licence".into()];
        let report = check(&corpus(vec![a]));
        let text = report.problems.join("\n");
        assert!(text.contains("entries/rule/a.md:"), "{text}");
        assert!(
            text.contains("unknown publication `no-such-book`"),
            "{text}"
        );
        assert!(text.contains("unknown licence `no-such-licence`"), "{text}");
        assert!(
            text.contains("unknown link target `ref/rule/missing`"),
            "{text}"
        );
    }

    #[test]
    fn a_missing_heading_anchor_names_both_files_and_the_fix() {
        let a = rule(
            "ref/rule/a",
            vec![paragraph(vec![link("ref/rule/b", Some("nowhere"), "b")])],
        );
        let b = rule(
            "ref/rule/b",
            vec![
                heading(Some("effects"), "Effects"),
                heading(None, "Other Part"),
            ],
        );
        let report = check(&corpus(vec![a, b]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("entries/rule/a.md:"), "{problem}");
        assert!(problem.contains("ref/rule/b#nowhere"), "{problem}");
        assert!(problem.contains("entries/rule/b.md"), "{problem}");
        assert!(problem.contains("`effects` (\"Effects\")"), "{problem}");
        assert!(problem.contains("`sec-other-part`"), "{problem}");
        assert!(problem.contains("add `{#nowhere}`"), "{problem}");
    }

    #[test]
    fn a_race_needs_a_group_once_any_race_has_one() {
        let mut missing = rule("ref/race/hillfolk", vec![]);
        missing.kind = EntryKind::Race;
        // An older corpus has no value anywhere. It still loads, because the pin predates the
        // field.
        let report = check(&corpus(vec![missing.clone()]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);

        let mut typed = missing.clone();
        typed.id = EntryId::parse("ref/race/hill-giant").unwrap();
        typed.race_group = Some(cast_iron_schema::RaceGroup::OtherRace);
        let mut carried = rule("ref/carried/old/race", vec![]);
        carried.kind = EntryKind::Race;
        let report = check(&corpus(vec![typed, missing, carried]));
        // Both the typed race and the carried chapter need a group: either one without it is
        // absent from the Races list facets.
        let named: Vec<_> = report
            .problems
            .iter()
            .filter(|p| p.contains("needs `race_group`"))
            .collect();
        assert_eq!(named.len(), 2, "{:?}", report.problems);
        assert!(
            named.iter().any(|p| p.contains("entries/race/hillfolk.md")),
            "{named:?}"
        );
        assert!(
            named
                .iter()
                .any(|p| p.contains("entries/carried/old/race.md")),
            "{named:?}"
        );

        let mut wrong = rule("ref/rule/x", vec![]);
        wrong.race_group = Some(cast_iron_schema::RaceGroup::CoreRace);
        let report = check(&corpus(vec![wrong]));
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.contains("only valid on a race")),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn a_book_node_anchor_is_checked_against_the_target() {
        let mut corpus = corpus(vec![rule(
            "ref/rule/b",
            vec![heading(Some("effects"), "Effects")],
        )]);
        corpus.books = BookIndex {
            books: vec![BookTree {
                source: "core-rulebook".into(),
                title: "Core".into(),
                basis: "reviewed".into(),
                review: vec![],
                chapters: vec![BookNode {
                    title: "Chapter".into(),
                    entry: Some(EntryId::parse("ref/rule/b").unwrap()),
                    anchor: Some("gone".into()),
                    review: None,
                    collapse: false,
                    lists_children: false,
                    children: vec![],
                }],
            }],
        };
        corpus.books_text = "[[books.chapters]]\nentry = \"ref/rule/b\"\n".into();
        let report = check(&corpus);
        let problem = &report.problems[0];
        assert!(problem.starts_with("books.toml:2:"), "{problem}");
        assert!(problem.contains("entries/rule/b.md"), "{problem}");
        assert!(problem.contains("`effects`"), "{problem}");
    }

    fn placed(title: &str, entry: &str, children: Vec<BookNode>) -> BookNode {
        BookNode {
            title: title.into(),
            entry: Some(EntryId::parse(entry).unwrap()),
            anchor: None,
            review: None,
            collapse: false,
            lists_children: false,
            children,
        }
    }

    fn tree(source: &str, chapters: Vec<BookNode>) -> BookTree {
        BookTree {
            source: source.into(),
            title: source.into(),
            basis: "reviewed".into(),
            review: vec![],
            chapters,
        }
    }

    /// Two publications, `core-rulebook` and `other-book`, and one rule in the first only.
    fn two_books() -> crate::Corpus {
        let mut corpus = corpus(vec![rule("ref/rule/a", vec![]), rule("ref/rule/b", vec![])]);
        add_publication(&mut corpus, "other-book", None);
        corpus
            .legal
            .section_15
            .push(cast_iron_schema::CopyrightNotice {
                title: "other-book".into(),
                notice: "Core Rulebook, (c) Paizo Inc.".into(),
            });
        corpus
    }

    #[test]
    fn a_placement_must_be_in_a_book_the_entry_names() {
        let mut corpus = two_books();
        corpus.books = BookIndex {
            books: vec![
                tree("core-rulebook", vec![placed("A", "ref/rule/a", vec![])]),
                tree("other-book", vec![placed("A again", "ref/rule/a", vec![])]),
            ],
        };
        corpus.books_text = "[[books]]\nsource = \"core-rulebook\"\n\n[[books.chapters]]\nentry = \"ref/rule/a\"\n\n[[books]]\nsource = \"other-book\"\n\n[[books.chapters]]\nentry = \"ref/rule/a\"\n".into();
        let report = check(&corpus);
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("books.toml:11:"), "{problem}");
        assert!(
            problem.contains("book `other-book` places `ref/rule/a`"),
            "{problem}"
        );
        assert!(
            problem.contains("`source_ids` (core-rulebook)"),
            "{problem}"
        );
        assert!(problem.contains("entries/rule/a.md"), "{problem}");
    }

    #[test]
    fn a_nested_placement_must_be_in_a_book_the_entry_names() {
        let mut corpus = two_books();
        corpus.books = BookIndex {
            books: vec![tree(
                "other-book",
                vec![placed(
                    "Chapter",
                    "ref/rule/a",
                    vec![placed("Section", "ref/rule/b", vec![])],
                )],
            )],
        };
        corpus
            .entries
            .get_mut("ref/rule/a")
            .unwrap()
            .entry
            .source_ids = vec!["other-book".into()];
        let report = check(&corpus);
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].contains("places `ref/rule/b` in `Section` (chapter `Chapter`)"),
            "{:?}",
            report.problems
        );
    }

    fn anchored(title: &str, entry: &str, anchor: &str) -> BookNode {
        BookNode {
            anchor: Some(anchor.into()),
            ..placed(title, entry, vec![])
        }
    }

    #[test]
    fn a_listing_or_catch_all_chapter_is_not_exempt_from_the_membership_rule() {
        for chapter in ["New entries (page order)", "Page order", "Unpaged"] {
            let mut corpus = two_books();
            corpus.books = BookIndex {
                books: vec![tree(
                    "other-book",
                    vec![placed(chapter, "ref/rule/a", vec![])],
                )],
            };
            let report = check(&corpus);
            assert_eq!(report.problems.len(), 1, "{chapter}: {:?}", report.problems);
            assert!(
                report.problems[0].contains(&format!("in `{chapter}` (chapter `{chapter}`)")),
                "{chapter}: {:?}",
                report.problems
            );
            // The same placement passes once the entry names the book.
            corpus
                .entries
                .get_mut("ref/rule/a")
                .unwrap()
                .entry
                .source_ids
                .push("other-book".into());
            let report = check(&corpus);
            assert!(
                report.problems.is_empty(),
                "{chapter}: {:?}",
                report.problems
            );
        }
    }

    #[test]
    fn a_repeated_anchored_placement_is_checked_each_time() {
        let heading = vec![heading(Some("effects"), "Effects")];
        let mut corpus = corpus(vec![
            rule("ref/rule/a", heading.clone()),
            rule("ref/rule/b", vec![]),
        ]);
        add_publication(&mut corpus, "other-book", None);
        corpus
            .legal
            .section_15
            .push(cast_iron_schema::CopyrightNotice {
                title: "other-book".into(),
                notice: "Core Rulebook, (c) Paizo Inc.".into(),
            });
        // An anchored node may repeat its entry within one book; each node is a placement.
        let repeated = || {
            vec![placed(
                "Page order",
                "ref/rule/b",
                vec![
                    anchored("First", "ref/rule/a", "effects"),
                    anchored("Second", "ref/rule/a", "effects"),
                ],
            )]
        };
        corpus
            .entries
            .get_mut("ref/rule/a")
            .unwrap()
            .entry
            .source_ids = vec!["other-book".into()];
        corpus
            .entries
            .get_mut("ref/rule/b")
            .unwrap()
            .entry
            .source_ids = vec!["other-book".into()];
        corpus.books = BookIndex {
            books: vec![tree("other-book", repeated())],
        };
        let report = check(&corpus);
        assert!(report.problems.is_empty(), "{:?}", report.problems);

        // Outside `source_ids`, both anchored nodes fail, not only the first.
        corpus
            .entries
            .get_mut("ref/rule/a")
            .unwrap()
            .entry
            .source_ids = vec!["core-rulebook".into()];
        let report = check(&corpus);
        let membership: Vec<_> = report
            .problems
            .iter()
            .filter(|problem| problem.contains("do not name that book"))
            .collect();
        assert_eq!(membership.len(), 2, "{:?}", report.problems);
        assert!(membership[0].contains("in `First`"), "{membership:?}");
        assert!(membership[1].contains("in `Second`"), "{membership:?}");
        assert!(
            membership[0].contains("(chapter `Page order`)"),
            "{membership:?}"
        );
    }

    #[test]
    fn a_placement_in_a_book_the_entry_names_passes() {
        let mut corpus = two_books();
        corpus
            .entries
            .get_mut("ref/rule/a")
            .unwrap()
            .entry
            .source_ids = vec!["core-rulebook".into(), "other-book".into()];
        corpus.books = BookIndex {
            books: vec![
                tree(
                    "core-rulebook",
                    vec![placed(
                        "A",
                        "ref/rule/a",
                        vec![placed("B", "ref/rule/b", vec![])],
                    )],
                ),
                tree("other-book", vec![placed("A", "ref/rule/a", vec![])]),
            ],
        };
        let report = check(&corpus);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn an_entry_with_no_source_ids_cannot_be_placed() {
        let mut corpus = two_books();
        corpus
            .entries
            .get_mut("ref/rule/a")
            .unwrap()
            .entry
            .source_ids
            .clear();
        corpus.books = BookIndex {
            books: vec![tree(
                "core-rulebook",
                vec![placed("A", "ref/rule/a", vec![])],
            )],
        };
        let text = check(&corpus).problems.join("\n");
        assert!(text.contains("`source_ids` (empty)"), "{text}");
    }

    #[test]
    fn every_book_needs_a_publication_row() {
        let mut corpus = two_books();
        corpus.books = BookIndex {
            books: vec![tree("core-rulebook", vec![]), tree("phantom-book", vec![])],
        };
        corpus.books_text =
            "[[books]]\nsource = \"core-rulebook\"\n\n[[books]]\nsource = \"phantom-book\"\n"
                .into();
        let report = check(&corpus);
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        let problem = &report.problems[0];
        assert!(problem.starts_with("books.toml:5:"), "{problem}");
        assert!(
            problem.contains("book `phantom-book` has no row in publications.toml"),
            "{problem}"
        );

        corpus.books = BookIndex {
            books: vec![tree("core-rulebook", vec![]), tree("other-book", vec![])],
        };
        assert!(check(&corpus).problems.is_empty());
    }

    #[test]
    fn a_marked_book_node_warns_when_its_page_does_not_link_every_child() {
        let node = || BookNode {
            title: "Long list".into(),
            entry: Some(EntryId::parse("ref/rule/list").unwrap()),
            anchor: None,
            review: None,
            collapse: true,
            lists_children: false,
            children: vec![BookNode {
                title: "Child".into(),
                entry: Some(EntryId::parse("ref/rule/child").unwrap()),
                anchor: None,
                review: None,
                collapse: false,
                lists_children: false,
                children: vec![],
            }],
        };
        let books = |node: BookNode| BookIndex {
            books: vec![BookTree {
                source: "core-rulebook".into(),
                title: "Core".into(),
                basis: "reviewed".into(),
                review: vec![],
                chapters: vec![node],
            }],
        };

        // The page lists neither child, so a marked row would hide the child.
        let mut first = corpus(vec![
            rule("ref/rule/list", vec![]),
            rule("ref/rule/child", vec![]),
        ]);
        first.books = books(node());
        first.books_text = "[[books.chapters]]\nentry = \"ref/rule/list\"\n".into();
        let report = check(&first);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(
            report.warnings[0].contains("marked node `Long list` collapses"),
            "{:?}",
            report.warnings
        );

        // The linked child is the list the page already shows, so the marker is safe.
        let mut corpus = corpus(vec![
            rule(
                "ref/rule/list",
                vec![Block::EntryLink {
                    target: EntryId::parse("ref/rule/child").unwrap(),
                    title: "Child".into(),
                }],
            ),
            rule("ref/rule/child", vec![]),
        ]);
        corpus.books = books(node());
        corpus.books_text = "[[books.chapters]]\nentry = \"ref/rule/list\"\n".into();
        assert!(check(&corpus).warnings.is_empty());
    }

    #[test]
    fn typed_fields_must_match_the_entry_kind() {
        let none = TypedFields::None;
        assert!(fields_match(EntryKind::Rule, &none));
        assert!(fields_match(EntryKind::Archetype, &none));
        assert!(!fields_match(EntryKind::Spell, &none));
        let archetype = TypedFields::Archetype(cast_iron_schema::ArchetypeFields::default());
        assert!(fields_match(EntryKind::Archetype, &archetype));
        assert!(!fields_match(EntryKind::ClassOption, &archetype));
        let option = TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
            group: "hex".into(),
            ..Default::default()
        });
        assert!(fields_match(EntryKind::ClassOption, &option));
        assert!(!fields_match(EntryKind::Archetype, &option));
        assert_eq!(fields_kind(&option), "class_option");
        let class = TypedFields::Class(cast_iron_schema::ClassFields::default());
        assert!(fields_match(EntryKind::Class, &class));
        assert!(fields_match(EntryKind::PrestigeClass, &class));
        assert!(!fields_match(EntryKind::Feat, &class));
        let spell = TypedFields::Spell(cast_iron_schema::SpellFields::default());
        assert!(fields_match(EntryKind::Spell, &spell));
        assert!(!fields_match(EntryKind::Item, &spell));
        assert_eq!(fields_kind(&spell), "spell");
    }

    #[test]
    fn an_entry_kind_that_disagrees_with_its_fields_is_a_problem() {
        let mut a = rule("ref/rule/a", vec![]);
        a.kind = EntryKind::Spell;
        let report = check(&corpus(vec![a]));
        assert!(report.problems[0].contains("does not match"));
    }

    fn typed(mut entry: RuleEntry, kind: EntryKind, fields: TypedFields) -> RuleEntry {
        entry.kind = kind;
        entry.fields = fields;
        entry
    }

    fn spell(id: &str) -> RuleEntry {
        typed(
            rule(id, vec![]),
            EntryKind::Spell,
            TypedFields::Spell(cast_iron_schema::SpellFields::default()),
        )
    }

    fn feat(id: &str, feat_types: &[&str]) -> RuleEntry {
        typed(
            rule(id, vec![]),
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                feat_types: feat_types.iter().map(|kind| kind.to_string()).collect(),
                ..Default::default()
            }),
        )
    }

    fn related(
        mut entry: RuleEntry,
        kind: RelationKind,
        target: &str,
        label: Option<&str>,
    ) -> RuleEntry {
        entry.relations.push(Relation {
            kind,
            target: EntryId::parse(target).unwrap(),
            label: label.map(|slug| RelationLabel::from_slug(slug).expect("a known label")),
        });
        entry
    }

    fn add_publication(corpus: &mut crate::Corpus, id: &str, suffix: Option<&str>) {
        corpus.publications.push(crate::Publication {
            id: id.into(),
            title: id.into(),
            aon: None,
            copyright: Some("Core Rulebook, (c) Paizo Inc.".into()),
            category: None,
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
            suffix: suffix.map(str::to_string),
        });
    }

    fn add_third_party(corpus: &mut crate::Corpus, id: &str, suffix: Option<&str>) {
        add_publication(corpus, id, suffix);
        let publication = corpus.publications.last_mut().unwrap();
        publication.title = format!("Third Party {id}");
        let copyright = format!("Third Party {id}. (c) 2020, Example Games.");
        publication.copyright = Some(copyright.clone());
        publication.third_party = true;
        corpus
            .legal
            .section_15
            .push(cast_iron_schema::CopyrightNotice {
                title: publication.title.clone(),
                notice: copyright,
            });
    }

    fn in_publication(mut entry: RuleEntry, source: &str) -> RuleEntry {
        entry.source_ids = vec![source.into()];
        entry
    }

    fn with_source(mut entry: RuleEntry, publication: &str, page: Option<&str>) -> RuleEntry {
        entry.source_ids.push(publication.into());
        entry.sources.push(Citation {
            publication: publication.into(),
            page: page.map(str::to_string),
        });
        entry
    }

    #[test]
    fn a_source_citation_must_name_a_known_publication() {
        let entry = with_source(rule("ref/rule/a", vec![]), "missing-book", Some("12"));
        let text = check(&corpus(vec![entry])).problems.join("\n");
        assert!(
            text.contains("unknown publication `missing-book`"),
            "{text}"
        );
        assert!(text.contains("[[sources]]"), "{text}");
    }

    #[test]
    fn a_source_citation_publication_must_be_in_source_ids() {
        let mut entry = rule("ref/rule/a", vec![]);
        entry.sources.push(Citation {
            publication: "core-rulebook".into(),
            page: Some("12".into()),
        });
        // `rule` already lists core-rulebook, so remove it to make the citation stray.
        entry.source_ids.clear();
        let text = check(&corpus(vec![entry])).problems.join("\n");
        assert!(text.contains("is not listed in `source_ids`"), "{text}");
    }

    fn expansion_corpus(entries: Vec<RuleEntry>) -> crate::Corpus {
        let mut corpus = corpus(entries);
        add_publication(&mut corpus, "monster-handbook", None);
        corpus
    }

    fn with_expansion(mut entry: RuleEntry, publication: &str, via: Option<&str>) -> RuleEntry {
        entry.expanded_in.push(cast_iron_schema::Expansion {
            publication: publication.into(),
            page: Some("18".into()),
            via: via.map(|via| EntryId::parse(via).unwrap()),
        });
        entry
    }

    #[test]
    fn an_expansion_must_name_a_known_publication() {
        let entry = with_expansion(rule("ref/rule/a", vec![]), "missing-book", None);
        let text = check(&expansion_corpus(vec![entry])).problems.join("\n");
        assert!(
            text.contains("[[expanded_in]]`: unknown publication `missing-book`"),
            "{text}"
        );
    }

    #[test]
    fn an_exact_duplicate_expansion_row_is_refused() {
        let entry = with_expansion(rule("ref/rule/a", vec![]), "monster-handbook", None);
        let entry = with_expansion(entry, "monster-handbook", None);
        let text = check(&expansion_corpus(vec![entry])).problems.join("\n");
        assert!(
            text.contains("[[expanded_in]]`: duplicate row for publication `monster-handbook`"),
            "{text}"
        );
    }

    #[test]
    fn expansion_rows_that_differ_in_page_or_via_are_not_duplicates() {
        // Same publication and `via`, different page.
        let mut entry = with_expansion(rule("ref/rule/a", vec![]), "monster-handbook", None);
        entry.expanded_in.push(cast_iron_schema::Expansion {
            publication: "monster-handbook".into(),
            page: Some("20".into()),
            via: None,
        });
        let report = check(&expansion_corpus(vec![entry]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);

        // Same publication and page, different `via`.
        let entry = with_expansion(rule("ref/rule/a", vec![]), "monster-handbook", None);
        let entry = with_expansion(entry, "monster-handbook", Some("ref/rule/b"));
        let via = in_publication(rule("ref/rule/b", vec![]), "monster-handbook");
        let report = check(&expansion_corpus(vec![entry, via]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn an_expansion_must_not_name_a_member_book() {
        // `rule` lists core-rulebook, so crediting it as an expansion is a membership leak.
        let entry = with_expansion(rule("ref/rule/a", vec![]), "core-rulebook", None);
        let text = check(&expansion_corpus(vec![entry])).problems.join("\n");
        assert!(
            text.contains("publication `core-rulebook` is listed in `source_ids`"),
            "{text}"
        );
    }

    #[test]
    fn an_expansion_via_entry_must_exist() {
        let entry = with_expansion(
            rule("ref/rule/a", vec![]),
            "monster-handbook",
            Some("ref/rule/gone"),
        );
        let text = check(&expansion_corpus(vec![entry])).problems.join("\n");
        assert!(
            text.contains("`via` entry `ref/rule/gone` does not exist"),
            "{text}"
        );
    }

    #[test]
    fn an_expansion_via_entry_must_be_printed_in_the_book() {
        let entry = with_expansion(
            rule("ref/rule/a", vec![]),
            "monster-handbook",
            Some("ref/rule/b"),
        );
        let via = rule("ref/rule/b", vec![]);
        let text = check(&expansion_corpus(vec![entry, via]))
            .problems
            .join("\n");
        assert!(
            text.contains("`via` entry `ref/rule/b` is not in publication `monster-handbook`"),
            "{text}"
        );
    }

    #[test]
    fn a_valid_expansion_passes() {
        let entry = with_expansion(
            rule("ref/rule/a", vec![]),
            "monster-handbook",
            Some("ref/rule/b"),
        );
        let via = in_publication(rule("ref/rule/b", vec![]), "monster-handbook");
        let report = check(&expansion_corpus(vec![entry, via]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let bare = with_expansion(rule("ref/rule/c", vec![]), "monster-handbook", None);
        let report = check(&expansion_corpus(vec![bare]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn citations_that_name_a_listed_publication_pass() {
        let entry = with_source(rule("ref/rule/a", vec![]), "core-rulebook", Some("12-14"));
        let report = check(&corpus(vec![entry]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn every_kind_and_label_pair_is_accepted_or_refused_as_listed() {
        use RelationKind as K;
        use RelationLabel as L;
        let target = EntryId::parse("ref/spell/base").unwrap();
        let problem = |kind: K, label: Option<L>| {
            relation_label_problem(&Relation {
                kind,
                target: target.clone(),
                label,
            })
        };
        // The accepted pairs, written out: every other pair of the 6 kinds and the 8 label
        // choices (none and the 7 labels) is refused.
        let accepted: &[(K, Option<L>)] = &[
            (K::Prerequisite, None),
            (K::Prerequisite, Some(L::Or)),
            (K::Part, None),
            (K::Part, Some(L::Alternate)),
            (K::Addon, Some(L::Mythic)),
            (K::Addon, Some(L::Stamina)),
            (K::Addon, Some(L::Changes)),
            (K::Revision, None),
            (K::Revision, Some(L::Removed)),
            (K::Revision, Some(L::Merged)),
            (K::Variant, None),
            (K::Archetype, None),
        ];
        let mut labels = vec![None];
        labels.extend(L::ALL.into_iter().map(Some));
        let mut refused = 0;
        for kind in K::ALL {
            for label in &labels {
                let listed = accepted.contains(&(kind, *label));
                assert_eq!(kind.allows(*label), listed, "{kind:?} {label:?}");
                let found = problem(kind, *label);
                assert_eq!(found.is_none(), listed, "{kind:?} {label:?}: {found:?}");
                refused += usize::from(!listed);
            }
        }
        assert_eq!(refused, 6 * 8 - accepted.len());
    }

    #[test]
    fn relation_labels_are_closed_per_kind() {
        let base = spell("ref/spell/base");
        let a = related(
            spell("ref/spell/a"),
            RelationKind::Prerequisite,
            "ref/spell/base",
            Some("alternate"),
        );
        let b = related(
            spell("ref/spell/b"),
            RelationKind::Addon,
            "ref/spell/base",
            None,
        );
        let c = related(
            spell("ref/spell/c"),
            RelationKind::Revision,
            "ref/spell/base",
            Some("or"),
        );
        let d = related(
            spell("ref/spell/d"),
            RelationKind::Variant,
            "ref/spell/base",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, a, b, c, d])).problems.join("\n");
        for expected in [
            "`alternate` is not a valid label on a `prerequisite` relation",
            "an `addon` relation needs exactly one of `mythic`, `stamina` or `changes`",
            "`or` is not a valid label on a `revision` relation",
            "`mythic` is not a valid label on a `variant` relation",
        ] {
            assert!(text.contains(expected), "{expected} missing from {text}");
        }
    }

    #[test]
    fn a_variant_goes_from_spell_to_spell_and_only_once() {
        let base = spell("ref/spell/base");
        let rule_variant = related(
            rule("ref/rule/a", vec![]),
            RelationKind::Variant,
            "ref/spell/base",
            None,
        );
        let twice = related(
            related(
                spell("ref/spell/b"),
                RelationKind::Variant,
                "ref/spell/base",
                None,
            ),
            RelationKind::Variant,
            "ref/spell/base",
            None,
        );
        let text = check(&corpus(vec![base, rule_variant, twice]))
            .problems
            .join("\n");
        assert!(
            text.contains("a `variant` relation must go from a spell to a spell"),
            "{text}"
        );
        assert!(
            text.contains("holds 2 `variant` relations; at most one is allowed"),
            "{text}"
        );
    }

    #[test]
    fn a_variant_cycle_is_an_error() {
        let a = related(
            spell("ref/spell/a"),
            RelationKind::Variant,
            "ref/spell/b",
            None,
        );
        let b = related(
            spell("ref/spell/b"),
            RelationKind::Variant,
            "ref/spell/c",
            None,
        );
        let c = related(
            spell("ref/spell/c"),
            RelationKind::Variant,
            "ref/spell/a",
            None,
        );
        let text = check(&corpus(vec![a, b, c])).problems.join("\n");
        assert!(
            text.contains(
                "`variant` relations form a cycle: `ref/spell/a` → `ref/spell/b` → `ref/spell/c` → `ref/spell/a`; remove one variant edge"
            ),
            "{text}"
        );
    }

    #[test]
    fn a_revision_target_must_be_a_non_revision_of_the_same_kind() {
        let base = feat("ref/feat/base", &[]);
        let first = in_publication(
            related(
                feat("ref/feat/base/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let second = in_publication(
            related(
                feat("ref/feat/base/two", &[]),
                RelationKind::Revision,
                "ref/feat/base/eitr",
                None,
            ),
            "eitr",
        );
        let wrong_kind = in_publication(
            related(
                spell("ref/spell/base/eitr"),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let mut c = corpus(vec![base, first, second, wrong_kind]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "a `revision` cannot point at `ref/feat/base/eitr`, which is itself a revision"
            ),
            "{text}"
        );
        assert!(
            text.contains("a `revision` must keep the target's kind"),
            "{text}"
        );
    }

    #[test]
    fn a_revision_must_come_from_another_book_and_only_once_per_book() {
        let base = feat("ref/feat/base", &[]);
        let same_book = related(
            feat("ref/feat/base/eitr", &[]),
            RelationKind::Revision,
            "ref/feat/base",
            None,
        );
        let text = check(&corpus(vec![base.clone(), same_book]))
            .problems
            .join("\n");
        assert!(text.contains("shares a book with it"), "{text}");

        let one = in_publication(
            related(
                feat("ref/feat/base/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let two = in_publication(
            related(
                feat("ref/feat/base/teitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let mut c = corpus(vec![base, one, two]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(text.contains("is a second revision"), "{text}");
    }

    #[test]
    fn a_revision_id_may_not_be_linked_or_named_as_a_prerequisite() {
        let base = feat("ref/feat/base", &[]);
        let revision = in_publication(
            related(
                feat("ref/feat/base/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                None,
            ),
            "eitr",
        );
        let linker = rule(
            "ref/rule/linker",
            vec![paragraph(vec![link(
                "ref/feat/base/eitr",
                None,
                "revision",
            )])],
        );
        let prereq = typed(
            rule("ref/feat/after", vec![]),
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                prerequisite_feats: vec![EntryId::parse("ref/feat/base/eitr").unwrap()],
                ..Default::default()
            }),
        );
        let mut c = corpus(vec![base, revision, linker, prereq]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("the body links `ref/feat/base/eitr`, which is a revision"),
            "{text}"
        );
        assert!(
            text.contains("`prerequisite_feats` names `ref/feat/base/eitr`, which is a revision"),
            "{text}"
        );
    }

    #[test]
    fn removed_and_merged_revisions_have_empty_fields_and_a_merged_link() {
        let base = feat("ref/feat/base", &[]);
        let removed = in_publication(
            related(
                typed(
                    rule("ref/feat/base/eitr", vec![]),
                    EntryKind::Feat,
                    TypedFields::Feat(FeatFields {
                        benefit: Some("not empty".into()),
                        ..Default::default()
                    }),
                ),
                RelationKind::Revision,
                "ref/feat/base",
                Some("removed"),
            ),
            "eitr",
        );
        let merged = in_publication(
            related(
                feat("ref/feat/merged/eitr", &[]),
                RelationKind::Revision,
                "ref/feat/base",
                Some("merged"),
            ),
            "eitr",
        );
        let target = feat("ref/feat/merged", &[]);
        let mut c = corpus(vec![base, removed, merged, target]);
        add_publication(&mut c, "eitr", Some("eitr"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("a `removed` revision must have empty `[fields]`"),
            "{text}"
        );
        assert!(
            text.contains("a `merged` revision must link the feat it merges into"),
            "{text}"
        );
    }

    #[test]
    fn a_mythic_add_on_must_be_a_mythic_feat() {
        let base = feat("ref/feat/base", &[]);
        let not_mythic = related(
            feat("ref/feat/base/mythic", &["Combat"]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, not_mythic])).problems.join("\n");
        assert!(
            text.contains("a `mythic` add-on must be a feat whose `feat_types` include `Mythic`"),
            "{text}"
        );
    }

    #[test]
    fn a_non_feat_mythic_holder_passes() {
        // The mythic-spell add-ons of slice 3 are `rule` entries under the spell they extend.
        let base = spell("ref/spell/heal");
        let holder = related(
            rule("ref/spell/heal/mythic", vec![]),
            RelationKind::Addon,
            "ref/spell/heal",
            Some("mythic"),
        );
        let report = check(&corpus(vec![base, holder]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn a_carried_mythic_twin_is_exempt_from_the_nested_id() {
        let base = feat("ref/feat/power-attack", &[]);
        let twin = typed(
            rule(
                "ref/carried/mythic-adventures/feats/power-attack/mythic",
                vec![],
            ),
            EntryKind::Feat,
            TypedFields::Feat(FeatFields {
                feat_types: vec!["Mythic".into()],
                ..Default::default()
            }),
        );
        let twin = related(
            twin,
            RelationKind::Addon,
            "ref/feat/power-attack",
            Some("mythic"),
        );
        let report = check(&corpus(vec![base.clone(), twin]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);

        let wrong = related(
            feat("ref/feat/elsewhere", &["Mythic"]),
            RelationKind::Addon,
            "ref/feat/power-attack",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, wrong])).problems.join("\n");
        assert!(
            text.contains("needs the id `ref/feat/power-attack/mythic`, not `ref/feat/elsewhere`"),
            "{text}"
        );
    }

    fn class_option(id: &str, group: &str) -> RuleEntry {
        typed(
            rule(id, vec![]),
            EntryKind::ClassOption,
            TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
                group: group.into(),
                ..Default::default()
            }),
        )
    }

    #[test]
    fn a_class_option_group_must_exist_resolve_its_classes_and_hold_an_entry() {
        let entry = class_option("ref/arcanedisc/a", "missing");
        let text = check(&corpus(vec![entry])).problems.join("\n");
        assert!(
            text.contains("class-option group `missing` is not declared in option-groups.toml"),
            "{text}"
        );

        let mut c = corpus(vec![class_option("ref/arcanedisc/a", "discoveries")]);
        c.option_groups.groups.insert(
            "discoveries".into(),
            crate::OptionGroupDef {
                label: "Discoveries".into(),
                singular: "Discovery".into(),
                classes: vec!["ref/class/ghost".into()],
            },
        );
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("group `discoveries` names unknown class `ref/class/ghost`"),
            "{text}"
        );

        let mut c = corpus(vec![]);
        c.option_groups.groups.insert(
            "empty".into(),
            crate::OptionGroupDef {
                label: "Empty".into(),
                singular: "Empty".into(),
                classes: vec![],
            },
        );
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("group `empty` holds no class_option entries"),
            "{text}"
        );
    }

    #[test]
    fn a_resolved_exclusion_must_name_entries_that_exist() {
        let mut c = corpus(vec![rule("ref/rule/a", vec![])]);
        c.exclusions.excluded.push(cast_iron_schema::Exclusion {
            path: "json/a.json#x".into(),
            records: 1,
            reason: "out of scope".into(),
        });
        let row = |entry: &str| crate::Resolution {
            path: "json/a.json#x".into(),
            supersedes: None,
            kind: None,
            entries: vec![entry.into()],
            reason: "ships. Refs #36.".into(),
        };
        c.exclusions.resolved = vec![row("ref/rule/a")];
        let text = check(&c).problems.join("\n");
        assert!(!text.contains("resolved row"), "{text}");

        c.exclusions.resolved = vec![row("ref/rule/missing")];
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("resolved row for `json/a.json#x` names entry `ref/rule/missing`"),
            "{text}"
        );
    }

    #[test]
    fn a_path_that_does_not_match_its_id_is_an_error() {
        let mut c = corpus(vec![rule("ref/rule/a", vec![])]);
        c.entries.get_mut("ref/rule/a").unwrap().path =
            std::path::PathBuf::from("entries/rule/wrong.md");
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "entry id `ref/rule/a` belongs at `entries/rule/a.md`, not `entries/rule/wrong.md`"
            ),
            "{text}"
        );
    }

    #[test]
    fn an_add_on_and_a_revision_take_the_expected_suffix() {
        let base = feat("ref/feat/base", &[]);
        let good = related(
            feat("ref/feat/base/mythic", &["Mythic"]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("mythic"),
        );
        let bad = related(
            feat("ref/feat/wrong", &["Mythic"]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("mythic"),
        );
        let text = check(&corpus(vec![base, good, bad])).problems.join("\n");
        assert!(
            text.contains("needs the id `ref/feat/base/mythic`, not `ref/feat/wrong`"),
            "{text}"
        );

        let base = feat("ref/feat/base", &[]);
        let changed = related(
            feat("ref/feat/base/teitr", &[]),
            RelationKind::Addon,
            "ref/feat/base",
            Some("changes"),
        );
        let text = check(&corpus(vec![base, changed])).problems.join("\n");
        assert!(
            text.contains("publication `core-rulebook` has no `suffix` in publications.toml"),
            "{text}"
        );

        let base = feat("ref/feat/base", &[]);
        let changed = in_publication(
            related(
                feat("ref/feat/base/teitr", &[]),
                RelationKind::Addon,
                "ref/feat/base",
                Some("changes"),
            ),
            "eitr",
        );
        let mut c = corpus(vec![base.clone(), changed]);
        add_publication(&mut c, "eitr", Some("teitr"));
        assert!(check(&c).problems.is_empty(), "{:?}", check(&c).problems);

        let mut c = corpus(vec![base, feat("ref/feat/other", &[])]);
        add_publication(&mut c, "eitr", Some("teitr"));
        add_publication(&mut c, "other", Some("teitr"));
        let text = check(&c).problems.join("\n");
        assert!(text.contains("share the suffix `teitr`"), "{text}");
    }

    #[test]
    fn a_publication_suffix_must_be_one_id_segment() {
        let mut c = corpus(vec![]);
        add_publication(&mut c, "eitr", Some("Bad Suffix"));
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains("has suffix `Bad Suffix`, which is not one id segment"),
            "{text}"
        );
    }

    #[test]
    fn an_id_under_an_entry_that_ends_in_a_suffix_must_hold_the_edge() {
        let base = feat("ref/feat/base", &[]);
        let orphan = feat("ref/feat/base/mythic", &["Mythic"]);
        let text = check(&corpus(vec![base, orphan])).problems.join("\n");
        assert!(
            text.contains("id ends in `/mythic` under `ref/feat/base`"),
            "{text}"
        );
    }

    #[test]
    fn replaces_and_alters_drift_against_the_printed_lines() {
        let archetype = typed(
            rule(
                "ref/arc/example",
                vec![paragraph(vec![
                    Inline::Strong {
                        c: vec![Inline::text("Replaces:")],
                    },
                    Inline::text(" Uncanny dodge."),
                ])],
            ),
            EntryKind::Archetype,
            TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
                trades: vec![Trade::unresolved(TradeOp::Replaces, "Trap sense")],
                ..Default::default()
            }),
        );
        let text = check(&corpus(vec![archetype])).problems.join("\n");
        assert!(
            text.contains(
                "[fields] replaces = \"Trap sense\" disagrees with the `**Replaces:**` line"
            ),
            "{text}"
        );

        let option = typed(
            rule(
                "ref/arcanedisc/example",
                vec![paragraph(vec![
                    Inline::Strong {
                        c: vec![Inline::text("Replaces:")],
                    },
                    Inline::text(" Arcane bond."),
                ])],
            ),
            EntryKind::ClassOption,
            TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
                group: "discoveries".into(),
                trades: vec![Trade::unresolved(TradeOp::Replaces, "Arcane school")],
                ..Default::default()
            }),
        );
        let mut c = corpus(vec![option]);
        c.option_groups.groups.insert(
            "discoveries".into(),
            crate::OptionGroupDef {
                label: "Discoveries".into(),
                singular: "Discovery".into(),
                classes: vec![],
            },
        );
        let text = check(&c).problems.join("\n");
        assert!(
            text.contains(
                "[fields] replaces = \"Arcane school\" disagrees with the `**Replaces:**` line"
            ),
            "{text}"
        );
    }

    fn printed(field: &str, text: &str) -> Block {
        paragraph(vec![
            Inline::Strong {
                c: vec![Inline::text(format!("{field}:"))],
            },
            Inline::text(format!(" {text}")),
        ])
    }

    fn fighter_class() -> RuleEntry {
        typed(
            rule(
                "ref/class/fighter",
                vec![
                    heading(Some("bravery"), "Bravery (Ex)"),
                    heading(Some("trap-sense"), "Trap Sense (Ex)"),
                ],
            ),
            EntryKind::Class,
            TypedFields::Class(Default::default()),
        )
    }

    fn trade(op: TradeOp, printed: &str, anchor: Option<&str>) -> Trade {
        Trade {
            op,
            printed: printed.into(),
            feature: anchor.map(|anchor| FeatureRef {
                anchor: anchor.into(),
                ..Default::default()
            }),
        }
    }

    fn fighter_archetype_with(
        trades: Vec<Trade>,
        legacy: cast_iron_schema::LegacyFeatureLists,
        printed_line: &str,
    ) -> RuleEntry {
        let mut entry = typed(
            rule(
                "ref/arc-fighter/example",
                vec![printed("Replaces", printed_line)],
            ),
            EntryKind::Archetype,
            TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
                trades,
                legacy_features: legacy,
            }),
        );
        entry.relations = vec![Relation {
            kind: RelationKind::Archetype,
            target: EntryId::parse("ref/class/fighter").unwrap(),
            label: None,
        }];
        entry
    }

    fn fighter_archetype(trades: Vec<Trade>, printed_line: &str) -> RuleEntry {
        fighter_archetype_with(trades, Default::default(), printed_line)
    }

    #[test]
    fn trade_features_accept_class_anchors_in_print_order() {
        let report = check(&corpus(vec![
            fighter_class(),
            fighter_archetype(
                vec![
                    trade(TradeOp::Replaces, "Bravery", Some("bravery")),
                    trade(TradeOp::Replaces, "Trap sense", Some("trap-sense")),
                ],
                "Bravery, Trap sense.",
            ),
        ]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    #[test]
    fn a_trade_feature_must_be_an_anchor_of_the_base_class() {
        let report = check(&corpus(vec![
            fighter_class(),
            fighter_archetype(
                vec![trade(TradeOp::Replaces, "Bravery", Some("weapon-mastery"))],
                "Bravery.",
            ),
        ]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].contains(
                "the replaces trade `Bravery` has feature `weapon-mastery`, which is not a heading anchor in `ref/class/fighter`"
            ),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn a_trade_class_must_be_a_base_class_that_holds_the_anchor() {
        let mut named = trade(TradeOp::Replaces, "Bravery", Some("bravery"));
        named.feature.as_mut().unwrap().class = Some(EntryId::parse("ref/class/rogue").unwrap());
        let report = check(&corpus(vec![
            fighter_class(),
            fighter_archetype(vec![named], "Bravery."),
        ]));
        assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
        assert!(
            report.problems[0].contains("names class `ref/class/rogue`, which is not a base class"),
            "{:?}",
            report.problems
        );
        let mut right = trade(TradeOp::Replaces, "Bravery", Some("bravery"));
        right.feature.as_mut().unwrap().class = Some(EntryId::parse("ref/class/fighter").unwrap());
        let report = check(&corpus(vec![
            fighter_class(),
            fighter_archetype(vec![right], "Bravery."),
        ]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn a_trade_feature_that_disagrees_with_the_printed_name_is_a_warning() {
        let report = check(&corpus(vec![
            fighter_class(),
            fighter_archetype(
                vec![trade(TradeOp::Replaces, "Bravery", Some("trap-sense"))],
                "Bravery.",
            ),
        ]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(
            report.warnings[0].contains(
                "the replaces trade `Bravery` has feature `trap-sense`, but the printed name resolves to `bravery`"
            ),
            "{:?}",
            report.warnings
        );
    }

    #[test]
    fn a_format_1_list_that_did_not_line_up_is_checked_as_a_list() {
        let legacy = cast_iron_schema::LegacyFeatureLists {
            replaces: vec!["bravery".into()],
            alters: vec![],
        };
        let entry = || {
            fighter_archetype_with(
                vec![
                    trade(TradeOp::Replaces, "Bravery", None),
                    trade(TradeOp::Replaces, "Trap sense", None),
                ],
                legacy.clone(),
                "Bravery, Trap sense.",
            )
        };
        let report = check(&corpus_in_format(1, vec![fighter_class(), entry()]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        // The second warning is the count of unresolved trades (#451).
        assert_eq!(report.warnings.len(), 2, "{:?}", report.warnings);
        assert!(
            report.warnings[0].contains(
                "[fields] replaces_features = [\"bravery\"] disagrees with the `**Replaces:**` line; the confident matches in print order are [\"bravery\", \"trap-sense\"]"
            ),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings[1].contains("2 of 2 trades in 1 entries are unresolved"),
            "{:?}",
            report.warnings
        );
        let mut bad = legacy.clone();
        bad.replaces = vec!["weapon-mastery".into()];
        let report = check(&corpus_in_format(
            1,
            vec![
                fighter_class(),
                fighter_archetype_with(
                    vec![trade(TradeOp::Replaces, "Bravery", None)],
                    bad,
                    "Bravery.",
                ),
            ],
        ));
        assert!(
            report.problems.iter().any(|problem| problem.contains(
                "[fields] replaces_features key `weapon-mastery` is not a heading anchor in `ref/class/fighter`"
            )),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn a_format_1_anchor_list_with_no_printed_trades_is_still_checked() {
        let legacy = cast_iron_schema::LegacyFeatureLists {
            replaces: vec!["missing-anchor".into()],
            alters: vec![],
        };
        let entry = fighter_archetype_with(vec![], legacy, "Bravery.");
        let report = check(&corpus_in_format(1, vec![fighter_class(), entry]));
        assert!(
            report.problems.iter().any(|problem| problem.contains(
                "[fields] replaces_features key `missing-anchor` is not a heading anchor in `ref/class/fighter`"
            )),
            "{:?}",
            report.problems
        );
        // With no base class at all, the retained anchors are an error too.
        let mut orphan = fighter_archetype_with(
            vec![],
            cast_iron_schema::LegacyFeatureLists {
                replaces: vec!["bravery".into()],
                alters: vec![],
            },
            "Bravery.",
        );
        orphan.relations.clear();
        let report = check(&corpus_in_format(1, vec![fighter_class(), orphan]));
        assert!(
            report
                .problems
                .iter()
                .any(|problem| problem.contains("this entry has no base class entry")),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn corpus_format_1_refuses_a_feature_that_is_not_the_slug_of_its_name() {
        let report = check(&corpus_in_format(
            1,
            vec![
                fighter_class(),
                fighter_archetype(
                    vec![trade(TradeOp::Replaces, "Bravery", Some("trap-sense"))],
                    "Bravery.",
                ),
            ],
        ));
        assert!(
            report
                .problems
                .iter()
                .any(|problem| problem.contains("which corpus format 1 cannot write")),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn corpus_format_1_refuses_a_trade_it_cannot_write() {
        let mut narrowed = trade(TradeOp::Replaces, "Bravery", Some("bravery"));
        narrowed.feature.as_mut().unwrap().levels = vec![2];
        let report = check(&corpus_in_format(
            1,
            vec![
                fighter_class(),
                fighter_archetype(vec![narrowed], "Bravery."),
            ],
        ));
        assert!(
            report
                .problems
                .iter()
                .any(|problem| problem.contains("which corpus format 1 cannot write")),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn unresolved_printed_names_and_ability_tags_are_not_drift() {
        let report = check(&corpus(vec![
            fighter_class(),
            fighter_archetype(
                vec![
                    trade(TradeOp::Replaces, "Bravery (Ex)", Some("bravery")),
                    trade(TradeOp::Replaces, "The 3rd-level bonus feat", None),
                ],
                "Bravery (Ex), The 3rd-level bonus feat.",
            ),
        ]));
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        // Only the count of unresolved trades (#451): the printed name is no drift.
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(
            report.warnings[0].contains("1 of 2 trades in 1 entries are unresolved"),
            "{:?}",
            report.warnings
        );
    }

    #[test]
    fn class_option_features_resolve_through_the_group_classes() {
        let wizard = typed(
            rule(
                "ref/class/wizard",
                vec![heading(Some("arcane-bond"), "Arcane Bond (Ex or Sp)")],
            ),
            EntryKind::Class,
            TypedFields::Class(Default::default()),
        );
        let option = typed(
            rule(
                "ref/arcanedisc/example",
                vec![printed("Replaces", "Arcane bond.")],
            ),
            EntryKind::ClassOption,
            TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
                group: "discoveries".into(),
                trades: vec![trade(TradeOp::Replaces, "Arcane bond", Some("arcane-bond"))],
                ..Default::default()
            }),
        );
        let mut c = corpus(vec![wizard, option]);
        c.option_groups.groups.insert(
            "discoveries".into(),
            crate::OptionGroupDef {
                label: "Discoveries".into(),
                singular: "Discovery".into(),
                classes: vec!["ref/class/wizard".into()],
            },
        );
        let report = check(&c);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    fn split(mut corpus: Corpus, trees: Vec<BookTree>) -> Corpus {
        corpus.layout = BookLayout::Split;
        corpus.config.format = SPLIT_BOOKS_FORMAT;
        corpus.book_texts = trees
            .iter()
            .map(|tree| (tree.source.clone(), crate::files::write_curated_book(tree)))
            .collect();
        corpus.books = BookIndex { books: trees };
        corpus
    }

    fn tree_with(chapter_titles: &[&str]) -> BookTree {
        BookTree {
            source: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            basis: String::new(),
            review: vec![],
            chapters: chapter_titles
                .iter()
                .map(|title| BookNode {
                    title: title.to_string(),
                    entry: None,
                    anchor: None,
                    review: None,
                    collapse: false,
                    lists_children: false,
                    children: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn a_curated_file_may_not_hold_a_chapter_the_compiler_owns() {
        let corpus = split(
            corpus(vec![rule("ref/rule/a", vec![])]),
            vec![tree_with(&[
                "Introduction",
                "Page order",
                "Unpaged",
                "New entries (page order)",
            ])],
        );
        let report = check(&corpus);
        let problems: Vec<_> = report
            .problems
            .iter()
            .filter(|p| p.contains("may not hold"))
            .collect();
        assert_eq!(problems.len(), 3, "{:?}", report.problems);
        assert!(
            problems[0].starts_with("books/core-rulebook.toml:"),
            "{}",
            problems[0]
        );
        // The single layout still stores these chapters.
        let mut single = corpus.clone();
        single.layout = BookLayout::Single;
        let report = check(&single);
        assert!(
            report.problems.iter().all(|p| !p.contains("may not hold")),
            "{:?}",
            report.problems
        );
    }

    #[test]
    fn a_polluted_page_is_a_problem_in_the_split_layout_and_a_warning_before() {
        let mut entry = rule("ref/rule/a", vec![]);
        entry.sources = vec![
            Citation {
                publication: "core-rulebook".into(),
                page: Some("58~Occult Origins/8".into()),
            },
            Citation {
                publication: "core-rulebook".into(),
                page: Some("12-14".into()),
            },
        ];
        let single = corpus(vec![entry]);
        let report = check(&single);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(
            report
                .warnings
                .iter()
                .filter(|w| w.contains("is not a page"))
                .count(),
            1,
            "{:?}",
            report.warnings
        );
        let report = check(&split(single, vec![]));
        let problems: Vec<_> = report
            .problems
            .iter()
            .filter(|p| p.contains("is not a page"))
            .collect();
        assert_eq!(problems.len(), 1, "{:?}", report.problems);
        assert!(
            problems[0].contains("entries/rule/a.md:"),
            "{}",
            problems[0]
        );
    }

    #[test]
    fn a_split_corpus_below_format_3_only_warns() {
        let mut corpus = split(corpus(vec![rule("ref/rule/a", vec![])]), vec![]);
        corpus.config.format = 2;
        let report = check(&corpus);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("holds the split")),
            "{:?}",
            report.warnings
        );
    }
}
