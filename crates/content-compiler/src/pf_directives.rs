//! Parsers for the two Pf Data directives that carry derived records.
//!
//! `::rtraits` lists a race's standard traits as `Name|Text` segments separated by
//! `~`. Short macro segments (`MEDIUM`, `DV`, `LANG`, and so on) encode a standard
//! trait without prose; the expansion below supplies the same wording the PF1e core
//! rules use for that trait. `::aff` is the affliction stat block documented in the
//! pinned dataset's `JSON.md`.
//!
//! Both parsers are pure: they return text or structured values and never touch the
//! filesystem. The compiler decides what to do with a rejection.

use crate::pf_markdown::bracket;
use std::collections::BTreeMap;

/// One standard racial trait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trait {
    /// Display title, such as `Stonecunning`, `Medium`, or `Attributes`.
    pub title: String,
    /// Prose without the title.
    pub text: String,
    /// The ability-score line, present only for the race's `Attributes` trait.
    pub ability: Option<String>,
}

impl Trait {
    /// Body Markdown for a standalone derived record, where the title is the entry title.
    pub fn body(&self) -> String {
        match &self.ability {
            Some(ability) if !self.text.is_empty() => format!("**{ability}.** {}", self.text),
            Some(ability) => format!("**{ability}.**"),
            None => self.text.clone(),
        }
    }
}

/// A parsed `::rtraits` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RacialTraits {
    pub plural: String,
    pub traits: Vec<Trait>,
}

/// Parse `::rtraits{plural="..." main="..."}`.
///
/// The first segment is the ability-score line. A segment whose head is an
/// upper-case macro token is expanded from [`macro_trait`]; anything else is a named
/// trait whose text is used verbatim.
pub fn rtraits(line: &str) -> Result<RacialTraits, String> {
    let attrs = lenient_attributes(line)?;
    let plural = attrs.get("plural").cloned().unwrap_or_default();
    let main = attrs
        .get("main")
        .ok_or_else(|| "::rtraits has no main attribute".to_string())?;
    let mut traits = Vec::new();
    for (index, segment) in main.split('~').enumerate() {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let (head, text) = match segment.split_once('|') {
            Some((head, text)) => (head.trim(), text.trim()),
            None => (segment, ""),
        };
        if index == 0 && !text.is_empty() && !is_macro(head) {
            traits.push(Trait {
                title: "Attributes".into(),
                text: text.into(),
                ability: Some(head.into()),
            });
        } else if let Some(trait_) = macro_trait(head, text, &plural) {
            traits.push(trait_);
        } else {
            traits.push(Trait {
                title: head.into(),
                text: text.into(),
                ability: None,
            });
        }
    }
    Ok(RacialTraits { plural, traits })
}

/// Render one trait for a parent record body that keeps its traits inline.
pub fn trait_markdown(trait_: &Trait) -> String {
    match &trait_.ability {
        Some(ability) if !trait_.text.is_empty() => {
            format!("**{}:** {ability}. {}", trait_.title, trait_.text)
        }
        Some(ability) => format!("**{}:** {ability}.", trait_.title),
        None if trait_.text.is_empty() => format!("**{}**", trait_.title),
        None => format!("**{}:** {}", trait_.title, trait_.text),
    }
}

/// Render `::rtraits` for a parent record body that keeps its traits inline.
pub fn racial_traits_markdown(rest: &str) -> Result<String, String> {
    let parsed = rtraits(rest)?;
    Ok(parsed
        .traits
        .iter()
        .map(trait_markdown)
        .collect::<Vec<_>>()
        .join("\n\n"))
}

/// A macro token is upper case, may end in `+`, and is at least two characters.
fn is_macro(head: &str) -> bool {
    head.len() >= 2
        && head
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '+' || c == '_')
}

