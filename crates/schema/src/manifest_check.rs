//! The structural check of a content manifest, shared by the app's installer and `cargo xtask`.
//!
//! It reads only the manifest: the schema version, the version string, every path, checksum
//! and total, and the files the reader cannot work without. It never reads a file.

use crate::{ContentManifest, SCHEMA_VERSION};
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

/// Checks a manifest and says what is wrong with it, before any download starts.
pub fn validate_manifest(manifest: &ContentManifest) -> Result<(), PlanError> {
    // Schema 4 adds attached-entry kinds, relation labels, and two derived index files. The
    // reader still opens schema-1, schema-2, and schema-3 libraries; a newer library shows the
    // update route instead of a parse error.
    if ![1, 2, 3, SCHEMA_VERSION].contains(&manifest.schema_version) {
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
        total = total.saturating_add(file.bytes);
    }
    for required in REQUIRED_FILES {
        let path = format!("{version}/{required}");
        if !seen.contains(path.as_str()) {
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
