//! The Options section of a class page (#739): the selectable abilities of the class's option
//! groups (rogue talents, domains, hexes), and the row under the class feature that grants a
//! choice from a group. Everything here is pure, over typed content, and has native tests; the
//! drawing is `views/option_list.rs`.
//!
//! The library holds no typed level or prerequisite for a class option, and an ability tag only on
//! an option that the corpus typed with `ability_type` (owner ruling 63). What it does hold is the
//! printed body, so the rows read three facts from the printed text, and the tag from the field
//! first (see [`facts_of`] and [`facts_with_type`]).

use crate::route::OptionChoice;
use cast_iron_schema::headings::plain_text;
use cast_iron_schema::power::{split_ability_tag, straight_apostrophe};
use cast_iron_schema::{Block, EntryId, Inline, OptionGroup};

// ---- the feature that grants a choice from a group ----------------------------------------

/// A name as a reader compares it: lower case, no ability tag, no trailing colon, no possessive
/// `'s`, and single spaces. `Rogue Talents (Ex)`, `Arcane Discoveries:` and `Oracle's Curse (Ex)`
/// become `rogue talents`, `arcane discoveries` and `oracle curse`.
pub fn compare_name(text: &str) -> String {
    let straight: String = text.chars().map(straight_apostrophe).collect();
    let untagged = straight.trim().trim_end_matches(':');
    let (name, _) = split_ability_tag(untagged);
    // A trailing space lets the possessive at the end of a name go as well.
    format!("{} ", name.trim())
        .to_lowercase()
        .replace("'s ", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The names under which a feature of `class_title` can print `group`: its label and its
/// singular, each also without the leading words that only name the class (`Witch hex` is
/// printed `Hex` on the Witch page, `Cleric domains` is `Domains`). A group label that does not
/// start with a word of the class title (`Arcane school`) keeps its whole name, so a heading that
/// is a bare `School` or `Discovery` links nothing.
pub fn group_names(group: &OptionGroup, class_title: &str) -> Vec<String> {
    let class_words: Vec<String> = compare_name(class_title)
        .split(' ')
        .map(str::to_string)
        .collect();
    let mut names: Vec<String> = Vec::new();
    for text in [&group.label, &group.singular] {
        let full = compare_name(text);
        let words: Vec<&str> = full.split(' ').collect();
        let skip = words
            .iter()
            .take_while(|word| class_words.iter().any(|class| class == *word))
            .count();
        if skip > 0 && skip < words.len() {
            names.push(words[skip..].join(" "));
        }
        names.push(full);
    }
    names.dedup();
    names
}

/// Where the row of one group goes on a class page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionLink {
    /// The slug of the group that the row opens in the Options section.
    pub group: String,
    /// The feature that grants the choice, as printed (`Rogue Talents`), for a test and a log.
    pub feature: String,
    /// The index of the body block that the row goes before; `body.len()` is the end of the body.
    pub before: usize,
}

/// Whether a top-level heading ends the Features section: the sections that follow it on every
/// class page. The other top-level headings (`Spells Known`, `Ex-Clerics`) stay inside it, because
/// a caster's features are printed as deeper headings after `Spells Known`.
fn ends_features(title: &str) -> bool {
    let title = title.trim().to_lowercase();
    title.starts_with("alternate capstone")
        || title == "archetypes"
        || title.starts_with("favored class")
}

/// The blocks of the Features section: from the first top-level heading that begins `Class
/// Features` to the heading that starts Alternate Capstones, Archetypes or the favored class
/// bonuses. `None` when the body has no such heading.
fn features_span(body: &[Block]) -> Option<(usize, usize)> {
    let top = body
        .iter()
        .filter_map(|block| match block {
            Block::Heading { level, .. } => Some(*level),
            _ => None,
        })
        .min()?;
    let start = body.iter().position(|block| match block {
        Block::Heading { level, content, .. } if *level == top => plain_text(content)
            .trim()
            .to_lowercase()
            .starts_with("class features"),
        _ => false,
    })?;
    let end = body[start + 1..]
        .iter()
        .position(|block| match block {
            Block::Heading { level, content, .. } => {
                *level <= top && ends_features(&plain_text(content))
            }
            _ => false,
        })
        .map_or(body.len(), |offset| start + 1 + offset);
    Some((start, end))
}

/// A candidate feature of the class: a heading (and the text under it) or a bold name that
/// starts a paragraph.
struct Candidate {
    name: String,
    printed: String,
    /// The block that the row goes before.
    before: usize,
}

/// Where each of `groups` is chosen in the Features section of `body`, one row per group at most.
/// A group is chosen where a heading of the section is named for it (see [`group_names`]); a
/// class that prints no such heading (the Wizard's `Arcane Discoveries:` is a bold name inside the
/// Bonus feats text) falls back to the first paragraph that opens with the group's name in bold.
/// A heading's row goes after all the text under it, before the next heading of the same or a
/// higher level; a bold name's row goes right after its paragraph. A group that the section does
/// not name gets no row, and its chip in the Options section still works. The match reads the
/// printed names, so it is the same on an imported body (bold names and plain headings) and on a
/// converted one (headings that list `features`).
pub fn option_links(body: &[Block], class_title: &str, groups: &[&OptionGroup]) -> Vec<OptionLink> {
    let Some((start, end)) = features_span(body) else {
        return Vec::new();
    };
    let mut headings: Vec<Candidate> = Vec::new();
    let mut bold: Vec<Candidate> = Vec::new();
    for index in start + 1..end {
        match &body[index] {
            Block::Heading { level, content, .. } => {
                let printed = plain_text(content);
                let before = body[index + 1..end]
                    .iter()
                    .position(|block| matches!(block, Block::Heading { level: next, .. } if next <= level))
                    .map_or(end, |offset| index + 1 + offset);
                headings.push(Candidate {
                    name: compare_name(&printed),
                    printed,
                    before,
                });
            }
            Block::Paragraph { content, .. } => {
                if let Some(Inline::Strong { c }) = content.first() {
                    let printed = plain_text(c);
                    bold.push(Candidate {
                        name: compare_name(&printed),
                        printed,
                        before: index + 1,
                    });
                }
            }
            _ => {}
        }
    }
    let mut links: Vec<OptionLink> = groups
        .iter()
        .filter_map(|group| {
            let names = group_names(group, class_title);
            let found = headings
                .iter()
                .chain(&bold)
                .find(|candidate| names.contains(&candidate.name))?;
            Some(OptionLink {
                group: group.slug.clone(),
                feature: found.printed.trim().to_string(),
                before: found.before,
            })
        })
        .collect();
    // Rows at one place keep the order of the groups.
    links.sort_by_key(|link| link.before);
    links
}

// ---- what a row says ----------------------------------------------------------------------

/// What the printed text of a class option says about it. Every field is `None` when the text
/// does not print it, and none is ever worked out from anything else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// The ability tag of the option's own name, as printed: `(Ex)`, `(Su)`, `(Sp)`, `(Ex or Su)`.
    pub tag: Option<String>,
    /// The number of a `Level:` row (Kineticist wild talents print one).
    pub level: Option<u8>,
    /// The text of a `Prereq:`, `Prerequisite:` or `Prerequisites:` row.
    pub prerequisite: Option<String>,
}

