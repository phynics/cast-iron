//! Class-option powers: the shared classifier, candidate test and canonical validation.
//!
//! A class-option power (school power, revelation, hex, spirit ability, domain power) is a bold
//! name paragraph followed by bold-label rows. Decision `2026-11-20-class-option-power-layout`
//! fixes one canonical row order for it. This module is the one implementation of that decision's
//! section 2 over typed [`Block`]s. The corpus check reports the candidates that fail validation,
//! the corpus rewrite mirrors the classifier and the validator, and the renderer lays out a
//! candidate only when [`validate_power`] accepts it.
//!
//! Decision `2026-11-21-class-option-power-owner-cases` refines that order with the owner's answers
//! on the open cases: a passive row and an action row in either order, continuation paragraphs
//! after `Special`, `Choice` and `Info` before the effect, and the section labels that are never
//! powers.
//!
//! Everything here is a pure function over borrowed content: no I/O, no clock, and no hash-map
//! iteration, so the output is deterministic.

use crate::headings::plain_text;
use crate::{Block, Inline, ParagraphStyle};
use std::fmt;
use std::ops::Range;

/// The row labels of a base effect, in the canonical spelling.
pub const EFFECT_LABELS: [&str; 9] = [
    "Passive",
    "Ability",
    "Standard",
    "Move",
    "Swift",
    "Immediate",
    "Free",
    "Full-round",
    "Benefit",
];

/// Row labels that the import wrote for an effect row and that the canonical form replaces.
pub const ALIAS_LABELS: [&str; 3] = ["Fullround", "Passive Ability", "Standard Action"];

/// The legacy row `**Action:** <value>`, whose value names the action.
pub const ACTION_LABEL: &str = "Action";

/// The effect labels that name an action. A power with two effect rows has one row with one of
/// these labels, so the renderer tells the action paragraph from the passive text by its label.
pub const ACTION_EFFECT_LABELS: [&str; 6] = [
    "Standard",
    "Move",
    "Swift",
    "Immediate",
    "Free",
    "Full-round",
];

/// The effect labels of the passive text that pairs with an action row, in either order:
/// `Passive`, and `Ability`, which print does not mark as an action.
pub const PASSIVE_EFFECT_LABELS: [&str; 2] = ["Passive", "Ability"];

/// Block names that are section labels, never powers. [`candidates`] skips them, matched on the
/// plain text of the name block: trimmed, without a trailing colon, with runs of white space
/// folded, ignoring ASCII case. A block under one of them renders as it does today; the real
/// powers after it are candidates of their own.
pub const NON_POWER_LABELS: [&str; 6] = [
    "Base Evolutions",
    "Bonus Spells",
    "Discipline Powers",
    "Domain Spells",
    "Patron Spells",
    "Replacement Domain Spells",
];

const GAINED: &str = "Gained";
const FLAVOR: &str = "Flavor";
const CHOICE: &str = "Choice";
const INFO: &str = "Info";
const USAGE: &str = "Usage";
const PREREQ: &str = "Prereq";
const REPLACES: &str = "Replaces";
const SPECIAL: &str = "Special";

/// The highest level a class feature is gained at.
const MAX_LEVEL: u8 = 20;

// ---------------------------------------------------------------------------------------------
// Level grammar
// ---------------------------------------------------------------------------------------------

/// The English suffix of an ordinal from 1 to 20.
fn ordinal_suffix(level: u8) -> &'static str {
    match level {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

/// A well-formed ordinal is a number from 1 to 20 with no leading zero, followed by its English
/// suffix: `st` for 1, `nd` for 2, `rd` for 3 and `th` for 4 to 20.
pub fn well_formed_ordinal(digits: &str, suffix: &str) -> Option<u8> {
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let level: u8 = digits.parse().ok()?;
    ((1..=MAX_LEVEL).contains(&level) && suffix == ordinal_suffix(level)).then_some(level)
}

/// What a broad level mention `At <digits><suffix> <L|l>evel` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LevelHead<'a> {
    /// The level, when the ordinal is well-formed.
    level: Option<u8>,
    /// The word is `level`, not `Level`.
    lowercase: bool,
    /// What follows the word `level`.
    rest: &'a str,
}

/// Detection is broad: `At ` + digits + one of `st`, `nd`, `rd`, `th` + a space + `Level` or
/// `level`. `text` must start with it.
fn level_head(text: &str) -> Option<LevelHead<'_>> {
    let text = text.strip_prefix("At ")?;
    let digits_len = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits_len == 0 {
        return None;
    }
    let (digits, text) = text.split_at(digits_len);
    let suffix = ["st", "nd", "rd", "th"]
        .into_iter()
        .find(|suffix| text.starts_with(suffix))?;
    let text = text.strip_prefix(suffix)?.strip_prefix(' ')?;
    let (lowercase, rest) = if let Some(rest) = text.strip_prefix("level") {
        (true, rest)
    } else {
        (false, text.strip_prefix("Level")?)
    };
    Some(LevelHead {
        level: well_formed_ordinal(digits, suffix),
        lowercase,
        rest,
    })
}

// ---------------------------------------------------------------------------------------------
// Block kinds
// ---------------------------------------------------------------------------------------------

/// A `**At 11th level,** <printed sentence>` paragraph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lead<'a> {
    /// The level, when the ordinal is well-formed.
    pub level: Option<u8>,
    /// The word is `level`, not `Level`.
    pub lowercase: bool,
    /// The inlines after the bold lead.
    pub rest: &'a [Inline],
}

impl Lead<'_> {
    /// Canonical: a well-formed ordinal, the word `level`, and at least one inline after it.
    pub fn is_canonical(&self) -> bool {
        self.level.is_some() && self.lowercase && !self.rest.is_empty()
    }
}

/// A `**Label:** content` paragraph. The content can be empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Row<'a> {
    /// The bold text without its colon.
    pub label: String,
    /// The remaining inlines.
    pub content: &'a [Inline],
}

impl Row<'_> {
    /// An empty row: no text after the label.
    pub fn is_empty(&self) -> bool {
        plain_text(self.content).trim().is_empty()
    }

    /// The level of an old upgrade label `At <digits><suffix> <L|l>evel`: `Some(None)` when the
    /// label has that shape with an ill-formed ordinal.
    fn old_upgrade_label(&self) -> Option<Option<u8>> {
        level_head(&self.label)
            .filter(|head| head.rest.is_empty())
            .map(|head| head.level)
    }
}

/// A plain-text upgrade `At 8th level: The bonus increases to +6.`, which the import left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegacyUpgrade<'a> {
    /// The level, when the ordinal is well-formed.
    pub level: Option<u8>,
    /// `:` or `,`.
    pub separator: char,
    /// The rest of the first text after the separator and one optional space.
    pub first: &'a str,
    /// The inlines after that first text.
    pub rest: &'a [Inline],
}

/// What one top-level block is for the power layout. The first match wins in this order.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind<'a> {
    /// `**At 11th level,** text`.
    Lead(Lead<'a>),
    /// `**Label:** content`.
    Row(Row<'a>),
    /// `**Name**`: a paragraph of one bold span that does not end with a colon.
    Name(String),
    /// A plain-text upgrade.
    LegacyUpgrade(LegacyUpgrade<'a>),
    /// A heading, list, table, quote, stat block, entry link or styled paragraph.
    Boundary,
    /// Any other paragraph.
    Paragraph,
}

/// Classify one top-level block.
pub fn classify(block: &Block) -> BlockKind<'_> {
    let content = match block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        } => content.as_slice(),
        Block::Paragraph { .. }
        | Block::Heading { .. }
        | Block::List { .. }
        | Block::Table(_)
        | Block::Quote { .. }
        | Block::StatBlock { .. }
        | Block::EntryLink { .. } => return BlockKind::Boundary,
    };
    if let Some(Inline::Strong { c }) = content.first() {
        let text = plain_text(c);
        if let Some(head) = level_head(&text)
            && head.rest == ","
        {
            return BlockKind::Lead(Lead {
                level: head.level,
                lowercase: head.lowercase,
                rest: &content[1..],
            });
        }
        if let Some(label) = text.strip_suffix(':') {
            return BlockKind::Row(Row {
                label: label.to_string(),
                content: &content[1..],
            });
        }
        if content.len() == 1 {
            return BlockKind::Name(text);
        }
    }
    if let Some(Inline::Text { v }) = content.first()
        && let Some(head) = level_head(v)
        && let Some(separator) = head.rest.chars().next().filter(|c| matches!(c, ':' | ','))
    {
        let after = &head.rest[1..];
        return BlockKind::LegacyUpgrade(LegacyUpgrade {
            level: head.level,
            separator,
            first: after.strip_prefix(' ').unwrap_or(after),
            rest: &content[1..],
        });
    }
    BlockKind::Paragraph
}

fn is_effect_label(label: &str) -> bool {
    EFFECT_LABELS.contains(&label)
}

