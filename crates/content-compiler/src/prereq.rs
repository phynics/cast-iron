//! Reads feat prerequisite wording into requirements.
//!
//! The wording is free text, such as `Dex 13, Dodge, base attack bonus +1` or
//! `Great Fortitude, Iron Will, or Lightning Reflexes`. This module only splits the text and
//! recognizes the numeric kinds of requirement (ability scores, base attack bonus, skill
//! ranks, levels), class features, racial traits, and races. Whatever is left is a
//! [`Reading::Candidate`] name. The caller decides whether the name is a feat; this module
//! never guesses. The original text stays on the entry in every case.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// A name that may be a feat. The caller resolves it against the library.
    Candidate(String),
    AbilityScore,
    BaseAttackBonus,
    SkillRanks,
    Level,
    ClassFeature,
    RacialTrait,
    Race,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The requirement as written, without list punctuation.
    pub text: String,
    pub reading: Reading,
    /// True when the requirement is one of several alternatives in an either-or list.
    pub alternative: bool,
}

/// PF1e races that name a requirement, singular, lowercase. Reviewed against the race
/// entries of the 16 books; a race missing here reads as an unclassified requirement.
const RACES: &[&str] = &[
    "aasimar",
    "catfolk",
    "changeling",
    "dhampir",
    "drow",
    "duergar",
    "dwarf",
    "elf",
    "fetchling",
    "gillman",
    "gnome",
    "goblin",
    "grippli",
    "half-elf",
    "half-orc",
    "halfling",
    "hobgoblin",
    "human",
    "ifrit",
    "kitsune",
    "kobold",
    "merfolk",
    "nagaji",
    "orc",
    "oread",
    "ratfolk",
    "samsaran",
    "strix",
    "suli",
    "svirfneblin",
    "sylph",
    "tengu",
    "tiefling",
    "undine",
    "vanara",
    "vishkanya",
    "wayang",
];

pub fn parse(text: &str) -> Vec<Requirement> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let pieces = split_top_level(&text, |c| c == ',' || c == ';');
    let mut out: Vec<Requirement> = Vec::new();
    let mut leading_or = None;
    for (index, piece) in pieces.iter().enumerate() {
        let piece = piece.trim();
        let (piece, starts_with_or) = match strip_prefix_word(piece, "or") {
            Some(rest) => (rest.trim(), true),
            None => (
                strip_prefix_word(piece, "and").unwrap_or(piece).trim(),
                false,
            ),
        };
        let alternatives = split_or(piece);
        let several = alternatives.len() > 1;
        for part in alternatives {
            if let Some(requirement) = read(&part, several || starts_with_or) {
                out.push(requirement);
            }
        }
        if starts_with_or && index > 0 && leading_or.is_none() {
            leading_or = Some(index);
        }
    }
    // `A, B, or C` is one either-or list when every item before the `or` is a name.
    if leading_or.is_some() {
        let names_only = out
            .iter()
            .all(|r| matches!(r.reading, Reading::Candidate(_)) || r.alternative);
        if names_only {
            for requirement in &mut out {
                requirement.alternative = true;
            }
        }
    }
    out
}

fn read(part: &str, alternative: bool) -> Option<Requirement> {
    let text = part.trim().trim_end_matches(['.', '*']).trim().to_string();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    let reading = if is_ability_score(&lower) {
        Reading::AbilityScore
    } else if lower.starts_with("base attack bonus") {
        Reading::BaseAttackBonus
    } else if is_skill_ranks(&lower) {
        Reading::SkillRanks
    } else if is_level(&lower) {
        Reading::Level
    } else if lower.ends_with("class feature") || lower.ends_with("class features") {
        Reading::ClassFeature
    } else if lower.ends_with("racial trait") || lower.ends_with("racial traits") {
        Reading::RacialTrait
    } else if RACES.contains(&lower.as_str()) {
        Reading::Race
    } else {
        Reading::Candidate(candidate_name(&text))
    };
    Some(Requirement {
        text,
        reading,
        alternative,
    })
}

/// The name without filler words: `the Dodge feat` is `Dodge`.
fn candidate_name(text: &str) -> String {
    let mut name = text.trim();
    for prefix in ["the", "have", "having", "either", "a", "an"] {
        if let Some(rest) = strip_prefix_word(name, prefix) {
            name = rest.trim();
        }
    }
    let lower = name.to_lowercase();
    for suffix in [" feats", " feat"] {
        if lower.ends_with(suffix) {
            name = name[..name.len() - suffix.len()].trim();
            break;
        }
    }
    name.to_string()
}

fn strip_prefix_word<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let head = text.get(..word.len())?;
    let rest = text.get(word.len()..)?;
    (head.eq_ignore_ascii_case(word) && rest.starts_with(' ')).then_some(rest)
}

fn is_ability_score(lower: &str) -> bool {
    const NAMES: [&str; 12] = [
        "str",
        "dex",
        "con",
        "int",
        "wis",
        "cha",
        "strength",
        "dexterity",
        "constitution",
        "intelligence",
        "wisdom",
        "charisma",
    ];
    let word_end = lower
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(lower.len());
    if !NAMES.contains(&&lower[..word_end]) {
        return false;
    }
    lower[word_end..]
        .trim_start_matches(['.', ' '])
        .starts_with(|c: char| c.is_ascii_digit())
}

