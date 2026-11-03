//! Typed field extraction. Runs once per record before any rendering, so a record whose
//! fields have an unexpected shape becomes a rejection instead of a half-built entry.

use crate::html::plain_text;
use crate::model::Node;
use cast_iron_schema::{
    ChallengeRating, ClassFields, CreatureFields, EntryKind, FeatFields, ItemFields, SpellFields,
    SpellLevel, TypedFields,
};
use serde_json::Value;

pub struct Extracted {
    pub fields: TypedFields,
    pub tags: Vec<String>,
    /// Search facets, lowercase except for challenge ratings.
    pub facets: Vec<(String, String)>,
}

impl Extracted {
    fn new(fields: TypedFields) -> Self {
        Self {
            fields,
            tags: Vec::new(),
            facets: Vec::new(),
        }
    }

    fn tag(&mut self, value: &str) {
        let tag = value.trim().to_lowercase();
        if !tag.is_empty() && !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    fn facet(&mut self, name: &str, value: &str) {
        let value = value.trim().to_lowercase();
        if !value.is_empty() && !self.facets.iter().any(|(n, v)| n == name && *v == value) {
            self.facets.push((name.to_string(), value));
        }
    }
}

pub fn extract(kind: EntryKind, node: &Node, rel: &str) -> Result<Extracted, String> {
    match kind {
        EntryKind::Spell => spell(node),
        EntryKind::Feat => feat(node),
        EntryKind::Creature => Ok(creature(node)),
        EntryKind::Item => item(node),
        EntryKind::Class => Ok(class(node, rel)),
        EntryKind::Race | EntryKind::Skill | EntryKind::Affliction | EntryKind::Trap => {
            let mut out = Extracted::new(TypedFields::None);
            match kind {
                EntryKind::Affliction => out.tag(node.subtype.as_deref().unwrap_or_default()),
                EntryKind::Trap => out.tag(node.str_field("trap_type").unwrap_or_default()),
                _ => {}
            }
            Ok(out)
        }
        EntryKind::Rule => Ok(Extracted::new(TypedFields::None)),
    }
}

fn text(node: &Node, key: &str) -> Option<String> {
    node.str_field(key)
        .map(plain_text)
        .filter(|s| !s.is_empty())
}

fn string_list(node: &Node, key: &str) -> Result<Vec<String>, String> {
    match node.rest.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(s)) => Ok(vec![plain_text(s)]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_str()
                    .map(plain_text)
                    .ok_or_else(|| format!("`{key}` holds a non-string item"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into_iter().filter(|s| !s.is_empty()).collect()),
        Some(_) => Err(format!("`{key}` is neither a string nor a list")),
    }
}

fn split_commas(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn spell(node: &Node) -> Result<Extracted, String> {
    let school = text(node, "school").ok_or("spell has no `school`")?;
    let mut subschools = string_list(node, "subschool")?;
    if subschools.is_empty() {
        subschools = text(node, "subschool_text")
            .map(|t| split_commas(&t))
            .unwrap_or_default();
    }
    let mut descriptors = string_list(node, "descriptor")?;
    if descriptors.is_empty() {
        descriptors = text(node, "descriptor_text")
            .map(|t| split_commas(&t))
            .unwrap_or_default();
    }
    let raw_levels = node
        .rest
        .get("levels")
        .and_then(Value::as_array)
        .ok_or("spell has no `levels` list")?;
    let mut levels = Vec::new();
    for (i, item) in raw_levels.iter().enumerate() {
        let class = item
            .get("class")
            .and_then(Value::as_str)
            .filter(|c| !c.trim().is_empty());
        let level = item
            .get("level")
            .and_then(Value::as_u64)
            .and_then(|l| u8::try_from(l).ok());
        match (class, level) {
            (Some(class), Some(level)) => levels.push(SpellLevel {
                class: plain_text(class),
                level,
            }),
            _ => {
                return Err(format!(
                    "`levels[{i}]` needs a class name and a small integer level"
                ));
            }
        }
    }
    let mut effects = Vec::new();
    if let Some(items) = node.rest.get("effects") {
        let items = items.as_array().ok_or("`effects` is not a list")?;
        for item in items {
            let value = item
                .get("text")
                .and_then(Value::as_str)
                .map(plain_text)
                .unwrap_or_default();
            if value.is_empty() {
                continue;
            }
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .map(plain_text)
                .filter(|n| !n.is_empty());
            effects.push((name.unwrap_or_else(|| "Effect".into()), value));
        }
    }
    let fields = SpellFields {
        school: school.clone(),
        subschools: subschools.clone(),
        descriptors: descriptors.clone(),
        levels: levels.clone(),
        casting_time: text(node, "casting_time"),
        components: text(node, "component_text").or_else(|| components_from_list(node)),
        range: text(node, "range"),
        effects,
        duration: text(node, "duration"),
        saving_throw: text(node, "saving_throw"),
        spell_resistance: text(node, "spell_resistance"),
    };
    let mut out = Extracted::new(TypedFields::Spell(fields));
    out.tag(&school);
    out.facet("school", &school);
    for s in &subschools {
        out.tag(s);
    }
    for d in &descriptors {
        out.tag(d);
        out.facet("descriptor", d);
    }
    for l in &levels {
        out.facet("class", &l.class);
        out.facet("spell_level", &l.level.to_string());
    }
    Ok(out)
}

fn components_from_list(node: &Node) -> Option<String> {
    let items = node.rest.get("components")?.as_array()?;
    let mut letters = Vec::new();
    let mut materials = Vec::new();
    for item in items {
        let kind = item.get("type").and_then(Value::as_str)?;
        letters.push(kind.to_string());
        if let Some(t) = item.get("text").and_then(Value::as_str) {
            materials.push(plain_text(t));
        }
    }
    if letters.is_empty() {
        return None;
    }
    let mut out = letters.join(", ");
    if !materials.is_empty() {
        out.push_str(&format!(" ({})", materials.join("; ")));
    }
    Some(out)
}

/// Text of a section: its own body, then its children's bodies.
pub fn section_text(section: &Node) -> String {
    let mut parts = Vec::new();
    if let Some(body) = &section.body {
        parts.push(plain_text(body));
    }
    for child in &section.sections {
        parts.push(section_text(child));
    }
    parts.retain(|p| !p.is_empty());
    parts.join(" ")
}

/// First section at any depth whose lowercase name is one of `names`.
pub fn find_section<'a>(sections: &'a [Node], names: &[&str]) -> Option<&'a Node> {
    for s in sections {
        if s.name
            .as_deref()
            .is_some_and(|n| names.contains(&n.trim().to_lowercase().as_str()))
        {
            return Some(s);
        }
        if let Some(found) = find_section(&s.sections, names) {
            return Some(found);
        }
    }
    None
}