fn letters(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The bold label that opens a paragraph, with the text after it: `("Prereq:", "9th-level magus")`.
fn row_of(block: &Block) -> Option<(String, String)> {
    let Block::Paragraph { content, .. } = block else {
        return None;
    };
    let Some(Inline::Strong { c }) = content.first() else {
        return None;
    };
    Some((
        plain_text(c).trim().to_string(),
        plain_text(&content[1..]).trim().to_string(),
    ))
}

/// The facts of the option titled `title` whose printed body is `body`.
///
/// - **Tag.** The first block is the option's name, as a heading or as a bold paragraph
///   (`Accurate Poisoner (Ex)`). Its tag counts only when the name without the tag is the option's
///   title: the first heading of an option that prints several powers is a power, not the option,
///   and its tag is not the option's.
/// - **Level.** A paragraph that opens with the bold label `Level:` and a whole number.
/// - **Prerequisite.** A paragraph that opens with `Prereq:`, `Prerequisite:` or `Prerequisites:`.
pub fn facts_of(title: &str, body: &[Block]) -> Facts {
    let mut facts = Facts::default();
    let name = match body.first() {
        Some(Block::Heading { content, .. }) => Some(plain_text(content)),
        Some(Block::Paragraph { content, .. }) => match content.first() {
            Some(Inline::Strong { c }) => Some(plain_text(c)),
            _ => None,
        },
        _ => None,
    };
    if let Some(name) = name {
        let (bare, tag) = split_ability_tag(name.trim().trim_end_matches(':'));
        if let Some(tag) = tag
            && letters(bare) == letters(title)
        {
            facts.tag = Some(tag.to_string());
        }
    }
    for block in body {
        let Some((label, rest)) = row_of(block) else {
            continue;
        };
        match label.to_lowercase().as_str() {
            "level:" if facts.level.is_none() => facts.level = rest.parse().ok(),
            "prereq:" | "prereqs:" | "prerequisite:" | "prerequisites:"
                if facts.prerequisite.is_none() =>
            {
                let text = rest.trim_end_matches('.').trim();
                if !text.is_empty() {
                    facts.prerequisite = Some(text.to_string());
                }
            }
            _ => {}
        }
    }
    facts
}

/// [`facts_of`], with the tag of the entry's typed `ability_type` (owner ruling 63) in place of a
/// printed one when the entry has it. The field holds the text inside the parentheses; a row
/// shows it as printed, in them. An option with the field has no head to read a tag from, so the
/// field is the only source for it.
pub fn facts_with_type(title: &str, body: &[Block], ability_type: Option<&str>) -> Facts {
    let mut facts = facts_of(title, body);
    if let Some(kind) = ability_type {
        facts.tag = Some(format!("({kind})"));
    }
    facts
}

/// The longest prerequisite that sits at the right of a row. A longer one goes on the muted line
/// under the name, where it can wrap.
const ASIDE_LIMIT: usize = 24;

/// One option of a group, as the list draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionRow {
    pub id: EntryId,
    pub title: String,
    pub facts: Facts,
    /// The publication id of the book the row names, for the book filter.
    pub source: String,
    /// That book's title.
    pub book: String,
}

