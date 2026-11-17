//! The values of a class page's summary card, read from the entry and its body.
//!
//! The corpus types only three class facts: the class type, the hit die, and (for 44 of the
//! 49 non-prestige classes) the skill ranks. Everything else the card shows exists as printed
//! prose or as the progression table, so this module reads it from there, and only when the
//! value is unambiguous. A fact it cannot read with confidence stays out of the card and stays
//! in the page body, so nothing is lost and nothing is shown twice.
//!
//! | Card row | Source |
//! |---|---|
//! | Hit Die | typed `hit_die`; the printed `Hit Die:` paragraph when the field is missing |
//! | Base attack, Good saves | the progression table: every row must follow one known formula |
//! | Skill ranks | typed `skill_ranks`; the printed `Skill Points …:` paragraph otherwise |
//! | Class skills | the printed `Class Skills (Ability):` paragraphs |
//! | Alignment, Starting wealth | the printed paragraph, when it is a short value |
//! | Spellcasting | the printed `Spell list:`, `Key ability:` and `Casting:` paragraphs |
//!
//! A printed paragraph that the card replaces is reported in [`Read::consumed`], and the page
//! drops it from the body. A paragraph the card cannot use (a prestige class's paragraph on
//! alignment, say) is left where it is.

use crate::views::progression;
use cast_iron_schema::{Block, Inline, Table, TypedFields, headings::plain_text};
use std::collections::BTreeSet;

/// How fast a class gains base attack bonus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rate {
    Good,
    Medium,
    Poor,
}

impl Rate {
    pub fn label(self) -> &'static str {
        match self {
            Rate::Good => "Good",
            Rate::Medium => "Medium",
            Rate::Poor => "Poor",
        }
    }

    /// The gain per level that the rate stands for, as the Core Rulebook prints it.
    pub fn per_level(self) -> &'static str {
        match self {
            Rate::Good => "+1 per level",
            Rate::Medium => "+3/4 per level",
            Rate::Poor => "+1/2 per level",
        }
    }

    fn at(self, level: u32) -> u32 {
        match self {
            Rate::Good => level,
            Rate::Medium => 3 * level / 4,
            Rate::Poor => level / 2,
        }
    }
}

/// The class skills of one key ability. `ability` is empty for an unsplit printed line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillGroup {
    pub ability: String,
    pub skills: Vec<String>,
}

/// What the printed spellcasting paragraphs say. The link to the class spell list comes from the
/// spell-list index, so it is not here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Casting {
    /// `Arcane`, `Divine` or `Psychic`.
    pub tradition: Option<&'static str>,
    /// `prepared` or `spontaneous`.
    pub style: Option<&'static str>,
    /// The key ability's name, such as `Intelligence`.
    pub ability: Option<&'static str>,
}

impl Casting {
    pub fn is_empty(&self) -> bool {
        self.tradition.is_none() && self.style.is_none() && self.ability.is_none()
    }