fn feat(node: &Node) -> Result<Extracted, String> {
    let feat_types = match node.rest.get("feat_types") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(s)) => vec![plain_text(s)],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(plain_text)
            .collect(),
        Some(Value::Object(map)) => {
            let mut types = Vec::new();
            for (key, value) in map {
                match value {
                    Value::String(s) => types.push(plain_text(s)),
                    Value::Array(items) => {
                        types.extend(items.iter().filter_map(Value::as_str).map(plain_text))
                    }
                    _ => return Err(format!("`feat_types.{key}` is neither a string nor a list")),
                }
            }
            types
        }
        Some(_) => return Err("`feat_types` has an unexpected shape".into()),
    };
    let feat_types: Vec<String> = feat_types.into_iter().filter(|t| !t.is_empty()).collect();
    let pick = |names: &[&str]| {
        node.sections
            .iter()
            .find(|s| {
                s.name
                    .as_deref()
                    .is_some_and(|n| names.contains(&n.trim().to_lowercase().as_str()))
            })
            .map(section_text)
            .filter(|t| !t.is_empty())
    };
    let fields = FeatFields {
        feat_types: feat_types.clone(),
        prerequisites: pick(&["prerequisites", "prerequisite"]),
        benefit: pick(&["benefit", "benefits"]),
        normal: pick(&["normal"]),
        special: pick(&["special"]),
    };
    let mut out = Extracted::new(TypedFields::Feat(fields));
    for t in &feat_types {
        out.tag(t);
        out.facet("feat_type", t);
    }
    Ok(out)
}

fn creature(node: &Node) -> Extracted {
    let cr = node
        .str_field("cr")
        .and_then(|v| ChallengeRating::parse(&plain_text(v)));
    let ability = |key: &str| text(node, key).unwrap_or_else(|| "-".into());
    let keys = [
        "strength",
        "dexterity",
        "constitution",
        "intelligence",
        "wisdom",
        "charisma",
    ];
    let abilities = keys
        .iter()
        .any(|k| node.rest.contains_key(*k))
        .then(|| keys.map(ability));
    let subtypes = text(node, "creature_subtype")
        .map(|t| split_commas(&t))
        .unwrap_or_default();
    let fields = CreatureFields {
        cr,
        creature_type: text(node, "creature_type"),
        subtypes: subtypes.clone(),
        size: text(node, "size"),
        alignment: text(node, "alignment"),
        ac: text(node, "ac"),
        hp: text(node, "hp"),
        fortitude: text(node, "fortitude"),
        reflex: text(node, "reflex"),
        will: text(node, "will"),
        melee: text(node, "melee"),
        ranged: text(node, "ranged"),
        special_attacks: text(node, "special_attacks"),
        special_qualities: text(node, "special_qualities"),
        abilities,
    };
    let mut out = Extracted::new(TypedFields::Creature(Box::new(fields.clone())));
    if let Some(t) = &fields.creature_type {
        out.tag(t);
        out.facet("creature_type", t);
    }
    for s in &subtypes {
        out.tag(s);
    }
    if let Some(s) = node.subtype.as_deref() {
        out.tag(s);
    }
    if let Some(s) = &fields.size {
        out.tag(s);
        out.facet("size", s);
    }
    if let Some(a) = &fields.alignment {
        out.facet("alignment", a);
    }
    if let Some(cr) = fields.cr {
        out.facets.push(("cr".into(), cr.display()));
    }
    out
}

