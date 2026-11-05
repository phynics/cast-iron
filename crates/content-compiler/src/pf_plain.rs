//! Plain display text for Pathfinder Data field strings.
//!
//! Body text goes through the Markdown renderer, which turns `‹kind/name›` tags into typed
//! links. Field strings (prerequisites, benefit, melee, school, ...) are plain strings, so
//! the same tags must reduce to their display text. This module reuses
//! [`pf_markdown::link_parts`], the parser the body renderer uses, so a tag reads the same
//! in both places, malformed forms included.
use crate::pf_markdown::link_parts;
use cast_iron_schema::{Block, Inline, TableRow, TypedFields};

const OPEN: char = '‹';
const CLOSE: char = '›';

/// Reduce Pathfinder Data inline markup in a field string to display text.
///
/// - `‹spelldef/transmutation›` becomes `transmutation`; display overrides such as
///   `‹rule/new>Unchained» Skill<s›` resolve the way the body renderer resolves them.
/// - `‹SOURCE Book/12›` becomes `Book pg 12`.
/// - `&#91&` and `&#93&` become `[` and `]`; other numeric `&#N&`, `&#N;` and `&#N`
///   entities decode to their character.
/// - An unterminated `‹` or a stray `›` is dropped, never left in the output.
pub fn plain_text(input: &str) -> String {
    let decoded = decode_numeric_entities(input);
    let mut out = String::with_capacity(decoded.len());
    let mut rest = decoded.as_str();
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&strip_stray_close(&rest[..start]));
        let after = &rest[start + OPEN.len_utf8()..];
        // `‹‹label/#anchor››` and `‹‹label/https://...››` are in-page and outside links.
        if let Some(nested) = after.strip_prefix(OPEN)
            && let Some(end) = nested.find("››")
        {
            out.push_str(&double_tag_text(&nested[..end]));
            rest = &nested[end + 2 * CLOSE.len_utf8()..];
            continue;
        }
        let Some(end) = after.find(CLOSE) else {
            // Unterminated tag: keep its words, lose the marker.
            rest = after;
            continue;
        };
        out.push_str(&tag_text(&after[..end]));
        rest = &after[end + CLOSE.len_utf8()..];
    }
    out.push_str(&strip_stray_close(rest));
    out
}

fn strip_stray_close(text: &str) -> String {
    text.replace(CLOSE, "")
}

fn double_tag_text(inner: &str) -> String {
    let label_end = inner
        .find("/#")
        .or_else(|| inner.find("/http"))
        .or_else(|| inner.rfind('/'));
    match label_end {
        Some(i) => inner[..i].to_string(),
        None => inner.to_string(),
    }
}

fn tag_text(inner: &str) -> String {
    if let Some(sources) = inner.strip_prefix("SOURCE ") {
        return sources
            .split(';')
            .filter_map(|part| {
                let part = part.trim();
                let (name, pages) = part
                    .split_once('/')
                    .map_or((part, None), |(n, p)| (n.trim(), Some(p.trim())));
                if name.is_empty() {
                    return None;
                }
                Some(match pages {
                    Some(p) if p.contains('-') || p.contains(',') => format!("{name} pg. {p}"),
                    Some(p) => format!("{name} pg {p}"),
                    None => name.to_string(),
                })
            })
            .collect::<Vec<_>>()
            .join(", ");
    }
    let (href, display) = link_parts(&format!("{OPEN}{inner}{CLOSE}"));
    if href.contains('/') {
        display
    } else {
        // No `kind/name` shape: the tag is just bracketed text.
        inner.to_string()
    }
}

/// Decode `&#91&`, `&#93&`, `&#42;`, `&#x27A4` and similar numeric entities. The upstream
/// writes a closing `&` where HTML writes `;`, and sometimes neither.
fn decode_numeric_entities(input: &str) -> String {
    map_numeric_entities(input, |c| c)
}

