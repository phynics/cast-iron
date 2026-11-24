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
//! one name per step. The words `ability` and `class feature` around a name are dropped too
//! ("the paladin's spells class feature" names `Spells`; `class ability` likewise, but `Class
//! skills` keeps both words). A trade may name one item of a sentence or the whole phrase.
//!
//! The readings of owner ruling 21 of 2026-11-24 (#540) tolerate the printed variants that do not
//! change what a name is:
//!
//! - `normal` and `normally` are determiners like `standard`; `bonus` never is ("bonus spells"
//!   are not "spells");
//! - a spelled ordinal (`first` to `twentieth`) is its numeral ("second favored enemy" is "2nd
//!   favored enemy"); different ordinals stay different;
//! - a class name after an `Nth-level` word ("the 10th-level wizard bonus feat") is an
//!   adjective, an alternative reading beside the one that keeps it; different levels stay
//!   different;
//! - "gun training 1 to 4", "gun training 1-4" and the en dash form are the steps 1 to 4: a head,
//!   a rising range of at most 20 steps;
//! - "as well as" joins two names like `and`;
//! - a possessor phrase before a name ("the sacred shield's holy champion ability": an article,
//!   at most two words and a possessive word) is dropped as an alternative reading, never across
//!   a comma or a connector.
//!
//! The sentence side has four more (see [`trade_sentences_headed`]): the phrase ends at ", but ";
//! a closing parenthesis of four words or more is an aside; a verb joined by `and` to another
//! clause is a bare sentence; and the feature's own heading is a subject. `changes` is an `alters`
//! verb (owner ruling 25), only in the sentence position.
//!
//! Owner ruling 22 (a level back-reference trade is renamed to the printed words) adds the
//! subjects that name a granted thing: `spell`, `skill` and `evolution` join `feat` as subject
//! words that name no ability (the sentence is bare), and `bonus` is a subject word right before a
//! granted noun ("These bonus spells replace the oracle's mystery bonus spells at these levels.",
//! "This bonus feat replaces ..."). "This bonus replaces ..." stays prose.

use cast_iron_schema::TradeOp;

/// One printed trade sentence: the verb and the phrase after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeSentence {
    pub op: TradeOp,
    /// True when the subject names no ability ("This replaces ...", "These replace ..."). A bare
    /// sentence is a trade sentence only when it names a stored trade ([`names_a_trade`]): the
    /// pronoun may stand for anything ("These replace the normal attack roll bonus ...").
    pub bare: bool,
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

/// The verb of the trade-sentence position, the first verb after the subject. Owner ruling 25 of
/// 2026-11-24 (#540): "This ability changes the Arcane Pool class feature" is an `alters`
/// sentence. `changes` is read only here, not as a further verb joined by "and".
fn lead_verb_of(word: &str) -> Option<TradeOp> {
    match word.to_lowercase().as_str() {
        "changes" | "change" => Some(TradeOp::Alters),
        _ => verb_of(word),
    }
}

/// The words that may stand between the opening `This` or `These` and the verb of a trade
/// sentence: "This ability replaces", "This class feature also replaces", "These abilities
/// replace", "This performance replaces". Any other subject ("This bonus replaces the usual
/// bonus") is prose, not a trade.
///
/// The first group names an ability: a noun that a book uses for a class ability or one of its
/// choices (a bard performance, a witch hex, an alchemist discovery, an oracle revelation, a
/// paladin mercy, a gunslinger deed, an arcanist exploit, a magus arcana, a rogue trick or
/// talent). A sentence without one is bare (see [`TradeSentence::bare`]). `performance` is the
/// printed wording of the Advanced Player's Guide bard archetypes ("This performance replaces
/// inspire courage."); the others are the same kind of noun for the other classes' choices. A
/// noun that names no ability ("bonus", "value", "effect") stays out.
const ABILITY_WORDS: [&str; 27] = [
    "ability",
    "abilities",
    "feature",
    "features",
    "power",
    "powers",
    "archetype",
    "performance",
    "performances",
    "trick",
    "tricks",
    "talent",
    "talents",
    "hex",
    "hexes",
    "discovery",
    "discoveries",
    "revelation",
    "revelations",
    "mercy",
    "mercies",
    "deed",
    "deeds",
    "exploit",
    "exploits",
    "arcana",
    "arcanum",
];

/// The other words of a subject: a class qualifier, `also`, the verbs of "otherwise functions like
/// and replaces", and the granted things that do not name an ability: `feat` ("These feats replace
/// the monk's normal bonus feats"), `spell`, `skill` and `evolution`. A sentence with one of them
/// is bare, so it counts only when it names a stored trade.
const FILLER_WORDS: [&str; 17] = [
    "class",
    "also",
    // "This ability otherwise functions like and replaces the standard witch familiar."
    "otherwise",
    "acts",
    "works",
    "functions",
    "as",
    "like",
    "and",
    "feat",
    "feats",
    "spell",
    "spells",
    "skill",
    "skills",
    "evolution",
    "evolutions",
];

/// Owner ruling 22 of 2026-11-24 (#540): the nouns that name what a feature grants. `bonus` is a
/// subject word right before one of them ("These bonus spells replace the oracle's mystery bonus
/// spells at these levels.", "This bonus feat replaces ..."), and only there: "This bonus
/// replaces ..." (a numeric bonus) stays prose.
const GRANTED_WORDS: [&str; 23] = [
    "spell",
    "spells",
    "feat",
    "feats",
    "ability",
    "abilities",
    "power",
    "powers",
    "talent",
    "talents",
    "trick",
    "tricks",
    "hex",
    "hexes",
    "skill",
    "skills",
    "revelation",
    "revelations",
    "arcana",
    "discovery",
    "discoveries",
    "evolution",
    "evolutions",
];

