//! The layout plan of a class option's powers (decisions `2026-11-20-class-option-power-layout`
//! and `2026-11-21-class-option-power-owner-cases`).
//!
//! The classifier, the candidate test, the validation, the one-power test and the heading level
//! are the functions of [`cast_iron_schema::power`]; nothing here classifies a block. This module
//! only decides what the renderer does with the result: which blocks the layout replaces, whether
//! the power head folds into the entry title, and the words of the meta and facts lines. It is
//! pure, so it has native tests.

use cast_iron_schema::headings::plain_text;
use cast_iron_schema::power::{
    Power, candidates, one_power_page, power_heading_level, split_ability_tag, validate_power,
};
use cast_iron_schema::{Block, Inline};
use std::collections::BTreeMap;

/// One validated power that the renderer lays out in place of its blocks.
pub struct Placed<'a> {
    pub power: Power<'a>,
    /// How many top-level blocks the power covers, its name included.
    pub len: usize,
    /// The level of the name's heading element (2 to 6).
    pub heading: u8,
    /// A one-power page: the name and the meta line live in the entry head, not in the body.
    pub folded: bool,
}

/// The powers of one body, found by the shared functions, keyed by the name block they start at.
/// A key is the address of the block in the body, as the heading keys of the renderer are.
#[derive(Default)]
pub struct Plan<'a> {
    by_start: BTreeMap<*const Block, Placed<'a>>,
}

impl<'a> Plan<'a> {
    /// The power that starts at `block`, if the layout applies to it.
    pub fn at(&self, block: &Block) -> Option<&Placed<'a>> {
        self.by_start.get(&std::ptr::from_ref(block))
    }
}

/// Lay out the powers of a class option's `body`. A candidate that fails validation, and every
/// block that is no candidate (a section label such as `Domain Spells` included), is left out, so
/// it renders as it did before.
pub fn plan<'a>(title: &str, body: &'a [Block]) -> Plan<'a> {
    let mut by_start = BTreeMap::new();
    let found = candidates(body);
    if let (Some(power), Some(first)) = (one_power_page(title, body), found.first()) {
        by_start.insert(
            std::ptr::from_ref(&body[first.start]),
            Placed {
                power,
                len: first.end - first.start,
                heading: 2,
                folded: true,
            },
        );
        return Plan { by_start };
    }
    for candidate in &found {
        if let Ok(power) = validate_power(candidate.blocks(body)) {
            by_start.insert(
                std::ptr::from_ref(&body[candidate.start]),
                Placed {
                    power,
                    len: candidate.end - candidate.start,
                    heading: power_heading_level(body, candidate.start),
                    folded: false,
                },
            );
        }
    }
    Plan { by_start }
}

/// The ability tag in words with the printed tag: `Supernatural (Su)`, `Extraordinary or
/// Supernatural (Ex or Su)`.
fn tag_in_words(tag: &str) -> String {
    const WORDS: [(&str, &str); 3] = [
        ("Ex", "Extraordinary"),
        ("Su", "Supernatural"),
        ("Sp", "Spell-Like"),
    ];
    let inner = tag
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or(tag);
    let mut words = String::new();
    let mut rest = inner;
    while let Some(next) = rest.chars().next() {
        match WORDS.iter().find(|(short, _)| rest.starts_with(short)) {
            Some((short, long)) => {
                words.push_str(long);
                rest = &rest[short.len()..];
            }
            None => {
                words.push(next);
                rest = &rest[next.len_utf8()..];
            }
        }
    }
    format!("{words} {tag}")
}

/// `Level 11`, from the `Gained` row.
pub fn level_text(level: u8) -> String {
    format!("Level {level}")
}

/// The action and the uses of a power, as the meta line holds them: `Standard action`, then the
/// text of `Usage`. `Passive` and `Ability` show nothing.
pub fn meta_parts(power: &Power<'_>) -> Vec<String> {
    let action = power
        .action()
        .map(|effect| format!("{} action", effect.label));
    let uses = power
        .usage
        .map(|usage| plain_text(usage).trim().to_string())
        .filter(|text| !text.is_empty());
    action.into_iter().chain(uses).collect()
}

