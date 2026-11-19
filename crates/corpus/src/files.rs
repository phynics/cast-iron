//! Stored non-entry source files.
//!
//! `publications`, `books`, `legal`, `exclusions`, `history`, and `aliases` are
//! stored source data. Catalog stubs, `entry_count`, relations, spell lists,
//! backlinks, search, the manifest, and every coverage count are compiler
//! output and are never stored here. Exclusions are stored because they were
//! decided when the corpus was cut and cannot be recomputed from the entries.

use crate::error::{Error, Result};
use cast_iron_schema::{
    AliasIndex, BookIndex, BookTree, CoverageHistory, Exclusion, LegalNotices, Rejection,
    SeriesKind, SourceInfo,
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
    /// The optional series line (#159). The three kinds are `adventure-path`, `module` and
    /// `society`; `corpus-check` holds the required fields for each.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_kind: Option<SeriesKind>,
    /// The Adventure Path name, such as `Rise of the Runelords`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// The issue within its Adventure Path, such as `1`, or the Society scenario, such as
    /// `3-01`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_number: Option<String>,
    /// The Pathfinder Society season, such as `3`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<u16>,
    /// True when the publication is not published by Paizo. Absent means a Paizo book.
    /// `corpus-check` rejects it beside a copyright line that names Paizo.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub third_party: bool,
    /// The id suffix that new add-on and revision entries from this publication take,
    /// such as `teitr`. Optional; a publication without one can still hold entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// The Archives of Nethys source page, as a path relative to `https://aonprd.com/`,
    /// such as `SourceDisplay.aspx?FixedSource=Core%20Rulebook`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aon: Option<String>,
}

impl Publication {
    pub fn from_source(source: &SourceInfo) -> Self {
        Self {
            id: source.id.clone(),
            title: source.title.clone(),
            copyright: source.copyright.clone(),
            category: source.category.clone(),
            publication_date: source.publication_date.clone(),
            series_kind: source.series_kind,
            series: source.series.clone(),
            series_number: source.series_number.clone(),
            season: source.season,
            third_party: source.third_party,
            suffix: None,
            aon: source.aon.clone(),
        }
    }

