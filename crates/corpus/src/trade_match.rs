//! Resolving the printed name of a trade to a feature of a base class, only when it is provable.
//!
//! A wrong link is worse than no link: a compatibility filter (#170) reads a resolved trade as
//! "this archetype takes exactly that feature", and reads an unresolved one as "unknown, never
//! compatible". So every rule here needs the printed name itself to prove the match. A position in
//! a list, the length of a list, a name that merely starts with or contains a feature name, and a
//! feature that two base classes both hold are not proof. They leave the trade unresolved.
//!
//! The rules, in order. Each needs exactly one feature to match:
//!
//! 1. **Exact.** The name, without its ability tag, slugs to the slug of a heading's text
//!    (`Bravery (Ex)` and `Bravery`), or to a heading anchor. An anchor with a number in it
//!    (`trap-sense-3`) may be a suffix that kept two headings apart, so it never proves a name
//!    such as `Trap sense +3`.
//! 2. **Token sets.** The name and the heading hold the same words, in any order, with a plural
//!    read as its singular (`Investigator talents` and `Investigator Talent (Ex or Su)`). Articles
//!    never count. A base class's own name may be left out of the printed name (`The fighter's
//!    bonus feat` for `Bonus Feats`).
//! 3. **Ordinal levels.** The name states class levels as ordinals, with the word `level` or
//!    `levels` (`The discovery gained at 6th level`, `2nd-, 12th-, 16th-level discoveries`). Once
//!    the ordinals and the words that only carry them (`the`, `at`, `gained`, `of`, `and`,
//!    `level`) are set aside, the rest must be the token set of one heading. The levels go to
//!    `levels`, and each must be 1 to 20. Each ordinal must belong to a level expression: a run of
//!    ordinals joined by `and` that `level` or `levels` follows. `2nd favored enemy` is the second
//!    enemy, which a ranger gains at 5th level, so `2nd favored enemy gained at 5th level` mixes
//!    an instance with a level and stays unresolved.
//! 4. **One qualifier.** The name is a feature plus one number (`Poison resistance +4`, `Weapon
//!    training 2`) or one trailing parenthesis of a single word (`Touch treatment (minor)`). The
//!    qualifier goes to `power`. A list of numbers (`Armor training 1, 2, 3, and 4`) cannot be one
//!    `power`, and a longer parenthesis is an editor's note (`Hunter's bond (but see text)`), so
//!    both stay unresolved.
//!
//! A name that adds a word the heading does not have (`Improved evasion` against `Evasion`) fails
//! rule 2, because the sets differ. Two headings of one class with the same words, or one anchor
//! that several base classes hold, make the name ambiguous, and an ambiguous name is unresolved.

use crate::check::{feature_key, strip_ability_tag};
use cast_iron_schema::FeatureRef;
use std::collections::{BTreeMap, BTreeSet};

/// One heading of a base class entry that a trade may name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseHeading {
    /// The id of the class entry that holds the heading.
    pub class: String,
    /// The heading's URL anchor.
    pub key: String,
    /// The heading's plain text, ability tag included.
    pub text: String,
}

/// The rule that proved a match, in the order the rules are tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    Exact,
    Tokens,
    Levels,
    Qualifier,
}

impl Rule {
    pub const ALL: [Rule; 4] = [Rule::Exact, Rule::Tokens, Rule::Levels, Rule::Qualifier];

    pub fn slug(self) -> &'static str {
        match self {
            Rule::Exact => "exact",
            Rule::Tokens => "tokens",
            Rule::Levels => "levels",
            Rule::Qualifier => "qualifier",
        }
    }
}

/// Why a printed name stays unresolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// The entry has no base class entry to hold a feature.
    NoBaseClass,
    /// No heading of a base class has the name's words.
    NoHeading,
    /// Several headings have the name's words (their `class#anchor`).
    AmbiguousHeading(Vec<String>),
    /// One anchor, held by several base classes (their ids), and a written `class` would be
    /// needed.
    AmbiguousClass(Vec<String>),
    /// The name carries a list of numbers, or a number and another qualifier.
    UnusableQualifier,
    /// An ordinal outside levels 1 to 20.
    LevelOutOfRange,
    /// An earlier trade of the same key names the same feature at the same levels and power: two
    /// trades cannot both be the proven reading, so the later one stays unresolved.
    SameFeatureTwice(String),
    /// Ordinals without the word `level`: an instance (`2nd favored enemy`), not a class level.
    OrdinalIsNotALevel,
}

