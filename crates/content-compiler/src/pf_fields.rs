//! Mechanical fields read directly from Pathfinder Data's stat-block directives.
use cast_iron_schema::{
    ChallengeRating, ClassFields, CreatureFields, EntryKind, FeatFields, ItemFields, SpellFields,
    SpellLevel, TypedFields,
};
use std::collections::BTreeMap;

fn directive<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let start = line.find(&format!("::{name}"))?;
    let rest = &line[start + name.len() + 2..];
    let open = rest.find('{')?;
    let tail = &rest[open..];
    let end = tail.rfind('}')?;
    Some(&tail[1..end])
}
fn attrs(lines: &[String], name: &str) -> Result<Option<BTreeMap<String, String>>, String> {
    for line in lines {
        if let Some(d) = directive(line, name) {
            return crate::pf_markdown::attributes(d).map(Some);
        }
    }
    Ok(None)
}
fn get(a: &BTreeMap<String, String>, k: &str) -> Option<String> {
    a.get(k).filter(|v| !v.is_empty()).cloned()
}
fn words(a: &BTreeMap<String, String>, key: &str) -> Vec<String> {
    get(a, key)
        .unwrap_or_default()
        .split('~')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn reject_unknown(
    a: &BTreeMap<String, String>,
    directive: &str,
    allowed: &[&str],
) -> Result<(), String> {
    if let Some(key) = a.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("unsupported ::{directive} attribute `{key}`"));
    }
    Ok(())
}

pub fn extract(kind: EntryKind, lines: &[String]) -> Result<TypedFields, String> {
    match kind {
        EntryKind::Spell => spell(lines),
        EntryKind::Feat => feat(lines),
        EntryKind::Item => item(lines),
        EntryKind::Class | EntryKind::PrestigeClass => class(kind, lines),
        EntryKind::Creature => creature(lines),
        EntryKind::Trait => trait_fields(lines),
        _ => Ok(TypedFields::None),
    }
}