    /// `Arcane, prepared · Intelligence`, or the parts that exist.
    pub fn line(&self) -> String {
        let kind = match (self.tradition, self.style) {
            (Some(tradition), Some(style)) => Some(format!("{tradition}, {style}")),
            (Some(tradition), None) => Some(tradition.to_string()),
            (None, Some(style)) => {
                let mut chars = style.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect())
            }
            (None, None) => None,
        };
        [kind, self.ability.map(str::to_string)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassSummary {
    pub hit_die: Option<String>,
    pub attack: Option<Rate>,
    /// The saves a class is good at, in Fortitude, Reflex, Will order. `None` when the table
    /// does not follow a known formula; an empty list means no save is good.
    pub good_saves: Option<Vec<&'static str>>,
    pub skill_ranks: Option<String>,
    pub skills: Vec<SkillGroup>,
    pub alignment: Option<String>,
    pub wealth: Option<String>,
    pub casting: Casting,
}

impl ClassSummary {
    /// Whether the card has a row to show. Spellcasting counts only with its printed text; the
    /// spell-list link is added by the view.
    pub fn is_empty(&self) -> bool {
        self.hit_die.is_none()
            && self.attack.is_none()
            && self.good_saves.is_none()
            && self.skill_ranks.is_none()
            && self.skills.is_empty()
            && self.alignment.is_none()
            && self.wealth.is_none()
            && self.casting.is_empty()
    }

    /// The count of class skills the card lists.
    pub fn skill_count(&self) -> usize {
        self.skills.iter().map(|group| group.skills.len()).sum()
    }
}

/// The card's values and the body blocks that it replaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Read {
    pub summary: ClassSummary,
    /// Indexes into the body of the paragraphs the card shows. Every fact in them is in the card.
    pub consumed: BTreeSet<usize>,
}

/// The body without the blocks the card replaces.
pub fn without(body: &[Block], consumed: &BTreeSet<usize>) -> Vec<Block> {
    body.iter()
        .enumerate()
        .filter(|(i, _)| !consumed.contains(i))
        .map(|(_, block)| block.clone())
        .collect()
}

/// One `**Label:** value` run of a paragraph. A prestige class prints two of them in one
/// paragraph, split by a line break.
struct Segment {
    label: String,
    value: Vec<Inline>,
}

fn segments(content: &[Inline]) -> Option<Vec<Segment>> {
    let mut out = Vec::new();
    let mut current: Option<Segment> = None;
    for item in content {
        match item {
            Inline::Break => out.extend(current.take()),
            Inline::Strong { c } if current.is_none() => {
                let label = plain_text(c);
                current = Some(Segment {
                    label: label.trim().trim_end_matches(':').trim().to_string(),
                    value: Vec::new(),
                });
            }
            other => match current.as_mut() {
                Some(segment) => segment.value.push(other.clone()),
                None if matches!(other, Inline::Text { v } if v.trim().is_empty()) => {}
                None => return None,
            },
        }
    }
    out.extend(current);
    (!out.is_empty()).then_some(out)
}

/// The text of a value, without its closing full stop. `None` when the value holds a link, a
/// bold run or a break, because the card shows plain text and would drop them.
fn plain_value(value: &[Inline]) -> Option<String> {
    fn plain(items: &[Inline]) -> bool {
        items.iter().all(|item| match item {
            Inline::Text { .. } => true,
            Inline::Em { c } | Inline::Sup { c } | Inline::Sub { c } => plain(c),
            Inline::Strong { .. } | Inline::Link { .. } | Inline::Break => false,
        })
    }
    if !plain(value) {
        return None;
    }
    let text = plain_text(value);
    let text = text.trim().trim_end_matches('.').trim();
    (!text.is_empty()).then(|| text.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// `d6`, `d10`; anything else (a die that is not one, a sentence) is not a hit die.
fn hit_die(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('.').trim();
    let digits = value.strip_prefix('d')?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| value.to_string())
}

fn skill_ranks(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('.').trim();
    (value.len() <= 60 && value.starts_with(|c: char| c.is_ascii_digit()))
        .then(|| value.to_string())
}

const SHORT: usize = 40;

/// A short value such as `Any`, `Lawful good` or `2d6 × 10 gp (average 70 gp)`. A prestige
/// class's alignment is a paragraph of prose; that stays in the body.
fn short(value: &str, limit: usize) -> Option<String> {
    (value.len() <= limit && !value.contains(". ")).then(|| value.to_string())
}

enum Field {
    HitDie(String),
    Alignment(String),
    Wealth(String),
    Ranks(String),
    Skills(SkillGroup),
}

fn skill_list(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (i, c) in value.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(value[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(value[start..].trim().to_string());
    out.retain(|skill| !skill.is_empty());
    out
}

fn classify(segment: &Segment) -> Option<Field> {
    let value = plain_value(&segment.value)?;
    let label = segment.label.to_lowercase();
    match label.as_str() {
        "hit die" => hit_die(&value).map(Field::HitDie),
        "alignment" => short(&value, SHORT).map(Field::Alignment),
        "starting wealth" => short(&value, 60).map(Field::Wealth),
        "skill points per level" | "skill points at each level" => {
            skill_ranks(&value).map(Field::Ranks)
        }
        _ => {
            let rest = label.strip_prefix("class skills")?.trim();
            let ability = match rest {
                "" => String::new(),
                _ => {
                    let inner = rest.strip_prefix('(')?.strip_suffix(')')?;
                    // Keep the printed capital: `Dex`, `Int`.
                    let start = segment.label.find('(')? + 1;
                    segment.label[start..start + inner.len()].to_string()
                }
            };
            let skills = skill_list(&value);
            (!skills.is_empty()).then_some(Field::Skills(SkillGroup { ability, skills }))
        }
    }
}

const ABILITIES: [&str; 6] = [
    "Strength",
    "Dexterity",
    "Constitution",
    "Intelligence",
    "Wisdom",
    "Charisma",
];

/// The blocks up to the next heading: a section's own text. A class page's body also holds every
/// archetype, which prints its own `Key ability:` and `Casting:` lines after its own heading.
fn until_heading(blocks: &[Block]) -> &[Block] {
    let end = blocks
        .iter()
        .position(|block| matches!(block, Block::Heading { .. }))
        .unwrap_or(blocks.len());
    &blocks[..end]
}

/// `(label, value text)` of a paragraph that holds exactly one `**Label:**` run.
fn single_label(block: &Block) -> Option<(String, String)> {
    let Block::Paragraph { content, .. } = block else {
        return None;
    };
    let mut found = segments(content)?;
    (found.len() == 1).then(|| {
        let segment = found.remove(0);
        (segment.label.to_lowercase(), plain_text(&segment.value))
    })
}

/// The printed `Spell list:`, `Key ability:` and `Casting:` lines of the Spells section.
fn casting_block(body: &[Block]) -> Option<Casting> {
    let start = body
        .iter()
        .position(|block| single_label(block).is_some_and(|(label, _)| label == "spell list"))?;
    let mut out = Casting::default();
    for (i, block) in until_heading(&body[start..]).iter().take(5).enumerate() {
        let Some((label, value)) = single_label(block) else {
            continue;
        };
        let value = value.trim();
        match (i, label.as_str()) {
            (0, _) => {
                out.tradition = TRADITIONS.into_iter().find(|tradition| {
                    value
                        .strip_prefix(tradition)
                        .is_some_and(|rest| rest.starts_with(" spells from"))
                });
            }
            (_, "key ability") => {
                let first = value.split('.').next().unwrap_or("").trim();
                out.ability = ABILITIES.into_iter().find(|ability| *ability == first);
            }
            (_, "casting") => {
                let first = value.split(':').next().unwrap_or("").trim();
                out.style = match first {
                    "Prepared" => Some("prepared"),
                    "Spontaneous" => Some("spontaneous"),
                    _ => None,
                };
            }
            _ => {}
        }
    }
    Some(out)
}

const TRADITIONS: [&str; 3] = ["Arcane", "Divine", "Psychic"];

/// The text of a class's own spellcasting feature: the blocks under a `Spells` or `Extracts`
/// heading, or an NPC class's `**Spells:**` paragraph and the plain paragraphs after it.
fn feature_text(body: &[Block]) -> Option<String> {
    let heading = body.iter().position(|block| match block {
        Block::Heading { content, .. } => {
            matches!(
                plain_text(content).trim().to_lowercase().as_str(),
                "spells" | "extracts"
            )
        }
        _ => false,
    });
    let scope: &[Block] = match heading {
        Some(i) => until_heading(&body[i + 1..]),
        None => {
            let i = body.iter().position(|block| {
                single_label(block).is_some_and(|(label, _)| label == "spells")
            })?;
            let rest = &body[i + 1..];
            let end = rest
                .iter()
                .position(|block| {
                    matches!(block, Block::Heading { .. }) || single_label(block).is_some()
                })
                .unwrap_or(rest.len());
            &body[i..i + 1 + end]
        }
    };
    let text: Vec<String> = scope
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph { content, .. } => Some(plain_text(content)),
            _ => None,
        })
        .collect();
    (!text.is_empty()).then(|| text.join(" ").to_lowercase())
}

/// Spellcasting read from the feature's own sentences, for a class without the printed block.
///
/// Each part must be stated plainly: one tradition (`divine spells drawn from the … list`); the preparing style
/// (`prepare` or `do not need to prepare`); and a key ability that the text names twice, as
/// `{Ability} score` and `{Ability} modifier`, with no other ability named that way. One
/// mention is not enough: the Bloodrager's section names `Constitution modifier` once, and the
/// corpus's own feat text gives its casting ability as Charisma, so it shows no ability.
fn casting_prose(body: &[Block]) -> Casting {
    let Some(text) = feature_text(body) else {
        return Casting::default();
    };
    // A tradition counts where the text says whose spells the class casts: `divine spells drawn
    // from the paladin spell list`. The word alone is not enough; the Alchemist's section names
    // `divine spells` while it compares extracts to a cleric's.
    let traditions: Vec<&'static str> = TRADITIONS
        .into_iter()
        .filter(|tradition| {
            let phrase = format!("{} spells", tradition.to_lowercase());
            text.match_indices(&phrase).any(|(at, _)| {
                let after = &text[at + phrase.len()..];
                after
                    .chars()
                    .take(30)
                    .collect::<String>()
                    .contains("drawn from")
            })
        })
        .collect();
    let style = if text.contains("do not need to prepare") || text.contains("don't need to prepare")
    {
        Some("spontaneous")
    } else if text.contains("prepare") {
        Some("prepared")
    } else {
        None
    };
    let named: Vec<&'static str> = ABILITIES
        .into_iter()
        .filter(|ability| {
            let name = ability.to_lowercase();
            text.contains(&format!("{name} score")) || text.contains(&format!("{name} modifier"))
        })
        .collect();
    let ability = match named.as_slice() {
        [only] => {
            let name = only.to_lowercase();
            (text.contains(&format!("{name} score")) && text.contains(&format!("{name} modifier")))
                .then_some(*only)
        }
        _ => None,
    };
    Casting {
        tradition: match traditions.as_slice() {
            [only] => Some(only),
            _ => None,
        },
        style,
        ability,
    }
}

fn casting(body: &[Block], prestige: bool) -> Casting {
    if let Some(printed) = casting_block(body) {
        return printed;
    }
    // A prestige class's `Spells:` line is a requirement, not a feature.
    if prestige {
        return Casting::default();
    }
    casting_prose(body)
}

/// The card's values and the blocks it replaces.
pub fn read(fields: &TypedFields, body: &[Block]) -> Read {
    let mut summary = ClassSummary::default();
    let (typed_die, typed_ranks) = match fields {
        TypedFields::Class(class) => (
            class.hit_die.as_deref().and_then(hit_die),
            class.skill_ranks.as_deref().and_then(skill_ranks),
        ),
        _ => (None, None),
    };
    summary.hit_die = typed_die.clone();
    summary.skill_ranks = typed_ranks.clone();

    // The printed basics sit before the progression table. Past it the body holds feature text
    // and, for a base class, every archetype's text, which prints the same labels.
    let end = body
        .iter()
        .position(|block| match block {
            Block::Table(_) => true,
            Block::Heading { content, .. } => plain_text(content).starts_with("Class Features"),
            _ => false,
        })
        .unwrap_or(body.len());

    let mut consumed = BTreeSet::new();
    for (index, block) in body[..end].iter().enumerate() {
        let Block::Paragraph { content, .. } = block else {
            continue;
        };
        let Some(found) = segments(content) else {
            continue;
        };
        let fields: Option<Vec<Field>> = found.iter().map(classify).collect();
        let Some(fields) = fields else {
            continue;
        };
        // A printed value that disagrees with the typed one is a corpus problem the card must
        // not hide: the typed value shows and the printed paragraph stays.
        let agrees = fields.iter().all(|field| match field {
            // The summary already holds the typed value, or the first printed one, so a second
            // printed paragraph that differs from either stays in the body.
            Field::HitDie(die) => summary.hit_die.as_ref().is_none_or(|have| have == die),
            Field::Ranks(ranks) => summary
                .skill_ranks
                .as_ref()
                .is_none_or(|have| have == ranks),
            Field::Alignment(_) => summary.alignment.is_none(),
            Field::Wealth(_) => summary.wealth.is_none(),
            Field::Skills(_) => true,
        });
        if !agrees {
            continue;
        }
        for field in fields {
            match field {
                Field::HitDie(die) => summary.hit_die.get_or_insert(die),
                Field::Ranks(ranks) => summary.skill_ranks.get_or_insert(ranks),
                Field::Alignment(value) => summary.alignment.insert(value),
                Field::Wealth(value) => summary.wealth.insert(value),
                Field::Skills(group) => {
                    summary.skills.push(group);
                    continue;
                }
            };
        }
        consumed.insert(index);
    }

    if let Some(split) = body.iter().find_map(|block| match block {
        Block::Table(table) => progression::split(table),
        _ => None,
    }) {
        let (attack, saves) = rates(&split.core);
        summary.attack = attack;
        summary.good_saves = saves;
    }
    let prestige = matches!(fields, TypedFields::Class(class) if class.class_type.as_deref() == Some("prestige"));
    summary.casting = casting(body, prestige);
    Read { summary, consumed }
}

const SAVES: [(&str, &str); 3] = [("fort", "Fortitude"), ("ref", "Reflex"), ("will", "Will")];

/// A level cell such as `1st` or `20th`.
fn level_of(text: &str) -> Option<u32> {
    let digits: String = text
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// The first number of a cell such as `+2` or `+11/+6/+1`.
fn bonus_of(text: &str) -> Option<u32> {
    let first = text.split('/').next()?.trim();
    first.strip_prefix('+').unwrap_or(first).parse().ok()
}

/// The good and poor save at a level, for a base class and for a prestige class, which starts
/// a step lower. A table must follow one formula in every row to read as good or poor.
const GOOD_SAVE: [fn(u32) -> u32; 2] = [|l| 2 + l / 2, |l| l.div_ceil(2)];
const POOR_SAVE: [fn(u32) -> u32; 2] = [|l| l / 3, |l| (l + 1) / 3];

fn rates(core: &Table) -> (Option<Rate>, Option<Vec<&'static str>>) {
    let Some(head) = core.head.first() else {
        return (None, None);
    };
    let names: Vec<String> = head
        .cells
        .iter()
        .map(|cell| plain_text(&cell.content).trim().to_lowercase())
        .collect();
    let column = |wanted: &dyn Fn(&str) -> bool| names.iter().position(|name| wanted(name));
    let cells = |col: usize| -> Option<Vec<(u32, u32)>> {
        let mut out = Vec::new();
        for (i, row) in core.body.iter().enumerate() {
            let level = level_of(&plain_text(&row.cells.first()?.content))?;
            if level as usize != i + 1 {
                return None;
            }
            out.push((level, bonus_of(&plain_text(&row.cells.get(col)?.content))?));
        }
        (!out.is_empty()).then_some(out)
    };

    let attack = column(&|name| name == "base attack bonus")
        .and_then(cells)
        .and_then(|rows| {
            let fits: Vec<Rate> = [Rate::Good, Rate::Medium, Rate::Poor]
                .into_iter()
                .filter(|rate| rows.iter().all(|(level, bonus)| *bonus == rate.at(*level)))
                .collect();
            match fits.as_slice() {
                [only] => Some(*only),
                _ => None,
            }
        });

    let mut good = Vec::new();
    let mut complete = true;
    for (prefix, name) in SAVES {
        let rows = column(&|header| header.starts_with(prefix)).and_then(cells);
        let matches = |formulas: &[fn(u32) -> u32; 2]| {
            rows.as_ref().is_some_and(|rows| {
                formulas
                    .iter()
                    .any(|f| rows.iter().all(|(level, bonus)| *bonus == f(*level)))
            })
        };
        if matches(&GOOD_SAVE) {
            good.push(name);
        } else if !matches(&POOR_SAVE) {
            complete = false;
        }
    }
    (attack, complete.then_some(good))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{ClassFields, TableCell, TableRow};

    fn text(value: &str) -> Inline {
        Inline::text(value)
    }

    fn labelled(label: &str, value: &str) -> Block {
        Block::Paragraph {
            style: Default::default(),
            content: vec![
                Inline::Strong {
                    c: vec![text(&format!("{label}:"))],
                },
                text(&format!(" {value}")),
            ],
        }
    }

    fn cell(value: &str) -> TableCell {
        TableCell {
            header: false,
            colspan: 1,
            rowspan: 1,
            content: vec![text(value)],
        }
    }

    fn table(bab: impl Fn(u32) -> u32, saves: [fn(u32) -> u32; 3], levels: u32) -> Block {
        let row = |cells: Vec<String>| TableRow {
            cells: cells.iter().map(|c| cell(c)).collect(),
        };
        let head = row([
            "Level",
            "Base Attack Bonus",
            "Fort Save",
            "Ref Save",
            "Will Save",
            "Special",
        ]
        .map(String::from)
        .to_vec());
        let body = (1..=levels)
            .map(|l| {
                row(vec![
                    format!("{l}th"),
                    format!("+{}", bab(l)),
                    format!("+{}", saves[0](l)),
                    format!("+{}", saves[1](l)),
                    format!("+{}", saves[2](l)),
                    "-".to_string(),
                ])
            })
            .collect();
        Block::Table(Table {
            caption: vec![],
            head: vec![head],
            body,
            foot: vec![],
        })
    }

    fn class(die: Option<&str>, ranks: Option<&str>) -> TypedFields {
        TypedFields::Class(ClassFields {
            class_type: Some("core".into()),
            hit_die: die.map(String::from),
            skill_ranks: ranks.map(String::from),
            class_skills: None,
        })
    }

    #[test]
    fn the_typed_values_win_and_the_printed_paragraph_goes() {
        let body = vec![
            labelled("Hit Die", "d6"),
            labelled("Skill Points per Level", "2 + Int modifier"),
        ];
        let read = read(&class(Some("d6"), Some("2 + Int modifier")), &body);
        assert_eq!(read.summary.hit_die.as_deref(), Some("d6"));
        assert_eq!(
            read.summary.skill_ranks.as_deref(),
            Some("2 + Int modifier")
        );
        assert_eq!(read.consumed, BTreeSet::from([0, 1]));
    }

    #[test]
    fn a_printed_value_that_disagrees_with_the_typed_one_stays_in_the_body() {
        let body = vec![labelled("Hit Die", "d8")];
        let read = read(&class(Some("d6"), None), &body);
        assert_eq!(read.summary.hit_die.as_deref(), Some("d6"));
        assert!(read.consumed.is_empty());
    }

    #[test]
    fn a_printed_value_loses_its_full_stop_and_so_does_a_typed_die() {
        let body = vec![labelled("Skill Points at each Level", "4 + Int modifier.")];
        let read = read(&class(Some("d10."), None), &body);
        assert_eq!(
            read.summary.skill_ranks.as_deref(),
            Some("4 + Int modifier")
        );
        // The corpus types a prestige class's die with the printed full stop (`d10.`).
        assert_eq!(read.summary.hit_die.as_deref(), Some("d10"));
    }

    #[test]
    fn two_labels_in_one_paragraph_are_both_read() {
        let body = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![
                Inline::Strong {
                    c: vec![text("Skill Points at each Level:")],
                },
                text(" 2 + Int modifier."),
                Inline::Break,
                Inline::Strong {
                    c: vec![text("Hit Die:")],
                },
                text(" d10."),
            ],
        }];
        let read = read(&class(None, None), &body);
        assert_eq!(read.summary.hit_die.as_deref(), Some("d10"));
        assert_eq!(
            read.summary.skill_ranks.as_deref(),
            Some("2 + Int modifier")
        );
        assert_eq!(read.consumed, BTreeSet::from([0]));
    }

    #[test]
    fn a_paragraph_with_one_unusable_label_stays_whole() {
        let body = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![
                Inline::Strong {
                    c: vec![text("Hit Die:")],
                },
                text(" d10."),
                Inline::Break,
                Inline::Strong {
                    c: vec![text("Alignment:")],
                },
                text(" The road to becoming one is varied. Many favor law."),
            ],
        }];
        let read = read(&class(Some("d10."), None), &body);
        assert!(read.consumed.is_empty());
        assert_eq!(read.summary.alignment, None);
    }

    #[test]
    fn a_long_alignment_paragraph_is_not_a_card_row() {
        let long = "The road to becoming an eldritch knight is as varied as the paths leading to apprenticeship.";
        let body = vec![labelled("Alignment", long), labelled("Alignment", "Any")];
        let read = read(&class(None, None), &body);
        // The first is prose and stays; the card takes the short one.
        assert_eq!(read.summary.alignment.as_deref(), Some("Any"));
        assert_eq!(read.consumed, BTreeSet::from([1]));
    }

    #[test]
    fn class_skills_group_by_ability_and_split_outside_parentheses() {
        let body = vec![
            labelled("Class Skills (Dex)", "Fly"),
            labelled(
                "Class Skills (Int)",
                "Appraise, Craft (any), Knowledge (arcana), Knowledge (history)",
            ),
        ];
        let read = read(&class(None, None), &body);
        assert_eq!(read.summary.skills.len(), 2);
        assert_eq!(read.summary.skills[0].ability, "Dex");
        assert_eq!(
            read.summary.skills[1].skills,
            [
                "Appraise",
                "Craft (any)",
                "Knowledge (arcana)",
                "Knowledge (history)"
            ]
        );
        assert_eq!(read.summary.skill_count(), 5);
        assert_eq!(read.consumed, BTreeSet::from([0, 1]));
    }

    #[test]
    fn paragraphs_after_the_progression_table_are_left_alone() {
        let body = vec![
            table(|l| l * 3 / 4, [|l| 2 + l / 2, |l| l / 3, |l| l / 3], 20),
            labelled("Alignment", "Lawful good"),
        ];
        let read = read(&class(None, None), &body);
        assert_eq!(read.summary.alignment, None);
        assert!(read.consumed.is_empty());
    }

    #[test]
    fn base_attack_and_saves_read_from_the_whole_table() {
        for (rate, bab) in [
            (Rate::Good, (|l| l) as fn(u32) -> u32),
            (Rate::Medium, |l| 3 * l / 4),
            (Rate::Poor, |l| l / 2),
        ] {
            let body = vec![table(bab, [|l| 2 + l / 2, |l| l / 3, |l| 2 + l / 2], 20)];
            let read = read(&class(None, None), &body);
            assert_eq!(read.summary.attack, Some(rate));
            assert_eq!(read.summary.good_saves, Some(vec!["Fortitude", "Will"]));
        }
    }

    #[test]
    fn a_prestige_class_starts_its_saves_a_step_lower() {
        let body = vec![table(
            |l| l,
            [|l| l.div_ceil(2), |l| (l + 1) / 3, |l| (l + 1) / 3],
            10,
        )];
        let read = read(&class(None, None), &body);
        assert_eq!(read.summary.attack, Some(Rate::Good));
        assert_eq!(read.summary.good_saves, Some(vec!["Fortitude"]));
    }

    #[test]
    fn a_table_that_follows_no_formula_gives_no_row() {
        // One row off the base attack formula, one save that is neither good nor poor.
        let body = vec![table(
            |l| if l == 7 { 9 } else { l },
            [|l| 2 + l / 2, |l| l / 3 + 1, |l| l / 3],
            10,
        )];
        let read = read(&class(None, None), &body);
        assert_eq!(read.summary.attack, None);
        assert_eq!(read.summary.good_saves, None);
    }

    #[test]
    fn a_short_table_that_fits_two_rates_gives_no_attack_row() {
        // At level 1 medium and poor are both +0.
        let body = vec![table(|_| 0, [|l| 2 + l / 2, |l| l / 3, |l| l / 3], 1)];
        let read = read(&class(None, None), &body);
        assert_eq!(read.summary.attack, None);
        assert_eq!(read.summary.good_saves, Some(vec!["Fortitude"]));
    }

    #[test]
    fn spellcasting_reads_the_printed_block() {
        let body = vec![
            labelled(
                "Spell list",
                "Arcane spells from the wizard list, from 1st level",
            ),
            labelled(
                "Key ability",
                "Intelligence. Minimum score 10 + spell level; save DC 10 + spell level.",
            ),
            labelled("Casting", "Prepared: choose and prepare spells in advance."),
        ];
        let casting = read(&class(None, None), &body).summary.casting;
        assert_eq!(casting.line(), "Arcane, prepared · Intelligence");
    }

    #[test]
    fn a_caster_without_the_printed_block_has_no_casting_text() {
        let body = vec![labelled(
            "Ability",
            "You cast arcane spells drawn from the list.",
        )];
        let read = read(&class(None, None), &body);
        assert!(read.summary.casting.is_empty());
        assert!(read.summary.is_empty());
    }

    #[test]
    fn the_spells_block_stays_in_the_body() {
        let body = vec![labelled(
            "Spell list",
            "Divine spells from the cleric list, from 1st level",
        )];
        assert!(read(&class(None, None), &body).consumed.is_empty());
    }

    #[test]
    fn without_drops_only_the_consumed_blocks() {
        let body = vec![labelled("Hit Die", "d6"), labelled("Role", "Support")];
        let kept = without(&body, &BTreeSet::from([0]));
        assert_eq!(kept, vec![labelled("Role", "Support")]);
    }

    fn heading(title: &str) -> Block {
        Block::Heading {
            level: 4,
            anchor: None,
            content: vec![text(title)],
        }
    }

    fn para(value: &str) -> Block {
        Block::Paragraph {
            style: Default::default(),
            content: vec![text(value)],
        }
    }

    #[test]
    fn two_differing_printed_hit_dice_without_a_typed_one_keep_the_second() {
        let body = vec![labelled("Hit Die", "d6"), labelled("Hit Die", "d8")];
        let read = read(&class(None, None), &body);
        assert_eq!(read.summary.hit_die.as_deref(), Some("d6"));
        assert_eq!(read.consumed, BTreeSet::from([0]));
        let body = vec![
            labelled("Skill Points per Level", "2 + Int modifier"),
            labelled("Skill Points per Level", "4 + Int modifier"),
        ];
        let read = read_without_typed(&body);
        assert_eq!(
            read.summary.skill_ranks.as_deref(),
            Some("2 + Int modifier")
        );
        assert_eq!(read.consumed, BTreeSet::from([0]));
    }

    fn read_without_typed(body: &[Block]) -> Read {
        read(&class(None, None), body)
    }

    #[test]
    fn the_printed_block_stops_at_the_next_heading() {
        let body = vec![
            labelled(
                "Spell list",
                "Arcane spells from the wizard list, from 1st level",
            ),
            labelled("Key ability", "Intelligence. Minimum score 10."),
            heading("Archetype"),
            labelled("Casting", "Spontaneous: cast any spell known."),
        ];
        let casting = read_without_typed(&body).summary.casting;
        assert_eq!(casting.line(), "Arcane · Intelligence");
    }

    #[test]
    fn a_spells_feature_names_tradition_style_and_a_twice_stated_ability() {
        let body = vec![
            heading("Spells"),
            labelled(
                "Ability",
                "You gain the ability to cast a small number of divine spells drawn from the \
                 paladin spell list. The DC is 10 + the spell level + your Charisma modifier. \
                 You must choose and prepare your spells in advance. You receive bonus spells \
                 per day if you have a high Charisma score.",
            ),
        ];
        let casting = read_without_typed(&body).summary.casting;
        assert_eq!(casting.line(), "Divine, prepared · Charisma");
    }

    #[test]
    fn an_ability_named_once_is_not_shown() {
        let body = vec![
            heading("Spells"),
            labelled(
                "Ability",
                "You cast arcane spells drawn from its list. You do not need to prepare these spells in advance. \
                 The DC is 10 + the spell level + your Constitution modifier.",
            ),
        ];
        let casting = read_without_typed(&body).summary.casting;
        assert_eq!(casting.line(), "Arcane, spontaneous");
    }

    #[test]
    fn an_npc_spells_paragraph_and_its_plain_followers_are_read() {
        let body = vec![
            labelled(
                "Spells",
                "An adept casts divine spells, which are drawn from the adept list. An adept must choose and prepare her spells.",
            ),
            para("An adept must have a Wisdom score of at least 10 + the spell level."),
            para("The DC is 10 + the spell level + the adept's Wisdom modifier."),
            labelled("Weapon and Armor Proficiency", "Arcane spells and armor."),
        ];
        let casting = read_without_typed(&body).summary.casting;
        assert_eq!(casting.line(), "Divine, prepared · Wisdom");
    }

    #[test]
    fn extracts_give_a_style_and_an_ability_but_no_tradition() {
        let body = vec![
            heading("Extracts"),
            labelled(
                "Ability",
                "You can prepare an extract of any formula you know. You must have an \
                 Intelligence score of 10 + the level. The DC is 10 + the level + your \
                 Intelligence modifier.",
            ),
        ];
        let casting = read_without_typed(&body).summary.casting;
        assert_eq!(casting.line(), "Prepared · Intelligence");
    }

    #[test]
    fn a_prestige_class_requirement_is_not_spellcasting() {
        let body = vec![labelled(
            "Spells",
            "Able to cast 3rd-level arcane spells. You prepare them.",
        )];
        let fields = TypedFields::Class(ClassFields {
            class_type: Some("prestige".into()),
            ..Default::default()
        });
        assert!(read(&fields, &body).summary.casting.is_empty());
    }

    #[test]
    fn a_spells_feature_stops_at_the_next_heading() {
        let body = vec![
            heading("Spells"),
            labelled(
                "Ability",
                "You cast arcane spells drawn from its list. You must prepare them.",
            ),
            heading("Familiar"),
            labelled(
                "Ability",
                "Your divine spells use your Wisdom modifier and Wisdom score.",
            ),
        ];
        let casting = read_without_typed(&body).summary.casting;
        assert_eq!(casting.line(), "Arcane, prepared");
    }

    #[test]
    fn a_tradition_named_in_passing_is_not_the_class_tradition() {
        let body = vec![
            heading("Extracts"),
            labelled(
                "Ability",
                "You can prepare an extract. A cleric casts divine spells, and so on. You must \
                 have an Intelligence score of 10 and add your Intelligence modifier.",
            ),
        ];
        let casting = read_without_typed(&body).summary.casting;
        assert_eq!(casting.line(), "Prepared · Intelligence");
    }
}