impl Why {
    /// The stable reason column of the review queue.
    pub fn slug(&self) -> &'static str {
        match self {
            Why::NoBaseClass => "no-base-class",
            Why::NoHeading => "no-heading",
            Why::AmbiguousHeading(_) => "ambiguous-heading",
            Why::AmbiguousClass(_) => "ambiguous-class",
            Why::UnusableQualifier => "unusable-qualifier",
            Why::LevelOutOfRange => "level-out-of-range",
            Why::OrdinalIsNotALevel => "ordinal-is-not-a-level",
            Why::SameFeatureTwice(_) => "duplicate-feature",
        }
    }

    /// The candidates the reason names, for the review queue.
    pub fn candidates(&self) -> &[String] {
        match self {
            Why::AmbiguousHeading(list) | Why::AmbiguousClass(list) => list,
            Why::SameFeatureTwice(anchor) => std::slice::from_ref(anchor),
            _ => &[],
        }
    }
}

/// The result of resolving one printed name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Resolved(FeatureRef, Rule),
    Unresolved(Why),
}

/// Words that never count when a name and a heading are compared.
const ARTICLES: [&str; 3] = ["the", "a", "an"];
/// Words that only carry ordinal levels.
const LEVEL_FILLER: [&str; 6] = ["at", "gained", "of", "and", "level", "levels"];

/// A printed name taken apart.
#[derive(Debug, Default, PartialEq, Eq)]
struct Parsed {
    /// Lowercase words, articles removed.
    words: Vec<String>,
    /// Ordinals such as `6th`, as numbers.
    ordinals: Vec<u32>,
    /// Plain or signed numbers such as `+4` or `2`.
    numbers: Vec<String>,
    /// The text of a trailing parenthesis that is not an ability tag.
    paren: Option<String>,
    /// Ordinals and words in print order, articles removed; a number breaks a run.
    sequence: Vec<Piece>,
}

/// One token of a printed name that matters for reading ordinals.
#[derive(Debug, PartialEq, Eq)]
enum Piece {
    Ordinal(u32),
    Word(String),
    Number,
}

/// The ordinals of `sequence`, if every one of them denotes a class level, in print order. An
/// ordinal denotes a level when the run it belongs to (ordinals joined by `and`) is followed by
/// `level` or `levels`: `at 6th, 12th, and 18th levels`, `2nd-level`. An ordinal in any other
/// place counts something else (`2nd favored enemy`), and one such ordinal makes the whole name
/// unreadable.
fn level_ordinals(sequence: &[Piece]) -> Option<Vec<u32>> {
    let mut levels = Vec::new();
    let mut at = 0;
    while at < sequence.len() {
        let mut end = at;
        while end < sequence.len()
            && match &sequence[end] {
                Piece::Ordinal(_) => true,
                Piece::Word(word) => word == "and",
                Piece::Number => false,
            }
        {
            end += 1;
        }
        let run: Vec<u32> = sequence[at..end]
            .iter()
            .filter_map(|piece| match piece {
                Piece::Ordinal(level) => Some(*level),
                _ => None,
            })
            .collect();
        if run.is_empty() {
            at = end.max(at + 1);
            continue;
        }
        let closes = matches!(sequence.get(end), Some(Piece::Word(word)) if word == "level" || word == "levels");
        if !closes {
            return None;
        }
        levels.extend(run);
        at = end;
    }
    Some(levels)
}

/// Reads `6th`, `1st`, `22nd` as a number.
fn ordinal(token: &str) -> Option<u32> {
    let digits = token.trim_end_matches(|ch: char| ch.is_ascii_alphabetic());
    let suffix = &token[digits.len()..];
    if digits.is_empty() || !matches!(suffix, "st" | "nd" | "rd" | "th") {
        return None;
    }
    digits.parse().ok()
}