/// Replace each numeric entity with `map(character)`. An `&#` that does not start an
/// entity loses its introducer, so no marker survives.
fn map_numeric_entities(input: &str, map: impl Fn(char) -> char) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("&#") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let (radix, digits_from) = match after.strip_prefix(['x', 'X']) {
            Some(_) => (16, 1),
            None => (10, 0),
        };
        let body = &after[digits_from..];
        let len = body.chars().take_while(|c| c.is_digit(radix)).count();
        let decoded = (len > 0)
            .then(|| u32::from_str_radix(&body[..len], radix).ok())
            .flatten()
            .and_then(char::from_u32);
        match decoded {
            Some(c) => {
                out.push(map(c));
                let tail = &body[len..];
                rest = tail
                    .strip_prefix('&')
                    .or_else(|| tail.strip_prefix(';'))
                    .unwrap_or(tail);
            }
            None => rest = after,
        }
    }
    out.push_str(rest);
    out
}

/// First private-use code point of the entity placeholders. Entity number `n` is carried
/// as `char(PLACEHOLDER_BASE + n)` while the Markdown renderer runs.
const PLACEHOLDER_BASE: u32 = 0xF0000;
const PLACEHOLDER_MAX: u32 = 0xFFFD;

/// Render body lines with [`crate::pf_markdown::render`], decoding numeric entities such as
/// `&#91&` in the typed text it produces.
///
/// The entities cannot be decoded before rendering: `&#42&` would become `*` and open
/// emphasis, and `&#91&` would become `[` and open a link. Each one is swapped for a
/// private-use placeholder, which Markdown treats as an ordinary letter, and swapped back
/// to the real character in every text node afterwards.
pub fn render_body(
    lines: &[String],
    env: &mut dyn crate::html::Env,
) -> Result<Vec<Block>, Vec<String>> {
    let protected: Vec<String> = lines.iter().map(|l| protect_entities(l)).collect();
    let mut blocks = crate::pf_markdown::render(&protected, env)?;
    for block in &mut blocks {
        restore_block(block);
    }
    Ok(blocks)
}

fn protect_entities(line: &str) -> String {
    map_numeric_entities(line, |c| {
        let n = c as u32;
        if n <= PLACEHOLDER_MAX {
            char::from_u32(PLACEHOLDER_BASE + n).unwrap_or(c)
        } else {
            c
        }
    })
}

/// Restore entity placeholders, then reduce any inline tag the renderer left in the text
/// (for example in a directive attribute) to its display text.
fn restore_text(text: &mut String) {
    restore_placeholders(text);
    if text.contains([OPEN, CLOSE]) || text.contains("&#") {
        *text = plain_text(text);
    }
}

fn restore_placeholders(text: &mut String) {
    if text
        .chars()
        .any(|c| (PLACEHOLDER_BASE..=PLACEHOLDER_BASE + PLACEHOLDER_MAX).contains(&(c as u32)))
    {
        *text = text
            .chars()
            .map(|c| {
                let n = c as u32;
                if (PLACEHOLDER_BASE..=PLACEHOLDER_BASE + PLACEHOLDER_MAX).contains(&n) {
                    char::from_u32(n - PLACEHOLDER_BASE).unwrap_or(c)
                } else {
                    c
                }
            })
            .collect();
    }
}

fn restore_inlines(inlines: &mut [Inline]) {
    for inline in inlines {
        match inline {
            Inline::Text { v } => restore_text(v),
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                restore_inlines(c)
            }
            Inline::Link { c, .. } => restore_inlines(c),
            Inline::Break => {}
        }
    }
}

fn restore_rows(rows: &mut [TableRow]) {
    for row in rows {
        for cell in &mut row.cells {
            restore_inlines(&mut cell.content);
        }
    }
}

fn restore_block(block: &mut Block) {
    match block {
        Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
            restore_inlines(content)
        }
        Block::List { items, .. } => {
            for item in items {
                for inner in item {
                    restore_block(inner);
                }
            }
        }
        Block::Table(table) => {
            restore_inlines(&mut table.caption);
            restore_rows(&mut table.head);
            restore_rows(&mut table.body);
            restore_rows(&mut table.foot);
        }
        Block::Quote { content } => {
            for inner in content {
                restore_block(inner);
            }
        }
        Block::StatBlock { title, rows } => {
            if let Some(title) = title {
                restore_text(title);
            }
            for row in rows {
                restore_text(&mut row.label);
                restore_inlines(&mut row.value);
            }
        }
        Block::EntryLink { title, .. } => restore_text(title),
    }
}