fn macro_trait(head: &str, text: &str, plural: &str) -> Option<Trait> {
    let plural = if plural.is_empty() { "They" } else { plural };
    let trait_ = |title: &str, text: String| Trait {
        title: title.into(),
        text,
        ability: None,
    };
    let (token, args) = match head.split_once('+') {
        Some((token, _)) => (token, text),
        None => (head, text),
    };
    Some(match token {
        "MEDIUM" => trait_(
            "Medium",
            format!(
                "{plural} are Medium creatures and have no bonuses or penalties due to their size."
            ),
        ),
        "SMALL" => trait_(
            "Small",
            format!(
                "{plural} are Small creatures and gain a +1 size bonus to their AC, a +1 size bonus on attack rolls, a -1 penalty to their Combat Maneuver Bonus and Combat Maneuver Defense, and a +4 size bonus on Stealth checks."
            ),
        ),
        "SPEED" => {
            let speed = match args {
                "" | "normal" => 30,
                "slow" => 20,
                other => other.parse().unwrap_or(30),
            };
            let title = match args {
                "" | "normal" => "Normal Speed",
                "slow" => "Slow Speed",
                _ => "Speed",
            };
            trait_(
                title,
                format!("{plural} have a base speed of {speed} feet."),
            )
        }
        "DV" => trait_(
            "Darkvision",
            format!("{plural} can see in the dark up to 60 feet."),
        ),
        "LLV" => trait_(
            "Low-Light Vision",
            format!("{plural} can see twice as far as humans in conditions of dim light."),
        ),
        "KEENSEN" => trait_(
            "Keen Senses",
            format!("{plural} receive a +2 racial bonus on Perception skill checks."),
        ),
        "LANG" => trait_("Languages", languages(args, plural)),
        "LANGX" => trait_("Languages", extended_languages(args, plural)),
        _ => return None,
    })
}

fn languages(body: &str, plural: &str) -> String {
    let parts: Vec<&str> = body.split('|').collect();
    let split = parts.iter().position(|p| p.is_empty());
    let (spoken, bonus) = match split {
        Some(index) => (&parts[..index], &parts[index + 1..]),
        None => (&parts[..], &parts[parts.len()..]),
    };
    let spoken: Vec<&str> = spoken.iter().copied().filter(|s| !s.is_empty()).collect();
    let mut text = format!("{plural} begin play speaking {}.", join_list(&spoken));
    let bonus: Vec<&str> = bonus.iter().copied().filter(|s| !s.is_empty()).collect();
    if bonus == ["ANY"] {
        text.push_str(&format!(
            " {plural} with high Intelligence scores can choose any languages they want (except secret languages, such as Druidic)."
        ));
    } else if !bonus.is_empty() {
        text.push_str(&format!(
            " {plural} with high Intelligence scores can choose from the following: {}.",
            join_list(&bonus)
        ));
    }
    text
}

fn extended_languages(body: &str, plural: &str) -> String {
    let parts: Vec<&str> = body.split('|').collect();
    let mut text = parts
        .iter()
        .take_while(|part| **part != "ANY")
        .copied()
        .collect::<Vec<_>>()
        .join("|");
    if parts.contains(&"ANY") {
        text.push_str(&format!(
            " {plural} with high Intelligence scores can choose any languages they want (except secret languages, such as Druidic)."
        ));
    }
    text
}

fn join_list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => (*one).to_string(),
        [one, two] => format!("{one} and {two}"),
        _ => {
            let (last, rest) = items.split_last().expect("non-empty");
            format!("{}, and {last}", rest.join(", "))
        }
    }
}

/// A parsed `::aff` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affliction {
    pub name: Option<String>,
    pub markdown: String,
}

