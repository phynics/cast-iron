//! Reviewed publication category and first release date.
//!
//! The pinned Pf Data registry (`json/sources.json`) carries only a title and a product URL;
//! it has no product line or release date. The reviewed file beside this module records the
//! two fields the Books page sorts by. Category is the Archives of Nethys source-catalogue
//! product line, and `publication_date` is its displayed Estimated Release Date, converted to
//! ISO 8601. See `docs/decisions.md` and `scripts/extract-publication-metadata.py`.
//!
//! The data is additive. A publication with no row here, or with only one of the fields, is
//! still listed; the Books page puts it under `Unknown category` or `Unknown date`.

use crate::pipeline::Failure;
use cast_iron_schema::SourceInfo;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Committed file name inside the canonical configuration directory.
pub const METADATA_FILE: &str = "publication-metadata.toml";
/// The format this compiler reads.
pub const METADATA_FORMAT: u32 = 1;

/// The reviewed metadata file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub version: u32,
    /// Canonical source id to its reviewed fields. Sorted by id.
    #[serde(default)]
    pub publication: BTreeMap<String, PublicationMetadata>,
}

/// The optional fields for one publication.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationMetadata {
    /// Product line, such as `RPG` or `Player Companion`.
    #[serde(default)]
    pub category: Option<String>,
    /// First publication date in ISO 8601 (`YYYY-MM-DD`).
    #[serde(default)]
    pub publication_date: Option<String>,
}

impl Metadata {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let metadata: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        metadata.validate()?;
        Ok(metadata)
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != METADATA_FORMAT {
            return Err(format!(
                "publication metadata version is {}; this compiler reads {METADATA_FORMAT}",
                self.version
            ));
        }
        for (source, row) in &self.publication {
            if source.is_empty() || source.trim() != source {
                return Err(format!(
                    "publication metadata source {source:?} is empty or padded"
                ));
            }
            if row.category.is_none() && row.publication_date.is_none() {
                return Err(format!(
                    "publication metadata {source:?} has neither a category nor a date"
                ));
            }
            if let Some(category) = &row.category
                && (category.is_empty() || category.trim() != category)
            {
                return Err(format!(
                    "publication metadata {source:?} has an empty or padded category"
                ));
            }
            if let Some(date) = &row.publication_date
                && !valid_iso_date(date)
            {
                return Err(format!(
                    "publication metadata {source:?} has date {date:?}; expected a real YYYY-MM-DD date"
                ));
            }
        }
        Ok(())
    }

    /// Copies the reviewed fields onto the catalog rows. A missing row leaves both `None`.
    pub fn apply(&self, sources: &mut [SourceInfo]) {
        for source in sources {
            if let Some(row) = self.publication.get(&source.id) {
                source.category = row.category.clone();
                source.publication_date = row.publication_date.clone();
            }
        }
    }
}

/// True for a real `YYYY-MM-DD` calendar date. The compiler has no calendar crate, so this
/// checks the month length and the leap year directly.
pub fn valid_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return false;
    }
    let year = date[0..4].parse::<u32>().ok();
    let month = date[5..7].parse::<u32>().ok();
    let day = date[8..10].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

/// Loads the reviewed file from the canonical configuration directory, failing the build when
/// it is malformed. A missing file is a configuration error: the Books page would silently lose
/// both sort modes.
pub fn load_from_canonical_dir(canonical_dir: &Path) -> Result<Metadata, Failure> {
    Metadata::load(&canonical_dir.join(METADATA_FILE)).map_err(|e| Failure { lines: vec![e] })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str) -> SourceInfo {
        SourceInfo {
            id: id.into(),
            title: id.into(),
            entry_count: 0,
            copyright: None,
            category: None,
            publication_date: None,
        }
    }

    #[test]
    fn parses_and_applies_reviewed_fields() {
        let metadata = Metadata::parse(
            r#"
version = 1
[publication.core-rulebook]
category = "RPG"
publication_date = "2009-08-01"
[publication.unknown]
category = "Module"
"#,
        )
        .unwrap();
        let mut sources = vec![source("core-rulebook"), source("unknown"), source("other")];
        metadata.apply(&mut sources);
        assert_eq!(sources[0].category.as_deref(), Some("RPG"));
        assert_eq!(sources[0].publication_date.as_deref(), Some("2009-08-01"));
        assert_eq!(sources[1].category.as_deref(), Some("Module"));
        assert_eq!(sources[1].publication_date, None);
        assert_eq!(sources[2].category, None);
    }

    #[test]
    fn rejects_a_wrong_version_an_empty_row_and_a_bad_date() {
        for text in [
            "version = 2\n[publication.a]\ncategory = \"RPG\"\n",
            "version = 1\n[publication.a]\n",
            "version = 1\n[publication.a]\npublication_date = \"2009-02-30\"\n",
            "version = 1\n[publication.a]\npublication_date = \"2009-2-1\"\n",
            "version = 1\n[publication.a]\ncategory = \" RPG\"\n",
            "version = 1\n[publication.a]\ntypo = \"x\"\n",
        ] {
            assert!(Metadata::parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn accepts_leap_days_and_rejects_them_in_common_years() {
        assert!(valid_iso_date("2024-02-29"));
        assert!(!valid_iso_date("2023-02-29"));
        assert!(valid_iso_date("2000-02-29"));
        assert!(!valid_iso_date("1900-02-29"));
        assert!(!valid_iso_date("2009-13-01"));
        assert!(!valid_iso_date("2009-00-01"));
        assert!(!valid_iso_date("2009-04-31"));
    }
}
