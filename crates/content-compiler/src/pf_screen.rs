//! Product Identity screen for Pf Data records, applied to source lines before conversion.
//!
//! Policy (docs/decisions.md, "Product Identity screen for the Pf Data corpus"): proper
//! names inside game mechanics stay, as descriptive references under Paizo's Community
//! Use Policy. Descriptive Product Identity prose, such as deity lore, unique-character
//! biographies, and named NPC stories, is omitted by reviewed section rules. Outbound
//! links are removed because the reference must stay self-contained. Every omission is
//! returned to the caller for the coverage report; nothing is dropped silently.

use serde::Deserialize;
use std::path::Path;

/// Coverage category for an omitted Product Identity section.
pub const SECTION_CATEGORY: &str = "product-identity-section";
/// Coverage category for a removed outbound link.
pub const EXTERNAL_LINK_CATEGORY: &str = "external-link";
const EXTERNAL_LINK_REASON: &str = "Outbound link removed: the offline reference does not link to third-party sites; the link text is kept";
const EXTERNAL_LINE_REASON: &str =
    "Outbound reference line removed: it only pointed to a third-party site";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PiScreen {
    #[serde(default)]
    note: String,
    #[serde(default)]
    sections: Vec<SectionRule>,
}

/// Omits a section, from its heading to the next heading of the same or a higher
/// level, in every record whose path starts with `files`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionRule {
    files: String,
    heading: String,
    /// Match every heading that starts with `heading` instead of the exact line.
    #[serde(default)]
    prefix: bool,
    reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Omission {
    pub category: &'static str,
    pub detail: String,
    pub reason: String,
}

impl PiScreen {
    /// Loads the reviewed screen. A missing file is an error: the screen is a
    /// publication gate, not an optional refinement.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let screen: Self =
            serde_json::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
        screen.validate()?;
        Ok(screen)
    }

    fn validate(&self) -> Result<(), String> {
        for rule in &self.sections {
            if !rule.files.starts_with("json/") || rule.reason.trim().is_empty() {
                return Err(format!(
                    "PI screen rule {} {} needs a json/ path prefix and a reason",
                    rule.files, rule.heading
                ));
            }
            if heading_level(&rule.heading).is_none() {
                return Err(format!(
                    "PI screen rule {} names `{}`, which is not a heading",
                    rule.files, rule.heading
                ));
            }
        }
        Ok(())
    }

    pub fn note(&self) -> &str {
        &self.note
    }

    /// Returns the screened lines and every omission, in source order.
    pub fn apply(&self, path: &str, lines: &[String]) -> (Vec<String>, Vec<Omission>) {
        let rules: Vec<&SectionRule> = self
            .sections
            .iter()
            .filter(|rule| path.starts_with(&rule.files))
            .collect();
        let mut kept = Vec::with_capacity(lines.len());
        let mut omissions = Vec::new();
        // Level of the section being omitted, if any.
        let mut omitting: Option<usize> = None;
        for line in lines {
            let level = heading_level(line);
            if let Some(open) = omitting {
                let closes = line.trim() == ":::" || level.is_some_and(|l| l <= open);
                if !closes {
                    continue;
                }
                omitting = None;
            }
            if let Some(level) = level
                && let Some(rule) = rules.iter().find(|rule| rule.matches(line))
            {
                omitting = Some(level);
                omissions.push(Omission {
                    category: SECTION_CATEGORY,
                    detail: line.trim().to_string(),
                    reason: rule.reason.clone(),
                });
                continue;
            }
            match strip_external_links(line) {
                Some((text, removed)) if removed > 0 => {
                    if is_only_label(&text) {
                        omissions.push(Omission {
                            category: EXTERNAL_LINK_CATEGORY,
                            detail: line.trim().to_string(),
                            reason: EXTERNAL_LINE_REASON.into(),
                        });
                    } else {
                        omissions.push(Omission {
                            category: EXTERNAL_LINK_CATEGORY,
                            detail: line.trim().to_string(),
                            reason: EXTERNAL_LINK_REASON.into(),
                        });
                        kept.push(text);
                    }
                }
                _ => kept.push(line.clone()),
            }
        }
        (kept, omissions)
    }
}