/// Parse and render `::aff[Name]{...}`.
///
/// `include_name` keeps the bold name line for a parent body; a derived record
/// already has the name as its title and passes `false`.
pub fn affliction(rest: &str, include_name: bool) -> Result<Affliction, String> {
    let name = bracket(rest).filter(|name| !name.trim().is_empty());
    let attrs = lenient_attributes(rest)?;
    let mut lines = Vec::new();
    if include_name && let Some(name) = &name {
        lines.push(format!("**{name}**"));
    }
    if let Some(kind) = affliction_type(&attrs) {
        lines.push(format!("**Type:** {kind}"));
    }
    if let Some(save) = saving_throw(&attrs) {
        lines.push(format!("**Save:** {save}"));
    }
    if let Some(onset) = attrs.get("onset").filter(|value| !value.is_empty()) {
        lines.push(format!("**Onset:** {onset}"));
    }
    if let Some(frequency) = frequency(&attrs) {
        lines.push(format!("**Frequency:** {frequency}"));
    }
    if let Some(track) = attrs.get("track").filter(|value| !value.is_empty()) {
        let text = match attrs.get("trackmod") {
            Some(extra) if !extra.is_empty() => format!("{track} ({extra})"),
            _ => track.clone(),
        };
        lines.push(format!("**Track:** {text}"));
    }
    for (prefix, label) in [
        ("", "Effect"),
        ("in", "Initial effect"),
        ("sec", "Secondary effect"),
    ] {
        if let Some(effect) = effect(&attrs, prefix) {
            lines.push(format!("**{label}:** {effect}"));
        }
    }
    if let Some(cure) = cure(&attrs) {
        lines.push(format!("**Cure:** {cure}"));
    }
    if let Some(extra) = attrs.get("extra").filter(|value| !value.is_empty()) {
        lines.push(extra.clone());
    }
    Ok(Affliction {
        name,
        markdown: lines.join("\n\n"),
    })
}

fn affliction_type(attrs: &BTreeMap<String, String>) -> Option<String> {
    let kind = if attrs.contains_key("poison") {
        Some("Poison")
    } else if attrs.contains_key("curse") {
        Some("Curse")
    } else if attrs.contains_key("infest") {
        Some("Infestation")
    } else if attrs.contains_key("disease") {
        Some("Disease")
    } else if attrs.contains_key("iconP") {
        Some("Poison")
    } else if attrs.contains_key("iconC") {
        Some("Curse")
    } else if attrs.contains_key("iconI") {
        Some("Infestation")
    } else if attrs.contains_key("iconD") {
        Some("Disease")
    } else if attrs.contains_key("iconA") {
        Some("Affliction")
    } else {
        None
    };
    match (kind, attrs.get("type").filter(|value| !value.is_empty())) {
        (Some(kind), Some(subtype)) => Some(format!("{kind} ({subtype})")),
        (Some(kind), None) => Some(kind.into()),
        (None, Some(subtype)) => Some(subtype.clone()),
        (None, None) => Some("Affliction".into()),
    }
}

fn saving_throw(attrs: &BTreeMap<String, String>) -> Option<String> {
    if let Some(save) = attrs.get("save").filter(|value| !value.is_empty()) {
        return Some(save.clone());
    }
    for (key, kind) in [("saveF", "Fortitude"), ("saveW", "Will")] {
        if let Some(dc) = attrs.get(key).filter(|value| !value.is_empty()) {
            return Some(format!("{kind} DC {dc}"));
        }
    }
    let kind = if attrs.contains_key("dcF") {
        "Fortitude"
    } else if attrs.contains_key("dcW") {
        "Will"
    } else if attrs.contains_key("dcR") {
        "Reflex"
    } else {
        return None;
    };
    let possessive = if attrs.contains_key("dcYou") {
        "your".to_string()
    } else {
        attrs.get("dcIt").cloned().unwrap_or_else(|| "their".into())
    };
    let stat = attrs.get("dcMod").filter(|value| !value.is_empty());
    let level = attrs
        .get("dcLev")
        .map(|value| (value, "level"))
        .or_else(|| attrs.get("dcHD").map(|value| (value, "HD")));
    match (level, stat) {
        (Some((level, unit)), Some(stat)) => {
            let determiner = if attrs.contains_key("dcYou") {
                "your"
            } else {
                "the"
            };
            let post = if let Some(possessive) = attrs.get("dcPoss") {
                possessive.as_str()
            } else if attrs.contains_key("dcYou") {
                ""
            } else {
                "'s"
            };
            Some(format!(
                "{kind} 10 + 1/2 {determiner} {level}{post} {unit} + {possessive} {stat} modifier"
            ))
        }
        (None, Some(stat)) => Some(format!("{kind} 10 + {possessive} {stat} modifier")),
        (Some((level, unit)), None) => Some(format!("{kind} {level} {unit}")),
        (None, None) => Some(kind.into()),
    }
}