    pub fn to_source(&self, entry_count: u32) -> SourceInfo {
        SourceInfo {
            share_slot: None,
            id: self.id.clone(),
            title: self.title.clone(),
            entry_count,
            copyright: self.copyright.clone(),
            category: self.category.clone(),
            publication_date: self.publication_date.clone(),
            series_kind: self.series_kind,
            series: self.series.clone(),
            series_number: self.series_number.clone(),
            season: self.season,
            third_party: self.third_party,
            aon: self.aon.clone(),
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

/// `option-groups.toml`: the class-option vocabulary. A missing file means no groups.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionGroups {
    /// Keyed by the slug a [`cast_iron_schema::ClassOptionFields::group`] names.
    #[serde(default)]
    pub groups: std::collections::BTreeMap<String, OptionGroupDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionGroupDef {
    pub label: String,
    pub singular: String,
    /// Class entries that take the option group.
    #[serde(default)]
    pub classes: Vec<String>,
}

pub fn write_option_groups(groups: &OptionGroups) -> String {
    toml::to_string(groups).expect("option groups serialize")
}

pub fn read_option_groups(path: &Path, text: &str) -> Result<OptionGroups> {
    let groups: OptionGroups = toml::from_str(text).map_err(|error| {
        Error::new(
            path,
            toml_line(&error, text),
            format!("option-groups: {error}"),
        )
    })?;
    for (slug, group) in &groups.groups {
        if slug.is_empty() || slug.contains('/') {
            return Err(Error::new(
                path,
                line_of_value(text, slug),
                format!("option group slug {slug:?} is not a valid id segment"),
            ));
        }
        if group.label.trim().is_empty() || group.singular.trim().is_empty() {
            return Err(Error::new(
                path,
                line_of_value(text, slug),
                format!("option group `{slug}` needs a label and a singular label"),
            ));
        }
    }
    Ok(groups)
}

pub fn write_books(books: &BookIndex) -> String {
    toml::to_string(books).expect("books serialize")
}

pub fn read_books(path: &Path, text: &str) -> Result<BookIndex> {
    toml::from_str(text)
        .map_err(|error| Error::new(path, toml_line(&error, text), format!("books: {error}")))
}

/// One curated book as `books/<publication>.toml` stores it: today's `[[books]]` block, exactly one.
pub fn write_curated_book(tree: &BookTree) -> String {
    write_books(&BookIndex {
        books: vec![tree.clone()],
    })
}

/// Reads `books/<publication>.toml`. The file holds one `[[books]]` block, and its `source` is the
/// file stem, so a file cannot name another book than its path does.
pub fn read_curated_book(path: &Path, text: &str) -> Result<BookTree> {
    let mut index = read_books(path, text)?;
    if index.books.len() != 1 {
        return Err(Error::new(
            path,
            1,
            format!(
                "a curated book file holds exactly one [[books]] block; this one holds {}",
                index.books.len()
            ),
        ));
    }
    let tree = index.books.remove(0);
    let stem = path.file_stem().and_then(|stem| stem.to_str());
    if stem != Some(tree.source.as_str()) {
        return Err(Error::new(
            path,
            text.lines()
                .position(|line| line.starts_with("source ="))
                .map_or(1, |index| index + 1),
            format!(
                "the file is for book `{}` but its name says `{}`; the file name is the publication id",
                tree.source,
                stem.unwrap_or("?")
            ),
        ));
    }
    Ok(tree)
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
///
/// A row is never edited, so a later change that ships an excluded or rejected record appends a
/// [`Resolution`] instead. The newest word on a path wins: the coverage report lists a resolved
/// path as included, not as excluded ([`Exclusions::active_excluded`]) or rejected
/// ([`Exclusions::active_rejected`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exclusions {
    /// Upstream paths not selected, with the number of records and the reason.
    #[serde(default)]
    pub excluded: Vec<Exclusion>,
    /// Records selected but dropped, with the reason.
    #[serde(default)]
    pub rejected: Vec<Rejection>,
    /// Later rows that supersede an earlier `excluded` or `rejected` row because the record now
    /// ships. Absent
    /// from files that have none, so an older file writes back unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved: Vec<Resolution>,
}

/// A superseding row: the upstream `path` of an earlier `excluded` or `rejected` row is no longer
/// left out, because the corpus now holds the record as the entries in `entries`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    /// The upstream path of the earlier `excluded` or `rejected` row. Matches it exactly; it
    /// supersedes every row of either kind with that path.
    pub path: String,
    /// Entry ids that now hold the record. At least one; `corpus-check` verifies each exists.
    pub entries: Vec<String>,
    /// Why the exclusion no longer applies, with the issue reference.
    pub reason: String,
}

impl Exclusions {
    /// The `excluded` rows whose path no `resolved` row supersedes, in file order. This is what
    /// the coverage report lists. With no `resolved` row it equals `excluded`.
    pub fn active_excluded(&self) -> Vec<Exclusion> {
        let resolved: std::collections::BTreeSet<&str> =
            self.resolved.iter().map(|row| row.path.as_str()).collect();
        self.excluded
            .iter()
            .filter(|row| !resolved.contains(row.path.as_str()))
            .cloned()
            .collect()
    }

    /// The `rejected` rows whose path no `resolved` row supersedes, in file order. This is what
    /// the coverage report lists as rejected. With no `resolved` row it equals `rejected`.
    pub fn active_rejected(&self) -> Vec<Rejection> {
        let resolved: std::collections::BTreeSet<&str> =
            self.resolved.iter().map(|row| row.path.as_str()).collect();
        self.rejected
            .iter()
            .filter(|row| !resolved.contains(row.path.as_str()))
            .cloned()
            .collect()
    }
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
    let superseded: std::collections::BTreeSet<&str> = exclusions
        .excluded
        .iter()
        .map(|row| row.path.as_str())
        .chain(exclusions.rejected.iter().map(|row| row.path.as_str()))
        .collect();
    let mut resolved = std::collections::BTreeSet::new();
    for row in &exclusions.resolved {
        // The last line with the path: the earlier `excluded` or `rejected` row holds it first.
        let needle = crate::tomlutil::string(&row.path);
        let at = text
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(&needle))
            .last()
            .map_or(1, |(index, _)| index + 1);
        if !superseded.contains(row.path.as_str()) {
            return Err(Error::new(
                path,
                at,
                format!(
                    "resolved row for {:?} supersedes nothing: no earlier excluded or rejected row has that path",
                    row.path
                ),
            ));
        }
        if !resolved.insert(row.path.as_str()) {
            return Err(Error::new(
                path,
                at,
                format!("duplicate resolved row for {:?}", row.path),
            ));
        }
        if row.entries.is_empty() || row.reason.trim().is_empty() {
            return Err(Error::new(
                path,
                at,
                format!(
                    "resolved row for {:?} needs a non-empty `entries` list and `reason`",
                    row.path
                ),
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
            share_slot: None,
            id: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            aon: None,
            entry_count: 42,
            copyright: Some("notice".into()),
            category: Some("RPG".into()),
            publication_date: Some("2009-08-01".into()),
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
        }];
        let text = write_publications(&sources);
        assert!(!text.contains("entry_count"));
        assert!(!text.contains("third_party"), "{text}");
        assert!(!text.contains("series_kind"), "{text}");
        let read = read_publications(Path::new("publications.toml"), &text).unwrap();
        assert_eq!(read[0].to_source(42), sources[0]);
        assert_eq!(read[0].suffix, None);
        assert!(!read[0].third_party, "an absent marker means a Paizo book");
    }

    #[test]
    fn the_third_party_marker_round_trips_in_canonical_order() {
        let publication = Publication {
            id: "tome-of-horrors".into(),
            title: "Tome of Horrors".into(),
            aon: None,
            copyright: Some("Necromancer Games, (c) 2011".into()),
            category: None,
            publication_date: Some("2011-01-01".into()),
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: true,
            suffix: Some("toh".into()),
        };
        let text = write_publication_list(std::slice::from_ref(&publication));
        assert!(text.contains("third_party = true"), "{text}");
        // The marker sits after the reviewed metadata and before the suffix.
        let order = |key: &str| {
            text.find(key)
                .unwrap_or_else(|| panic!("no {key} in {text}"))
        };
        assert!(order("publication_date") < order("third_party"), "{text}");
        assert!(order("third_party") < order("suffix"), "{text}");
        let read = read_publications(Path::new("publications.toml"), &text).unwrap();
        assert_eq!(read, vec![publication.clone()]);
        assert_eq!(
            read[0].to_source(0),
            SourceInfo {
                share_slot: None,
                id: "tome-of-horrors".into(),
                title: "Tome of Horrors".into(),
                aon: None,
                entry_count: 0,
                copyright: Some("Necromancer Games, (c) 2011".into()),
                category: None,
                publication_date: Some("2011-01-01".into()),
                series_kind: None,
                series: None,
                series_number: None,
                season: None,
                third_party: true,
            }
        );
    }

    #[test]
    fn aon_round_trips_and_is_optional() {
        let publication = Publication {
            id: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            aon: Some("SourceDisplay.aspx?FixedSource=Core%20Rulebook".into()),
            copyright: Some("notice".into()),
            category: None,
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
            suffix: None,
        };
        let text = write_publication_list(std::slice::from_ref(&publication));
        assert!(
            text.contains("aon = \"SourceDisplay.aspx?FixedSource=Core%20Rulebook\""),
            "{text}"
        );
        let read = read_publications(Path::new("publications.toml"), &text).unwrap();
        assert_eq!(read, vec![publication]);
        assert_eq!(
            read[0].to_source(0).aon.as_deref(),
            Some("SourceDisplay.aspx?FixedSource=Core%20Rulebook")
        );

        let without = Publication {
            aon: None,
            ..read[0].clone()
        };
        let text = write_publication_list(std::slice::from_ref(&without));
        assert!(!text.contains("aon ="), "{text}");
    }

    #[test]
    fn series_fields_round_trip_in_canonical_order_and_stay_optional() {
        let adventure = Publication {
            id: "pathfinder-85-fires-of-creation".into(),
            title: "Pathfinder #85: Fires of Creation".into(),
            aon: None,
            copyright: Some("notice".into()),
            category: Some("Adventure Path".into()),
            publication_date: Some("2014-09-10".into()),
            series_kind: Some(SeriesKind::AdventurePath),
            series: Some("Iron Gods".into()),
            series_number: Some("1".into()),
            season: None,
            third_party: false,
            suffix: None,
        };
        let text = write_publication_list(std::slice::from_ref(&adventure));
        let order = |key: &str| {
            text.find(&format!("{key} = "))
                .unwrap_or_else(|| panic!("no {key} in {text}"))
        };
        assert!(text.contains("series_kind = \"adventure-path\""), "{text}");
        assert!(order("publication_date") < order("series_kind"), "{text}");
        assert!(order("series_kind") < order("series"), "{text}");
        assert!(order("series") < order("series_number"), "{text}");

        let read = read_publications(Path::new("publications.toml"), &text).unwrap();
        assert_eq!(read, vec![adventure.clone()]);
        assert_eq!(read[0].to_source(2).series, Some("Iron Gods".into()));

        // A module carries no series name or number.
        let module = Publication {
            id: "academy-of-secrets".into(),
            title: "Academy of Secrets".into(),
            aon: None,
            copyright: Some("notice".into()),
            category: Some("Module".into()),
            publication_date: Some("2011-05-01".into()),
            series_kind: Some(SeriesKind::Module),
            series: None,
            series_number: None,
            season: None,
            third_party: false,
            suffix: None,
        };
        let text = write_publication_list(std::slice::from_ref(&module));
        assert!(!text.contains("series ="), "{text}");
        assert!(!text.contains("season"), "{text}");
        let read = read_publications(Path::new("publications.toml"), &text).unwrap();
        assert_eq!(read, vec![module]);
    }

    #[test]
    fn a_publication_suffix_round_trips_and_is_optional() {
        let publication = Publication {
            id: "eitr".into(),
            title: "The Elephant in the Room".into(),
            aon: None,
            copyright: Some("notice".into()),
            category: None,
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
            suffix: Some("teitr".into()),
        };
        let text = write_publication_list(std::slice::from_ref(&publication));
        assert!(text.contains("suffix = \"teitr\""), "{text}");
        let read = read_publications(Path::new("publications.toml"), &text).unwrap();
        assert_eq!(read, vec![publication.clone()]);

        let mut without = publication;
        without.suffix = None;
        let text = write_publication_list(std::slice::from_ref(&without));
        assert!(!text.contains("suffix"), "{text}");
        assert_eq!(
            read_publications(Path::new("publications.toml"), &text).unwrap(),
            vec![without]
        );
    }

    #[test]
    fn option_groups_round_trip_in_slug_order() {
        let mut groups = OptionGroups::default();
        groups.groups.insert(
            "hex".into(),
            OptionGroupDef {
                label: "Hexes".into(),
                singular: "Hex".into(),
                classes: vec!["ref/class/witch".into()],
            },
        );
        groups.groups.insert(
            "arcane-discovery".into(),
            OptionGroupDef {
                label: "Arcane discoveries".into(),
                singular: "Arcane discovery".into(),
                classes: vec!["ref/class/wizard".into()],
            },
        );
        let text = write_option_groups(&groups);
        assert!(
            text.find("[groups.arcane-discovery]") < text.find("[groups.hex]"),
            "{text}"
        );
        let path = Path::new("option-groups.toml");
        assert_eq!(read_option_groups(path, &text).unwrap(), groups);
        assert_eq!(
            read_option_groups(path, "").unwrap(),
            OptionGroups::default()
        );
        assert!(read_option_groups(path, "[groups.bad]\nlabel = \"L\"\n").is_err());
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
                    collapse: true,
                    lists_children: false,
                    children: vec![BookNode {
                        title: "Actions".into(),
                        entry: Some(cast_iron_schema::EntryId::parse("ref/rule/actions").unwrap()),
                        anchor: Some("act".into()),
                        review: None,
                        collapse: false,
                        lists_children: false,
                        children: vec![],
                    }],
                }],
            }],
        };
        let text = write_books(&books);
        assert!(text.contains("collapse = true"), "{text}");
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
            publication_licenses: vec![],
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
            resolved: vec![],
        };
        let text = write_exclusions(&exclusions);
        assert!(!text.contains("resolved"), "{text}");
        let path = Path::new("exclusions.toml");
        assert_eq!(read_exclusions(path, &text).unwrap(), exclusions);
        let repeated = format!("{text}\n{}", text.replace("records = 3", "records = 4"));
        let error = read_exclusions(path, &repeated).unwrap_err();
        assert!(error.message.contains("duplicate"), "{error}");
        assert!(read_exclusions(path, "unknown = 1\n").is_err());
        assert_eq!(read_exclusions(path, "").unwrap(), Exclusions::default());
    }

    #[test]
    fn a_resolved_row_supersedes_its_excluded_row() {
        let path = Path::new("exclusions.toml");
        let old = "[[excluded]]\npath = \"json/a.json#x\"\nrecords = 1\nreason = \"out of scope\"\n\n[[excluded]]\npath = \"json/b.json#y\"\nrecords = 1\nreason = \"art\"\n";
        // An older file reads with no resolved row and writes no `resolved` key.
        let before = read_exclusions(path, old).unwrap();
        assert!(before.resolved.is_empty());
        assert!(!write_exclusions(&before).contains("resolved"));
        assert_eq!(before.active_excluded(), before.excluded);

        let text = format!(
            "{old}\n[[resolved]]\npath = \"json/a.json#x\"\nentries = [\"ref/a/x\"]\nreason = \"ships now. Refs #36.\"\n"
        );
        let after = read_exclusions(path, &text).unwrap();
        assert_eq!(after.excluded, before.excluded);
        assert_eq!(after.resolved.len(), 1);
        let active = after.active_excluded();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].path, "json/b.json#y");
        assert_eq!(
            read_exclusions(path, &write_exclusions(&after)).unwrap(),
            after
        );
    }

    #[test]
    fn a_resolved_row_without_an_earlier_excluded_row_is_refused() {
        let path = Path::new("exclusions.toml");
        let orphan =
            "[[resolved]]\npath = \"json/a.json#x\"\nentries = [\"ref/a/x\"]\nreason = \"ships\"\n";
        let error = read_exclusions(path, orphan).unwrap_err();
        assert!(error.message.contains("supersedes nothing"), "{error}");

        // A row for another path than the rejected row's is still an orphan.
        let other = format!("[[rejected]]\npath = \"json/b.json#y\"\nreason = \"r\"\n\n{orphan}");
        let error = read_exclusions(path, &other).unwrap_err();
        assert!(error.message.contains("supersedes nothing"), "{error}");
    }

    #[test]
    fn a_resolved_row_supersedes_its_rejected_row() {
        let path = Path::new("exclusions.toml");
        let old = "[[rejected]]\npath = \"json/domains.json#x\"\nreason = \"converter\"\n\n[[rejected]]\npath = \"json/domains.json#y\"\nreason = \"converter\"\n\n[[excluded]]\npath = \"json/b.json#z\"\nrecords = 1\nreason = \"art\"\n";
        let before = read_exclusions(path, old).unwrap();
        assert_eq!(before.active_rejected(), before.rejected);
        assert_eq!(before.active_excluded(), before.excluded);

        let text = format!(
            "{old}\n[[resolved]]\npath = \"json/domains.json#x\"\nentries = [\"ref/domain/x\"]\nreason = \"ships now. Refs #103.\"\n"
        );
        let after = read_exclusions(path, &text).unwrap();
        // The old rows stay in the file; only the report's view changes.
        assert_eq!(after.rejected, before.rejected);
        let rejected = after.active_rejected();
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].path, "json/domains.json#y");
        assert_eq!(after.active_excluded(), before.excluded);
        assert_eq!(
            read_exclusions(path, &write_exclusions(&after)).unwrap(),
            after
        );
    }

    #[test]
    fn a_repeated_or_empty_resolved_row_is_refused() {
        let path = Path::new("exclusions.toml");
        let base = "[[excluded]]\npath = \"json/a.json#x\"\nrecords = 1\nreason = \"r\"\n\n";
        let row = "[[resolved]]\npath = \"json/a.json#x\"\nentries = [\"ref/a/x\"]\nreason = \"ships\"\n\n";
        assert!(read_exclusions(path, &format!("{base}{row}")).is_ok());
        let error = read_exclusions(path, &format!("{base}{row}{row}")).unwrap_err();
        assert!(error.message.contains("duplicate resolved"), "{error}");
        let no_entries = row.replace("[\"ref/a/x\"]", "[]");
        assert!(read_exclusions(path, &format!("{base}{no_entries}")).is_err());
        let no_reason = row.replace("ships", " ");
        assert!(read_exclusions(path, &format!("{base}{no_reason}")).is_err());

        // The same holds when the earlier row is a rejected row.
        let rejected = "[[rejected]]\npath = \"json/a.json#x\"\nreason = \"r\"\n\n";
        assert!(read_exclusions(path, &format!("{rejected}{row}")).is_ok());
        let error = read_exclusions(path, &format!("{rejected}{row}{row}")).unwrap_err();
        assert!(error.message.contains("duplicate resolved"), "{error}");
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
