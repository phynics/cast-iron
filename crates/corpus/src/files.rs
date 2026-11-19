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
    ResolvedExclusion, SeriesKind, SourceInfo,
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
/// row as resolved ([`Exclusions::resolved_rows`]), not as excluded
/// ([`Exclusions::active_excluded`]) or rejected ([`Exclusions::active_rejected`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exclusions {
    /// Upstream paths not selected, with the number of records and the reason.
    #[serde(default)]
    pub excluded: Vec<Exclusion>,
    /// Records selected but dropped, with the reason.
    #[serde(default)]
    pub rejected: Vec<Rejection>,
    /// Rows that supersede one `excluded` or `rejected` row because the record now ships. By
    /// convention they come last. Absent from files that have none, so an older file writes back
    /// unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved: Vec<Resolution>,
}

/// A superseding row: the upstream `path` of an `excluded` or `rejected` row is no longer left
/// out, because the corpus now holds the record as the entries in `entries`.
///
/// A resolution supersedes exactly one row. When one row of either kind has the path, `path`
/// alone names it. When several rows share the path (an `excluded` and a `rejected` row, or rows
/// with different reasons), `supersedes` is required: it gives the full `reason` of the row, which
/// may be blank. `kind` alone never selects a row on a shared path; it names the table when both
/// tables hold a row with that path and reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    /// The upstream path of the row. Matches it exactly.
    pub path: String,
    /// The full `reason` of the row, copied exactly, when `path` alone is ambiguous. With `path`
    /// it is the identity of a row within its table. Leave it out when only one row has the path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    /// The table of the row, when `path` and `supersedes` still match a row of each table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<RowKind>,
    /// Entry ids that now hold the record. At least one; `corpus-check` verifies each exists.
    pub entries: Vec<String>,
    /// Why the exclusion no longer applies, with the issue reference.
    pub reason: String,
}

/// The table an exclusion row lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowKind {
    Excluded,
    Rejected,
}

impl RowKind {
    fn as_str(self) -> &'static str {
        match self {
            RowKind::Excluded => "excluded",
            RowKind::Rejected => "rejected",
        }
    }
}

impl Resolution {
    /// Whether this row supersedes the `kind` row with `path` and `reason`: the same path, the
    /// same reason when `supersedes` is given (whole string, not a prefix), and the same table
    /// when `kind` is given.
    fn supersedes_row(&self, kind: RowKind, path: &str, reason: &str) -> bool {
        self.path == path
            && self.supersedes.as_deref().is_none_or(|full| reason == full)
            && self.kind.is_none_or(|k| k == kind)
    }
}

impl Exclusions {
    /// The `excluded` rows that no `resolved` row supersedes, in file order. This is what the
    /// coverage report lists as excluded. With no `resolved` row it equals `excluded`.
    pub fn active_excluded(&self) -> Vec<Exclusion> {
        self.excluded
            .iter()
            .filter(|row| {
                !self
                    .resolved
                    .iter()
                    .any(|r| r.supersedes_row(RowKind::Excluded, &row.path, &row.reason))
            })
            .cloned()
            .collect()
    }

    /// The `rejected` rows that no `resolved` row supersedes, in file order. This is what the
    /// coverage report lists as rejected. With no `resolved` row it equals `rejected`.
    pub fn active_rejected(&self) -> Vec<Rejection> {
        self.rejected
            .iter()
            .filter(|row| {
                !self
                    .resolved
                    .iter()
                    .any(|r| r.supersedes_row(RowKind::Rejected, &row.path, &row.reason))
            })
            .cloned()
            .collect()
    }