fn is_alias_label(label: &str) -> bool {
    ALIAS_LABELS.contains(&label)
}

/// What an effect row is, for the pair rule: its alias counts as the canonical label it becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectClass {
    /// `Passive` or `Ability`.
    Passive,
    /// `Standard`, `Move`, `Swift`, `Immediate`, `Free` or `Full-round`.
    Action,
    /// `Benefit`: neither.
    Neither,
}

fn effect_class(label: &str) -> EffectClass {
    let label = match label {
        "Fullround" => "Full-round",
        "Passive Ability" => "Passive",
        "Standard Action" => "Standard",
        other => other,
    };
    if PASSIVE_EFFECT_LABELS.contains(&label) {
        EffectClass::Passive
    } else if ACTION_EFFECT_LABELS.contains(&label) {
        EffectClass::Action
    } else {
        EffectClass::Neither
    }
}

/// Which paragraphs the unlabeled paragraphs that follow a row continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Continues {
    /// Nothing: an unlabeled paragraph here is stray.
    Nothing,
    /// The first effect row, in print order.
    Effect,
    /// The second effect row.
    Second,
    /// An effect row that is already a problem: its paragraphs are dropped without a new one.
    Dropped,
    /// The `Special` row at this index of `other`.
    Special(usize),
}

/// An effect label, an alias or `Action` followed by digits: `Ability2`.
fn is_numbered_effect_label(label: &str) -> bool {
    let base = label.trim_end_matches(|c: char| c.is_ascii_digit());
    base.len() < label.len()
        && (is_effect_label(base) || is_alias_label(base) || base == ACTION_LABEL)
}

/// A row label that marks a power: an effect label, an alias or a numbered effect label.
fn is_effect_marker(label: &str) -> bool {
    is_effect_label(label) || is_alias_label(label) || is_numbered_effect_label(label)
}

/// A row, lead or legacy upgrade that marks a run as a power.
fn is_power_marker(kind: &BlockKind<'_>) -> bool {
    match kind {
        BlockKind::Lead(_) | BlockKind::LegacyUpgrade(_) => true,
        BlockKind::Row(row) => {
            row.label == GAINED
                || row.label == ACTION_LABEL
                || is_effect_marker(&row.label)
                || row.old_upgrade_label().is_some()
        }
        BlockKind::Name(_) | BlockKind::Boundary | BlockKind::Paragraph => false,
    }
}

// ---------------------------------------------------------------------------------------------
// Candidates
// ---------------------------------------------------------------------------------------------

/// A section label that is never a power: one of [`NON_POWER_LABELS`], matched on the name's
/// text trimmed, without a trailing colon, with runs of white space folded, ignoring ASCII case.
pub fn is_non_power_label(name: &str) -> bool {
    let folded = name
        .trim()
        .trim_end_matches(':')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    NON_POWER_LABELS
        .iter()
        .any(|label| label.eq_ignore_ascii_case(&folded))
}

/// A name directly followed by a row, a lead or a legacy upgrade, and the run that follows it up
/// to the next boundary or name, when that run holds a power marker. A name that is a
/// [`NON_POWER_LABELS`] section label is never a candidate; it still ends the run before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The index of the name block in the body.
    pub start: usize,
    /// One past the last block of the run.
    pub end: usize,
    /// The plain text of the name.
    pub name: String,
    /// The position among the candidates of the body with this same name, from 1.
    pub occurrence: usize,
}

impl Candidate {
    /// The blocks of the power in `body`: the name, then its run.
    pub fn blocks<'a>(&self, body: &'a [Block]) -> &'a [Block] {
        &body[self.start..self.end]
    }
}

/// Every candidate in the top-level blocks of `body`, in order. Blocks inside lists and quotes
/// are not looked at: a list or a quote is a boundary.
pub fn candidates(body: &[Block]) -> Vec<Candidate> {
    let kinds: Vec<BlockKind<'_>> = body.iter().map(classify).collect();
    let mut out: Vec<Candidate> = Vec::new();
    let mut at = 0;
    while at < kinds.len() {
        let BlockKind::Name(name) = &kinds[at] else {
            at += 1;
            continue;
        };
        let starts_run = matches!(
            kinds.get(at + 1),
            Some(BlockKind::Row(_) | BlockKind::Lead(_) | BlockKind::LegacyUpgrade(_))
        );
        if !starts_run || is_non_power_label(name) {
            at += 1;
            continue;
        }
        let end = (at + 1..kinds.len())
            .find(|&index| matches!(kinds[index], BlockKind::Boundary | BlockKind::Name(_)))
            .unwrap_or(kinds.len());
        if !kinds[at + 1..end].iter().any(is_power_marker) {
            at += 1;
            continue;
        }
        let occurrence = out.iter().filter(|other| &other.name == name).count() + 1;
        out.push(Candidate {
            start: at,
            end,
            name: name.clone(),
            occurrence,
        });
        at = end;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------------------------

/// A reason a candidate fails validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerProblem {
    /// The first block is not a name (`**Name**`). A legacy name such as `**Accurate Strike (Ex):**`
    /// is an empty row, so a run that starts with it is no candidate.
    NotAName,
    /// A name or a boundary inside the span: the span is more than one power's run.
    BlockInSpan,
    /// A row with no content after its label. It is a row, never a name.
    EmptyRow { label: String },
    /// A plain-text upgrade, which is invalid in every position.
    LegacyUpgrade,
    /// `Action`, an alias or an old upgrade label.
    LegacyLabel { label: String },
    /// A numbered effect label such as `Ability2`.
    NumberedEffectLabel { label: String },
    /// A lead whose ordinal is ill-formed, whose word is `Level`, or that holds no text.
    NonCanonicalLead,
    /// A `Gained` value that is not `At <well-formed ordinal> Level`.
    NonCanonicalGained,
    /// No effect row. `suggest_lowest_upgrade` holds when the run has no `Gained` row but has an
    /// upgrade: its lowest upgrade can become the `Gained` level and the base effect.
    NoEffectRow { suggest_lowest_upgrade: bool },
    /// More than one effect row, other than a passive row and an action row in either order: a
    /// third effect row, two passive rows, two action rows, or a `Benefit` row beside another.
    MultipleEffectRows,
    /// A second `Gained`, `Flavor`, `Choice`, `Info`, `Usage`, `Prereq` or `Replaces` row.
    SecondSingletonRow { label: String },
    /// A row or an upgrade in the wrong position of the canonical order.
    OutOfOrder { label: String },
    /// An unlabeled paragraph that is not directly after the effect row or its continuation.
    StrayParagraph,
    /// An upgrade that is not above the `Gained` level.
    UpgradeNotAboveGained { level: u8 },
    /// Two upgrades at one level.
    UpgradeAtSameLevel { level: u8 },
    /// An upgrade below the one before it.
    UpgradesNotAscending { level: u8 },
}

impl PowerProblem {
    /// A stable slug for counts and reports.
    pub fn reason(&self) -> &'static str {
        match self {
            PowerProblem::NotAName => "not-a-name",
            PowerProblem::BlockInSpan => "block-in-span",
            PowerProblem::EmptyRow { .. } => "empty-row",
            PowerProblem::LegacyUpgrade => "legacy-upgrade",
            PowerProblem::LegacyLabel { .. } => "legacy-label",
            PowerProblem::NumberedEffectLabel { .. } => "numbered-effect-label",
            PowerProblem::NonCanonicalLead => "non-canonical-lead",
            PowerProblem::NonCanonicalGained => "non-canonical-gained",
            PowerProblem::NoEffectRow { .. } => "no-effect-row",
            PowerProblem::MultipleEffectRows => "multiple-effect-rows",
            PowerProblem::SecondSingletonRow { .. } => "second-singleton-row",
            PowerProblem::OutOfOrder { .. } => "out-of-order",
            PowerProblem::StrayParagraph => "stray-paragraph",
            PowerProblem::UpgradeNotAboveGained { .. } => "upgrade-not-above-gained",
            PowerProblem::UpgradeAtSameLevel { .. } => "upgrade-at-same-level",
            PowerProblem::UpgradesNotAscending { .. } => "upgrades-not-ascending",
        }
    }
}

