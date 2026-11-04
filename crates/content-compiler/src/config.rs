//! `source.toml` and `id-overrides.toml`.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    pub dataset: DatasetConfig,
    #[serde(default)]
    pub supplement: Option<crate::pf_approved::Config>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatasetConfig {
    pub id: String,
    pub repository: String,
    #[serde(default)]
    pub branch: String,
    pub commit: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub input_digest: String,
    /// Directories whose JSON files (recursively) are typed records. In `source.toml` these
    /// keys sit under `[dataset]`, because the file has a single table.
    pub typed: Vec<String>,
    /// Rules-tree files.
    pub rules: Vec<String>,
    /// Reviewed directory-to-publication mapping for upstream placeholder sources.
    #[serde(default)]
    pub books: BTreeMap<String, String>,
    /// Sections that upstream stores without its otherwise standard class type.
    #[serde(default)]
    pub prestige_classes: Vec<String>,
}

impl SourceConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
    }
}

/// Explicit id decisions, kept in `id-overrides.toml` next to `source.toml`.
///
/// ```toml
/// [ids]               # upstream url -> explicit entry id (resolves slug collisions)
/// "pfsrd://Book/A..." = "book/a-dots"
/// [aliases]           # entry id -> extra search aliases
/// "book/a-dots" = ["A dots"]
/// [equivalent_urls]   # upstream href -> the url that it should resolve as
/// "pfsrd://Old/Name" = "pfsrd://New/Name"
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdOverrides {
    #[serde(default)]
    pub ids: BTreeMap<String, String>,
    #[serde(default)]
    pub aliases: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub equivalent_urls: BTreeMap<String, String>,
}

impl IdOverrides {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }
}

/// Replaces the `input_digest = "..."` line in place and leaves the rest of the file as is.
pub fn rewrite_digest(toml_text: &str, digest: &str) -> Result<String, String> {
    let mut replaced = false;
    let mut out = String::with_capacity(toml_text.len() + 64);
    for line in toml_text.split_inclusive('\n') {
        if !replaced && line.trim_start().starts_with("input_digest") && line.contains('=') {
            let newline = if line.ends_with('\n') { "\n" } else { "" };
            out.push_str(&format!("input_digest = \"{digest}\"{newline}"));
            replaced = true;
        } else {
            out.push_str(line);
        }
    }
    if replaced {
        Ok(out)
    } else {
        Err("source.toml has no input_digest line to update".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_only_the_digest_line() {
        let src = "# c\n[dataset]\ninput_digest = \"\"\nid = \"x\"\n";
        let out = rewrite_digest(src, "abc").unwrap();
        assert_eq!(out, "# c\n[dataset]\ninput_digest = \"abc\"\nid = \"x\"\n");
        assert!(rewrite_digest("[dataset]\n", "abc").is_err());
    }

    #[test]
    fn parses_overrides_and_rejects_unknown_tables() {
        let o = IdOverrides::parse(
            "[ids]\n\"pfsrd://A/B\" = \"a/b-2\"\n[aliases]\n\"a/b-2\" = [\"Bee\"]\n",
        )
        .unwrap();
        assert_eq!(o.ids["pfsrd://A/B"], "a/b-2");
        assert_eq!(o.aliases["a/b-2"], vec!["Bee"]);
        assert!(IdOverrides::parse("[typo]\nx = 1\n").is_err());
    }
}
