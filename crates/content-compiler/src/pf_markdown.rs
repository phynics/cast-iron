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
        if let Some(existing) = out.get(&key) {
            if existing == &value {
                // The upstream data repeats some flags verbatim. A repeated value
                // adds no second meaning, so keep the first and move on.
                continue;
            }
            if key == "type" {
                // A duplicated `type` is an upstream data defect. The first value
                // names the supported category; later repeats are stray copies.
                continue;
            }
            if repeatable_metadata(&key) {
                // Presentation and ordering keys are sometimes written twice with
                // different values (an ability ordered into two jump-list groups).
                // The last value is the one the upstream renderer uses.
                out.insert(key, value);
                continue;
            }
            return Err(format!("duplicate attribute {key}"));
        }
        out.insert(key, value);
    }
    Ok(out)
}

/// Keys whose repeated occurrence is upstream presentation/ordering metadata
/// rather than two competing content values. Repeats are safe to collapse.
fn repeatable_metadata(key: &str) -> bool {
    matches!(
        key,
        "icon"
            | "order"
            | "clear"
            | "jl"
            | "id"
            | "head"
            | "next"
            | "sub"
            | "indent"
            | "useMod"
            | "useF"
            | "useL"
            | "useM"
            | "useHD"
            | "useNC"
            | "useUnit"
    ) || numbered_key(key, 'l')
        || numbered_key(key, 's')
        || key
            .strip_prefix("imp")
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

fn numbered_key(key: &str, prefix: char) -> bool {
    key.strip_prefix(prefix)
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// Parse a directive's attributes when it has braces; a directive without
/// braces (such as `:::stamina`) has no attributes.
fn optional_attributes(input: &str) -> Result<BTreeMap<String, String>, String> {
    if input.contains('{') {
        attributes(input)
    } else {
        Ok(BTreeMap::new())
    }
}

/// Map source statblock IDs to the stable heading slugs used by typed content.
pub fn anchors(lines: &[String]) -> BTreeMap<String, String> {
    lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Standard Markdown footnote definitions (`[^D]: Domain spell`) are
            // rendered by the HTML pass as an anchored heading. Map the source
            // marker to that heading so `[^D]` links resolve locally.
            if let Some(rest) = trimmed.strip_prefix("[^")
                && let Some((mark, _)) = rest.split_once("]:")
            {
                let slug = slug_segment(mark);
                return (!slug.is_empty()).then(|| (mark.to_string(), format!("footnote-{slug}")));
            }
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
        // A block directive can be quoted (`>::tabular{...}`) to sit inside a note.
        let trimmed = {
            let unquoted = trimmed.trim_start_matches('>').trim_start();
            if unquoted.starts_with("::") {
                unquoted
            } else {
                trimmed
            }
        };
        if trimmed.starts_with("::") {
            if trimmed == ":::" {
                continue;
            }
            let (raw_name, rest) = split_directive(trimmed);
            let name = raw_name.trim_start_matches(':');
            let allowed_headers = ["mainheader", "h2", "h3", "h4", "h5", "h6", "gh", "sh"];
            let linked_headers = ["hl2", "hl3", "hl4", "hl5", "hl6"];
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
                "list",
                "spellAb",
                "cantrips",
            ];
            if allowed_headers.contains(&name) {
                let title = bracket(rest).unwrap_or_default();
                if rest.contains('{') {
                    match attributes(trimmed) {
                        Ok(attrs) => {
                            for key in attrs.keys() {
                                match key.as_str() {
                                    // `jl` is a jump-list group name, not body content.
                                    "jl" => env.stripped("heading_jump_list"),
                                    "id" | "clear" => {}
                                    _ => {
                                        errors.push(format!("unknown attribute {key} on ::{name}"))
                                    }
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
            } else if name == "block" {
                match optional_attributes(trimmed) {
                    Ok(attrs) => {
                        for key in attrs.keys() {
                            match key.as_str() {
                                // Size, highlight, clear, and class only change the
                                // upstream layout. `title` adds a real heading below.
                                "size" | "hl" | "clear" | "classes" | "className" | "title"
                                | "minimal" => {}
                                _ => errors.push(format!("unknown attribute {key} on :::block")),
                            }
                        }
                        if let Some(title) = attrs.get("title").filter(|t| !t.trim().is_empty()) {
                            md.push_str(&format!(
                                "\n\n<h3>{}</h3>\n\n",
                                escape_html(&decode_source_markers(&convert_encoded_links(title)))
                            ));
                        }
                        env.stripped("block_container");
                    }
                    Err(error) => errors.push(error),
                }
            } else if name == "group" {
                match optional_attributes(trimmed) {
                    Ok(attrs) if attrs.is_empty() => env.stripped("group_container"),
                    Ok(_) => errors.push("unsupported attribute on :::group".into()),
                    Err(error) => errors.push(error),
                }
            } else if name == "stamina" {
                match optional_attributes(trimmed) {
                    Ok(attrs) if attrs.is_empty() => {
                        md.push_str("\n\n<h3>Stamina</h3>\n\n");
                    }
                    Ok(_) => errors.push("unsupported attribute on :::stamina".into()),
                    Err(error) => errors.push(error),
                }
            } else if name == "mythic" {
                match attributes(trimmed) {
                    Ok(attrs) => {
                        for key in attrs.keys() {
                            match key.as_str() {
                                "title" => {}
                                "jl" => env.stripped("heading_jump_list"),
                                _ => errors.push(format!("unknown attribute {key} on :::mythic")),
                            }
                        }
                        let title = attrs.get("title").map(String::as_str).unwrap_or("Mythic");
                        md.push_str(&format!(
                            "\n\n<h3>{}</h3>\n\n",
                            escape_html(&convert_encoded_links(title))
                        ));
                    }
                    Err(error) => errors.push(error),
                }
            } else if name == "kinetics" {
                match attributes(trimmed)
                    .and_then(|attrs| render_kinetics(&bracket(rest).unwrap_or_default(), &attrs))
                {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if linked_headers.contains(&name) {
                match render_linked_heading(name, rest, env) {
                    Ok(rendered) => {
                        md.push_str("\n\n");
                        md.push_str(&rendered);
                        md.push_str("\n\n");
                    }
                    Err(error) => errors.push(error),
                }
            } else if name == "aside" || name == "fakeFootnotes" || name == "elephant" {
                // These are layout containers. `@FN` definitions inside the
                // fake-footnote container are rendered by the inline pass.
                if name == "elephant" {
                    match optional_attributes(trimmed) {
                        Ok(attrs) => {
                            for key in attrs.keys() {
                                if !["title", "jl", "clear", "id"].contains(&key.as_str()) {
                                    errors.push(format!("unknown attribute {key} on :::elephant"));
                                }
                            }
                            if let Some(title) = attrs.get("title").filter(|t| !t.trim().is_empty())
                            {
                                md.push_str(&format!(
                                    "\n\n<h3>{}</h3>\n\n",
                                    escape_html(&convert_encoded_links(title))
                                ));
                            }
                        }
                        Err(error) => errors.push(error),
                    }
                }
                env.stripped(if name == "aside" {
                    "aside_container"
                } else if name == "elephant" {
                    "elephant_container"
                } else {
                    "fake_footnotes_container"
                });
            } else if name == "class" {
                match render_class_directive(rest) {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if name == "prof" {
                match render_prof_directive(rest) {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if name == "archetype" {
                match render_archetype_directive(rest) {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if name == "archetypeDesc" || name == "altCapstoneDesc" {
                md.push_str(&format!(
                    "\n\n{}\n\n",
                    if name == "archetypeDesc" {
                        ARCHETYPE_NOTE
                    } else {
                        CAPSTONE_NOTE
                    }
                ));
            } else if name == "altCapstone" {
                match render_capstone_directive(rest) {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if name == "classoptions" {
                match render_class_options(rest, env) {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if name == "tabular" {
                match render_tabular_directive(rest) {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if name == "row" {
                let title = bracket(rest).unwrap_or_default();
                match attributes(trimmed) {
                    Ok(attrs) => {
                        let mut rendered = format!("**{}**", escape_html(&title));
                        for (key, value) in attrs {
                            if value.trim().is_empty() {
                                continue;
                            }
                            let label = if key == "info" {
                                "Details".to_string()
                            } else {
                                let label = key.replace('_', " ");
                                let mut chars = label.chars();
                                chars
                                    .next()
                                    .map(|first| {
                                        first.to_ascii_uppercase().to_string() + chars.as_str()
                                    })
                                    .unwrap_or(label)
                            };
                            rendered.push_str(&format!(
                                "\n\n**{label}:** {}",
                                convert_encoded_links(&value)
                            ));
                        }
                        md.push_str("\n\n");
                        md.push_str(&rendered);
                        md.push_str("\n\n");
                    }
                    Err(error) => errors.push(error),
                }
            } else if name == "rtraits" {
                match render_rtraits_directive(rest) {
                    Ok(rendered) => md.push_str(&format!("\n\n{rendered}\n\n")),
                    Err(error) => errors.push(error),
                }
            } else if generic.contains(&name) {
                match render_semantic_directive(name, rest, env) {
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
    // Icon directives are presentation-only; remove them but keep a coverage note.
    let md = strip_icon_directives(&md, env);
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

const ARCHETYPE_NOTE: &str = "<p><strong>Source</strong> Advanced Player's Guide pg. 72<br />From the noble paladin to the skillful rogue, each class draws upon a central archetype. Yet, beyond that basic concept exists the potential for innumerable interpretations, details, and refinements; and some archetypes prove pervasive and exciting enough to see use in play time and time again.</p>\n<p>To help players interested in creating iconic fantasy characters, <strong>archetypes</strong> were introduced. These optional features represent a unique view of what a class deliberately designed to capture a specific character archetype might become. While the types of options presented for each class differ, each subsystem is customized to best serve that class, emulate the abilities and talents of classic fantasy archetypes, and expand players' freedom to design exactly the characters they desire.</p>";

const CAPSTONE_NOTE: &str = "<p><strong>Source</strong> Chronicle of Legends pg. 28<br />When a character reaches the 20th level of a class, she gains a powerful class feature or ability, sometimes referred to as a capstone. The following section provides new capstones for characters to select at 20th level. A character can select one of the following capstones in place of the capstone provided by her class. Some capstones are for specific classes, while others are for a range of classes that qualify for them. In some cases, a capstone specifies what ability it replaces. A character can't select a new capstone if she has previously traded away her class capstone via an archetype. Clerics and wizards can receive a capstone at 20th level, despite not having one to begin with. A character that gains certain abilities may be eligible for capstones other than those listed below.</p>";

fn ordinal(n: u32) -> String {
    let suffix = if (11..=13).contains(&(n % 100)) {
        "th"
    } else {
        match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{n}{suffix}")
}

fn joined(value: &str, separator: &str) -> String {
    value
        .split('~')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(separator)
}

fn engine_join(values: &[String]) -> String {
    match values.len() {
        0 => String::new(),
        1 => values[0].clone(),
        2 => format!("{} and {}", values[0], values[1]),
        _ => format!(
            "{}, and {}",
            values[..values.len() - 1].join(", "),
            values[values.len() - 1]
        ),
    }
}

fn attr<'a>(attrs: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    attrs
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

fn render_linked_heading(
    name: &str,
    rest: &str,
    env: &mut dyn crate::html::Env,
) -> Result<String, String> {
    let title = bracket(rest).ok_or_else(|| format!("::{name} has no link text"))?;
    let attrs = if rest.contains('{') {
        attributes(&format!("::{name}{rest}"))?
    } else {
        BTreeMap::new()
    };
    for key in attrs.keys() {
        match key.as_str() {
            "pre" | "post" | "id" | "clear" => {}
            "jl" => env.stripped("linked_heading_jump_list"),
            _ => return Err(format!("unknown attribute {key} on ::{name}")),
        }
    }
    let level = name
        .strip_prefix("hl")
        .and_then(|level| level.parse().ok())
        .unwrap_or(3);
    let inner = if title.contains('/') {
        convert_encoded_links(&format!("‹{title}›"))
    } else {
        title
    };
    let pre = attr(&attrs, "pre").unwrap_or("");
    let post = attr(&attrs, "post").unwrap_or("");
    Ok(format!("<h{level}>{pre}{inner}{post}</h{level}>"))
}

const CLASS_ATTRS: &[&str] = &[
    "clear",
    "al",
    "any",
    "noAlignmentTable",
    "lg",
    "ln",
    "le",
    "ng",
    "n",
    "ne",
    "cg",
    "cn",
    "ce",
    "hd",
    "parent",
    "wealth",
    "skp",
    "acro",
    "app",
    "bluff",
    "climb",
    "craft",
    "diplo",
    "dis",
    "dd",
    "ea",
    "fly",
    "ha",
    "heal",
    "intm",
    "ka",
    "kd",
    "ke",
    "kg",
    "kh",
    "kl",
    "kna",
    "kno",
    "kp",
    "kr",
    "ling",
    "per",
    "perf",
    "prof",
    "ride",
    "sm",
    "soh",
    "spc",
    "stl",
    "sur",
    "swim",
    "umd",
    "skillNote",
];

fn render_class_directive(rest: &str) -> Result<String, String> {
    let attrs = attributes(&format!("::class{rest}"))?;
    for key in attrs.keys() {
        if !CLASS_ATTRS.contains(&key.as_str()) {
            return Err(format!("unknown attribute {key} on ::class"));
        }
    }
    let mut out = String::new();
    if let Some(alignment) = attr(&attrs, "al") {
        push_labeled(&mut out, "Alignment", &convert_encoded_links(alignment));
    } else if attrs.contains_key("any") {
        push_labeled(&mut out, "Alignment", "Any");
    }
    if let Some(hd) = attr(&attrs, "hd") {
        push_labeled(&mut out, "Hit Die", &format!("d{hd}"));
    }
    if let Some(parent) = attr(&attrs, "parent") {
        push_labeled(&mut out, "Parent Class(es)", &convert_encoded_links(parent));
    }
    if let Some(wealth) = attr(&attrs, "wealth") {
        let average = wealth
            .parse::<u32>()
            .map(|wealth| wealth * 35)
            .unwrap_or_default();
        push_labeled(
            &mut out,
            "Starting Wealth",
            &format!("{wealth}d6 × 10 gp (average {average} gp)"),
        );
    }
    if let Some(skp) = attr(&attrs, "skp") {
        push_labeled(
            &mut out,
            "Skill Points per Level",
            &format!("{skp} + Int modifier"),
        );
    }
    let skill_groups: &[(&str, &[(&str, &str)])] = &[
        ("Str", &[("climb", "Climb"), ("swim", "Swim")]),
        (
            "Dex",
            &[
                ("acro", "Acrobatics"),
                ("dd", "Disable Device"),
                ("ea", "Escape Artist"),
                ("fly", "Fly"),
                ("ride", "Ride"),
                ("soh", "Sleight of Hand"),
                ("stl", "Stealth"),
            ],
        ),
        (
            "Int",
            &[
                ("app", "Appraise"),
                ("craft", "Craft (any)"),
                ("ka", "Knowledge (arcana)"),
                ("kd", "Knowledge (dungeoneering)"),
                ("ke", "Knowledge (engineering)"),
                ("kg", "Knowledge (geography)"),
                ("kh", "Knowledge (history)"),
                ("kl", "Knowledge (local)"),
                ("kna", "Knowledge (nature)"),
                ("kno", "Knowledge (nobility)"),
                ("kp", "Knowledge (planes)"),
                ("kr", "Knowledge (religion)"),
                ("ling", "Linguistics"),
                ("spc", "Spellcraft"),
            ],
        ),
        (
            "Wis",
            &[
                ("heal", "Heal"),
                ("per", "Perception"),
                ("prof", "Profession (any)"),
                ("sm", "Sense Motive"),
                ("sur", "Survival"),
            ],
        ),
        (
            "Cha",
            &[
                ("bluff", "Bluff"),
                ("diplo", "Diplomacy"),
                ("dis", "Disguise"),
                ("ha", "Handle Animal"),
                ("intm", "Intimidate"),
                ("perf", "Perform (any)"),
                ("umd", "Use Magic Device"),
            ],
        ),
    ];
    for (stat, skills) in skill_groups {
        let names: Vec<_> = skills
            .iter()
            .filter(|(key, _)| attrs.contains_key(*key))
            .map(|(_, label)| (*label).to_string())
            .collect();
        if !names.is_empty() {
            push_labeled(
                &mut out,
                &format!("Class Skills ({stat})"),
                &names.join(", "),
            );
        }
    }
    if let Some(note) = attr(&attrs, "skillNote") {
        push_labeled(&mut out, "Skill Note", &convert_encoded_links(note));
    }
    Ok(out)
}

const PROF_ATTRS: &[&str] = &[
    "clear", "simple", "martial", "weaps", "wExtra", "light", "medium", "heavy", "shields",
    "tower", "aExtra", "extra",
];

fn render_prof_directive(rest: &str) -> Result<String, String> {
    let attrs = attributes(&format!("::prof{rest}"))?;
    for key in attrs.keys() {
        if !PROF_ATTRS.contains(&key.as_str()) {
            return Err(format!("unknown attribute {key} on ::prof"));
        }
    }
    let mut weapons = Vec::new();
    if attrs.contains_key("simple") {
        weapons.push("All simple weapons".to_string());
    }
    if attrs.contains_key("martial") {
        weapons.push("All martial weapons".to_string());
    }
    if let Some(value) = attr(&attrs, "weaps") {
        for item in value.split('~').filter(|item| !item.is_empty()) {
            weapons.push(item.strip_prefix('!').unwrap_or(item).to_string());
        }
    }
    if let Some(extra) = attr(&attrs, "wExtra") {
        weapons.push(convert_encoded_links(extra));
    }
    let armors: Vec<_> = ["light", "medium", "heavy"]
        .iter()
        .filter(|key| attrs.contains_key(**key))
        .map(|key| format!("{} armor", field_label(key)))
        .collect();
    let shields = if let Some(value) = attr(&attrs, "shields") {
        convert_encoded_links(value)
    } else if attrs.contains_key("shields") {
        if attrs.contains_key("tower") {
            "Yes, including tower shields".to_string()
        } else {
            "Yes, except tower shields".to_string()
        }
    } else {
        "No".to_string()
    };
    let mut out = String::from("**Weapon and Armor Proficiencies**");
    let weapon_text = if weapons.is_empty() {
        "None".to_string()
    } else {
        weapons.join(", ")
    };
    push_labeled(&mut out, "Weapons", &weapon_text);
    let armor_text = if armors.is_empty() {
        "No armor".to_string()
    } else {
        armors.join(", ")
    };
    push_labeled(&mut out, "Armor", &armor_text);
    push_labeled(&mut out, "Shields", &shields);
    if let Some(extra) = attr(&attrs, "aExtra") {
        push_labeled(&mut out, "Armor Notes", &convert_encoded_links(extra));
    }
    if let Some(extra) = attr(&attrs, "extra") {
        push_labeled(&mut out, "Notes", &convert_encoded_links(extra));
    }
    Ok(out)
}

fn render_archetype_directive(rest: &str) -> Result<String, String> {
    let title = bracket(rest).unwrap_or_default();
    let attrs = attributes(&format!("::archetype{rest}"))?;
    for key in attrs.keys() {
        if !["clear", "c", "r", "e"].contains(&key.as_str()) {
            return Err(format!("unknown attribute {key} on ::archetype"));
        }
    }
    let mut out = format!("**{}**", convert_encoded_links(&title));
    if let Some(class) = attr(&attrs, "c") {
        push_labeled(&mut out, "Class", class);
    }
    for (key, label) in [("r", "Modifies or Replaces"), ("e", "Description")] {
        if let Some(value) = attr(&attrs, key) {
            push_labeled(&mut out, label, &convert_encoded_links(value));
        }
    }
    Ok(out)
}

fn render_capstone_directive(rest: &str) -> Result<String, String> {
    let title = bracket(rest).ok_or("::altCapstone has no title")?;
    let attrs = attributes(&format!("::altCapstone{rest}"))?;
    let flags = [
        ("deepMagics", "Deep Magics"),
        ("walkingLibrary", "Walking Library"),
        ("greatBeast", "Great Beast"),
        ("oldDogNewTricks", "Old Dog, New Tricks"),
        ("kiSage", "Ki Sage"),
        ("theRightSpot", "The Right Spot"),
        ("archFamiliar", "Arch-Familiar"),
        ("soulChannel", "Soul Channel"),
    ];
    for key in attrs.keys() {
        if key != "clear" && key != "capstone" && !flags.iter().any(|(flag, _)| flag == key) {
            return Err(format!("unknown attribute {key} on ::altCapstone"));
        }
    }
    let mut out = format!("**{}**", convert_encoded_links(&title));
    if let Some(benefit) = attr(&attrs, "capstone") {
        out.push_str(&format!("\n\n{}", convert_encoded_links(benefit)));
    }
    let extra: Vec<_> = flags
        .iter()
        .filter(|(flag, _)| attrs.contains_key(*flag))
        .map(|(_, label)| (*label).to_string())
        .collect();
    if !extra.is_empty() {
        push_labeled(&mut out, "Also available", &extra.join(", "));
    }
    Ok(out)
}

fn render_class_options(rest: &str, env: &mut dyn crate::html::Env) -> Result<String, String> {
    let attrs = attributes(&format!("::classoptions{rest}"))?;
    for key in attrs.keys() {
        if !["clear", "cls"].contains(&key.as_str()) {
            return Err(format!("unknown attribute {key} on ::classoptions"));
        }
    }
    let class = attr(&attrs, "cls").ok_or("::classoptions has no class")?;
    // The upstream site renders this list from a separate generated component.
    // The pointer is preserved as coverage because it is not part of this record.
    env.stripped(&format!("class_options_external_list:{class}"));
    Ok(format!(
        "*The favored class options for {class} are listed on the separate class options page.*"
    ))
}

fn md_cell(value: &str) -> String {
    convert_encoded_links(value)
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
}

fn table_row(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

fn render_tabular_directive(rest: &str) -> Result<String, String> {
    let attrs = attributes(&format!("::tabular{rest}"))?;
    let format = attr(&attrs, "format").ok_or("::tabular has no format")?;
    for key in attrs.keys() {
        let valid = ["format", "id", "clear"].contains(&key.as_str())
            || (key.len() == 1 && key.as_bytes()[0].is_ascii_lowercase());
        if !valid {
            return Err(format!("unknown attribute {key} on ::tabular"));
        }
    }
    let cell = |key: &str| attr(&attrs, key).map(md_cell).unwrap_or_default();
    let (headers, row) = match format {
        "alchstats" => {
            let equipment = attr(&attrs, "d")
                .map(|name| match attr(&attrs, "c") {
                    Some(kind) => format!("{name} ({kind})"),
                    None => name.to_string(),
                })
                .or_else(|| attr(&attrs, "e").map(str::to_string))
                .unwrap_or_default();
            (
                vec!["Price".to_string(), "Weight".into(), "Equipment".into()],
                vec![cell("a"), cell("b"), md_cell(&equipment)],
            )
        }
        "powercomp" => (
            vec!["Doses".to_string(), "Spells".into(), "Effects".into()],
            vec![cell("a"), cell("b"), cell("c")],
        ),
        "conmod" => (
            vec![
                "Type".to_string(),
                "Cost".into(),
                "Requirements".into(),
                "CR Increase".into(),
            ],
            vec![cell("a"), cell("b"), cell("c"), cell("d")],
        ),
        "reqcost" => (
            vec!["Requirements".to_string(), "Cost".into()],
            vec![cell("a"), cell("b")],
        ),
        other => {
            let mut row = Vec::new();
            for key in ["a", "b", "c", "d", "e", "f", "g", "h"] {
                if attrs.contains_key(key) {
                    row.push(cell(key));
                }
            }
            if row.is_empty() {
                return Err(format!("unsupported ::tabular format {other}"));
            }
            (
                (0..row.len())
                    .map(|index| format!("Field {}", index + 1))
                    .collect(),
                row,
            )
        }
    };
    let mut out = table_row(
        &headers
            .iter()
            .map(|header| (*header).to_string())
            .collect::<Vec<_>>(),
    );
    out.push('\n');
    out.push_str(&table_row(&vec!["---".to_string(); headers.len()]));
    out.push('\n');
    out.push_str(&table_row(&row));
    Ok(out)
}

fn render_rtraits_directive(rest: &str) -> Result<String, String> {
    let attrs = attributes(&format!("::rtraits{rest}"))?;
    for key in attrs.keys() {
        if !["clear", "plural", "main"].contains(&key.as_str()) {
            return Err(format!("unknown attribute {key} on ::rtraits"));
        }
    }
    let plural = attr(&attrs, "plural").ok_or("::rtraits has no plural name")?;
    let main = attr(&attrs, "main").ok_or("::rtraits has no main list")?;
    let mut out = String::new();
    for bit in main.split('~') {
        let mut parts = bit.split('|');
        let kind = parts.next().unwrap_or_default();
        let data: Vec<&str> = parts.collect();
        let (title, text) = match kind {
            "LANG" | "LANGX" => {
                let split = data.iter().position(|part| part.is_empty());
                let (pre, post) = match split {
                    Some(index) => (&data[..index], &data[index + 1..]),
                    None => (&data[..], &[][..]),
                };
                let pre: Vec<String> = pre.iter().map(|part| (*part).to_string()).collect();
                let post: Vec<String> = post.iter().map(|part| (*part).to_string()).collect();
                let speaking = if kind == "LANGX" {
                    format!("{} can choose from the following bonus languages", plural)
                } else {
                    format!("{plural} begin play speaking {}", engine_join(&pre))
                };
                let bonus = if post.first() == Some(&"ANY".to_string()) {
                    "any languages they want (except secret languages, such as Druidic)".to_string()
                } else {
                    format!("from the following bonus languages: {}", engine_join(&post))
                };
                (
                    "Languages".to_string(),
                    format!(
                        "{speaking}. {plural} with high Intelligence scores can choose {bonus}."
                    ),
                )
            }
            "SPEED" => match data.first().copied() {
                Some("normal") => (
                    "Normal Speed".into(),
                    format!("{plural} have a base speed of 30 feet."),
                ),
                Some("normalLand") => (
                    "Normal Speed".into(),
                    format!("{plural} have a base speed of 30 feet on land."),
                ),
                Some("slow") => (
                    "Slow Speed".into(),
                    format!(
                        "{plural} have a base speed of {} feet.",
                        data.get(1).copied().unwrap_or("20")
                    ),
                ),
                _ => ("Speed".into(), data.join(" ")),
            },
            "CLIMB" => (
                "Natural Climber".into(),
                format!(
                    "{plural} have a climb speed of {} feet.",
                    data.first().copied().unwrap_or_default()
                ),
            ),
            "FSWIM" => (
                "Fast Swimmer".into(),
                format!("{plural} have a swim speed of 60 feet."),
            ),
            "SWIM" => (
                "Swim Speed".into(),
                format!(
                    "{plural} have a swim speed of {} feet.",
                    data.first().copied().unwrap_or_default()
                ),
            ),
            "SWIM+" => (
                "Swim Speed".into(),
                format!(
                    "{plural} have a swim speed of 30 feet, can move in water without making Swim checks, and always treat Swim as a class skill."
                ),
            ),
            "MEDIUM" => (
                "Medium".into(),
                format!(
                    "{plural} are Medium creatures and have no bonuses or penalties due to their size."
                ),
            ),
            "SMALL" => (
                "Small".into(),
                format!(
                    "{plural} are Small creatures and gain a +1 size bonus to their AC, a +1 size bonus on attack rolls, a -1 penalty on combat maneuver checks and to their Combat Maneuver Defense, and a +4 size bonus on Stealth checks."
                ),
            ),
            "NATOUT" => (
                "Native Outsider".into(),
                format!("{plural} are outsiders with the native subtype."),
            ),
            "MONHUM" => (
                "Monstrous Humanoid".into(),
                match data.first().copied() {
                    Some(subtype) if !subtype.is_empty() => {
                        format!("{plural} are monstrous humanoids with the {subtype} subtype.")
                    }
                    _ => format!("{plural} are monstrous humanoids."),
                },
            ),
            "HUM" => (
                "Humanoid".into(),
                format!(
                    "{plural} are humanoids with the {} subtype{}.",
                    engine_join(
                        &data
                            .iter()
                            .map(|subtype| (*subtype).to_string())
                            .collect::<Vec<_>>()
                    ),
                    if data.len() > 1 { "s" } else { "" }
                ),
            ),
            "LSENS" => (
                "Light Sensitivity".into(),
                format!("{plural} are dazzled in areas of bright light."),
            ),
            "LSUN" => (
                "Light Sensitivity".into(),
                format!(
                    "{plural} are dazzled in areas of bright sunlight or within the radius of a daylight spell."
                ),
            ),
            "DV" => (
                "Darkvision".into(),
                format!("{plural} can see in the dark up to 60 feet."),
            ),
            "SDV" => (
                "Superior Darkvision".into(),
                format!("{plural} can see in the dark up to 120 feet."),
            ),
            "LLV" => (
                "Low-Light Vision".into(),
                format!("{plural} can see twice as far as humans in conditions of dim light."),
            ),
            "NATARM" => (
                "Natural Armor".into(),
                format!(
                    "{plural} have a +{} natural armor bonus.",
                    data.first().copied().unwrap_or_default()
                ),
            ),
            "KEENSEN" => (
                "Keen Senses".into(),
                format!("{plural} receive a +2 racial bonus on Perception checks."),
            ),
            _ => (kind.to_string(), convert_encoded_links(&data.join(" "))),
        };
        if title.is_empty() && text.is_empty() {
            continue;
        }
        push_labeled(&mut out, &title, &text);
    }
    Ok(out)
}

fn strip_icon_directives(md: &str, env: &mut dyn crate::html::Env) -> String {
    let mut out = String::new();
    let mut rest = md;
    while let Some(start) = rest.find("@I-") {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let end = tail[1..]
            .char_indices()
            .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
            .map(|(index, _)| index + 1)
            .unwrap_or(tail.len());
        let token = &tail[..end];
        env.stripped(&format!("icon_directive:{token}"));
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
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
fn render_semantic_directive(
    name: &str,
    rest: &str,
    env: &mut dyn crate::html::Env,
) -> Result<String, String> {
    // `::rtraits` and `::aff` have their own parsers because their attributes hold
    // repeated values and macro tokens that the generic attribute map cannot express.
    if name == "rtraits" {
        return crate::pf_directives::racial_traits_markdown(rest);
    }
    if name == "aff" {
        return Ok(crate::pf_directives::affliction(rest, true)?.markdown);
    }
    let title = bracket(rest).unwrap_or_default();
    let attrs = if name == "mfn"
        || (!rest.contains('{') && matches!(name, "spellAb" | "cantrips" | "list"))
    {
        BTreeMap::new()
    } else {
        attributes(&format!("::{name}{rest}"))?
    };
    let allowed: &[&str] = match name {
        "ab" => &[
            "clear",
            "jl",
            "id",
            "flavor",
            "indent",
            "sub",
            "next",
            "head",
            "icon",
            "l",
            "l1",
            "l2",
            "l3",
            "l4",
            "l5",
            "l6",
            "l7",
            "l8",
            "l9",
            "l10",
            "l11",
            "l12",
            "l13",
            "l14",
            "l15",
            "l16",
            "l17",
            "l18",
            "l19",
            "l20",
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
            "repeat",
            "repeatAt",
            "standard",
            "swift",
            "immediate",
            "fullround",
            "move",
            "free",
            "passive",
            "special",
            "note",
            "choice",
            "info",
            "benefit",
            "normal",
            "goal",
            "compbenefit",
            "provokes",
            "order",
            "usage",
            "title",
            "replace",
            "alter",
            "type",
            "prereq",
            "hl",
            "action",
            "ability",
            "ability2",
            "ability3",
            "xFunctions_As",
            "yAt_8th_Level",
            "useInc",
            "useF",
            "useL",
            "useLMod",
            "useMod",
            "useM",
            "useHD",
            "useUnit",
            "useNC",
            "increment",
            "incrementAt",
            "incrementEnd",
            "incrementPlain",
            "incrementDesc",
            "incrementOrd",
            "incrementMulti",
            "incrementMax",
            "incrementRoman",
            "incrementUse",
            "s0",
            "s1",
            "s2",
            "s3",
            "s4",
            "s5",
            "s6",
            "s7",
            "s8",
            "s9",
            "s10",
            "s11",
            "s12",
            "s13",
            "s14",
            "s15",
            "s16",
            "s17",
            "s18",
            "s19",
            "s20",
        ],
        "aff" => &[
            "clear",
            "iconP",
            "iconI",
            "iconC",
            "iconD",
            "iconA",
            "poison",
            "curse",
            "infest",
            "disease",
            "type",
            "save",
            "saveF",
            "saveW",
            "onset",
            "dcF",
            "dcW",
            "dcR",
            "dcYou",
            "dcIt",
            "dcPoss",
            "dcLev",
            "dcHD",
            "dcMod",
            "track",
            "trackmod",
            "freq",
            "freqR",
            "freqM",
            "freqH",
            "freqD",
            "eff",
            "ineff",
            "seceff",
            "cure",
            "cure1",
            "cure2",
            "cure2c",
            "cure3",
            "cure3c",
            "extra",
            "start",
            "nolink",
            "effStr",
            "effStrD",
            "effDex",
            "effDexD",
            "effCon",
            "effConD",
            "effInt",
            "effIntD",
            "effWis",
            "effWisD",
            "effCha",
            "effChaD",
            "effExtra",
            "effOr",
            "ineffStr",
            "ineffStrD",
            "ineffDex",
            "ineffDexD",
            "ineffCon",
            "ineffConD",
            "ineffInt",
            "ineffIntD",
            "ineffWis",
            "ineffWisD",
            "ineffCha",
            "ineffChaD",
            "ineffExtra",
            "ineffOr",
            "seceffStr",
            "seceffStrD",
            "seceffDex",
            "seceffDexD",
            "seceffCon",
            "seceffConD",
            "seceffInt",
            "seceffIntD",
            "seceffWis",
            "seceffWisD",
            "seceffCha",
            "seceffChaD",
            "seceffExtra",
            "seceffOr",
        ],
        "trait" => &[
            "clear",
            "jl",
            "id",
            "title",
            "type",
            "flavor",
            "benefit",
            "req",
            "region",
            "religion",
            "subs",
            "surs",
            "campaign",
            "sub",
            "head",
            "indent",
            "singleBreaks",
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
            "keenScent",
            "scent",
            "thoughtsense",
            "greensight",
            "lifesense",
            "blindsight",
            "blindsightParens",
            "blindsense",
            "tremorsense",
            "tremorParens",
            "mistsight",
            "xray",
            "aav",
            "sid",
            "pcp",
            "aura",
            "clear",
        ],
        "mdefense" => &[
            "ac", "mod", "hpRaw", "hp", "fh", "regen", "fort", "ref", "will", "def", "chanRes",
            "fortif", "split", "ink", "pBlood", "trapS", "unstop", "blockAt", "rockCt", "secSave",
            "ferocity", "amorph", "aav", "incorp", "noB", "eva", "unc", "impEva", "impUnc", "dr",
            "immune", "resist", "sr", "vulner", "weak",
        ],
        "moffense" => &[
            "clear",
            "sp",
            "spP",
            "br",
            "brP",
            "cl",
            "clP",
            "sw",
            "swP",
            "fl",
            "flP",
            "clumsy",
            "poor",
            "average",
            "good",
            "perfect",
            "jet",
            "spOther",
            "spExtra",
            "melee",
            "ranged",
            "space",
            "reach",
            "reachP",
            "specAtt",
            "bDrain",
            "bleed",
            "brWeap",
            "burn",
            "capsize",
            "chEn",
            "chNEn",
            "chPEn",
            "constrict",
            "distraction",
            "eDrain",
            "engulf",
            "entrap",
            "favEn",
            "grab",
            "heat",
            "mMagic",
            "mPower",
            "paralysis",
            "powCh",
            "pull",
            "push",
            "rake",
            "rend",
            "rockTh",
            "sneak",
            "swallow",
            "trample",
            "web",
            "whirlwind",
            "attach",
            "bloodRage",
            "fSwallow",
            "ferocity",
            "gaze",
            "pounce",
            "smother",
            "strangle",
            "trip",
            "burrow",
            "fly",
        ],
        "mstats" => &[
            "clear",
            "str",
            "dex",
            "con",
            "int",
            "wis",
            "cha",
            "bab",
            "cmb",
            "cmd",
            "feats",
            "skills",
            "racial",
            "lang",
            "sq",
            "combat",
            "othergear",
            "gear",
            "faith",
        ],
        "mspell" => &[
            "clear", "cl", "con", "sla", "atWill", "constant", "day", "hour", "week", "month",
            "year", "other", "prep", "ex", "know", "psy", "psyMag", "pe", "peP", "title", "data",
            "newLine", "l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9",
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
        "prereq" => &[
            "clear",
            "l",
            "c",
            "r",
            "g1",
            "g1title",
            "g1protocol",
            "other",
            "sep",
        ],
        "list" => &[
            "clear", "all", "link", "and", "end", "hl", "em", "sep", "comma", "special",
        ],
        "spellAb" => &[
            "stat",
            "learn",
            "prepare",
            "zeroMsg",
            "type",
            "caster",
            "alignLimit",
            "spontaneous",
            "occultist",
            "bard",
            "skald",
            "ritualDivine",
            "choice",
            "druid",
            "meditate",
            "shaman",
            "oracle",
            "medium",
            "ritualArcane",
            "witch",
            "limited",
            "limitedFull",
            "cureInflict",
            "summonNature",
            "trade53",
            "trade42",
            "tradeLimit",
            "clear",
            "id",
        ],
        "cantrips" => &["c", "p", "clear", "id"],
        _ => {
            return Err(format!(
                "unsupported semantic renderer for directive ::{name}"
            ));
        }
    };
    for key in attrs.keys() {
        let extension =
            name == "ab" && key.len() > 1 && matches!(key.chars().next(), Some('x' | 'y' | 'z'));
        if !allowed.contains(&key.as_str()) && !extension {
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
    if name == "prereq" {
        return Ok(render_prerequisite(&attrs));
    }
    if name == "list" {
        return render_list(&title, &attrs);
    }
    if name == "spellAb" {
        return render_spell_ability(&attrs);
    }
    if name == "cantrips" {
        return render_cantrips(&attrs);
    }
    if name == "mstats" {
        return render_mstats(&title, &attrs);
    }
    if name == "meco" {
        return render_meco(&title, &attrs);
    }
    if name == "ab" {
        return Ok(render_ability(&title, &attrs, env));
    }
    if name == "aff" {
        return render_aff(&title, &attrs);
    }
    if name == "moffense" {
        return render_moffense(&title, &attrs);
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
        let list_field = name == "mdefense" && matches!(key.as_str(), "hp" | "def" | "weak");
        if value.contains(['~', '|']) && !list_field {
            return Err(format!("unsupported encoded {key} value on ::{name}"));
        }
    }
    let title = if title.is_empty() {
        attrs.get("title").cloned().unwrap_or_default()
    } else {
        title
    };
    // An empty title is valid (`::minfo{...}` has no bracketed heading). Do not
    // emit an empty `****`, which Markdown parses as a thematic break and the
    // HTML pass then drops as an `<hr>`.
    let mut out = if title.is_empty() {
        String::new()
    } else {
        format!("**{}**", convert_encoded_links(&title))
    };
    for (key, value) in attrs {
        if ["icon", "head", "next", "order", "hl", "clear"].contains(&key.as_str()) {
            continue;
        }
        if name == "mdefense" && key == "hp" {
            let (amount, expression) = value
                .split_once('~')
                .ok_or_else(|| "unsupported hit point encoding on ::mdefense".to_string())?;
            push_labeled(&mut out, "Hit points", &format!("{amount} ({expression})"));
            continue;
        }
        if name == "mdefense" && (key == "def" || key == "weak") {
            let label = if key == "def" {
                "Defensive abilities"
            } else {
                "Weaknesses"
            };
            let list = value
                .split(['~', '|'])
                .filter(|part| !part.is_empty())
                .map(convert_encoded_links)
                .collect::<Vec<_>>()
                .join(", ");
            push_labeled(&mut out, label, &list);
            continue;
        }
        let value = if value.is_empty() {
            "present"
        } else {
            value.as_str()
        };
        push_labeled(&mut out, &field_label(&key), &convert_encoded_links(value));
    }
    Ok(out)
}

fn render_prerequisite(attrs: &BTreeMap<String, String>) -> String {
    let separator = attr(attrs, "sep").unwrap_or("~");
    let mut lines = Vec::new();
    if let Some(level) = attr(attrs, "l") {
        let label = level
            .parse::<u32>()
            .map(ordinal)
            .unwrap_or_else(|_| level.to_string());
        if let Some(class) = attr(attrs, "c") {
            lines.push(format!("{label} level {class}"));
        } else {
            lines.push(format!("{label} level"));
        }
    }
    if let Some(race) = attr(attrs, "r") {
        lines.push(convert_encoded_links(race));
    }
    if let (Some(items), Some(_protocol)) = (attr(attrs, "g1"), attr(attrs, "g1protocol")) {
        let list = items
            .split(separator)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(convert_encoded_links)
            .collect::<Vec<_>>()
            .join(", ");
        if let Some(title) = attr(attrs, "g1title") {
            lines.push(format!("{}: {list}", convert_encoded_links(title)));
        } else {
            lines.push(list);
        }
    }
    if let Some(other) = attr(attrs, "other") {
        lines.push(convert_encoded_links(other));
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("**Prerequisites:** {}", lines.join("; "))
    }
}

/// The `::list` values use upstream link markers (`»`, `«`, `<_`, `_>`).
/// Only the visible text belongs in the typed body.
fn clean_list_item(input: &str) -> String {
    let mut text = input.trim();
    if let Some((_, after)) = text.rsplit_once('»') {
        text = after;
    }
    if let Some((before, _)) = text.split_once('<') {
        text = before;
    }
    if let Some((_, after)) = text.split_once('>') {
        text = after;
    }
    convert_encoded_links(text.replace('«', "").trim())
}

fn render_list(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let all = attr(attrs, "all").ok_or("::list has no all attribute")?;
    let separator = attr(attrs, "sep").unwrap_or("~");
    let comma = attr(attrs, "comma").unwrap_or(", ");
    let items: Vec<String> = all
        .split(separator)
        .map(clean_list_item)
        .filter(|item| !item.is_empty())
        .collect();
    if items.is_empty() {
        return Ok(String::new());
    }
    let joined = if let Some(and) = attr(attrs, "and") {
        if items.len() >= 3 {
            format!(
                "{}{comma}{and} {}",
                items[..items.len() - 1].join(comma),
                items[items.len() - 1]
            )
        } else if items.len() == 2 {
            format!("{} {and} {}", items[0], items[1])
        } else {
            items[0].clone()
        }
    } else {
        items.join(comma)
    };
    let heading = if title.trim().is_empty() {
        "List"
    } else {
        title.trim()
    };
    Ok(format!(
        "**{}:** {joined}{}",
        convert_encoded_links(heading),
        attr(attrs, "end").unwrap_or("")
    ))
}

fn capitalize_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// A class spellcasting summary from the attributes of `::spellAb`.
///
/// The pinned OGC JSON stores only flags (`type`, `stat`, `caster`, `spontaneous`,
/// `trade53`, ...). Upstream's prose for them lives in its GPL renderer
/// (`prebuild/block/spellAb.js`), which must not be copied or ported (AGENTS.md
/// rule 12). This states the same mechanics as short labelled facts in our own words.
fn render_spell_ability(attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let has = |key: &str| attrs.contains_key(key);
    let caster = attr(attrs, "caster").unwrap_or("class");
    let stat = attr(attrs, "stat").unwrap_or("spellcasting ability");
    let mut out = String::from("**Spells**");
    let kind = attr(attrs, "type").map(capitalize_first);
    push_labeled(
        &mut out,
        "Spell list",
        &match kind {
            Some(kind) => format!("{kind} spells from the {caster} list, from 1st level"),
            None => format!("Spells from the {caster} list, from 1st level"),
        },
    );
    let mut ability = format!(
        "{stat}. Minimum score 10 + spell level; save DC 10 + spell level + {stat} modifier; a high score grants bonus spells per day"
    );
    if has("zeroMsg") {
        ability.push_str(", even at a level where the table shows 0 spells per day");
    }
    ability.push('.');
    push_labeled(&mut out, "Key ability", &ability);
    let casting = if has("spontaneous") {
        "Spontaneous: cast any spell known without preparation, within the daily allotment per spell level."
    } else {
        "Prepared: choose and prepare spells in advance."
    };
    push_labeled(&mut out, "Casting", casting);
    if has("limited") || has("limitedFull") {
        let start = if has("medium") {
            "two 0-level spells"
        } else {
            "four 0-level and two 1st-level spells"
        };
        push_labeled(
            &mut out,
            "Spells known",
            &format!(
                "{} selection; starts with {start} and grows by level as shown on the table. {stat} does not change the number known.",
                if has("limited") {
                    "Very limited"
                } else {
                    "Limited"
                }
            ),
        );
    }
    if has("trade53") || has("trade42") {
        let (first, step) = if has("trade53") { (5, 3) } else { (4, 2) };
        let mut swap = format!(
            "At {}, {}, {}, and every {step} levels after, one known spell may be exchanged for another of the same level",
            ordinal(first),
            ordinal(first + step),
            ordinal(first + 2 * step)
        );
        if has("tradeLimit") {
            swap.push_str(" that is at least one level below the highest level you can cast");
        }
        swap.push('.');
        push_labeled(&mut out, "Spell swap", &swap);
    }
    let mut added = Vec::new();
    if has("cureInflict") {
        added.push("all cure or all inflict spells (chosen at 1st level, permanent)");
    }
    if has("summonNature") {
        added.push("all summon nature's ally spells");
    }
    if has("occultist") {
        added.push("one spell per castable level from each implement school's list");
    }
    if !added.is_empty() {
        push_labeled(
            &mut out,
            "Added to spells known",
            &format!("{}.", added.join("; ")),
        );
    }
    let mut notes = Vec::new();
    if has("alignLimit") {
        notes.push("Alignment can bar spells opposed to it.".to_string());
    }
    if has("bard") || has("skald") {
        notes.push("Every spell has a verbal component (song, recitation, or music).".into());
    }
    if has("occultist") {
        notes.push("Every spell has an implement component.".into());
    }
    if has("oracle") {
        notes.push("No divine focus is needed for spells that list DF.".into());
    }
    if has("ritualDivine") {
        let how = if has("druid") {
            "meditating on nature"
        } else if has("meditate") {
            "in meditation"
        } else {
            "in contemplation or prayer"
        };
        let when = if has("choice") {
            " at a chosen time"
        } else {
            ""
        };
        notes.push(format!(
            "Regains spells after 1 hour {how}{when} each day; any spell on the list can be prepared."
        ));
    }
    if has("ritualArcane") {
        let with = if has("witch") {
            "communing with the familiar"
        } else {
            "with the spellbook"
        };
        notes.push(format!(
            "Prepares spells after 8 hours of rest and 1 hour {with}; no limit on spells known."
        ));
    }
    if !notes.is_empty() {
        push_labeled(&mut out, "Notes", &notes.join(" "));
    }
    Ok(out)
}

/// A cantrip summary from `::cantrips{c=... p}` attributes, in our own words.
fn render_cantrips(attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let c = attr(attrs, "c").unwrap_or("cantrips");
    let kind = match c {
        "knacks" => "psychic",
        "orisons" => "divine",
        _ => "arcane",
    };
    let mut out = format!("**{}**", capitalize_first(c));
    let reuse = if attrs.contains_key("p") {
        "Prepared like other spells but not expended when cast"
    } else {
        "Cast without using a spell slot"
    };
    push_labeled(
        &mut out,
        "Ability",
        &format!(
            "0-level {kind} spells. {reuse}; one placed in a higher slot (for example through metamagic) uses that slot."
        ),
    );
    Ok(out)
}

fn render_bonus_macro(token: &str) -> Option<String> {
    let body = token.strip_prefix("@B-")?;
    let parts: Vec<&str> = body.split('-').collect();
    let formula_index = parts.iter().position(|part| {
        part.chars()
            .next()
            .is_some_and(|first| matches!(first, '2' | '3' | '4' | '5' | '6'))
    })?;
    let formula = parts[formula_index];
    let class = parts.get(formula_index + 1)?;
    let mut chars = formula.chars();
    let denominator = chars.next()?.to_digit(10)?;
    let gender = chars.next()?;
    let rest: String = chars.collect();
    let plural = rest.contains('p');
    let extra = rest
        .chars()
        .find(|c| c.is_ascii_digit())
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0);
    let fraction = match denominator {
        2 => "half",
        3 => "one-third",
        4 => "one-fourth",
        5 => "one-fifth",
        6 => "one-sixth",
        _ => return None,
    };
    let possessive = match gender {
        'm' => "his",
        'f' => "her",
        'y' => "your",
        't' => "the",
        'a' => "a",
        'n' => "an",
        _ => return None,
    };
    let apostrophe = if matches!(gender, 't' | 'a' | 'n') {
        "'s"
    } else {
        ""
    };
    let kind = if formula_index > 0 {
        parts[..formula_index].join("-").replace('_', " ")
    } else {
        String::new()
    };
    let pre = if plural {
        format!(
            "These {} are equal to ",
            if kind.is_empty() {
                "bonuses"
            } else {
                kind.as_str()
            }
        )
    } else {
        format!(
            "This {} is equal to ",
            if kind.is_empty() {
                "bonus"
            } else {
                kind.as_str()
            }
        )
    };
    let plus = if extra > 0 {
        format!("{extra} + ")
    } else {
        String::new()
    };
    Some(format!(
        "{pre}{plus}{fraction} of {possessive} {}{apostrophe} level",
        class.replace('_', " ")
    ))
}

fn pretty_value(value: &str) -> String {
    value.replace("~~~", "\n\n").replace('~', ", ")
}

fn render_ability(
    title: &str,
    attrs: &BTreeMap<String, String>,
    env: &mut dyn crate::html::Env,
) -> String {
    let heading = if title.is_empty() {
        attrs.get("title").cloned().unwrap_or_default()
    } else {
        title.to_string()
    };
    // An unnamed ability is valid. Emitting `****` here would turn into a
    // Markdown thematic break and be dropped as an `<hr>`.
    let mut out = if heading.is_empty() {
        String::new()
    } else {
        format!("**{}**", convert_encoded_links(&heading))
    };
    for (key, value) in attrs {
        match key.as_str() {
            "title" | "icon" | "hl" | "clear" => {}
            "head" | "next" | "sub" | "indent" => env.stripped("ab_structural_metadata"),
            "order" => env.stripped("ab_order_metadata"),
            "jl" => {
                if value.is_empty() || value == "strip" {
                    env.stripped("ab_jump_list_metadata");
                } else {
                    push_labeled(&mut out, "Section", value);
                }
            }
            "replace" => push_labeled(&mut out, "Replaces", &joined(value, ", ")),
            "alter" => push_labeled(&mut out, "Alters", &joined(value, ", ")),
            "provokes" => push_labeled(
                &mut out,
                "Provokes attack of opportunity",
                if value.is_empty() { "Yes" } else { value },
            ),
            key if key.starts_with("use") || key.starts_with("increment") => {}
            "l" => push_labeled(
                &mut out,
                "Gained",
                &if let Ok(level) = value.parse::<u32>() {
                    format!("At {} Level", ordinal(level))
                } else {
                    convert_encoded_links(value)
                },
            ),
            key if key.starts_with('l')
                && key[1..].chars().all(|c| c.is_ascii_digit())
                && !key[1..].is_empty() =>
            {
                let level = key[1..].parse::<u32>().unwrap_or_default();
                push_labeled(
                    &mut out,
                    &format!("At {} Level", ordinal(level)),
                    &convert_encoded_links(&pretty_value(value)),
                )
            }
            key if key.starts_with("imp")
                && key[3..].chars().all(|c| c.is_ascii_digit())
                && !key[3..].is_empty() =>
            {
                let level = key[3..].parse::<u32>().unwrap_or_default();
                push_labeled(
                    &mut out,
                    &format!("At {} Level", ordinal(level)),
                    &convert_encoded_links(&pretty_value(value)),
                )
            }
            key if key.starts_with('s')
                && key[1..].chars().all(|c| c.is_ascii_digit())
                && !key[1..].is_empty() =>
            {
                push_labeled(
                    &mut out,
                    if key == "s0" { "Cantrips" } else { "Spells" },
                    &convert_encoded_links(&pretty_value(value)),
                )
            }
            key if key.len() > 1 && matches!(key.chars().next(), Some('x' | 'y' | 'z')) => {
                // The upstream renderer marks source-extension fields with a
                // leading `x`, `y`, or `z`. They carry content (alternate orders,
                // penalties, descriptions), so render the field without the prefix.
                push_labeled(
                    &mut out,
                    &field_label(&key[1..].replace('_', " ")),
                    &convert_encoded_links(&pretty_value(value)),
                )
            }
            _ => {
                let value = if value.is_empty() { "present" } else { value };
                push_labeled(
                    &mut out,
                    &field_label(key),
                    &convert_encoded_links(&pretty_value(value)),
                );
            }
        }
    }
    if let Some(usage) = render_usage(attrs) {
        push_labeled(&mut out, "Usage", &usage);
    }
    for line in render_increment(attrs) {
        out.push_str(&format!("\n\n{line}"));
    }
    out
}

fn usage_unit(attrs: &BTreeMap<String, String>) -> &'static str {
    match attrs.get("useUnit").map(String::as_str) {
        Some("r") => "round",
        Some("m") => "minute",
        Some("h") => "hour",
        Some("t") => "time",
        Some("round") => "round",
        Some("minute") => "minute",
        Some("hour") => "hour",
        Some("time") => "time",
        _ => "time",
    }
}

fn plural_unit(unit: &str, count: u32) -> String {
    if count == 1 {
        unit.to_string()
    } else {
        format!("{unit}s")
    }
}

fn render_usage(attrs: &BTreeMap<String, String>) -> Option<String> {
    let unit = usage_unit(attrs);
    let base = if let Some(usage) = attr(attrs, "usage") {
        convert_encoded_links(usage)
    } else if let Some(use_inc) = attr(attrs, "useInc") {
        let parts: Vec<&str> = use_inc.split('~').collect();
        let class = parts.first().copied().unwrap_or("class");
        let interval = parts
            .get(1)
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1);
        let start = parts.get(2).copied().unwrap_or_default();
        let initial = parts
            .get(3)
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1);
        let unit = plural_unit(unit, initial);
        if start.is_empty() {
            format!("{initial} {unit}/day per {interval} {class} levels")
        } else if start == "+" {
            format!("{initial} {unit}/day + 1 per {interval} {class} levels")
        } else {
            let level = start.parse::<u32>().unwrap_or(1);
            format!(
                "{initial} {unit}/day + 1 per {interval} {class} levels beyond {}",
                ordinal(level)
            )
        }
    } else if let Some(use_l) = attr(attrs, "useL") {
        let parts: Vec<&str> = use_l.split('~').collect();
        let class = parts.first().copied().unwrap_or("class");
        let amount = parts
            .get(1)
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1);
        format!(
            "{amount} {}/day per {class} level",
            plural_unit(unit, amount)
        )
    } else if let Some(use_lmod) = attr(attrs, "useLMod") {
        let (class, modifier) = use_lmod.split_once('~').unwrap_or((use_lmod, ""));
        format!(
            "{class} level + {} modifier {}/day",
            if modifier.is_empty() {
                "ability"
            } else {
                modifier
            },
            plural_unit(unit, 2)
        )
    } else if let Some(use_mod) = attr(attrs, "useMod") {
        format!("{use_mod} modifier {}/day", plural_unit(unit, 2))
    } else if attr(attrs, "useF").is_some() {
        "Uses follow the level progression encoded in the source.".to_string()
    } else {
        return None;
    };
    let mut base = base;
    if let Some(use_nc) = attr(attrs, "useNC") {
        if use_nc == "useNC" {
            base.push_str(&format!("; these {unit}s need not be consecutive"));
        } else {
            base.push_str(&format!(
                "; these {unit}s need not be consecutive, but must be spent in {use_nc}-{unit} increments"
            ));
        }
    }
    if let Some(use_m) = attr(attrs, "useM") {
        base.push_str(&format!(
            " (minimum {})",
            if use_m == "useM" { "1" } else { use_m }
        ));
    }
    Some(base)
}

fn render_increment(attrs: &BTreeMap<String, String>) -> Vec<String> {
    let max = attr(attrs, "incrementMax")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(20)
        .clamp(2, 20);
    let end = attr(attrs, "incrementEnd").unwrap_or(".");
    let descriptors: Vec<&str> = attr(attrs, "incrementDesc")
        .map(|value| value.split('~').filter(|part| !part.is_empty()).collect())
        .unwrap_or_default();
    let source = attr(attrs, "increment")
        .or_else(|| attr(attrs, "incrementOrd"))
        .or_else(|| attr(attrs, "incrementPlain"))
        .or_else(|| attr(attrs, "incrementRoman"));
    let Some(source) = source else {
        return Vec::new();
    };
    if attrs.contains_key("incrementMulti") || attrs.contains_key("incrementUse") {
        return vec![format!(
            "Progression: {}",
            convert_encoded_links(&joined(source, ", "))
        )];
    }
    let parts: Vec<&str> = source.split('~').collect();
    let message = parts.first().copied().unwrap_or("Ability");
    let start = parts
        .get(1)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let add = parts
        .get(2)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let mut bonus = parts
        .get(3)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(2);
    let bonus_inc = parts
        .get(4)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(1);
    let use_ordinal = attrs.contains_key("incrementOrd");
    let use_plain = attrs.contains_key("incrementPlain") || bonus <= 0;
    let mut level = start + add;
    let mut lines = Vec::new();
    let mut descriptor_index = 0;
    while level <= max && add > 0 {
        let bonus_text = if use_ordinal && bonus > 0 {
            ordinal(bonus as u32)
        } else if use_plain {
            bonus.to_string()
        } else {
            format!("+{bonus}")
        };
        let descriptor = if descriptors.is_empty() {
            String::new()
        } else {
            let value = descriptors[descriptor_index % descriptors.len()];
            descriptor_index += 1;
            format!("{value} ")
        };
        lines.push(format!(
            "At {} level: {} {}{}{}",
            ordinal(level),
            convert_encoded_links(message),
            descriptor,
            bonus_text,
            end
        ));
        level += add;
        bonus += bonus_inc;
    }
    lines
}

fn render_feats(value: &str) -> String {
    value
        .split('~')
        .filter(|item| !item.is_empty())
        .map(|item| {
            if let Some(literal) = item.strip_prefix('!') {
                return convert_encoded_links(literal);
            }
            let (item, footnote) = match item.split_once('#') {
                Some((item, footnote)) => (item, Some(footnote)),
                None => (item, None),
            };
            let (name, parens) = match item.split_once('|') {
                Some((name, parens)) => (name, Some(parens)),
                None => (item, None),
            };
            let (mythic, name) = match name.strip_prefix("M/") {
                Some(name) => (true, name),
                None => (false, name),
            };
            let mut out = String::new();
            if mythic {
                out.push_str("Mythic ");
            }
            out.push_str(&convert_encoded_links(name));
            if let Some(parens) = parens {
                out.push_str(&format!(" ({})", convert_encoded_links(parens)));
            }
            if let Some(footnote) = footnote {
                out.push_str(&format!("<sup>{footnote}</sup>"));
            }
            out
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn skill_name(key: &str) -> &str {
    match key {
        "acro" => "Acrobatics",
        "app" => "Appraise",
        "bluff" => "Bluff",
        "climb" => "Climb",
        "diplo" => "Diplomacy",
        "dis" => "Disguise",
        "dd" => "Disable Device",
        "ea" => "Escape Artist",
        "fly" => "Fly",
        "ha" => "Handle Animal",
        "heal" => "Heal",
        "intm" => "Intimidate",
        "ling" => "Linguistics",
        "per" => "Perception",
        "ride" => "Ride",
        "sm" => "Sense Motive",
        "soh" => "Sleight of Hand",
        "spc" => "Spellcraft",
        "stl" => "Stealth",
        "sur" => "Survival",
        "swim" => "Swim",
        "umd" => "Use Magic Device",
        other => other,
    }
}

fn knowledge_name(letter: &str) -> &str {
    match letter {
        "a" => "arcana",
        "d" => "dungeoneering",
        "e" => "engineering",
        "g" => "geography",
        "h" => "history",
        "l" => "local",
        "n" => "nature",
        "o" => "nobility",
        "p" => "planes",
        "r" => "religion",
        other => other,
    }
}

fn signed(value: &str) -> String {
    match value.parse::<i64>() {
        Ok(number) if number >= 0 => format!("+{number}"),
        _ => value.to_string(),
    }
}

fn render_skills(value: &str) -> String {
    value
        .split('~')
        .filter(|item| !item.is_empty())
        .map(|item| {
            let parts: Vec<&str> = item.split('|').collect();
            let key = parts.first().copied().unwrap_or_default();
            match key {
                "k" => {
                    let topics = parts.get(1).copied().unwrap_or_default();
                    let bonus = parts.get(2).copied().unwrap_or("0");
                    let names: Vec<_> = topics.split('/').map(knowledge_name).collect::<Vec<_>>();
                    let extra = parts.get(3).map(|value| format!(" ({value})"));
                    format!(
                        "Knowledge ({}) {}{}",
                        names.join(", "),
                        signed(bonus),
                        extra.unwrap_or_default()
                    )
                }
                "craft" | "perf" | "prof" => {
                    let of = parts.get(1).copied().unwrap_or_default();
                    let bonus = parts.get(2).copied().unwrap_or("0");
                    let extra = parts.get(3).map(|value| format!(" ({value})"));
                    let label = match key {
                        "craft" => "Craft",
                        "perf" => "Perform",
                        _ => "Profession",
                    };
                    format!(
                        "{label} ({}) {}{}",
                        convert_encoded_links(of),
                        signed(bonus),
                        extra.unwrap_or_default()
                    )
                }
                _ => {
                    let bonus = parts.get(1).copied().unwrap_or("0");
                    let extra = parts.get(2).map(|value| format!(" ({value})"));
                    format!(
                        "{} {}{}",
                        skill_name(key),
                        signed(bonus),
                        extra.unwrap_or_default()
                    )
                }
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn language_name(code: &str) -> String {
    let (code, can_speak) = match code.strip_suffix('X') {
        Some(code) => (code, false),
        None => (code, true),
    };
    let name = match code {
        "A" => "Abyssal",
        "Ab" => "Aboleth",
        "ALL" => "all (language mastery)",
        "Ak" => "Aklo",
        "AO" => "Ancient Osiriani",
        "An" => "Androffan",
        "Aq" => "Aquan",
        "Au" => "Auran",
        "Az" => "Azlanti",
        "B" => "Boggard",
        "C" => "Common",
        "Ce" => "Celestial",
        "Cy" => "Cyclops",
        "D" => "Draconic",
        "DF" => "Dark Folk",
        "Dr" => "Druidic",
        "Dw" => "Dwarven",
        "E" => "Elven",
        "G" => "Giant",
        "Gl" => "Gnoll",
        "Gb" => "Goblin",
        "H" => "Halfling",
        "Ht" => "Hallit",
        "I" => "Infernal",
        "Ig" => "Ignan",
        "N" => "Necril",
        "O" => "Orc",
        "P" => "Protean",
        "Po" => "Polyglot",
        "S" => "Sylvan",
        "Sh" => "Shadowtongue",
        "Sx" => "Sphinx",
        "T" => "Terran",
        "Th" => "Thassilonian",
        "U" => "Undercommon",
        other => other,
    };
    if can_speak {
        name.to_string()
    } else {
        format!("{name} (can't speak)")
    }
}

fn render_languages(value: &str) -> String {
    let mut parts = value.splitn(2, "~~");
    let normal = parts.next().unwrap_or_default();
    let special = parts.next();
    let mut out = normal
        .split('~')
        .filter(|code| !code.is_empty())
        .map(language_name)
        .collect::<Vec<_>>()
        .join(", ");
    if let Some(special) = special.filter(|special| !special.is_empty()) {
        out.push_str("; ");
        out.push_str(&convert_encoded_links(special));
    }
    out
}

fn render_mstats(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("**{}**", convert_encoded_links(title)));
    } else {
        out.push_str("**Statistics**");
    }
    let ability_keys = ["str", "dex", "con", "int", "wis", "cha"];
    if ability_keys.iter().any(|key| attrs.contains_key(*key)) {
        let scores: Vec<_> = [
            ("Str", "str"),
            ("Dex", "dex"),
            ("Con", "con"),
            ("Int", "int"),
            ("Wis", "wis"),
            ("Cha", "cha"),
        ]
        .iter()
        .filter_map(|(label, key)| attrs.get(*key).map(|value| format!("{label} {value}")))
        .collect();
        push_labeled(&mut out, "Ability scores", &scores.join(", "));
    }
    for (key, label) in [
        ("bab", "Base attack bonus"),
        ("cmb", "Combat maneuver bonus"),
        ("cmd", "Combat maneuver defense"),
    ] {
        if let Some(value) = attr(attrs, key) {
            push_labeled(&mut out, label, &convert_encoded_links(value));
        }
    }
    if let Some(value) = attr(attrs, "feats") {
        push_labeled(&mut out, "Feats", &render_feats(value));
    }
    if let Some(value) = attr(attrs, "skills") {
        push_labeled(&mut out, "Skills", &render_skills(value));
    }
    if let Some(value) = attr(attrs, "racial") {
        push_labeled(&mut out, "Racial Modifiers", &convert_encoded_links(value));
    }
    if let Some(value) = attr(attrs, "lang") {
        push_labeled(&mut out, "Languages", &render_languages(value));
    }
    if let Some(value) = attr(attrs, "sq") {
        push_labeled(&mut out, "Special qualities", &convert_encoded_links(value));
    }
    for (key, label) in [
        ("combat", "Combat gear"),
        ("gear", "Gear"),
        ("othergear", "Other gear"),
    ] {
        if let Some(value) = attr(attrs, key) {
            push_labeled(&mut out, label, &render_loot_list(value));
        }
    }
    if let Some(value) = attr(attrs, "faith") {
        push_labeled(&mut out, "Faith", &convert_encoded_links(value));
    }
    Ok(out)
}

const ARMOR_CODES: &[(&str, &str)] = &[
    ("l", "leather armor"),
    ("sl", "studded leather armor"),
    ("bp", "breastplate"),
    ("fp", "full plate"),
    ("hp", "half-plate"),
    ("hss", "heavy steel shield"),
    ("hws", "heavy wooden shield"),
    ("lws", "light wooden shield"),
    ("lss", "light steel shield"),
    ("cs", "chain shirt"),
    ("sm", "scale mail"),
    ("b", "buckler"),
    ("bd", "barding"),
    ("p", "padded"),
    ("cm", "chainmail"),
    ("w", "wooden"),
    ("bm", "banded mail"),
    ("ak", "armored kilt"),
];

const WEAPON_CODES: &[(&str, &str)] = &[
    ("sp", "spear"),
    ("sps", "spears"),
    ("sc", "scimitar"),
    ("scs", "scimitars"),
    ("si", "sickle"),
    ("sis", "sickles"),
    ("k", "katana"),
    ("d", "dagger"),
    ("ds", "daggers"),
    ("ls", "longsword"),
    ("lss", "longswords"),
    ("spkc", "spiked chain"),
    ("ss", "short sword"),
    ("sss", "short swords"),
    ("c", "club"),
    ("q", "quarterstaff"),
    ("n", "net"),
    ("j", "javelin"),
    ("js", "javelins"),
    ("ms", "morningstar"),
    ("drt", "dart"),
    ("sb", "shortbow"),
    ("lb", "longbow"),
    ("sng", "sling"),
    ("t", "trident"),
    ("ba", "battleaxe"),
    ("bas", "battleaxes"),
    ("ga", "greataxe"),
    ("gs", "greatsword"),
    ("bs", "bastard sword"),
    ("gc", "greatclub"),
    ("f", "falchion"),
    ("sy", "scythe"),
    ("lf", "light flail"),
    ("lfs", "light flails"),
    ("sk", "starknife"),
    ("hc", "heavy crossbow"),
    ("lc", "light crossbow"),
    ("wh", "warhammer"),
    ("ssp", "shortspear"),
    ("ssps", "shortspears"),
    ("lh", "lucerne hammer"),
    ("r", "rapier"),
    ("rs", "rapiers"),
    ("g", "glaive"),
    ("ku", "kukri"),
    ("kus", "kukris"),
    ("hp", "heavy pick"),
    ("hm", "heavy mace"),
    ("rn", "ranseur"),
    ("hl", "halberd"),
    ("ha", "handaxe"),
    ("has", "handaxes"),
    ("tt", "tetsubo"),
    ("hf", "heavy flail"),
    ("lm", "light mace"),
    ("lsp", "longspear"),
    ("l", "lance"),
    ("w", "whip"),
    ("af", "alchemist's fire"),
    ("ts", "thunderstone"),
    ("tss", "thunderstones"),
    ("tb", "tanglefoot bag"),
    ("tbs", "tanglefoot bags"),
    ("sh", "shuriken"),
    ("a", "acid"),
    ("ta", "throwing axe"),
    ("tas", "throwing axes"),
    ("h", "horsechopper"),
    ("dg", "dogslicer"),
    ("dgs", "dogslicers"),
    ("rlc", "repeating light crossbow"),
    ("rhc", "repeating heavy crossbow"),
    ("ars", "arrows"),
    ("bolts", "bolts"),
];

fn decode_loot_code(code: &str) -> String {
    if code.is_empty() {
        return String::new();
    }
    if code.chars().all(|c| c.is_ascii_digit())
        || (code.starts_with(['+', '-']) && code[1..].chars().all(|c| c.is_ascii_digit()))
    {
        return code.to_string();
    }
    if let Some(rest) = code.strip_prefix("Mw") {
        return if rest.is_empty() {
            "mwk".to_string()
        } else {
            format!("mwk {}", decode_loot_code(rest))
        };
    }
    if let Some(rest) = code.strip_prefix('A')
        && let Some((_, name)) = ARMOR_CODES.iter().find(|(key, _)| *key == rest)
    {
        return (*name).to_string();
    }
    if let Some(rest) = code.strip_prefix('W') {
        if let Some((_, name)) = WEAPON_CODES.iter().find(|(key, _)| *key == rest) {
            return (*name).to_string();
        }
        for prefix in ["rlc", "rhc", "lc", "hc"] {
            if let Some(count) = rest.strip_prefix(prefix)
                && !count.is_empty()
                && count.chars().all(|c| c.is_ascii_digit())
                && let Some((_, name)) = WEAPON_CODES.iter().find(|(key, _)| *key == prefix)
            {
                return format!("{name} with {count} bolts");
            }
        }
        return rest.to_string();
    }
    if let Some(rest) = code.strip_prefix("WI") {
        if let Some(plus) = rest.strip_prefix("amu") {
            return format!("amulet of natural armor +{plus}");
        }
        if let Some(plus) = rest.strip_prefix("clk") {
            return format!("cloak of resistance +{plus}");
        }
        if let Some(plus) = rest.strip_prefix("boid") {
            return format!("belt of incredible dexterity +{plus}");
        }
        if let Some(plus) = rest.strip_prefix("bomc") {
            return format!("belt of mighty constitution +{plus}");
        }
        if let Some(plus) = rest.strip_prefix("bogs") {
            return format!("belt of giant strength +{plus}");
        }
        if let Some(plus) = rest.strip_prefix("bopp") {
            return format!("belt of physical perfection +{plus}");
        }
        return rest.replace('_', " ");
    }
    for (prefix, label) in [
        ("WD", "wand of "),
        ("AF", ""),
        ("P", "potion of "),
        ("S", "scroll of "),
        ("O", "oil of "),
        ("STF", "staff of "),
        ("ROD", "rod of "),
        ("Q", ""),
    ] {
        if let Some(rest) = code.strip_prefix(prefix)
            && !rest.is_empty()
        {
            return format!("{label}{rest}");
        }
    }
    if let Some(rest) = code.strip_prefix('R') {
        return format!("ring of {rest}");
    }
    if let Some(rest) = code.strip_prefix('E') {
        return match rest {
            "F" => "flaming".into(),
            "FB" => "flaming burst".into(),
            "K" => "keen".into(),
            "R" => "returning".into(),
            "S" => "speed".into(),
            "H" => "holy".into(),
            "U" => "unholy".into(),
            "X" => "axiomatic".into(),
            "SH" => "shock".into(),
            "SHB" => "shocking burst".into(),
            "G" => "ghost touch".into(),
            other => other.to_string(),
        };
    }
    code.to_string()
}

fn decode_loot_item(item: &str) -> String {
    item.split('|')
        .filter(|part| !part.is_empty())
        .map(decode_loot_code)
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_loot_list(value: &str) -> String {
    value
        .split('~')
        .filter(|item| !item.is_empty())
        .map(|item| {
            if let Some(literal) = item.strip_prefix('!') {
                convert_encoded_links(literal)
            } else {
                decode_loot_item(item)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_treasure(value: &str) -> String {
    let (kind, body) = value.split_once('=').unwrap_or((value, ""));
    let label = match kind {
        "S" => "Standard",
        "I" => "Incidental",
        "T" => "Triple",
        "D" => "Double",
        "N" => "NPC gear",
        "X" => "None",
        "!" => "Other",
        other => other,
    };
    if body.is_empty() {
        return label.to_string();
    }
    let mut items = Vec::new();
    let mut free = Vec::new();
    for part in body.split('~').filter(|part| !part.is_empty()) {
        if let Some(literal) = part.strip_prefix('!') {
            free.push(convert_encoded_links(literal));
        } else {
            items.push(decode_loot_item(part));
        }
    }
    let mut out = if items.is_empty() {
        label.to_string()
    } else {
        format!("{label} ({})", items.join(", "))
    };
    if !free.is_empty() {
        out.push_str(&format!("; {}", free.join(", ")));
    }
    out
}

fn render_meco(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("**{}**", convert_encoded_links(title)));
    } else {
        out.push_str("**Ecology**");
    }
    for (key, label) in [
        ("env", "Environment"),
        ("org", "Organization"),
        ("treasure", "Treasure"),
    ] {
        if let Some(value) = attr(attrs, key) {
            if key == "treasure" {
                push_labeled(&mut out, label, &render_treasure(value));
            } else {
                push_labeled(&mut out, label, &convert_encoded_links(value));
            }
        }
    }
    Ok(out)
}

fn render_moffense(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("**{}**", convert_encoded_links(title)));
    } else {
        out.push_str("**Offense**");
    }
    let mut speeds = Vec::new();
    for (key, prefix) in [
        ("sp", ""),
        ("br", "burrow "),
        ("cl", "climb "),
        ("fl", "fly "),
        ("sw", "swim "),
    ] {
        if let Some(value) = attr(attrs, key) {
            speeds.push(format!("{prefix}{value} ft."));
        }
    }
    if let Some(value) = attr(attrs, "fl") {
        let maneuverability = attr(attrs, "flP").or_else(|| {
            ["clumsy", "poor", "average", "good", "perfect"]
                .iter()
                .find(|key| attrs.contains_key(**key))
                .copied()
        });
        if let Some(maneuverability) = maneuverability {
            speeds.retain(|speed| !speed.starts_with("fly "));
            speeds.push(format!("fly {value} ft. ({maneuverability})"));
        }
    }
    if let Some(value) = attr(attrs, "spP") {
        speeds.push(format!("({value})"));
    }
    if let Some(value) = attr(attrs, "brP") {
        speeds.push(format!("burrow ({value})"));
    }
    if let Some(value) = attr(attrs, "clP") {
        speeds.push(format!("climb ({value})"));
    }
    if let Some(value) = attr(attrs, "swP") {
        speeds.push(format!("swim ({value})"));
    }
    if !speeds.is_empty() {
        push_labeled(&mut out, "Speed", &speeds.join(", "));
    }
    for (key, value) in attrs {
        if [
            "clear", "sp", "spP", "br", "brP", "cl", "clP", "fl", "flP", "sw", "swP", "clumsy",
            "poor", "average", "good", "perfect",
        ]
        .contains(&key.as_str())
        {
            continue;
        }
        let value = if value.is_empty() { "present" } else { value };
        push_labeled(
            &mut out,
            &field_label(key),
            &convert_encoded_links(&pretty_value(value)),
        );
    }
    Ok(out)
}

fn render_aff(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = if title.is_empty() {
        "**Affliction**".to_string()
    } else {
        format!("**{}**", convert_encoded_links(title))
    };
    let mut kind = if attrs.contains_key("poison") {
        "Poison"
    } else if attrs.contains_key("curse") {
        "Curse"
    } else if attrs.contains_key("infest") {
        "Infestation"
    } else if attrs.contains_key("disease") {
        "Disease"
    } else {
        "Affliction"
    }
    .to_string();
    if let Some(value) = attr(attrs, "type") {
        kind.push_str("; ");
        kind.push_str(value);
    }
    push_labeled(&mut out, "Type", &kind);
    let saving = attr(attrs, "save")
        .map(convert_encoded_links)
        .or_else(|| {
            attr(attrs, "saveF")
                .map(|dc| format!("Fort {dc}"))
                .or_else(|| attr(attrs, "saveW").map(|dc| format!("Will {dc}")))
                .or_else(|| render_aff_dc(attrs))
        })
        .unwrap_or_else(|| "none".to_string());
    push_labeled(&mut out, "Save DC", &saving);
    let frequency = attr(attrs, "freq").map(convert_encoded_links).or_else(|| {
        [
            ("freqR", "round"),
            ("freqM", "minute"),
            ("freqH", "hour"),
            ("freqD", "day"),
        ]
        .iter()
        .find_map(|(key, unit)| {
            attr(attrs, key).map(|amount| format!("1/{unit} for {amount} {unit}s"))
        })
    });
    if let Some(frequency) = frequency {
        push_labeled(
            &mut out,
            "Onset",
            &attr(attrs, "onset")
                .map(convert_encoded_links)
                .unwrap_or_else(|| "immediate".to_string()),
        );
        push_labeled(&mut out, "Frequency", &frequency);
        if let Some(track) = attr(attrs, "track") {
            push_labeled(&mut out, "Track", &convert_encoded_links(track));
        }
        if let Some(track) = attr(attrs, "trackmod") {
            push_labeled(&mut out, "Track", &convert_encoded_links(track));
        }
    }
    if let Some(effect) = attr(attrs, "eff") {
        push_labeled(&mut out, "Effect", &convert_encoded_links(effect));
    } else if let (Some(initial), Some(secondary)) = (attr(attrs, "ineff"), attr(attrs, "seceff")) {
        push_labeled(&mut out, "Initial Effect", &convert_encoded_links(initial));
        push_labeled(
            &mut out,
            "Secondary Effect",
            &convert_encoded_links(secondary),
        );
    } else if let Some(effect) = render_aff_effects(attrs) {
        push_labeled(&mut out, "Effect", &effect);
    }
    let cure = attr(attrs, "cure").map(convert_encoded_links).or_else(|| {
        [
            ("cure1", "1 save"),
            ("cure2", "2 saves"),
            ("cure2c", "2 consecutive saves"),
            ("cure3", "3 saves"),
            ("cure3c", "3 consecutive saves"),
        ]
        .iter()
        .find(|(key, _)| attrs.contains_key(*key))
        .map(|(_, label)| (*label).to_string())
    });
    if let Some(cure) = cure {
        push_labeled(&mut out, "Cure", &cure);
    }
    if let Some(extra) = attr(attrs, "extra") {
        push_labeled(&mut out, "Note", &convert_encoded_links(extra));
    }
    Ok(out)
}

fn render_aff_dc(attrs: &BTreeMap<String, String>) -> Option<String> {
    let kind = if attrs.contains_key("dcF") {
        "Fort"
    } else if attrs.contains_key("dcW") {
        "Will"
    } else if attrs.contains_key("dcR") {
        "Refl"
    } else {
        return None;
    };
    let level = attr(attrs, "dcLev").or_else(|| attr(attrs, "dcHD"))?;
    let unit = if attrs.contains_key("dcLev") {
        "level"
    } else {
        "HD"
    };
    Some(format!(
        "{kind} 10 + 1/2 {} {level} {unit} + {} modifier",
        if attrs.contains_key("dcYou") {
            "your"
        } else {
            "the"
        },
        attr(attrs, "dcMod").unwrap_or("ability")
    ))
}

fn render_aff_effects(attrs: &BTreeMap<String, String>) -> Option<String> {
    let mut effects = Vec::new();
    for (key, label) in [
        ("effStr", "Str damage"),
        ("effDex", "Dex damage"),
        ("effCon", "Con damage"),
        ("effInt", "Int damage"),
        ("effWis", "Wis damage"),
        ("effCha", "Cha damage"),
        ("effStrD", "Str drain"),
        ("effDexD", "Dex drain"),
        ("effConD", "Con drain"),
        ("effIntD", "Int drain"),
        ("effWisD", "Wis drain"),
        ("effChaD", "Cha drain"),
    ] {
        if let Some(value) = attr(attrs, key) {
            effects.push(format!("{value} {label}"));
        }
    }
    if let Some(extra) = attr(attrs, "effExtra") {
        effects.push(convert_encoded_links(extra));
    }
    if effects.is_empty() {
        None
    } else {
        Some(effects.join(", "))
    }
}

fn render_kinetics(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    let allowed = [
        "el", "type", "l", "burn", "btype", "dmg", "source", "prereq", "save", "assoc", "sr",
        "title", "icon", "clear", "jl",
    ];
    for key in attrs.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown attribute {key} on ::kinetics"));
        }
    }
    let mut out = String::new();
    let heading = if title.is_empty() {
        attrs.get("title").map(String::as_str).unwrap_or_default()
    } else {
        title
    };
    if !heading.is_empty() {
        out.push_str(&format!("**{}**", convert_encoded_links(heading)));
    }
    for (key, label) in [
        ("el", "Element"),
        ("type", "Type"),
        ("l", "Level"),
        ("burn", "Burn"),
        ("btype", "Damage type"),
        ("dmg", "Damage"),
        ("prereq", "Prerequisite"),
        ("assoc", "Associated"),
        ("save", "Saving throw"),
        ("sr", "Spell resistance"),
        ("source", "Source"),
    ] {
        let Some(value) = attrs.get(key).filter(|value| !value.is_empty()) else {
            continue;
        };
        let value = value.replace("~;", ", ");
        push_labeled(&mut out, label, &convert_encoded_links(&value));
    }
    Ok(out)
}

fn render_minfo(title: &str, attrs: &BTreeMap<String, String>) -> Result<String, String> {
    // `~` separates list items in subtype and sense fields. Other minfo fields
    // are single values, so an encoded separator there is still unsupported.
    let list_fields = ["subs", "othersubs", "augment", "sen", "senSpell"];
    for (key, value) in attrs {
        if value.contains('~') && !list_fields.contains(&key.as_str()) {
            return Err(format!("unsupported encoded {key} value on ::minfo"));
        }
        if value.contains('|') && !list_fields.contains(&key.as_str()) {
            return Err(format!("unsupported encoded {key} value on ::minfo"));
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
        let display = if list_fields.contains(&key.as_str()) {
            value
                .split(['~', '|'])
                .filter(|part| !part.is_empty())
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
        "constant", "atWill", "day", "hour", "week", "month", "year", "other", "psyMag", "l9",
        "l8", "l7", "l6", "l5", "l4", "l3", "l2", "l1", "l0",
    ] {
        let Some(value) = attrs.get(key) else {
            continue;
        };
        let label = match key {
            "constant" => "Constant",
            "atWill" => "At will",
            "day" => "Per day",
            "hour" => "Per hour",
            "week" => "Per week",
            "month" => "Per month",
            "year" => "Per year",
            "other" => "Other",
            "psyMag" => "Psychoactive powers",
            _ => key,
        };
        out.push_str(&format!("\n\n**{label}:**"));
        for group in value.split("~~").filter(|group| !group.is_empty()) {
            let mut parts = group.split('~').filter(|part| !part.is_empty());
            let Some(first) = parts.next() else { continue };
            let rest = parts.map(spell_entry).collect::<Vec<_>>().join(", ");
            if key == "other" && !rest.is_empty() {
                out.push_str(&format!("\n- {first}: {rest}"));
            } else if key == "day"
                || key.starts_with('l') && first.chars().all(|c| c.is_ascii_digit())
            {
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

/// Remove the upstream link and footnote marker tokens from a display title.
fn decode_source_markers(s: &str) -> String {
    s.replace("&L&", "").replace("&FN&", "").trim().to_string()
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
    let mut out = String::new();
    let mut errors = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('@') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let name_end = tail[1..]
            .char_indices()
            .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '-'))
            .map(|(index, _)| index + 1)
            .unwrap_or(tail.len());
        let token = &tail[..name_end];
        let after = &tail[name_end..];
        if let Some(bracketed) = after.strip_prefix('[') {
            let Some(close) = bracketed.find(']') else {
                errors.push(format!("unsupported or malformed inline directive {token}"));
                out.push_str(token);
                rest = after;
                continue;
            };
            let body = &bracketed[..close];
            let mut consumed_after = 1 + close + 1;
            let mut attrs = "";
            if let Some(attribute_text) = after[consumed_after..].strip_prefix('{')
                && let Some(attr_close) = attribute_text.find('}')
            {
                attrs = &attribute_text[..attr_close];
                consumed_after += attr_close + 2;
            }
            let consumed = name_end + consumed_after;
            let replacement = match token {
                "@HL" | "@hl" | "@strong" | "@b" => {
                    format!("<strong>{body}</strong>")
                }
                "@em" | "@i" => format!("<em>{body}</em>"),
                "@sup" => format!("<sup>{body}</sup>"),
                "@sub" => format!("<sub>{body}</sub>"),
                "@span" => body.to_string(),
                "@hll" | "@ripple" => {
                    if body.contains('/') {
                        convert_encoded_links(&format!("‹{body}›"))
                    } else {
                        body.to_string()
                    }
                }
                "@FN" => {
                    if attrs.is_empty() {
                        format!("<sup>{body}</sup>")
                    } else {
                        let marker = attrs
                            .split_whitespace()
                            .find_map(|part| part.strip_prefix("from="))
                            .unwrap_or("FN");
                        format!("<p><sup>{marker}</sup> {body}</p>")
                    }
                }
                token if token.starts_with("@B-") => match render_bonus_macro(token) {
                    Some(rendered) => rendered,
                    None => {
                        errors.push(format!("unsupported or malformed inline directive {token}"));
                        token.to_string()
                    }
                },
                _ => {
                    errors.push(format!("unsupported or malformed inline directive {token}"));
                    token.to_string()
                }
            };
            out.push_str(&replacement);
            rest = &tail[consumed..];
        } else {
            let replacement = match token {
                "@altCapstoneDesc" => CAPSTONE_NOTE.to_string(),
                token if token.starts_with("@B-") => match render_bonus_macro(token) {
                    Some(rendered) => rendered,
                    None => {
                        errors.push(format!("unsupported or malformed inline directive {token}"));
                        token.to_string()
                    }
                },
                token if token.starts_with("@HL") => {
                    let text = token[3..].replace('_', " ");
                    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    if text.is_empty() {
                        errors.push(format!("unsupported or malformed inline directive {token}"));
                        token.to_string()
                    } else {
                        format!("<strong>{text}</strong>")
                    }
                }
                token if token.starts_with("@FN") => {
                    let marker = &token[3..];
                    if marker.is_empty()
                        || !marker
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-')
                    {
                        errors.push(format!("unsupported or malformed inline directive {token}"));
                        token.to_string()
                    } else {
                        format!("<sup>{marker}</sup>")
                    }
                }
                _ => {
                    errors.push(format!("unsupported or malformed inline directive {token}"));
                    token.to_string()
                }
            };
            out.push_str(&replacement);
            rest = after;
        }
    }
    out.push_str(rest);
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
    use crate::text::blocks_text;

    #[test]
    fn duplicate_presentation_metadata_is_collapsed_but_content_is_not() {
        let attrs = attributes("::ab{icon=power icon=power order=sw order=uswa l=1 l=1}")
            .expect("repeated presentation metadata");
        assert_eq!(attrs["icon"], "power");
        assert_eq!(attrs["order"], "uswa");
        assert_eq!(attrs["l"], "1");
        assert!(
            attributes("::ab{ability=one ability=two}").is_err(),
            "two competing content values must still fail"
        );
        // `::spellAb{caster=shaman ... type=divine type=Wisdom}` keeps the first type.
        assert_eq!(
            attributes("::spellAb{type=divine type=Wisdom}")
                .unwrap()
                .get("type")
                .map(String::as_str),
            Some("divine")
        );
    }

    #[test]
    fn class_spellcasting_directives_render_rules_text() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                "::spellAb{stat=Intelligence type=arcane caster=wizard learn prepare ritualArcane zeroMsg}".into(),
                "::cantrips{p}".into(),
            ],
            &mut env,
        )
        .unwrap();
        let rendered = format!("{blocks:?}");
        for value in [
            "Spells",
            "Intelligence",
            "arcane",
            "wizard list",
            "Cantrips",
            "Prepared",
        ] {
            assert!(rendered.contains(value), "missing {value}: {rendered}");
        }
        // Upstream renderer prose (GPL, prebuild/block/spellAb.js) must not appear.
        for copied in [
            "You can cast only a certain number of spells of each spell level per day",
            "You learn a number of",
        ] {
            assert!(!rendered.contains(copied), "copied renderer text: {copied}");
        }
    }

    #[test]
    fn generated_bonus_macro_expands_to_readable_prose() {
        assert_eq!(
            render_bonus_macro("@B-3y-ninja-x").as_deref(),
            Some("This bonus is equal to one-third of your ninja level")
        );
        assert_eq!(
            render_bonus_macro("@B-morale_bonus-4m1-cavalier").as_deref(),
            Some("This morale bonus is equal to 1 + one-fourth of his cavalier level")
        );
        let mut env = NullEnv;
        let blocks = render(&["A bonus on checks. @B-5yp-cavalier-x.".into()], &mut env).unwrap();
        let rendered = format!("{blocks:?}");
        assert!(rendered.contains("These bonuses are equal to one-fifth of your cavalier level"));
    }

    #[test]
    fn source_extension_attributes_on_ability_render_their_values() {
        let mut env = NullEnv;
        let blocks = render(
            &["::ab[Resolute (Ex)]{xPossible_Mounts=\"camel, horse\" yResolute=+2}".into()],
            &mut env,
        )
        .unwrap();
        let rendered = format!("{blocks:?}");
        assert!(rendered.contains("Possible Mounts"));
        assert!(rendered.contains("camel, horse"));
        assert!(rendered.contains("Resolute"));
        assert!(rendered.contains("+2"));
    }

    #[test]
    fn optional_rules_container_keeps_its_contents() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                ":::elephant".into(),
                "‹SOURCE The Elephant in the Room›  ".into(),
                "An optional rule.".into(),
                ":::".into(),
            ],
            &mut env,
        )
        .unwrap();
        let rendered = format!("{blocks:?}");
        assert!(rendered.contains("An optional rule"));
        assert!(rendered.contains("The Elephant in the Room"));
    }

    #[test]
    fn list_directive_keeps_visible_names_and_drops_link_markers() {
        let mut env = NullEnv;
        let blocks = render(
            &["::list[Companions]{link=companion end=\".\" all=\"Antelope~Bird« (Eagle)~Swan, »Trumpeter<_swan\"}".into()],
            &mut env,
        )
        .unwrap();
        let rendered = format!("{blocks:?}");
        for value in ["Companions", "Antelope", "Bird (Eagle)", "Trumpeter"] {
            assert!(rendered.contains(value), "missing {value}: {rendered}");
        }
        assert!(!rendered.contains('«') && !rendered.contains('»'));
    }

    #[test]
    fn prerequisite_directive_renders_levels_races_and_other_requirements() {
        let mut env = NullEnv;
        let blocks = render(
            &["::prereq{l=3 c=wizard r=Elf other=\"Must prepare spells.\"}".into()],
            &mut env,
        )
        .unwrap();
        let rendered = format!("{blocks:?}");
        for value in [
            "Prerequisites",
            "3rd level wizard",
            "Elf",
            "Must prepare spells",
        ] {
            assert!(rendered.contains(value), "missing {value}: {rendered}");
        }
    }

    #[test]
    fn explicit_prof_shields_value_is_not_overwritten() {
        // An explicit `shields="bucklers only"` must survive; only a bare `shields`
        // flag falls back to the tower-shield wording.
        let explicit = render_prof_directive("{shields=\"bucklers only\"}").unwrap();
        assert!(explicit.contains("bucklers only"), "{explicit}");
        let bare = render_prof_directive("{shields}").unwrap();
        assert!(bare.contains("except tower shields"), "{bare}");
    }

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
        let encoded = render(
            &[r#"::mdefense{ac="37/7/37" hp="372~24d10+240" def="ichor~never surprised or flat-footed" weak="‹umr/light sensitivity›~revulsion"}"#.into()],
            &mut env,
        )
        .unwrap();
        let encoded = blocks_text(&encoded);
        assert!(
            encoded.contains("Defensive abilities: ichor, never surprised or flat-footed"),
            "{encoded}"
        );
        assert!(
            encoded.contains("Weaknesses: light sensitivity, revulsion"),
            "{encoded}"
        );
        let offense = render(&["::moffense{sp=30 spP=\"20 ft. in armor\" melee=\"heavy mace -1 (1d8-1)\" chPEn=\"5/day, 1d6, DC 12\"}".into()], &mut env).unwrap();
        assert!(format!("{offense:?}").contains("DC 12"));
        let skills = render(
            &["::mstats{skills=\"diplo|8~heal|7~k|r|5\" lang=Ce~C~D}".into()],
            &mut env,
        )
        .unwrap();
        let skills = format!("{skills:?}");
        for value in [
            "Diplomacy +8",
            "Heal +7",
            "Knowledge (religion) +5",
            "Languages",
            "Celestial",
        ] {
            assert!(skills.contains(value), "missing {value}: {skills}");
        }
        let treasure = render(
            &["::meco{env=\"any land\" treasure=\"N=Asm~Whm~Wlc10\"}".into()],
            &mut env,
        )
        .unwrap();
        let treasure = format!("{treasure:?}");
        for value in ["NPC gear", "scale mail", "heavy mace", "light crossbow"] {
            assert!(treasure.contains(value), "missing {value}: {treasure}");
        }
        let flags = render(&["::minfo{lg large humanoid}".into()], &mut env).unwrap();
        let flags = format!("{flags:?}");
        assert!(
            flags.contains("Lawful good") && flags.contains("large") && flags.contains("humanoid")
        );
        let usage = render(
            &[":::ab{title=Feature useInc=shaman~4~10}".into()],
            &mut env,
        )
        .unwrap();
        let usage = format!("{usage:?}");
        assert!(usage.contains("Usage"), "{usage}");
        assert!(usage.contains("shaman"), "{usage}");
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

    #[test]
    fn ability_metadata_is_rendered_or_counted() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                "::ab[Feature]{jl=strip increment=\"The bonus~2~3~4~2\" incrementMax=9 incrementDesc=a incrementEnd=\" cruelty.\" provokes=No replace=\"A~B\" order=pa useInc=antipaladin~3~1}".into(),
                "::h3[Section]{jl}".into(),
            ],
            &mut env,
        )
        .unwrap();
        let text = format!("{blocks:?}");
        for value in [
            "Replaces",
            "A, B",
            "Provokes attack of opportunity",
            "At 5th level",
            "Usage",
            "antipaladin",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
        assert!(!text.contains('~'), "unencoded metadata leaked: {text}");
    }

    #[test]
    fn class_proficiency_capstone_and_archetype_directives_render() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                "::class{any hd=8 wealth=3 app craft dd prof skp=4}".into(),
                "::prof{simple weaps=!Bombs light shields}".into(),
                ":::aside".into(),
                ":::".into(),
                "::hl3[rule/Favored Class« Bonuses]{jl}".into(),
                "::archetypeDesc".into(),
                "::altCapstoneDesc".into(),
                "::archetype[Test Archetype]{c=alchemist r=\"Bomb\" e=\"A description.\"}".into(),
                "::altCapstone[Test Capstone]{capstone=\"A benefit.\" greatBeast}".into(),
                "::classoptions{cls=alchemist}".into(),
            ],
            &mut env,
        )
        .unwrap();
        let text = format!("{blocks:?}");
        for value in [
            "Hit Die",
            "d8",
            "Class Skills (Int)",
            "All simple weapons",
            "Bombs",
            "Light armor",
            "Favored Class Bonuses",
            "From the noble paladin",
            "When a character reaches the 20th level",
            "Test Archetype",
            "Modifies or Replaces",
            "A description",
            "Test Capstone",
            "A benefit",
            "Great Beast",
            "class options page",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
    }

    #[test]
    fn tabular_formats_render_typed_tables() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                "::tabular{format=alchstats a=\"10 gp\" b=\"1 lb.\" c=weapon d=Acid}".into(),
                "::tabular{format=powercomp a=\"1 (10 gp)\" b=\"acid arrow\" c=\"Uses are listed below.\"}".into(),
                ">::tabular{format=reqcost a=\"‹spell/symbol of pain›\" b=\"18,000 gp\"}".into(),
            ],
            &mut env,
        )
        .unwrap();
        assert_eq!(
            blocks
                .iter()
                .filter(|block| matches!(block, Block::Table(_)))
                .count(),
            3
        );
        let text = format!("{blocks:?}");
        for value in [
            "Price",
            "Weight",
            "Acid",
            "Doses",
            "Requirements",
            "18,000 gp",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
    }

    #[test]
    fn inline_footnotes_links_and_action_tokens_are_typed() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                "Text @FN1-2 @hll[spell/Acid Arrow] @HLswift_action @I-boost end.".into(),
                ":::fakeFootnotes".into(),
                "@FN[Source text]{from=1 multi=3}".into(),
                ":::".into(),
            ],
            &mut env,
        )
        .unwrap();
        let text = format!("{blocks:?}");
        for value in ["1-2", "Acid Arrow", "swift action", "Source text"] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
        assert!(!text.contains("@"), "inline directive leaked: {text}");
    }

    #[test]
    fn monster_encodings_render_feats_skills_languages_treasure_month_fly_and_affliction() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                "::mstats{feats=\"Combat Reflexes~Skill Focus|Perception~Hover#B~M/Flyby Attack\" skills=\"acro|21~k|r|5~craft|alchemy|7\" lang=\"Ak~C~D~U~~‹umr/telepathy› 100 ft.\"}".into(),
                "::meco{treasure=\"N=Asm~Whm~Wlc10~!other treasure\"}".into(),
                "::mspell{sla cl=13 month=\"interplanetary teleport\"}".into(),
                "::moffense{fl=60 poor}".into(),
                "::aff[Poison (Ex)]{iconP type=Sting-injury saveF=19 freqR=6 effCon=\"1d4\" cure2c}".into(),
            ],
            &mut env,
        )
        .unwrap();
        let text = format!("{blocks:?}");
        for value in [
            "Mythic Flyby Attack",
            "Skill Focus (Perception)",
            "Hover",
            "Acrobatics +21",
            "Knowledge (religion) +5",
            "Craft (alchemy) +7",
            "Undercommon",
            "telepathy",
            "NPC gear",
            "scale mail",
            "heavy mace",
            "light crossbow with 10 bolts",
            "other treasure",
            "Per month",
            "interplanetary teleport",
            "fly 60 ft. (poor)",
            "Poison (Sting-injury)",
            "Fortitude DC 19",
            "1/round for 6 rounds",
            "Constitution damage 1d4",
            "2 consecutive saves",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
    }

    #[test]
    fn untitled_stat_directive_does_not_emit_a_thematic_break() {
        #[derive(Default)]
        struct Recorder {
            stripped: Vec<String>,
        }
        impl crate::html::Env for Recorder {
            fn link(&mut self, _href: &str) -> Option<(cast_iron_schema::EntryId, Option<String>)> {
                None
            }
            fn stripped(&mut self, key: &str) {
                self.stripped.push(key.to_string());
            }
        }
        let mut env = Recorder::default();
        // `::mdefense` has no bracketed title. Before the fix, the empty bold
        // heading produced `****`, which Markdown parsed as an `<hr>`.
        let blocks = render(&[r#"::mdefense{ac=15 hp="11~1d8+3"}"#.into()], &mut env).unwrap();
        assert!(
            !env.stripped.iter().any(|key| key == "hr"),
            "{:?}",
            env.stripped
        );
        let text = blocks
            .iter()
            .filter_map(|block| match block {
                Block::Paragraph { content, .. } => Some(inline_text(content)),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("Armor class: 15"), "{text}");
        assert!(text.contains("Hit points: 11 (1d8+3)"), "{text}");
        assert!(!text.contains("****"), "{text}");
    }

    #[test]
    fn container_directives_keep_their_headings_and_mark_layout_only_wrappers() {
        #[derive(Default)]
        struct Recorder {
            stripped: Vec<String>,
        }
        impl crate::html::Env for Recorder {
            fn link(&mut self, _href: &str) -> Option<(cast_iron_schema::EntryId, Option<String>)> {
                None
            }
            fn stripped(&mut self, key: &str) {
                self.stripped.push(key.to_string());
            }
        }
        let mut env = Recorder::default();
        let blocks = render(
            &[
                ":::block{size=small title=\"&L&Acid Trap (Ex or Su)&FN&\"}".into(),
                "".into(),
                "Trap body.".into(),
                "".into(),
                ":::".into(),
                "".into(),
                ":::group".into(),
                "".into(),
                "Grouped body.".into(),
                "".into(),
                ":::".into(),
                "".into(),
                ":::stamina".into(),
                "".into(),
                "Stamina trick body.".into(),
                "".into(),
                ":::".into(),
                "".into(),
                ":::mythic{jl title=\"Mythic Wish\"}".into(),
                "".into(),
                "Mythic body.".into(),
                "".into(),
                ":::".into(),
            ],
            &mut env,
        )
        .unwrap();
        let text = blocks_text(&blocks);
        for value in [
            "Acid Trap (Ex or Su)",
            "Trap body.",
            "Grouped body.",
            "Stamina",
            "Stamina trick body.",
            "Mythic Wish",
            "Mythic body.",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
        assert!(!text.contains("&L&"), "{text}");
        assert!(!text.contains("&FN&"), "{text}");
        for key in ["block_container", "group_container", "heading_jump_list"] {
            assert!(
                env.stripped.iter().any(|found| found == key),
                "{:?}",
                env.stripped
            );
        }
    }

    #[test]
    fn kinetic_talent_and_encoded_minfo_lists_render_as_labeled_fields() {
        let mut env = NullEnv;
        let blocks = render(
            &[
                r#"::kinetics{el=air type="simple blast (Sp)" l=0 burn=0 btype=physical dmg=bludgeoning source="Occult Adventures/15"}"#.into(),
                r#"::minfo{source="Bestiary 5/197" xp="4,800" n small outsider subs=extraplanar~incorporeal init=+8 thoughtsense=60 dv=60 pcp=+18}"#.into(),
                r#"::minfo{source="Bestiary 2/221" xp=600 ce tiny outsider subs=chaotic~evil~extraplanar~qlippoth init=+1 senSpell="detect law~detect magic" dv=60 pcp=+5}"#.into(),
            ],
            &mut env,
        )
        .unwrap();
        let text = blocks_text(&blocks);
        for value in [
            "Element: air",
            "Type: simple blast (Sp)",
            "Level: 0",
            "Burn: 0",
            "Damage type: physical",
            "Damage: bludgeoning",
            "Source: Occult Adventures/15",
            "Subtype: Extraplanar, Incorporeal",
            "Senses: Detect law, Detect magic",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
    }

    #[test]
    fn markdown_footnote_definitions_become_anchored_headings() {
        #[derive(Default)]
        struct Recorder {
            stripped: Vec<String>,
        }
        impl crate::html::Env for Recorder {
            fn link(&mut self, _href: &str) -> Option<(cast_iron_schema::EntryId, Option<String>)> {
                None
            }
            fn stripped(&mut self, key: &str) {
                self.stripped.push(key.to_string());
            }
        }
        let lines = [
            "A domain spell[^D] and a note[^1].".into(),
            "".into(),
            "[^D]: Domain spell".into(),
            "[^1]: A footnote note.".into(),
        ];
        let anchors = anchors(&lines);
        assert_eq!(anchors.get("D").map(String::as_str), Some("footnote-d"));
        assert_eq!(anchors.get("1").map(String::as_str), Some("footnote-1"));
        let mut env = Recorder::default();
        let blocks = render(&lines, &mut env).unwrap();
        let headings: Vec<_> = blocks
            .iter()
            .filter_map(|block| match block {
                Block::Heading { anchor, .. } => anchor.as_deref(),
                _ => None,
            })
            .collect();
        assert_eq!(headings, ["footnote-d", "footnote-1"]);
        let text = blocks_text(&blocks);
        assert!(text.contains("Domain spell"), "{text}");
        assert!(text.contains("A footnote note."), "{text}");
        assert!(
            !env.stripped
                .iter()
                .any(|key| key.starts_with("unwrapped_tag:") || key.starts_with("flattened_tag:")),
            "{:?}",
            env.stripped
        );
    }

    #[test]
    fn racial_traits_special_tokens_are_decoded() {
        let mut env = NullEnv;
        let blocks = render(
            &["::rtraits{plural=Aasimars main=\"+2 Wisdom|A description.~NATOUT~MEDIUM~SPEED|normal~DV~LANG|Common|Celestial||Draconic|Elven\"}".into()],
            &mut env,
        )
        .unwrap();
        let text = format!("{blocks:?}");
        for value in [
            "Native Outsider",
            "Medium",
            "Normal Speed",
            "Darkvision",
            "Languages",
            "begin play speaking Common and Celestial",
            "bonus languages: Draconic and Elven",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
    }
}
