//! Stored non-entry source files.
//!
//! `publication`, `books`, `legal`, `coverage`, and `aliases` are stored source
//! data. Catalog stubs, `entry_count`, relations, spell lists, backlinks,
//! search, and the manifest are compiler output and are never stored here.
//! Coverage is stored because its exclusions were decided when the corpus was
//! cut and cannot be recomputed from the entries.

use crate::error::{Error, Result};
use cast_iron_schema::{
    AliasIndex, BookIndex, CoverageReport, EntryKind, LegalNotices, SourceInfo,
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
    let mut publications: Vec<Publication> = sources.iter().map(Publication::from_source).collect();
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

/// The frozen coverage report. It preserves exclusions and rejections that were
/// decided when the corpus was cut, so a corpus compile can emit the same report.
pub fn write_coverage(coverage: &CoverageReport) -> String {
    toml::to_string(coverage).expect("coverage serializes")
}

pub fn read_coverage(path: &Path, text: &str) -> Result<CoverageReport> {
    let coverage: CoverageReport = toml::from_str(text)
        .map_err(|error| Error::new(path, toml_line(&error, text), format!("coverage: {error}")))?;
    validate_coverage(&coverage)
        .map_err(|problem| Error::new(path, 1, format!("coverage: {problem}")))?;
    Ok(coverage)
}

fn validate_coverage(coverage: &CoverageReport) -> std::result::Result<(), String> {
    if coverage.input_digest.len() != 64
        || !coverage
            .input_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("input_digest must be 64 hexadecimal characters".into());
    }
    for kind in coverage.by_kind.keys() {
        if EntryKind::from_slug(kind).is_none() {
            return Err(format!("unknown kind {kind:?} in by_kind"));
        }
    }
    Ok(())
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
    fn coverage_round_trips_and_validates_its_digest_and_kinds() {
        let coverage = cast_iron_schema::CoverageReport {
            input_digest: "a".repeat(64),
            by_kind: [("spell".to_string(), 2)].into_iter().collect(),
            ..Default::default()
        };
        let text = write_coverage(&coverage);
        assert_eq!(
            read_coverage(Path::new("coverage.toml"), &text).unwrap(),
            coverage
        );
        let bad_digest = text.replace(&"a".repeat(64), "not-a-digest");
        assert!(read_coverage(Path::new("coverage.toml"), &bad_digest).is_err());
        let bad_kind = text.replace("spell = 2", "not_a_kind = 2");
        assert!(read_coverage(Path::new("coverage.toml"), &bad_kind).is_err());
    }

    #[test]
    fn bad_toml_names_a_line() {
        let error = read_books(Path::new("books.toml"), "books = [").unwrap_err();
        assert_eq!(error.path, Path::new("books.toml"));
        assert!(error.line >= 1);
    }
}
