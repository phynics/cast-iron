//! The printed trade sentence of a feature (corpus format 4, #540), and the lead that states
//! its level.
//!
//! A book prints a trade as a sentence at the end of the feature: "This ability replaces
//! bravery.", "This ability alters weapon training.", "This ability replaces armor training and
//! bravery.", "This replaces the 1st-, 2nd- and 3rd-level bonus feats." An entry with `features`
//! keeps those sentences as the only statement of its trades in the body, and the drift check
//! compares them with the typed `trades`. This module reads the sentences; it holds no corpus
//! state, so every wording is a native unit test.
//!
//! A name is compared as words: case, punctuation, hyphens, articles, ability tags (`(Ex)`) and a
//! plural `s` do not matter, and a possessive word before a name ("the rogue's trapfinding") is
//! dropped. A hyphen-suspended list ("1st-, 2nd- and 3rd-level bonus feats") reads as one name per
//! level ("1st-level bonus feats", ...), and numbered steps ("armor training 1, 2, 3, and 4") as
//! one name per step. A trade may name one item of a sentence or the whole phrase.

use cast_iron_schema::TradeOp;

/// One printed trade sentence: the verb and the phrase after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeSentence {
    pub op: TradeOp,
    /// What follows the verb, as printed, without the final period.
    pub phrase: String,
}

/// The sentences of `text`, split at a period that ends a word.
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        current.push(*ch);
        let ends = matches!(ch, '.' | '!' | '?')
            && chars.get(index + 1).is_none_or(|next| next.is_whitespace());
        if ends {
            out.push(std::mem::take(&mut current));
        }
    }
    out.push(current);
    out.into_iter()
        .map(|sentence| sentence.trim().to_string())
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

fn verb_of(word: &str) -> Option<TradeOp> {
    match word.to_lowercase().as_str() {
        "replaces" | "replace" => Some(TradeOp::Replaces),
        "alters" | "alter" => Some(TradeOp::Alters),
        _ => None,
    }
}

/// The words that may stand between the opening `This` or `These` and the verb of a trade
/// sentence: "This ability replaces", "This class feature also replaces", "These abilities
/// replace". Any other subject ("This bonus replaces the usual bonus") is prose, not a trade.
const SUBJECT_WORDS: [&str; 16] = [
    "ability",
    "abilities",
    "class",
    "feature",
    "features",
    "power",
    "powers",
    "archetype",
    "also",
    // "This ability otherwise functions like and replaces the standard witch familiar."
    "otherwise",
    "acts",
    "works",
    "functions",
    "as",
    "like",
    "and",
];

fn is_negation(word: &str) -> bool {
    let word = word.to_lowercase();
    matches!(word.as_str(), "not" | "never" | "cannot" | "no") || word.ends_with("n't")
}