impl fmt::Display for PowerProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PowerProblem::NotAName => write!(f, "the first block is not a power name"),
            PowerProblem::BlockInSpan => {
                write!(f, "a name or a boundary inside the power's run")
            }
            PowerProblem::EmptyRow { label } => {
                write!(f, "the `{label}:` row has no content")
            }
            PowerProblem::LegacyUpgrade => {
                write!(f, "a plain-text upgrade; write `**At Nth level,** text`")
            }
            PowerProblem::LegacyLabel { label } => {
                write!(f, "the legacy label `{label}:`")
            }
            PowerProblem::NumberedEffectLabel { label } => {
                write!(f, "the numbered effect label `{label}:`")
            }
            PowerProblem::NonCanonicalLead => write!(
                f,
                "an upgrade lead that is not `**At Nth level,**` with a well-formed ordinal and text after it"
            ),
            PowerProblem::NonCanonicalGained => {
                write!(f, "a `Gained` value that is not `At Nth Level`")
            }
            PowerProblem::NoEffectRow {
                suggest_lowest_upgrade,
            } => {
                write!(f, "no effect row")?;
                if *suggest_lowest_upgrade {
                    write!(
                        f,
                        " (the lowest upgrade can become the `Gained` level and the base effect)"
                    )?;
                }
                Ok(())
            }
            PowerProblem::MultipleEffectRows => write!(
                f,
                "more than one effect row (two are valid only as a passive row and an action row)"
            ),
            PowerProblem::SecondSingletonRow { label } => {
                write!(f, "a second `{label}:` row")
            }
            PowerProblem::OutOfOrder { label } => {
                write!(f, "`{label}` is out of order")
            }
            PowerProblem::StrayParagraph => write!(
                f,
                "an unlabeled paragraph that is not directly after the effect row"
            ),
            PowerProblem::UpgradeNotAboveGained { level } => {
                write!(f, "the level {level} upgrade is not above `Gained`")
            }
            PowerProblem::UpgradeAtSameLevel { level } => {
                write!(f, "two upgrades at level {level}")
            }
            PowerProblem::UpgradesNotAscending { level } => {
                write!(f, "the level {level} upgrade is not in ascending order")
            }
        }
    }
}

/// An effect row of a valid power.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect<'a> {
    /// `Passive`, `Ability`, `Standard`, ...
    pub label: String,
    pub content: &'a [Inline],
    /// The unlabeled paragraphs directly after the effect row.
    pub continuation: Vec<&'a [Inline]>,
}

impl Effect<'_> {
    /// The row names an action: `Standard`, `Move`, `Swift`, `Immediate`, `Free` or `Full-round`.
    pub fn is_action(&self) -> bool {
        effect_class(&self.label) == EffectClass::Action
    }
}

/// A row of position 11 with the unlabeled paragraphs that continue it. Only a `Special` row has
/// continuation paragraphs.
#[derive(Debug, Clone, PartialEq)]
pub struct OtherRow<'a> {
    /// The bold text without its colon.
    pub label: String,
    pub content: &'a [Inline],
    /// The unlabeled paragraphs directly after a `Special` row, in print order.
    pub continuation: Vec<&'a [Inline]>,
}

/// One upgrade of a valid power: `**At 11th level,** text`.
#[derive(Debug, Clone, PartialEq)]
pub struct Upgrade<'a> {
    pub level: u8,
    /// The inlines after the bold lead.
    pub text: &'a [Inline],
}

/// A power that passes validation: its parts, borrowed from the blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Power<'a> {
    pub name: String,
    /// The level of the `Gained` row.
    pub gained: Option<u8>,
    pub flavor: Option<&'a [Inline]>,
    /// The `Choice` and `Info` rows, which stand right after `Flavor`, in source order. Each label
    /// appears at most once.
    pub preface: Vec<Row<'a>>,
    /// The first effect row, in print order.
    pub effect: Effect<'a>,
    /// The second effect row, in print order, when the power has two. A power with two effect rows
    /// has a passive row and an action row, in either order: print's order is authoritative.
    pub second_effect: Option<Effect<'a>>,
    pub usage: Option<&'a [Inline]>,
    pub upgrades: Vec<Upgrade<'a>>,
    pub prereq: Option<&'a [Inline]>,
    pub replaces: Option<&'a [Inline]>,
    /// Rows in position 11, in source order: `Special`, `Penalty`, `Functions As`, ...
    pub other: Vec<OtherRow<'a>>,
}

impl Power<'_> {
    /// The effect row that names an action, if the power has one.
    pub fn action(&self) -> Option<&Effect<'_>> {
        std::iter::once(&self.effect)
            .chain(self.second_effect.as_ref())
            .find(|effect| effect.is_action())
    }
}

// The position of a row in the canonical order.
const RANK_GAINED: u8 = 2;
const RANK_FLAVOR: u8 = 3;
const RANK_PREFACE: u8 = 4;
const RANK_EFFECT: u8 = 5;
const RANK_USAGE: u8 = 7;
const RANK_UPGRADE: u8 = 8;
const RANK_PREREQ: u8 = 9;
const RANK_REPLACES: u8 = 10;
const RANK_OTHER: u8 = 11;

fn rank_of(label: &str) -> u8 {
    match label {
        GAINED => RANK_GAINED,
        FLAVOR => RANK_FLAVOR,
        CHOICE | INFO => RANK_PREFACE,
        USAGE => RANK_USAGE,
        PREREQ => RANK_PREREQ,
        REPLACES => RANK_REPLACES,
        _ => RANK_OTHER,
    }
}

/// The level a `Gained` row names, when its content is the text `At <well-formed ordinal> Level`.
fn gained_level(content: &[Inline]) -> Option<u8> {
    if !content
        .iter()
        .all(|inline| matches!(inline, Inline::Text { .. }))
    {
        return None;
    }
    let text = plain_text(content);
    let head = level_head(text.trim())?;
    (head.rest.is_empty() && !head.lowercase)
        .then_some(head.level)
        .flatten()
}

