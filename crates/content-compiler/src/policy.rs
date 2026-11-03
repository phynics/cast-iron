//! Reviewed omissions applied before planning, including copies nested in rules.

use crate::model::Node;
use cast_iron_schema::Rejection;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPolicy {
    #[serde(default)]
    excluded_urls: BTreeMap<String, String>,
    #[serde(default)]
    omitted_bodies: BTreeMap<String, String>,
    #[serde(default)]
    omit_npc_stories: bool,
    #[serde(default)]
    omit_race_names: bool,
}

impl ContentPolicy {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
    }

    /// Returns false for an excluded whole node. Every omission is reported.
    pub fn retain(&self, node: &mut Node, path: &str, rejected: &mut Vec<Rejection>) -> bool {
        self.walk(node, path, false, false, rejected)
    }

    fn walk(
        &self,
        node: &mut Node,
        path: &str,
        npc: bool,
        race: bool,
        rejected: &mut Vec<Rejection>,
    ) -> bool {
        let location = node.url.as_deref().unwrap_or(path);
        let reason = self.excluded_urls.get(location).cloned().or_else(|| {
            let name = node.title();
            if self.omit_npc_stories && npc && !name.is_empty() && !name.eq_ignore_ascii_case("tactics") {
                Some("Product Identity: named NPC story omitted; generic stat block and tactics retained".into())
            } else if self.omit_race_names && race && name.to_ascii_lowercase().ends_with("names") {
                Some("Product Identity: suggested character names omitted".into())
            } else {
                None
            }
        });
        if let Some(reason) = reason {
            rejected.push(Rejection {
                path: format!("{path}#{location}"),
                reason,
            });
            return false;
        }
        if let Some(reason) = self.omitted_bodies.get(location)
            && node.body.take().is_some()
        {
            rejected.push(Rejection {
                path: format!("{path}#{location}:body"),
                reason: reason.clone(),
            });
        }
        let npc =
            node.kind.as_deref() == Some("creature") && node.source.as_deref() == Some("NPC Codex");
        let race = race || node.kind.as_deref() == Some("race");
        let mut kept = Vec::new();
        for (index, mut child) in std::mem::take(&mut node.sections).into_iter().enumerate() {
            if self.walk(
                &mut child,
                &format!("{path}/sections/{index}"),
                npc,
                race,
                rejected,
            ) {
                kept.push(child);
            }
        }
        node.sections = kept;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_npc_mechanics_and_reports_the_named_story() {
        let policy: ContentPolicy = toml::from_str("omit_npc_stories = true").unwrap();
        let mut node: Node = serde_json::from_str(r#"{"name":"Guard","type":"creature","source":"NPC Codex","sections":[{"name":"Tactics","body":"<p>Attack</p>"},{"name":"Named Person","body":"<p>Story</p>"}]}"#).unwrap();
        let mut report = Vec::new();
        assert!(policy.retain(&mut node, "guard.json", &mut report));
        assert_eq!(node.sections.len(), 1);
        assert_eq!(node.sections[0].title(), "Tactics");
        assert_eq!(report.len(), 1);
        assert!(report[0].reason.contains("named NPC"));
    }
}
