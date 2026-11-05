//! Body-link enrichment for the canonical library.
//!
//! The Pf Data conversion links most cross-references, but it leaves a residue: spell and
//! special-ability names that the book sets in italic or bold and that no link covers
//! (`scripts/measure-reference-links.py` counts them). This pass links that residue. It is the
//! one-time linking step for the planned curated corpus, so it is a self-contained pure
//! function: [`link_entries`] takes entries and returns entries plus a [`Report`]. It reads no
//! file and keeps no state between calls.
//!
//! The rules, in the order that the pass applies them to each text span:
//!
//! 1. **Blocklist.** A [`BLOCKLIST`] term is never linked automatically.
//! 2. **Exact title.** An italic or bold span whose whole text equals the title of a spell or
//!    special-ability entry, ignoring case, runs of whitespace, and trailing punctuation,
//!    links to that entry. Plain, non-emphasised text is never linked automatically.
//!    A spell wins over a special ability of the same title, because the books set spell
//!    names in italic. Among the rest, the Core Rulebook wins, then the earliest
//!    publication. A tie that remains stays unlinked and is reported.
//! 3. **Curated terms.** A [`TERMS`] row names plain text that links to one fixed target.
//!    Plain text links only at its first occurrence in a block, which avoids a wall of
//!    links. An italic or bold span that is exactly a curated term and names no spell links
//!    to the curated target, because that target settles a title with many copies. A span
//!    that also names a spell links to the spell (rule 2).
//!
//! A span is left alone, and counted, when it is already inside a link, inside a heading, a
//! whole-paragraph label, or a header cell, or when it names the entry that holds it. A
//! curated plain-text match that sits in a heading, a label, a link, or emphasised text is
//! counted too. The pass never changes a word: it only wraps existing text in a link, and
//! the wrapped text keeps its original case and spelling.

use cast_iron_schema::{
    Block, EntryId, EntryKind, EntryReference, Inline, RuleEntry, StatRow, Table, TableCell,
    TableRow,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The source id that wins an ambiguous title.
const CORE_SOURCE: &str = "core-rulebook";

/// Terms that are never linked automatically, each with the reason. The pass counts every
/// blocked span that would otherwise have matched a target title.
///
/// A curated term overrides this list for emphasised spans, so a term must not appear in
/// both; a unit test enforces that.
pub const BLOCKLIST: &[(&str, &str)] = &[
    (
        "poison",
        "A spell and a Core rule share the title, and the spell would win, but emphasised `poison` usually names the poison rules or the word itself.",
    ),
    (
        "fear",
        "A spell and a Core rule share the title, and the spell would win, but emphasised `fear` usually names the fear rules or the condition family.",
    ),
    (
        "invisibility",
        "A spell and a Core rule share the title, and the spell would win, but emphasised `invisibility` often names the condition or the rules.",
    ),
    (
        "resistance",
        "Resolves to the Core spell, but italic or bold `resistance` usually names the general concept (energy resistance, spell resistance), not the spell.",
    ),
    (
        "web",
        "Resolves to the Core spell, but bold `web` also labels the web attack of a creature, and the type does not say which one the text means.",
    ),
    (
        "spells",
        "A bold run-in label (`Spells:` in an archetype's spell list); the only target is a Bestiary 5 monster rule, which is not what the label means.",
    ),
    (
        "burn",
        "The kineticist's burn mechanic is set in bold and italic; the only target is the Bestiary monster rule of the same name.",
    ),
    (
        "rage",
        "Barbarian text sets the rage class feature in italic; a link would point to the unrelated Rage spell.",
    ),
    (
        "bleed",
        "Bold `Bleed` in a creature's attack line is the bleed damage ability, and a link would point to the Bleed spell.",
    ),
    (
        "special abilities",
        "A bold section label for the Special Abilities heading of a stat block, not a reference.",
    ),
];

/// A plain-text term that links to one fixed entry.
#[derive(Debug, Clone, Copy)]
pub struct Term {
    /// The text to match, ASCII lower case, matched at word boundaries and case-insensitively.
    pub text: &'static str,
    /// The permanent id of the target entry.
    pub target: &'static str,
    pub reason: &'static str,
}

/// Curated terms. These are the high-frequency terms of the measured residue whose meaning
/// does not depend on the entry that holds them. `water breathing` is left out on purpose:
/// it names both a Core spell and a Bestiary monster ability, and the plain phrase means
/// the spell in item and spell text but the ability in creature text.
pub const TERMS: &[Term] = &[
    Term {
        text: "spell resistance",
        target: "ref/rule/spell-resistance",
        reason: "The Core Rulebook's Magic chapter rule; the Combat chapter, Ultimate Magic, and Bestiary copies make the title ambiguous.",
    },
    Term {
        text: "spell-like abilities",
        target: "ref/umr/spell-like-abilities",
        reason: "The universal monster rule is the full definition; the Core glossary entry is a one-sentence stub.",
    },
    Term {
        text: "spell-like ability",
        target: "ref/umr/spell-like-abilities",
        reason: "Singular form of the same rule.",
    },
    Term {
        text: "hardness",
        target: "ref/umr/hardness",
        reason: "The only entry titled Hardness.",
    },
    Term {
        text: "damage reduction",
        target: "ref/rule/damage-reduction",
        reason: "The Core Rulebook's special-abilities glossary rule; evolutions and the Bestiary repeat the title.",
    },
    Term {
        text: "darkvision",
        target: "ref/rule/darkvision",
        reason: "The Core Rulebook rule; the Darkvision spell and several racial traits share the title.",
    },
    Term {
        text: "low-light vision",
        target: "ref/rule/low-light-vision",
        reason: "The Core Rulebook rule; racial traits and evolutions share the title.",
    },
    Term {
        text: "fast healing",
        target: "ref/umr/fast-healing",
        reason: "The universal monster rule; no Core entry has the title.",
    },
    Term {
        text: "blindsight",
        target: "ref/rule/blindsight-and-blindsense",
        reason: "The Core Rulebook rule that covers both senses.",
    },
    Term {
        text: "blindsense",
        target: "ref/rule/blindsight-and-blindsense",
        reason: "The Core Rulebook rule that covers both senses.",
    },
    Term {
        text: "tremorsense",
        target: "ref/umr/tremorsense",
        reason: "The universal monster rule; no Core entry has the title.",
    },
    Term {
        text: "channel resistance",
        target: "ref/rule/channel-resistance",
        reason: "The Core Rulebook rule; the Bestiary copies share the title.",
    },
];

/// Records of `json/rules_combat01.json` that are the Core "Special Abilities" glossary. They
/// are separate entries with no parent link, so a record list is the only marker.
const CORE_SPECIAL_ABILITY_RECORDS: &[&str] = &[
    "ability_damage_drain",
    "ability_score_bonuses",
    "afflictions",
    "blindsight_and_blindsense",
    "channel_resistance",
    "charm_and_compulsion",
    "curses",
    "damage_reduction",
    "darkvision",
    "death_attacks",
    "diseases",
    "energy_drain_and_negative_levels",
    "energy_immunity_and_vulnerability",
    "fear",
    "infestations",
    "invisibility",
    "low_light_vision",
    "paralysis",
    "poison",
    "scent",
    "special_abilities_2",
    "spell_resistance_2",
    "successful_spell_resistance",
    "when_spell_resistance_applies",
];

const UMR_PATH: &str = "json/umr.json";
const CORE_GLOSSARY_PATH: &str = "json/rules_combat01.json";
const CARRIED_UMR_SECTION: &str = "Universal Monster Rules";

/// What a link target is, for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Category {
    Spell,
    SpecialAbility,
}

