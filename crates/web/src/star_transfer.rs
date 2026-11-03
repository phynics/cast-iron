//! Portable, versioned JSON archives for moving stars between isolated profiles.
use cast_iron_schema::{EntryId, EntryKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const FORMAT: &str = "cast-iron-stars";
pub const VERSION: u32 = 1;
pub const MAX_ARCHIVE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_RECORDS: usize = 10_000;
pub const MAX_TITLE_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferError(pub String);
impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for TransferError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StarRecord {
    pub entry_id: EntryId,
    pub title: String,
    pub kind: EntryKind,
    pub added_at: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StarArchive {
    pub format: String,
    pub version: u32,
    pub stars: Vec<StarRecord>,
}

pub fn export_archive(bookmarks: &[crate::store::Bookmark]) -> Result<String, TransferError> {
    if bookmarks.len() > MAX_RECORDS {
        return Err(TransferError("There are too many stars to export.".into()));
    }
    let stars = bookmarks
        .iter()
        .map(|b| StarRecord {
            entry_id: b.entry_id.clone(),
            title: b.title_snapshot.clone(),
            kind: b.kind,
            added_at: b.added_at,
        })
        .collect();
    let archive = StarArchive {
        format: FORMAT.into(),
        version: VERSION,
        stars,
    };
    encode_archive(&archive)
}

fn encode_archive(archive: &StarArchive) -> Result<String, TransferError> {
    let json = serde_json::to_string(archive)
        .map_err(|_| TransferError("Could not create the stars file.".into()))?;
    if json.len() > MAX_ARCHIVE_BYTES {
        return Err(TransferError(
            "The stars file exceeds the 2 MB limit.".into(),
        ));
    }
    Ok(json)
}

/// Parse and validate the complete archive before the caller begins any storage write.
pub fn parse_archive(json: &str) -> Result<Vec<StarRecord>, TransferError> {
    if json.len() > MAX_ARCHIVE_BYTES {
        return Err(TransferError(
            "This file is larger than the 2 MB limit.".into(),
        ));
    }
    let archive: StarArchive = serde_json::from_str(json)
        .map_err(|_| TransferError("This is not a valid Cast Iron stars file.".into()))?;
    if archive.format != FORMAT {
        return Err(TransferError(
            "This file is not a Cast Iron stars archive.".into(),
        ));
    }
    if archive.version != VERSION {
        return Err(TransferError(format!(
            "Stars file version {} is not supported.",
            archive.version
        )));
    }
    if archive.stars.is_empty() {
        return Err(TransferError("This file contains no stars.".into()));
    }
    if archive.stars.len() > MAX_RECORDS {
        return Err(TransferError(
            "This file contains too many stars (limit: 10,000).".into(),
        ));
    }
    let mut ids = BTreeSet::new();
    for star in &archive.stars {
        if star.title.trim().is_empty() || star.title.len() > MAX_TITLE_BYTES {
            return Err(TransferError(
                "A star has an empty or oversized title.".into(),
            ));
        }
        if !star.added_at.is_finite() || star.added_at < 0.0 {
            return Err(TransferError("A star has an invalid saved date.".into()));
        }
        if !ids.insert(star.entry_id.clone()) {
            return Err(TransferError(
                "This file contains duplicate entry IDs.".into(),
            ));
        }
    }
    // Numeric normalization can enlarge an otherwise compact input. Accept only files
    // that also fit when serialized in the exact format used by export.
    encode_archive(&archive)?;
    Ok(archive.stars)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn star(id: &str, title: &str) -> StarRecord {
        StarRecord {
            entry_id: EntryId::parse(id).unwrap(),
            title: title.into(),
            kind: EntryKind::Rule,
            added_at: 1.0,
        }
    }
    fn json(stars: Vec<StarRecord>, version: u32) -> String {
        serde_json::to_string(&StarArchive {
            format: FORMAT.into(),
            version,
            stars,
        })
        .unwrap()
    }
    #[test]
    fn valid_roundtrip_and_collection_ids_are_not_exported() {
        let b = crate::store::Bookmark {
            entry_id: EntryId::parse("a").unwrap(),
            title_snapshot: "A".into(),
            kind: EntryKind::Rule,
            added_at: 1.0,
            collection_ids: vec!["retired".into()],
        };
        let data = export_archive(&[b]).unwrap();
        assert!(!data.contains("retired"));
        assert_eq!(parse_archive(&data).unwrap(), vec![star("a", "A")]);
    }
    #[test]
    fn near_limit_compact_archive_round_trips_without_pretty_print_expansion() {
        let title = "x".repeat(MAX_TITLE_BYTES);
        let incoming = (0..3400)
            .map(|i| star(&format!("entry/{i}"), &title))
            .collect::<Vec<_>>();
        let compact = json(incoming.clone(), VERSION);
        assert!(compact.len() > 1_900_000 && compact.len() <= MAX_ARCHIVE_BYTES);
        let archive: StarArchive = serde_json::from_str(&compact).unwrap();
        assert!(serde_json::to_string_pretty(&archive).unwrap().len() > MAX_ARCHIVE_BYTES);
        let accepted = parse_archive(&compact).unwrap();
        let bookmarks = accepted
            .into_iter()
            .map(|s| crate::store::Bookmark::new(s.entry_id, s.title, s.kind, s.added_at))
            .collect::<Vec<_>>();
        let exported = export_archive(&bookmarks).unwrap();
        assert_eq!(parse_archive(&exported).unwrap(), incoming);
        assert_eq!(exported, compact);
    }

    #[test]
    fn canonical_numeric_expansion_is_checked_before_import() {
        let title = "x".repeat(MAX_TITLE_BYTES);
        let mut incoming = (0..3400)
            .map(|i| star(&format!("entry/{i}"), &title))
            .collect::<Vec<_>>();
        let size = json(incoming.clone(), VERSION).len();
        // Add valid ID characters until canonical encoding exceeds the limit by one byte.
        // Compact integer dates save two bytes per record and still fit on input.
        let needed = MAX_ARCHIVE_BYTES + 1 - size;
        for (i, record) in incoming.iter_mut().enumerate() {
            let extra = needed / 3400 + usize::from(i < needed % 3400);
            record.entry_id = EntryId::parse(format!("entry/{i}{}", "x".repeat(extra))).unwrap();
        }
        let canonical = json(incoming, VERSION);
        assert_eq!(canonical.len(), MAX_ARCHIVE_BYTES + 1);
        let input = canonical.replace("\"added_at\":1.0", "\"added_at\":1");
        assert!(input.len() < MAX_ARCHIVE_BYTES);
        assert!(parse_archive(&input).unwrap_err().0.contains("2 MB"));
    }

    #[test]
    fn unsupported_version_rejected() {
        assert!(
            parse_archive(&json(vec![star("a", "A")], 2))
                .unwrap_err()
                .0
                .contains("not supported")
        );
    }
    #[test]
    fn invalid_archive_rejected_before_any_write_plan() {
        let malformed = r#"{"format":"cast-iron-stars","version":1,"stars":[{"entry_id":"BAD ID","title":"bad","kind":"rule","added_at":1.0}]}"#;
        assert!(parse_archive(malformed).is_err());
    }
    #[test]
    fn limits_and_nan_rejected() {
        assert!(parse_archive(&" ".repeat(MAX_ARCHIVE_BYTES + 1)).is_err());
        assert!(parse_archive(&json(vec![star("a", " ")], 1)).is_err());
    }
}
