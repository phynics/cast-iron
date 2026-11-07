//! The pure core of the installer: manifest validation, work scheduling, progress,
//! the quota check, and the verification rule. No web types, so it is tested natively.

use cast_iron_schema::{ContentManifest, ManifestFile, SCHEMA_VERSION};
use std::collections::BTreeSet;
use std::fmt;

/// Files that download at once.
pub const CONCURRENCY: usize = 3;
/// Tries per file before the install fails.
pub const MAX_ATTEMPTS: u32 = 3;
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

/// Why downloaded bytes are not the bytes that the manifest promised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    Size { expected: u64, actual: u64 },
    Hash { expected: String, actual: String },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::Size { expected, actual } => {
                write!(f, "expected {expected} bytes, got {actual}")
            }
            VerifyError::Hash { expected, .. } => {
                write!(
                    f,
                    "checksum mismatch (expected {})",
                    &expected[..expected.len().min(12)]
                )
            }
        }
    }
}

/// The one rule behind every integrity check: byte count first, then SHA-256.
pub fn verify_file(
    file: &ManifestFile,
    actual_len: u64,
    actual_sha256: &str,
) -> Result<(), VerifyError> {
    if actual_len != file.bytes {
        return Err(VerifyError::Size {
            expected: file.bytes,
            actual: actual_len,
        });
    }
    if !actual_sha256.eq_ignore_ascii_case(&file.sha256) {
        return Err(VerifyError::Hash {
            expected: file.sha256.clone(),
            actual: actual_sha256.to_string(),
        });
    }
    Ok(())
}

/// Milliseconds to wait after failed attempt number `attempt` (1-based).
pub fn backoff_ms(attempt: u32) -> u32 {
    match attempt {
        0 | 1 => 500,
        2 => 1500,
        n => 1500 * (n - 1),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Progress {
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

impl Progress {
    /// Whole percent of bytes, 0 to 100.
    pub fn percent(&self) -> u8 {
        if self.bytes_total == 0 {
            return if self.files_done >= self.files_total {
                100
            } else {
                0
            };
        }
        ((self.bytes_done.min(self.bytes_total) as u128 * 100) / self.bytes_total as u128) as u8
    }

    /// One line for a status region, such as `3 of 24 files, 3.1 MB of 11.2 MB (28%)`.
    pub fn summary(&self) -> String {
        format!(
            "{} of {} files, {} of {} ({}%)",
            self.files_done,
            self.files_total,
            crate::fmt::bytes(self.bytes_done),
            crate::fmt::bytes(self.bytes_total),
            self.percent()
        )
    }
}

/// Work scheduling over a validated manifest. Index `i` identifies `manifest.files[i]`.
#[derive(Debug, Clone)]
pub struct InstallPlan {
    manifest: ContentManifest,
    verified: Vec<bool>,
    claimed: Vec<bool>,
}

impl InstallPlan {
    pub fn new(manifest: ContentManifest) -> Result<Self, PlanError> {
        validate_manifest(&manifest)?;
        let n = manifest.files.len();
        Ok(Self {
            manifest,
            verified: vec![false; n],
            claimed: vec![false; n],
        })
    }

    pub fn manifest(&self) -> &ContentManifest {
        &self.manifest
    }

    pub fn file(&self, index: usize) -> &ManifestFile {
        &self.manifest.files[index]
    }

    pub fn file_count(&self) -> usize {
        self.manifest.files.len()
    }

    /// Records a file that is in the cache and verified, including one found there on resume.
    pub fn mark_verified(&mut self, index: usize) {
        self.verified[index] = true;
    }

    /// Hands out the next file that is neither verified nor taken, or `None` when none is left.
    pub fn claim_next(&mut self) -> Option<usize> {
        let index = (0..self.file_count()).find(|&i| !self.verified[i] && !self.claimed[i])?;
        self.claimed[index] = true;
        Some(index)
    }

    pub fn is_complete(&self) -> bool {
        self.verified.iter().all(|&done| done)
    }

    pub fn remaining_bytes(&self) -> u64 {
        self.manifest
            .files
            .iter()
            .zip(&self.verified)
            .filter(|(_, done)| !**done)
            .map(|(file, _)| file.bytes)
            .sum()
    }

    pub fn progress(&self) -> Progress {
        let done = self.verified.iter().filter(|&&d| d).count();
        Progress {
            files_done: done,
            files_total: self.file_count(),
            bytes_done: self.manifest.total_bytes - self.remaining_bytes(),
            bytes_total: self.manifest.total_bytes,
        }
    }
}

/// The browser could not promise room for the download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaShortfall {
    pub needed: u64,
    pub available: u64,
}

impl fmt::Display for QuotaShortfall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Not enough storage: the library needs about {} more, and the browser offers {}. Free some space and retry.",
            crate::fmt::bytes(self.needed),
            crate::fmt::bytes(self.available)
        )
    }
}