/// `misc` is `{sub-heading: {label: value}}`; the sub-heading `null` is dropped.
pub fn misc_pairs(node: &Node) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    match node.rest.get("misc") {
        None | Some(Value::Null) => {}
        Some(Value::Object(subs)) => {
            for (sub, inner) in subs {
                let Value::Object(inner) = inner else {
                    return Err(format!("`misc.{sub}` is not an object"));
                };
                for (label, value) in inner {
                    let raw = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        _ => return Err(format!("`misc.{sub}.{label}` is not a string")),
                    };
                    out.push((sub.clone(), label.clone(), raw));
                }
            }
        }
        Some(_) => return Err("`misc` is not an object".into()),
    }
    Ok(out)
}

pub fn misc_label(sub: &str, label: &str) -> String {
    if sub == "null" {
        label.to_string()
    } else {
        format!("{sub}: {label}")
    }
}

fn item(node: &Node) -> Result<Extracted, String> {
    let pairs = misc_pairs(node)?;
    let attributes: Vec<(String, String)> = pairs
        .iter()
        .map(|(sub, label, raw)| (misc_label(sub, label), plain_text(raw)))
        .collect();
    let category = node
        .subtype
        .clone()
        .or_else(|| {
            pairs
                .iter()
                .find(|(_, l, _)| l == "Gear Type")
                .map(|(_, _, v)| plain_text(v))
        })
        .filter(|c| !c.is_empty());
    let slot = text(node, "slot").map(|s| s.to_lowercase());
    let fields = ItemFields {
        category: category.clone(),
        price: text(node, "price"),
        weight: text(node, "weight"),
        slot: slot.clone(),
        aura: text(node, "aura"),
        caster_level: text(node, "cl"),
        attributes,
    };
    let mut out = Extracted::new(TypedFields::Item(fields));
    if let Some(c) = &category {
        out.tag(c);
        out.facet("item_category", c);
    }
    if let Some(s) = &slot
        && s != "none"
    {
        out.tag(s);
        out.facet("slot", s);
    }
    Ok(out)
}

fn class(node: &Node, rel: &str) -> Extracted {
    let from_path = rel
        .split('/')
        .skip_while(|p| *p != "class")
        .nth(1)
        .filter(|p| !p.ends_with(".json"))
        .map(str::to_string);
    let class_type = node.subtype.clone().or(from_path).filter(|c| !c.is_empty());
    let skill_ranks = find_section(
        &node.sections,
        &["skill ranks per level", "skill ranks at each level"],
    )
    .map(section_text_own)
    .filter(|t| !t.is_empty());
    let class_skills = find_section(&node.sections, &["class skills", "class skill"])
        .map(section_text_own)
        .filter(|t| !t.is_empty());
    let fields = ClassFields {
        class_type: class_type.clone(),
        hit_die: text(node, "hit_die"),
        skill_ranks,
        class_skills,
    };
    let mut out = Extracted::new(TypedFields::Class(fields));
    if let Some(c) = &class_type {
        out.tag(c);
        out.facet("class_type", c);
    }
    out
}