/// Splits a printed name into words, ordinals, numbers and a trailing parenthesis. `None` when
/// the name has a token that no rule reads (a `+` inside a word).
fn parse(printed: &str) -> Option<Parsed> {
    let mut text = strip_ability_tag(printed).trim().to_string();
    let mut parsed = Parsed::default();
    if text.ends_with(')')
        && let Some(open) = text.rfind('(')
        && open > 0
    {
        let inner = text[open + 1..text.len() - 1].trim().to_string();
        if !inner.is_empty() {
            parsed.paren = Some(inner);
            text.truncate(open);
        }
    }
    let lower = text
        .to_lowercase()
        .replace("\u{2019}s", "")
        .replace("'s", "");
    for token in lower.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '+') {
        if token.is_empty() {
            continue;
        }
        if let Some(level) = ordinal(token) {
            parsed.ordinals.push(level);
            parsed.sequence.push(Piece::Ordinal(level));
        } else if let Some(number) = token.strip_prefix('+') {
            if number.is_empty() || !number.chars().all(|ch| ch.is_ascii_digit()) {
                return None;
            }
            parsed.numbers.push(token.to_string());
            parsed.sequence.push(Piece::Number);
        } else if token.contains('+') {
            return None;
        } else if token.chars().all(|ch| ch.is_ascii_digit()) {
            parsed.numbers.push(token.to_string());
            parsed.sequence.push(Piece::Number);
        } else if !ARTICLES.contains(&token) {
            parsed.words.push(token.to_string());
            parsed.sequence.push(Piece::Word(token.to_string()));
        }
    }
    Some(parsed)
}

/// A word read as its singular: `discoveries` as `discovery`, `hexes` as `hex`, `feats` as
/// `feat`. Words ending in `ss`, `us` or `is` are left alone.
fn singular(word: &str) -> String {
    let n = word.len();
    if n > 4 && word.ends_with("ies") {
        return format!("{}y", &word[..n - 3]);
    }
    if n > 4
        && ["xes", "ches", "shes", "sses"]
            .iter()
            .any(|end| word.ends_with(end))
    {
        return word[..n - 2].to_string();
    }
    if n > 3 && word.ends_with('s') && !word.ends_with("ss") && !word.ends_with("us") {
        if word.ends_with("is") {
            return word.to_string();
        }
        return word[..n - 1].to_string();
    }
    word.to_string()
}

/// The sorted singular words of a set of words: the "token set" two names are compared by.
fn token_set<'a>(words: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut set: Vec<String> = words.into_iter().map(singular).collect();
    set.sort();
    set
}

/// The token set of a heading's text: no ability tag, no articles, and every other token,
/// numbers and parentheses included, so `Weapon Training 1` is not `Weapon Training`.
fn heading_set(text: &str) -> Vec<String> {
    let lower = strip_ability_tag(text)
        .to_lowercase()
        .replace("\u{2019}s", "")
        .replace("'s", "");
    token_set(
        lower
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|token| !token.is_empty() && !ARTICLES.contains(token)),
    )
}

/// Settles a set of matching headings to one anchor, or says why it cannot. `all` are the
/// headings of every base class.
fn settle(matches: &[&BaseHeading], all: &[BaseHeading]) -> Result<String, Why> {
    let anchors: BTreeSet<&str> = matches.iter().map(|heading| heading.key.as_str()).collect();
    if anchors.len() > 1 {
        let mut found: Vec<String> = matches
            .iter()
            .map(|heading| format!("{}#{}", heading.class, heading.key))
            .collect();
        found.sort();
        found.dedup();
        return Err(Why::AmbiguousHeading(found));
    }
    // Every base class that holds the anchor counts, not only the classes whose heading text
    // matched: `Alchemy` in one class and `Alchemy and Extracts` in another share one anchor,
    // and a trade that names it needs a written `class`, which a migration cannot prove.
    let anchor = matches[0].key.as_str();
    let classes: BTreeSet<&str> = all
        .iter()
        .filter(|heading| heading.key == anchor)
        .map(|heading| heading.class.as_str())
        .collect();
    if classes.len() > 1 {
        return Err(Why::AmbiguousClass(
            classes.iter().map(|class| class.to_string()).collect(),
        ));
    }
    // Two headings of one class never share an anchor, so `anchors` holds exactly one key here.
    Ok(matches[0].key.clone())
}