fn frequency(attrs: &BTreeMap<String, String>) -> Option<String> {
    if let Some(freq) = attrs.get("freq").filter(|value| !value.is_empty()) {
        return Some(freq.clone());
    }
    for (key, unit) in [
        ("freqR", "round"),
        ("freqM", "minute"),
        ("freqH", "hour"),
        ("freqD", "day"),
    ] {
        if let Some(amount) = attrs.get(key).filter(|value| !value.is_empty()) {
            let plural = if amount == "1" { "" } else { "s" };
            return Some(format!("1/{unit} for {amount} {unit}{plural}"));
        }
    }
    None
}

fn effect(attrs: &BTreeMap<String, String>, prefix: &str) -> Option<String> {
    let direct = match prefix {
        "" => attrs.get("eff"),
        "in" => attrs.get("ineff"),
        _ => attrs.get("seceff"),
    };
    if let Some(effect) = direct.filter(|value| !value.is_empty()) {
        return Some(effect.clone());
    }
    let mut items = Vec::new();
    for (abbr, name) in [
        ("Str", "Strength"),
        ("Dex", "Dexterity"),
        ("Con", "Constitution"),
        ("Int", "Intelligence"),
        ("Wis", "Wisdom"),
        ("Cha", "Charisma"),
    ] {
        if let Some(value) = attrs
            .get(&format!("{prefix}eff{abbr}"))
            .filter(|value| !value.is_empty())
        {
            items.push(format!("{name} damage {value}"));
        }
        if let Some(value) = attrs
            .get(&format!("{prefix}eff{abbr}D"))
            .filter(|value| !value.is_empty())
        {
            items.push(format!("{name} drain {value}"));
        }
    }
    if let Some(value) = attrs
        .get(&format!("{prefix}effExtra"))
        .filter(|value| !value.is_empty())
    {
        items.push(value.clone());
    }
    let joiner = match attrs.get(&format!("{prefix}effOr")) {
        Some(value) if !value.is_empty() => value.as_str(),
        Some(_) => " or ",
        None => " and ",
    };
    (!items.is_empty()).then(|| join_items(&items, joiner))
}

fn join_items(items: &[String], joiner: &str) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one}{joiner}{two}"),
        _ => {
            let (last, rest) = items.split_last().expect("non-empty");
            format!("{},{joiner}{last}", rest.join(", "))
        }
    }
}

fn cure(attrs: &BTreeMap<String, String>) -> Option<String> {
    if let Some(cure) = attrs.get("cure").filter(|value| !value.is_empty()) {
        return Some(cure.clone());
    }
    for (key, text) in [
        ("cure1", "1 save"),
        ("cure2", "2 saves"),
        ("cure2c", "2 consecutive saves"),
        ("cure3", "3 saves"),
        ("cure3c", "3 consecutive saves"),
    ] {
        if attrs.contains_key(key) {
            return Some(text.into());
        }
    }
    None
}