/// Only the section's own body, without nested sections.
fn section_text_own(section: &Node) -> String {
    section.body.as_deref().map(plain_text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(json: &str) -> Node {
        Node::parse_record(json.as_bytes()).unwrap()
    }

    #[test]
    fn spell_fields_come_from_real_shapes() {
        let n = node(
            r#"{"name":"Fireball","url":"pfsrd://Core Rulebook/Spells/Fireball","source":"Core Rulebook","type":"spell",
            "school":"evocation","descriptor":["fire"],"levels":[{"class":"Wizard","level":3},{"class":"Sorcerer","level":3}],
            "effects":[{"text":"20-ft.-radius spread","name":"Area"}],"component_text":"V, S, M (a ball of bat guano and sulfur)",
            "saving_throw":"Reflex half","spell_resistance":"yes"}"#,
        );
        let e = extract(EntryKind::Spell, &n, "x.json").unwrap();
        let TypedFields::Spell(f) = &e.fields else {
            panic!()
        };
        assert_eq!(f.school, "evocation");
        assert_eq!(
            f.effects,
            vec![("Area".to_string(), "20-ft.-radius spread".to_string())]
        );
        assert_eq!(f.levels.len(), 2);
        assert!(e.facets.contains(&("spell_level".into(), "3".into())));
        assert!(e.facets.contains(&("class".into(), "wizard".into())));
    }

    #[test]
    fn spell_without_levels_is_rejected() {
        let n = node(
            r#"{"name":"X","url":"pfsrd://A/X","source":"A","type":"spell","school":"evocation"}"#,
        );
        assert!(extract(EntryKind::Spell, &n, "x.json").is_err());
    }

    #[test]
    fn feat_types_accept_the_dict_shape() {
        let n = node(
            r#"{"name":"Acrobatic","url":"pfsrd://A/Acrobatic","source":"A","type":"feat","feat_types":{"feat_type":"General"},
            "sections":[{"name":"Prerequisites","body":"<p>Dex 13</p>"},{"name":"Benefits","body":"<p class=\"stat-block-1\"> You get a +2 bonus.</p>"}]}"#,
        );
        let e = extract(EntryKind::Feat, &n, "x.json").unwrap();
        let TypedFields::Feat(f) = &e.fields else {
            panic!()
        };
        assert_eq!(f.feat_types, ["General"]);
        assert_eq!(f.prerequisites.as_deref(), Some("Dex 13"));
        assert_eq!(f.benefit.as_deref(), Some("You get a +2 bonus."));
        assert_eq!(f.normal, None);
    }

    #[test]
    fn creature_cr_parses_and_mdash_is_none() {
        let mk = |cr: &str| {
            node(&format!(
                r#"{{"name":"C","url":"pfsrd://A/C","source":"A","type":"creature","cr":"{cr}","strength":"8","creature_subtype":"human, orc"}}"#
            ))
        };
        let e = extract(EntryKind::Creature, &mk("1/2"), "x.json").unwrap();
        let TypedFields::Creature(f) = &e.fields else {
            panic!()
        };
        assert_eq!(f.cr.unwrap().display(), "1/2");
        assert_eq!(f.subtypes, ["human", "orc"]);
        assert_eq!(f.abilities.as_ref().unwrap()[0], "8");
        assert_eq!(f.abilities.as_ref().unwrap()[1], "-");
        let e = extract(EntryKind::Creature, &mk("&mdash"), "x.json").unwrap();
        let TypedFields::Creature(f) = &e.fields else {
            panic!()
        };
        assert_eq!(f.cr, None);
    }

    #[test]
    fn item_misc_flattens_in_order_and_drops_null_heading() {
        let n = node(
            r#"{"name":"Acid","url":"pfsrd://A/Acid","source":"A","type":"item","slot":"None","price":"10 gp",
            "misc":{"null":{"Gear Type":"Special Substances and Items","Craft":"DC 15"},"Weapon":{"Critical":"&times;3"}}}"#,
        );
        let e = extract(EntryKind::Item, &n, "x.json").unwrap();
        let TypedFields::Item(f) = &e.fields else {
            panic!()
        };
        assert_eq!(f.slot.as_deref(), Some("none"));
        assert_eq!(f.category.as_deref(), Some("Special Substances and Items"));
        let labels: Vec<_> = f.attributes.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["Gear Type", "Craft", "Weapon: Critical"]);
        assert_eq!(f.attributes[2].1, "\u{d7}3");
        assert!(!e.facets.iter().any(|(n, _)| n == "slot"));
    }

    #[test]
    fn class_type_comes_from_subtype_or_path() {
        let n = node(
            r#"{"name":"Fighter","url":"pfsrd://A/Fighter","source":"A","type":"class","hit_die":"d10",
            "sections":[{"name":"Class Skills","body":"<p>Climb</p>","sections":[{"name":"Skill Ranks per Level","body":"<p>2 + Int</p>"}]}]}"#,
        );
        let e = extract(
            EntryKind::Class,
            &n,
            "core_rulebook/class/prestige/fighter.json",
        )
        .unwrap();
        let TypedFields::Class(f) = &e.fields else {
            panic!()
        };
        assert_eq!(f.class_type.as_deref(), Some("prestige"));
        assert_eq!(f.skill_ranks.as_deref(), Some("2 + Int"));
        assert_eq!(f.class_skills.as_deref(), Some("Climb"));
    }
}