/// Compares the bytes still to download, plus 5% headroom, with the free quota.
/// Unknown usage or quota (some browsers hide them) passes: the write itself would then fail loudly.
pub fn check_space(
    needed: u64,
    usage: Option<f64>,
    quota: Option<f64>,
) -> Result<(), QuotaShortfall> {
    let (Some(usage), Some(quota)) = (usage, quota) else {
        return Ok(());
    };
    if quota <= 0.0 {
        return Ok(());
    }
    let available = (quota - usage).max(0.0) as u64;
    if needed + needed / 20 > available {
        Err(QuotaShortfall { needed, available })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// The manifest or a file could not be fetched.
    Network,
    /// The manifest is for another content format.
    Schema,
    Quota,
    /// A file did not match its size or checksum.
    Integrity,
    /// The Cache API or IndexedDB refused a write or read.
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
}

impl Failure {
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<PlanError> for Failure {
    fn from(error: PlanError) -> Self {
        let kind = match error {
            PlanError::SchemaMismatch { .. } => FailureKind::Schema,
            _ => FailureKind::Integrity,
        };
        Failure::new(kind, error.to_string())
    }
}

/// Where the installer is. The UI renders it; the driver moves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Fetching `content/manifest.json`.
    Checking,
    /// The manifest is valid and the user has not chosen to download yet.
    Offer,
    /// Looking at what the cache already holds.
    Scanning,
    Downloading,
    /// The final pass that re-reads and re-hashes every file.
    Verifying,
    Activating,
    Done,
    Failed(Failure),
}

impl Phase {
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Phase::Checking
                | Phase::Scanning
                | Phase::Downloading
                | Phase::Verifying
                | Phase::Activating
        )
    }

    pub fn label(&self) -> &'static str {
        match self {
            Phase::Checking => "Checking the library",
            Phase::Offer => "Ready to download",
            Phase::Scanning => "Checking what is already stored",
            Phase::Downloading => "Downloading",
            Phase::Verifying => "Verifying every file",
            Phase::Activating => "Activating",
            Phase::Done => "Ready for offline use",
            Phase::Failed(_) => "Installation stopped",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(version: &str, name: &str, bytes: u64) -> ManifestFile {
        ManifestFile {
            path: format!("{version}/{name}"),
            sha256: "ab".repeat(32),
            bytes,
        }
    }

    fn manifest() -> ContentManifest {
        let v = "v0123456789abcdef";
        let mut files: Vec<ManifestFile> = REQUIRED_FILES.iter().map(|n| file(v, n, 100)).collect();
        files.push(file(v, "entries/spell-000.json", 600));
        ContentManifest {
            schema_version: SCHEMA_VERSION,
            content_version: v.into(),
            total_bytes: 1000,
            files,
            entry_count: 7,
            datasets: vec![],
        }
    }

    #[test]
    fn a_good_manifest_validates() {
        assert_eq!(validate_manifest(&manifest()), Ok(()));
    }

    #[test]
    fn schema_one_to_three_libraries_remain_readable_during_the_upgrade() {
        let mut old = manifest();
        for version in [1, 2, 3] {
            old.schema_version = version;
            assert_eq!(validate_manifest(&old), Ok(()), "schema {version}");
        }
        old.schema_version = SCHEMA_VERSION;
        assert_eq!(validate_manifest(&old), Ok(()));
    }

    #[test]
    fn a_schema_three_library_opens_and_offers_an_update_rather_than_crashing() {
        // The active library a reader already installed stays usable; the update check compares
        // the published content version and offers the schema-4 library.
        let mut active = manifest();
        active.schema_version = 3;
        active.content_version = "v0000000000000003".into();
        for file in &mut active.files {
            file.path = file
                .path
                .replacen("v0123456789abcdef", &active.content_version, 1);
        }
        assert_eq!(validate_manifest(&active), Ok(()));
        assert!(active.schema_version < SCHEMA_VERSION);

        let mut published = active.clone();
        published.schema_version = SCHEMA_VERSION;
        published.content_version = "v0000000000000004".into();
        for file in &mut published.files {
            file.path = file
                .path
                .replacen(&active.content_version, &published.content_version, 1);
        }
        assert_eq!(validate_manifest(&published), Ok(()));
        assert_ne!(active.content_version, published.content_version);
    }

    #[test]
    fn other_schema_versions_are_rejected_with_a_message() {
        let mut m = manifest();
        m.schema_version = SCHEMA_VERSION + 1;
        let error = validate_manifest(&m).unwrap_err();
        assert!(matches!(error, PlanError::SchemaMismatch { .. }));
        assert!(error.to_string().contains("Reload the app"), "{error}");
        m.schema_version = 0;
        assert!(
            validate_manifest(&m)
                .unwrap_err()
                .to_string()
                .contains("older")
        );
        assert_eq!(
            Failure::from(validate_manifest(&m).unwrap_err()).kind,
            FailureKind::Schema
        );
    }

    #[test]
    fn unsafe_paths_hashes_and_versions_are_rejected() {
        let v = "v0123456789abcdef";
        for path in [
            format!("{v}/../x"),
            format!("{v}//x"),
            format!("{v}/a b"),
            "other/catalog.json".to_string(),
            format!("{v}/"),
            v.to_string(),
            format!("/{v}/x"),
            format!("{v}/a\\b"),
        ] {
            let mut m = manifest();
            m.files[0].path = path.clone();
            assert!(
                matches!(validate_manifest(&m), Err(PlanError::BadPath(_))),
                "{path}"
            );
        }
        let mut m = manifest();
        m.files[0].sha256 = "AB".repeat(32);
        assert!(matches!(validate_manifest(&m), Err(PlanError::BadHash(_))));
        let mut m = manifest();
        m.content_version = "../evil".into();
        assert!(matches!(
            validate_manifest(&m),
            Err(PlanError::BadVersion(_))
        ));
    }

    #[test]
    fn duplicates_missing_files_and_wrong_totals_are_rejected() {
        let mut m = manifest();
        m.files.push(m.files[0].clone());
        m.total_bytes += 100;
        assert!(matches!(
            validate_manifest(&m),
            Err(PlanError::DuplicatePath(_))
        ));
        let mut m = manifest();
        m.files.remove(3);
        m.total_bytes -= 100;
        assert!(matches!(validate_manifest(&m), Err(PlanError::Missing(_))));
        let mut m = manifest();
        m.total_bytes = 5;
        assert!(matches!(
            validate_manifest(&m),
            Err(PlanError::TotalMismatch { .. })
        ));
    }

    #[test]
    fn verification_checks_size_before_hash() {
        let f = file("v1", "x", 10);
        assert_eq!(verify_file(&f, 10, &"ab".repeat(32)), Ok(()));
        assert_eq!(verify_file(&f, 10, &"AB".repeat(32)), Ok(()));
        assert!(matches!(
            verify_file(&f, 9, &"ab".repeat(32)),
            Err(VerifyError::Size {
                expected: 10,
                actual: 9
            })
        ));
        assert!(matches!(
            verify_file(&f, 10, &"cd".repeat(32)),
            Err(VerifyError::Hash { .. })
        ));
    }

    #[test]
    fn backoff_grows() {
        assert!(backoff_ms(1) < backoff_ms(2));
        assert!(backoff_ms(2) < backoff_ms(3));
    }

    #[test]
    fn plan_hands_out_each_unverified_file_once_and_resumes() {
        let mut plan = InstallPlan::new(manifest()).unwrap();
        assert_eq!(plan.progress().percent(), 0);
        plan.mark_verified(0);
        plan.mark_verified(4);
        let mut claimed = vec![];
        while let Some(i) = plan.claim_next() {
            claimed.push(i);
        }
        assert_eq!(claimed, vec![1, 2, 3]);
        assert!(!plan.is_complete());
        assert_eq!(plan.remaining_bytes(), 300);
        for i in claimed {
            plan.mark_verified(i);
        }
        assert!(plan.is_complete());
        let p = plan.progress();
        assert_eq!(
            (p.files_done, p.files_total, p.bytes_done, p.percent()),
            (5, 5, 1000, 100)
        );
    }

    #[test]
    fn progress_has_a_one_line_summary() {
        let p = Progress {
            files_done: 3,
            files_total: 24,
            bytes_done: 1024 * 1024,
            bytes_total: 4 * 1024 * 1024,
        };
        assert_eq!(p.summary(), "3 of 24 files, 1.0 MB of 4.0 MB (25%)");
    }

    #[test]
    fn progress_percent_is_bounded() {
        let p = Progress {
            files_done: 1,
            files_total: 2,
            bytes_done: 250,
            bytes_total: 1000,
        };
        assert_eq!(p.percent(), 25);
        let p = Progress {
            files_done: 0,
            files_total: 0,
            bytes_done: 0,
            bytes_total: 0,
        };
        assert_eq!(p.percent(), 100);
        let p = Progress {
            files_done: 0,
            files_total: 1,
            bytes_done: 5000,
            bytes_total: 1000,
        };
        assert_eq!(p.percent(), 100);
    }

    #[test]
    fn quota_check_leaves_headroom_and_tolerates_unknowns() {
        assert!(check_space(100, Some(0.0), Some(1000.0)).is_ok());
        assert!(check_space(100, Some(950.0), Some(1000.0)).is_err());
        assert!(check_space(100, Some(895.0), Some(1000.0)).is_ok()); // 105 free fits 100 + 5%
        assert!(check_space(100, Some(896.0), Some(1000.0)).is_err());
        assert!(check_space(100, None, Some(10.0)).is_ok());
        assert!(check_space(100, Some(5.0), None).is_ok());
        let error = check_space(5_000_000, Some(0.0), Some(1_000_000.0)).unwrap_err();
        assert!(error.to_string().contains("Not enough storage"));
    }

    #[test]
    fn phases_report_busy() {
        assert!(Phase::Downloading.is_busy());
        assert!(!Phase::Offer.is_busy());
        assert!(!Phase::Failed(Failure::new(FailureKind::Network, "x")).is_busy());
    }
}