/// Parse `{key=value key="quoted value" flag}`.
///
/// Duplicate keys keep the first value and append later prose to `extra`, so the
/// one malformed upstream block that repeats `saveF` loses nothing.
fn lenient_attributes(input: &str) -> Result<BTreeMap<String, String>, String> {
    let open = input
        .find('{')
        .ok_or_else(|| "directive has no attribute braces".to_string())?;
    let close = input
        .rfind('}')
        .ok_or_else(|| "unterminated directive attributes".to_string())?;
    if close < open || !input[close + 1..].trim().is_empty() {
        return Err("malformed directive attribute braces".into());
    }
    let body = &input[open + 1..close];
    let mut out = BTreeMap::new();
    let mut chars = body.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut key = String::new();
        while chars
            .peek()
            .is_some_and(|c| !c.is_whitespace() && *c != '=')
        {
            key.push(chars.next().unwrap());
        }
        if key.is_empty() {
            return Err("empty attribute name".into());
        }
        let mut value = String::new();
        if chars.peek() == Some(&'=') {
            chars.next();
            if matches!(chars.peek(), Some('"') | Some('\'')) {
                let quote = chars.next().unwrap();
                loop {
                    match chars.next() {
                        Some(c) if c == quote => break,
                        Some(c) => value.push(c),
                        None => return Err(format!("unterminated quoted value for {key}")),
                    }
                }
            } else {
                while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                    value.push(chars.next().unwrap());
                }
            }
        }
        if out.contains_key(&key) {
            if !value.is_empty() {
                let extra = out.entry("extra".into()).or_insert_with(String::new);
                if !extra.is_empty() {
                    extra.push(' ');
                }
                extra.push_str(&value);
            }
            continue;
        }
        out.insert(key, value);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_rracial_traits_and_expands_macros() {
        let parsed = rtraits(
            r#"::rtraits{plural=Dwarves main="+2 Constitution, +2 Wisdom, -2 Charisma|Dwarves are tough.~MEDIUM~Slow and Steady|Dwarves have a base speed of 20 feet.~DV~LANG|Common|Dwarven||Giant|Orc"}"#,
        )
        .unwrap();
        assert_eq!(parsed.plural, "Dwarves");
        let titles: Vec<_> = parsed.traits.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Attributes",
                "Medium",
                "Slow and Steady",
                "Darkvision",
                "Languages"
            ]
        );
        assert_eq!(
            parsed.traits[0].body(),
            "**+2 Constitution, +2 Wisdom, -2 Charisma.** Dwarves are tough."
        );
        assert_eq!(
            parsed.traits[1].text,
            "Dwarves are Medium creatures and have no bonuses or penalties due to their size."
        );
        assert_eq!(
            parsed.traits[4].text,
            "Dwarves begin play speaking Common and Dwarven. Dwarves with high Intelligence scores can choose from the following: Giant and Orc."
        );
    }

    #[test]
    fn expands_any_languages_and_unknown_macros_without_panicking() {
        let parsed = rtraits(
            r#"::rtraits{plural=Half-Elves main="+2 to One Ability Score|Half-elves are varied.~LANG|Common|Elven||ANY~WEIRD"}"#,
        )
        .unwrap();
        assert!(parsed.traits[1].text.contains("choose any languages"));
        assert_eq!(parsed.traits[2].title, "WEIRD");
    }

    #[test]
    fn renders_afflictions_from_every_documented_field_group() {
        let parsed = affliction(
            r#"[Fiendish Bile]{iconP type=Bite-injury saveF=25 freqR=6 effStr="1d4" effExtra="good-aligned creatures take 2d8" cure2c extra="The save DC is Constitution-based."}"#,
            false,
        )
        .unwrap();
        assert_eq!(parsed.name.as_deref(), Some("Fiendish Bile"));
        assert_eq!(
            parsed.markdown,
            "**Type:** Poison (Bite-injury)\n\n**Save:** Fortitude DC 25\n\n**Frequency:** 1/round for 6 rounds\n\n**Effect:** Strength damage 1d4 and good-aligned creatures take 2d8\n\n**Cure:** 2 consecutive saves\n\nThe save DC is Constitution-based."
        );
    }

    #[test]
    fn renders_dc_expressions_and_duplicate_notes() {
        let parsed = affliction(
            r#"[Wolf-in-Sheep's-Clothing Egg]{iconI type=Infestation-ingestion saveF=19 onset="1 day" freqD=2 effCon=1 cure3c saveF="The save DC is Con-based."}"#,
            true,
        )
        .unwrap();
        assert!(
            parsed
                .markdown
                .starts_with("**Wolf-in-Sheep's-Clothing Egg**")
        );
        assert!(parsed.markdown.contains("**Save:** Fortitude DC 19"));
        assert!(parsed.markdown.contains("**Frequency:** 1/day for 2 days"));
        assert!(parsed.markdown.ends_with("The save DC is Con-based."));

        let parsed = affliction(
            r#"[Thing]{iconC curse type=Bite saveW=26 freq="1/day" effConD="1d6" effOr=" or "}"#,
            false,
        )
        .unwrap();
        assert!(parsed.markdown.contains("**Type:** Curse (Bite)"));
        assert!(parsed.markdown.contains("**Save:** Will DC 26"));
        assert!(
            parsed
                .markdown
                .contains("**Effect:** Constitution drain 1d6")
        );
    }
}