/// Validate one candidate against the canonical form: the blocks are the name, then its run.
///
/// It rejects each rule of the canonical order that the run breaks, an empty row, a legacy upgrade
/// in any position, any legacy label, a non-canonical lead or `Gained` value, a missing effect row,
/// more than one effect row unless they are a passive row (`Passive`, `Ability`) and an action row
/// (`Standard`, `Move`, `Swift`, `Immediate`, `Free`, `Full-round`) in either order, a second
/// singleton row, and an unlabeled paragraph that is not in the continuation of an effect row or
/// of a `Special` row. Every problem found is returned, in the order of the blocks.
pub fn validate_power(blocks: &[Block]) -> Result<Power<'_>, Vec<PowerProblem>> {
    let kinds: Vec<BlockKind<'_>> = blocks.iter().map(classify).collect();
    let name = match kinds.first() {
        Some(BlockKind::Name(name)) => name.clone(),
        _ => String::new(),
    };
    let run = kinds.get(1..).unwrap_or_default();

    let mut problems: Vec<PowerProblem> = Vec::new();
    let mut note = |problem: PowerProblem| {
        if !problems.contains(&problem) {
            problems.push(problem);
        }
    };

    // The span must be what `candidates` finds: a name, then a run with no name or boundary.
    if !matches!(kinds.first(), Some(BlockKind::Name(_))) {
        note(PowerProblem::NotAName);
    }
    if run
        .iter()
        .any(|kind| matches!(kind, BlockKind::Name(_) | BlockKind::Boundary))
    {
        note(PowerProblem::BlockInSpan);
    }

    let gained = run.iter().find_map(|kind| match kind {
        BlockKind::Row(row) if row.label == GAINED => gained_level(row.content),
        _ => None,
    });
    // `Gained` metadata: a `Gained` row, or an `Action` row whose value is `At Nth Level`.
    let has_gained_row = run.iter().any(|kind| match kind {
        BlockKind::Row(row) if row.label == GAINED => true,
        BlockKind::Row(row) if row.label == ACTION_LABEL => {
            matches!(map_action_value(row.content), ActionMapping::Gained(_))
        }
        _ => false,
    });
    let has_upgrade = run.iter().any(|kind| match kind {
        BlockKind::Lead(_) | BlockKind::LegacyUpgrade(_) => true,
        BlockKind::Row(row) => row.old_upgrade_label().is_some(),
        _ => false,
    });

    let mut last_rank = 1u8;
    let mut seen_singletons: Vec<&str> = Vec::new();
    let mut effect_rows = 0usize;
    let mut continues = Continues::Nothing;
    let mut first_class = EffectClass::Neither;
    let mut last_upgrade: Option<u8> = None;

    let mut power = Power {
        name,
        gained,
        flavor: None,
        preface: Vec::new(),
        effect: Effect {
            label: String::new(),
            content: &[],
            continuation: Vec::new(),
        },
        second_effect: None,
        usage: None,
        upgrades: Vec::new(),
        prereq: None,
        replaces: None,
        other: Vec::new(),
    };

    for (index, kind) in run.iter().enumerate() {
        match kind {
            BlockKind::Row(row) => {
                let label = row.label.as_str();
                continues = Continues::Nothing;
                if row.is_empty() {
                    note(PowerProblem::EmptyRow {
                        label: row.label.clone(),
                    });
                }
                if is_effect_label(label) || is_alias_label(label) {
                    effect_rows += 1;
                    if is_alias_label(label) {
                        note(PowerProblem::LegacyLabel {
                            label: row.label.clone(),
                        });
                    }
                    let class = effect_class(label);
                    // Two rows are valid as a passive row and an action row, in either order.
                    let pair_ok = effect_rows == 2
                        && matches!(
                            (first_class, class),
                            (EffectClass::Passive, EffectClass::Action)
                                | (EffectClass::Action, EffectClass::Passive)
                        );
                    if effect_rows > 2 || (effect_rows == 2 && !pair_ok) {
                        note(PowerProblem::MultipleEffectRows);
                    }
                    if effect_rows == 1 || pair_ok {
                        if last_rank > RANK_EFFECT {
                            note(PowerProblem::OutOfOrder {
                                label: row.label.clone(),
                            });
                        }
                        last_rank = last_rank.max(RANK_EFFECT);
                    }
                    if effect_rows == 1 {
                        first_class = class;
                        power.effect.label = row.label.clone();
                        power.effect.content = row.content;
                        continues = Continues::Effect;
                    } else if pair_ok {
                        power.second_effect = Some(Effect {
                            label: row.label.clone(),
                            content: row.content,
                            continuation: Vec::new(),
                        });
                        continues = Continues::Second;
                    } else {
                        continues = Continues::Dropped;
                    }
                    continue;
                }
                // A numbered effect label is not an effect row: it is its own reason.
                if is_numbered_effect_label(label) {
                    note(PowerProblem::NumberedEffectLabel {
                        label: row.label.clone(),
                    });
                    continue;
                }
                if label == ACTION_LABEL || row.old_upgrade_label().is_some() {
                    note(PowerProblem::LegacyLabel {
                        label: row.label.clone(),
                    });
                    continue;
                }
                let rank = rank_of(label);
                if rank != RANK_OTHER {
                    if seen_singletons.contains(&label) {
                        note(PowerProblem::SecondSingletonRow {
                            label: row.label.clone(),
                        });
                        continue;
                    }
                    seen_singletons.push(label);
                }
                if rank < last_rank {
                    note(PowerProblem::OutOfOrder {
                        label: row.label.clone(),
                    });
                }
                last_rank = last_rank.max(rank);
                match label {
                    GAINED => {
                        if gained_level(row.content).is_none() {
                            note(PowerProblem::NonCanonicalGained);
                        }
                    }
                    FLAVOR => power.flavor = Some(row.content),
                    CHOICE | INFO => power.preface.push(row.clone()),
                    USAGE => power.usage = Some(row.content),
                    PREREQ => power.prereq = Some(row.content),
                    REPLACES => power.replaces = Some(row.content),
                    _ => {
                        if label == SPECIAL {
                            continues = Continues::Special(power.other.len());
                        }
                        power.other.push(OtherRow {
                            label: row.label.clone(),
                            content: row.content,
                            continuation: Vec::new(),
                        });
                    }
                }
            }
            BlockKind::Lead(lead) => {
                continues = Continues::Nothing;
                if !lead.is_canonical() {
                    note(PowerProblem::NonCanonicalLead);
                    continue;
                }
                let level = lead.level.unwrap_or_default();
                if gained.is_some_and(|gained| level <= gained) {
                    note(PowerProblem::UpgradeNotAboveGained { level });
                }
                match last_upgrade {
                    Some(previous) if level == previous => {
                        note(PowerProblem::UpgradeAtSameLevel { level });
                    }
                    Some(previous) if level < previous => {
                        note(PowerProblem::UpgradesNotAscending { level });
                    }
                    _ => {}
                }
                last_upgrade = Some(level);
                if last_rank > RANK_UPGRADE {
                    note(PowerProblem::OutOfOrder {
                        label: format!("At {level}{} level,", ordinal_suffix(level)),
                    });
                }
                last_rank = last_rank.max(RANK_UPGRADE);
                power.upgrades.push(Upgrade {
                    level,
                    text: lead.rest,
                });
            }
            BlockKind::LegacyUpgrade(_) => {
                continues = Continues::Nothing;
                note(PowerProblem::LegacyUpgrade);
            }
            BlockKind::Paragraph => {
                // An unlabeled paragraph continues the row right before it, and only an effect row
                // or a `Special` row has continuation paragraphs.
                // `blocks[0]` is the name, so the run's block `index` is `blocks[index + 1]`.
                let Block::Paragraph { content, .. } = &blocks[index + 1] else {
                    continue;
                };
                match continues {
                    Continues::Effect => power.effect.continuation.push(content),
                    Continues::Second => {
                        if let Some(second) = power.second_effect.as_mut() {
                            second.continuation.push(content);
                        }
                    }
                    Continues::Special(at) => power.other[at].continuation.push(content),
                    Continues::Dropped => {}
                    Continues::Nothing => note(PowerProblem::StrayParagraph),
                }
            }
            // Reported once above.
            BlockKind::Name(_) | BlockKind::Boundary => {}
        }
    }
    if effect_rows == 0 {
        note(PowerProblem::NoEffectRow {
            suggest_lowest_upgrade: !has_gained_row && has_upgrade,
        });
    }
    if problems.is_empty() {
        Ok(power)
    } else {
        Err(problems)
    }
}

// ---------------------------------------------------------------------------------------------
// The legacy `Action` row
// ---------------------------------------------------------------------------------------------

/// What the value of a legacy `**Action:** <value>` row becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionMapping {
    /// The row becomes the effect row with this label. The effect text is the run's group of
    /// unlabeled paragraphs.
    Effect(&'static str),
    /// `At Nth Level`: a `Gained` row only. It never takes effect text.
    Gained(u8),
    /// Not in the map. The run is queued for print review.
    Unmapped,
}

/// The explicit map of an `Action` value. There is no catch-all.
pub fn map_action_value(value: &[Inline]) -> ActionMapping {
    if !value
        .iter()
        .all(|inline| matches!(inline, Inline::Text { .. }))
    {
        return ActionMapping::Unmapped;
    }
    let text = plain_text(value);
    let text = text.trim();
    match text {
        "Ability" | "ability" => ActionMapping::Effect("Ability"),
        "Passive Ability" => ActionMapping::Effect("Passive"),
        "Standard Action" | "Standard" | "Standard." => ActionMapping::Effect("Standard"),
        "Move Action" => ActionMapping::Effect("Move"),
        "Swift Action" => ActionMapping::Effect("Swift"),
        "Immediate Action" => ActionMapping::Effect("Immediate"),
        "Free Action" => ActionMapping::Effect("Free"),
        "Full-Round Action" | "Full-round." => ActionMapping::Effect("Full-round"),
        _ => match level_head(text) {
            Some(head) if head.rest.is_empty() && !head.lowercase => head
                .level
                .map_or(ActionMapping::Unmapped, ActionMapping::Gained),
            _ => ActionMapping::Unmapped,
        },
    }
}

/// Why the effect text of a mapped `Action` row cannot be settled without a person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionGroupProblem {
    /// The block at the index is not an `Action` row whose value maps to an effect label. An
    /// `At Nth Level` value is `Gained` metadata and never takes effect text.
    NotAnEffectAction,
    /// No group of unlabeled paragraphs after the `Action` row.
    NoGroup,
    /// A group of unlabeled paragraphs before the `Action` row.
    GroupBeforeRow,
    /// More than one group of unlabeled paragraphs.
    SecondGroup,
    /// The group is not directly reachable: this lead, legacy upgrade, empty row or row other
    /// than `Gained`, `Flavor`, `Choice`, `Info`, `Usage`, `Prereq` and `Replaces` stands between
    /// the row and it.
    Crosses { what: String },
}

/// Find the effect text of the `Action` row at `action` in a candidate's blocks (the name is
/// block 0). The block at `action` must be an `Action` row whose value maps to an effect label
/// ([`ActionMapping::Effect`]). The run must hold exactly one group of consecutive unlabeled
/// paragraphs, not counting the paragraphs that continue a `Special` row (those directly after a
/// `Special` row, which stay with its note). The group must come after the `Action` row, and only
/// rows labeled `Gained`, `Flavor`, `Choice`, `Info`, `Usage`, `Prereq` or `Replaces`, each with
/// content, may stand between them. The result is the range of the group in `blocks`.
pub fn action_effect_group(
    blocks: &[Block],
    action: usize,
) -> Result<Range<usize>, ActionGroupProblem> {
    let kinds: Vec<BlockKind<'_>> = blocks.iter().map(classify).collect();
    let is_effect_action = action > 0
        && matches!(
            kinds.get(action),
            Some(BlockKind::Row(row))
                if row.label == ACTION_LABEL
                    && matches!(map_action_value(row.content), ActionMapping::Effect(_))
        );
    if !is_effect_action {
        return Err(ActionGroupProblem::NotAnEffectAction);
    }
    let mut groups: Vec<Range<usize>> = Vec::new();
    // The unlabeled paragraphs directly after a `Special` row continue its note, not the effect.
    let mut in_special = false;
    for (index, kind) in kinds.iter().enumerate().skip(1) {
        match kind {
            BlockKind::Paragraph if in_special => {}
            BlockKind::Paragraph => match groups.last_mut() {
                Some(group) if group.end == index => group.end = index + 1,
                _ => groups.push(index..index + 1),
            },
            BlockKind::Row(row) => in_special = row.label == SPECIAL,
            _ => in_special = false,
        }
    }
    let group = match groups.as_slice() {
        [] => return Err(ActionGroupProblem::NoGroup),
        [group] => group.clone(),
        _ => return Err(ActionGroupProblem::SecondGroup),
    };
    if group.start < action {
        return Err(ActionGroupProblem::GroupBeforeRow);
    }
    for kind in &kinds[action + 1..group.start] {
        let crossed = match kind {
            BlockKind::Row(row)
                if !row.is_empty()
                    && matches!(
                        row.label.as_str(),
                        GAINED | FLAVOR | CHOICE | INFO | USAGE | PREREQ | REPLACES
                    ) =>
            {
                continue;
            }
            BlockKind::Row(row) if row.is_empty() => format!("the empty row `{}:`", row.label),
            BlockKind::Row(row) => format!("the row `{}:`", row.label),
            BlockKind::Lead(_) => "an upgrade lead".to_string(),
            BlockKind::LegacyUpgrade(_) => "a plain-text upgrade".to_string(),
            BlockKind::Name(_) | BlockKind::Boundary | BlockKind::Paragraph => {
                "another block".to_string()
            }
        };
        return Err(ActionGroupProblem::Crosses { what: crossed });
    }
    Ok(group)
}