fn is_subject_word(word: &str) -> bool {
    let word = word.to_lowercase();
    ABILITY_WORDS.contains(&word.as_str()) || FILLER_WORDS.contains(&word.as_str())
}

/// The words between `This`/`These` and the verb are all subject words, where `bonus` counts
/// right before a granted noun ([`GRANTED_WORDS`]).
fn is_subject(words: &[&str]) -> bool {
    words.iter().enumerate().all(|(index, word)| {
        is_subject_word(word)
            || (word.eq_ignore_ascii_case("bonus")
                && words
                    .get(index + 1)
                    .is_some_and(|next| GRANTED_WORDS.contains(&next.to_lowercase().as_str())))
    })
}

fn is_negation(word: &str) -> bool {
    let word = word.to_lowercase();
    matches!(word.as_str(), "not" | "never" | "cannot" | "no") || word.ends_with("n't")
}

/// The trade sentences in `text` (one paragraph or more), with no section heading to read a
/// subject from (see [`trade_sentences_headed`]).
#[cfg(test)]
pub fn trade_sentences(text: &str) -> Vec<TradeSentence> {
    trade_sentences_headed(text, None)
}

/// The trade sentences in `text` (one paragraph or more). A trade sentence starts with `This`,
/// `These` or `It`, then only subject words (`ability`, `class feature`, `also`, ...), then the
/// verb `replaces`, `alters` or `changes` (the last reads as `alters`, and only here). A `both`
/// right after the verb is dropped ("This ability replaces both diamond body and quivering palm."
/// names two features). A sentence that only mentions the word ("The bonus replaces the usual
/// bonus."), has another subject ("This bonus replaces ...") or is negated ("This ability does
/// not replace ...") is not one. A sentence with two verbs ("This ability replaces X and alters
/// Y.") gives two entries.
///
/// Four more readings, from owner ruling 21 of 2026-11-24 (#540):
///
/// - `head`, the plain text of the heading of the section, is a subject too: "Sermonic performance
///   replaces the 1st-level channel energy." under that heading is a sentence of an ability (not
///   bare).
/// - A verb joined by `and` to another clause ("This movement provokes attacks of opportunity as
///   normal, and replaces deadly performance.") is read as a bare sentence: it counts only when
///   it names a stored trade.
/// - The sentence ends at ", but ": what follows is a remark about the trade, not a name.
/// - A parenthesis of four words or more that ends the phrase is an aside, not part of the name.
pub fn trade_sentences_headed(text: &str, head: Option<&str>) -> Vec<TradeSentence> {
    let mut found = Vec::new();
    for sentence in sentences(text) {
        let sentence = sentence.trim_end_matches(['.', '!', '?']).trim();
        // A trailing clause after a semicolon is not part of the name list.
        let sentence = sentence.split(';').next().unwrap_or(sentence).trim();
        // R8: ", but ..." after the names is a remark about the trade, not a name.
        let sentence = sentence.split(", but ").next().unwrap_or(sentence).trim();
        let words: Vec<&str> = sentence.split_whitespace().collect();
        let Some(first) = words.first().map(|word| word.to_lowercase()) else {
            continue;
        };
        let Some(at) = words.iter().position(|word| lead_verb_of(word).is_some()) else {
            continue;
        };
        // R12: the feature's own name as the subject ("Sermonic performance replaces ...").
        let head_subject = at > 0
            && head.is_some_and(|head| {
                let head = name_words(head);
                !head.is_empty() && name_words(&words[..at].join(" ")) == head
            });
        if !head_subject && !matches!(first.as_str(), "this" | "these" | "it") {
            continue;
        }
        let plain_subject = head_subject || is_subject(&words[1..at]);
        // R11: "This movement provokes ..., and replaces X": a verb joined by `and` to another
        // clause. Read as bare: it counts only when it names a stored trade.
        let coordinated = !plain_subject
            && at >= 2
            && words[at - 1]
                .trim_end_matches(',')
                .eq_ignore_ascii_case("and");
        // "functions like and replaces X" is a trade; "works like alter self" is the spell.
        let compares = words[1..at]
            .iter()
            .any(|word| matches!(word.to_lowercase().as_str(), "like" | "as"));
        let compares_a_spell = compares && !words[at - 1].eq_ignore_ascii_case("and");
        if (!plain_subject && !coordinated)
            || compares_a_spell
            || words[..at].iter().any(|word| is_negation(word))
        {
            continue;
        }
        let bare = coordinated
            || (!head_subject
                && !words[1..at]
                    .iter()
                    .any(|word| ABILITY_WORDS.contains(&word.to_lowercase().as_str())));
        // Split the rest at every further verb joined by "and".
        let mut op = lead_verb_of(words[at]).expect("a verb was found");
        let mut phrase: Vec<&str> = Vec::new();
        let mut index = at + 1;
        while index < words.len() {
            let word = words[index];
            // "replaces both X and Y": `both` only announces a list of two.
            if phrase.is_empty() && word.eq_ignore_ascii_case("both") {
                index += 1;
                continue;
            }
            let joined_verb = word.eq_ignore_ascii_case("and")
                && words
                    .get(index + 1)
                    .and_then(|next| verb_of(next))
                    .is_some();
            if joined_verb {
                found.push(TradeSentence {
                    op,
                    bare,
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
            bare,
            phrase: phrase.join(" "),
        });
    }
    for sentence in &mut found {
        sentence.phrase = drop_aside(&sentence.phrase);
    }
    found.retain(|sentence| !sentence.phrase.trim().is_empty());
    found
}

/// R9: a parenthesis at the end of the phrase that holds a clause (four words or more) is an
/// aside, not part of a name: "the mutagen class ability (a mindchemist cannot create mutagens
/// unless he selects mutagen as a discovery)". Tags ("(Ex)") and short qualifiers stay.
fn drop_aside(phrase: &str) -> String {
    let trimmed = phrase.trim_end();
    if !trimmed.ends_with(')') {
        return phrase.to_string();
    }
    let Some(open) = trimmed.rfind('(') else {
        return phrase.to_string();
    };
    let inner = &trimmed[open + 1..trimmed.len() - 1];
    if inner.contains('(') || inner.split_whitespace().count() < 4 {
        return phrase.to_string();
    }
    trimmed[..open].trim_end().to_string()
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
const ARTICLES: [&str; 11] = [
    "the", "a", "an", "its", "his", "her", "their", "all", "standard", "normal", "normally",
];

/// The words of a printed name or phrase, normalized for comparison.
pub fn name_words(text: &str) -> Vec<String> {
    // R7: "as well as" joins two names like `and`.
    let text = replace_words(text, "as well as", "and");
    let words: Vec<String> = expand_lists(&strip_possessives(&strip_tags(&text)))
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .filter(|word| !ARTICLES.contains(&word.as_str()))
        .map(|word| ordinal(&word))
        .map(|word| singular(&word))
        .collect();
    // "the paladin's spells class feature" and "the nature sense ability" name `Spells` and
    // `Nature sense`: the words that only say what kind of thing it is are not part of the name.
    let mut kept = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        let noise = word == "ability"
            || word == "feature"
            || (word == "class"
                && words
                    .get(index + 1)
                    .is_some_and(|next| next == "feature" || next == "ability"));
        if !noise {
            kept.push(word.clone());
        }
    }
    if kept.is_empty() { words } else { kept }
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
    let mut ranged = false;
    let items: Vec<String> = items
        .into_iter()
        .flat_map(|item| match expand_range(&item) {
            Some(steps) => {
                ranged = true;
                steps
            }
            None => vec![item],
        })
        .collect();
    let suspended = items.iter().any(|item| item.ends_with('-'));
    // "the 3rd-level and 9th-level mercies": a bare "3rd-level" shares the tail of a later item.
    let shared_tail = |index: usize| -> Option<String> {
        let (word, rest) = ordinal_head(&items[index])?;
        if !rest.is_empty() {
            return None;
        }
        items[index + 1..].iter().find_map(|later| {
            let (later_word, later_rest) = ordinal_head(later)?;
            (later_word == word && !later_rest.is_empty()).then_some(later_rest)
        })
    };
    let shared = (0..items.len()).any(|index| shared_tail(index).is_some());
    let numbered = items.iter().enumerate().any(|(index, item)| {
        step_head(item).is_some() && items.get(index + 1).is_some_and(|next| is_number(next))
    });
    if !suspended && !numbered && !shared && !ranged {
        return text.to_string();
    }
    let tail = items
        .iter()
        .find(|item| !item.ends_with('-') && item.contains('-'))
        .and_then(|item| item.split_once('-').map(|(_, tail)| tail.to_string()));
    let mut head: Option<String> = None;
    let mut out = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if let Some(rest) = shared_tail(index) {
            out.push(format!("{item} {rest}"));
            continue;
        }
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

/// `("level", "mercies")` of `3rd-level mercies`: the word that follows the ordinal's hyphen, and
/// the rest of the item. A leading article does not count.
fn ordinal_head(item: &str) -> Option<(String, String)> {
    let mut tokens = item
        .split_whitespace()
        .skip_while(|token| ARTICLES.contains(&token.to_lowercase().as_str()));
    let first = tokens.next()?;
    let (ordinal, word) = first.split_once('-')?;
    let digits = ordinal.trim_end_matches(|ch: char| ch.is_alphabetic());
    let suffix = &ordinal[digits.len()..];
    let is_ordinal = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && matches!(suffix, "st" | "nd" | "rd" | "th");
    (is_ordinal && !word.is_empty())
        .then(|| (word.to_lowercase(), tokens.collect::<Vec<_>>().join(" ")))
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
    // `hexes`, `witches`, `classes`, `bonuses`: the plural adds `es`.
    if ["xes", "ches", "shes", "sses", "zes"]
        .iter()
        .any(|end| word.ends_with(end))
        || (word.len() > 5 && word.ends_with("uses"))
    {
        return word[..word.len() - 2].to_string();
    }
    if word.len() > 3 && word.ends_with('s') && !word.ends_with("ss") && !word.ends_with("us") {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// True when `phrase` is exactly a run of the names in `names`, each used any number of times,
/// with `and`, `or` and commas between them. `names` are word lists from [`name_words`].
pub fn phrase_is_covered(phrase: &str, names: &[Vec<String>]) -> bool {
    let covers = |text: &str| {
        let words = name_words(text);
        let mut memo = vec![None; words.len() + 1];
        covered_from(&words, 0, names, &mut memo)
    };
    covers(phrase)
        || covers(&strip_class_names(&strip_possessives(phrase)))
        || covers(&strip_class_names(&strip_possessor_phrases(phrase)))
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

/// The base-class names (core, base, hybrid, occult, unchained and NPC classes). A class name
/// that leads a name in a trade sentence is an adjective, not part of the feature's name:
/// "the standard alchemist bomb class feature" names `Bombs`.
const CLASS_NAMES: [&str; 45] = [
    "adept",
    "alchemist",
    "antipaladin",
    "arcanist",
    "aristocrat",
    "barbarian",
    "bard",
    "bloodrager",
    "brawler",
    "cavalier",
    "cleric",
    "commoner",
    "druid",
    "expert",
    "fighter",
    "gunslinger",
    "hunter",
    "inquisitor",
    "investigator",
    "kineticist",
    "magus",
    "medium",
    "mesmerist",
    "monk",
    "ninja",
    "occultist",
    "oracle",
    "paladin",
    "psychic",
    "ranger",
    "rogue",
    "samurai",
    "shaman",
    "shifter",
    "skald",
    "slayer",
    "sorcerer",
    "spiritualist",
    "summoner",
    "swashbuckler",
    "vigilante",
    "warpriest",
    "warrior",
    "witch",
    "wizard",
];

/// The phrase without a base-class name that leads an item: at the start, or after a comma, `and`,
/// `or`, an article or `standard`, and followed by another word. Possessives are already gone.
fn strip_class_names(phrase: &str) -> String {
    let tokens: Vec<&str> = phrase.split_whitespace().collect();
    let mut out: Vec<&str> = Vec::with_capacity(tokens.len());
    for (index, token) in tokens.iter().enumerate() {
        let word = token
            .trim_matches(|ch: char| !ch.is_alphanumeric())
            .to_lowercase();
        let leads = match index.checked_sub(1).map(|previous| tokens[previous]) {
            None => true,
            Some(previous) => {
                let bare = previous.trim_end_matches(',').to_lowercase();
                previous.ends_with(',')
                    || CONNECTORS.contains(&bare.as_str())
                    || ARTICLES.contains(&bare.as_str())
                    // R5: "the 10th-level wizard bonus feat"
                    || ordinal_head(&bare).is_some_and(|(word, rest)| word == "level" && rest.is_empty())
            }
        };
        if leads && index + 1 < tokens.len() && CLASS_NAMES.contains(&word.as_str()) {
            continue;
        }
        out.push(token);
    }
    out.join(" ")
}

/// True when `name` occurs as a run of words in `phrase`, as a whole name or as one of the names
/// of a list. A base-class name that leads an item of the phrase is read through.
pub fn name_in_phrase(name: &[String], phrase: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let within = |text: &str| {
        name_words(text)
            .windows(name.len())
            .any(|window| window == name)
    };
    within(phrase)
        || within(&strip_class_names(&strip_possessives(phrase)))
        || within(&strip_class_names(&strip_possessor_phrases(phrase)))
}

/// True when `name_in_phrase` finds a trade name of the same op in the sentence. A bare sentence
/// ("These replace the normal attack roll bonus ...") counts as a trade sentence only if it does.
pub fn names_a_trade(sentence: &TradeSentence, trades: &[(TradeOp, String)]) -> bool {
    trades.iter().any(|(op, printed)| {
        *op == sentence.op && name_in_phrase(&name_words(printed), &sentence.phrase)
    })
}

/// R7 helper: replaces a run of words, case-insensitively, at word boundaries.
fn replace_words(text: &str, from: &str, to: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let pattern: Vec<&str> = from.split_whitespace().collect();
    let mut out: Vec<&str> = Vec::new();
    let mut index = 0;
    while index < words.len() {
        let hit = index + pattern.len() <= words.len()
            && words[index..index + pattern.len()]
                .iter()
                .zip(&pattern)
                .all(|(word, want)| word.eq_ignore_ascii_case(want));
        if hit {
            out.push(to);
            index += pattern.len();
        } else {
            out.push(words[index]);
            index += 1;
        }
    }
    out.join(" ")
}

/// R3: a spelled ordinal reads as its numeral form: "second" is "2nd".
fn ordinal(word: &str) -> String {
    const ORDINALS: [&str; 20] = [
        "first",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "eighth",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "fifteenth",
        "sixteenth",
        "seventeenth",
        "eighteenth",
        "nineteenth",
        "twentieth",
    ];
    match ORDINALS.iter().position(|spelled| *spelled == word) {
        Some(index) => {
            let n = index + 1;
            let suffix = match (n % 10, n % 100) {
                (1, 11) | (2, 12) | (3, 13) => "th",
                (1, _) => "st",
                (2, _) => "nd",
                (3, _) => "rd",
                _ => "th",
            };
            format!("{n}{suffix}")
        }
        None => word.to_string(),
    }
}

/// R6: "gun training 1 to 4" and "gun training 1-4" are the steps 1, 2, 3 and 4.
fn expand_range(item: &str) -> Option<Vec<String>> {
    let normalized = item.replace('\u{2013}', "-");
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    let (head, low, high) =
        if tokens.len() >= 4 && tokens[tokens.len() - 2].eq_ignore_ascii_case("to") {
            (
                tokens[..tokens.len() - 3].join(" "),
                tokens[tokens.len() - 3],
                tokens[tokens.len() - 1],
            )
        } else if tokens.len() >= 2 {
            let (low, high) = tokens[tokens.len() - 1].split_once('-')?;
            (tokens[..tokens.len() - 1].join(" "), low, high)
        } else {
            return None;
        };
    let low: u32 = is_number(low).then(|| low.parse().ok())??;
    let high: u32 = is_number(high).then(|| high.parse().ok())??;
    if head.is_empty() || low >= high || high - low > 20 {
        return None;
    }
    Some((low..=high).map(|step| format!("{head} {step}")).collect())
}

/// R10: "the sacred shield's holy champion ability": an article, at most two words and a
/// possessive word before a name are dropped together (sentence side only).
fn strip_possessor_phrases(phrase: &str) -> String {
    let tokens: Vec<&str> = phrase.split_whitespace().collect();
    let mut keep = vec![true; tokens.len()];
    for (index, token) in tokens.iter().enumerate() {
        let word =
            token.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '\'' && ch != '\u{2019}');
        if !(word.ends_with("'s") || word.ends_with("\u{2019}s")) {
            continue;
        }
        let from = index.saturating_sub(3);
        if let Some(article) = (from..index)
            .rev()
            .find(|at| matches!(tokens[*at].to_lowercase().as_str(), "the" | "a" | "an"))
        {
            let clean = tokens[article..index].iter().all(|token| {
                !token.ends_with(',') && !CONNECTORS.contains(&token.to_lowercase().as_str())
            });
            if clean {
                keep[article..=index].fill(false);
            }
        }
    }
    tokens
        .iter()
        .zip(keep)
        .filter(|(_, keep)| *keep)
        .map(|(token, _)| *token)
        .collect::<Vec<_>>()
        .join(" ")
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
                    bare: false,
                    phrase: "bravery".into()
                },
                TradeSentence {
                    op: TradeOp::Alters,
                    bare: false,
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
    fn a_comparison_to_a_spell_is_not_a_trade() {
        assert!(
            trade_sentences("This ability functions like alter self, except as noted here.")
                .is_empty()
        );
        assert!(trade_sentences("This works as alter self, except it lasts longer.").is_empty());
        assert_eq!(
            trade_sentences("This ability works like and replaces wild shape.").len(),
            1
        );
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

    #[test]
    fn extra_words_around_a_name_are_not_part_of_it() {
        assert_eq!(
            name_words("the paladin's spells class feature"),
            name_words("Spells")
        );
        assert_eq!(
            name_words("the nature sense ability"),
            name_words("Nature sense")
        );
        assert!(phrase_is_covered(
            "the paladin's spells class feature",
            &names(&["Spells"])
        ));
        // A name made only of such words is kept as it is.
        assert_eq!(name_words("Ability"), ["ability"]);
    }

    #[test]
    fn a_shared_tail_without_a_trailing_hyphen_expands_per_level() {
        assert_eq!(
            name_words("the paladin's 3rd-level and 9th-level mercies"),
            name_words("3rd-level mercies, 9th-level mercies")
        );
        let per_level = names(&["3rd-level mercy", "9th-level mercy"]);
        assert!(phrase_is_covered(
            "the paladin's 3rd-level and 9th-level mercies",
            &per_level
        ));
        // One trade for the whole phrase reads the same way.
        assert!(phrase_is_covered(
            "3rd-level and 9th-level mercies",
            &names(&["3rd-level and 9th-level mercies"])
        ));
        assert!(!phrase_is_covered(
            "3rd-level and 9th-level mercies",
            &names(&["3rd-level mercy"])
        ));
    }

    #[test]
    fn plurals_in_es_read_as_their_singular() {
        assert_eq!(name_words("hexes"), name_words("hex"));
        assert_eq!(name_words("witches"), name_words("witch"));
        assert_eq!(name_words("classes"), name_words("class"));
        assert_eq!(name_words("Bombs"), name_words("bomb"));
        assert_eq!(name_words("mercies"), name_words("mercy"));
        assert_eq!(name_words("bonuses"), name_words("bonus"));
    }

    #[test]
    fn a_leading_class_name_is_an_adjective_on_the_sentence_side() {
        let sentence = one(
            "This ability otherwise functions as and replaces the standard alchemist bomb class feature.",
        );
        assert!(!sentence.bare);
        assert!(phrase_is_covered(&sentence.phrase, &names(&["Bombs"])));
        assert!(name_in_phrase(&name_words("Bombs"), &sentence.phrase));
        // The class name stays part of the name too, so a trade that holds it still matches.
        assert!(phrase_is_covered(
            "the standard witch familiar",
            &names(&["Witch familiar"])
        ));
        assert!(phrase_is_covered(
            "the standard witch familiar",
            &names(&["Familiar"])
        ));
        // Only a class name that leads an item is read through.
        assert!(!phrase_is_covered(
            "the alchemist bomb and discovery",
            &names(&["Bombs"])
        ));
    }

    #[test]
    fn a_bare_subject_is_marked_and_an_ability_subject_is_not() {
        let bare = one("These replace the normal attack roll bonus and AC penalty for charging.");
        assert!(bare.bare);
        assert!(!one("This ability replaces bravery.").bare);
        assert!(!one("This class feature also replaces bravery.").bare);
        assert!(one("This replaces bravery.").bare);
        assert!(one("It alters bravery.").bare);
        let trades = [(TradeOp::Replaces, "Bravery".to_string())];
        assert!(!names_a_trade(&bare, &trades));
        assert!(names_a_trade(&one("This replaces bravery."), &trades));
        // The op counts.
        assert!(!names_a_trade(&one("This alters bravery."), &trades));
    }

    #[test]
    fn a_performance_is_an_ability_subject() {
        // Advanced Player's Guide bard archetypes.
        let sentence = one("This performance replaces inspire courage.");
        assert!(!sentence.bare);
        assert_eq!(sentence.op, TradeOp::Replaces);
        assert_eq!(sentence.phrase, "inspire courage");
        assert!(!one("This performance replaces jack of all trades.").bare);
        let sentence = one("These performances replace inspire competence and inspire greatness.");
        assert!(!sentence.bare);
        assert_eq!(sentence.phrase, "inspire competence and inspire greatness");
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Inspire competence", "Inspire greatness"])
        ));
        assert!(!one("This class performance also replaces suggestion.").bare);
        // Print italicizes a spell name; the text reaches the reader without the marks.
        let sentence = one("This performance replaces mass suggestion.");
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Mass suggestion"])
        ));
    }

    #[test]
    fn the_other_ability_nouns_are_subjects_and_a_bonus_is_not() {
        for noun in [
            "trick",
            "talent",
            "hex",
            "discovery",
            "revelation",
            "mercy",
            "deed",
            "exploit",
            "arcana",
        ] {
            let sentence = one(&format!("This {noun} replaces bravery."));
            assert!(!sentence.bare, "{noun}");
        }
        assert!(!one("These hexes replace the witch's patron spells.").bare);
        assert!(!one("These mercies replace channel energy.").bare);
        for noun in ["bonus", "penalty", "effect", "value"] {
            assert!(
                trade_sentences(&format!("This {noun} replaces the usual bonus.")).is_empty(),
                "{noun}"
            );
        }
    }

    #[test]
    fn a_granted_thing_is_a_subject_with_or_without_bonus() {
        // Ultimate Magic pp. 58-59: the six oracle archetypes (owner ruling 22).
        let sentence =
            one("These bonus spells replace the oracle's mystery bonus spells at these levels.");
        assert_eq!(sentence.op, TradeOp::Replaces);
        assert!(sentence.bare);
        for stored in [
            "Mystery bonus spells at these levels",
            "Oracle's mystery bonus spells at these levels",
            "The oracle's mystery bonus spells at these levels",
        ] {
            let trades = [(TradeOp::Replaces, stored.to_string())];
            assert!(names_a_trade(&sentence, &trades), "{stored}");
        }
        // Bare: it counts only when it names a stored trade.
        let other = [(TradeOp::Replaces, "Bravery".to_string())];
        assert!(!names_a_trade(&sentence, &other));
        // Other nouns, with and without `bonus`.
        let sentence = one("This bonus feat replaces the fighter's bonus feat at 2nd level.");
        assert_eq!(sentence.op, TradeOp::Replaces);
        assert!(sentence.bare);
        assert!(!one("These bonus hexes replace the witch's patron spells.").bare);
        assert!(!one("These bonus powers replace the mystery's powers.").bare);
        assert!(one("These spells replace the sorcerer's bloodline spells.").bare);
        assert!(one("This skill replaces Perception.").bare);
        assert!(one("These evolutions replace the eidolon's base evolutions.").bare);
        assert!(!one("This bonus ability alters bravery.").bare);
        assert_eq!(
            one("This bonus ability alters bravery.").op,
            TradeOp::Alters
        );
    }

    #[test]
    fn a_numeric_bonus_or_an_unlisted_noun_is_not_a_subject() {
        assert!(trade_sentences("This bonus replaces the fighter's bonus feat.").is_empty());
        assert!(trade_sentences("This bonus alters the fighter's bonus feat.").is_empty());
        assert!(trade_sentences("These rounds replace the oracle's mystery spells.").is_empty());
        assert!(trade_sentences("These bonus rounds replace the oracle's spells.").is_empty());
        // `bonus` before a verb or an unlisted noun is not a subject word.
        assert!(trade_sentences("This bonus value replaces bravery.").is_empty());
        // A negation still rejects.
        assert!(trade_sentences("These bonus spells do not replace bravery.").is_empty());
    }

    #[test]
    fn feats_are_a_bare_subject() {
        // Advanced Player's Guide, Zen Archer.
        let sentence = one("These feats replace the monk's normal bonus feats.");
        assert!(sentence.bare);
        assert_eq!(sentence.op, TradeOp::Replaces);
        let trades = [(
            TradeOp::Replaces,
            "The monk's normal bonus feats".to_string(),
        )];
        assert!(names_a_trade(&sentence, &trades));
        // A bare sentence that names no trade is not one.
        let other = [(TradeOp::Replaces, "Bravery".to_string())];
        assert!(!names_a_trade(&sentence, &other));
    }

    #[test]
    fn both_announces_a_list_of_two() {
        // Advanced Player's Guide, Monk of the Healing Hand.
        let sentence = one("This ability replaces both diamond body and quivering palm.");
        assert_eq!(sentence.op, TradeOp::Replaces);
        assert!(!sentence.bare);
        assert_eq!(sentence.phrase, "diamond body and quivering palm");
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Diamond body", "Quivering palm"])
        ));
        // One trade for the whole phrase reads the same way.
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Diamond body and quivering palm"])
        ));
        assert!(!phrase_is_covered(
            &sentence.phrase,
            &names(&["Diamond body"])
        ));
        // `both` after a second verb is dropped too; one inside a name is kept.
        let found = trade_sentences(
            "This ability replaces bravery and alters both armor training and weapon training.",
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].phrase, "armor training and weapon training");
        assert_eq!(
            one("This ability replaces bothersome armor.").phrase,
            "bothersome armor"
        );
    }

    // ---- #540 batch 1 (Ultimate Magic, Ultimate Combat) descriptor rules ----

    #[test]
    fn class_ability_is_a_descriptor_like_class_feature() {
        // Ultimate Magic, Mindchemist; Staff Magus.
        let sentence = one(
            "This ability replaces the mutagen class ability (a mindchemist cannot create mutagens unless he selects mutagen as a discovery).",
        );
        assert_eq!(sentence.phrase, "the mutagen class ability");
        assert!(phrase_is_covered(&sentence.phrase, &names(&["Mutagen"])));
        let sentence =
            one("This ability replaces the medium armor and heavy armor class abilities.");
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Medium armor", "heavy armor"])
        ));
        assert!(!phrase_is_covered(
            &sentence.phrase,
            &names(&["Medium armor"])
        ));
        // `class` before another word stays part of the name.
        assert_eq!(name_words("Class skills"), ["class", "skill"]);
    }

    #[test]
    fn a_trailing_aside_in_parentheses_is_not_a_name() {
        assert_eq!(one("This replaces bravery (Ex).").phrase, "bravery (Ex)");
        assert_eq!(
            one("This ability replaces Weapon Focus (longsword).").phrase,
            "Weapon Focus (longsword)"
        );
        assert_eq!(
            one("This ability replaces bravery (but not armor training 1).").phrase,
            "bravery"
        );
    }

    #[test]
    fn a_but_clause_after_the_names_is_not_a_name() {
        // Ultimate Combat, Arcane Bomber, Siege Mage, Spellslinger.
        let sentence = one(
            "This ability replaces cantrips, but the siege mage gains the detect magic and read magic cantrips and places them in his spellbook.",
        );
        assert_eq!(sentence.phrase, "cantrips");
        assert!(phrase_is_covered(&sentence.phrase, &names(&["Cantrips"])));
    }

    #[test]
    fn normal_and_normally_are_determiners() {
        // Ultimate Magic, Broodmaster, Master Summoner; Ultimate Combat, Tetori, Kensai.
        let sentence = one("This ability replaces the summoner's normal eidolon ability.");
        assert!(phrase_is_covered(&sentence.phrase, &names(&["Eidolon"])));
        let sentence = one(
            "This ability replaces the summoner's normal summon monster I ability and shield ally.",
        );
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Summon monster I", "shield ally"])
        ));
        let sentence = one("These feats replace a monk's normal bonus feats.");
        assert!(sentence.bare);
        assert!(names_a_trade(
            &sentence,
            &[(TradeOp::Replaces, "Bonus feats".to_string())]
        ));
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Bonus feats"])
        ));
        assert!(phrase_is_covered(
            "the magus arcana normally gained at 9th level",
            &names(&["Magus arcana gained at 9th level"])
        ));
        // The bard and magus proficiency sentences: a class name after `normal` is an adjective.
        assert!(phrase_is_covered(
            "the normal magus weapon and armor proficiency feature",
            &names(&["Weapon and armor proficiencies"])
        ));
        // `bonus` is not a determiner: bonus spells are not spells.
        assert!(!phrase_is_covered(
            "the oracle's mystery bonus spells",
            &names(&["Spells"])
        ));
        assert!(!phrase_is_covered(
            "the bonus teamwork feat gained at 6th level",
            &names(&["Teamwork feat gained at 6th level"])
        ));
    }

    #[test]
    fn spelled_ordinals_read_as_numerals() {
        // Ultimate Combat, Battle Scout, Warden, Wild Stalker.
        assert_eq!(
            name_words("second favored enemy"),
            name_words("2nd favored enemy")
        );
        assert!(phrase_is_covered(
            "the ranger's second favored enemy",
            &names(&["2nd favored enemy"])
        ));
        assert!(phrase_is_covered(
            "the ranger's first favored enemy ability",
            &names(&["1st favored enemy"])
        ));
        assert!(phrase_is_covered(
            "the ranger's second, third, and fourth favored enemies",
            &names(&["2nd, 3rd, and 4th favored enemies"])
        ));
        assert!(phrase_is_covered(
            "the ranger's fifth favored enemy and master hunter",
            &names(&["5th favored enemy", "master hunter"])
        ));
        assert!(!phrase_is_covered(
            "the ranger's second favored enemy",
            &names(&["3rd favored enemy"])
        ));
        assert_eq!(
            name_words("eleventh twelfth twentieth"),
            ["11th", "12th", "20th"]
        );
    }

    #[test]
    fn a_class_name_after_a_level_is_an_adjective() {
        // Ultimate Magic, Scrollmaster.
        assert!(phrase_is_covered(
            "the 10th-level wizard bonus feat",
            &names(&["10th-level bonus feat"])
        ));
        assert!(name_in_phrase(
            &name_words("10th-level bonus feat"),
            "the 10th-level wizard bonus feat"
        ));
        // The trade may keep the class name too.
        assert!(phrase_is_covered(
            "the 10th-level wizard bonus feat",
            &names(&["10th-level wizard bonus feat"])
        ));
        assert!(!phrase_is_covered(
            "the 10th-level wizard bonus feat",
            &names(&["12th-level bonus feat"])
        ));
    }

    #[test]
    fn a_step_range_reads_as_its_steps() {
        // Ultimate Combat, Pistolero.
        assert_eq!(
            name_words("gun training 1 to 4"),
            name_words("Gun training 1-4")
        );
        assert_eq!(
            name_words("gun training 1 to 4"),
            name_words("gun training 1, 2, 3, and 4")
        );
        assert!(phrase_is_covered(
            "gun training 1 to 4",
            &names(&["Gun training 1-4"])
        ));
        assert!(phrase_is_covered(
            "gun training 1 to 4",
            &names(&[
                "Gun training 1",
                "Gun training 2",
                "Gun training 3",
                "Gun training 4"
            ])
        ));
        assert!(!phrase_is_covered(
            "gun training 1 to 4",
            &names(&["Gun training 1"])
        ));
        // No head, or a falling range, is not a range of steps.
        assert_eq!(name_words("1 to 4"), ["1", "to", "4"]);
        assert_eq!(
            name_words("gun training 4 to 1"),
            ["gun", "training", "4", "to", "1"]
        );
    }

    #[test]
    fn as_well_as_joins_two_names() {
        // Ultimate Combat, Beastmorph.
        let sentence = one(
            "This ability replaces poison resistance +2, +4, and +6, as well as poison immunity.",
        );
        assert!(phrase_is_covered(
            &sentence.phrase,
            &names(&["Poison resistance +2, +4, and +6", "poison immunity"])
        ));
        assert!(!phrase_is_covered(
            &sentence.phrase,
            &names(&["Poison resistance +2, +4, and +6"])
        ));
    }

    #[test]
    fn a_possessor_phrase_before_a_name_is_dropped() {
        // Ultimate Combat, Sacred Shield.
        assert!(phrase_is_covered(
            "the sacred shield's holy champion ability",
            &names(&["Holy champion"])
        ));
        assert!(phrase_is_covered(
            "the standard paladin's divine bond",
            &names(&["Divine bond"])
        ));
        assert!(!phrase_is_covered(
            "the sacred shield's holy champion ability",
            &names(&["Champion"])
        ));
    }

    #[test]
    fn a_verb_joined_to_another_clause_is_read_as_bare() {
        // Ultimate Combat, Dervish Dancer, Battle Fury.
        let sentence = one(
            "This movement provokes attacks of opportunity as normal, and replaces deadly performance.",
        );
        assert!(sentence.bare);
        assert_eq!(sentence.phrase, "deadly performance");
        assert!(names_a_trade(
            &sentence,
            &[(TradeOp::Replaces, "Deadly performance".to_string())]
        ));
        assert!(!names_a_trade(
            &sentence,
            &[(TradeOp::Replaces, "Bravery".to_string())]
        ));
        // Without `and` before the verb the other subject still is not a trade.
        assert!(trade_sentences("This bonus replaces the usual bonus.").is_empty());
        assert!(
            trade_sentences(
                "This movement provokes attacks of opportunity and does not replace bravery."
            )
            .is_empty()
        );
    }

    #[test]
    fn changes_is_an_alters_verb_in_the_sentence_position() {
        // Owner ruling 25 of 2026-11-24 (#540), Magus, Bladebound.
        let found = trade_sentences(
            "This ability changes the Arcane Pool class feature and replaces the magus arcana gained at 3rd level.",
        );
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].op, TradeOp::Alters);
        assert!(!found[0].bare);
        assert!(phrase_is_covered(
            &found[0].phrase,
            &names(&["Arcane Pool"])
        ));
        assert_eq!(found[1].op, TradeOp::Replaces);
        assert!(phrase_is_covered(
            &found[1].phrase,
            &names(&["Magus arcana gained at 3rd level"])
        ));
        let plural = one("These abilities change the bard's performance class feature.");
        assert_eq!(plural.op, TradeOp::Alters);
        // Not in a prose sentence, a negation or another subject.
        assert!(trade_sentences("A magus changes his weapon.").is_empty());
        assert!(trade_sentences("This ability does not change arcane pool.").is_empty());
        assert!(trade_sentences("This bonus changes the usual bonus.").is_empty());
        // Not as a further verb joined by `and`: "changes" there is prose.
        let found = trade_sentences("This ability replaces bravery and changes the saving throw.");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].op, TradeOp::Replaces);
    }

    #[test]
    fn bonus_is_never_dropped_from_a_name() {
        assert_eq!(name_words("bonus spells"), ["bonus", "spell"]);
        assert!(!phrase_is_covered(
            "the oracle's mystery bonus spells",
            &names(&["Spells"])
        ));
        assert!(!phrase_is_covered(
            "bonus teamwork feats",
            &names(&["Teamwork feats"])
        ));
        assert!(!phrase_is_covered(
            "teamwork feats",
            &names(&["Bonus teamwork feats"])
        ));
    }

    #[test]
    fn a_comma_and_a_connector_stop_a_possessor_phrase() {
        // Dropped across a comma or `and`, the phrase would shrink to "divine bond".
        assert!(!phrase_is_covered(
            "the shield, sacred knight's divine bond",
            &names(&["Divine bond"])
        ));
        assert!(!phrase_is_covered(
            "the shield and sacred knight's divine bond",
            &names(&["Divine bond"])
        ));
        assert!(!phrase_is_covered(
            "the shield or sacred knight's divine bond",
            &names(&["Divine bond"])
        ));
        // An article, two words and the possessive word are dropped; a longer run is not.
        assert!(phrase_is_covered(
            "the grand sacred knight's divine bond",
            &names(&["Divine bond"])
        ));
        assert!(!phrase_is_covered(
            "the very grand sacred knight's divine bond",
            &names(&["Divine bond"])
        ));
        assert!(phrase_is_covered(
            "the sacred knight's divine bond",
            &names(&["Divine bond"])
        ));
    }

    #[test]
    fn the_feature_name_as_subject_is_an_ability_subject() {
        // Ultimate Combat, Evangelist.
        let text = "Sermonic performance replaces the 1st-, 9th-, and 15th-level channel energy abilities.";
        assert!(trade_sentences(text).is_empty());
        let found = trade_sentences_headed(text, Some("Sermonic Performance (Su)"));
        assert_eq!(found.len(), 1);
        assert!(!found[0].bare);
        assert!(phrase_is_covered(
            &found[0].phrase,
            &names(&["The 1st-, 9th-, and 15th-level channel energy abilities"])
        ));
        assert!(trade_sentences_headed(text, Some("Spontaneous Casting")).is_empty());
    }
}