impl Category {
    fn name(self) -> &'static str {
        match self {
            Category::Spell => "spell",
            Category::SpecialAbility => "special_ability",
        }
    }
}

/// The category of an entry as a link target, if it has one.
fn category_of(entry: &RuleEntry) -> Option<Category> {
    match entry.kind {
        EntryKind::Spell => Some(Category::Spell),
        EntryKind::Rule => {
            let umr = entry.provenance.iter().any(|p| p.path == UMR_PATH)
                || (entry.id.as_str().starts_with("ref/carried/")
                    && entry.section.iter().any(|s| s == CARRIED_UMR_SECTION));
            let glossary = entry.provenance.iter().any(|p| {
                p.path == CORE_GLOSSARY_PATH
                    && CORE_SPECIAL_ABILITY_RECORDS.contains(&p.record.as_str())
            });
            (umr || glossary).then_some(Category::SpecialAbility)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// Report

/// Everything that the pass did and everything that it left alone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Entries whose body gained at least one link.
    pub entries_changed: u32,
    pub links_added: u32,
    /// Counts by the category of the link target: `spell`, `special_ability`, or `other` for a
    /// curated target outside both. Skipped counts use `mixed` when the candidates span
    /// categories.
    pub categories: BTreeMap<String, CategoryCounts>,
    /// Hits per rule: `auto:title`, `auto:title+trailing-punctuation`, and `term:<text>`.
    pub rules: BTreeMap<String, u32>,
    /// Matching spans that the pass did not touch, by reason.
    pub skipped: Skipped,
    /// Blocked terms and how often each one stopped a link.
    pub blocked: BTreeMap<String, BlockedRow>,
    /// Titles that name several entries and could not be resolved.
    pub ambiguous: BTreeMap<String, AmbiguousRow>,
    /// Per curated term.
    pub curated: BTreeMap<String, CuratedRow>,
    /// Curated terms whose target is not in the input. Their matches are left alone.
    pub missing_targets: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CategoryCounts {
    pub added: u32,
    pub skipped_ambiguous: u32,
    pub skipped_blocked: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// Matching spans that sit inside a link already.
    pub in_link: u32,
    pub heading: u32,
    /// A paragraph that is one italic or bold span, or a header cell: a label, not a reference.
    pub label: u32,
    /// The span names the entry that holds it.
    pub self_title: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BlockedRow {
    pub spans: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AmbiguousRow {
    pub spans: u32,
    pub category: String,
    /// The entries tied for first place after the Core and publication-date preference.
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CuratedRow {
    pub target: String,
    /// Links added to italic or bold spans.
    pub emphasised: u32,
    /// Links added to plain text, one per block.
    pub plain: u32,
    /// Later plain matches in a block that already linked the target.
    pub skipped_repeat: u32,
    /// Matches inside the target entry or an entry with the term as its title.
    pub skipped_self: u32,
    /// Emphasised spans that name a spell as well, and so link to the spell, not to the
    /// curated target.
    pub emphasised_to_spell: u32,
    /// Plain-text matches that the pass did not link, by where they sit.
    pub skipped_in_heading: u32,
    pub skipped_in_label: u32,
    pub skipped_in_link: u32,
    pub skipped_in_emphasis: u32,
}

/// The pass's output.
#[derive(Debug, Clone, PartialEq)]
pub struct Linked {
    pub entries: Vec<RuleEntry>,
    pub report: Report,
}

// ---------------------------------------------------------------------------------------------
// Index

#[derive(Debug, Clone)]
struct Candidate {
    id: EntryId,
    category: Category,
    core: bool,
    /// Earliest known publication date of any source, ISO 8601.
    date: Option<String>,
}

impl Candidate {
    /// Lower sorts first: Core Rulebook, then the earliest date, with unknown dates last.
    fn rank(&self) -> (u8, &str) {
        (
            u8::from(!self.core),
            self.date.as_deref().unwrap_or("9999-99-99"),
        )
    }
}

enum Resolution {
    Linked(EntryId),
    Ambiguous { tied: Vec<String>, category: String },
}

/// Lower case, with typographic apostrophes straightened and whitespace runs collapsed.
fn normalise(text: &str) -> String {
    text.replace('\u{2019}', "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Splits a span into leading whitespace, the match text, and trailing whitespace and
/// punctuation.
fn split_edges(text: &str) -> (&str, &str, &str) {
    let rest = text.trim_start();
    let lead = &text[..text.len() - rest.len()];
    let core = rest.trim_end_matches(|c: char| c.is_whitespace() || ",;:.!?".contains(c));
    (lead, core, &rest[core.len()..])
}

fn category_label(categories: &BTreeSet<Category>) -> String {
    match categories.len() {
        1 => categories
            .iter()
            .next()
            .map_or("mixed", |c| c.name())
            .to_string(),
        _ => "mixed".to_string(),
    }
}

/// True when `needle` at `start` in `haystack` sits on word boundaries.
fn on_boundaries(haystack: &str, start: usize, len: usize) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '-';
    let before = haystack[..start].chars().next_back();
    let after = haystack[start + len..].chars().next();
    !before.is_some_and(word) && !after.is_some_and(word)
}

/// The first whole-word occurrence of `needle` at or after `from`.
fn find_word(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let mut at = from;
    while let Some(found) = haystack.get(at..)?.find(needle) {
        let start = at + found;
        if on_boundaries(haystack, start, needle.len()) {
            return Some(start);
        }
        at = start + 1;
    }
    None
}

struct Pass {
    titles: BTreeMap<String, Vec<Candidate>>,
    terms: BTreeMap<String, &'static Term>,
    /// Normalised term texts per target, to skip an entry that is titled like the term.
    target_texts: BTreeMap<&'static str, BTreeSet<String>>,
    blocked: BTreeMap<String, &'static str>,
    /// Category of every target entry.
    categories: BTreeMap<EntryId, Category>,
    report: Report,
    /// Targets added to the entry that is being processed.
    added: Vec<EntryId>,
}

/// What the pass knows about the entry that holds the span.
struct Holder {
    id: EntryId,
    title_key: String,
}

/// Where a plain-text match sits when the pass cannot link it.
#[derive(Clone, Copy)]
enum Place {
    Heading,
    Label,
    Link,
    Emphasis,
}

#[derive(Clone, Copy, Default)]
struct Context {
    in_link: bool,
    heading: bool,
    label: bool,
}

impl Pass {
    fn new(entries: &[RuleEntry], ranks: &BTreeMap<String, String>) -> Self {
        let mut titles: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
        let mut ids: BTreeSet<&EntryId> = BTreeSet::new();
        let mut categories = BTreeMap::new();
        for entry in entries {
            ids.insert(&entry.id);
            let Some(category) = category_of(entry) else {
                continue;
            };
            categories.insert(entry.id.clone(), category);
            let date = entry
                .source_ids
                .iter()
                .filter_map(|source| ranks.get(source))
                .min()
                .cloned();
            titles
                .entry(normalise(&entry.title))
                .or_default()
                .push(Candidate {
                    id: entry.id.clone(),
                    category,
                    core: entry.source_ids.iter().any(|s| s == CORE_SOURCE),
                    date,
                });
        }
        for candidates in titles.values_mut() {
            candidates.sort_by(|a, b| a.id.cmp(&b.id));
        }

        let mut terms = BTreeMap::new();
        let mut target_texts: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
        let mut report = Report::default();
        for term in TERMS {
            target_texts
                .entry(term.target)
                .or_default()
                .insert(normalise(term.text));
            // A term whose target is not in the input would link to nothing. The report
            // names it, and its matches stay as they are.
            let present = EntryId::parse(term.target).is_ok_and(|id| ids.contains(&id));
            if present {
                terms.insert(normalise(term.text), term);
            } else {
                report
                    .missing_targets
                    .push(format!("{} -> {}", term.text, term.target));
            }
        }

        let blocked = BLOCKLIST
            .iter()
            .map(|(text, reason)| (normalise(text), *reason))
            .collect();
        Pass {
            titles,
            terms,
            target_texts,
            blocked,
            categories,
            report,
            added: Vec::new(),
        }
    }

    /// Picks the target of a title. A spell wins over a special ability of the same title,
    /// because the books set spell names in italic. Among the remaining candidates the Core
    /// Rulebook, then the earliest publication, wins. The category of an ambiguous title is
    /// the category of the entries tied for first place, not of every entry with the title.
    fn resolve(&self, candidates: &[Candidate]) -> Resolution {
        let spells: Vec<&Candidate> = candidates
            .iter()
            .filter(|c| c.category == Category::Spell)
            .collect();
        let pool: Vec<&Candidate> = if spells.is_empty() {
            candidates.iter().collect()
        } else {
            spells
        };
        let best = pool.iter().map(|c| c.rank()).min();
        let tied: Vec<&Candidate> = pool
            .into_iter()
            .filter(|c| Some(c.rank()) == best)
            .collect();
        match tied.as_slice() {
            [only] => Resolution::Linked(only.id.clone()),
            _ => {
                let categories: BTreeSet<Category> = tied.iter().map(|c| c.category).collect();
                Resolution::Ambiguous {
                    tied: tied.iter().map(|c| c.id.to_string()).collect(),
                    category: category_label(&categories),
                }
            }
        }
    }

    /// True when `holder` is the term's target, or is titled like any term for that target.
    fn is_self_for(&self, holder: &Holder, term: &Term, target: &EntryId) -> bool {
        &holder.id == target
            || self
                .target_texts
                .get(term.target)
                .is_some_and(|texts| texts.contains(&holder.title_key))
    }

    fn count_added(&mut self, target: &EntryId, category: &str, rule: String) {
        self.report.links_added += 1;
        self.report
            .categories
            .entry(category.into())
            .or_default()
            .added += 1;
        *self.report.rules.entry(rule).or_default() += 1;
        self.added.push(target.clone());
    }

    fn category_of_target(&self, target: &EntryId) -> &'static str {
        self.categories.get(target).map_or("other", |c| c.name())
    }

    // -----------------------------------------------------------------------------------------
    // Blocks

    fn blocks(&mut self, blocks: Vec<Block>, holder: &Holder) -> Vec<Block> {
        blocks
            .into_iter()
            .map(|block| match block {
                Block::Heading {
                    level,
                    anchor,
                    content,
                } => Block::Heading {
                    level,
                    anchor,
                    content: self.run(
                        content,
                        holder,
                        Context {
                            heading: true,
                            ..Context::default()
                        },
                    ),
                },
                Block::Paragraph { style, content } => {
                    let label = content.len() == 1
                        && matches!(content[0], Inline::Em { .. } | Inline::Strong { .. });
                    Block::Paragraph {
                        style,
                        content: self.run(
                            content,
                            holder,
                            Context {
                                label,
                                ..Context::default()
                            },
                        ),
                    }
                }
                Block::List { ordered, items } => Block::List {
                    ordered,
                    items: items
                        .into_iter()
                        .map(|item| self.blocks(item, holder))
                        .collect(),
                },
                Block::Table(table) => Block::Table(self.table(table, holder)),
                Block::Quote { content } => Block::Quote {
                    content: self.blocks(content, holder),
                },
                Block::StatBlock { title, rows } => Block::StatBlock {
                    title,
                    rows: rows
                        .into_iter()
                        .map(|row| StatRow {
                            label: row.label,
                            value: self.run(row.value, holder, Context::default()),
                        })
                        .collect(),
                },
                entry_link @ Block::EntryLink { .. } => entry_link,
            })
            .collect()
    }

    fn table(&mut self, table: Table, holder: &Holder) -> Table {
        let rows = |rows: Vec<TableRow>, this: &mut Self| -> Vec<TableRow> {
            rows.into_iter()
                .map(|row| TableRow {
                    cells: row
                        .cells
                        .into_iter()
                        .map(|cell| TableCell {
                            content: this.run(
                                cell.content,
                                holder,
                                Context {
                                    label: cell.header,
                                    ..Context::default()
                                },
                            ),
                            ..cell
                        })
                        .collect(),
                })
                .collect()
        };
        Table {
            caption: table.caption,
            head: rows(table.head, self),
            body: rows(table.body, self),
            foot: rows(table.foot, self),
        }
    }

    /// One block of inline content: emphasised spans first, then curated plain text.
    fn run(&mut self, content: Vec<Inline>, holder: &Holder, context: Context) -> Vec<Inline> {
        let mut linked = BTreeSet::new();
        existing_targets(&content, &mut linked);
        // Plain-text matches that this block cannot link are counted from the input, before
        // the span pass adds links of its own.
        let place = if context.heading {
            Some(Place::Heading)
        } else if context.label {
            Some(Place::Label)
        } else {
            None
        };
        self.count_skipped_plain(&content, place);
        let content = self.spans(content, holder, context, &mut linked);
        if context.heading || context.label {
            return content;
        }
        self.plain(content, holder, &mut linked)
    }

    // -----------------------------------------------------------------------------------------
    // Emphasised spans

    fn spans(
        &mut self,
        inlines: Vec<Inline>,
        holder: &Holder,
        context: Context,
        linked: &mut BTreeSet<EntryId>,
    ) -> Vec<Inline> {
        inlines
            .into_iter()
            .map(|node| match node {
                Inline::Link { target, anchor, c } => Inline::Link {
                    target,
                    anchor,
                    c: self.spans(
                        c,
                        holder,
                        Context {
                            in_link: true,
                            ..context
                        },
                        linked,
                    ),
                },
                Inline::Em { c } => match self.span(&c, holder, context, linked) {
                    Some(c) => Inline::Em { c },
                    None => Inline::Em {
                        c: self.spans(c, holder, context, linked),
                    },
                },
                Inline::Strong { c } => match self.span(&c, holder, context, linked) {
                    Some(c) => Inline::Strong { c },
                    None => Inline::Strong {
                        c: self.spans(c, holder, context, linked),
                    },
                },
                Inline::Sup { c } => Inline::Sup {
                    c: self.spans(c, holder, context, linked),
                },
                Inline::Sub { c } => Inline::Sub {
                    c: self.spans(c, holder, context, linked),
                },
                other => other,
            })
            .collect()
    }

    /// Decides one italic or bold span that holds only text. Returns the replacement content
    /// when the span becomes a link.
    fn span(
        &mut self,
        content: &[Inline],
        holder: &Holder,
        context: Context,
        linked: &mut BTreeSet<EntryId>,
    ) -> Option<Vec<Inline>> {
        let mut text = String::new();
        for node in content {
            match node {
                Inline::Text { v } => text.push_str(v),
                _ => return None,
            }
        }
        let (lead, core, trail) = split_edges(&text);
        let key = normalise(core);
        if key.is_empty() {
            return None;
        }
        let curated = self.terms.get(&key).copied();
        let candidates = self.titles.get(&key).cloned();
        if curated.is_none() && candidates.is_none() {
            return None;
        }

        if context.in_link {
            self.report.skipped.in_link += 1;
            return None;
        }
        if context.heading {
            self.report.skipped.heading += 1;
            return None;
        }
        if context.label {
            self.report.skipped.label += 1;
            return None;
        }
        if key == holder.title_key {
            self.report.skipped.self_title += 1;
            return None;
        }

        // A span that names a spell links to the spell: italic is how the books mark spell
        // names. The curated target covers the other emphasised spans.
        let names_spell = candidates
            .as_ref()
            .is_some_and(|all| all.iter().any(|c| c.category == Category::Spell));
        let (target, category, rule) = if let Some(term) = curated.filter(|_| !names_spell) {
            let target = EntryId::parse(term.target).ok()?;
            let is_self = self.is_self_for(holder, term, &target);
            let row = self.report.curated.entry(term.text.into()).or_default();
            row.target = term.target.into();
            if is_self {
                row.skipped_self += 1;
                return None;
            }
            row.emphasised += 1;
            let category = self.category_of_target(&target);
            (target, category, format!("term:{}", term.text))
        } else {
            let candidates = candidates?;
            if let Some(reason) = self.blocked.get(&key) {
                let categories: BTreeSet<Category> =
                    candidates.iter().map(|c| c.category).collect();
                let row = self.report.blocked.entry(key.clone()).or_default();
                row.spans += 1;
                row.reason = (*reason).to_string();
                self.report
                    .categories
                    .entry(category_label(&categories))
                    .or_default()
                    .skipped_blocked += 1;
                return None;
            }
            match self.resolve(&candidates) {
                Resolution::Ambiguous { tied, category } => {
                    let row = self.report.ambiguous.entry(key).or_default();
                    row.spans += 1;
                    row.category.clone_from(&category);
                    row.candidates = tied;
                    self.report
                        .categories
                        .entry(category)
                        .or_default()
                        .skipped_ambiguous += 1;
                    return None;
                }
                Resolution::Linked(id) if id == holder.id => {
                    self.report.skipped.self_title += 1;
                    return None;
                }
                Resolution::Linked(id) => {
                    if let Some(term) = curated {
                        self.report
                            .curated
                            .entry(term.text.into())
                            .or_default()
                            .emphasised_to_spell += 1;
                    }
                    let category = self.category_of_target(&id);
                    let rule = if trail.trim().is_empty() {
                        "auto:title"
                    } else {
                        "auto:title+trailing-punctuation"
                    };
                    (id, category, rule.to_string())
                }
            }
        };

        self.count_added(&target, category, rule);
        linked.insert(target.clone());
        let mut out = Vec::new();
        if !lead.is_empty() {
            out.push(Inline::text(lead));
        }
        out.push(Inline::Link {
            target,
            anchor: None,
            c: vec![Inline::text(core)],
        });
        if !trail.is_empty() {
            out.push(Inline::text(trail));
        }
        Some(out)
    }

    // -----------------------------------------------------------------------------------------
    // Curated plain text

    fn plain(
        &mut self,
        inlines: Vec<Inline>,
        holder: &Holder,
        linked: &mut BTreeSet<EntryId>,
    ) -> Vec<Inline> {
        if self.terms.is_empty() {
            return inlines;
        }
        let mut out = Vec::with_capacity(inlines.len());
        for node in inlines {
            match node {
                Inline::Text { v } => out.extend(self.text(v, holder, linked)),
                other => out.push(other),
            }
        }
        out
    }

    /// The earliest curated term at or after `pos`; at one position the longer term wins.
    fn next_match(&self, lower: &str, pos: usize) -> Option<(usize, &'static Term)> {
        let mut best: Option<(usize, &'static Term)> = None;
        for term in self.terms.values().copied() {
            if let Some(start) = find_word(lower, term.text, pos)
                && best.is_none_or(|(at, held)| {
                    start < at || (start == at && term.text.len() > held.text.len())
                })
            {
                best = Some((start, term));
            }
        }
        best
    }

    /// Counts curated plain-text matches that the pass leaves alone because of where they
    /// sit: in a heading, in a label, inside a link, or inside emphasised text that is not
    /// itself a curated term. `place` is where the walk starts; `None` counts nothing at the
    /// top level, because the plain pass handles that text.
    fn count_skipped_plain(&mut self, inlines: &[Inline], place: Option<Place>) {
        for node in inlines {
            match node {
                Inline::Text { v } => {
                    if let Some(place) = place {
                        self.count_text(v, place);
                    }
                }
                Inline::Link { c, .. } => {
                    self.count_skipped_plain(c, place.or(Some(Place::Link)));
                }
                Inline::Em { c } | Inline::Strong { c } => {
                    // A span that is exactly a curated term is counted by the span pass.
                    let whole: Option<String> = c
                        .iter()
                        .map(|n| match n {
                            Inline::Text { v } => Some(v.as_str()),
                            _ => None,
                        })
                        .collect();
                    if let Some(text) = whole
                        && self.terms.contains_key(&normalise(split_edges(&text).1))
                    {
                        continue;
                    }
                    self.count_skipped_plain(c, place.or(Some(Place::Emphasis)));
                }
                Inline::Sup { c } | Inline::Sub { c } => self.count_skipped_plain(c, place),
                Inline::Break => {}
            }
        }
    }

    fn count_text(&mut self, v: &str, place: Place) {
        let lower = v.to_ascii_lowercase();
        let mut pos = 0;
        while let Some((start, term)) = self.next_match(&lower, pos) {
            pos = start + term.text.len();
            let row = self.report.curated.entry(term.text.into()).or_default();
            row.target = term.target.into();
            match place {
                Place::Heading => row.skipped_in_heading += 1,
                Place::Label => row.skipped_in_label += 1,
                Place::Link => row.skipped_in_link += 1,
                Place::Emphasis => row.skipped_in_emphasis += 1,
            }
        }
    }

    fn text(&mut self, v: String, holder: &Holder, linked: &mut BTreeSet<EntryId>) -> Vec<Inline> {
        let lower = v.to_ascii_lowercase();
        let mut out = Vec::new();
        let (mut pos, mut last) = (0, 0);
        while pos < v.len() {
            let Some((start, term)) = self.next_match(&lower, pos) else {
                break;
            };
            let end = start + term.text.len();
            pos = end;
            let Ok(target) = EntryId::parse(term.target) else {
                continue;
            };
            let is_self = self.is_self_for(holder, term, &target);
            let row = self.report.curated.entry(term.text.into()).or_default();
            row.target = term.target.into();
            if is_self {
                row.skipped_self += 1;
                continue;
            }
            if linked.contains(&target) {
                row.skipped_repeat += 1;
                continue;
            }
            row.plain += 1;
            if start > last {
                out.push(Inline::text(&v[last..start]));
            }
            out.push(Inline::Link {
                target: target.clone(),
                anchor: None,
                c: vec![Inline::text(&v[start..end])],
            });
            last = end;
            let category = self.category_of_target(&target);
            self.count_added(&target, category, format!("term:{}", term.text));
            linked.insert(target);
        }
        if last == 0 && out.is_empty() {
            return vec![Inline::Text { v }];
        }
        if last < v.len() {
            out.push(Inline::text(&v[last..]));
        }
        out
    }
}

/// Targets of the links that a block already holds.
fn existing_targets(inlines: &[Inline], out: &mut BTreeSet<EntryId>) {
    for node in inlines {
        match node {
            Inline::Link { target, c, .. } => {
                out.insert(target.clone());
                existing_targets(c, out);
            }
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                existing_targets(c, out);
            }
            _ => {}
        }
    }
}

/// Links the residue of one library.
///
/// `ranks` maps a publication id to its first release date (ISO 8601). It orders entries that
/// share a title when none is in the Core Rulebook; a publication without a date sorts last.
/// The result is the same for the same input, in the same order.
pub fn link_entries(entries: Vec<RuleEntry>, ranks: &BTreeMap<String, String>) -> Linked {
    let mut pass = Pass::new(&entries, ranks);
    let mut out = Vec::with_capacity(entries.len());
    for mut entry in entries {
        let holder = Holder {
            id: entry.id.clone(),
            title_key: normalise(&entry.title),
        };
        pass.added.clear();
        let body = std::mem::take(&mut entry.body);
        entry.body = pass.blocks(body, &holder);
        if !pass.added.is_empty() {
            pass.report.entries_changed += 1;
            entry
                .links
                .extend(pass.added.drain(..).map(|target| EntryReference {
                    target,
                    anchor: None,
                }));
            entry.links.sort();
            entry.links.dedup();
        }
        out.push(entry);
    }
    Linked {
        entries: out,
        report: pass.report,
    }
}

/// Renders the report as a Markdown section for `coverage.md`.
pub fn markdown(report: &Report) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "## Body-link pass\n");
    let _ = writeln!(
        s,
        "{} links added in {} entries.\n",
        report.links_added, report.entries_changed
    );
    let _ = writeln!(
        s,
        "| Category | Added | Skipped, ambiguous | Skipped, blocked |\n|---|---:|---:|---:|"
    );
    for (name, counts) in &report.categories {
        let _ = writeln!(
            s,
            "| {name} | {} | {} | {} |",
            counts.added, counts.skipped_ambiguous, counts.skipped_blocked
        );
    }
    let _ = writeln!(s, "\n| Rule | Hits |\n|---|---:|");
    for (rule, hits) in &report.rules {
        let _ = writeln!(s, "| `{rule}` | {hits} |");
    }
    let skipped = &report.skipped;
    let _ = writeln!(
        s,
        "\nMatching spans left alone: {} inside a link, {} in a heading, {} as a label, {} naming their own entry.",
        skipped.in_link, skipped.heading, skipped.label, skipped.self_title
    );
    let _ = writeln!(s, "\n### Blocked terms\n");
    if report.blocked.is_empty() {
        let _ = writeln!(s, "None matched.");
    }
    for (term, row) in &report.blocked {
        let _ = writeln!(s, "- `{term}`: {} spans. {}", row.spans, row.reason);
    }
    let _ = writeln!(s, "\n### Curated terms\n");
    let _ = writeln!(
        s,
        "| Term | Target | Emphasised | To spell | Plain | Repeats skipped | Self skipped | In heading | In label | In link | In emphasis |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for (term, row) in &report.curated {
        let _ = writeln!(
            s,
            "| {term} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.target,
            row.emphasised,
            row.emphasised_to_spell,
            row.plain,
            row.skipped_repeat,
            row.skipped_self,
            row.skipped_in_heading,
            row.skipped_in_label,
            row.skipped_in_link,
            row.skipped_in_emphasis
        );
    }
    for missing in &report.missing_targets {
        let _ = writeln!(s, "\nMissing curated target, term skipped: `{missing}`");
    }
    let _ = writeln!(
        s,
        "\n### Ambiguous titles ({})\n\nThe Core Rulebook, then the earliest publication, settles a title. These stay unlinked because entries remain tied.\n",
        report.ambiguous.len()
    );
    for (title, row) in &report.ambiguous {
        let _ = writeln!(
            s,
            "- `{title}` ({}): {} spans; tied: {}",
            row.category,
            row.spans,
            row.candidates.join(", ")
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{ParagraphStyle, Provenance, TypedFields};

    fn base(id: &str, kind: EntryKind, title: &str, sources: &[&str]) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: title.into(),
            source_ids: sources.iter().map(|s| (*s).to_string()).collect(),
            section: Vec::new(),
            parent: None,
            body: Vec::new(),
            fields: TypedFields::None,
            tags: Vec::new(),
            links: Vec::new(),
            provenance: Vec::new(),
            license_refs: Vec::new(),
            relations: Vec::new(),
        }
    }

    fn spell(id: &str, title: &str, sources: &[&str]) -> RuleEntry {
        base(id, EntryKind::Spell, title, sources)
    }

    /// A universal monster rule from the compiled Pf Data corpus.
    fn ability(id: &str, title: &str, sources: &[&str]) -> RuleEntry {
        let mut entry = base(id, EntryKind::Rule, title, sources);
        entry.provenance.push(Provenance {
            dataset: "pf-data".into(),
            record: title.to_lowercase(),
            path: UMR_PATH.into(),
        });
        entry
    }

    fn host(id: &str, title: &str, body: Vec<Block>) -> RuleEntry {
        let mut entry = base(id, EntryKind::Creature, title, &["bestiary"]);
        entry.body = body;
        entry
    }

    fn t(text: &str) -> Inline {
        Inline::text(text)
    }

    fn em(text: &str) -> Inline {
        Inline::Em { c: vec![t(text)] }
    }

    fn strong(text: &str) -> Inline {
        Inline::Strong { c: vec![t(text)] }
    }

    fn link(target: &str, text: &str) -> Inline {
        Inline::Link {
            target: EntryId::parse(target).unwrap(),
            anchor: None,
            c: vec![t(text)],
        }
    }

    fn para(content: Vec<Inline>) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }
    }

    fn no_ranks() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn run(entries: Vec<RuleEntry>) -> Linked {
        link_entries(entries, &no_ranks())
    }

    fn body_of<'a>(linked: &'a Linked, id: &str) -> &'a [Block] {
        &linked
            .entries
            .iter()
            .find(|e| e.id.as_str() == id)
            .unwrap()
            .body
    }

    fn content_of(block: &Block) -> &[Inline] {
        match block {
            Block::Paragraph { content, .. } | Block::Heading { content, .. } => content,
            other => panic!("not a text block: {other:?}"),
        }
    }

    #[test]
    fn an_exact_italic_title_links_without_changing_wording() {
        let linked = run(vec![
            spell("ref/spell/fireball", "Fireball", &["core-rulebook"]),
            host(
                "ref/monster/a",
                "A",
                vec![para(vec![t("It casts "), em("  FIREBALL "), t(" often.")])],
            ),
        ]);
        let content = content_of(&body_of(&linked, "ref/monster/a")[0]).to_vec();
        assert_eq!(
            content,
            vec![
                t("It casts "),
                Inline::Em {
                    c: vec![t("  "), link("ref/spell/fireball", "FIREBALL"), t(" ")],
                },
                t(" often."),
            ]
        );
        assert_eq!(linked.report.links_added, 1);
        assert_eq!(linked.report.entries_changed, 1);
        assert_eq!(linked.report.categories["spell"].added, 1);
        assert_eq!(linked.report.rules["auto:title"], 1);
        let entry = linked
            .entries
            .iter()
            .find(|e| e.id.as_str() == "ref/monster/a")
            .unwrap();
        assert_eq!(entry.links.len(), 1);
        assert_eq!(entry.links[0].target.as_str(), "ref/spell/fireball");
    }

    #[test]
    fn trailing_punctuation_stays_outside_the_link() {
        let linked = run(vec![
            spell("ref/spell/magic-circle", "Magic Circle", &["core-rulebook"]),
            host(
                "ref/monster/a",
                "A",
                vec![para(vec![t("x "), em("magic circle,"), t(" y")])],
            ),
        ]);
        let content = content_of(&body_of(&linked, "ref/monster/a")[0]);
        assert_eq!(
            content[1],
            Inline::Em {
                c: vec![link("ref/spell/magic-circle", "magic circle"), t(",")],
            }
        );
        assert_eq!(linked.report.rules["auto:title+trailing-punctuation"], 1);
    }

    #[test]
    fn a_bold_span_links_to_a_special_ability() {
        let linked = run(vec![
            ability("ref/umr/rend", "Rend", &["bestiary-5"]),
            host(
                "ref/monster/a",
                "A",
                vec![para(vec![strong("Rend"), t(": 2 claws")])],
            ),
        ]);
        let content = content_of(&body_of(&linked, "ref/monster/a")[0]);
        assert_eq!(
            content[0],
            Inline::Strong {
                c: vec![link("ref/umr/rend", "Rend")],
            }
        );
        assert_eq!(linked.report.categories["special_ability"].added, 1);
    }

    #[test]
    fn plain_text_is_never_linked_automatically() {
        let before = vec![para(vec![t("It casts fireball and has rend.")])];
        let linked = run(vec![
            spell("ref/spell/fireball", "Fireball", &["core-rulebook"]),
            ability("ref/umr/rend", "Rend", &["bestiary-5"]),
            host("ref/monster/a", "A", before.clone()),
        ]);
        assert_eq!(body_of(&linked, "ref/monster/a"), before.as_slice());
        assert_eq!(linked.report.links_added, 0);
    }

    #[test]
    fn a_span_inside_a_link_is_left_alone_and_counted() {
        let inside = Inline::Link {
            target: EntryId::parse("ref/other/x").unwrap(),
            anchor: None,
            c: vec![em("fireball")],
        };
        let before = vec![para(vec![inside])];
        let linked = run(vec![
            spell("ref/spell/fireball", "Fireball", &["core-rulebook"]),
            host("ref/monster/a", "A", before.clone()),
        ]);
        assert_eq!(body_of(&linked, "ref/monster/a"), before.as_slice());
        assert_eq!(linked.report.skipped.in_link, 1);
    }

    #[test]
    fn headings_labels_and_header_cells_are_skipped() {
        let heading = Block::Heading {
            level: 2,
            anchor: None,
            content: vec![t("About "), em("fireball")],
        };
        let label = para(vec![strong("Fireball")]);
        let table = Block::Table(Table {
            head: vec![TableRow {
                cells: vec![TableCell {
                    header: true,
                    content: vec![em("fireball")],
                    ..TableCell::default()
                }],
            }],
            ..Table::default()
        });
        let before = vec![heading, label, table];
        let linked = run(vec![
            spell("ref/spell/fireball", "Fireball", &["core-rulebook"]),
            host("ref/monster/a", "A", before.clone()),
        ]);
        assert_eq!(body_of(&linked, "ref/monster/a"), before.as_slice());
        assert_eq!(linked.report.skipped.heading, 1);
        assert_eq!(linked.report.skipped.label, 2);
        assert_eq!(linked.report.links_added, 0);
    }

    #[test]
    fn list_items_table_cells_and_stat_rows_are_linked() {
        let list = Block::List {
            ordered: false,
            items: vec![vec![para(vec![t("Cast "), em("fireball")])]],
        };
        let table = Block::Table(Table {
            body: vec![TableRow {
                cells: vec![TableCell {
                    content: vec![em("fireball")],
                    ..TableCell::default()
                }],
            }],
            ..Table::default()
        });
        let stats = Block::StatBlock {
            title: None,
            rows: vec![StatRow {
                label: "Spells".into(),
                value: vec![em("fireball")],
            }],
        };
        let quote = Block::Quote {
            content: vec![para(vec![t("Cast "), em("fireball")])],
        };
        let linked = run(vec![
            spell("ref/spell/fireball", "Fireball", &["core-rulebook"]),
            host("ref/monster/a", "A", vec![list, table, stats, quote]),
        ]);
        assert_eq!(linked.report.links_added, 4);
    }

    #[test]
    fn a_span_that_names_its_own_entry_is_not_linked() {
        let spell_entry = {
            let mut entry = spell("ref/spell/fireball", "Fireball", &["core-rulebook"]);
            entry.body = vec![para(vec![t("Cast "), em("fireball")])];
            entry
        };
        let linked = run(vec![spell_entry]);
        assert_eq!(linked.report.links_added, 0);
        assert_eq!(linked.report.skipped.self_title, 1);
    }

    fn ranked(entries: Vec<RuleEntry>, ranks: &[(&str, &str)]) -> Linked {
        let ranks = ranks
            .iter()
            .map(|(source, date)| ((*source).to_string(), (*date).to_string()))
            .collect();
        link_entries(entries, &ranks)
    }

    fn target_of(linked: &Linked, id: &str) -> Option<String> {
        fn find(inlines: &[Inline]) -> Option<String> {
            inlines.iter().find_map(|node| match node {
                Inline::Link { target, .. } => Some(target.to_string()),
                Inline::Em { c } | Inline::Strong { c } => find(c),
                _ => None,
            })
        }
        body_of(linked, id)
            .iter()
            .find_map(|block| find(content_of(block)))
    }

    #[test]
    fn the_core_rulebook_wins_an_ambiguous_title() {
        let linked = ranked(
            vec![
                spell("ref/spell/aid-later", "Aid", &["advanced-players-guide"]),
                spell("ref/spell/aid", "Aid", &["ultimate-magic", "core-rulebook"]),
                host(
                    "ref/monster/a",
                    "A",
                    vec![para(vec![t("Cast "), em("aid"), t(".")])],
                ),
            ],
            &[
                ("advanced-players-guide", "2010-08-13"),
                ("core-rulebook", "2009-08-01"),
                ("ultimate-magic", "2011-04-01"),
            ],
        );
        assert_eq!(
            target_of(&linked, "ref/monster/a").as_deref(),
            Some("ref/spell/aid")
        );
        assert!(linked.report.ambiguous.is_empty());
    }

    #[test]
    fn the_earliest_publication_wins_when_none_is_core() {
        let linked = ranked(
            vec![
                spell("ref/spell/aid-b", "Aid", &["ultimate-magic"]),
                spell("ref/spell/aid-a", "Aid", &["advanced-players-guide"]),
                host(
                    "ref/monster/a",
                    "A",
                    vec![para(vec![t("Cast "), em("aid"), t(".")])],
                ),
            ],
            &[
                ("advanced-players-guide", "2010-08-13"),
                ("ultimate-magic", "2011-04-01"),
            ],
        );
        assert_eq!(
            target_of(&linked, "ref/monster/a").as_deref(),
            Some("ref/spell/aid-a")
        );
    }

    fn cast_aid() -> RuleEntry {
        host(
            "ref/monster/a",
            "A",
            vec![para(vec![t("Cast "), em("aid"), t(".")])],
        )
    }

    #[test]
    fn a_tie_stays_unlinked_and_is_reported() {
        let linked = ranked(
            vec![
                spell("ref/spell/aid-a", "Aid", &["core-rulebook"]),
                spell("ref/spell/aid-b", "Aid", &["core-rulebook"]),
                cast_aid(),
            ],
            &[("core-rulebook", "2009-08-01")],
        );
        assert_eq!(target_of(&linked, "ref/monster/a"), None);
        let row = &linked.report.ambiguous["aid"];
        assert_eq!(row.spans, 1);
        assert_eq!(row.category, "spell");
        assert_eq!(row.candidates, vec!["ref/spell/aid-a", "ref/spell/aid-b"]);
        assert_eq!(linked.report.categories["spell"].skipped_ambiguous, 1);
    }

    #[test]
    fn the_ambiguous_category_comes_from_the_tied_entries_only() {
        // Two Core spells tie. A later special ability with the same title is not tied and
        // must not turn the category into `mixed`.
        let linked = ranked(
            vec![
                spell("ref/spell/aid-a", "Aid", &["core-rulebook"]),
                spell("ref/spell/aid-b", "Aid", &["core-rulebook"]),
                ability("ref/umr/aid", "Aid", &["bestiary-5"]),
                cast_aid(),
            ],
            &[
                ("core-rulebook", "2009-08-01"),
                ("bestiary-5", "2017-01-01"),
            ],
        );
        assert_eq!(linked.report.ambiguous["aid"].category, "spell");
        assert_eq!(linked.report.categories["spell"].skipped_ambiguous, 1);
        assert!(!linked.report.categories.contains_key("mixed"));
        // A tie between abilities reports the ability category.
        let linked = run(vec![
            ability("ref/umr/aid-a", "Aid", &["bestiary-5"]),
            ability("ref/umr/aid-b", "Aid", &["bestiary-5"]),
            cast_aid(),
        ]);
        assert_eq!(linked.report.ambiguous["aid"].category, "special_ability");
    }

    #[test]
    fn a_spell_wins_over_a_special_ability_of_the_same_title() {
        let linked = ranked(
            vec![
                ability("ref/umr/aid", "Aid", &["core-rulebook"]),
                spell("ref/spell/aid", "Aid", &["core-rulebook"]),
                cast_aid(),
            ],
            &[("core-rulebook", "2009-08-01")],
        );
        assert_eq!(
            target_of(&linked, "ref/monster/a").as_deref(),
            Some("ref/spell/aid")
        );
        assert!(linked.report.ambiguous.is_empty());
    }

    #[test]
    fn a_blocked_term_is_never_linked_and_is_counted() {
        let linked = run(vec![
            spell("ref/spell/web", "Web", &["core-rulebook"]),
            host(
                "ref/monster/a",
                "A",
                vec![para(vec![t("Cast "), em("web"), t(".")])],
            ),
        ]);
        assert_eq!(target_of(&linked, "ref/monster/a"), None);
        assert_eq!(linked.report.blocked["web"].spans, 1);
        assert_eq!(linked.report.categories["spell"].skipped_blocked, 1);
        assert!(!linked.report.blocked["web"].reason.is_empty());
    }

    fn darkvision() -> RuleEntry {
        base(
            "ref/rule/darkvision",
            EntryKind::Rule,
            "Darkvision",
            &["core-rulebook"],
        )
    }

    #[test]
    fn a_curated_term_links_only_its_first_plain_occurrence_in_a_block() {
        let linked = run(vec![
            darkvision(),
            host(
                "ref/monster/a",
                "A",
                vec![
                    para(vec![t("It has Darkvision 60 ft. and darkvision again.")]),
                    para(vec![t("darkvision.")]),
                ],
            ),
        ]);
        let body = body_of(&linked, "ref/monster/a");
        assert_eq!(
            content_of(&body[0]),
            &[
                t("It has "),
                link("ref/rule/darkvision", "Darkvision"),
                t(" 60 ft. and darkvision again."),
            ]
        );
        assert_eq!(
            content_of(&body[1]),
            &[link("ref/rule/darkvision", "darkvision"), t(".")]
        );
        let row = &linked.report.curated["darkvision"];
        assert_eq!((row.plain, row.skipped_repeat), (2, 1));
        assert_eq!(linked.report.rules["term:darkvision"], 2);
    }

    #[test]
    fn a_curated_term_respects_word_boundaries() {
        let before = vec![para(vec![t(
            "Darkvisions and nondarkvision and dark-vision.",
        )])];
        let linked = run(vec![
            darkvision(),
            host("ref/monster/a", "A", before.clone()),
        ]);
        assert_eq!(body_of(&linked, "ref/monster/a"), before.as_slice());
    }

    #[test]
    fn a_curated_term_skips_a_block_that_links_its_target() {
        let before = vec![para(vec![
            link("ref/rule/darkvision", "sight"),
            t(" and darkvision"),
        ])];
        let linked = run(vec![
            darkvision(),
            host("ref/monster/a", "A", before.clone()),
        ]);
        assert_eq!(body_of(&linked, "ref/monster/a"), before.as_slice());
        assert_eq!(linked.report.curated["darkvision"].skipped_repeat, 1);
    }

    #[test]
    fn an_emphasised_curated_term_that_names_a_spell_links_the_spell() {
        let linked = run(vec![
            darkvision(),
            spell("ref/spell/darkvision", "Darkvision", &["core-rulebook"]),
            host(
                "ref/monster/a",
                "A",
                vec![para(vec![
                    t("It casts "),
                    em("darkvision"),
                    t(" and has darkvision, "),
                    strong("Darkvision"),
                    t("."),
                ])],
            ),
        ]);
        let content = content_of(&body_of(&linked, "ref/monster/a")[0]);
        assert_eq!(
            content[1],
            Inline::Em {
                c: vec![link("ref/spell/darkvision", "darkvision")],
            }
        );
        // The plain occurrence still takes the curated rule target.
        assert_eq!(content[2], t(" and has "));
        assert_eq!(content[3], link("ref/rule/darkvision", "darkvision"));
        let row = &linked.report.curated["darkvision"];
        assert_eq!(
            (row.emphasised, row.emphasised_to_spell, row.plain),
            (0, 2, 1)
        );
        assert!(linked.report.ambiguous.is_empty());
    }

    #[test]
    fn an_emphasised_curated_term_with_no_spell_title_links_the_curated_target() {
        let linked = run(vec![
            darkvision(),
            ability("ref/umr/darkvision", "Darkvision", &["bestiary-5"]),
            host(
                "ref/monster/a",
                "A",
                vec![para(vec![em("darkvision"), t(", "), em("darkvision")])],
            ),
        ]);
        let content = content_of(&body_of(&linked, "ref/monster/a")[0]);
        assert_eq!(
            content[0],
            Inline::Em {
                c: vec![link("ref/rule/darkvision", "darkvision")],
            }
        );
        let row = &linked.report.curated["darkvision"];
        assert_eq!((row.emphasised, row.emphasised_to_spell), (2, 0));
        assert!(linked.report.ambiguous.is_empty());
    }

    #[test]
    fn an_emphasised_curated_term_skips_an_entry_titled_like_another_form_of_the_term() {
        // The entry is titled `Spell-Like Abilities`, the target's title. The span is the
        // singular form, so only the title-like check can catch it.
        let linked = run(vec![
            ability(
                "ref/umr/spell-like-abilities",
                "Spell-Like Abilities",
                &["bestiary-5"],
            ),
            host(
                "ref/rule/copy",
                "Spell-Like Abilities",
                vec![para(vec![t("Use a "), em("spell-like ability"), t(".")])],
            ),
        ]);
        assert_eq!(linked.report.links_added, 0);
        let row = &linked.report.curated["spell-like ability"];
        assert_eq!((row.skipped_self, row.emphasised), (1, 0));
    }

    #[test]
    fn plain_matches_in_headings_labels_links_and_emphasis_are_counted() {
        let other = Inline::Link {
            target: EntryId::parse("ref/other/x").unwrap(),
            anchor: None,
            c: vec![t("see darkvision")],
        };
        let exact_in_link = Inline::Link {
            target: EntryId::parse("ref/other/y").unwrap(),
            anchor: None,
            c: vec![em("darkvision")],
        };
        let before = vec![
            Block::Heading {
                level: 2,
                anchor: None,
                content: vec![t("About darkvision and Darkvision")],
            },
            // A label that is not itself a term, and a label that is exactly the term.
            para(vec![strong("Darkvision 60 ft.")]),
            para(vec![strong("Darkvision")]),
            para(vec![t("x "), other, t(" "), exact_in_link]),
            para(vec![t("y "), em("darkvision 60 ft.")]),
        ];
        let linked = run(vec![
            darkvision(),
            host("ref/monster/a", "A", before.clone()),
        ]);
        assert_eq!(body_of(&linked, "ref/monster/a"), before.as_slice());
        assert_eq!(linked.report.links_added, 0);
        let row = &linked.report.curated["darkvision"];
        assert_eq!(row.skipped_in_heading, 2);
        // Only the label with extra text is a plain match; the exact label is counted once,
        // by the span pass.
        assert_eq!(row.skipped_in_label, 1);
        assert_eq!(linked.report.skipped.label, 1);
        assert_eq!(row.skipped_in_link, 1);
        assert_eq!(linked.report.skipped.in_link, 1);
        assert_eq!(row.skipped_in_emphasis, 1);
    }

    #[test]
    fn a_curated_term_does_not_link_inside_its_own_target() {
        let mut own = darkvision();
        own.body = vec![para(vec![t("Darkvision lets you see.")])];
        let mut titled = base(
            "ref/spell/darkvision",
            EntryKind::Spell,
            "Darkvision",
            &["core-rulebook"],
        );
        titled.body = vec![para(vec![t("The subject gains darkvision.")])];
        let linked = run(vec![own, titled]);
        assert_eq!(linked.report.links_added, 0);
        assert_eq!(linked.report.curated["darkvision"].skipped_self, 2);
    }

    #[test]
    fn a_curated_term_with_a_missing_target_is_reported_and_left_alone() {
        let before = vec![para(vec![t("It has darkvision.")])];
        let linked = run(vec![host("ref/monster/a", "A", before.clone())]);
        assert_eq!(body_of(&linked, "ref/monster/a"), before.as_slice());
        assert!(
            linked
                .report
                .missing_targets
                .iter()
                .any(|m| m.starts_with("darkvision ->"))
        );
    }

    #[test]
    fn running_the_pass_twice_adds_nothing_and_output_is_stable() {
        let entries = vec![
            darkvision(),
            spell("ref/spell/fireball", "Fireball", &["core-rulebook"]),
            host(
                "ref/monster/a",
                "A",
                vec![para(vec![
                    em("fireball,"),
                    t(" and darkvision and darkvision."),
                ])],
            ),
        ];
        let first = run(entries.clone());
        assert_eq!(first, run(entries));
        let second = run(first.entries.clone());
        assert_eq!(second.entries, first.entries);
        assert_eq!(second.report.links_added, 0);
    }

    #[test]
    fn special_ability_targets_are_the_umr_the_core_glossary_and_the_carried_umr() {
        let mut glossary = base("ref/rule/rend", EntryKind::Rule, "Rend", &["core-rulebook"]);
        glossary.provenance.push(Provenance {
            dataset: "pf-data".into(),
            record: "not_a_record".into(),
            path: CORE_GLOSSARY_PATH.into(),
        });
        let mut carried = base(
            "ref/carried/bestiary/rules/universal-monster-rules/pounce",
            EntryKind::Rule,
            "Pounce",
            &["bestiary"],
        );
        carried.section = vec!["Rules".into(), CARRIED_UMR_SECTION.into()];
        let plain_rule = base(
            "ref/rule/grapple",
            EntryKind::Rule,
            "Grapple",
            &["core-rulebook"],
        );
        let other_glossary = {
            let mut entry = base("ref/rule/trip", EntryKind::Rule, "Trip", &["core-rulebook"]);
            entry.provenance.push(Provenance {
                dataset: "pf-data".into(),
                record: "not_a_glossary_record".into(),
                path: CORE_GLOSSARY_PATH.into(),
            });
            entry
        };
        assert_eq!(
            category_of(&ability("ref/umr/a", "A", &[])),
            Some(Category::SpecialAbility)
        );
        assert_eq!(
            category_of(&glossary),
            None,
            "the record is not in the glossary list"
        );
        glossary.provenance[0].record = "paralysis".into();
        assert_eq!(category_of(&glossary), Some(Category::SpecialAbility));
        assert_eq!(category_of(&carried), Some(Category::SpecialAbility));
        assert_eq!(category_of(&plain_rule), None);
        assert_eq!(category_of(&other_glossary), None);
        assert_eq!(
            category_of(&spell("ref/spell/a", "A", &[])),
            Some(Category::Spell)
        );
    }

    #[test]
    fn the_curated_list_and_the_blocklist_are_consistent() {
        let mut texts = BTreeSet::new();
        for term in TERMS {
            assert_eq!(
                term.text,
                normalise(term.text),
                "{} is not normalised",
                term.text
            );
            assert!(texts.insert(term.text), "duplicate term {}", term.text);
            assert!(
                EntryId::parse(term.target).is_ok(),
                "bad target {}",
                term.target
            );
            assert!(!term.reason.is_empty());
        }
        let mut blocked = BTreeSet::new();
        for (text, reason) in BLOCKLIST {
            assert_eq!(*text, normalise(text), "{text} is not normalised");
            assert!(blocked.insert(*text), "duplicate blocked term {text}");
            assert!(!texts.contains(text), "{text} is both blocked and curated");
            assert!(!reason.is_empty());
        }
    }
}