// ---------------------------------------------------------------------------------------------
// One-power pages and heading levels
// ---------------------------------------------------------------------------------------------

/// The ability tags of a power name.
const ABILITY_TAGS: [&str; 3] = ["Ex", "Su", "Sp"];

/// Split a trailing ability tag from a power name. The tag is `(Ex)`, `(Su)` or `(Sp)`, or a
/// combination joined by `, `, ` or ` or ` and `, such as `(Ex or Su)` or `(Sp and Ex)`. The
/// name part has no trailing space; the tag is returned as written, with its parentheses.
pub fn split_ability_tag(name: &str) -> (&str, Option<&str>) {
    let trimmed = name.trim_end();
    let Some(open) = trimmed
        .strip_suffix(')')
        .and_then(|before| before.rfind('('))
    else {
        return (trimmed, None);
    };
    let inner = &trimmed[open + 1..trimmed.len() - 1];
    if is_tag_list(inner) {
        (trimmed[..open].trim_end(), Some(&trimmed[open..]))
    } else {
        (trimmed, None)
    }
}

/// The separators of a combined ability tag, longest first.
const TAG_SEPARATORS: [&str; 5] = [", or ", ", and ", ", ", " or ", " and "];

/// `Ex`, `Su` and `Sp` joined by the allowed separators and nothing else.
fn is_tag_list(mut rest: &str) -> bool {
    loop {
        let Some(after) = ABILITY_TAGS.iter().find_map(|tag| rest.strip_prefix(tag)) else {
            return false;
        };
        if after.is_empty() {
            return true;
        }
        let Some(next) = TAG_SEPARATORS
            .iter()
            .find_map(|separator| after.strip_prefix(separator))
        else {
            return false;
        };
        rest = next;
    }
}

/// A title or name for comparison: lowercase, with the typographic apostrophes straight.
fn normalized(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{2019}' | '\u{2018}' | '\u{02BC}' => '\'',
            other => other,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// The power of a one-power page, when `body` is one. All four hold: the body has exactly one
/// candidate, it passes validation, its name is the first block of the body, and the name without
/// its ability tag equals the entry `title`, ignoring case and apostrophe style.
pub fn one_power_page<'a>(title: &str, body: &'a [Block]) -> Option<Power<'a>> {
    let found = candidates(body);
    let [candidate] = found.as_slice() else {
        return None;
    };
    if candidate.start != 0 {
        return None;
    }
    let power = validate_power(candidate.blocks(body)).ok()?;
    let (base, _) = split_ability_tag(&power.name);
    (normalized(base) == normalized(title)).then_some(power)
}