/// Where Pathfinder Data inline markup is left in `text`: each `‹`, `›` or `&#`, with
/// a few characters of context. Empty when the text is clean.
pub fn markup_leaks(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, _) in text
        .match_indices(['‹', '›'])
        .chain(text.match_indices("&#"))
    {
        let from = text[..index].char_indices().rev().nth(30).map_or(0, |(i, _)| i);
        let to = text[index..]
            .char_indices()
            .nth(40)
            .map_or(text.len(), |(i, _)| index + i);
        found.push(text[from..to].replace('\n', " "));
    }
    found
}

/// Apply [`plain_text`] to every free-text field of a typed record.
///
/// Field types stay strings, so the serialized shape does not change.
pub fn plain_fields(fields: TypedFields) -> TypedFields {
    fn opt(value: &mut Option<String>) {
        if let Some(v) = value {
            *v = plain_text(v);
        }
    }
    fn list(values: &mut [String]) {
        for v in values {
            *v = plain_text(v);
        }
    }
    let mut fields = fields;
    match &mut fields {
        TypedFields::Spell(f) => {
            f.school = plain_text(&f.school);
            list(&mut f.subschools);
            list(&mut f.descriptors);
            for level in &mut f.levels {
                level.class = plain_text(&level.class);
            }
            opt(&mut f.casting_time);
            opt(&mut f.components);
            opt(&mut f.range);
            for (k, v) in &mut f.effects {
                *k = plain_text(k);
                *v = plain_text(v);
            }
            opt(&mut f.duration);
            opt(&mut f.saving_throw);
            opt(&mut f.spell_resistance);
        }
        TypedFields::Feat(f) => {
            list(&mut f.feat_types);
            opt(&mut f.prerequisites);
            opt(&mut f.benefit);
            opt(&mut f.normal);
            opt(&mut f.special);
        }
        TypedFields::Creature(f) => {
            opt(&mut f.creature_type);
            list(&mut f.subtypes);
            opt(&mut f.size);
            opt(&mut f.alignment);
            opt(&mut f.ac);
            opt(&mut f.hp);
            opt(&mut f.fortitude);
            opt(&mut f.reflex);
            opt(&mut f.will);
            opt(&mut f.melee);
            opt(&mut f.ranged);
            opt(&mut f.special_attacks);
            opt(&mut f.special_qualities);
            if let Some(scores) = &mut f.abilities {
                list(scores);
            }
        }
        TypedFields::Item(f) => {
            opt(&mut f.category);
            opt(&mut f.price);
            opt(&mut f.weight);
            opt(&mut f.slot);
            opt(&mut f.aura);
            opt(&mut f.caster_level);
            for (k, v) in &mut f.attributes {
                *k = plain_text(k);
                *v = plain_text(v);
            }
        }
        TypedFields::Class(f) => {
            opt(&mut f.class_type);
            opt(&mut f.hit_die);
            opt(&mut f.skill_ranks);
            opt(&mut f.class_skills);
        }
        TypedFields::Trait(f) => {
            f.category = plain_text(&f.category);
            opt(&mut f.detail);
        }
        TypedFields::RacialTrait(_) | TypedFields::None => {}
    }
    fields
}

