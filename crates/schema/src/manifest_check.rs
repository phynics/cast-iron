//! The structural check of a content manifest, shared by the app's installer and `cargo xtask`.
//!
//! It reads only the manifest: the schema version, the version string, every path, checksum
//! and total, and the files the reader cannot work without. It never reads a file.

use crate::{ContentManifest, ManifestFile, SCHEMA_VERSION};
use std::collections::BTreeSet;
use std::fmt;

/// Files that the reader cannot work without, relative to the version directory.
pub const REQUIRED_FILES: [&str; 4] = ["catalog.json", "legal.json", "coverage.json", "search.bin"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// The library was built for another content format than this app reads.
    SchemaMismatch {
        found: u32,
        expected: u32,
    },
    BadVersion(String),
    BadPath(String),
    BadHash(String),
    DuplicatePath(String),
    Missing(String),
    TotalMismatch {
        declared: u64,
        actual: u64,
    },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::SchemaMismatch { found, expected } if found > expected => write!(
                f,
                "This library uses content format {found}, but this version of the app reads format {expected}. Reload the app to get the latest version, then try again."
            ),
            PlanError::SchemaMismatch { found, expected } => write!(
                f,
                "This library uses the older content format {found}, but this version of the app reads format {expected}. The library needs to be republished."
            ),
            PlanError::BadVersion(v) => {
                write!(f, "The library manifest has an invalid version {v:?}.")
            }
            PlanError::BadPath(p) => {
                write!(f, "The library manifest lists an invalid file path {p:?}.")
            }
            PlanError::BadHash(p) => write!(
                f,
                "The library manifest lists an invalid checksum for {p:?}."
            ),
            PlanError::DuplicatePath(p) => write!(f, "The library manifest lists {p:?} twice."),
            PlanError::Missing(p) => write!(
                f,
                "The library manifest does not list the required file {p:?}."
            ),
            PlanError::TotalMismatch { declared, actual } => write!(
                f,
                "The library manifest declares {declared} bytes, but its files add up to {actual}."
            ),
        }
    }
}

impl std::error::Error for PlanError {}

/// The content formats a reader opens: the current one and every older one it still reads. An
/// installed library of an older format stays valid and opens (AGENTS.md rule 7); only a format
/// outside this list is refused. Add a number here, never remove one, when `SCHEMA_VERSION` rises.
///
/// Schema 4 adds attached-entry kinds, relation labels, and two derived index files. Schema 5 adds
/// the member books of each catalog stub (a schema-4 stub has no `books`, and the entry then
/// belongs to its `source` alone). Schema 6 moves the trades of archetypes and class options into
/// `trades.json`; a schema-4 or schema-5 library keeps `replaces.json`, and the reader falls back
/// to it. Schema 7 adds the haunt kind and changes nothing else. Schema 8 stores every file but the
/// manifest as gzip and adds the `gzip` key to each manifest row; a row without it is read as
/// stored, so every older library opens as it is.
pub const READABLE_SCHEMAS: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, SCHEMA_VERSION];

/// Checks a manifest and says what is wrong with it, before any download starts.
pub fn validate_manifest(manifest: &ContentManifest) -> Result<(), PlanError> {
    // A newer library shows the update route instead of a parse error.
    if !READABLE_SCHEMAS.contains(&manifest.schema_version) {
        return Err(PlanError::SchemaMismatch {
            found: manifest.schema_version,
            expected: SCHEMA_VERSION,
        });
    }
    let version = &manifest.content_version;
    let version_ok = version.len() > 1
        && version.starts_with('v')
        && version[1..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase());
    if !version_ok {
        return Err(PlanError::BadVersion(version.clone()));
    }
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    for file in &manifest.files {
        if !path_is_safe(&file.path, version) {
            return Err(PlanError::BadPath(file.path.clone()));
        }
        if file.gzip.is_some()
            && !file
                .path
                .strip_suffix(".gz")
                .is_some_and(|path| path.rsplit('/').next().is_some_and(|name| !name.is_empty()))
        {
            return Err(PlanError::BadPath(file.path.clone()));
        }
        let sha_ok = file.sha256.len() == 64
            && file
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !sha_ok {
            return Err(PlanError::BadHash(file.path.clone()));
        }
        if !seen.insert(file.path.as_str()) {
            return Err(PlanError::DuplicatePath(file.path.clone()));
        }
        if let Some(decoded) = &file.gzip {
            let decoded_hash_ok = decoded.sha256.len() == 64
                && decoded
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if !decoded_hash_ok {
                return Err(PlanError::BadHash(file.path.clone()));
            }
        }
        total = total.saturating_add(file.bytes);
    }
    let mut logical_paths: Vec<&str> = manifest
        .files
        .iter()
        .map(ManifestFile::logical_path)
        .collect();
    logical_paths.sort_unstable();
    if let Some(duplicate) = logical_paths.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(PlanError::DuplicatePath(duplicate[0].to_owned()));
    }
    let logical_paths: BTreeSet<&str> = logical_paths.into_iter().collect();
    for required in REQUIRED_FILES {
        let path = format!("{version}/{required}");
        if !logical_paths.contains(path.as_str()) {
            return Err(PlanError::Missing(path));
        }
    }
    if total != manifest.total_bytes {
        return Err(PlanError::TotalMismatch {
            declared: manifest.total_bytes,
            actual: total,
        });
    }
    Ok(())
}