/// The heading level of the power name at block `index` of `body`. A power before the first
/// top-level heading of the body is `h2`. After one, it is one level below the preceding top-level
/// body heading, at most `h6`.
pub fn power_heading_level(body: &[Block], index: usize) -> u8 {
    body[..index.min(body.len())]
        .iter()
        .rev()
        .find_map(|block| match block {
            Block::Heading { level, .. } => Some(level.saturating_add(1).min(6)),
            _ => None,
        })
        .unwrap_or(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryId, Table};

    fn text(value: &str) -> Inline {
        Inline::text(value)
    }

    fn strong(value: &str) -> Inline {
        Inline::Strong {
            c: vec![text(value)],
        }
    }

    fn paragraph(content: Vec<Inline>) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }
    }

    fn kind(content: Vec<Inline>) -> BlockKind<'static> {
        // The tests classify owned blocks, so leak the block to get a `'static` borrow.
        let block: &'static Block = Box::leak(Box::new(paragraph(content)));
        classify(block)
    }

    #[test]
    fn a_well_formed_ordinal_has_its_english_suffix() {
        for (level, suffix) in [
            (1, "st"),
            (2, "nd"),
            (3, "rd"),
            (4, "th"),
            (11, "th"),
            (12, "th"),
            (13, "th"),
            (20, "th"),
        ] {
            assert_eq!(well_formed_ordinal(&level.to_string(), suffix), Some(level));
        }
        for (digits, suffix) in [
            ("0", "th"),
            ("01", "st"),
            ("21", "st"),
            ("1", "th"),
            ("2", "st"),
            ("3", "th"),
            ("11", "st"),
            ("12", "nd"),
            ("13", "rd"),
            ("", "th"),
            ("x", "th"),
            ("300", "th"),
        ] {
            assert_eq!(
                well_formed_ordinal(digits, suffix),
                None,
                "{digits}{suffix}"
            );
        }
    }

    #[test]
    fn detection_is_broad_and_conversion_needs_a_well_formed_ordinal() {
        let head = level_head("At 11th level, text").unwrap();
        assert_eq!(head.level, Some(11));
        assert!(head.lowercase);
        assert_eq!(head.rest, ", text");
        let head = level_head("At 21st Level:").unwrap();
        assert_eq!((head.level, head.lowercase, head.rest), (None, false, ":"));
        assert!(level_head("At 1th level").is_some());
        assert!(level_head("At level 3").is_none());
        assert!(level_head("at 3rd level").is_none());
        assert!(level_head("At 3rdlevel").is_none());
        assert!(level_head("At 3rd  level").is_none());
        assert!(level_head("At 3rd leve").is_none());
    }

    #[test]
    fn a_lead_is_a_bold_level_with_a_comma() {
        let BlockKind::Lead(lead) = kind(vec![strong("At 11th level,"), text(" more.")]) else {
            panic!("not a lead");
        };
        assert!(lead.is_canonical());
        assert_eq!(lead.level, Some(11));

        for (label, expected) in [
            ("At 11th Level,", false),
            ("At 21st level,", false),
            ("At 1th level,", false),
            ("At 011th level,", false),
        ] {
            let BlockKind::Lead(lead) = kind(vec![strong(label), text(" more.")]) else {
                panic!("`{label}` is a lead");
            };
            assert_eq!(lead.is_canonical(), expected, "{label}");
        }
        // A lead with no inline after it is a lead that is not canonical.
        let BlockKind::Lead(lead) = kind(vec![strong("At 11th level,")]) else {
            panic!("not a lead");
        };
        assert!(!lead.is_canonical());
    }

    #[test]
    fn a_row_ends_its_bold_text_with_a_colon_and_may_have_no_content() {
        let BlockKind::Row(row) = kind(vec![strong("Usage:"), text(" 3/day")]) else {
            panic!("not a row");
        };
        assert_eq!(row.label, "Usage");
        assert!(!row.is_empty());
        // An empty `**Usage:**` paragraph is a row, never a name.
        let BlockKind::Row(row) = kind(vec![strong("Usage:")]) else {
            panic!("not a row");
        };
        assert!(row.is_empty());
        // The old upgrade label is a row.
        let BlockKind::Row(row) = kind(vec![strong("At 8th Level:"), text(" x")]) else {
            panic!("not a row");
        };
        assert_eq!(row.old_upgrade_label(), Some(Some(8)));
    }

    #[test]
    fn a_name_is_one_bold_span_without_a_colon() {
        assert_eq!(
            kind(vec![strong("Resistance (Ex)")]),
            BlockKind::Name("Resistance (Ex)".into())
        );
        // Bold followed by text is a paragraph, not a name.
        assert_eq!(kind(vec![strong("Note"), text(" x")]), BlockKind::Paragraph);
        // Two bold spans are not a name either.
        assert_eq!(
            kind(vec![strong("One"), strong("Two")]),
            BlockKind::Paragraph
        );
    }

    #[test]
    fn a_legacy_upgrade_starts_with_plain_level_text_and_a_separator() {
        let BlockKind::LegacyUpgrade(upgrade) = kind(vec![
            text("At 8th level: The bonus increases to +6."),
            strong(" x"),
        ]) else {
            panic!("not a legacy upgrade");
        };
        assert_eq!(upgrade.level, Some(8));
        assert_eq!(upgrade.separator, ':');
        assert_eq!(upgrade.first, "The bonus increases to +6.");
        assert_eq!(upgrade.rest.len(), 1);

        let BlockKind::LegacyUpgrade(upgrade) = kind(vec![text("At 8th Level,the bonus")]) else {
            panic!("not a legacy upgrade");
        };
        assert_eq!((upgrade.separator, upgrade.first), (',', "the bonus"));

        let BlockKind::LegacyUpgrade(upgrade) = kind(vec![text("At 08th level: x")]) else {
            panic!("not a legacy upgrade");
        };
        assert_eq!(upgrade.level, None);

        // No separator: an ordinary paragraph.
        assert_eq!(
            kind(vec![text("At 8th level the bonus increases.")]),
            BlockKind::Paragraph
        );
    }

    #[test]
    fn structural_blocks_and_styled_paragraphs_are_boundaries() {
        let boundaries = [
            Block::Heading {
                level: 3,
                anchor: None,
                content: vec![text("School")],
            },
            Block::List {
                ordered: false,
                items: vec![],
            },
            Block::Table(Table::default()),
            Block::Quote { content: vec![] },
            Block::StatBlock {
                title: None,
                rows: vec![],
            },
            Block::EntryLink {
                target: EntryId::parse("ref/rule/x").unwrap(),
                title: "X".into(),
            },
            Block::Paragraph {
                style: ParagraphStyle::StatLine,
                content: vec![strong("Name")],
            },
            Block::Paragraph {
                style: ParagraphStyle::Indented,
                content: vec![strong("Row:"), text(" x")],
            },
        ];
        for block in &boundaries {
            assert_eq!(classify(block), BlockKind::Boundary, "{block:?}");
        }
    }

    fn row(label: &str, value: &str) -> Block {
        paragraph(vec![
            strong(&format!("{label}:")),
            text(&format!(" {value}")),
        ])
    }

    fn name(value: &str) -> Block {
        paragraph(vec![strong(value)])
    }

    fn lead(level: &str, value: &str) -> Block {
        paragraph(vec![
            strong(&format!("At {level} level,")),
            text(&format!(" {value}")),
        ])
    }

    fn plain(value: &str) -> Block {
        paragraph(vec![text(value)])
    }

    fn problems(body: &[Block]) -> Vec<&'static str> {
        let found = candidates(body);
        assert_eq!(found.len(), 1, "{found:?}");
        match validate_power(found[0].blocks(body)) {
            Ok(_) => vec![],
            Err(list) => list.iter().map(PowerProblem::reason).collect(),
        }
    }

    #[test]
    fn a_name_needs_a_row_a_lead_or_a_legacy_upgrade_right_after_it() {
        // A paragraph between the name and its rows: no candidate.
        let body = vec![name("Power"), plain("Prose."), row("Passive", "x")];
        assert!(candidates(&body).is_empty());
        // A name and a lead.
        assert_eq!(candidates(&[name("P"), lead("3rd", "x")]).len(), 1);
        // A name and a legacy upgrade.
        assert_eq!(candidates(&[name("P"), plain("At 3rd level: x")]).len(), 1);
    }

    #[test]
    fn a_run_needs_a_power_marker() {
        // An affliction holds no marker, which is why `Effect` is no effect label.
        let affliction = vec![
            name("Blight Hex"),
            row("Type", "Curse"),
            row("Save", "Will negates"),
            row("Frequency", "1/day"),
            row("Effect", "Constitution damage 1"),
        ];
        assert!(candidates(&affliction).is_empty());
        // Each marker alone makes the run a candidate.
        for marker in [
            row("Gained", "At 1st Level"),
            row("Action", "Ability"),
            row("Passive", "x"),
            row("Fullround", "x"),
            row("Passive Ability", "x"),
            row("Ability2", "x"),
            row("At 3rd Level", "x"),
            lead("3rd", "x"),
            plain("At 3rd level, x"),
        ] {
            let body = vec![name("P"), row("Flavor", "x"), marker];
            assert_eq!(candidates(&body).len(), 1, "{body:?}");
        }
    }

    #[test]
    fn a_run_ends_at_a_boundary_or_the_next_name() {
        let body = vec![
            name("One"),
            row("Passive", "x"),
            plain("Prose."),
            Block::Heading {
                level: 3,
                anchor: None,
                content: vec![text("Next")],
            },
            plain("More prose."),
            name("Two"),
            row("Ability", "y"),
            Block::List {
                ordered: false,
                items: vec![],
            },
        ];
        let found = candidates(&body);
        let spans: Vec<(usize, usize)> = found.iter().map(|c| (c.start, c.end)).collect();
        assert_eq!(spans, [(0, 3), (5, 7)]);
    }

    #[test]
    fn blocks_inside_lists_and_quotes_are_not_looked_at() {
        let body = vec![Block::Quote {
            content: vec![name("P"), row("Passive", "x")],
        }];
        assert!(candidates(&body).is_empty());
    }

    #[test]
    fn the_occurrence_counts_candidates_with_the_same_name() {
        let body = vec![
            name("Twin"),
            row("Passive", "a"),
            name("Other"),
            row("Passive", "b"),
            name("Twin"),
            row("Passive", "c"),
        ];
        let found = candidates(&body);
        let occurrences: Vec<(&str, usize)> = found
            .iter()
            .map(|c| (c.name.as_str(), c.occurrence))
            .collect();
        assert_eq!(occurrences, [("Twin", 1), ("Other", 1), ("Twin", 2)]);
    }

    #[test]
    fn gained_must_say_at_a_well_formed_ordinal_level() {
        for good in ["At 1st Level", "At 20th Level", "At 12th Level"] {
            let body = vec![name("P"), row("Gained", good), row("Passive", "x")];
            assert!(problems(&body).is_empty(), "{good}");
        }
        for bad in [
            "At 21st Level",
            "At 1th Level",
            "At 3rd level",
            "At 1st Level.",
            "1st level",
            "At 01st Level",
        ] {
            let body = vec![name("P"), row("Gained", bad), row("Passive", "x")];
            assert_eq!(problems(&body), ["non-canonical-gained"], "{bad}");
        }
    }

    #[test]
    fn leads_must_be_canonical() {
        for bad in ["3th", "21st", "03rd"] {
            let body = vec![name("P"), row("Passive", "x"), lead(bad, "y")];
            assert_eq!(problems(&body), ["non-canonical-lead"], "{bad}");
        }
        let capital = vec![
            name("P"),
            row("Passive", "x"),
            paragraph(vec![strong("At 3rd Level,"), text(" y")]),
        ];
        assert_eq!(problems(&capital), ["non-canonical-lead"]);
        let empty = vec![
            name("P"),
            row("Passive", "x"),
            paragraph(vec![strong("At 3rd level,")]),
        ];
        assert_eq!(problems(&empty), ["non-canonical-lead"]);
    }

    #[test]
    fn upgrades_ascend_strictly_and_stay_above_gained() {
        let ascending = vec![
            name("P"),
            row("Gained", "At 3rd Level"),
            row("Passive", "x"),
            lead("4th", "a"),
            lead("11th", "b"),
        ];
        assert!(problems(&ascending).is_empty());
        let same = vec![
            name("P"),
            row("Passive", "x"),
            lead("4th", "a"),
            lead("4th", "b"),
        ];
        assert_eq!(problems(&same), ["upgrade-at-same-level"]);
        let descending = vec![
            name("P"),
            row("Passive", "x"),
            lead("11th", "a"),
            lead("4th", "b"),
        ];
        assert_eq!(problems(&descending), ["upgrades-not-ascending"]);
        let at_gained = vec![
            name("P"),
            row("Gained", "At 3rd Level"),
            row("Passive", "x"),
            lead("3rd", "a"),
        ];
        assert_eq!(problems(&at_gained), ["upgrade-not-above-gained"]);
        let below_gained = vec![
            name("P"),
            row("Gained", "At 8th Level"),
            row("Passive", "x"),
            lead("3rd", "a"),
        ];
        assert_eq!(problems(&below_gained), ["upgrade-not-above-gained"]);
    }

    #[test]
    fn unlabeled_paragraphs_belong_only_directly_after_the_effect_row() {
        let continuation = vec![
            name("P"),
            row("Gained", "At 1st Level"),
            row("Passive", "x"),
            plain("One."),
            plain("Two."),
            row("Usage", "y"),
        ];
        let found = candidates(&continuation);
        let power = validate_power(found[0].blocks(&continuation)).unwrap();
        assert_eq!(power.effect.continuation.len(), 2);

        // Before the effect.
        let before = vec![
            name("P"),
            row("Flavor", "f"),
            plain("Stray."),
            row("Passive", "x"),
        ];
        assert_eq!(problems(&before), ["stray-paragraph"]);
        // After `Usage`.
        let after_usage = vec![
            name("P"),
            row("Passive", "x"),
            row("Usage", "y"),
            plain("Stray."),
        ];
        assert_eq!(problems(&after_usage), ["stray-paragraph"]);
        // After an upgrade.
        let after_lead = vec![
            name("P"),
            row("Passive", "x"),
            lead("3rd", "a"),
            plain("Stray."),
        ];
        assert_eq!(problems(&after_lead), ["stray-paragraph"]);
        // After a Flavor row that follows the effect (the Flavor is out of order too).
        let after_flavor = vec![
            name("P"),
            row("Gained", "At 1st Level"),
            row("Flavor", "f"),
            row("Passive", "x"),
            row("Penalty", "s"),
            plain("Stray."),
        ];
        assert_eq!(problems(&after_flavor), ["stray-paragraph"]);
    }

    #[test]
    fn paragraphs_after_special_continue_the_special_note_in_print_order() {
        let body = vec![
            name("P"),
            row("Passive", "x"),
            row("Special", "First."),
            plain("Second."),
            plain("Third."),
            row("Penalty", "p"),
            row("Special", "Another."),
            plain("Fourth."),
        ];
        let found = candidates(&body);
        let power = validate_power(found[0].blocks(&body)).unwrap();
        let labels: Vec<&str> = power.other.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(labels, ["Special", "Penalty", "Special"]);
        assert_eq!(power.other[0].continuation.len(), 2);
        assert_eq!(power.other[1].continuation.len(), 0);
        assert_eq!(power.other[2].continuation.len(), 1);
        // The effect has none: the paragraphs belong to `Special`.
        assert!(power.effect.continuation.is_empty());

        // After any other row, an unlabeled paragraph is still stray.
        for label in ["Penalty", "Functions As", "Prereq", "Usage", "Replaces"] {
            let body = vec![
                name("P"),
                row("Passive", "x"),
                row(label, "v"),
                plain("Stray."),
            ];
            assert_eq!(problems(&body), ["stray-paragraph"], "{label}");
        }
        // Between `Special` and its paragraph nothing may stand.
        let body = vec![
            name("P"),
            row("Passive", "x"),
            row("Special", "s"),
            row("Penalty", "p"),
            plain("Stray."),
        ];
        assert_eq!(problems(&body), ["stray-paragraph"]);
    }

    #[test]
    fn a_legacy_upgrade_is_invalid_in_every_position() {
        for body in [
            vec![name("P"), plain("At 3rd level: a"), row("Passive", "x")],
            vec![name("P"), row("Passive", "x"), plain("At 3rd level: a")],
            vec![
                name("P"),
                row("Passive", "x"),
                row("Usage", "u"),
                plain("At 3rd level, a"),
            ],
        ] {
            assert!(problems(&body).contains(&"legacy-upgrade"), "{body:?}");
        }
    }

    #[test]
    fn singleton_rows_appear_once_in_the_canonical_order() {
        for label in ["Gained", "Flavor", "Usage", "Prereq", "Replaces"] {
            let value = if label == "Gained" {
                "At 1st Level"
            } else {
                "v"
            };
            // The row in its place twice: the second is a duplicate.
            let mut body = vec![name("P")];
            let rows = [
                ("Gained", "At 1st Level"),
                ("Flavor", "v"),
                ("Passive", "x"),
                ("Usage", "v"),
                ("Prereq", "v"),
                ("Replaces", "v"),
            ];
            for (candidate, text) in rows {
                body.push(row(candidate, text));
                if candidate == label {
                    body.push(row(label, value));
                }
            }
            assert_eq!(problems(&body), ["second-singleton-row"], "{label}");
        }
        // Prereq, then Replaces: the reverse is out of order.
        let swapped = vec![
            name("P"),
            row("Passive", "x"),
            row("Replaces", "r"),
            row("Prereq", "p"),
        ];
        assert_eq!(problems(&swapped), ["out-of-order"]);
        // A row of another label may not come before the effect or between the fixed rows.
        let early = vec![name("P"), row("Special", "s"), row("Passive", "x")];
        assert_eq!(problems(&early), ["out-of-order"]);
        let between = vec![
            name("P"),
            row("Passive", "x"),
            row("Special", "s"),
            row("Usage", "u"),
        ];
        assert_eq!(problems(&between), ["out-of-order"]);
        // Upgrades after the other rows are out of order.
        let late = vec![
            name("P"),
            row("Passive", "x"),
            row("Special", "s"),
            lead("3rd", "a"),
        ];
        assert_eq!(problems(&late), ["out-of-order"]);
    }

    #[test]
    fn a_numbered_label_is_its_own_reason_and_an_alias_is_an_effect_row() {
        // A numbered label is not an effect row: it adds `Ability2` to a run that has `Ability`.
        let twin = vec![name("P"), row("Ability", "x"), row("Ability2", "y")];
        assert_eq!(problems(&twin), ["numbered-effect-label"]);
        // Two real effect rows are two, whatever else the run holds.
        let two = vec![
            name("P"),
            row("Ability", "x"),
            row("Ability2", "y"),
            row("Passive", "z"),
        ];
        assert_eq!(
            problems(&two),
            ["numbered-effect-label", "multiple-effect-rows"]
        );
        // Alone, it leaves the run without an effect row.
        let numbered = vec![name("P"), row("Standard2", "x")];
        assert_eq!(
            problems(&numbered),
            ["numbered-effect-label", "no-effect-row"]
        );
        let alias = vec![name("P"), row("Standard Action", "x")];
        assert_eq!(problems(&alias), ["legacy-label"]);
        let missing = vec![name("P"), row("Gained", "At 1st Level"), row("Flavor", "x")];
        assert_eq!(problems(&missing), ["no-effect-row"]);
    }

    #[test]
    fn the_suggestion_needs_no_gained_row_and_an_upgrade() {
        let found = |body: Vec<Block>| {
            let list = candidates(&body);
            validate_power(list[0].blocks(&body)).unwrap_err()
        };
        let with_gained = found(vec![
            name("P"),
            row("Gained", "At 1st Level"),
            lead("3rd", "x"),
        ]);
        assert!(with_gained.contains(&PowerProblem::NoEffectRow {
            suggest_lowest_upgrade: false
        }));
        let without_upgrade = found(vec![name("P"), row("Gained", "At 1st Level")]);
        assert!(without_upgrade.contains(&PowerProblem::NoEffectRow {
            suggest_lowest_upgrade: false
        }));
    }

    #[test]
    fn the_action_map_is_explicit() {
        let map = |value: &str| map_action_value(&[text(value)]);
        for (value, label) in [
            ("Ability", "Ability"),
            ("ability", "Ability"),
            ("Passive Ability", "Passive"),
            ("Standard Action", "Standard"),
            ("Standard", "Standard"),
            ("Standard.", "Standard"),
            ("Move Action", "Move"),
            ("Swift Action", "Swift"),
            ("Immediate Action", "Immediate"),
            ("Free Action", "Free"),
            ("Full-Round Action", "Full-round"),
            ("Full-round.", "Full-round"),
        ] {
            assert_eq!(map(value), ActionMapping::Effect(label), "{value}");
        }
        assert_eq!(map("At 8th Level"), ActionMapping::Gained(8));
        // On corpus `main` these are not in the map.
        for value in [
            "Move-Equivalent Action",
            "Benefit",
            "Choice",
            "Free Action=",
            "Info",
            "At 8th level",
            "At 21st Level",
            "",
        ] {
            assert_eq!(map(value), ActionMapping::Unmapped, "{value}");
        }
        assert_eq!(
            map_action_value(&[Inline::Em {
                c: vec![text("Ability")]
            }]),
            ActionMapping::Unmapped
        );
    }

    #[test]
    fn the_action_group_must_be_one_group_after_the_row() {
        let group = |body: &[Block], action: usize| action_effect_group(body, action);
        // Gained, Flavor, Choice, Info, Usage, Prereq and Replaces rows with content may stand
        // between.
        let ok = vec![
            name("P"),
            row("Action", "Ability"),
            row("Gained", "At 1st Level"),
            row("Flavor", "f"),
            row("Choice", "c"),
            row("Info", "i"),
            row("Usage", "u"),
            row("Prereq", "p"),
            row("Replaces", "r"),
            plain("One."),
            plain("Two."),
            row("Special", "s"),
            plain("The note continues."),
        ];
        assert_eq!(group(&ok, 1), Ok(9..11));
        // The paragraphs that continue `Special` are not an effect group.
        assert_eq!(
            group(
                &[
                    name("P"),
                    row("Action", "Ability"),
                    row("Special", "s"),
                    plain("The note.")
                ],
                1
            ),
            Err(ActionGroupProblem::NoGroup)
        );
        // No group.
        assert_eq!(
            group(&[name("P"), row("Action", "Ability")], 1),
            Err(ActionGroupProblem::NoGroup)
        );
        // A group before the row.
        assert_eq!(
            group(&[name("P"), plain("Before."), row("Action", "Ability")], 2),
            Err(ActionGroupProblem::GroupBeforeRow)
        );
        // A second group.
        assert_eq!(
            group(
                &[
                    name("P"),
                    row("Action", "Ability"),
                    plain("One."),
                    row("Usage", "u"),
                    plain("Two."),
                ],
                1
            ),
            Err(ActionGroupProblem::SecondGroup)
        );
        // A lead, a legacy upgrade, an empty row or another row in between.
        for (between, what) in [
            (lead("3rd", "x"), "an upgrade lead"),
            (plain("At 3rd level: x"), "a plain-text upgrade"),
            (paragraph(vec![strong("Usage:")]), "the empty row `Usage:`"),
            (row("Penalty", "s"), "the row `Penalty:`"),
        ] {
            assert_eq!(
                group(
                    &[name("P"), row("Action", "Ability"), between, plain("One.")],
                    1
                ),
                Err(ActionGroupProblem::Crosses { what: what.into() })
            );
        }
    }

    #[test]
    fn an_ability_tag_is_ex_su_or_sp_alone_or_combined() {
        for (name, base, tag) in [
            ("Resistance (Ex)", "Resistance", Some("(Ex)")),
            ("Hex (Su)", "Hex", Some("(Su)")),
            ("Spell (Sp)", "Spell", Some("(Sp)")),
            ("Gaze (Ex or Su)", "Gaze", Some("(Ex or Su)")),
            ("Gaze (Sp and Ex)", "Gaze", Some("(Sp and Ex)")),
            ("Gaze (Ex, Su)", "Gaze", Some("(Ex, Su)")),
            ("Gaze (Ex, Su, or Sp)", "Gaze", Some("(Ex, Su, or Sp)")),
            ("Plain Name", "Plain Name", None),
            ("Name (Special)", "Name (Special)", None),
            ("Name (ex)", "Name (ex)", None),
            ("Name (Ex) trailing", "Name (Ex) trailing", None),
            ("Name ()", "Name ()", None),
            // A literal pipe is no separator: the suffix stays in the name.
            ("Gaze (Ex|Su)", "Gaze (Ex|Su)", None),
            ("Gaze (Ex||Su)", "Gaze (Ex||Su)", None),
            ("Gaze (Ex | Su)", "Gaze (Ex | Su)", None),
            ("Gaze (Ex or)", "Gaze (Ex or)", None),
            ("Gaze (Ex, )", "Gaze (Ex, )", None),
            ("Gaze (Exx)", "Gaze (Exx)", None),
        ] {
            assert_eq!(split_ability_tag(name), (base, tag), "{name}");
        }
    }

    #[test]
    fn a_one_power_page_matches_its_title_and_starts_with_its_power() {
        let canonical = |head: &str| {
            vec![
                name(head),
                row("Gained", "At 1st Level"),
                row("Passive", "x"),
            ]
        };
        assert!(one_power_page("Resistance", &canonical("Resistance (Ex)")).is_some());
        assert!(one_power_page("RESISTANCE", &canonical("Resistance")).is_some());
        // Apostrophe style and case are ignored.
        assert!(one_power_page("Witch\u{2019}s Brew", &canonical("witch's brew (Su)")).is_some());
        assert!(
            one_power_page(
                "Witch's Brew",
                &canonical("Witch\u{2019}s Brew (Sp and Ex)")
            )
            .is_some()
        );
        // The name differs from the title.
        assert!(one_power_page("Resistances", &canonical("Resistance (Ex)")).is_none());
        // The power is not the first block.
        let mut late = vec![plain("Intro.")];
        late.extend(canonical("Resistance (Ex)"));
        assert!(one_power_page("Resistance", &late).is_none());
        // Two candidates.
        let mut two = canonical("Resistance (Ex)");
        two.extend(canonical("Other (Ex)"));
        assert!(one_power_page("Resistance", &two).is_none());
        // The candidate fails validation.
        let failing = vec![name("Resistance (Ex)"), row("Gained", "At 1st Level")];
        assert!(one_power_page("Resistance", &failing).is_none());
    }

    #[test]
    fn a_heading_level_is_one_below_the_preceding_top_level_heading_at_most_six() {
        let heading = |level| Block::Heading {
            level,
            anchor: None,
            content: vec![text("H")],
        };
        let body = vec![
            name("Zero"),
            heading(2),
            name("One"),
            heading(3),
            Block::Quote {
                content: vec![heading(5)],
            },
            name("Two"),
            heading(6),
            name("Three"),
        ];
        assert_eq!(power_heading_level(&body, 0), 2);
        assert_eq!(power_heading_level(&body, 2), 3);
        assert_eq!(power_heading_level(&body, 5), 4);
        assert_eq!(power_heading_level(&body, 7), 6);
    }

    #[test]
    fn a_span_must_be_what_candidates_finds() {
        // The legacy name `**Accurate Strike (Ex):**` is an empty row, not a name: a rewrite that
        // validates its replacement directly cannot get a power with an empty name out of it.
        let legacy = vec![
            paragraph(vec![strong("Accurate Strike (Ex):")]),
            row("Swift", "The magus can expend 2 points."),
            row("Prereq", "9th-level magus"),
        ];
        assert!(candidates(&legacy).is_empty());
        let problems = validate_power(&legacy).unwrap_err();
        assert_eq!(problems, [PowerProblem::NotAName]);
        // Neither a first block that is a row, a lead or a paragraph.
        for first in [row("Passive", "x"), lead("3rd", "x"), plain("Prose.")] {
            let span = vec![first, row("Passive", "y")];
            assert!(
                validate_power(&span)
                    .unwrap_err()
                    .contains(&PowerProblem::NotAName)
            );
        }
        assert_eq!(
            validate_power(&[]).unwrap_err(),
            [
                PowerProblem::NotAName,
                PowerProblem::NoEffectRow {
                    suggest_lowest_upgrade: false
                }
            ]
        );
        // A boundary or a second name inside the span is not one power's run.
        let heading = Block::Heading {
            level: 3,
            anchor: None,
            content: vec![text("School")],
        };
        for inner in [heading, name("Other")] {
            let span = vec![name("P"), row("Passive", "x"), inner, row("Usage", "u")];
            assert_eq!(
                validate_power(&span).unwrap_err(),
                [PowerProblem::BlockInSpan]
            );
        }
        // What `candidates` finds is still accepted as before.
        let ok = vec![name("P"), row("Passive", "x")];
        assert!(validate_power(&ok).is_ok());
    }

    #[test]
    fn a_gained_action_value_is_existing_gained_metadata() {
        // Fascinate: `Action: At 1st Level` already says the level, so no suggestion.
        let body = vec![name("P"), row("Action", "At 1st Level"), lead("3rd", "x")];
        let list = candidates(&body);
        let problems = validate_power(list[0].blocks(&body)).unwrap_err();
        assert!(problems.contains(&PowerProblem::NoEffectRow {
            suggest_lowest_upgrade: false
        }));
        // An `Action` row that names an action is not Gained metadata.
        let body = vec![name("P"), row("Action", "Ability"), lead("3rd", "x")];
        let list = candidates(&body);
        let problems = validate_power(list[0].blocks(&body)).unwrap_err();
        assert!(problems.contains(&PowerProblem::NoEffectRow {
            suggest_lowest_upgrade: true
        }));
    }

    #[test]
    fn the_action_group_needs_an_effect_action_row_at_the_index() {
        let blocks = vec![
            name("P"),
            row("Action", "Ability"),
            row("Gained", "At 1st Level"),
            plain("One."),
            plain("Two."),
        ];
        assert_eq!(action_effect_group(&blocks, 1), Ok(3..5));
        // The first paragraph of the group, the name, a row that is no `Action` row, and an index
        // past the end: an error, never a panic.
        for index in [3, 4, 0, 2, 5, 99] {
            assert_eq!(
                action_effect_group(&blocks, index),
                Err(ActionGroupProblem::NotAnEffectAction),
                "{index}"
            );
        }
        // An `Action` value that is Gained metadata or not in the map takes no effect text.
        for value in ["At 1st Level", "Move-Equivalent Action"] {
            let blocks = vec![name("P"), row("Action", value), plain("One.")];
            assert_eq!(
                action_effect_group(&blocks, 1),
                Err(ActionGroupProblem::NotAnEffectAction),
                "{value}"
            );
        }
    }

    #[test]
    fn the_result_does_not_depend_on_anything_but_the_blocks() {
        let body = vec![
            name("P"),
            row("Gained", "At 1st Level"),
            row("Passive", "x"),
            lead("3rd", "a"),
        ];
        assert_eq!(candidates(&body), candidates(&body.clone()));
        let found = candidates(&body);
        assert_eq!(
            validate_power(found[0].blocks(&body)),
            validate_power(found[0].blocks(&body))
        );
    }
}