/// The facts line of a one-power page: the tag in words, `Level N`, the action and the uses. It
/// is empty for a power with none of them.
pub fn facts_parts(power: &Power<'_>) -> Vec<String> {
    let tag = split_ability_tag(&power.name).1.map(tag_in_words);
    let level = power.gained.map(level_text);
    tag.into_iter()
        .chain(level)
        .chain(meta_parts(power))
        .collect()
}

/// The facts line for an entry page: empty unless `body` is a one-power page of `title`.
pub fn entry_facts(title: &str, body: &[Block]) -> Vec<String> {
    one_power_page(title, body)
        .map(|power| facts_parts(&power))
        .unwrap_or_default()
}

/// The inlines of a row's content without the white space that separates them from the label.
pub fn trimmed(content: &[Inline]) -> Vec<Inline> {
    let mut out = content.to_vec();
    if let Some(Inline::Text { v }) = out.first_mut() {
        *v = v.trim_start().to_string();
        if v.is_empty() {
            out.remove(0);
        }
    }
    out
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

    fn name(value: &str) -> Block {
        para(vec![strong(value)])
    }

    fn row(label: &str, value: &str) -> Block {
        para(vec![
            strong(&format!("{label}:")),
            text(&format!(" {value}")),
        ])
    }

    fn lead(level: &str, value: &str) -> Block {
        para(vec![
            strong(&format!("At {level} level,")),
            text(&format!(" {value}")),
        ])
    }

    fn plain(value: &str) -> Block {
        para(vec![text(value)])
    }

    fn heading(level: u8, value: &str) -> Block {
        Block::Heading {
            level,
            anchor: None,
            content: vec![text(value)],
        }
    }

    /// Abjuration's Resistance (Ex): the fixture of the decision, in canonical order.
    fn resistance() -> Vec<Block> {
        vec![
            name("Resistance (Ex)"),
            row("Gained", "At 1st Level"),
            row("Passive", "You gain resistance 5."),
            lead("11th", "this resistance increases to 10."),
            lead("20th", "this resistance changes to immunity."),
        ]
    }

    /// A power that fails validation: two passive rows.
    fn failing(label: &str) -> Vec<Block> {
        vec![
            name(label),
            row("Gained", "At 1st Level"),
            row("Ability", "One."),
            row("Passive", "Two."),
        ]
    }

    #[test]
    fn a_valid_power_is_placed_with_its_length_and_heading_level() {
        let mut body = resistance();
        body.push(name("Protective Ward (Su)"));
        body.push(row("Standard", "You ward an ally."));
        body.push(row("Usage", "3 + Intelligence modifier times/day"));
        let plan = plan("Abjuration", &body);
        let first = plan.at(&body[0]).expect("the first power is placed");
        assert_eq!((first.len, first.heading, first.folded), (5, 2, false));
        let second = plan.at(&body[5]).expect("the second power is placed");
        assert_eq!((second.len, second.heading, second.folded), (3, 2, false));
        assert!(plan.at(&body[1]).is_none());
    }

    #[test]
    fn the_heading_level_follows_the_preceding_body_heading() {
        let mut body = resistance();
        body.push(heading(3, "Banishment School"));
        body.extend(resistance());
        let plan = plan("Abjuration", &body);
        assert_eq!(plan.at(&body[0]).unwrap().heading, 2);
        assert_eq!(plan.at(&body[6]).unwrap().heading, 4);
    }

    #[test]
    fn a_candidate_that_fails_validation_is_not_placed() {
        let mut body = failing("Counterspell Mastery");
        let valid = resistance();
        body.extend(valid);
        let plan = plan("Abjuration", &body);
        assert!(
            plan.at(&body[0]).is_none(),
            "the failing candidate renders as today"
        );
        assert!(plan.at(&body[4]).is_some(), "the valid one is laid out");
    }

    #[test]
    fn a_section_label_is_not_a_power() {
        let mut body = vec![
            name("Domain Spells"),
            row("Gained", "At 1st Level"),
            row("1st", "bless"),
        ];
        body.extend(resistance());
        let plan = plan("Abjuration", &body);
        assert!(plan.at(&body[0]).is_none());
        assert!(plan.at(&body[3]).is_some());
    }

    #[test]
    fn a_one_power_page_folds_the_head() {
        let body = resistance();
        let plan = plan("Resistance", &body);
        let placed = plan.at(&body[0]).expect("placed");
        assert!(placed.folded);
        assert_eq!(placed.len, 5);
    }

    #[test]
    fn a_page_whose_title_differs_keeps_the_head() {
        let body = resistance();
        let plan = plan("Energy Resistance", &body);
        assert!(!plan.at(&body[0]).expect("placed").folded);
        assert!(entry_facts("Energy Resistance", &body).is_empty());
    }

    #[test]
    fn a_failing_only_power_is_never_folded() {
        let body = failing("Resistance (Ex)");
        assert!(plan("Resistance", &body).at(&body[0]).is_none());
        assert!(entry_facts("Resistance", &body).is_empty());
    }

    #[test]
    fn the_facts_line_holds_the_tag_in_words_the_level_the_action_and_the_uses() {
        let body = vec![
            name("Evil Eye (Su)"),
            row("Gained", "At 3rd Level"),
            row("Swift", "You cast a malign gaze."),
            row("Usage", "Once per rage"),
        ];
        assert_eq!(
            entry_facts("Evil Eye", &body),
            [
                "Supernatural (Su)",
                "Level 3",
                "Swift action",
                "Once per rage"
            ]
        );
    }

    #[test]
    fn a_passive_power_without_a_level_has_only_its_tag() {
        let body = vec![
            name("Elixir of Life"),
            row("Ability", "You brew an elixir."),
        ];
        assert!(entry_facts("Elixir of Life", &body).is_empty());
        let body = vec![
            name("Elixir of Life (Ex)"),
            row("Passive", "You brew an elixir."),
        ];
        assert_eq!(entry_facts("Elixir of Life", &body), ["Extraordinary (Ex)"]);
    }

    #[test]
    fn a_combined_tag_names_each_ability() {
        assert_eq!(
            tag_in_words("(Ex or Su)"),
            "Extraordinary or Supernatural (Ex or Su)"
        );
        assert_eq!(
            tag_in_words("(Sp and Ex)"),
            "Spell-Like and Extraordinary (Sp and Ex)"
        );
        assert_eq!(tag_in_words("(Su)"), "Supernatural (Su)");
    }

    #[test]
    fn the_meta_line_shows_the_action_and_uses_and_a_passive_shows_nothing() {
        let passive = resistance();
        let power = validate_power(&passive).unwrap();
        assert!(meta_parts(&power).is_empty());
        let active = vec![
            name("Protective Ward (Su)"),
            row("Standard", "You ward an ally."),
            row("Usage", "3 + Intelligence modifier times/day"),
        ];
        let power = validate_power(&active).unwrap();
        assert_eq!(
            meta_parts(&power),
            ["Standard action", "3 + Intelligence modifier times/day"]
        );
    }

    #[test]
    fn a_passive_and_an_action_row_show_the_action_of_the_second_row() {
        let body = vec![
            name("Noble Visage (Su)"),
            row("Passive", "You gain a bonus."),
            row("Standard", "You touch an ally."),
            row("Usage", "3 + Wisdom modifier times/day"),
        ];
        let power = validate_power(&body).unwrap();
        assert_eq!(
            meta_parts(&power),
            ["Standard action", "3 + Wisdom modifier times/day"]
        );
    }

    #[test]
    fn a_row_loses_the_white_space_after_its_label() {
        assert_eq!(trimmed(&[text("  gain")]), [text("gain")]);
        assert_eq!(trimmed(&[text(" "), strong("x")]), [strong("x")]);
        assert_eq!(trimmed(&[strong("x")]), [strong("x")]);
        assert!(trimmed(&[]).is_empty());
    }

    #[test]
    fn a_power_after_other_text_keeps_its_head() {
        let body = vec![
            plain("Intro."),
            name("Resistance (Ex)"),
            row("Passive", "x"),
        ];
        let plan = plan("Resistance", &body);
        assert!(plan.at(&body[0]).is_none());
        assert!(
            !plan.at(&body[1]).expect("placed").folded,
            "the name is not the first block"
        );
    }
}