/// A manifest path must sit under its own version directory and cannot climb out of it.
fn path_is_safe(path: &str, version: &str) -> bool {
    let Some(rest) = path
        .strip_prefix(version)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return false;
    };
    !rest.is_empty()
        && rest.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ManifestFile;

    fn manifest(schema_version: u32) -> ContentManifest {
        let v = "v0123456789abcdef";
        let file = |name: &str, bytes: u64| ManifestFile {
            path: format!("{v}/{name}"),
            sha256: "ab".repeat(32),
            bytes,
            gzip: None,
        };
        let mut files: Vec<ManifestFile> = REQUIRED_FILES.iter().map(|n| file(n, 100)).collect();
        files.push(file("entries/spell-000.json", 600));
        ContentManifest {
            schema_version,
            content_version: v.into(),
            total_bytes: 1000,
            files,
            entry_count: 7,
            datasets: vec![],
            corpus_tag: None,
        }
    }

    #[test]
    fn the_current_schema_and_every_older_one_validate() {
        // Schema 4 and 5 are the formats of libraries that are installed today; they must keep
        // validating and opening after the reader moves to schema 8.
        assert_eq!(SCHEMA_VERSION, 8);
        for version in [1, 2, 3, 4, 5, 6, 7, 8] {
            assert_eq!(
                validate_manifest(&manifest(version)),
                Ok(()),
                "schema {version}"
            );
        }
        assert_eq!(READABLE_SCHEMAS.last().copied(), Some(SCHEMA_VERSION));
    }

    #[test]
    fn a_schema_outside_the_readable_list_is_refused_with_its_direction() {
        let newer = validate_manifest(&manifest(SCHEMA_VERSION + 1)).unwrap_err();
        assert!(matches!(
            newer,
            PlanError::SchemaMismatch { found, expected } if found == SCHEMA_VERSION + 1 && expected == SCHEMA_VERSION
        ));
        assert!(newer.to_string().contains("Reload the app"), "{newer}");
        let zero = validate_manifest(&manifest(0)).unwrap_err();
        assert!(zero.to_string().contains("older"), "{zero}");
    }

    #[test]
    fn legacy_manifest_round_trips_without_changing_its_json() {
        let input = br#"{"schema_version":7,"content_version":"v0123456789abcdef","total_bytes":1000,"files":[{"path":"v0123456789abcdef/catalog.json","sha256":"abababababababababababababababababababababababababababababababab","bytes":100}],"entry_count":0,"datasets":[]}"#;
        let parsed: ContentManifest = serde_json::from_slice(input).unwrap();
        assert_eq!(serde_json::to_vec(&parsed).unwrap(), input);
    }

    #[test]
    fn valid_gzip_rows_use_logical_paths_for_required_files() {
        let mut m = manifest(SCHEMA_VERSION);
        m.files[0].path.push_str(".gz");
        m.files[0].gzip = Some(crate::Decoded {
            sha256: "cd".repeat(32),
            bytes: 200,
        });
        assert_eq!(m.files[0].logical_path(), "v0123456789abcdef/catalog.json");
        assert_eq!(validate_manifest(&m), Ok(()));
    }

    #[test]
    fn gzip_path_without_suffix_is_rejected() {
        let mut m = manifest(SCHEMA_VERSION);
        m.files[0].gzip = Some(crate::Decoded {
            sha256: "cd".repeat(32),
            bytes: 100,
        });
        assert!(matches!(validate_manifest(&m), Err(PlanError::BadPath(_))));
        let error = validate_manifest(&m).unwrap_err();
        assert!(error.to_string().contains("invalid file path"), "{error}");
    }

    #[test]
    fn gzip_path_with_empty_name_is_rejected() {
        let mut m = manifest(SCHEMA_VERSION);
        m.files[0].path = "v0123456789abcdef/.gz".into();
        m.files[0].gzip = Some(crate::Decoded {
            sha256: "cd".repeat(32),
            bytes: 100,
        });
        assert!(matches!(validate_manifest(&m), Err(PlanError::BadPath(_))));
        let error = validate_manifest(&m).unwrap_err();
        assert!(error.to_string().contains("invalid file path"), "{error}");
    }

    #[test]
    fn invalid_decoded_hash_is_rejected() {
        let mut m = manifest(SCHEMA_VERSION);
        m.files[0].path.push_str(".gz");
        m.files[0].gzip = Some(crate::Decoded {
            sha256: "zz".repeat(32),
            bytes: 100,
        });
        assert!(matches!(validate_manifest(&m), Err(PlanError::BadHash(_))));
        let error = validate_manifest(&m).unwrap_err();
        assert!(error.to_string().contains("invalid checksum"), "{error}");
    }

    #[test]
    fn duplicate_logical_paths_are_rejected() {
        let mut m = manifest(SCHEMA_VERSION);
        let mut compressed = m.files[0].clone();
        compressed.path.push_str(".gz");
        compressed.gzip = Some(crate::Decoded {
            sha256: "cd".repeat(32),
            bytes: 100,
        });
        m.files.push(compressed);
        m.total_bytes += 100;
        assert!(matches!(
            validate_manifest(&m),
            Err(PlanError::DuplicatePath(_))
        ));
        let error = validate_manifest(&m).unwrap_err();
        assert!(
            error.to_string().contains("v0123456789abcdef/catalog.json"),
            "{error}"
        );
    }
}