impl OptionRow {
    /// The right-hand text of the row: `Level 3`, or a short prerequisite such as `9th-level
    /// magus`. `None` when the text prints neither.
    pub fn aside(&self) -> Option<String> {
        if let Some(level) = self.facts.level {
            return Some(format!("Level {level}"));
        }
        self.facts
            .prerequisite
            .clone()
            .filter(|text| text.chars().count() <= ASIDE_LIMIT)
    }

    /// A prerequisite too long for [`Self::aside`], for the muted line.
    pub fn prerequisite_line(&self) -> Option<String> {
        if self.facts.level.is_some() {
            return self
                .facts
                .prerequisite
                .as_ref()
                .map(|text| format!("Prerequisite: {text}"));
        }
        self.facts
            .prerequisite
            .as_ref()
            .filter(|text| text.chars().count() > ASIDE_LIMIT)
            .map(|text| format!("Prerequisite: {text}"))
    }
}

// ---- narrowing ----------------------------------------------------------------------------

/// The indexes of `rows` that the name text, the book and the level keep, in row order. The three
/// arguments are separate so a menu can count its own values under the other two filters.
pub fn narrow_by(
    rows: &[OptionRow],
    find: &str,
    source: Option<&str>,
    level: Option<u8>,
) -> Vec<usize> {
    let find = find.trim().to_lowercase();
    rows.iter()
        .enumerate()
        .filter(|(_, row)| {
            (find.is_empty() || row.title.to_lowercase().contains(&find))
                && source.is_none_or(|wanted| row.source == wanted)
                && level.is_none_or(|wanted| row.facts.level == Some(wanted))
        })
        .map(|(index, _)| index)
        .collect()
}

/// [`narrow_by`] with a route's choices.
pub fn narrow(rows: &[OptionRow], choice: &OptionChoice) -> Vec<usize> {
    narrow_by(rows, &choice.find, choice.source.as_deref(), choice.level)
}

/// The books of `rows` with how many rows each keeps under the other two filters, in the order
/// of their titles. A book that the route chose stays listed with its count, even at nothing, so
/// the choice can be cleared.
pub fn book_counts(rows: &[OptionRow], choice: &OptionChoice) -> Vec<(String, String, usize)> {
    let kept = narrow_by(rows, &choice.find, None, choice.level);
    let mut books: Vec<(String, String, usize)> = Vec::new();
    for row in rows {
        if !books.iter().any(|(source, ..)| *source == row.source) {
            let count = kept
                .iter()
                .filter(|&&i| rows[i].source == row.source)
                .count();
            books.push((row.source.clone(), row.book.clone(), count));
        }
    }
    cast_iron_sort::sort_by_key(&mut books, |(_, title, _)| title.to_lowercase());
    books
}

