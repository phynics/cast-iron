//! Parser for Pathfinder's Markdown dialect. The upstream syntax is converted at the
//! compiler boundary; only the existing typed HTML adapter produces schema nodes.
use crate::slug::slug_segment;
use cast_iron_schema::Block;
use pulldown_cmark::{Options, Parser, html};
use std::collections::BTreeMap;

/// Parse a directive's brace attributes. Bare words are flags with an empty value.
pub fn attributes(input: &str) -> Result<BTreeMap<String, String>, String> {
    let input = input.trim();
    let input = if input.starts_with("::") {
        let open = input.find('{').ok_or("directive has no attribute braces")?;
        let close = input
            .rfind('}')
            .ok_or("unterminated directive attributes")?;
        if close < open || !input[close + 1..].trim().is_empty() {
            return Err("malformed directive attribute braces".into());
        }
        &input[open + 1..close]
    } else if input.contains(['{', '}']) {
        return Err("malformed attribute braces".into());
    } else {
        input
    };
    let mut out = BTreeMap::new();
    let mut chars = input.chars().peekable();
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
        if out.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate attribute {key}"));
        }
    }
    Ok(out)
}

/// Map source statblock IDs to the stable heading slugs used by typed content.
pub fn anchors(lines: &[String]) -> BTreeMap<String, String> {
    lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with("::") {
                return None;
            }
            let (name, rest) = split_directive(trimmed);
            let name = name.trim_start_matches(':');
            if name == "mfn" {
                let text = bracket(rest)?;
                let (mark, _) = text.split_once('~')?;
                let slug = slug_segment(mark);
                return (!slug.is_empty()).then(|| (mark.to_string(), format!("footnote-{slug}")));
            }
            if name != "sh" {
                return None;
            }
            let attrs = attributes(trimmed).ok()?;
            let id = attrs.get("id")?;
            let slug = slug_segment(&bracket(rest)?);
            (!slug.is_empty()).then(|| (id.clone(), slug))
        })
        .collect()
}

/// Turn a source encoded link into its literal href and visible text.
pub fn link_parts(text: &str) -> (String, String) {
    let s = text.strip_prefix('‹').unwrap_or(text);
    let s = s.strip_suffix('›').unwrap_or(s);
    let Some((protocol, body)) = s.split_once('/') else {
        return (String::new(), s.to_string());
    };
    let (display, url_text) = split_extras(body);
    (format!("{protocol}/{url_text}"), display)
}
fn split_extras(s: &str) -> (String, String) {
    // Forward pairs mark extra text/URL inside the pair. Reverse pairs mark the
    // extra outside the pair, which also handles the upstream edge shorthand.
    let first_text = s.chars().find(|c| matches!(c, '«' | '»'));
    let first_url = s.chars().find(|c| matches!(c, '<' | '>'));
    let mut text_only = first_text == Some('»');
    let mut url_only = first_url == Some('>');
    let (mut display, mut slug) = (String::new(), String::new());
    let mut normal = String::new();
    for c in s.chars() {
        match c {
            '«' | '»' => {
                flush_slug(&mut normal, &mut slug);
                text_only = !text_only;
            }
            '›' if text_only => {
                flush_slug(&mut normal, &mut slug);
                text_only = false;
            }
            '<' | '>' => {
                flush_slug(&mut normal, &mut slug);
                url_only = !url_only;
            }
            _ => {
                if !url_only {
                    display.push(c);
                }
                if url_only {
                    if c == '_' && slug.ends_with('_') {
                        slug.pop();
                    }
                    slug.push(c);
                } else if !text_only {
                    normal.push(c);
                }
            }
        }
    }
    flush_slug(&mut normal, &mut slug);
    (display, slug)
}

fn flush_slug(normal: &mut String, slug: &mut String) {
    slug.push_str(&normalize_slug(normal));
    normal.clear();
}

fn normalize_slug(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else if matches!(c, ' ' | '-' | '/') {
                '_'
            } else {
                '\0'
            }
        })
        .filter(|c| *c != '\0')
        .collect()
}