impl SectionRule {
    fn matches(&self, line: &str) -> bool {
        let line = line.trim();
        if self.prefix {
            line.starts_with(&self.heading)
        } else {
            line == self.heading || line.starts_with(&format!("{}{{", self.heading))
        }
    }
}

/// Section level of a Markdown or upstream heading line, or `None` for body text.
/// `::fh` is the record title, `::h2`/`::hl2` and `##` are level 2, `::sh` and
/// `###` are level 3, and so on.
fn heading_level(line: &str) -> Option<usize> {
    let line = line.trim_start();
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        return Some(hashes);
    }
    let rest = line.strip_prefix("::")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if !rest[name.len()..].starts_with('[') {
        return None;
    }
    match name.as_str() {
        "fh" => Some(1),
        "sh" => Some(3),
        _ => {
            let digits = name.strip_prefix("hl").or_else(|| name.strip_prefix('h'))?;
            digits.parse().ok().filter(|level| (1..=6).contains(level))
        }
    }
}

/// Replaces each `[text](http…)` with `text`, then removes any bare outbound
/// address that survived as text. Returns the new line and the number of
/// removals, or `None` when the line has none.
///
/// A bare address occurs in upstream source annotations such as
/// `‹‹From the FAQ/https://…››`, which is not a Markdown link. Keeping the
/// address would make the reference point at a third-party site even though the
/// schema has no outbound-link node.
fn strip_external_links(line: &str) -> Option<(String, usize)> {
    let (mut text, mut removed) = strip_markdown_links(line);
    let (bare_text, bare) = strip_bare_outbound(&text);
    text = bare_text;
    removed += bare;
    (removed > 0).then_some((text, removed))
}

fn strip_markdown_links(line: &str) -> (String, usize) {
    let mut out = String::with_capacity(line.len());
    let mut removed = 0;
    let mut rest = line;
    while let Some(close) = rest.find("](http") {
        let Some(open) = rest[..close].rfind('[') else {
            break;
        };
        let after = &rest[close + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..close]);
        rest = &after[end + 1..];
        removed += 1;
    }
    out.push_str(rest);
    (out, removed)
}

/// Removes `http://…`, `https://…`, and the bare word `pathfinderwiki`.
fn strip_bare_outbound(text: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut removed = 0;
    let mut rest = text;
    loop {
        let lower = rest.to_ascii_lowercase();
        let hit = ["https://", "http://", "pathfinderwiki"]
            .iter()
            .filter_map(|needle| lower.find(needle).map(|index| (index, *needle)))
            .min_by_key(|(index, _)| *index);
        let Some((index, needle)) = hit else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..index]);
        let after = &rest[index + needle.len()..];
        if needle == "pathfinderwiki" {
            rest = after;
        } else {
            let end = after
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, ')' | '›' | ']' | '"' | '<' | '>' | ',' | ';')
                })
                .unwrap_or(after.len());
            rest = &after[end..];
        }
        removed += 1;
    }
    (out, removed)
}