fn feature(anchor: String) -> FeatureRef {
    FeatureRef {
        anchor,
        ..Default::default()
    }
}

/// Resolves `printed` against the headings of the base classes. `class_words` are the words of
/// the base classes' names (`fighter`, `unchained`, `rogue`).
pub fn resolve(printed: &str, headings: &[BaseHeading], class_words: &BTreeSet<String>) -> Verdict {
    // Rule 1: the whole name is a heading's text, or a heading anchor. An anchor with a number
    // in it proves nothing alone: `trap-sense-3` may be the third `Trap Sense` heading of a page,
    // not `Trap sense +3`. Such a name needs the heading's own text, or it goes to the later
    // rules, which read the number as a qualifier.
    let key = feature_key(printed);
    if !key.is_empty() {
        let numbered = key
            .split('-')
            .any(|part| part.starts_with(|ch: char| ch.is_ascii_digit()));
        let found: Vec<&BaseHeading> = headings
            .iter()
            .filter(|heading| {
                feature_key(&heading.text) == key || (!numbered && heading.key == key)
            })
            .collect();
        if !found.is_empty() {
            return match settle(&found, headings) {
                Ok(anchor) => Verdict::Resolved(feature(anchor), Rule::Exact),
                Err(why) => Verdict::Unresolved(why),
            };
        }
    }
    let Some(parsed) = parse(printed) else {
        return Verdict::Unresolved(Why::NoHeading);
    };
    let words: Vec<&str> = parsed
        .words
        .iter()
        .map(String::as_str)
        .filter(|word| {
            let level_word = !parsed.ordinals.is_empty() && LEVEL_FILLER.contains(word);
            let list_word = parsed.numbers.len() > 1 && *word == "and";
            !level_word && !list_word
        })
        .collect();
    // Rules 2 to 4 match the words. The first attempt keeps a class's name; the second leaves it
    // out, so `Rogue talents` still finds its own heading before `The fighter's bonus feat`
    // drops `fighter`.
    let without_class: Vec<&str> = words
        .iter()
        .copied()
        .filter(|word| !class_words.contains(*word))
        .collect();
    let mut found: Vec<&BaseHeading> = Vec::new();
    for attempt in [&words, &without_class] {
        if attempt.is_empty() {
            continue;
        }
        let wanted = token_set(attempt.iter().copied());
        found = headings
            .iter()
            .filter(|heading| heading_set(&heading.text) == wanted)
            .collect();
        if !found.is_empty() {
            break;
        }
    }
    if found.is_empty() {
        return Verdict::Unresolved(Why::NoHeading);
    }
    let anchor = match settle(&found, headings) {
        Ok(anchor) => anchor,
        Err(why) => return Verdict::Unresolved(why),
    };
    let mut resolved = feature(anchor);
    let rule = if !parsed.ordinals.is_empty() {
        if !parsed.numbers.is_empty() || parsed.paren.is_some() {
            return Verdict::Unresolved(Why::UnusableQualifier);
        }
        let Some(mut levels) = level_ordinals(&parsed.sequence) else {
            return Verdict::Unresolved(Why::OrdinalIsNotALevel);
        };
        levels.sort_unstable();
        levels.dedup();
        if levels.iter().any(|level| !(1..=20).contains(level)) {
            return Verdict::Unresolved(Why::LevelOutOfRange);
        }
        resolved.levels = levels.iter().map(|level| *level as u8).collect();
        Rule::Levels
    } else if !parsed.numbers.is_empty() || parsed.paren.is_some() {
        match (parsed.numbers.as_slice(), &parsed.paren) {
            ([number], None) => resolved.power = Some(number.clone()),
            // Only a one-word parenthesis names a part of the feature; a longer one is a note.
            ([], Some(inner)) if inner.chars().all(|ch| ch.is_ascii_alphabetic()) => {
                resolved.power = Some(inner.clone());
            }
            _ => return Verdict::Unresolved(Why::UnusableQualifier),
        }
        Rule::Qualifier
    } else {
        Rule::Tokens
    };
    Verdict::Resolved(resolved, rule)
}