fn is_skill_ranks(lower: &str) -> bool {
    let words: Vec<&str> = lower.split(' ').collect();
    words.windows(2).any(|w| {
        (w[0].chars().all(|c| c.is_ascii_digit()) && !w[0].is_empty() && w[1].starts_with("rank"))
            || (w[0] == "rank" && w[1].chars().all(|c| c.is_ascii_digit()) && !w[1].is_empty())
    })
}

fn is_level(lower: &str) -> bool {
    let words: Vec<&str> = lower.split(' ').collect();
    let ordinal = |w: &str| {
        let digits = w.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        !digits.is_empty()
            && digits.chars().all(|c| c.is_ascii_digit())
            && matches!(&w[digits.len()..], "st" | "nd" | "rd" | "th" | "")
    };
    // `6th-level fighter`, `8th level`.
    if words
        .first()
        .is_some_and(|w| w.split_once("-level").is_some_and(|(n, _)| ordinal(n)))
    {
        return true;
    }
    for pair in words.windows(2) {
        // `fighter level 4th`, `character level 7`.
        if pair[0] == "level" && ordinal(pair[1]) {
            return true;
        }
        // `3rd mythic tier`, `3rd level`.
        if ordinal(pair[0]) && matches!(pair[1], "level" | "tier") {
            return true;
        }
    }
    words.windows(3).any(|w| ordinal(w[0]) && w[2] == "tier")
}

/// Splits on a separator outside parentheses and brackets.
fn split_top_level(text: &str, is_separator: impl Fn(char) -> bool) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    for c in text.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ => {}
        }
        if depth == 0 && is_separator(c) {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    parts
}

/// Splits `A or B` outside parentheses. The word must stand alone.
fn split_or(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ => {}
        }
        let is_or = depth == 0
            && c == ' '
            && bytes
                .get(i + 1)
                .is_some_and(|c| c.eq_ignore_ascii_case(&'o'))
            && bytes
                .get(i + 2)
                .is_some_and(|c| c.eq_ignore_ascii_case(&'r'))
            && bytes.get(i + 3) == Some(&' ');
        if is_or {
            parts.push(std::mem::take(&mut current));
            i += 4;
        } else {
            current.push(c);
            i += 1;
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readings(text: &str) -> Vec<(String, String, bool)> {
        parse(text)
            .into_iter()
            .map(|r| {
                let kind = match r.reading {
                    Reading::Candidate(name) => format!("name:{name}"),
                    other => format!("{other:?}"),
                };
                (r.text, kind, r.alternative)
            })
            .collect()
    }

    #[test]
    fn reads_the_numeric_requirements_of_a_core_feat() {
        // Improved Bull Rush, Core Rulebook.
        assert_eq!(
            readings("Str 13, Power Attack, base attack bonus +1."),
            [
                ("Str 13".into(), "AbilityScore".into(), false),
                ("Power Attack".into(), "name:Power Attack".into(), false),
                (
                    "base attack bonus +1".into(),
                    "BaseAttackBonus".into(),
                    false
                ),
            ]
        );
    }

    #[test]
    fn reads_skills_levels_races_and_features() {
        // Brewmaster, Advanced Race Guide.
        let brewmaster = readings("Craft (alchemy) 1 rank, Profession (brewer) 1 rank, dwarf.");
        assert_eq!(brewmaster[0].1, "SkillRanks");
        assert_eq!(brewmaster[1].1, "SkillRanks");
        assert_eq!(brewmaster[2].1, "Race");
        // Cloud Step, Ultimate Combat.
        assert_eq!(readings("Spider Step, monk level 12th.")[1].1, "Level");
        assert_eq!(readings("6th-level fighter.")[0].1, "Level");
        // Extra Hex.
        assert_eq!(readings("Hex class feature.")[0].1, "ClassFeature");
        // Eagle Eyes.
        assert_eq!(readings("keen senses racial trait.")[0].1, "RacialTrait");
        assert_eq!(readings("3rd mythic tier")[0].1, "Level");
    }

    #[test]
    fn an_either_or_list_marks_every_name_as_an_alternative() {
        // Marked for Glory, Mythic.
        let list = readings("Great Fortitude, Iron Will, or Lightning Reflexes.");
        assert_eq!(list.len(), 3);
        assert!(
            list.iter()
                .all(|(_, kind, alt)| kind.starts_with("name:") && *alt)
        );
        // Ironhide: one list of races inside a longer requirement.
        let ironhide = readings("Con 13; dwarf, half-orc, or orc.");
        assert_eq!(ironhide[0], ("Con 13".into(), "AbilityScore".into(), false));
        assert_eq!(ironhide[1].1, "Race");
        // No Name: the alternatives are inside one comma item.
        let mixed = readings("Grit class feature or Amateur Gunslinger feat, Bluff 4 ranks.");
        assert_eq!(
            mixed[0],
            ("Grit class feature".into(), "ClassFeature".into(), true)
        );
        assert_eq!(mixed[1].1, "name:Amateur Gunslinger");
        assert!(mixed[1].2);
        assert_eq!(
            mixed[2],
            ("Bluff 4 ranks".into(), "SkillRanks".into(), false)
        );
    }

    #[test]
    fn keeps_names_with_parentheses_whole() {
        let parts =
            readings("Weapon Focus (any slashing weapon), Weapon Specialization (longsword).");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].1, "name:Weapon Focus (any slashing weapon)");
    }

    #[test]
    fn strips_filler_words_around_a_name() {
        assert_eq!(readings("the Dodge feat")[0].1, "name:Dodge");
        assert_eq!(readings("Elemental Fist**")[0].1, "name:Elemental Fist");
        assert!(parse("").is_empty());
        assert!(parse(" . ").is_empty());
    }
}