/// The trade sentences in `text` (one paragraph or more). A trade sentence starts with `This`,
/// `These` or `It`, then only subject words (`ability`, `class feature`, `also`, ...), then the
/// verb `replaces` or `alters`. A sentence that only mentions the word ("The bonus replaces the
/// usual bonus."), has another subject ("This bonus replaces ...") or is negated ("This ability
/// does not replace ...") is not one. A sentence with two verbs ("This ability replaces X and
/// alters Y.") gives two entries.
pub fn trade_sentences(text: &str) -> Vec<TradeSentence> {
    let mut found = Vec::new();
    for sentence in sentences(text) {
        let sentence = sentence.trim_end_matches(['.', '!', '?']).trim();
        // A trailing clause after a semicolon is not part of the name list.
        let sentence = sentence.split(';').next().unwrap_or(sentence).trim();
        let words: Vec<&str> = sentence.split_whitespace().collect();
        let Some(first) = words.first().map(|word| word.to_lowercase()) else {
            continue;
        };
        if !matches!(first.as_str(), "this" | "these" | "it") {
            continue;
        }
        // The verb is the first word that is one; everything before it must be a plain subject.
        let Some(at) = words.iter().position(|word| verb_of(word).is_some()) else {
            continue;
        };
        let plain_subject = words[1..at]
            .iter()
            .all(|word| SUBJECT_WORDS.contains(&word.to_lowercase().as_str()));
        if !plain_subject || words[..at].iter().any(|word| is_negation(word)) {
            continue;
        }
        // Split the rest at every further verb joined by "and".
        let mut op = verb_of(words[at]).expect("a verb was found");
        let mut phrase: Vec<&str> = Vec::new();
        let mut index = at + 1;
        while index < words.len() {
            let word = words[index];
            let joined_verb = word.eq_ignore_ascii_case("and")
                && words
                    .get(index + 1)
                    .and_then(|next| verb_of(next))
                    .is_some();
            if joined_verb {
                found.push(TradeSentence {
                    op,
                    phrase: phrase.join(" "),
                });
                op = verb_of(words[index + 1]).expect("a verb was found");
                phrase.clear();
                index += 2;
                continue;
            }
            phrase.push(word);
            index += 1;
        }
        found.push(TradeSentence {
            op,
            phrase: phrase.join(" "),
        });
    }
    found.retain(|sentence| !sentence.phrase.trim().is_empty());
    found
}

/// The level in an "At Nth level" or "Starting at Nth level" lead at the start of `text`.
pub fn level_lead(text: &str) -> Option<u8> {
    let lower = text.trim_start().to_lowercase();
    let rest = ["starting at ", "beginning at ", "starting with ", "at "]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix))?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let after = &rest[digits.len()..];
    let after = ["st", "nd", "rd", "th"]
        .iter()
        .find_map(|suffix| after.strip_prefix(suffix))?;
    // `level` ends the word: "level", "level,", "levels" is not a lead ("at 5 levels").
    let after = after.trim_start();
    let word = after.split(|ch: char| !ch.is_alphabetic()).next()?;
    (word == "level").then_some(())?;
    digits.parse().ok()
}

const CONNECTORS: [&str; 2] = ["and", "or"];
const ARTICLES: [&str; 9] = [
    "the", "a", "an", "its", "his", "her", "their", "all", "standard",
];

/// The words of a printed name or phrase, normalized for comparison.
pub fn name_words(text: &str) -> Vec<String> {
    expand_lists(&strip_possessives(&strip_tags(text)))
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .filter(|word| !ARTICLES.contains(&word.as_str()))
        .map(|word| singular(&word))
        .collect()
}