/// Whether two features of one entry name the same thing: one anchor (in one class, when both
/// name a class), levels that share a level (no levels means every level), and one power.
pub fn overlaps(a: &FeatureRef, b: &FeatureRef) -> bool {
    if a.anchor != b.anchor || a.power != b.power {
        return false;
    }
    if let (Some(x), Some(y)) = (&a.class, &b.class)
        && x != y
    {
        return false;
    }
    a.levels.is_empty()
        || b.levels.is_empty()
        || a.levels.iter().any(|level| b.levels.contains(level))
}

/// The anchors of every base class, grouped by anchor: which classes hold each one. Used to
/// check that a feature a trade already names is held by one base class.
pub fn holders(headings: &[BaseHeading]) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut by_anchor: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for heading in headings {
        by_anchor
            .entry(heading.key.as_str())
            .or_default()
            .insert(heading.class.as_str());
    }
    by_anchor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(class: &str, key: &str, text: &str) -> BaseHeading {
        BaseHeading {
            class: class.into(),
            key: key.into(),
            text: text.into(),
        }
    }

    fn words(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|word| word.to_string()).collect()
    }

    fn fighter() -> Vec<BaseHeading> {
        let class = "ref/class/fighter";
        vec![
            heading(class, "bravery", "Bravery (Ex)"),
            heading(class, "bonus-feats", "Bonus Feats"),
            heading(class, "weapon-training", "Weapon Training (Ex)"),
            heading(class, "armor-training", "Armor Training (Ex)"),
            heading(class, "evasion", "Evasion"),
            heading(class, "poison-resistance", "Poison Resistance (Ex)"),
            heading(class, "touch-treatment", "Touch Treatment (Su)"),
            heading(
                class,
                "weapon-and-armor-proficiencies",
                "Weapon and Armor Proficiencies",
            ),
        ]
    }

    fn resolved(printed: &str) -> Option<(String, Rule, Vec<u8>, Option<String>)> {
        match resolve(printed, &fighter(), &words(&["fighter"])) {
            Verdict::Resolved(feature, rule) => {
                Some((feature.anchor, rule, feature.levels, feature.power))
            }
            Verdict::Unresolved(_) => None,
        }
    }

    fn why(printed: &str, headings: &[BaseHeading]) -> Why {
        match resolve(printed, headings, &words(&["fighter"])) {
            Verdict::Unresolved(why) => why,
            Verdict::Resolved(feature, _) => panic!("{printed} resolved to {feature:?}"),
        }
    }

    #[test]
    fn an_exact_name_resolves_by_its_slug_and_ignores_the_ability_tag() {
        assert_eq!(
            resolved("Bravery (Ex)"),
            Some(("bravery".into(), Rule::Exact, vec![], None))
        );
        assert_eq!(
            resolved("bravery"),
            Some(("bravery".into(), Rule::Exact, vec![], None))
        );
        // The heading's text matches even when its explicit anchor differs.
        let renamed = [heading("ref/class/x", "sec-x", "Trap Sense (Ex)")];
        let found = resolve("Trap sense", &renamed, &words(&["x"]));
        assert_eq!(
            found,
            Verdict::Resolved(feature("sec-x".into()), Rule::Exact)
        );
    }

    #[test]
    fn a_plural_and_a_class_name_resolve_by_token_set() {
        assert_eq!(
            resolved("Bonus feat"),
            Some(("bonus-feats".into(), Rule::Tokens, vec![], None))
        );
        assert_eq!(
            resolved("Armor and weapon proficiency"),
            Some((
                "weapon-and-armor-proficiencies".into(),
                Rule::Tokens,
                vec![],
                None
            ))
        );
        assert_eq!(
            resolved("The fighter's bonus feats"),
            Some(("bonus-feats".into(), Rule::Tokens, vec![], None))
        );
    }

    #[test]
    fn ordinal_levels_resolve_to_the_anchor_and_its_levels() {
        assert_eq!(
            resolved("The bonus feat gained at 6th level"),
            Some(("bonus-feats".into(), Rule::Levels, vec![6], None))
        );
        assert_eq!(
            resolved("2nd-, 12th-, 16th-level bonus feats"),
            Some(("bonus-feats".into(), Rule::Levels, vec![2, 12, 16], None))
        );
        assert_eq!(
            resolved("The fighter's 1st-level bonus feat"),
            Some(("bonus-feats".into(), Rule::Levels, vec![1], None))
        );
        // Levels are sorted and unique.
        assert_eq!(
            resolved("Bonus feats at 18th, 6th and 6th levels"),
            Some(("bonus-feats".into(), Rule::Levels, vec![6, 18], None))
        );
    }

    #[test]
    fn levels_outside_one_to_twenty_stay_unresolved() {
        assert_eq!(
            why("The bonus feat gained at 21st level", &fighter()),
            Why::LevelOutOfRange
        );
        assert_eq!(
            why("Bonus feat at 0th level", &fighter()),
            Why::LevelOutOfRange
        );
    }

    #[test]
    fn one_number_or_one_parenthesis_is_a_qualifier_and_goes_to_power() {
        assert_eq!(
            resolved("Poison resistance +4"),
            Some((
                "poison-resistance".into(),
                Rule::Qualifier,
                vec![],
                Some("+4".into())
            ))
        );
        assert_eq!(
            resolved("Weapon training 2"),
            Some((
                "weapon-training".into(),
                Rule::Qualifier,
                vec![],
                Some("2".into())
            ))
        );
        assert_eq!(
            resolved("Touch treatment (minor)"),
            Some((
                "touch-treatment".into(),
                Rule::Qualifier,
                vec![],
                Some("minor".into())
            ))
        );
        assert_eq!(
            resolved("Touch treatment (minor) (Su)"),
            Some((
                "touch-treatment".into(),
                Rule::Qualifier,
                vec![],
                Some("minor".into())
            ))
        );
    }

    #[test]
    fn a_note_in_parentheses_is_not_a_qualifier() {
        for name in [
            "Hunter's bond (but see text)",
            "Alchemy (including extracts)",
            "Touch treatment (break enchantment)",
            "Weapon training (1st)",
        ] {
            assert!(
                matches!(
                    why(name, &fighter_plus()),
                    Why::UnusableQualifier | Why::OrdinalIsNotALevel
                ),
                "{name}"
            );
        }
    }

    fn fighter_plus() -> Vec<BaseHeading> {
        let mut headings = fighter();
        headings.push(heading(
            "ref/class/fighter",
            "hunter-s-bond",
            "Hunter's Bond (Ex)",
        ));
        headings.push(heading("ref/class/fighter", "alchemy", "Alchemy (Su)"));
        headings
    }

    #[test]
    fn an_ordinal_without_the_word_level_is_not_a_class_level() {
        let favored = [heading(
            "ref/class/ranger",
            "favored-enemy",
            "Favored Enemy (Ex)",
        )];
        for name in [
            "2nd favored enemy",
            "5th favored enemies",
            "Favored enemy 3rd",
        ] {
            assert_eq!(
                match resolve(name, &favored, &words(&["ranger"])) {
                    Verdict::Unresolved(why) => why,
                    Verdict::Resolved(f, _) => panic!("{name} resolved to {f:?}"),
                },
                Why::OrdinalIsNotALevel,
                "{name}"
            );
        }
        assert!(matches!(
            resolve("The favored enemy gained at 5th level", &favored, &words(&["ranger"])),
            Verdict::Resolved(f, Rule::Levels) if f.levels == [5]
        ));
        assert!(matches!(
            resolve("5th-level favored enemy", &favored, &words(&["ranger"])),
            Verdict::Resolved(f, Rule::Levels) if f.levels == [5]
        ));
    }

    #[test]
    fn a_list_of_numbers_or_mixed_qualifiers_stay_unresolved() {
        for name in [
            "Armor training 1, 2, 3, and 4",
            "Weapon training 3 and 4",
            "Poison resistance +2, +4, and +6",
            "Weapon training 2 (greater)",
            "Weapon training 2 (but see text)",
        ] {
            assert_eq!(why(name, &fighter()), Why::UnusableQualifier, "{name}");
        }
        assert_eq!(
            why("Bonus feat 2 gained at 4th level", &fighter()),
            Why::UnusableQualifier
        );
    }

    #[test]
    fn a_prefix_or_a_longer_name_never_resolves_to_the_shorter_feature() {
        for name in [
            "Improved evasion",
            "Evasion and uncanny dodge",
            "Bravery bonus",
            "Greater weapon training",
            "Poison immunity",
            "Spellcasting",
        ] {
            assert_eq!(why(name, &fighter()), Why::NoHeading, "{name}");
        }
        // The shorter heading does not claim the longer name's prefix either.
        let longer = [
            heading("ref/class/x", "evasion", "Evasion"),
            heading("ref/class/x", "improved-evasion", "Improved Evasion"),
        ];
        assert!(matches!(
            resolve("Improved evasion", &longer, &words(&["x"])),
            Verdict::Resolved(f, Rule::Exact) if f.anchor == "improved-evasion"
        ));
        assert!(matches!(
            resolve("Evasion", &longer, &words(&["x"])),
            Verdict::Resolved(f, Rule::Exact) if f.anchor == "evasion"
        ));
    }

    #[test]
    fn an_anchor_that_two_base_classes_hold_is_ambiguous() {
        let two = [
            heading("ref/class/alchemist", "alchemy", "Alchemy (Su)"),
            heading("ref/class/investigator", "alchemy", "Alchemy (Su)"),
        ];
        for name in ["Alchemy", "alchemy (Su)", "Alchemies"] {
            assert_eq!(
                why(name, &two),
                Why::AmbiguousClass(vec![
                    "ref/class/alchemist".into(),
                    "ref/class/investigator".into()
                ]),
                "{name}"
            );
        }
    }

    #[test]
    fn an_anchor_is_ambiguous_when_any_base_class_holds_it_under_another_text() {
        // `Alchemies` token-matches only the first heading, but the anchor is also held by the
        // second class, so a trade that names it needs a `class`.
        let two = [
            heading("ref/class/alchemist", "alchemy", "Alchemy (Su)"),
            heading("ref/class/investigator", "alchemy", "Alchemy and Extracts"),
        ];
        for name in ["Alchemies", "Alchemy"] {
            assert_eq!(
                why(name, &two),
                Why::AmbiguousClass(vec![
                    "ref/class/alchemist".into(),
                    "ref/class/investigator".into()
                ]),
                "{name}"
            );
        }
    }

    #[test]
    fn a_numeric_anchor_suffix_does_not_prove_a_qualifier() {
        // The second `Trap Sense` heading got the anchor `trap-sense-3`; `Trap sense +3` is the
        // +3 step, not that heading.
        let twin = [
            heading("ref/class/rogue", "trap-sense", "Trap Sense (Ex)"),
            heading("ref/class/rogue", "trap-sense-3", "Trap Sense (Ex)"),
        ];
        assert!(matches!(
            why("Trap sense +3", &twin),
            Why::AmbiguousHeading(_)
        ));
        // With one such heading the name resolves by its words, as a qualifier, never as Exact.
        let one = [heading(
            "ref/class/rogue",
            "trap-sense-3",
            "Trap Sense (Ex)",
        )];
        let found = resolve("Trap sense +3", &one, &words(&["rogue"]));
        assert!(matches!(
            found,
            Verdict::Resolved(f, Rule::Qualifier) if f.anchor == "trap-sense-3" && f.power.as_deref() == Some("+3")
        ));
        // An explicit numeric anchor on another text is not proof either.
        let explicit = [heading(
            "ref/class/fighter",
            "weapon-training-2",
            "Weapon Training (Ex)",
        )];
        assert!(matches!(
            resolve("Weapon training 2", &explicit, &words(&["fighter"])),
            Verdict::Resolved(_, Rule::Qualifier)
        ));
        // A heading whose own text carries the number is exact.
        let own = [heading(
            "ref/class/fighter",
            "weapon-training-2",
            "Weapon Training 2",
        )];
        assert!(matches!(
            resolve("Weapon training 2", &own, &words(&["fighter"])),
            Verdict::Resolved(_, Rule::Exact)
        ));
    }

    #[test]
    fn every_ordinal_must_belong_to_a_level_expression() {
        let favored = [heading(
            "ref/class/ranger",
            "favored-enemy",
            "Favored Enemy (Ex)",
        )];
        let ranger = words(&["ranger"]);
        for name in [
            "2nd favored enemy gained at 5th level",
            "Favored enemy 2nd, gained at 5th level",
        ] {
            assert!(
                matches!(
                    resolve(name, &favored, &ranger),
                    Verdict::Unresolved(Why::OrdinalIsNotALevel)
                ),
                "{name}"
            );
        }
        // Mixed with a repeated word the token set differs, so it stays unresolved anyway.
        assert!(matches!(
            resolve(
                "1st favored enemy and the 5th-level favored enemy",
                &favored,
                &ranger
            ),
            Verdict::Unresolved(_)
        ));
        for (name, levels) in [
            ("Favored enemy gained at 5th level", vec![5]),
            ("Favored enemy at 1st level and at 6th level", vec![1, 6]),
            (
                "Favored enemies gained at 5th, 10th, and 15th levels",
                vec![5, 10, 15],
            ),
            ("1st-level and 5th-level favored enemy", vec![1, 5]),
        ] {
            assert!(
                matches!(
                    resolve(name, &favored, &ranger),
                    Verdict::Resolved(f, Rule::Levels) if f.levels == levels
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn two_headings_with_the_same_words_are_ambiguous() {
        // One class prints `Spell Mastery` twice: the second heading's anchor takes a suffix.
        let twin = [
            heading("ref/class/wizard", "spell-mastery", "Spell Mastery"),
            heading("ref/class/wizard", "spell-mastery-2", "Spell Mastery"),
        ];
        assert_eq!(
            why("Spell mastery", &twin),
            Why::AmbiguousHeading(vec![
                "ref/class/wizard#spell-mastery".into(),
                "ref/class/wizard#spell-mastery-2".into()
            ])
        );
        assert!(matches!(
            why("Spell masteries", &twin),
            Why::AmbiguousHeading(_)
        ));
    }

    #[test]
    fn a_class_name_is_dropped_only_when_the_name_has_no_heading_with_it() {
        let rogue = [
            heading("ref/class/rogue", "rogue-talents", "Rogue Talents"),
            heading("ref/class/rogue", "sneak-attack", "Sneak Attack"),
        ];
        // `rogue` belongs to the heading, so the first attempt keeps it.
        let found = resolve(
            "Rogue talent gained at 2nd level",
            &rogue,
            &words(&["rogue"]),
        );
        assert!(matches!(
            found,
            Verdict::Resolved(f, Rule::Levels) if f.anchor == "rogue-talents" && f.levels == [2]
        ));
        // Dropping `rogue` must not make `Rogue sneak attack` a `Sneak Attack`.
        let found = resolve("Rogue sneak attack", &rogue, &words(&["rogue"]));
        assert!(matches!(
            found,
            Verdict::Resolved(f, Rule::Tokens) if f.anchor == "sneak-attack"
        ));
    }

    #[test]
    fn the_words_of_a_name_are_not_proof_when_a_token_cannot_be_read() {
        assert_eq!(why("Brave+ry", &fighter()), Why::NoHeading);
        assert_eq!(why("", &fighter()), Why::NoHeading);
        assert_eq!(why("(Ex)", &fighter()), Why::NoHeading);
    }

    #[test]
    fn singular_folds_only_plain_plurals() {
        for (plural, word) in [
            ("discoveries", "discovery"),
            ("hexes", "hex"),
            ("feats", "feat"),
            ("bombs", "bomb"),
            ("proficiencies", "proficiency"),
            ("bonus", "bonus"),
            ("mass", "mass"),
            ("analysis", "analysis"),
        ] {
            assert_eq!(singular(plural), word, "{plural}");
        }
    }
}