/// Traits carry their category in the `::trait` directive. Schema validation rejects
/// `TypedFields::None` for this kind, so a missing directive still gets a valid category.
fn trait_fields(lines: &[String]) -> Result<TypedFields, String> {
    let Some(a) = attrs(lines, "trait")? else {
        return Ok(TypedFields::Trait(cast_iron_schema::TraitFields {
            category: "Other".into(),
            detail: None,
        }));
    };
    reject_unknown(
        &a,
        "trait",
        &[
            "title", "type", "flavor", "benefit", "req", "region", "religion", "subs", "surs",
            "campaign",
        ],
    )?;
    let raw = get(&a, "type").unwrap_or_else(|| "Other".into());
    let category = match raw.to_ascii_lowercase().as_str() {
        "combat" => "Combat",
        "faith" => "Faith",
        "magic" => "Magic",
        "social" => "Social",
        "campaign" => "Campaign",
        "regional" => "Regional",
        "religion" => "Religion",
        "race" => "Race",
        "drawback" => "Drawback",
        _ => raw.as_str(),
    }
    .to_string();
    Ok(TypedFields::Trait(cast_iron_schema::TraitFields {
        category,
        detail: None,
    }))
}
fn spell(lines: &[String]) -> Result<TypedFields, String> {
    let a = attrs(lines, "spell")?.ok_or("spell has no ::spell directive")?;
    const SPELL_ATTRS: &[&str] = &[
        "id",
        "source",
        "faith",
        "school",
        "abjuration",
        "conjuration",
        "divination",
        "enchantment",
        "illusion",
        "necromancy",
        "transmutation",
        "evocation",
        "universal",
        "compulsion",
        "charm",
        "creation",
        "calling",
        "shadowSub",
        "summoning",
        "teleportation",
        "healing",
        "figment",
        "glamer",
        "pattern",
        "phantasm",
        "polymorph",
        "scrying",
        "acid",
        "air",
        "chaotic",
        "cold",
        "curse",
        "darkness",
        "death",
        "disease",
        "draconic",
        "earth",
        "electricity",
        "emotion",
        "evil",
        "fear",
        "fire",
        "force",
        "good",
        "haunted",
        "languageDependant",
        "languageDependent",
        "lawful",
        "light",
        "meditative",
        "mindAffecting",
        "pain",
        "poison",
        "ruse",
        "shadowDesc",
        "sonic",
        "water",
        "adp",
        "alc",
        "ant",
        "arc",
        "bld",
        "brd",
        "clr",
        "drd",
        "hnt",
        "inq",
        "inv",
        "mag",
        "med",
        "mes",
        "occ",
        "orc",
        "pal",
        "psy",
        "rgr",
        "rma",
        "sha",
        "skd",
        "sor",
        "spr",
        "sum",
        "usm",
        "war",
        "wit",
        "wiz",
        "ct",
        "ctFRA",
        "ctIm",
        "ctSw",
        "ctSt",
        "ctH",
        "ctM",
        "ctR",
        "cSpecial",
        "cV",
        "cS",
        "cM",
        "cDF",
        "cF",
        "cMDF",
        "cFDF",
        "cMp",
        "cFp",
        "cMDFp",
        "cFDFp",
        "cText",
        "r",
        "rTouch",
        "rPers",
        "rClose",
        "rMed",
        "rLong",
        "rText",
        "rFt",
        "target",
        "targets",
        "targetOrTargets",
        "area",
        "areaOrTarget",
        "targetOrArea",
        "effect",
        "dur",
        "durR",
        "durM",
        "durH",
        "durC",
        "durCon",
        "durI",
        "durP",
        "durRL",
        "durML",
        "durHL",
        "durDL",
        "durD",
        "save",
        "saveNo",
        "fort",
        "fortHalf",
        "fortPartial",
        "refl",
        "reflHalf",
        "reflPartial",
        "will",
        "willDisbelief",
        "willHalf",
        "willPartial",
        "sr",
        "srY",
        "srN",
        "svHarmless",
        "svObject",
        "srHarmless",
        "srObject",
        "harmless",
        "object",
    ];
    reject_unknown(&a, "spell", SPELL_ATTRS)?;
    let school_flags = [
        "abjuration",
        "conjuration",
        "divination",
        "enchantment",
        "evocation",
        "illusion",
        "necromancy",
        "transmutation",
        "universal",
    ];
    let active_schools = school_flags.iter().filter(|k| a.contains_key(**k)).count()
        + usize::from(a.get("school").is_some_and(|s| !s.is_empty()));
    if active_schools != 1 {
        return Err(format!(
            "spell directive has {active_schools} school values; expected exactly one"
        ));
    }
    let school = get(&a, "school")
        .or_else(|| {
            [
                "abjuration",
                "conjuration",
                "divination",
                "enchantment",
                "evocation",
                "illusion",
                "necromancy",
                "transmutation",
                "universal",
            ]
            .iter()
            .find(|k| a.contains_key(**k))
            .map(|s| s.to_string())
        })
        .ok_or("spell directive has no school")?;
    let abbreviations = [
        ("adp", "Adept"),
        ("alc", "Alchemist"),
        ("ant", "Antipaladin"),
        ("arc", "Arcanist"),
        ("bld", "Bloodrager"),
        ("brd", "Bard"),
        ("clr", "Cleric"),
        ("drd", "Druid"),
        ("hnt", "Hunter"),
        ("inq", "Inquisitor"),
        ("inv", "Investigator"),
        ("mag", "Magus"),
        ("med", "Medium"),
        ("mes", "Mesmerist"),
        ("occ", "Occultist"),
        ("orc", "Oracle"),
        ("pal", "Paladin"),
        ("psy", "Psychic"),
        ("rgr", "Ranger"),
        ("rma", "Red Mantis Assassin"),
        ("sha", "Shaman"),
        ("skd", "Skald"),
        ("sor", "Sorcerer"),
        ("spr", "Spiritualist"),
        ("sum", "Summoner"),
        ("usm", "Unchained Summoner"),
        ("war", "Warpriest"),
        ("wit", "Witch"),
        ("wiz", "Wizard"),
    ];
    let mut levels = Vec::new();
    for (k, n) in abbreviations {
        if let Some(v) = a.get(k) {
            let level = v
                .parse()
                .map_err(|_| format!("invalid spell level {k}={v}"))?;
            if level > 9 {
                return Err(format!("spell level {k}={level} is outside 0..=9"));
            }
            levels.push(SpellLevel {
                class: n.into(),
                level,
            });
        }
    }
    if levels.is_empty() {
        return Err("spell directive has no recognized class levels".into());
    }
    let subschools = [
        ("compulsion", "Compulsion"),
        ("charm", "Charm"),
        ("creation", "Creation"),
        ("calling", "Calling"),
        ("shadowSub", "Shadow"),
        ("summoning", "Summoning"),
        ("teleportation", "Teleportation"),
        ("healing", "Healing"),
        ("figment", "Figment"),
        ("glamer", "Glamer"),
        ("pattern", "Pattern"),
        ("phantasm", "Phantasm"),
        ("polymorph", "Polymorph"),
        ("scrying", "Scrying"),
    ]
    .iter()
    .filter(|(k, _)| a.contains_key(*k))
    .map(|(_, v)| v.to_string())
    .collect();
    let descriptors = [
        ("acid", "acid"),
        ("air", "air"),
        ("chaotic", "chaotic"),
        ("cold", "cold"),
        ("curse", "curse"),
        ("darkness", "darkness"),
        ("death", "death"),
        ("disease", "disease"),
        ("draconic", "draconic"),
        ("earth", "earth"),
        ("electricity", "electricity"),
        ("emotion", "emotion"),
        ("evil", "evil"),
        ("fear", "fear"),
        ("fire", "fire"),
        ("force", "force"),
        ("good", "good"),
        ("glamer", "glamer"),
        ("haunted", "haunted"),
        ("languageDependant", "language-dependant"),
        ("languageDependent", "language-dependant"),
        ("lawful", "lawful"),
        ("light", "light"),
        ("meditative", "meditative"),
        ("mindAffecting", "mind-affecting"),
        ("pain", "pain"),
        ("poison", "poison"),
        ("ruse", "ruse"),
        ("shadowDesc", "shadow"),
        ("sonic", "sonic"),
        ("water", "water"),
    ]
    .iter()
    .filter(|(k, _)| a.contains_key(*k))
    .map(|(_, v)| v.to_string())
    .collect();
    let cast_flags = ["ctFRA", "ctIm", "ctSw", "ctSt", "ctH", "ctM", "ctR"];
    if a.get("ct").is_some_and(|v| !v.is_empty()) && cast_flags.iter().any(|k| a.contains_key(*k))
        || cast_flags.iter().filter(|k| a.contains_key(**k)).count() > 1
    {
        return Err("spell directive has conflicting casting-time values".into());
    }
    let casting = get(&a, "ct").or_else(|| {
        if a.contains_key("ctSt") {
            Some("1 standard action".into())
        } else if a.contains_key("ctFRA") {
            Some("1 full-round action".into())
        } else if a.contains_key("ctSw") {
            Some("1 swift action".into())
        } else if a.contains_key("ctIm") {
            Some("1 immediate action".into())
        } else {
            [("ctH", "hour"), ("ctM", "minute"), ("ctR", "round")]
                .iter()
                .find_map(|(k, u)| {
                    a.get(*k).map(|v| {
                        format!(
                            "{v} {}",
                            if v == "1" {
                                *u
                            } else {
                                match *u {
                                    "hour" => "hours",
                                    "minute" => "minutes",
                                    _ => "rounds",
                                }
                            }
                        )
                    })
                })
        }
    });
    let casting = casting.ok_or("spell directive has no casting time")?;
    let components = get(&a, "cSpecial").or_else(|| {
        let mut c = Vec::new();
        for (k, v) in [
            ("cV", "V"),
            ("cS", "S"),
            ("cM", "M"),
            ("cDF", "DF"),
            ("cF", "F"),
            ("cMDF", "M/DF"),
            ("cFDF", "F/DF"),
        ] {
            if a.contains_key(k) {
                c.push(v.to_string())
            }
        }
        for (k, v) in [
            ("cMp", "M"),
            ("cFp", "F"),
            ("cMDFp", "M/DF"),
            ("cFDFp", "F/DF"),
        ] {
            if let Some(x) = a.get(k) {
                c.push(format!("{v} ({x})"))
            }
        }
        (!c.is_empty()).then(|| c.join(", "))
    });
    if a.contains_key("cSpecial")
        && [
            "cV", "cS", "cM", "cDF", "cF", "cMDF", "cFDF", "cMp", "cFp", "cMDFp", "cFDFp", "cText",
        ]
        .iter()
        .any(|k| a.contains_key(*k))
    {
        return Err("spell directive combines cSpecial with component flags".into());
    }
    if ["cM", "cMp", "cMDF", "cMDFp"]
        .iter()
        .filter(|k| a.contains_key(**k))
        .count()
        > 1
        || ["cF", "cFp", "cFDFp"]
            .iter()
            .filter(|k| a.contains_key(**k))
            .count()
            > 1
        || ["cDF", "cMDF", "cMDFp", "cFDFp"]
            .iter()
            .filter(|k| a.contains_key(**k))
            .count()
            > 1
    {
        return Err("spell directive combines conflicting component flags".into());
    }
    if !a.contains_key("cSpecial")
        && ![
            "cV", "cS", "cM", "cDF", "cF", "cMDF", "cFDF", "cMp", "cFp", "cMDFp", "cFDFp",
        ]
        .iter()
        .any(|k| a.contains_key(*k))
    {
        return Err("spell directive has no component semantics".into());
    }
    let range_flags = ["rTouch", "rPers", "rClose", "rMed", "rLong", "rText", "rFt"];
    if a.get("r").is_some_and(|v| !v.is_empty()) && range_flags.iter().any(|k| a.contains_key(*k))
        || range_flags.iter().filter(|k| a.contains_key(**k)).count() > 1
    {
        return Err("spell directive has conflicting range values".into());
    }
    let range = get(&a, "r").or_else(|| {
        [
            ("rTouch", "touch"),
            ("rPers", "personal"),
            ("rClose", "close (25 ft. + 5 ft./2 levels)"),
            ("rMed", "medium (100 ft. + 10 ft./level)"),
            ("rLong", "long (400 ft. + 40 ft./level)"),
            ("rText", "see text"),
        ]
        .iter()
        .find_map(|(k, v)| a.contains_key(*k).then(|| (*v).into()))
        .or_else(|| a.get("rFt").map(|x| format!("{x} ft.")))
    });
    let effects = [
        ("target", "Target"),
        ("targets", "Targets"),
        ("targetOrTargets", "Target or Targets"),
        ("area", "Area"),
        ("areaOrTarget", "Area or Target"),
        ("targetOrArea", "Target or Area"),
        ("effect", "Effect"),
    ]
    .iter()
    .filter_map(|(k, label)| a.get(*k).map(|v| ((*label).to_string(), v.clone())))
    .collect();
    let duration = get(&a, "dur").or_else(|| {
        [
            ("durI", "instantaneous"),
            ("durP", "permanent"),
            ("durC", "concentration"),
        ]
        .iter()
        .find_map(|(k, v)| a.contains_key(*k).then(|| (*v).into()))
        .or_else(|| {
            [
                ("durR", "round"),
                ("durM", "minute"),
                ("durH", "hour"),
                ("durRL", "round/level"),
                ("durML", "minute/level"),
                ("durHL", "hour/level"),
                ("durDL", "day/level"),
            ]
            .iter()
            .find_map(|(k, unit)| {
                a.get(*k).map(|v| {
                    let (base, per_level) = unit
                        .split_once("/level")
                        .map_or((*unit, false), |(b, _)| (b, true));
                    let plural = v != "1";
                    let base = if plural {
                        format!("{base}s")
                    } else {
                        base.to_string()
                    };
                    if per_level {
                        format!("{v} {base}/level")
                    } else {
                        format!("{v} {base}")
                    }
                })
            })
        })
    });
    let duration =
        duration.or_else(|| get(&a, "durCon").map(|suffix| format!("concentration{suffix}")));
    let duration = duration.map(|mut d| {
        if let Some(suffix) = get(&a, "durCon")
            && !d.starts_with("concentration")
        {
            d.push_str(&suffix);
        }
        if a.contains_key("durD") {
            d.push_str(" (D)");
        }
        d
    });
    let duration = duration.ok_or("spell directive has no duration")?;
    let saving = get(&a, "save").or_else(|| {
        [
            ("saveNo", "none"),
            ("fort", "Fortitude negates"),
            ("fortHalf", "Fortitude half"),
            ("fortPartial", "Fortitude partial"),
            ("refl", "Reflex negates"),
            ("reflHalf", "Reflex half"),
            ("reflPartial", "Reflex partial"),
            ("will", "Will negates"),
            ("willHalf", "Will half"),
            ("willPartial", "Will partial"),
            ("willDisbelief", "Will disbelief"),
        ]
        .iter()
        .find_map(|(k, v)| a.contains_key(*k).then(|| (*v).into()))
    });
    let sr = get(&a, "sr").or_else(|| {
        if a.contains_key("srY") {
            Some("yes".into())
        } else if a.contains_key("srN") {
            Some("no".into())
        } else {
            None
        }
    });
    let saving = saving.map(|mut v| {
        if a.contains_key("harmless") || a.contains_key("svHarmless") {
            v.push_str(" (harmless)");
        }
        if a.contains_key("object") || a.contains_key("svObject") {
            v.push_str(" (object)");
        }
        v
    });
    // Some source spells omit the line entirely. Render that absence explicitly
    // instead of failing the whole record.
    let saving = saving.unwrap_or_else(|| "none".into());
    let sr = sr.map(|mut v| {
        if a.contains_key("harmless") || a.contains_key("srHarmless") {
            v.push_str(" (harmless)");
        }
        if a.contains_key("object") || a.contains_key("srObject") {
            v.push_str(" (object)");
        }
        v
    });
    let sr = sr.unwrap_or_else(|| "no".into());
    let components = components.map(|mut v| {
        if a.contains_key("cText") {
            v.push_str("; see text");
        }
        v
    });
    Ok(TypedFields::Spell(SpellFields {
        school,
        subschools,
        descriptors,
        levels,
        casting_time: Some(casting),
        components,
        range,
        effects,
        duration: Some(duration),
        saving_throw: Some(saving),
        spell_resistance: Some(sr),
    }))
}
fn feat(lines: &[String]) -> Result<TypedFields, String> {
    let mut f = FeatFields::default();
    // Keep this aligned with the feat_type metadata consumed by fields.rs. Headings
    // often include parenthetical prose, so only recognized feat categories count.
    const FEAT_TYPES: &[&str] = &[
        "Achievement",
        "Alignment",
        "Armor Mastery",
        "Betrayal",
        "Blood Hex",
        "BoA",
        "Called Shot",
        "Combat",
        "Combination",
        "Conduit",
        "Coven",
        "Critical",
        "Damnation",
        "Dhampir",
        "Esoteric",
        "Faction",
        "Familiar",
        "Fetchling",
        "Gathlain Court Title",
        "Gillman",
        "Grit",
        "Hero Point",
        "Item Creation",
        "Item Mastery",
        "MC",
        "Meditation",
        "Metamagic",
        "Monster",
        "Mythic",
        "Origin",
        "Panache",
        "Performance",
        "Shield Mastery",
        "Stare",
        "Story",
        "Style",
        "Taldor Variant",
        "Targeting",
        "Teamwork",
        "Trick",
        "UM",
        "UW",
        "Weapon Mastery",
        "Words of Power",
        "Zon-Kuthon's Kiss",
    ];
    for l in lines {
        let heading = l.trim().trim_start_matches('#').trim();
        if let Some((_, tail)) = heading.rsplit_once('(')
            && let Some(types) = tail.strip_suffix(')')
        {
            let candidate: Vec<_> = types.split(',').map(str::trim).collect();
            if !candidate.is_empty() && candidate.iter().all(|t| FEAT_TYPES.contains(t)) {
                f.feat_types = candidate.into_iter().map(str::to_owned).collect();
            }
        }
        for label in [
            "Prerequisites:",
            "Prerequisite:",
            "Benefit:",
            "Normal:",
            "Special:",
        ] {
            if let Some(i) = l.find(label) {
                let v = l[i + label.len()..]
                    .trim()
                    .trim_matches('*')
                    .trim()
                    .to_string();
                if !v.is_empty() {
                    match label {
                        "Prerequisites:" | "Prerequisite:" => f.prerequisites = Some(v),
                        "Benefit:" => f.benefit = Some(v),
                        "Normal:" => f.normal = Some(v),
                        "Special:" => f.special = Some(v),
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
    Ok(TypedFields::Feat(f))
}
fn item(lines: &[String]) -> Result<TypedFields, String> {
    let mut f = ItemFields::default();
    let a = attrs(lines, "item")?
        .or(attrs(lines, "equipment")?)
        .unwrap_or_default();
    f.category = get(&a, "category").or(get(&a, "type"));
    f.price = get(&a, "price");
    f.weight = get(&a, "weight");
    f.slot = get(&a, "slot");
    f.aura = get(&a, "aura");
    f.caster_level = get(&a, "cl").or(get(&a, "casterlevel"));
    for (k, v) in a {
        if ![
            "category",
            "type",
            "price",
            "weight",
            "slot",
            "aura",
            "cl",
            "casterlevel",
            "id",
            "source",
        ]
        .contains(&k.as_str())
        {
            f.attributes.push((k, v))
        }
    }
    for l in lines {
        for (labels, slot) in [
            (&["Price", "Price:"][..], &mut f.price),
            (&["Weight", "Weight:"][..], &mut f.weight),
            (&["Slot", "Slot:"][..], &mut f.slot),
            (&["Aura", "Aura:"][..], &mut f.aura),
            (&["CL", "Caster Level:"][..], &mut f.caster_level),
        ] {
            for label in labels {
                let marker = format!("**{label}**");
                if let Some(i) = l.find(&marker) {
                    let tail = l[i + marker.len()..].trim_start_matches([' ', ':']);
                    let n = tail.find("; **").unwrap_or(tail.len());
                    let value = tail[..n].trim();
                    if !value.is_empty() && slot.is_none() {
                        *slot = Some(value.to_string());
                    }
                    break;
                }
            }
        }
    }
    Ok(TypedFields::Item(f))
}
fn class(kind: EntryKind, lines: &[String]) -> Result<TypedFields, String> {
    let a = attrs(lines, "class")?.unwrap_or_default();
    let mut f = ClassFields {
        class_type: get(&a, "class_type")
            .or_else(|| (kind == EntryKind::PrestigeClass).then(|| "prestige".into())),
        hit_die: get(&a, "hd").map(|x| format!("d{x}")),
        skill_ranks: get(&a, "skp").map(|x| format!("{x} + Int modifier")),
        class_skills: None,
    };
    for l in lines {
        for (label, slot) in [
            ("Hit Die:", &mut f.hit_die),
            ("Class Skills:", &mut f.class_skills),
            ("Skill Ranks per Level:", &mut f.skill_ranks),
        ] {
            if let Some(i) = l.find(label) {
                let x = l[i + label.len()..].trim().trim_matches('*').trim();
                if !x.is_empty() {
                    *slot = Some(x.into())
                }
            }
        }
    }
    if f.hit_die.is_none() {
        return Err("class has no hit die".into());
    }
    Ok(TypedFields::Class(f))
}
fn creature(lines: &[String]) -> Result<TypedFields, String> {
    let mut f = CreatureFields::default();
    if let Some(a) = attrs(lines, "minfo")? {
        f.cr = get(&a, "cr").and_then(|x| ChallengeRating::parse(&x));
        f.creature_type = get(&a, "type").or_else(|| {
            [
                "aberration",
                "animal",
                "construct",
                "dragon",
                "fey",
                "humanoid",
                "magicalBeast",
                "monstrousHumanoid",
                "ooze",
                "outsider",
                "plant",
                "undead",
                "vermin",
            ]
            .iter()
            .find(|k| a.contains_key(**k))
            .map(|x| x.to_string())
        });
        f.subtypes = words(&a, "subtypes");
        if f.subtypes.is_empty() {
            f.subtypes = words(&a, "subs");
        }
        f.size = get(&a, "size").or_else(|| {
            [
                "fine",
                "diminutive",
                "tiny",
                "small",
                "medium",
                "large",
                "huge",
                "gargantuan",
                "colossal",
            ]
            .iter()
            .find(|k| a.contains_key(**k))
            .map(|x| x.to_string())
        });
        f.alignment = get(&a, "al").or_else(|| {
            [
                ("lg", "lawful good"),
                ("ln", "lawful neutral"),
                ("le", "lawful evil"),
                ("ng", "neutral good"),
                ("n", "neutral"),
                ("ne", "neutral evil"),
                ("cg", "chaotic good"),
                ("cn", "chaotic neutral"),
                ("ce", "chaotic evil"),
            ]
            .iter()
            .find_map(|(k, v)| a.contains_key(*k).then(|| (*v).into()))
        });
    }
    if let Some(a) = attrs(lines, "mdefense")? {
        f.ac = get(&a, "ac");
        f.hp = get(&a, "hpRaw").or(get(&a, "hp"));
        f.fortitude = get(&a, "fort");
        f.reflex = get(&a, "ref").or(get(&a, "reflex"));
        f.will = get(&a, "will");
    }
    if let Some(a) = attrs(lines, "moffense")? {
        f.melee = get(&a, "melee");
        f.ranged = get(&a, "ranged");
        f.special_attacks = get(&a, "specAtt")
            .or(get(&a, "sa"))
            .or(get(&a, "specialattacks"));
    }
    if let Some(a) = attrs(lines, "mstats")? {
        f.special_qualities = get(&a, "sq");
        if let Some(s) = get(&a, "str") {
            f.abilities = Some([
                s,
                get(&a, "dex").unwrap_or_default(),
                get(&a, "con").unwrap_or_default(),
                get(&a, "int").unwrap_or_default(),
                get(&a, "wis").unwrap_or_default(),
                get(&a, "cha").unwrap_or_default(),
            ]);
        }
    }
    Ok(TypedFields::Creature(Box::new(f)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_real_spell_directive_and_rejects_missing_levels() {
        let lines = vec![r#"::spell{id=1 source="PRPG Core Rulebook/341" polymorph transmutation arc=9 drd=9 sha=9 sor=9 wiz=9 ctSt cV cS cFp="jade circlet worth 1,500 gp" rPers target=you durML=10 durD saveNo srN}"#.into()];
        let TypedFields::Spell(f) = extract(EntryKind::Spell, &lines).unwrap() else {
            panic!()
        };
        assert_eq!(f.school, "transmutation");
        assert_eq!(f.levels.len(), 5);
        assert_eq!(f.casting_time.as_deref(), Some("1 standard action"));
        assert!(f.components.as_deref().unwrap().contains("jade circlet"));
        let invalid = vec!["::spell{transmutation ctSt}".into()];
        assert!(extract(EntryKind::Spell, &invalid).is_err());
        let unsupported = vec!["::spell{transmutation arc=1 cV mystery=3}".into()];
        assert!(
            extract(EntryKind::Spell, &unsupported)
                .unwrap_err()
                .contains("mystery")
        );
    }

    #[test]
    fn reads_realm_retribution_spell_stats_without_losing_directive_values() {
        let lines = vec![r#"::spell{id=1 source="Chronicle of Legends/14" conjuration teleportation clr=8 inq=6 orc=8 war=6 ctSt cV cS rTouch target="creature touched" durRL=1 save="Will negates (see text)" srN}"#.into()];
        let TypedFields::Spell(f) = extract(EntryKind::Spell, &lines).unwrap() else {
            panic!()
        };
        assert_eq!(f.school, "conjuration");
        assert_eq!(f.subschools, ["Teleportation"]);
        assert_eq!(f.levels.len(), 4);
        assert_eq!(f.casting_time.as_deref(), Some("1 standard action"));
        assert_eq!(f.components.as_deref(), Some("V, S"));
        assert_eq!(f.range.as_deref(), Some("touch"));
        assert_eq!(f.effects, [("Target".into(), "creature touched".into())]);
        assert_eq!(f.duration.as_deref(), Some("1 round/level"));
        assert_eq!(f.saving_throw.as_deref(), Some("Will negates (see text)"));
        assert_eq!(f.spell_resistance.as_deref(), Some("no"));
    }

    #[test]
    fn maps_concentration_range_formulas_and_feat_heading_types() {
        let base = "transmutation arc=1 ctSt cV cS rClose target=you";
        let spell = format!("::spell{{{base} durCon=\", up to a minute\" saveNo srN}}");
        let TypedFields::Spell(f) = extract(EntryKind::Spell, &[spell]).unwrap() else {
            panic!()
        };
        assert_eq!(f.duration.as_deref(), Some("concentration, up to a minute"));
        assert_eq!(f.range.as_deref(), Some("close (25 ft. + 5 ft./2 levels)"));
        let TypedFields::Feat(f) = extract(
            EntryKind::Feat,
            &[
                "## Apocalyptic Spell (Metamagic)".into(),
                "## Arcing Weapon (Combat)".into(),
            ],
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(f.feat_types, ["Combat"]);
        let TypedFields::Feat(f) = extract(
            EntryKind::Feat,
            &["## Example (A parenthetical aside)".into()],
        )
        .unwrap() else {
            panic!()
        };
        assert!(f.feat_types.is_empty());
    }

    #[test]
    fn reads_feat_and_class_labels_and_directives() {
        let feat = vec![
            "**Prerequisites:** Dex 13; Dodge".into(),
            "**Benefit:** You gain a bonus.".into(),
        ];
        let TypedFields::Feat(f) = extract(EntryKind::Feat, &feat).unwrap() else {
            panic!()
        };
        assert_eq!(f.prerequisites.as_deref(), Some("Dex 13; Dodge"));
        assert_eq!(f.benefit.as_deref(), Some("You gain a bonus."));
        let class = vec!["::class{any hd=8 wealth=3 app craft dd fly heal ka kna per prof soh spc sur umd skp=4}".into()];
        let TypedFields::Class(c) = extract(EntryKind::Class, &class).unwrap() else {
            panic!()
        };
        assert_eq!(c.hit_die.as_deref(), Some("d8"));
        assert_eq!(c.skill_ranks.as_deref(), Some("4 + Int modifier"));
    }

    #[test]
    fn reads_monster_stat_directives() {
        let lines = vec![
            "::minfo{source=Bestiary/1 xp=1200 ne large undead init=-1 tremorsense=30 dv=60 pcp=+3}".into(),
            "::mdefense{ac=18/9/17 hp=51~6d10+18 fort=+5 ref=+2 will=+4}".into(),
            "::moffense{sp=5 melee=\"2 roots +7\" specAtt=\"bloody sap~drag under\"}".into(),
        ];
        let TypedFields::Creature(f) = extract(EntryKind::Creature, &lines).unwrap() else {
            panic!()
        };
        assert_eq!(f.creature_type.as_deref(), Some("undead"));
        assert_eq!(f.ac.as_deref(), Some("18/9/17"));
        assert_eq!(f.melee.as_deref(), Some("2 roots +7"));
    }

    #[test]
    fn reads_trait_category_from_the_directive() {
        let lines = vec![
            r#"::trait{title="&L&Abject Belief&FN&" type=Faith flavor="..." benefit="..."}"#.into(),
        ];
        let TypedFields::Trait(f) = extract(EntryKind::Trait, &lines).unwrap() else {
            panic!()
        };
        assert_eq!(f.category, "Faith");
        let TypedFields::Trait(f) = extract(EntryKind::Trait, &[]).unwrap() else {
            panic!()
        };
        assert_eq!(f.category, "Other");
    }

    #[test]
    fn spell_without_save_or_sr_records_explicit_defaults() {
        let lines = vec![
            r#"::spell{id=1 source="Ultimate Combat/221" abjuration force arc=1 ctSt cV cS rPers target=you dur="1 round/level or until discharged"}"#.into(),
        ];
        let TypedFields::Spell(f) = extract(EntryKind::Spell, &lines).unwrap() else {
            panic!()
        };
        assert_eq!(f.saving_throw.as_deref(), Some("none"));
        assert_eq!(f.spell_resistance.as_deref(), Some("no"));
    }
}