/// Render Pathfinder Markdown and its supported directives into typed rich content.
pub fn render(lines: &[String], env: &mut dyn crate::html::Env) -> Result<Vec<Block>, Vec<String>> {
    let mut errors = Vec::new();
    let mut md = String::new();
    let mut heading_anchor_slugs = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("::") {
            if trimmed == ":::" {
                continue;
            }
            let (raw_name, rest) = split_directive(trimmed);
            let name = raw_name.trim_start_matches(':');
            let allowed_headers = ["h2", "h3", "h4", "h5", "h6", "gh", "sh"];
            let generic = [
                "spell",
                "drug",
                "trap",
                "haunt",
                "aff",
                "archetype",
                "th",
                "mh",
                "fh",
                "ph",
                "prereq",
                "minfo",
                "mdefense",
                "ab",
                "cskill",
                "moffense",
                "mstats",
                "meco",
                "mspell",
                "mfn",
                "trait",
                "rtraits",
            ];
            if allowed_headers.contains(&name) {
                let title = bracket(rest).unwrap_or_default();
                if rest.contains('{') {
                    match attributes(trimmed) {
                        Ok(attrs) => {
                            for key in attrs.keys() {
                                // `jl` is presentation-only justification.
                                if !["id", "clear", "jl"].contains(&key.as_str()) {
                                    errors.push(format!("unknown attribute {key} on ::{name}"));
                                }
                            }
                            if let Some(id) = attrs.get("id") {
                                let slug = slug_segment(&title);
                                if slug.is_empty() {
                                    errors.push(format!(
                                        "cannot derive anchor for ::{name} with empty title"
                                    ));
                                } else {
                                    heading_anchor_slugs.push(slug);
                                }
                                if id.is_empty() {
                                    errors.push(format!("empty id on ::{name}"));
                                }
                            }
                        }
                        Err(error) => errors.push(error),
                    }
                }
                let level = name
                    .strip_prefix('h')
                    .and_then(|level| level.parse().ok())
                    .unwrap_or(3);
                md.push_str(&format!(
                    "\n\n<h{level}>{}</h{level}>\n\n",
                    escape_html(&title)
                ));
            } else if name == "spelldeitynote" {
                md.push_str("\n\n<p>Some spells are more common among the worshipers of a god. Worshipers of a spell's associated deity always treat the spell as common, and need not research it in order to prepare or learn it. This spell is available to members of other faiths, though some temples or religious organizations may proscribe the use of specific spells.</p>\n\n");
            } else if name == "mfn" {
                let value = bracket(rest).unwrap_or_default();
                let Some((mark, note)) = value.split_once('~') else {
                    errors.push("malformed ::mfn footnote marker".into());
                    continue;
                };
                let slug = slug_segment(mark);
                if slug.is_empty() || note.is_empty() {
                    errors.push("malformed ::mfn footnote marker".into());
                    continue;
                }
                heading_anchor_slugs.push(format!("footnote-{slug}"));
                md.push_str(&format!(
                    "\n\n<h6>{}</h6>\n\n{}\n\n",
                    escape_html(mark),
                    convert_encoded_links(note)
                ));
            } else if name == "div" {
                match attributes(trimmed) {
                    Ok(attrs) if attrs.keys().all(|key| key == "className") => {
                        // className is presentation-only; contents are preserved below.
                    }
                    Ok(_) => errors.push("unsupported attribute on :::div".into()),
                    Err(error) => errors.push(error),
                }
            } else if generic.contains(&name) {
                match render_semantic_directive(name, rest) {
                    Ok(rendered) => {
                        md.push_str("\n\n");
                        md.push_str(&rendered);
                        md.push_str("\n\n");
                    }
                    Err(error) => errors.push(error),
                }
            } else {
                errors.push(format!("unsupported directive ::{name}"));
            }
        } else {
            md.push_str(&convert_encoded_links(line));
            md.push('\n');
        }
    }
    // Inline formatting conventions represented with semantic tags before the Markdown pass.
    let md = match replace_inline(&md) {
        Ok(md) => md,
        Err(mut inline_errors) => {
            errors.append(&mut inline_errors);
            return Err(errors);
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut html_out = String::new();
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    html::push_html(&mut html_out, Parser::new_ext(&md, options));
    let mut blocks = crate::html::to_blocks(&html_out, env);
    let mut next_anchor = 0;
    for block in &mut blocks {
        if let Block::Heading {
            anchor, content, ..
        } = block
            && next_anchor < heading_anchor_slugs.len()
        {
            let text = inline_text(content);
            let wanted = heading_anchor_slugs[next_anchor]
                .strip_prefix("footnote-")
                .unwrap_or(&heading_anchor_slugs[next_anchor]);
            if slug_segment(&text) == wanted {
                *anchor = Some(heading_anchor_slugs[next_anchor].clone());
                next_anchor += 1;
            }
        }
    }
    Ok(blocks)
}

fn inline_text(content: &[cast_iron_schema::Inline]) -> String {
    use cast_iron_schema::Inline;
    fn append(inline: &Inline, out: &mut String) {
        match inline {
            Inline::Text { v } => out.push_str(v),
            Inline::Em { c }
            | Inline::Strong { c }
            | Inline::Sup { c }
            | Inline::Sub { c }
            | Inline::Link { c, .. } => {
                for child in c {
                    append(child, out);
                }
            }
            Inline::Break => out.push(' '),
        }
    }
    let mut out = String::new();
    for node in content {
        append(node, &mut out);
    }
    out
}

/// Render the text-bearing Pathfinder directives as ordinary Markdown prose. Their
/// style flags are intentionally omitted; every content attribute is emitted.
fn render_semantic_directive(name: &str, rest: &str) -> Result<String, String> {
    // `::rtraits` and `::aff` have their own parsers because their attributes hold
    // repeated values and macro tokens that the generic attribute map cannot express.
    if name == "rtraits" {
        return crate::pf_directives::racial_traits_markdown(rest);
    }
    if name == "aff" {
        return Ok(crate::pf_directives::affliction(rest, true)?.markdown);
    }
    let title = bracket(rest).unwrap_or_default();
    let attrs = if name == "mfn" {
        BTreeMap::new()
    } else {
        attributes(&format!("::{name}{rest}"))?
    };
    let allowed: &[&str] = match name {
        "ab" => &[
            "title",
            "action",
            "sub",
            "hl",
            "xFunctions_As",
            "yAt_8th_Level",
            "imp1",
            "imp2",
            "imp3",
            "imp4",
            "imp5",
            "imp6",
            "imp7",
            "imp8",
            "imp9",
            "imp10",
            "imp11",
            "imp12",
            "imp13",
            "imp14",
            "imp15",
            "imp16",
            "imp17",
            "imp18",
            "imp19",
            "imp20",
            "l",
            "icon",
            "head",
            "next",
            "flavor",
            "info",
            "ability",
            "ability2",
            "special",
            "replace",
            "alter",
            "prereq",
            "choice",
            "usage",
            "standard",
            "immediate",
            "passive",
            "fullround",
            "swift",
            "move",
            "free",
            "useInc",
            "useF",
            "useL",
            "useUnit",
            "useNC",
            "incrementPlain",
            "incrementEnd",
            "incrementDesc",
            "imp4",
            "imp8",
            "imp11",
            "imp12",
            "imp14",
            "imp16",
            "imp18",
            "imp20",
            "repeatAt",
            "order",
            "s1",
            "s2",
            "s3",
            "s4",
            "s5",
            "s6",
            "s7",
            "s8",
            "s9",
        ],
        "trait" => &[
            "title", "type", "flavor", "benefit", "req", "region", "religion", "subs", "surs",
            "campaign",
        ],
        "cskill" => &["gain", "lose"],
        "mh" => &["cr", "mr", "clear"],
        "minfo" => &[
            "source",
            "xp",
            "al",
            "lg",
            "ln",
            "le",
            "ng",
            "n",
            "ne",
            "cg",
            "cn",
            "ce",
            "fine",
            "diminutive",
            "tiny",
            "small",
            "medium",
            "large",
            "huge",
            "gargantuan",
            "colossal",
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
            "subtypes",
            "subs",
            "othersubs",
            "augment",
            "init",
            "sen",
            "senSpell",
            "dv",
            "llv",
            "keenscent",
            "scent",
            "thoughtsense",
            "greensight",
            "lifesense",
            "blindsight",
            "blindsense",
            "tremorsense",
            "mistsight",
            "xray",
            "aav",
            "sid",
            "pcp",
            "aura",
        ],
        "mdefense" => &[
            "ac", "mod", "hpRaw", "hp", "fh", "regen", "fort", "ref", "will", "def", "chanRes",
            "fortif", "split", "ink", "pBlood", "trapS", "unstop", "blockAt", "rockCt", "secSave",
            "ferocity", "amorph", "aav", "incorp", "noB", "eva", "unc", "impEva", "impUnc", "dr",
            "immune", "resist", "sr", "vulner", "weak",
        ],
        "moffense" => &[
            "sp",
            "spP",
            "cl",
            "fl",
            "sw",
            "burrow",
            "fly",
            "average",
            "good",
            "perfect",
            "melee",
            "ranged",
            "space",
            "reach",
            "reachP",
            "br",
            "brWeap",
            "constrict",
            "fSwallow",
            "swallow",
            "push",
            "pull",
            "pounce",
            "trip",
            "grab",
            "bleed",
            "burn",
            "chNEn",
            "chPEn",
            "mPower",
            "specAtt",
        ],
        "mstats" => &[
            "str", "dex", "con", "int", "wis", "cha", "bab", "cmb", "cmd", "feats", "skills",
            "racial", "lang", "sq",
        ],
        "mspell" => &[
            "sla", "psy", "prep", "know", "cl", "con", "pe", "title", "data", "constant", "atWill",
            "day", "psyMag", "l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "clear",
        ],
        "mfn" => &[],
        "meco" => &["env", "org", "treasure", "clear"],
        "fh" | "ph" => &["sub", "desc", "cat", "clear"],
        "th" => &[
            "cr",
            "source",
            "acquired",
            "inherited",
            "simple",
            "summonable",
            "maybesummon",
            "clear",
        ],
        _ => {
            return Err(format!(
                "unsupported semantic renderer for directive ::{name}"
            ));
        }
    };
    for key in attrs.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown attribute {key} on ::{name}"));
        }
    }
    if name == "cskill" {
        let mut out = Vec::new();
        for key in ["gain", "lose"] {
            if let Some(value) = attrs.get(key) {
                out.push(format!(
                    "**{} class skills:** {}",
                    if key == "gain" { "Gained" } else { "Lost" },
                    convert_encoded_links(value)
                ));
            }
        }
        return Ok(out.join("\n\n"));
    }
    if name == "mfn" {
        let (mark, note) = title.split_once('~').unwrap_or(("", title.as_str()));
        return Ok(format!(
            "**{}:** {}",
            convert_encoded_links(mark),
            convert_encoded_links(note)
        ));
    }
    if name == "trait" {
        if attrs.values().any(|value| value.contains(['~', '|'])) {
            return Err("unsupported encoded or footnote metadata in ::trait; source metadata structure cannot be reconstructed".into());
        }
        let mut out = String::new();
        for key in [
            "type", "req", "region", "religion", "campaign", "subs", "surs", "flavor", "benefit",
        ] {
            if let Some(value) = attrs.get(key) {
                if value.is_empty() {
                    continue;
                }
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                let label = match key {
                    "req" => "Prerequisite",
                    "subs" => "Subtypes",
                    "surs" => "Sources",
                    value => value,
                };
                out.push_str(&format!(
                    "**{}:** {}",
                    field_label(label),
                    convert_encoded_links(value)
                ));
            }
        }
        return Ok(out);
    }
    if name == "mstats" {
        for key in ["feats", "skills", "lang"] {
            if attrs
                .get(key)
                .is_some_and(|value| value.contains(['~', '|']))
            {
                return Err(format!("unsupported encoded {key} values on ::mstats"));
            }
        }
    }
    if name == "meco" && attrs.contains_key("treasure") {
        return Err("unsupported coded treasure value on ::meco".into());
    }
    if name == "ab"
        && attrs.keys().any(|key| {
            [
                "head",
                "next",
                "order",
                "useInc",
                "useF",
                "useL",
                "useUnit",
                "useNC",
                "incrementPlain",
                "incrementEnd",
                "incrementDesc",
            ]
            .contains(&key.as_str())
        })
    {
        return Err("unsupported encoded ordering or usage metadata on ::ab".into());
    }
    if name == "mspell" {
        return Ok(render_mspell(&attrs));
    }
    if name == "minfo" {
        return render_minfo(&title, &attrs);
    }
    if name == "mdefense"
        && let Some(hp) = attrs.get("hp")
        && !hp.contains('~')
    {
        return Err("unsupported hit point encoding on ::mdefense".into());
    }
    for (key, value) in &attrs {
        if value.contains(['~', '|']) && !(name == "mdefense" && key == "hp") {
            return Err(format!("unsupported encoded {key} value on ::{name}"));
        }
    }
    let title = if title.is_empty() {
        attrs.get("title").cloned().unwrap_or_default()
    } else {
        title
    };
    let mut out = format!("**{}**", convert_encoded_links(&title));
    for (key, value) in attrs {
        if ["icon", "head", "next", "order", "hl", "clear"].contains(&key.as_str()) {
            continue;
        }
        if name == "mdefense" && key == "hp" {
            let (amount, expression) = value
                .split_once('~')
                .ok_or_else(|| "unsupported hit point encoding on ::mdefense".to_string())?;
            out.push_str(&format!("\n\n**Hit points:** {amount} ({expression})"));
            continue;
        }
        let value = if value.is_empty() {
            "present"
        } else {
            value.as_str()
        };
        out.push_str(&format!(
            "\n\n**{}:** {}",
            field_label(&key),
            convert_encoded_links(value)
        ));
    }
    Ok(out)
}