/// The levels that `rows` print with how many rows each keeps under the other two filters, from
/// the lowest. Empty when no row prints a level: the group has no level menu then.
pub fn level_counts(rows: &[OptionRow], choice: &OptionChoice) -> Vec<(u8, usize)> {
    let kept = narrow_by(rows, &choice.find, choice.source.as_deref(), None);
    let mut levels: Vec<u8> = rows.iter().filter_map(|row| row.facts.level).collect();
    levels.sort_unstable();
    levels.dedup();
    levels
        .into_iter()
        .map(|level| {
            let count = kept
                .iter()
                .filter(|&&i| rows[i].facts.level == Some(level))
                .count();
            (level, count)
        })
        .collect()
}

/// The group the route shows: the one it names, else the first of the class.
pub fn shown_group<'a, T>(
    groups: &'a [T],
    slug_of: impl Fn(&T) -> &str,
    wanted: Option<&str>,
) -> Option<&'a T> {
    wanted
        .and_then(|wanted| groups.iter().find(|group| slug_of(group) == wanted))
        .or_else(|| groups.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::ParagraphStyle;

    fn group(slug: &str, label: &str, singular: &str) -> OptionGroup {
        OptionGroup {
            slug: slug.into(),
            label: label.into(),
            singular: singular.into(),
            classes: Vec::new(),
            entries: Vec::new(),
        }
    }

    fn heading(level: u8, title: &str) -> Block {
        Block::Heading {
            level,
            anchor: None,
            content: vec![Inline::text(title)],
        }
    }

    fn plain(text: &str) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![Inline::text(text)],
        }
    }

    fn row(label: &str, text: &str) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                Inline::Strong {
                    c: vec![Inline::text(label)],
                },
                Inline::text(format!(" {text}")),
            ],
        }
    }

    fn bold_name(name: &str) -> Block {
        Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![Inline::Strong {
                c: vec![Inline::text(name)],
            }],
        }
    }

    #[test]
    fn a_name_loses_its_tag_colon_case_and_possessives() {
        assert_eq!(compare_name("Rogue Talents (Ex)"), "rogue talents");
        assert_eq!(compare_name("Arcane   Bond (Ex or Sp)"), "arcane bond");
        assert_eq!(compare_name("Arcane Discoveries:"), "arcane discoveries");
        assert_eq!(
            compare_name("Witch\u{2019}s Familiar (Ex)"),
            "witch familiar"
        );
        assert_eq!(compare_name("Oracle's Curse (Ex)"), "oracle curse");
        assert_eq!(compare_name("Hunter\u{2019}s"), "hunter");
        // Words in brackets that are no tag stay.
        assert_eq!(compare_name("Trap Sense (Lesser)"), "trap sense (lesser)");
    }

    #[test]
    fn a_group_answers_to_its_label_its_singular_and_both_without_the_class_name() {
        let hex = group("witch-hex", "Witch hexes", "Witch hex");
        assert_eq!(
            group_names(&hex, "Witch"),
            ["hexes", "witch hexes", "hex", "witch hex"]
        );
        // On another class's page the class name is no reason to drop a word.
        assert_eq!(group_names(&hex, "Shaman"), ["witch hexes", "witch hex"]);
        // A first word that is no word of the class title stays: a bare `School` is no match.
        let school = group("arcane-school", "Arcane schools", "Arcane school");
        assert_eq!(
            group_names(&school, "Wizard"),
            ["arcane schools", "arcane school"]
        );
        // A one-word group name is never emptied.
        let orders = group("order", "Orders", "Order");
        assert_eq!(group_names(&orders, "Order"), ["orders", "order"]);
    }

    /// The Rogue's body in the corpus's imported shape.
    fn rogue() -> Vec<Block> {
        vec![
            row("Role:", "Rogues excel."),
            heading(3, "Class Features"),
            heading(4, "Sneak Attack"),
            plain("If a rogue can catch an opponent..."),
            heading(4, "Rogue Talents"),
            row("Gained:", "2nd level"),
            plain("As a rogue gains experience, she learns a number of talents."),
            heading(4, "Trap Sense (Ex)"),
            plain("At 3rd level..."),
            heading(3, "Alternate Capstones"),
            bold_name("Masterful Talent (Ex)"),
            heading(3, "Archetypes"),
        ]
    }

    #[test]
    fn a_heading_that_names_the_group_gets_its_row_after_the_text_under_it() {
        let talents = group("rogue-talent", "Rogue talents", "Rogue talent");
        let links = option_links(&rogue(), "Rogue", &[&talents]);
        assert_eq!(
            links,
            [OptionLink {
                group: "rogue-talent".into(),
                feature: "Rogue Talents".into(),
                // Before `Trap Sense (Ex)`: after the heading's two paragraphs.
                before: 7,
            }]
        );
    }

    #[test]
    fn a_tag_on_the_heading_and_a_class_name_missing_from_it_do_not_stop_the_match() {
        let body = vec![
            heading(3, "Class Features and Spells Per Day"),
            heading(4, "Hex"),
            plain("Hexes are..."),
            heading(4, "Major Hex"),
            plain("At 10th level..."),
            heading(3, "Archetypes"),
        ];
        let hex = group("witch-hex", "Witch hexes", "Witch hex");
        let links = option_links(&body, "Witch", &[&hex]);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].feature, "Hex");
        assert_eq!(links[0].before, 3);
        // `Major Hex` is no name of the group, so the first match is the one that counts.
        let barbarian = vec![
            heading(3, "Class Features"),
            heading(4, "Rage Powers (Ex)"),
            plain("As a barbarian gains levels..."),
            heading(3, "Archetypes"),
        ];
        let powers = group("rage-power", "Rage powers", "Rage power");
        let links = option_links(&barbarian, "Barbarian", &[&powers]);
        assert_eq!(links[0].feature, "Rage Powers (Ex)");
        assert_eq!(links[0].before, 3);
    }

    #[test]
    fn a_bold_name_is_a_feature_when_no_heading_names_the_group() {
        let body = vec![
            heading(3, "Class Features and Spells Per Day"),
            heading(4, "Bonus Feats"),
            plain("At 5th level..."),
            row("Arcane Discoveries:", "A wizard can pick one."),
            plain("More text."),
            heading(3, "Archetypes"),
        ];
        let discoveries = group("arcane-discovery", "Arcane discoveries", "Arcane discovery");
        let links = option_links(&body, "Wizard", &[&discoveries]);
        // The row goes right after the paragraph that opens with the bold name.
        assert_eq!(links[0].before, 4);
        // A heading wins over a bold name: it is the feature's own place.
        let both = vec![
            heading(3, "Class Features"),
            row("Domains:", "Choose two."),
            heading(4, "Domains"),
            plain("A cleric chooses..."),
            heading(3, "Archetypes"),
        ];
        let domains = group("cleric-domain", "Cleric domains", "Cleric domain");
        assert_eq!(option_links(&both, "Cleric", &[&domains])[0].before, 4);
    }

    #[test]
    fn a_casters_features_after_spells_known_are_in_the_section() {
        // The Sorcerer prints `Spells Known` as a top-level heading before its deeper features.
        let body = vec![
            heading(3, "Class Features and Spells Per Day"),
            heading(3, "Spells Known"),
            heading(4, "Bloodline"),
            plain("Each sorcerer has a bloodline."),
            heading(3, "Alternate Capstones"),
        ];
        let bloodlines = group(
            "sorcerer-bloodline",
            "Sorcerer bloodlines",
            "Sorcerer bloodline",
        );
        let links = option_links(&body, "Sorcerer", &[&bloodlines]);
        assert_eq!(links[0].feature, "Bloodline");
        assert_eq!(links[0].before, 4);
        // A possessive in the heading does not hide the group.
        let oracle = vec![
            heading(3, "Class Features"),
            heading(4, "Oracle\u{2019}s Curse (Ex)"),
            plain("Each oracle is cursed."),
            heading(3, "Archetypes"),
        ];
        let curses = group("oracle-curse", "Oracle curses", "Oracle curse");
        assert_eq!(option_links(&oracle, "Oracle", &[&curses])[0].before, 3);
    }

    #[test]
    fn only_the_features_section_is_read() {
        // `Masterful Talent (Ex)` sits under Alternate Capstones, and an archetype's own heading
        // under Archetypes: neither is a place for a group's row.
        let body = vec![
            heading(3, "Class Features"),
            plain("Text."),
            heading(3, "Alternate Capstones"),
            heading(4, "Rogue Talents"),
            heading(3, "Archetypes"),
        ];
        let talents = group("rogue-talent", "Rogue talents", "Rogue talent");
        assert!(option_links(&body, "Rogue", &[&talents]).is_empty());
        // A body without a Features heading has no rows.
        assert!(option_links(&[heading(3, "Rogue Talents")], "Rogue", &[&talents]).is_empty());
    }

    #[test]
    fn two_groups_of_a_class_each_find_their_own_feature() {
        let body = vec![
            heading(3, "Class Features"),
            heading(4, "Animal Companion"),
            plain("A druid..."),
            heading(4, "Domain"),
            plain("A druid who chooses a domain..."),
            heading(3, "Archetypes"),
        ];
        let companions = group("animal-companion", "Animal companions", "Animal companion");
        let domains = group("druid-domain", "Druid domains", "Druid domain");
        let links = option_links(&body, "Druid", &[&domains, &companions]);
        let found: Vec<(&str, &str)> = links
            .iter()
            .map(|link| (link.group.as_str(), link.feature.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                ("animal-companion", "Animal Companion"),
                ("druid-domain", "Domain")
            ]
        );
    }

    #[test]
    fn a_group_that_no_feature_names_gets_no_row() {
        let body = vec![
            heading(3, "Class Features"),
            heading(4, "Favored Enemy (Ex)"),
            plain("..."),
            heading(3, "Archetypes"),
        ];
        let traps = group("ranger-trap", "Ranger traps", "Ranger trap");
        assert!(option_links(&body, "Ranger", &[&traps]).is_empty());
    }

    #[test]
    fn a_tag_is_read_only_from_the_options_own_name() {
        let named = vec![
            bold_name("Accurate Poisoner (Ex)"),
            row("Flavor:", "A rogue..."),
        ];
        assert_eq!(
            facts_of("Accurate Poisoner", &named).tag.as_deref(),
            Some("(Ex)")
        );
        // A heading that is a power of the option, not the option, lends no tag.
        let powers = vec![heading(3, "Lore Keeper (Su)"), plain("...")];
        assert_eq!(facts_of("Arcane School", &powers).tag, None);
        // A combined tag, curly apostrophes and a colon do not hide the name.
        let combined = vec![bold_name("Witch\u{2019}s Cunning (Ex or Su):")];
        assert_eq!(
            facts_of("Witch's Cunning", &combined).tag.as_deref(),
            Some("(Ex or Su)")
        );
        // No tag printed, no tag shown.
        assert_eq!(
            facts_of("Quick Hands", &[bold_name("Quick Hands")]).tag,
            None
        );
        assert_eq!(facts_of("Empty", &[]), Facts::default());
    }

    #[test]
    fn the_typed_ability_type_gives_the_tag_and_wins_over_the_body() {
        // A single-power option has no head: the field is the tag, in parentheses.
        let headless = vec![plain("Once per day, the rage...")];
        assert_eq!(
            facts_with_type("Agony", &headless, Some("Su"))
                .tag
                .as_deref(),
            Some("(Su)")
        );
        assert_eq!(
            facts_with_type("Agony", &headless, Some("Ex or Sp"))
                .tag
                .as_deref(),
            Some("(Ex or Sp)")
        );
        // Without the field the body is read as before.
        assert_eq!(facts_with_type("Agony", &headless, None).tag, None);
        let named = vec![bold_name("Agony (Ex)")];
        assert_eq!(
            facts_with_type("Agony", &named, None).tag.as_deref(),
            Some("(Ex)")
        );
        assert_eq!(
            facts_with_type("Agony", &named, Some("Su")).tag.as_deref(),
            Some("(Su)")
        );
    }

    #[test]
    fn level_and_prerequisite_are_the_printed_rows_and_nothing_else() {
        let body = vec![
            bold_name("Burning Infusion"),
            row("Type:", "Utility"),
            row("Level:", "3"),
            row("Prerequisite:", "Kinetic blast."),
            row("Prereq:", "ignored: the first one counts"),
        ];
        let facts = facts_of("Burning Infusion", &body);
        assert_eq!(facts.level, Some(3));
        assert_eq!(facts.prerequisite.as_deref(), Some("Kinetic blast"));
        // `Level:` that is not a whole number is no level; no row is no fact.
        let odd = vec![row("Level:", "1st"), row("Prereq:", "")];
        assert_eq!(facts_of("X", &odd), Facts::default());
    }

    fn option(
        title: &str,
        source: &str,
        level: Option<u8>,
        prerequisite: Option<&str>,
    ) -> OptionRow {
        OptionRow {
            id: EntryId::parse(format!(
                "ref/talent/{}",
                title.to_lowercase().replace(' ', "-")
            ))
            .unwrap(),
            title: title.into(),
            facts: Facts {
                tag: None,
                level,
                prerequisite: prerequisite.map(String::from),
            },
            source: source.into(),
            book: format!("Book {source}"),
        }
    }

    #[test]
    fn the_right_side_of_a_row_is_a_level_or_a_short_prerequisite() {
        assert_eq!(
            option("A", "core", Some(3), None).aside().as_deref(),
            Some("Level 3")
        );
        assert_eq!(
            option("A", "core", None, Some("9th-level magus"))
                .aside()
                .as_deref(),
            Some("9th-level magus")
        );
        let long = option(
            "A",
            "core",
            None,
            Some("6th-level magus, levels in at least one other spellcasting class"),
        );
        assert_eq!(long.aside(), None);
        assert_eq!(
            long.prerequisite_line().as_deref(),
            Some("Prerequisite: 6th-level magus, levels in at least one other spellcasting class")
        );
        // A row that has a level keeps its prerequisite on the muted line.
        let both = option("A", "core", Some(1), Some("Kinetic blast"));
        assert_eq!(both.aside().as_deref(), Some("Level 1"));
        assert_eq!(
            both.prerequisite_line().as_deref(),
            Some("Prerequisite: Kinetic blast")
        );
        assert_eq!(option("A", "core", None, None).aside(), None);
        assert_eq!(option("A", "core", None, None).prerequisite_line(), None);
    }

    #[test]
    fn the_name_book_and_level_filters_combine() {
        let rows = vec![
            option("Bleeding Attack", "core", Some(1), None),
            option("Black Market", "apg", Some(2), None),
            option("Combat Trick", "core", Some(2), None),
        ];
        let choice = |find: &str, source: Option<&str>, level| OptionChoice {
            find: find.into(),
            source: source.map(String::from),
            level,
            ..Default::default()
        };
        assert_eq!(narrow(&rows, &OptionChoice::default()), [0, 1, 2]);
        assert_eq!(narrow(&rows, &choice("bl", None, None)), [0, 1]);
        assert_eq!(narrow(&rows, &choice("  BLEED ", None, None)), [0]);
        assert_eq!(narrow(&rows, &choice("", Some("core"), None)), [0, 2]);
        assert_eq!(narrow(&rows, &choice("", None, Some(2))), [1, 2]);
        assert_eq!(
            narrow(&rows, &choice("bl", Some("core"), Some(2))),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn a_menu_counts_its_values_under_the_other_filters_and_keeps_a_chosen_empty_value() {
        let rows = vec![
            option("Bleeding Attack", "core", Some(1), None),
            option("Black Market", "apg", Some(2), None),
            option("Combat Trick", "core", Some(2), None),
        ];
        let choice = OptionChoice {
            find: "bl".into(),
            source: Some("core".into()),
            ..Default::default()
        };
        // Books are counted under the name text and the level, but not under the book itself.
        assert_eq!(
            book_counts(&rows, &choice),
            [
                ("apg".to_string(), "Book apg".to_string(), 1),
                ("core".to_string(), "Book core".to_string(), 1),
            ]
        );
        // Levels are counted under the name text and the book.
        assert_eq!(level_counts(&rows, &choice), [(1, 1), (2, 0)]);
        // A group whose rows print no level has no level menu.
        let no_levels = vec![option("A", "core", None, None)];
        assert!(level_counts(&no_levels, &OptionChoice::default()).is_empty());
    }

    #[test]
    fn the_route_names_the_group_and_an_unknown_one_shows_the_first() {
        let groups = vec!["rogue-talent", "advanced-talent"];
        let by = |wanted| shown_group(&groups, |slug| slug, wanted).copied();
        assert_eq!(by(Some("advanced-talent")), Some("advanced-talent"));
        assert_eq!(by(Some("nope")), Some("rogue-talent"));
        assert_eq!(by(None), Some("rogue-talent"));
        assert_eq!(shown_group(&Vec::<&str>::new(), |s| s, None), None);
    }
}
