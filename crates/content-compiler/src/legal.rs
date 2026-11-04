//! Legal notices and the Section 15 lookup.

use crate::config::DatasetConfig;
use cast_iron_schema::{CopyrightNotice, DatasetNotice, LegalNotices, LicenseText};
use serde::Deserialize;

const OGL_TEXT: &str = include_str!("../../../LICENSES/OGL-1.0a.txt");
const LICENSING_AUDIT: &str = include_str!("../../../docs/research/licensing-audit.md");
pub const PRODUCT_NAME: &str = "Cast Iron";
/// The Community Use notice section of the legal page. CUP records record this path.
pub const COMMUNITY_USE_NOTICE_PATH: &str = "legal#community-use";
const PLACEHOLDER: &str = "[This website, product name, Twitch show title, etc.]";

#[derive(Debug, Deserialize)]
pub struct ExtractedNotices {
    pub product_identity: String,
    pub section_15: Vec<NoticeEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NoticeEntry {
    pub title: String,
    pub notice: String,
}

impl ExtractedNotices {
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid legal-notices.json: {e}"))
    }
}

/// The Community Use Policy notice from the licensing audit, with the product name filled in.
pub fn community_use_notice(audit: &str, product: &str) -> Result<String, String> {
    let start = audit
        .find(PLACEHOLDER)
        .ok_or("the licensing audit holds no Community Use notice template")?;
    let tail = &audit[start..];
    let end_marker = "visit paizo.com.";
    let end = tail
        .find(end_marker)
        .ok_or("the licensing audit notice template has no end")?
        + end_marker.len();
    Ok(tail[..end].replace(PLACEHOLDER, product))
}

pub fn build(dataset: &DatasetConfig, notices: &ExtractedNotices) -> Result<LegalNotices, String> {
    Ok(LegalNotices {
        licenses: vec![LicenseText {
            id: crate::build::LICENSE_OGL.to_string(),
            title: "Open Game License v1.0a".to_string(),
            text: OGL_TEXT.trim_end().to_string(),
        }],
        product_identity: notices.product_identity.clone(),
        community_use_notice: community_use_notice(LICENSING_AUDIT, PRODUCT_NAME)?,
        section_15: notices
            .section_15
            .iter()
            .map(|n| CopyrightNotice {
                title: n.title.clone(),
                notice: n.notice.clone(),
            })
            .collect(),
        datasets: vec![DatasetNotice {
            id: dataset.id.clone(),
            repository: dataset.repository.clone(),
            commit: dataset.commit.clone(),
            note: dataset.note.clone(),
        }],
    })
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Section 15 titles read "Pathfinder Roleplaying Game Core Rulebook"; upstream `source`
/// values read "Core Rulebook". Sources with no such pairing go through the alias table.
pub fn copyright_for(source: &str, notices: &[NoticeEntry]) -> Option<String> {
    const PREFIXES: &[&str] = &["pathfinderroleplayinggame", "pathfindercampaignsetting"];
    const ALIASES: &[(&str, &str)] = &[("pfsrd", "pathfinderroleplayinggamereferencedocument")];
    let wanted = norm(source);
    let alias = ALIASES
        .iter()
        .find(|(s, _)| *s == wanted)
        .map(|(_, full)| *full);
    let hit = notices.iter().find(|n| {
        let title = norm(&n.title);
        if alias == Some(title.as_str()) {
            return true;
        }
        let bare = PREFIXES
            .iter()
            .find_map(|p| title.strip_prefix(p))
            .unwrap_or(&title);
        !wanted.is_empty() && bare == wanted
    })?;
    Some(format!(
        "{}. {}",
        hit.title.trim_end_matches('.'),
        hit.notice
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notices() -> Vec<NoticeEntry> {
        [
            (
                "Pathfinder Roleplaying Game Reference Document.",
                "© 2011, Paizo.",
            ),
            (
                "Pathfinder Roleplaying Game Core Rulebook",
                "© 2009, Paizo.",
            ),
            ("Pathfinder Roleplaying Game Bestiary 2", "© 2010, Paizo."),
            ("Pathfinder Roleplaying Game Bestiary", "© 2009, Bestiary."),
            (
                "Pathfinder Roleplaying Game GameMastery Guide",
                "© 2010, GMG.",
            ),
            (
                "Pathfinder Roleplaying Game Advanced Player's Guide",
                "© 2010, APG.",
            ),
            (
                "Pathfinder Campaign Setting: Technology Guide",
                "© 2014, TG.",
            ),
            ("Community Use License", "Not a book."),
        ]
        .iter()
        .map(|(t, n)| NoticeEntry {
            title: t.to_string(),
            notice: n.to_string(),
        })
        .collect()
    }

    #[test]
    fn matches_book_titles_with_and_without_prefixes() {
        let n = notices();
        assert_eq!(
            copyright_for("Core Rulebook", &n).as_deref(),
            Some("Pathfinder Roleplaying Game Core Rulebook. © 2009, Paizo.")
        );
        assert_eq!(
            copyright_for("Bestiary", &n).as_deref(),
            Some("Pathfinder Roleplaying Game Bestiary. © 2009, Bestiary.")
        );
        assert_eq!(
            copyright_for("Bestiary 2", &n).as_deref(),
            Some("Pathfinder Roleplaying Game Bestiary 2. © 2010, Paizo.")
        );
        assert!(
            copyright_for("Game Mastery Guide", &n)
                .unwrap()
                .contains("GMG")
        );
        assert!(
            copyright_for("Advanced Player's Guide", &n)
                .unwrap()
                .contains("APG")
        );
        assert!(
            copyright_for("Technology Guide", &n)
                .unwrap()
                .contains("TG")
        );
    }

    #[test]
    fn pfsrd_falls_back_to_the_reference_document_line() {
        let n = notices();
        assert_eq!(
            copyright_for("PFSRD", &n).as_deref(),
            Some("Pathfinder Roleplaying Game Reference Document. © 2011, Paizo.")
        );
    }

    #[test]
    fn unknown_sources_get_no_line() {
        assert_eq!(copyright_for("Made Up Book", &notices()), None);
        assert_eq!(copyright_for("", &notices()), None);
        assert_eq!(copyright_for("License", &notices()), None);
    }

    #[test]
    fn community_use_notice_is_filled_in_from_the_audit() {
        let notice = community_use_notice(LICENSING_AUDIT, "Cast Iron").unwrap();
        assert!(
            notice.starts_with("Cast Iron uses trademarks and/or copyrights owned by Paizo Inc.")
        );
        assert!(
            notice.contains(
                "Cast Iron is not published, endorsed, or specifically approved by Paizo."
            )
        );
        assert!(notice.ends_with("visit paizo.com."));
        assert!(!notice.contains('['));
    }
}