/// A line such as `**Pathfinder Wiki** Abadar` that only labelled the removed link.
fn is_only_label(text: &str) -> bool {
    let text = text.trim();
    text.starts_with("**Pathfinder Wiki**") || text.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> PiScreen {
        serde_json::from_str(
            r####"{"sections":[
                {"files":"json/faiths","heading":"::h2[Description]","reason":"deity lore"},
                {"files":"json/npcs","heading":"### ","prefix":true,"reason":"named story"}
            ]}"####,
        )
        .unwrap()
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(String::from).collect()
    }

    #[test]
    fn omits_a_reviewed_section_until_the_next_peer_heading() {
        let input = lines(
            "::fh[Abadar]\n### Details\n**Domains** Law\n::h2[Description]{jl}\nLore one.\n### Inner\nLore two.\n::h2[Follower Options]{jl}\nMechanics.",
        );
        let (kept, omitted) = screen().apply("json/faiths2.json", &input);
        assert_eq!(
            kept,
            lines(
                "::fh[Abadar]\n### Details\n**Domains** Law\n::h2[Follower Options]{jl}\nMechanics."
            )
        );
        assert_eq!(omitted.len(), 1);
        assert_eq!(omitted[0].category, SECTION_CATEGORY);
        assert_eq!(omitted[0].detail, "::h2[Description]{jl}");
    }

    #[test]
    fn a_rule_applies_only_to_its_files() {
        let input = lines("::h2[Description]\nGeneric text.");
        let (kept, omitted) = screen().apply("json/feat.json", &input);
        assert_eq!(kept, input);
        assert!(omitted.is_empty());
    }

    #[test]
    fn a_named_story_ends_at_the_next_subheading_or_container_end() {
        let input = lines(
            "::sh[Description]\nGeneric rake.\n### Tristram Reis\nStory.\n::sh[Tactics]\nAttack.\n### Other Name\nStory.\n:::\nAfter.",
        );
        let (kept, omitted) = screen().apply("json/npcs3.json", &input);
        assert_eq!(
            kept,
            lines("::sh[Description]\nGeneric rake.\n::sh[Tactics]\nAttack.\n:::\nAfter.")
        );
        assert_eq!(omitted.len(), 2);
    }

    #[test]
    fn outbound_links_keep_their_text_and_wiki_lines_go() {
        let input = lines(
            "**Pathfinder Wiki** [Abadar](https://www.pathfinderwiki.com/wiki/Abadar)\nSee [the map](http://example.com/x) and ‹spell/wish›.",
        );
        let (kept, omitted) = screen().apply("json/faiths.json", &input);
        assert_eq!(kept, lines("See the map and ‹spell/wish›."));
        assert_eq!(omitted.len(), 2);
        assert!(omitted.iter().all(|o| o.category == EXTERNAL_LINK_CATEGORY));
    }

    #[test]
    fn a_bare_address_in_a_source_annotation_is_removed() {
        let input = lines(
            "::ab[Wild Shape]{special=\"‹‹From the FAQ/https://paizo.com/paizo/faq/x››: The lack of stat blocks.\"}",
        );
        let (kept, omitted) = screen().apply("json/archetypes_druid.json", &input);
        assert_eq!(
            kept,
            lines("::ab[Wild Shape]{special=\"‹‹From the FAQ/››: The lack of stat blocks.\"}")
        );
        assert_eq!(omitted.len(), 1);
        assert!(kept[0].find("https://").is_none());
    }

    #[test]
    fn heading_levels_cover_upstream_and_markdown_forms() {
        assert_eq!(heading_level("::fh[X]"), Some(1));
        assert_eq!(heading_level("::hl2[rule/Divine Gift<s]{jl}"), Some(2));
        assert_eq!(heading_level("::h3[Shark Mount]"), Some(3));
        assert_eq!(heading_level("::sh[Tactics]"), Some(3));
        assert_eq!(heading_level("### Details"), Some(3));
        assert_eq!(heading_level("::mstats{str=1}"), None);
        assert_eq!(heading_level("#O"), None);
    }

    #[test]
    fn a_rule_without_a_reason_or_heading_fails_validation() {
        let bad: PiScreen = serde_json::from_str(
            r#"{"sections":[{"files":"json/faiths","heading":"Description","reason":"x"}]}"#,
        )
        .unwrap();
        assert!(bad.validate().is_err());
        let bad: PiScreen = serde_json::from_str(
            r#"{"sections":[{"files":"json/faiths","heading":"::h2[D]","reason":" "}]}"#,
        )
        .unwrap();
        assert!(bad.validate().is_err());
    }
}