/// The facet value for a spell school: the school word without a trailing bracketed note,
/// so `transmutation [see text]` counts under `transmutation`.
pub fn school_facet(school: &str) -> String {
    let base = school.split('[').next().unwrap_or(school);
    base.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_becomes_its_display_text() {
        assert_eq!(plain_text("‹spelldef/transmutation›"), "transmutation");
        assert_eq!(
            plain_text("Caster level 4th, ‹race/gillman›, ‹ability/favored enemy›"),
            "Caster level 4th, gillman, favored enemy"
        );
    }

    #[test]
    fn a_tag_with_display_overrides_resolves_like_the_body() {
        // `«text»` is display-only text and `<slug>` is link-only text.
        assert_eq!(plain_text("‹rule/un»loved« foods›"), "unloved foods");
        assert_eq!(plain_text("‹rule/starting<_a> business›"), "starting business");
    }

    #[test]
    fn entity_brackets_become_brackets() {
        assert_eq!(
            plain_text("‹spelldef/transmutation› &#91&see text&#93&"),
            "transmutation [see text]"
        );
        assert_eq!(plain_text("a &#91&b&#93& c"), "a [b] c");
    }

    #[test]
    fn other_numeric_entities_decode_with_or_without_a_terminator() {
        assert_eq!(plain_text("Str 13&#42&"), "Str 13*");
        assert_eq!(plain_text("Str 13&#42;"), "Str 13*");
        assert_eq!(plain_text("Str 13&#42"), "Str 13*");
        assert_eq!(plain_text("go &#x27A4 on"), "go \u{27A4} on");
        assert_eq!(plain_text("bare &# here"), "bare  here");
    }

    #[test]
    fn malformed_unchained_skill_tag_resolves_to_display_text() {
        let text = plain_text("‹rule/new>Unchained» Skill<s›");
        assert_eq!(text, "Unchained Skill");
        assert!(!text.contains(['‹', '›', '<', '>', '«', '»']));
    }

    #[test]
    fn malformed_sanity_system_tag_resolves_to_display_text() {
        let text = plain_text("‹rule/sanity« system›");
        assert_eq!(text, "sanity system");
        assert!(!text.contains(['‹', '›', '<', '>', '«', '»']));
    }

    #[test]
    fn double_bracket_links_keep_their_label() {
        assert_eq!(
            plain_text("‹‹electricity resistance/#res›› power of the Air domain"),
            "electricity resistance power of the Air domain"
        );
        assert_eq!(
            plain_text("‹‹From the FAQ/https://paizo.com/paizo/faq/v5748nruor1fn#v5748eaic9raj››: The lack"),
            "From the FAQ: The lack"
        );
    }

    #[test]
    fn unterminated_and_stray_markers_never_survive() {
        assert_eq!(plain_text("see ‹spell/fly"), "see spell/fly");
        assert_eq!(plain_text("stray ◊ › end"), "stray ◊  end");
        assert_eq!(plain_text("‹bracketed words›"), "bracketed words");
    }

    #[test]
    fn source_tags_name_the_book_and_page() {
        assert_eq!(plain_text("‹SOURCE Core Rulebook/12›"), "Core Rulebook pg 12");
    }

    #[test]
    fn plain_text_is_unchanged_and_deterministic() {
        let s = "Dex 13; Dodge; Weapon Focus (longsword)";
        assert_eq!(plain_text(s), s);
        let t = "‹feat/Iron Will› and ‹feat/Dodge›";
        assert_eq!(plain_text(t), plain_text(t));
    }

    #[test]
    fn body_entities_decode_without_opening_markdown() {
        let mut env = crate::html::NullEnv;
        let lines = vec![
            "Bonus feats marked with an asterisk (&#42&) apply. Skill Focus (Knowledge &#91&history&#93&) works.".to_string(),
        ];
        let blocks = render_body(&lines, &mut env).unwrap();
        let text = crate::text::blocks_text(&blocks);
        assert!(
            text.contains("asterisk (*) apply") && text.contains("Knowledge [history])"),
            "{text}"
        );
        assert!(markup_leaks(&serde_json::to_string(&blocks).unwrap()).is_empty());
    }

    #[test]
    fn markup_leaks_reports_each_marker_with_context() {
        assert!(markup_leaks("clean [text] & more").is_empty());
        assert_eq!(markup_leaks("a ‹b› c &#91& d").len(), 3);
    }

    #[test]
    fn school_facet_drops_the_bracketed_note() {
        assert_eq!(school_facet("transmutation [see text]"), "transmutation");
        assert_eq!(school_facet("Transmutation [see text]"), "Transmutation");
        assert_eq!(school_facet("evocation"), "evocation");
        assert_eq!(school_facet("  conjuration   [healing]  "), "conjuration");
    }

    #[test]
    fn plain_fields_cleans_every_string_in_a_record() {
        let fields = TypedFields::Feat(cast_iron_schema::FeatFields {
            prerequisites: Some("Caster level 4th, ‹race/gillman›".into()),
            benefit: Some("+2 &#91&bonus&#93&".into()),
            ..Default::default()
        });
        let TypedFields::Feat(f) = plain_fields(fields) else {
            unreachable!()
        };
        assert_eq!(f.prerequisites.as_deref(), Some("Caster level 4th, gillman"));
        assert_eq!(f.benefit.as_deref(), Some("+2 [bonus]"));
    }
}