/// Drops a possessive word before a name: "the rogue's trapfinding" reads as "trapfinding", the
/// way a trade names it. A name that holds a possessive ("Hunter's bond") drops it on both sides
/// of the comparison, so the two still agree.
fn strip_possessives(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| {
            let word = word
                .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '\'' && ch != '\u{2019}');
            !(word.ends_with("'s") || word.ends_with("\u{2019}s"))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Removes `(Ex)`, `(Su)`, `(Sp)` and `(Ex or Sp)`: a parenthesis that holds only ability tags.
fn strip_tags(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')') else {
            break;
        };
        let inner = &rest[open + 1..open + close];
        let is_tag = !inner.trim().is_empty()
            && inner
                .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '/'))
                .filter(|token| !token.is_empty())
                .all(|token| {
                    matches!(
                        token.to_lowercase().as_str(),
                        "ex" | "su" | "sp" | "or" | "and"
                    )
                });
        out.push_str(&rest[..open]);
        if is_tag {
            out.push(' ');
        } else {
            out.push_str(&rest[open..=open + close]);
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// Expands the two printed shorthands of a list of names, so each name stands alone:
///
/// - a hyphen-suspended list: "1st-, 2nd- and 3rd-level bonus feats" becomes "1st-level bonus
///   feats, 2nd-level bonus feats, 3rd-level bonus feats";
/// - numbered steps after a shared head: "armor training 1, 2, 3, and 4" becomes "armor training
///   1, armor training 2, armor training 3, armor training 4".
///
/// Text with neither is returned as it is.
fn expand_lists(text: &str) -> String {
    // Items end at a comma, "and" or "or".
    let mut items: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for raw in text.split_whitespace() {
        let word = raw.trim_end_matches(',');
        if CONNECTORS.contains(&word.to_lowercase().as_str()) {
            if !current.is_empty() {
                items.push(current.join(" "));
                current.clear();
            }
            continue;
        }
        if !word.is_empty() {
            current.push(word);
        }
        if raw.ends_with(',') && !current.is_empty() {
            items.push(current.join(" "));
            current.clear();
        }
    }
    if !current.is_empty() {
        items.push(current.join(" "));
    }
    let suspended = items.iter().any(|item| item.ends_with('-'));
    let numbered = items.iter().enumerate().any(|(index, item)| {
        step_head(item).is_some() && items.get(index + 1).is_some_and(|next| is_number(next))
    });
    if !suspended && !numbered {
        return text.to_string();
    }
    let tail = items
        .iter()
        .find(|item| !item.ends_with('-') && item.contains('-'))
        .and_then(|item| item.split_once('-').map(|(_, tail)| tail.to_string()));
    let mut head: Option<String> = None;
    let mut out = Vec::new();
    for item in &items {
        if suspended
            && item.ends_with('-')
            && let Some(tail) = &tail
        {
            out.push(format!("{item}{tail}"));
            continue;
        }
        if is_number(item)
            && let Some(head) = &head
        {
            out.push(format!("{head} {item}"));
            continue;
        }
        head = step_head(item).map(str::to_string);
        out.push(item.clone());
    }
    out.join(", ")
}

fn is_number(item: &str) -> bool {
    !item.is_empty() && item.bytes().all(|byte| byte.is_ascii_digit())
}

/// `armor training` of `armor training 1`: a name with a trailing step number.
fn step_head(item: &str) -> Option<&str> {
    let (head, number) = item.rsplit_once(' ')?;
    (is_number(number) && !head.trim().is_empty()).then_some(head)
}

fn singular(word: &str) -> String {
    if word.len() > 4 && word.ends_with("ies") {
        return format!("{}y", &word[..word.len() - 3]);
    }
    if word.len() > 3 && word.ends_with('s') && !word.ends_with("ss") && !word.ends_with("us") {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// True when `phrase` is exactly a run of the names in `names`, each used any number of times,
/// with `and`, `or` and commas between them. `names` are word lists from [`name_words`].
pub fn phrase_is_covered(phrase: &str, names: &[Vec<String>]) -> bool {
    let words = name_words(phrase);
    let mut memo = vec![None; words.len() + 1];
    covered_from(&words, 0, names, &mut memo)
}

fn covered_from(
    words: &[String],
    start: usize,
    names: &[Vec<String>],
    memo: &mut [Option<bool>],
) -> bool {
    if start >= words.len() {
        return true;
    }
    if let Some(known) = memo[start] {
        return known;
    }
    // A connector may stand between two names.
    let skip = usize::from(start > 0 && CONNECTORS.contains(&words[start].as_str()));
    let result = [start, start + skip].into_iter().any(|at| {
        at < words.len()
            && names.iter().any(|name| {
                !name.is_empty()
                    && words[at..].starts_with(name)
                    && covered_from(words, at + name.len(), names, memo)
            })
    });
    memo[start] = Some(result);
    result
}

/// True when `name` occurs as a run of words in `phrase`, as a whole name or as one of the names
/// of a list.
pub fn name_in_phrase(name: &[String], phrase: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let words = name_words(phrase);
    words.windows(name.len()).any(|window| window == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> TradeSentence {
        let found = trade_sentences(text);
        assert_eq!(found.len(), 1, "{text}: {found:?}");
        found.into_iter().next().unwrap()
    }

    #[test]
    fn the_plain_wordings_are_read() {
        let sentence = one(
            "At 2nd level, a two-handed fighter gains a +1 bonus. This ability replaces bravery.",
        );
        assert_eq!(sentence.op, TradeOp::Replaces);
        assert_eq!(sentence.phrase, "bravery");
        let sentence = one("This ability alters weapon training.");
        assert_eq!(sentence.op, TradeOp::Alters);
        assert_eq!(sentence.phrase, "weapon training");
        assert_eq!(one("This replaces bravery (Ex).").phrase, "bravery (Ex)");
        assert_eq!(
            one("These abilities replace armor training.").phrase,
            "armor training"
        );
        assert_eq!(one("It alters bravery").op, TradeOp::Alters);
    }

    #[test]
    fn a_sentence_that_only_mentions_the_verb_is_not_a_trade() {
        assert!(trade_sentences("The bonus replaces the usual bonus on the roll.").is_empty());
        assert!(trade_sentences("A fighter can use this to replace a feat.").is_empty());
        assert!(trade_sentences("This ability grants a bonus on attack rolls.").is_empty());
    }

    #[test]
    fn two_verbs_in_one_sentence_give_two_trades() {
        let found = trade_sentences("This ability replaces bravery and alters armor training.");
        assert_eq!(
            found,
            vec![
                TradeSentence {
                    op: TradeOp::Replaces,
                    phrase: "bravery".into()
                },
                TradeSentence {
                    op: TradeOp::Alters,
                    phrase: "armor training".into()
                }
            ]
        );
    }

    #[test]
    fn several_sentences_in_a_section_are_all_read() {
        let found = trade_sentences(
            "This ability replaces bravery. It gives a bonus. This ability alters armor training.",
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].op, TradeOp::Alters);
    }

    #[test]
    fn the_level_lead_reads_both_printed_forms() {
        assert_eq!(
            level_lead("At 2nd level, a fighter gains a bonus."),
            Some(2)
        );
        assert_eq!(level_lead("Starting at 3rd level, a ranger can"), Some(3));
        assert_eq!(level_lead("At 1st level a cleric"), Some(1));
        assert_eq!(level_lead("At 11th level and every 4 levels"), Some(11));
        assert_eq!(level_lead("At 20th level, the monk"), Some(20));
        assert_eq!(level_lead("The fighter gains a bonus at 2nd level."), None);
        assert_eq!(level_lead("At 5 levels beyond"), None);
        assert_eq!(level_lead("At will, the oracle"), None);
    }

    fn names(printed: &[&str]) -> Vec<Vec<String>> {
        printed.iter().map(|name| name_words(name)).collect()
    }

    #[test]
    fn names_ignore_case_tags_articles_and_plurals() {
        assert_eq!(name_words("Bravery (Ex)"), name_words("bravery"));
        assert_eq!(name_words("the Bonus Feats"), name_words("bonus feat"));
        assert_eq!(
            name_words("Weapon Training (Ex or Sp)"),
            name_words("weapon training")
        );
        assert_ne!(name_words("armor training"), name_words("armor"));
        assert!(phrase_is_covered("bravery", &names(&["Bravery (Ex)"])));
    }

    #[test]
    fn a_list_is_covered_by_one_trade_per_item_or_by_one_trade() {
        let two = names(&["Armor training", "Bravery"]);
        assert!(phrase_is_covered("armor training and bravery", &two));
        assert!(phrase_is_covered("armor training, bravery", &two));
        assert!(phrase_is_covered("armor training, and bravery", &two));
        assert!(!phrase_is_covered(
            "armor training and bravery and weapon training",
            &two
        ));
        assert!(!phrase_is_covered("armor training", &names(&["Bravery"])));
        let whole = names(&["Armor training and bravery"]);
        assert!(phrase_is_covered("armor training and bravery", &whole));
    }

    #[test]
    fn a_hyphen_suspended_list_reads_as_one_name_per_level() {
        let phrase = "the 1st-, 2nd- and 3rd-level bonus feats";
        assert_eq!(
            name_words(phrase),
            name_words("1st-level bonus feats, 2nd-level bonus feats, 3rd-level bonus feats")
        );
        let per_level = names(&[
            "1st-level bonus feat",
            "2nd-level bonus feat",
            "3rd-level bonus feat",
        ]);
        assert!(phrase_is_covered(phrase, &per_level));
        assert!(name_in_phrase(&per_level[1], phrase));
        // One trade may state the whole phrase.
        let whole = names(&["1st-, 2nd- and 3rd-level bonus feats"]);
        assert!(phrase_is_covered(phrase, &whole));
        // A missing level is not covered.
        assert!(!phrase_is_covered(
            phrase,
            &names(&["1st-level bonus feat", "2nd-level bonus feat"])
        ));
    }

    #[test]
    fn a_name_with_and_inside_it_is_one_name() {
        let names = names(&["Armor and shield proficiency"]);
        assert!(phrase_is_covered("armor and shield proficiency", &names));
    }

    #[test]
    fn a_name_appears_in_a_phrase_as_a_whole_run_of_words() {
        let phrase = "armor training and bravery";
        assert!(name_in_phrase(&name_words("Bravery"), phrase));
        assert!(name_in_phrase(&name_words("armor training"), phrase));
        assert!(!name_in_phrase(&name_words("armor"), "weapon training"));
        assert!(!name_in_phrase(&[], phrase));
    }

    #[test]
    fn a_negated_sentence_or_another_subject_is_not_a_trade() {
        assert!(trade_sentences("This ability does not replace bravery.").is_empty());
        assert!(trade_sentences("This ability can't replace bravery.").is_empty());
        assert!(trade_sentences("This bonus replaces the usual bonus.").is_empty());
        assert!(trade_sentences("This ability grants bravery. It never alters armor.").is_empty());
        assert_eq!(
            trade_sentences("This class feature also replaces bravery.").len(),
            1
        );
    }

    #[test]
    fn the_printed_variants_of_the_subject_are_read() {
        let sentence =
            one("This ability otherwise functions like and replaces the standard witch familiar.");
        assert_eq!(sentence.op, TradeOp::Replaces);
        assert_eq!(name_words(&sentence.phrase), name_words("Witch familiar"));
        let sentence = one("This otherwise works like and replaces wild shape.");
        assert_eq!(sentence.phrase, "wild shape");
        assert_eq!(
            one("This ability replaces trackless step and woodland stride.").phrase,
            "trackless step and woodland stride"
        );
    }

    #[test]
    fn numbered_steps_read_one_name_per_step() {
        assert_eq!(
            name_words("armor training 1, 2, 3, and 4"),
            name_words("armor training 1, armor training 2, armor training 3, armor training 4")
        );
        let steps = names(&["Armor training 2", "Armor training 3"]);
        assert!(phrase_is_covered("armor training 2 and 3", &steps));
        assert!(!phrase_is_covered("armor training 2, 3 and 4", &steps));
        assert!(name_in_phrase(
            &name_words("Armor training 3"),
            "armor training 1, 2, 3, and 4"
        ));
        // A name with a trailing number and nothing after it is unchanged.
        assert_eq!(name_words("Armor training 1"), ["armor", "training", "1"]);
    }

    #[test]
    fn a_possessive_before_a_name_is_dropped_on_both_sides() {
        assert_eq!(
            name_words("the rogue's trapfinding"),
            name_words("Trapfinding")
        );
        assert_eq!(
            name_words("Hunter's bond"),
            name_words("hunter\u{2019}s bond")
        );
        assert!(phrase_is_covered(
            "the rogue's trapfinding",
            &names(&["Trapfinding"])
        ));
    }
}
