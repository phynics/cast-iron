//! The two inputs of the book view: the curated chapter list (`book-order.toml`) and the
//! upstream sibling order of typed records (`upstream-order.json`).
//!
//! `book-order.toml` is written and reviewed by people. `upstream-order.json` is extracted
//! from the dataset's SQLite databases by `scripts/extract-upstream-order.py`, because the
//! compiler never opens those databases. Both files are committed, so a compile reads the
//! same bytes every time.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookOrderFile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub book: Vec<BookSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookSpec {
    /// Catalog source id, such as `core-rulebook`.
    pub source: String,
    pub title: String,
    /// Where the chapter list comes from, and how sure the curator is.
    #[serde(default)]
    pub basis: String,
    /// Open questions for the human reviewer.
    #[serde(default)]
    pub review: Vec<String>,
    #[serde(default)]
    pub chapter: Vec<ChapterSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterSpec {
    /// Title in the printed book.
    pub title: String,
    /// Why this chapter needs a second look, when it does.
    #[serde(default)]
    pub review: Option<String>,
    /// Upstream locations that make up the chapter, in the order that they appear in it.
    #[serde(rename = "match", default)]
    pub matches: Vec<MatchSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchSpec {
    /// An upstream url. It claims the record with this url and every record below it.
    pub url: String,
    /// Entry kind slugs that this match accepts. Empty accepts every kind.
    #[serde(default)]
    pub kinds: Vec<String>,
    /// A prefix of the upstream file path, such as `core_rulebook/class/npc/`. It tells apart
    /// records that share a url prefix, like the classes of one book. Empty accepts every path.
    #[serde(default)]
    pub path: String,
}

impl BookOrderFile {
    pub fn load(path: &Path) -> Result<Option<Self>, String> {
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let file: Self =
            toml::from_str(&text).map_err(|e| format!("invalid {}: {e}", path.display()))?;
        file.check()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Some(file))
    }

    fn check(&self) -> Result<(), String> {
        let mut sources = std::collections::BTreeSet::new();
        for book in &self.book {
            if !sources.insert(book.source.as_str()) {
                return Err(format!("book `{}` is listed twice", book.source));
            }
            for chapter in &book.chapter {
                for m in &chapter.matches {
                    if !m.url.starts_with("pfsrd://") {
                        return Err(format!(
                            "`{}` / `{}`: match url {:?} is not a pfsrd:// url",
                            book.source, chapter.title, m.url
                        ));
                    }
                    for kind in &m.kinds {
                        if cast_iron_schema::EntryKind::from_slug(kind).is_none() {
                            return Err(format!(
                                "`{}` / `{}`: unknown kind {kind:?}",
                                book.source, chapter.title
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpstreamOrderFile {
    #[serde(default)]
    #[allow(dead_code)]
    version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    note: String,
    #[serde(default)]
    lists: Vec<UpstreamList>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpstreamList {
    parent: String,
    /// Child names in upstream order. A child that starts with `pfsrd://` is a full url;
    /// any other child is appended to `parent` after a `/`.
    children: Vec<String>,
}

/// Position of each typed record among its upstream siblings.
#[derive(Debug, Clone, Default)]
pub struct UpstreamOrder {
    rank: BTreeMap<String, u32>,
}

impl UpstreamOrder {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let file: UpstreamOrderFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let mut rank = BTreeMap::new();
        let mut next = 0u32;
        for list in file.lists {
            for child in list.children {
                let url = if child.starts_with("pfsrd://") {
                    child
                } else {
                    format!("{}/{child}", list.parent)
                };
                rank.entry(url).or_insert(next);
                next += 1;
            }
        }
        Ok(Self { rank })
    }

    pub fn rank(&self, url: &str) -> Option<u32> {
        self.rank.get(url).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.rank.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_follow_the_file_and_children_extend_the_parent() {
        let order = UpstreamOrder::parse(
            r#"{"lists":[{"parent":"pfsrd://Core Rulebook/Races","children":["Dwarves","Elves","pfsrd://Core Rulebook/Other/X"]}]}"#,
        )
        .unwrap();
        assert_eq!(order.rank("pfsrd://Core Rulebook/Races/Dwarves"), Some(0));
        assert_eq!(order.rank("pfsrd://Core Rulebook/Races/Elves"), Some(1));
        assert_eq!(order.rank("pfsrd://Core Rulebook/Other/X"), Some(2));
        assert_eq!(order.rank("pfsrd://Core Rulebook/Races/Gnomes"), None);
    }

    #[test]
    fn the_chapter_file_rejects_unknown_keys_and_kinds() {
        let ok = r#"
[[book]]
source = "core-rulebook"
title = "Core Rulebook"
[[book.chapter]]
title = "Combat"
match = [{ url = "pfsrd://Core Rulebook/Rules/Combat" }]
"#;
        let file: BookOrderFile = toml::from_str(ok).unwrap();
        assert!(file.check().is_ok());
        assert!(
            toml::from_str::<BookOrderFile>("[[book]]\nsource = \"a\"\ntitle = \"A\"\ntypo = 1\n")
                .is_err()
        );
        let bad_kind = ok.replace(
            "{ url = \"pfsrd://Core Rulebook/Rules/Combat\" }",
            "{ url = \"pfsrd://Core Rulebook/Rules/Combat\", kinds = [\"vehicle\"] }",
        );
        let file: BookOrderFile = toml::from_str(&bad_kind).unwrap();
        assert!(file.check().unwrap_err().contains("unknown kind"));
    }
}