fn render_minfo(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    if attrs.values().any(|value| value.contains(['~', '|'])) {
        return Err("unsupported encoded value on ::minfo".into());
    }
    for key in ["subs", "othersubs", "augment"] {
        if attrs
            .get(key)
            .is_some_and(|value| value.contains(['~', '|']))
        {
            return Err(format!(
                "unsupported encoded subtype value on ::minfo ({key})"
            ));
        }
    }
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("**{}**", convert_encoded_links(title)));
    }
    let alignments = [
        ("lg", "Lawful good"),
        ("ln", "Lawful neutral"),
        ("le", "Lawful evil"),
        ("ng", "Neutral good"),
        ("n", "Neutral"),
        ("ne", "Neutral evil"),
        ("cg", "Chaotic good"),
        ("cn", "Chaotic neutral"),
        ("ce", "Chaotic evil"),
    ];
    let sizes = [
        "fine",
        "diminutive",
        "tiny",
        "small",
        "medium",
        "large",
        "huge",
        "gargantuan",
        "colossal",
    ];
    let types = [
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
    ];
    for (key, value) in attrs {
        if value.is_empty() {
            if let Some((_, alignment)) = alignments.iter().find(|(code, _)| *code == key) {
                push_labeled(&mut out, "Alignment", alignment);
            } else if sizes.contains(&key.as_str()) {
                push_labeled(&mut out, "Size", key);
            } else if types.contains(&key.as_str()) {
                let label = match key.as_str() {
                    "magicalBeast" => "Magical beast",
                    "monstrousHumanoid" => "Monstrous humanoid",
                    _ => key,
                };
                push_labeled(&mut out, "Type", label);
            } else if [
                "llv",
                "keenscent",
                "scent",
                "mistsight",
                "xray",
                "aav",
                "sid",
            ]
            .contains(&key.as_str())
            {
                push_labeled(&mut out, "Sense", &field_label(key));
            } else {
                push_labeled(&mut out, &field_label(key), "present");
            }
            continue;
        }
        let label = match key.as_str() {
            "xp" => "Experience points",
            "al" => "Alignment",
            "init" => "Initiative",
            "pcp" => "Perception",
            "subs" => "Subtype",
            "sen" | "senSpell" => "Senses",
            other => other,
        };
        let display = if key == "subs" {
            value
                .split('~')
                .map(|part| {
                    part.chars()
                        .next()
                        .map(|c| c.to_uppercase().to_string() + &part[c.len_utf8()..])
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            convert_encoded_links(value)
        };
        push_labeled(&mut out, label, &display);
    }
    Ok(out)
}

fn push_labeled(out: &mut String, label: &str, value: &str) {
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(&format!("**{label}:** {value}"));
}

fn render_mspell(attrs: &BTreeMap<String, String>) -> String {
    let heading = attrs
        .get("title")
        .cloned()
        .or_else(|| {
            ["sla", "psy", "prep", "know"].iter().find_map(|kind| {
                attrs.contains_key(*kind).then(|| match *kind {
                    "sla" => "Spell-Like Abilities".to_string(),
                    "psy" => "Psychic Magic".to_string(),
                    "prep" => "Prepared Spells".to_string(),
                    _ => "Spells Known".to_string(),
                })
            })
        })
        .unwrap_or_else(|| "Spellcasting".to_string());
    let mut out = format!("**{}**", convert_encoded_links(&heading));
    for key in ["cl", "con", "pe", "data"] {
        if let Some(value) = attrs.get(key) {
            out.push_str(&format!(
                "\n\n**{}:** {}",
                field_label(key),
                convert_encoded_links(value)
            ));
        }
    }
    for key in [
        "constant", "atWill", "day", "psyMag", "l9", "l8", "l7", "l6", "l5", "l4", "l3", "l2",
        "l1", "l0",
    ] {
        let Some(value) = attrs.get(key) else {
            continue;
        };
        let label = match key {
            "constant" => "Constant",
            "atWill" => "At will",
            "day" => "Per day",
            "psyMag" => "Psychoactive powers",
            _ => key,
        };
        out.push_str(&format!("\n\n**{label}:**"));
        for group in value.split("~~").filter(|group| !group.is_empty()) {
            let mut parts = group.split('~').filter(|part| !part.is_empty());
            let Some(first) = parts.next() else { continue };
            let rest = parts.map(spell_entry).collect::<Vec<_>>().join(", ");
            if key == "day" || key.starts_with('l') && first.chars().all(|c| c.is_ascii_digit()) {
                out.push_str(&format!("\n- {first}/day: {rest}"));
            } else if key.starts_with('l') && first == "at will" {
                out.push_str(&format!("\n- At will: {rest}"));
            } else if rest.is_empty() {
                out.push_str(&format!("\n- {}", spell_entry(first)));
            } else {
                out.push_str(&format!("\n- {}, {}", spell_entry(first), rest));
            }
        }
    }
    out
}

fn spell_entry(value: &str) -> String {
    let value = value
        .replace("#D", " (domain spell)")
        .replace("#M", " (mythic spell)")
        .replace("#B", " (bonus spell)")
        .replace('|', ", ");
    convert_encoded_links(&value)
}

fn field_label(key: &str) -> String {
    match key {
        "cr" => return "Challenge rating".into(),
        "mr" => return "Mythic rank".into(),
        "xp" => return "Experience points".into(),
        "hp" => return "Hit points".into(),
        "ac" => return "Armor class".into(),
        "bab" => return "Base attack bonus".into(),
        "cmb" => return "Combat maneuver bonus".into(),
        "cmd" => return "Combat maneuver defense".into(),
        "eva" => return "Evasion".into(),
        "impEva" => return "Improved evasion".into(),
        "incorp" => return "Incorporeal".into(),
        "unc" => return "Uncanny dodge".into(),
        "impUnc" => return "Improved uncanny dodge".into(),
        _ => {}
    }
    let mut out = String::new();
    for (i, c) in key.chars().enumerate() {
        if i > 0 && c.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(if i == 0 { c.to_ascii_uppercase() } else { c });
    }
    out
}
fn split_directive(s: &str) -> (&str, &str) {
    let end = s[2..]
        .find(|c: char| c == '[' || c == '{' || c.is_whitespace())
        .map(|i| i + 2)
        .unwrap_or(s.len());
    (&s[2..end], &s[end..])
}
pub(crate) fn bracket(s: &str) -> Option<String> {
    let a = s.find('[')? + 1;
    let b = s[a..].find(']')? + a;
    Some(s[a..b].to_string())
}
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn convert_encoded_links(s: &str) -> String {
    let mut out = String::new();
    let mut rem = s;
    while let Some(start) = rem.find('‹') {
        out.push_str(&rem[..start]);
        let open_len = '‹'.len_utf8();
        let Some(end) = rem[start + open_len..].find('›') else {
            out.push_str(&rem[start..]);
            return out;
        };
        let inner_start = start + open_len;
        let inner_end = inner_start + end;
        let inner = &rem[inner_start..inner_end];
        if let Some(source_text) = inner.strip_prefix("SOURCE ") {
            let parts = source_text
                .split(';')
                .filter_map(|part| {
                    let (source, pages) = part
                        .trim()
                        .split_once('/')
                        .map_or((part.trim(), None), |(s, p)| (s.trim(), Some(p.trim())));
                    if source.is_empty() {
                        return None;
                    }
                    let (href, _) = link_parts(&format!("source/{source}"));
                    let page_label = pages
                        .map(|p| {
                            if p.contains('-') || p.contains(',') {
                                format!(" pg. {p}")
                            } else {
                                format!(" pg {p}")
                            }
                        })
                        .unwrap_or_default();
                    Some(format!("[{source}{page_label}]({href})"))
                })
                .collect::<Vec<_>>();
            out.push_str("**Sources:** ");
            out.push_str(&parts.join(", "));
        } else {
            let (href, display) = link_parts(inner);
            if href.contains('/') {
                out.push_str(&format!("[{}]({})", display, href));
            } else {
                out.push_str(&rem[start..inner_end + '›'.len_utf8()]);
            }
        }
        let close_start = inner_end;
        let close_end = close_start + '›'.len_utf8();
        rem = &rem[close_end..];
    }
    out.push_str(rem);
    out
}
fn replace_inline(s: &str) -> Result<String, Vec<String>> {
    let mut out = s.to_string();
    for (tag, open, close) in [
        ("@HL[", "<strong>", "</strong>"),
        ("@hl[", "<strong>", "</strong>"),
        ("@strong[", "<strong>", "</strong>"),
        ("@b[", "<strong>", "</strong>"),
        ("@em[", "<em>", "</em>"),
        ("@i[", "<em>", "</em>"),
        ("@sup[", "<sup>", "</sup>"),
        ("@sub[", "<sub>", "</sub>"),
    ] {
        while let Some(a) = out.find(tag) {
            let start = a + tag.len();
            let Some(rel) = out[start..].find(']') else {
                break;
            };
            let end = start + rel;
            let val = out[start..end].to_string();
            out.replace_range(a..end + 1, &format!("{open}{val}{close}"));
        }
    }
    let mut errors = Vec::new();
    let bytes = out.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if i > start + 1 {
                let token = &out[start..i];
                let is_supported =
                    ["@HL", "@hl", "@strong", "@b", "@em", "@i", "@sup", "@sub"].contains(&token);
                let has_body = out[i..].starts_with('[') && out[i + 1..].contains(']');
                if !is_supported || !has_body {
                    errors.push(format!("unsupported or malformed inline directive {token}"));
                }
            }
        } else {
            i += 1;
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::NullEnv;
    #[test]
    fn extras_follow_upstream_rules() {
        let examples = [
            (
                "protocol/Gorgon's Bar/Grill!",
                "protocol/gorgons_bar_grill",
                "Gorgon's Bar/Grill!",
            ),
            (
                "protocol/Pizza«-riffic› Patrol",
                "protocol/pizza_patrol",
                "Pizza-riffic Patrol",
            ),
            (
                "protocol/un»loved« foods",
                "protocol/loved",
                "unloved foods",
            ),
            (
                "protocol/starting<_a> business",
                "protocol/starting_a_business",
                "starting business",
            ),
            (
                "protocol/a1_>company time<_policy",
                "protocol/a1_company_time_policy",
                "company time",
            ),
            ("protocol/a b c< D E> f", "protocol/a_b_c D E_f", "a b c f"),
            (
                "protocol/break«ing» up is<_not> hard",
                "protocol/break_up_is_not_hard",
                "breaking up is hard",
            ),
        ];
        for (input, href, label) in examples {
            let (actual_href, actual_label) = link_parts(&format!("‹{input}›"));
            assert_eq!(actual_href, href, "{input}");
            assert_eq!(actual_label, label, "{input}");
        }
        assert_eq!(
            link_parts("‹proto/pre>core» center «tail<_suffix›"),
            ("proto/pre_center_suffix".into(), "core center tail".into())
        );
    }
    #[test]
    fn parses_flags_and_quotes() {
        assert_eq!(
            attributes("minor dc=15 type=\"contact poison\"")
                .unwrap()
                .get("type")
                .unwrap(),
            "contact poison"
        );
        assert!(attributes("::spell{broken=\"x}").is_err());
        assert!(attributes("::spell broken=1").is_err());
    }
    #[test]
    fn spell_body_survives_and_unknown_fails() {
        let mut env = NullEnv;
        let error = render(
            &[
                "::spell{id=1 source=\"Core/1\" transmutation arc=3}".into(),
                "The body remains.".into(),
            ],
            &mut env,
        )
        .unwrap_err();
        assert!(
            error
                .iter()
                .any(|e| e.contains("unsupported semantic renderer"))
        );
        assert!(render(&["::mystery[x]".into()], &mut env).is_err());
        assert!(
            render(&["text @unexpected[value]".into()], &mut env)
                .unwrap_err()
                .iter()
                .any(|e| e.contains("@unexpected"))
        );
        assert!(render(&["text @HL[unfinished".into()], &mut env).is_err());
    }
    #[test]
    fn markdown_table_is_typed() {
        let mut env = NullEnv;
        let blocks = render(
            &["| A | B |".into(), "|---|---|".into(), "| x | y |".into()],
            &mut env,
        )
        .unwrap();
        assert!(blocks.iter().any(|b| matches!(b, Block::Table(_))));
    }

    #[test]
    fn source_keeps_page_refs_and_unicode_is_safe() {
        let source = convert_encoded_links("‹SOURCE This Book/12;A Fine Source #14/10-22›");
        assert!(source.contains("This Book pg 12"));
        assert!(source.contains("A Fine Source #14 pg. 10-22"));
        assert!(source.contains("source/a_fine_source_14"));
        let linked = convert_encoded_links("‹protocol/café›");
        assert!(linked.contains("[café](protocol/caf)"));
        let mut env = NullEnv;
        assert!(render(&["‹SOURCE Édition/4›".into()], &mut env).is_ok());
    }

    #[test]
    fn multiple_encoded_links_do_not_consume_intervening_prose() {
        let actual = convert_encoded_links(
            "A ‹feat/Iron Will› and ‹spell/dominate person› remain separate.",
        );
        assert_eq!(
            actual,
            "A [Iron Will](feat/iron_will) and [dominate person](spell/dominate_person) remain separate."
        );
    }

    #[test]
    fn archetype_directive_preserves_prose_and_links() {
        let mut env = NullEnv;
        let blocks = render(&["::ab[Spirit Surge (Ex)]{l=3 icon=magic ability=\"Cast ‹spell/etherealness› and ‹spell/astral projection›.\" replace=Hexes}".into()], &mut env).unwrap();
        let html = format!("{blocks:?}");
        assert!(html.contains("Spirit Surge"));
        assert!(html.contains("Cast"));
        assert!(html.contains("etherealness"));
        assert!(html.contains("astral projection"));
        assert!(html.contains("Hexes"));
    }

    #[test]
    fn creature_statblock_directives_keep_mechanics_and_body() {
        let mut env = NullEnv;
        let lines = [
            "::mh[Sample Creature]{cr=5}".into(),
            "::minfo{source=\"‹SOURCE Bestiary/12›\" xp=1600 al=ng medium dragon init=+4 dv=60 aura=\"fear\"}".into(),
            "::mdefense{ac=20/12/18 hp=\"45~6d10+12\" fort=+8 ref=+5 will=+6 dr=\"5/cold iron\" immune=\"sleep, paralysis\"}".into(),
            "::moffense{sp=40 melee=\"bite +8 (1d8+4)\" reachP=\"10 ft. with bite\" specAtt=\"breath weapon\"}".into(),
            "::mstats{str=18 dex=12 con=14 int=10 wis=13 cha=11 bab=+6 cmb=+10 cmd=21 feats=\"Power Attack\" skills=\"Perception +9\" lang=Common sq=\"keen senses\"}".into(),
            "::meco{env=mountains org=solitary}".into(),
            "Its lair is described here.".into(),
        ];
        let blocks = render(&lines, &mut env).unwrap();
        let html = format!("{blocks:?}");
        for value in [
            "45",
            "6d10",
            "bite +8",
            "breath weapon",
            "Power Attack",
            "mountains",
            "Its lair",
        ] {
            assert!(html.contains(value), "missing {value}: {html}");
        }
    }

    #[test]
    fn unknown_semantic_attribute_is_rejected() {
        let mut env = NullEnv;
        let error = render(&["::mstats{str=12 mysterystat=4}".into()], &mut env).unwrap_err();
        assert!(
            error
                .iter()
                .any(|e| e.contains("unknown attribute mysterystat"))
        );
    }

    #[test]
    fn div_container_keeps_children_and_ignores_documented_style() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                ":::div{className=reduce}".into(),
                "Contained prose.".into(),
                ":::".into(),
            ],
            &mut env,
        )
        .unwrap();
        assert!(format!("{blocks:?}").contains("Contained prose"));
    }

    #[test]
    fn spell_directives_preserve_frequency_dc_and_footnotes() {
        let mut env = NullEnv;
        let lines = [
            "::mspell{sla cl=9 con=+12 constant=\"true seeing\" atWill=\"mage hand\" day=\"3~‹spell/hold person›|DC 16~~1~‹spell/time stop›\"}".into(),
            "::mfn[B~Bonus feat.]".into(),
        ];
        let blocks = render(&lines, &mut env).unwrap();
        let rendered = format!("{blocks:?}");
        for value in [
            "Spell-Like Abilities",
            "9",
            "+12",
            "3/day",
            "DC 16",
            "1/day",
            "time stop",
            "Bonus feat",
        ] {
            assert!(rendered.contains(value), "missing {value}: {rendered}");
        }
        assert!(!rendered.contains('~') && !rendered.contains('|'));
    }

    #[test]
    fn triple_colon_archetype_attribute_container_and_generated_macros() {
        let mut env = NullEnv;
        let blocks = render(
            &[":::ab{title=Wrangle action=Ability l=2 imp5=\"Ends the condition.\"}".into()],
            &mut env,
        )
        .unwrap();
        let rendered = format!("{blocks:?}");
        assert!(rendered.contains("Wrangle"));
        assert!(rendered.contains("Ends the condition"));
        assert!(render(&["Text @B-something[value]".into()], &mut env).is_err());
    }

    #[test]
    fn trait_content_omits_placeholder_title_and_keeps_metadata_and_links() {
        let mut env = NullEnv;
        let blocks = render(
            &["::trait{title=\"&L&Air-Touched&FN&\" type=Magic flavor=\"A magical affinity.\" benefit=\"You gain ‹feat/Iron Will›.\" req=\"Any alignment\"}".into()],
            &mut env,
        ).unwrap();
        let rendered = format!("{blocks:?}");
        assert!(!rendered.contains("&L&"));
        assert!(!rendered.contains("&FN&"));
        for value in ["Magic", "A magical affinity", "Iron Will", "Prerequisite"] {
            assert!(rendered.contains(value), "missing {value}: {rendered}");
        }
        assert!(
            render(
                &["::trait{benefit=\"Effect.~~~Footnote.\"}".into()],
                &mut env
            )
            .unwrap_err()
            .iter()
            .any(|error| error.contains("footnote metadata"))
        );
    }

    #[test]
    fn source_statblock_heading_ids_map_to_and_set_canonical_anchors() {
        let lines = [
            "".into(),
            "Éclat".into(),
            "::sh[Defense]{id=D}".into(),
            "::sh[Offense]{id=O}".into(),
            "::sh[Statistics]{id=S}".into(),
        ];
        assert_eq!(
            anchors(&lines).get("D").map(String::as_str),
            Some("defense")
        );
        assert_eq!(
            anchors(&lines).get("O").map(String::as_str),
            Some("offense")
        );
        let mut env = NullEnv;
        let blocks = render(&lines, &mut env).unwrap();
        let headings: Vec<_> = blocks
            .iter()
            .filter_map(|block| match block {
                Block::Heading { anchor, .. } => anchor.as_deref(),
                _ => None,
            })
            .collect();
        assert_eq!(headings, ["defense", "offense", "statistics"]);
        assert!(render(&["::sh[Defense]{unknown=x}".into()], &mut env).is_err());
    }

    #[test]
    fn aasimar_statblock_fields_are_decoded_or_explicitly_unsupported() {
        let mut env = NullEnv;
        let cr = render(&["::mh[Aasimar]{cr=\"1/2\"}".into()], &mut env).unwrap();
        assert!(format!("{cr:?}").contains("Challenge rating"));
        let minfo = render(&["::minfo[Aasimar ‹class/cleric› 1]{source=\"Pathfinder RPG Bestiary/7\" xp=200 ng medium outsider subs=native init=+0 dv=60 pcp=+5}".into()], &mut env).unwrap();
        let minfo = format!("{minfo:?}");
        for value in [
            "Experience points",
            "200",
            "Neutral good",
            "medium",
            "outsider",
            "Native",
        ] {
            assert!(minfo.contains(value), "missing {value}: {minfo}");
        }
        let defense = render(&["::mdefense{ac=\"15/10/15\" mod=\"+5 armor\" hp=\"11~1d8+3\" fort=+4 ref=+0 will=+5 resist=\"acid 5, cold 5, electricity 5\"}".into()], &mut env).unwrap();
        let defense = format!("{defense:?}");
        assert!(defense.contains("11 (1d8+3)"));
        assert!(!defense.contains("11~1d8"));
        let offense = render(&["::moffense{sp=30 spP=\"20 ft. in armor\" melee=\"heavy mace -1 (1d8-1)\" chPEn=\"5/day, 1d6, DC 12\"}".into()], &mut env).unwrap();
        assert!(format!("{offense:?}").contains("DC 12"));
        let skills = render(
            &["::mstats{skills=\"diplo|8~heal|7~k|r|5\" lang=Ce~C~D}".into()],
            &mut env,
        )
        .unwrap_err();
        assert!(
            skills
                .iter()
                .any(|error| error.contains("unsupported encoded"))
        );
        let treasure = render(
            &["::meco{env=\"any land\" treasure=\"N=Asm~Whm~Wlc10\"}".into()],
            &mut env,
        )
        .unwrap_err();
        assert!(
            treasure
                .iter()
                .any(|error| error.contains("coded treasure"))
        );
        let flags = render(&["::minfo{lg large humanoid}".into()], &mut env).unwrap();
        let flags = format!("{flags:?}");
        assert!(
            flags.contains("Lawful good") && flags.contains("large") && flags.contains("humanoid")
        );
        assert!(
            render(
                &[":::ab{title=Feature useInc=shaman~4~10}".into()],
                &mut env
            )
            .unwrap_err()
            .iter()
            .any(|error| error.contains("usage metadata"))
        );
    }

    #[test]
    fn footnote_markers_provide_canonical_local_heading_anchors() {
        let lines = [
            "".into(),
            "Öther text".into(),
            "::mfn[D~Domain spell]".into(),
        ];
        assert_eq!(
            anchors(&lines).get("D").map(String::as_str),
            Some("footnote-d")
        );
        let mut env = NullEnv;
        let blocks = render(&lines, &mut env).unwrap();
        assert!(blocks.iter().any(|block| matches!(block, Block::Heading { anchor: Some(anchor), .. } if anchor == "footnote-d")));
    }
}
