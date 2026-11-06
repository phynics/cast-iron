//! Stored non-entry source files.
//!
//! `publications`, `books`, `legal`, `exclusions`, `history`, and `aliases` are
//! stored source data. Catalog stubs, `entry_count`, relations, spell lists,
//! backlinks, search, the manifest, and every coverage count are compiler
//! output and are never stored here. Exclusions are stored because they were
//! decided when the corpus was cut and cannot be recomputed from the entries.

use crate::error::{Error, Result};
use cast_iron_schema::{
    AliasIndex, BookIndex, CoverageHistory, Exclusion, LegalNotices, Rejection, SourceInfo,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One publication's reviewed metadata. `entry_count` is derived from the
/// entries and is not stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<String>,
}

impl Publication {
    pub fn from_source(source: &SourceInfo) -> Self {
        Self {
            id: source.id.clone(),
            title: source.title.clone(),
            copyright: source.copyright.clone(),
            category: source.category.clone(),
            publication_date: source.publication_date.clone(),
        }
    }

    pub fn to_source(&self, entry_count: u32) -> SourceInfo {
        SourceInfo {
            id: self.id.clone(),
            title: self.title.clone(),
            entry_count,
            copyright: self.copyright.clone(),
            category: self.category.clone(),
            publication_date: self.publication_date.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationsFile {
    #[serde(default)]
    pub publication: Vec<Publication>,
}

pub fn write_publications(sources: &[SourceInfo]) -> String {
    let publications: Vec<Publication> = sources.iter().map(Publication::from_source).collect();
    write_publication_list(&publications)
}

/// The canonical `publications.toml`: sorted by id.
pub fn write_publication_list(publications: &[Publication]) -> String {
    let mut publications = publications.to_vec();
    publications.sort_by(|a, b| a.id.cmp(&b.id));
    let file = PublicationsFile {
        publication: publications,
    };
    toml::to_string(&file).expect("publications serialize")
}

pub fn read_publications(path: &Path, text: &str) -> Result<Vec<Publication>> {
    let file: PublicationsFile = toml::from_str(text).map_err(|error| {
        Error::new(
            path,
            toml_line(&error, text),
            format!("publications: {error}"),
        )
    })?;
    let mut seen = std::collections::BTreeSet::new();
    for publication in &file.publication {
        if !seen.insert(publication.id.as_str()) {
            return Err(Error::new(
                path,
                1,
                format!("duplicate publication id {:?}", publication.id),
            ));
        }
    }
    Ok(file.publication)
}

pub fn write_books(books: &BookIndex) -> String {
    toml::to_string(books).expect("books serialize")
}

pub fn read_books(path: &Path, text: &str) -> Result<BookIndex> {
    toml::from_str(text)
        .map_err(|error| Error::new(path, toml_line(&error, text), format!("books: {error}")))
}

pub fn write_aliases(aliases: &AliasIndex) -> String {
    toml::to_string(aliases).expect("aliases serialize")
}

pub fn read_aliases(path: &Path, text: &str) -> Result<AliasIndex> {
    let aliases: AliasIndex = toml::from_str(text)
        .map_err(|error| Error::new(path, toml_line(&error, text), format!("aliases: {error}")))?;
    aliases
        .validate()
        .map_err(|problem| Error::new(path, 1, format!("aliases: {problem}")))?;
    Ok(aliases)
}

pub fn write_legal(legal: &LegalNotices) -> String {
    toml::to_string(legal).expect("legal notices serialize")
}

pub fn read_legal(path: &Path, text: &str) -> Result<LegalNotices> {
    toml::from_str(text)
        .map_err(|error| Error::new(path, toml_line(&error, text), format!("legal: {error}")))
}

/// `exclusions.toml`: what the import left out and why. Rows are only appended.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exclusions {
    /// Upstream paths not selected, with the number of records and the reason.
    #[serde(default)]
    pub excluded: Vec<Exclusion>,
    /// Records selected but dropped, with the reason.
    #[serde(default)]
    pub rejected: Vec<Rejection>,
}

pub fn write_exclusions(exclusions: &Exclusions) -> String {
    toml::to_string(exclusions).expect("exclusions serialize")
}

pub fn read_exclusions(path: &Path, text: &str) -> Result<Exclusions> {
    let exclusions: Exclusions = toml::from_str(text).map_err(|error| {
        Error::new(
            path,
            toml_line(&error, text),
            format!("exclusions: {error}"),
        )
    })?;
    let mut seen = std::collections::BTreeSet::new();
    for row in &exclusions.excluded {
        if !seen.insert((row.path.as_str(), row.reason.as_str())) {
            return Err(Error::new(
                path,
                line_of_value(text, &row.path),
                format!("duplicate excluded row for {:?}", row.path),
            ));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &exclusions.rejected {
        if !seen.insert((row.path.as_str(), row.reason.as_str())) {
            return Err(Error::new(
                path,
                line_of_value(text, &row.path),
                format!("duplicate rejected row for {:?}", row.path),
            ));
        }
    }
    Ok(exclusions)
}

/// `history.toml`: the frozen part of the import report. Never edited.
pub fn write_history(history: &CoverageHistory) -> String {
    toml::to_string(history).expect("history serializes")
}

pub fn read_history(path: &Path, text: &str) -> Result<CoverageHistory> {
    let history: CoverageHistory = toml::from_str(text)
        .map_err(|error| Error::new(path, toml_line(&error, text), format!("history: {error}")))?;
    if history.input_digest.len() != 64
        || !history
            .input_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(Error::new(
            path,
            1,
            "history: input_digest must be 64 hexadecimal characters",
        ));
    }
    Ok(history)
}

fn line_of_value(text: &str, value: &str) -> usize {
    let needle = crate::tomlutil::string(value);
    text.lines()
        .position(|line| line.contains(&needle))
        .map_or(1, |index| index + 1)
}

fn toml_line(error: &toml::de::Error, text: &str) -> usize {
    let Some(span) = error.span() else {
        return 1;
    };
    let end = span.start.min(text.len());
    1 + text[..end].bytes().filter(|byte| *byte == b'\n').count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{AliasBasis, AliasEntry, AliasLegacy, BookNode, BookTree, SourceInfo};

    #[test]
    fn publication_metadata_round_trips_without_derived_counts() {
        let sources = vec![SourceInfo {
            id: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            entry_count: 42,
            copyright: Some("notice".into()),
            category: Some("RPG".into()),
            publication_date: Some("2009-08-01".into()),
        }];
        let text = write_publications(&sources);
        assert!(!text.contains("entry_count"));
        let read = read_publications(Path::new("publications.toml"), &text).unwrap();
        assert_eq!(read[0].to_source(42), sources[0]);
    }

    #[test]
    fn books_round_trip() {
        let books = BookIndex {
            books: vec![BookTree {
                source: "core-rulebook".into(),
                title: "Core Rulebook".into(),
                basis: "reviewed".into(),
                review: vec!["check chapter 3".into()],
                chapters: vec![BookNode {
                    title: "Combat".into(),
                    entry: None,
                    anchor: None,
                    review: None,
                    children: vec![BookNode {
                        title: "Actions".into(),
                        entry: Some(cast_iron_schema::EntryId::parse("ref/rule/actions").unwrap()),
                        anchor: Some("act".into()),
                        review: None,
                        children: vec![],
                    }],
                }],
            }],
        };
        let text = write_books(&books);
        assert_eq!(read_books(Path::new("books.toml"), &text).unwrap(), books);
    }

    #[test]
    fn aliases_round_trip_and_validate() {
        let aliases = AliasIndex {
            format: 1,
            legacy: AliasLegacy {
                count: 1,
                sha256: "abc".into(),
            },
            entries: vec![AliasEntry {
                old: cast_iron_schema::EntryId::parse("old/id").unwrap(),
                target: cast_iron_schema::EntryId::parse("ref/rule/id").unwrap(),
                basis: AliasBasis::Direct,
            }],
            retired: vec![],
        };
        let text = write_aliases(&aliases);
        assert_eq!(
            read_aliases(Path::new("aliases.toml"), &text).unwrap(),
            aliases
        );
    }

    #[test]
    fn legal_notices_round_trip_long_text() {
        let legal = LegalNotices {
            licenses: vec![cast_iron_schema::LicenseText {
                id: "ogl-1.0a".into(),
                title: "Open Game License".into(),
                text: "line one\nline two\n".into(),
            }],
            product_identity: "identity".into(),
            community_use_notice: "notice".into(),
            section_15: vec![cast_iron_schema::CopyrightNotice {
                title: "Core Rulebook".into(),
                notice: "notice".into(),
            }],
            notice_pending: vec!["pending".into()],
            datasets: vec![cast_iron_schema::DatasetNotice {
                id: "psrd-data".into(),
                repository: "https://example.invalid/repo".into(),
                commit: "abc".into(),
                note: "note".into(),
            }],
        };
        let text = write_legal(&legal);
        assert!(text.contains("line one"), "{text}");
        assert_eq!(read_legal(Path::new("legal.toml"), &text).unwrap(), legal);
    }

    #[test]
    fn exclusions_round_trip_and_reject_a_repeated_row() {
        let exclusions = Exclusions {
            excluded: vec![Exclusion {
                path: "json/art.json".into(),
                records: 3,
                reason: "artwork".into(),
            }],
            rejected: vec![Rejection {
                path: "json/a.json#b".into(),
                reason: "not convertible".into(),
            }],
        };
        let text = write_exclusions(&exclusions);
        let path = Path::new("exclusions.toml");
        assert_eq!(read_exclusions(path, &text).unwrap(), exclusions);
        let repeated = format!("{text}\n{}", text.replace("records = 3", "records = 4"));
        let error = read_exclusions(path, &repeated).unwrap_err();
        assert!(error.message.contains("duplicate"), "{error}");
        assert!(read_exclusions(path, "unknown = 1\n").is_err());
        assert_eq!(read_exclusions(path, "").unwrap(), Exclusions::default());
    }

    #[test]
    fn history_round_trips_and_validates_its_digest() {
        let history = CoverageHistory {
            input_digest: "a".repeat(64),
            publications: vec![cast_iron_schema::UpstreamPublication {
                source_id: "core-rulebook".into(),
                upstream_records: 9,
                title: None,
            }],
            ..Default::default()
        };
        let text = write_history(&history);
        let path = Path::new("history.toml");
        assert_eq!(read_history(path, &text).unwrap(), history);
        let bad = text.replace(&"a".repeat(64), "not-a-digest");
        assert!(read_history(path, &bad).is_err());
    }

    #[test]
    fn bad_toml_names_a_line() {
        let error = read_books(Path::new("books.toml"), "books = [").unwrap_err();
        assert_eq!(error.path, Path::new("books.toml"));
        assert!(error.line >= 1);
    }
}