    /// The `resolved` rows with the kind of the row each supersedes, in file order. This is what
    /// the coverage report lists as resolved. Empty when the file has no `resolved` row.
    pub fn resolved_rows(&self) -> Vec<ResolvedExclusion> {
        self.resolved
            .iter()
            .filter_map(|r| {
                let was = if self
                    .excluded
                    .iter()
                    .any(|row| r.supersedes_row(RowKind::Excluded, &row.path, &row.reason))
                {
                    "excluded"
                } else if self
                    .rejected
                    .iter()
                    .any(|row| r.supersedes_row(RowKind::Rejected, &row.path, &row.reason))
                {
                    "rejected"
                } else {
                    return None;
                };
                Some(ResolvedExclusion {
                    path: r.path.clone(),
                    was: was.to_string(),
                    entries: r.entries.clone(),
                    reason: r.reason.clone(),
                })
            })
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
    // Every `resolved` row names the one row it supersedes. The reader looks at the whole file;
    // that the `resolved` rows come last is a convention, not something it checks.
    let rows: Vec<(RowKind, &str, &str)> = exclusions
        .excluded
        .iter()
        .map(|row| (RowKind::Excluded, row.path.as_str(), row.reason.as_str()))
        .chain(
            exclusions
                .rejected
                .iter()
                .map(|row| (RowKind::Rejected, row.path.as_str(), row.reason.as_str())),
        )
        .collect();
    let mut taken = std::collections::BTreeSet::new();
    for row in &exclusions.resolved {
        // The last line with the path: the `excluded` or `rejected` row holds it first.
        let needle = crate::tomlutil::string(&row.path);
        let at = text
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(&needle))
            .last()
            .map_or(1, |(index, _)| index + 1);
        let shared: Vec<_> = rows
            .iter()
            .filter(|(_, row_path, _)| *row_path == row.path)
            .collect();
        if shared.is_empty() {
            return Err(Error::new(
                path,
                at,
                format!(
                    "resolved row for {:?} supersedes nothing: no excluded or rejected row has that path",
                    row.path
                ),
            ));
        }
        // A path with more than one row, across both tables, needs `supersedes`. `kind` alone
        // would pick a row today and turn ambiguous when a row with that path is appended.
        // `supersedes` may be blank: it is the exact reason of a row, and a row can have one.
        if shared.len() > 1 && row.supersedes.is_none() {
            return Err(Error::new(
                path,
                at,
                format!(
                    "resolved row for {:?} is ambiguous: {} rows have that path ({}); add `supersedes`, the exact full `reason` of the row it resolves, copied from the file (`kind` alone does not select a row on a shared path)",
                    row.path,
                    shared.len(),
                    shared
                        .iter()
                        .map(|(kind, _, _)| kind.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        let matched: Vec<_> = shared
            .iter()
            .filter(|(kind, row_path, reason)| row.supersedes_row(*kind, row_path, reason))
            .collect();
        match matched.len() {
            0 => {
                let what = match (row.supersedes.as_deref(), row.kind) {
                    (Some(full), Some(kind))
                        if shared
                            .iter()
                            .any(|(k, _, reason)| *k != kind && *reason == full) =>
                    {
                        let other = if kind == RowKind::Excluded {
                            RowKind::Rejected
                        } else {
                            RowKind::Excluded
                        };
                        format!(
                            "that `reason` exists in the {} table, not in the {} table `kind` names",
                            other.as_str(),
                            kind.as_str()
                        )
                    }
                    (Some(_), _) => {
                        "`supersedes` is not the exact, full `reason` of any row with that path"
                            .to_string()
                    }
                    (None, _) => "no row with that path is in the table `kind` names".to_string(),
                };
                return Err(Error::new(
                    path,
                    at,
                    format!("resolved row for {:?} supersedes nothing: {what}", row.path),
                ));
            }
            1 => {}
            n => {
                // The path has several rows, so `supersedes` is set. The reader rejects a
                // repeated path and reason within one table, so the rows left are one of each
                // table.
                let hint = "add `kind = \"excluded\"` or `kind = \"rejected\"`";
                return Err(Error::new(
                    path,
                    at,
                    format!(
                        "resolved row for {:?} is ambiguous: it matches {n} rows with that path ({}); {hint}",
                        row.path,
                        matched
                            .iter()
                            .map(|(kind, _, _)| kind.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
        }
        let (kind, _, reason) = matched[0];
        if !taken.insert((*kind, row.path.as_str(), *reason)) {
            return Err(Error::new(
                path,
                at,
                format!(
                    "duplicate resolved row for {:?}: another `resolved` row already supersedes that row",
                    row.path
                ),
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
    fn a_resolved_row_that_matches_no_excluded_or_rejected_row_is_refused() {
        let path = Path::new("exclusions.toml");
        let orphan =
            "[[resolved]]\npath = \"json/a.json#x\"\nentries = [\"ref/a/x\"]\nreason = \"ships\"\n";
        let error = read_exclusions(path, orphan).unwrap_err();
        assert!(error.message.contains("supersedes nothing"), "{error}");

        // A row for another path than the excluded or rejected row's is still an orphan.
        for kind in ["excluded", "rejected"] {
            let records = if kind == "excluded" {
                "records = 1\n"
            } else {
                ""
            };
            let other = format!(
                "[[{kind}]]\npath = \"json/b.json#y\"\n{records}reason = \"r\"\n\n{orphan}"
            );
            let error = read_exclusions(path, &other).unwrap_err();
            assert!(error.message.contains("supersedes nothing"), "{error}");
        }

        // The reader looks at the whole file, so a `resolved` row that comes first still finds
        // its row. Appending last is a convention.
        let first = "[[resolved]]\npath = \"json/a.json#x\"\nentries = [\"ref/a/x\"]\nreason = \"ships\"\n\n[[rejected]]\npath = \"json/a.json#x\"\nreason = \"r\"\n";
        assert!(read_exclusions(path, first).is_ok());
    }

    #[test]
    fn a_resolved_row_supersedes_its_rejected_row() {
        let path = Path::new("exclusions.toml");
        let old = "[[rejected]]\npath = \"json/domains.json#x\"\nreason = \"converter\"\n\n[[rejected]]\npath = \"json/domains.json#y\"\nreason = \"converter\"\n\n[[excluded]]\npath = \"json/b.json#z\"\nrecords = 1\nreason = \"art\"\n";
        let before = read_exclusions(path, old).unwrap();
        assert_eq!(before.active_rejected(), before.rejected);
        assert_eq!(before.active_excluded(), before.excluded);
        assert!(before.resolved_rows().is_empty());

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
        let resolved = after.resolved_rows();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].path, "json/domains.json#x");
        assert_eq!(resolved[0].was, "rejected");
        assert_eq!(resolved[0].entries, vec!["ref/domain/x".to_string()]);
        assert_eq!(
            read_exclusions(path, &write_exclusions(&after)).unwrap(),
            after
        );
    }

    /// The pinned corpus has two `rejected` rows for `json/main04.json#traits_race`, and the
    /// Death and Repose domains have an `excluded` and a `rejected` row for one path.
    #[test]
    fn a_resolved_row_names_which_of_several_rows_with_its_path_it_supersedes() {
        let path = Path::new("exclusions.toml");
        let death_blog = "Paizo Blog: an adjustment";
        let death_native = "converter native exclusion (ref/domain/death): bad directive";
        let race_native = "converter native exclusion (ref/main/race): bad";
        let race_rule = "converter rule exclusion (ref/rule/race/core): skipped";
        let old = format!(
            "[[excluded]]\npath = \"json/d.json#death\"\nrecords = 1\nreason = {death_blog:?}\n\n[[rejected]]\npath = \"json/d.json#death\"\nreason = {death_native:?}\n\n[[rejected]]\npath = \"json/m.json#race\"\nreason = {race_native:?}\n\n[[rejected]]\npath = \"json/m.json#race\"\nreason = {race_rule:?}\n"
        );
        let row = |path: &str, supersedes: Option<&str>, kind: Option<&str>| {
            let supersedes = supersedes.map_or(String::new(), |s| format!("supersedes = {s:?}\n"));
            let kind = kind.map_or(String::new(), |k| format!("kind = {k:?}\n"));
            format!(
                "\n[[resolved]]\npath = {path:?}\n{supersedes}{kind}entries = [\"ref/x/x\"]\nreason = \"ships now\"\n"
            )
        };
        let read = |extra: &str| read_exclusions(path, &format!("{old}{extra}"));

        // The path alone is ambiguous, across kinds and within one kind. The message says to
        // add `supersedes` with the exact full reason.
        for shared in ["json/d.json#death", "json/m.json#race"] {
            let error = read(&row(shared, None, None)).unwrap_err();
            assert!(error.message.contains("is ambiguous"), "{error}");
            assert!(error.message.contains("add `supersedes`"), "{error}");
            assert!(error.message.contains("exact full `reason`"), "{error}");
        }
        // A start of the reason is not enough: `supersedes` is the whole reason, copied exactly.
        for partial in ["converter ", "converter rule exclusion", "Paizo Blog", " "] {
            let error = read(&row("json/m.json#race", Some(partial), None)).unwrap_err();
            assert!(
                error.message.contains("supersedes nothing") || error.message.contains("empty"),
                "{error}"
            );
        }
        let error = read(&row("json/d.json#death", Some("nothing like it"), None)).unwrap_err();
        assert!(error.message.contains("supersedes nothing"), "{error}");
        assert!(error.message.contains("exact, full `reason`"), "{error}");
        // On a shared path `kind` alone selects nothing, even when it would pick one row.
        for (shared, kind) in [
            ("json/m.json#race", "excluded"),
            ("json/m.json#race", "rejected"),
            ("json/d.json#death", "excluded"),
            ("json/d.json#death", "rejected"),
        ] {
            let error = read(&row(shared, None, Some(kind))).unwrap_err();
            assert!(error.message.contains("is ambiguous"), "{error}");
            assert!(error.message.contains("add `supersedes`"), "{error}");
        }

        // Naming the rejected row resolves it and leaves the excluded row with the same path.
        let death = read(&row("json/d.json#death", Some(death_native), None)).unwrap();
        assert_eq!(death.active_rejected().len(), 2);
        assert!(
            death
                .active_rejected()
                .iter()
                .all(|r| r.path != "json/d.json#death")
        );
        assert_eq!(death.active_excluded(), death.excluded);
        assert_eq!(death.resolved_rows()[0].was, "rejected");

        // Naming the excluded row resolves that one and leaves the rejected row.
        let blog = read(&row("json/d.json#death", Some(death_blog), None)).unwrap();
        assert!(blog.active_excluded().is_empty());
        assert_eq!(blog.active_rejected(), blog.rejected);
        assert_eq!(blog.resolved_rows()[0].was, "excluded");

        // Both rows of the Death path, each with its own resolved row (what data PR #106 needs).
        let both_death = read(&format!(
            "{}{}",
            row("json/d.json#death", Some(death_native), None),
            row("json/d.json#death", Some(death_blog), None)
        ))
        .unwrap();
        assert!(both_death.active_excluded().is_empty());
        assert!(
            both_death
                .active_rejected()
                .iter()
                .all(|r| r.path != "json/d.json#death")
        );
        assert_eq!(both_death.resolved_rows().len(), 2);

        // Two rows with the same path in one kind: each is resolved on its own.
        let derived = read(&row("json/m.json#race", Some(race_rule), None)).unwrap();
        let left = derived.active_rejected();
        assert_eq!(left.len(), 2);
        assert!(
            left.iter()
                .any(|r| r.path == "json/m.json#race" && r.reason == race_native)
        );
        assert!(!left.iter().any(|r| r.reason == race_rule));
        let both = read(&format!(
            "{}{}",
            row("json/m.json#race", Some(race_rule), None),
            row("json/m.json#race", Some(race_native), Some("rejected"))
        ))
        .unwrap();
        assert_eq!(both.active_rejected().len(), 1);
        assert_eq!(both.resolved_rows().len(), 2);

        // Two rows that pick the same row repeat each other.
        let error = read(&format!(
            "{}{}",
            row("json/m.json#race", Some(race_rule), None),
            row("json/m.json#race", Some(race_rule), Some("rejected"))
        ))
        .unwrap_err();
        assert!(error.message.contains("duplicate resolved"), "{error}");

        // The keys are optional and additive: a row without them writes back without them.
        let plain = "[[rejected]]\npath = \"json/a.json#x\"\nreason = \"r\"\n\n[[resolved]]\npath = \"json/a.json#x\"\nentries = [\"ref/a/x\"]\nreason = \"ships\"\n";
        let written = write_exclusions(&read_exclusions(path, plain).unwrap());
        assert!(!written.contains("supersedes") && !written.contains("kind"));
    }

    /// The selection rule, cell by cell (docs/corpus-format.md has the same table): rows with the
    /// path (N) against the keys the `resolved` row gives.
    #[test]
    fn the_selection_rule_holds_in_every_cell() {
        let path = Path::new("exclusions.toml");
        let excluded = "[[excluded]]\npath = \"json/d.json#x\"\nrecords = 1\nreason = \"blog\"\n\n";
        let rejected = "[[rejected]]\npath = \"json/d.json#x\"\nreason = \"converter\"\n\n";
        let rejected_more =
            "[[rejected]]\npath = \"json/d.json#x\"\nreason = \"converter again\"\n\n";
        let row = |supersedes: Option<&str>, kind: Option<&str>| {
            let supersedes = supersedes.map_or(String::new(), |s| format!("supersedes = {s:?}\n"));
            let kind = kind.map_or(String::new(), |k| format!("kind = {k:?}\n"));
            format!(
                "[[resolved]]\npath = \"json/d.json#x\"\n{supersedes}{kind}entries = [\"ref/x/x\"]\nreason = \"ships\"\n"
            )
        };
        let read = |old: &str, supersedes: Option<&str>, kind: Option<&str>| {
            read_exclusions(path, &format!("{old}{}", row(supersedes, kind)))
        };
        let err = |old: &str, supersedes: Option<&str>, kind: Option<&str>, expect: &str| {
            let error = read(old, supersedes, kind).unwrap_err();
            assert!(error.message.contains(expect), "{expect}: {error}");
        };

        // N = 0: always refused.
        let error = read_exclusions(
            path,
            "[[resolved]]\npath = \"json/none.json#x\"\nentries = [\"ref/x/x\"]\nreason = \"ships\"\n",
        )
        .unwrap_err();
        assert!(error.message.contains("supersedes nothing"), "{error}");

        // N = 1, no keys: the unique-path shorthand.
        assert!(read(rejected, None, None).is_ok());
        assert!(read(excluded, None, None).is_ok());
        // N = 1 with keys: each key present must match the row.
        assert!(read(rejected, Some("converter"), None).is_ok());
        assert!(read(rejected, Some("converter"), Some("rejected")).is_ok());
        assert!(read(rejected, None, Some("rejected")).is_ok());
        err(rejected, Some("conv"), None, "supersedes nothing");
        err(rejected, None, Some("excluded"), "supersedes nothing");
        err(rejected, Some("blog"), None, "supersedes nothing");

        // N >= 2 (both tables, or one table): `supersedes` is required, `kind` alone is not enough.
        for old in [
            format!("{excluded}{rejected}"),
            format!("{rejected}{rejected_more}"),
        ] {
            err(&old, None, None, "add `supersedes`");
            err(&old, None, Some("rejected"), "add `supersedes`");
            err(&old, None, Some("excluded"), "add `supersedes`");
            // With the exact full reason, one row is selected.
            assert!(read(&old, Some("converter"), None).is_ok());
            assert!(read(&old, Some("converter"), Some("rejected")).is_ok());
            err(&old, Some("conv"), None, "supersedes nothing");
        }
        // A reason in the other table is named as such.
        let both = format!("{excluded}{rejected}");
        err(
            &both,
            Some("converter"),
            Some("excluded"),
            "exists in the rejected table",
        );
        err(
            &both,
            Some("blog"),
            Some("rejected"),
            "exists in the excluded table",
        );
        assert!(read(&both, Some("blog"), Some("excluded")).is_ok());
        // A `kind` names no row of the table and the reason is in no table.
        err(
            &both,
            Some("nothing"),
            Some("excluded"),
            "exact, full `reason`",
        );
        // Within one table, a `kind` that names the other table has no row for the reason.
        let two = format!("{rejected}{rejected_more}");
        err(
            &two,
            Some("converter"),
            Some("excluded"),
            "supersedes nothing",
        );

        // The same path and reason in both tables: `supersedes` alone is ambiguous.
        let same = "[[excluded]]\npath = \"json/d.json#x\"\nrecords = 1\nreason = \"same\"\n\n[[rejected]]\npath = \"json/d.json#x\"\nreason = \"same\"\n\n";
        err(same, None, Some("rejected"), "add `supersedes`");
        err(same, Some("same"), None, "add `kind");
        assert!(read(same, Some("same"), Some("excluded")).is_ok());
    }

    /// A row can have a blank or whitespace-only reason. It is selected by the exact string.
    #[test]
    fn a_row_with_a_blank_reason_can_be_resolved() {
        let path = Path::new("exclusions.toml");
        for blank in ["", " ", "  \t"] {
            let old = format!(
                "[[rejected]]\npath = \"json/d.json#x\"\nreason = {blank:?}\n\n[[rejected]]\npath = \"json/d.json#x\"\nreason = \"converter error\"\n\n"
            );
            let row = |supersedes: &str| {
                format!(
                    "[[resolved]]\npath = \"json/d.json#x\"\nsupersedes = {supersedes:?}\nentries = [\"ref/x/x\"]\nreason = \"ships\"\n"
                )
            };
            let read =
                |supersedes: &str| read_exclusions(path, &format!("{old}{}", row(supersedes)));
            let blank_row = read(blank).unwrap();
            let left = blank_row.active_rejected();
            assert_eq!(left.len(), 1, "{blank:?}");
            assert_eq!(left[0].reason, "converter error");
            assert_eq!(blank_row.resolved_rows()[0].was, "rejected");
            // The other row is selected by its own reason, and blank is not a prefix match.
            let other = read("converter error").unwrap();
            assert_eq!(other.active_rejected()[0].reason, blank);
            // A different blank string matches nothing.
            let other_blank = if blank.is_empty() { " " } else { "" };
            let error = read(other_blank).unwrap_err();
            assert!(error.message.contains("supersedes nothing"), "{error}");
        }
        // A blank `supersedes` for a row with a non-blank reason matches nothing.
        let old = "[[rejected]]\npath = \"json/d.json#x\"\nreason = \"converter\"\n\n[[resolved]]\npath = \"json/d.json#x\"\nsupersedes = \"\"\nentries = [\"ref/x/x\"]\nreason = \"ships\"\n";
        let error = read_exclusions(path, old).unwrap_err();
        assert!(error.message.contains("supersedes nothing"), "{error}");
    }

    /// A reason that is a whole prefix of another reason on the same path: a prefix `supersedes`
    /// could never select the shorter row. The full reason selects either one.
    #[test]
    fn a_reason_that_starts_another_reason_on_the_same_path_can_be_resolved() {
        let path = Path::new("exclusions.toml");
        let old = "[[rejected]]\npath = \"json/d.json#x\"\nreason = \"unsupported directive\"\n\n[[rejected]]\npath = \"json/d.json#x\"\nreason = \"unsupported directive @list\"\n";
        let row = |supersedes: &str| {
            format!(
                "\n[[resolved]]\npath = \"json/d.json#x\"\nsupersedes = {supersedes:?}\nentries = [\"ref/x/x\"]\nreason = \"ships\"\n"
            )
        };
        let short =
            read_exclusions(path, &format!("{old}{}", row("unsupported directive"))).unwrap();
        let left = short.active_rejected();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].reason, "unsupported directive @list");
        let long = read_exclusions(
            path,
            &format!("{old}{}", row("unsupported directive @list")),
        )
        .unwrap();
        let left = long.active_rejected();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].reason, "unsupported directive");
        // A longer reason appended later cannot make the shorter selector ambiguous.
        let later = format!(
            "{old}\n[[rejected]]\npath = \"json/d.json#x\"\nreason = \"unsupported directive @list (nested)\"\n{}",
            row("unsupported directive @list")
        );
        assert_eq!(
            read_exclusions(path, &later)
                .unwrap()
                .active_rejected()
                .len(),
            2
        );
    }

    /// The same path and reason in both tables are told apart by `kind`.
    #[test]
    fn kind_tells_apart_the_same_path_and_reason_in_both_tables() {
        let path = Path::new("exclusions.toml");
        let old = "[[excluded]]\npath = \"json/d.json#x\"\nrecords = 1\nreason = \"same\"\n\n[[rejected]]\npath = \"json/d.json#x\"\nreason = \"same\"\n";
        let row = |kind: Option<&str>| {
            let kind = kind.map_or(String::new(), |k| format!("kind = {k:?}\n"));
            format!(
                "\n[[resolved]]\npath = \"json/d.json#x\"\nsupersedes = \"same\"\n{kind}entries = [\"ref/x/x\"]\nreason = \"ships\"\n"
            )
        };
        let error = read_exclusions(path, &format!("{old}{}", row(None))).unwrap_err();
        assert!(error.message.contains("is ambiguous"), "{error}");
        assert!(error.message.contains("add `kind"), "{error}");
        let rejected = read_exclusions(path, &format!("{old}{}", row(Some("rejected")))).unwrap();
        assert!(rejected.active_rejected().is_empty());
        assert_eq!(rejected.active_excluded().len(), 1);
        assert_eq!(rejected.resolved_rows()[0].was, "rejected");
        let excluded = read_exclusions(path, &format!("{old}{}", row(Some("excluded")))).unwrap();
        assert!(excluded.active_excluded().is_empty());
        assert_eq!(excluded.active_rejected().len(), 1);
        assert_eq!(excluded.resolved_rows()[0].was, "excluded");
        let written = write_exclusions(&excluded);
        assert!(written.contains("kind = \"excluded\""), "{written}");
        assert_eq!(read_exclusions(path, &written).unwrap(), excluded);
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

        // The same holds when the row is a rejected row.
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
