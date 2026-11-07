//! Installer IO. The decisions live in [`plan`]; this file moves bytes.
//!
//! Order of an install: fetch the manifest, scan what the cache already holds (resume),
//! check space, download with verification before every `cache.put`, store the manifest,
//! re-read and re-hash everything, write `active_library` in one transaction, and only
//! then delete the other content caches.

pub mod plan;

use crate::content_cache::{self, FetchError};
use crate::js;
use crate::store::{self, ActiveLibrary, PendingInstall};
use cast_iron_schema::{ContentManifest, ManifestFile};
use futures::future::join_all;
use plan::{
    CONCURRENCY, Failure, FailureKind, InstallPlan, MAX_ATTEMPTS, Phase, Progress, backoff_ms,
    validate_manifest, verify_file,
};
use std::cell::RefCell;
use web_sys::Cache;

pub use plan::PlanError;

/// Called as the installer moves: the phase it is in and the files and bytes verified so far.
pub type Report<'a> = &'a dyn Fn(Phase, Progress);

/// Path of the copy of the manifest that is stored with a version, relative to `content/`.
pub fn stored_manifest_path(version: &str) -> String {
    format!("{version}/manifest.json")
}

/// Fetches and validates `content/manifest.json` from the network.
pub async fn fetch_manifest() -> Result<ContentManifest, Failure> {
    let bytes = content_cache::fetch_network("manifest.json")
        .await
        .map_err(|e| match e {
            FetchError::Network(_) => Failure::new(
                FailureKind::Network,
                "Cannot reach the library. Check your connection and try again.",
            ),
            FetchError::Status(code) => Failure::new(
                FailureKind::Network,
                format!("The library manifest is not available (HTTP {code})."),
            ),
        })?;
    let manifest: ContentManifest = serde_json::from_slice(&bytes).map_err(|e| {
        Failure::new(
            FailureKind::Integrity,
            format!("The library manifest is not valid: {e}"),
        )
    })?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheck {
    UpToDate,
    Available(ContentManifest),
}

/// Asks the network whether a newer library exists than `active_version`. Reads only the app's
/// own `content/manifest.json`. Used by the explicit check and the opt-out daily check (#206).
pub async fn check_for_update(active_version: &str) -> Result<UpdateCheck, Failure> {
    let manifest = fetch_manifest().await?;
    Ok(if manifest.content_version == active_version {
        UpdateCheck::UpToDate
    } else {
        UpdateCheck::Available(manifest)
    })
}

/// Downloads, verifies, and activates a library. Safe to call again after a failure:
/// files that are already cached and verify are skipped.
pub async fn install(
    manifest: &ContentManifest,
    report: Report<'_>,
) -> Result<ActiveLibrary, Failure> {
    let version = manifest.content_version.clone();
    let plan = RefCell::new(InstallPlan::new(manifest.clone())?);
    let say = |phase: Phase| report(phase, plan.borrow().progress());

    // Best effort: the answer shows in Settings.
    let _ = js::storage_persist().await;
    store::set_pending_install(&PendingInstall {
        content_version: version.clone(),
        started_at: js::now_ms(),
    })
    .await
    .map_err(storage_failure)?;

    let cache = content_cache::open(&version)
        .await
        .map_err(storage_message)?;

    say(Phase::Scanning);
    let file_count = plan.borrow().file_count();
    for index in 0..file_count {
        let file = plan.borrow().file(index).clone();
        if cached_file_is_good(&cache, &file).await {
            plan.borrow_mut().mark_verified(index);
            say(Phase::Scanning);
        }
    }

    let needed = plan.borrow().remaining_bytes();
    let estimate = js::storage_estimate().await;
    plan::check_space(needed, estimate.usage, estimate.quota)
        .map_err(|e| Failure::new(FailureKind::Quota, e.to_string()))?;

    say(Phase::Downloading);
    let first_failure: RefCell<Option<Failure>> = RefCell::new(None);
    let worker = || async {
        loop {
            if first_failure.borrow().is_some() {
                return;
            }
            let Some(index) = plan.borrow_mut().claim_next() else {
                return;
            };
            let file = plan.borrow().file(index).clone();
            match download_file(&cache, &file).await {
                Ok(()) => {
                    plan.borrow_mut().mark_verified(index);
                    say(Phase::Downloading);
                }
                Err(failure) => {
                    first_failure.borrow_mut().get_or_insert(failure);
                    return;
                }
            }
        }
    };
    join_all((0..CONCURRENCY).map(|_| worker())).await;
    if let Some(failure) = first_failure.into_inner() {
        return Err(failure);
    }

    let stored = serde_json::to_vec(manifest)
        .map_err(|e| Failure::new(FailureKind::Storage, e.to_string()))?;
    content_cache::put(&cache, &stored_manifest_path(&version), &stored)
        .await
        .map_err(storage_message)?;

    verify_installed(manifest, report).await?;

    say(Phase::Activating);
    let active = ActiveLibrary {
        content_version: version.clone(),
        schema_version: manifest.schema_version,
        entry_count: manifest.entry_count,
        total_bytes: manifest.total_bytes,
        activated_at: js::now_ms(),
    };
    store::activate_library(&active)
        .await
        .map_err(storage_failure)?;

    // Only now is it safe to drop the previous library and any stale pending install.
    remove_other_caches(&version).await;
    report(Phase::Done, plan.borrow().progress());
    Ok(active)
}

/// Deletes every content cache except the one for `keep`. Failures are ignored: the leftovers
/// are unused, and the next activation tries again.
pub async fn remove_other_caches(keep: &str) {
    let keep = content_cache::cache_name(keep);
    if let Ok(names) = content_cache::list_names().await {
        for name in names.into_iter().filter(|n| *n != keep) {
            let _ = content_cache::delete(&name).await;
        }
    }
}

/// What a verification pass covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifyReport {
    pub files: usize,
    pub bytes: u64,
}

/// Re-reads every manifest file from the Cache API and re-hashes it.
pub async fn verify_installed(
    manifest: &ContentManifest,
    report: Report<'_>,
) -> Result<VerifyReport, Failure> {
    let cache = content_cache::open_existing(&manifest.content_version)
        .await
        .map_err(storage_message)?
        .ok_or_else(|| {
            Failure::new(
                FailureKind::Integrity,
                "The library cache is missing from this device.",
            )
        })?;
    let total = manifest.files.len();
    let mut bytes_done = 0u64;
    for (index, file) in manifest.files.iter().enumerate() {
        let bytes = content_cache::read(&cache, &file.path)
            .await
            .map_err(storage_message)?
            .ok_or_else(|| {
                Failure::new(
                    FailureKind::Integrity,
                    format!("{} is missing from storage.", file.path),
                )
            })?;
        let sha = js::sha256_hex(&bytes)
            .await
            .map_err(|e| Failure::new(FailureKind::Storage, e))?;
        verify_file(file, bytes.len() as u64, &sha).map_err(|e| {
            Failure::new(
                FailureKind::Integrity,
                format!("{} failed verification: {e}.", file.path),
            )
        })?;
        bytes_done += file.bytes;
        report(
            Phase::Verifying,
            Progress {
                files_done: index + 1,
                files_total: total,
                bytes_done,
                bytes_total: manifest.total_bytes,
            },
        );
    }
    Ok(VerifyReport {
        files: total,
        bytes: manifest.total_bytes,
    })
}

/// The boot check. Confirms that the active library's cache exists and holds every file of the
/// manifest stored with it. Looks at presence only; "Verify library" re-hashes.
pub async fn check_installed(active: &ActiveLibrary) -> Result<ContentManifest, String> {
    let version = &active.content_version;
    let cache = content_cache::open_existing(version)
        .await?
        .ok_or_else(|| format!("the cache for library {version} is gone"))?;
    let bytes = content_cache::read(&cache, &stored_manifest_path(version))
        .await?
        .ok_or_else(|| "the stored library manifest is missing".to_string())?;
    let manifest: ContentManifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("the stored manifest is damaged: {e}"))?;
    validate_manifest(&manifest).map_err(|e| e.to_string())?;
    if manifest.content_version != *version || manifest.schema_version != active.schema_version {
        return Err("the stored manifest does not match the active library".into());
    }
    for file in &manifest.files {
        if !content_cache::contains(&cache, &file.path).await? {
            return Err(format!("{} is missing", file.path));
        }
    }
    Ok(manifest)
}

/// True when the cache already holds this file and its bytes verify. Never fails: any problem
/// means "download it again".
async fn cached_file_is_good(cache: &Cache, file: &ManifestFile) -> bool {
    let Ok(Some(bytes)) = content_cache::read(cache, &file.path).await else {
        return false;
    };
    match js::sha256_hex(&bytes).await {
        Ok(sha) => verify_file(file, bytes.len() as u64, &sha).is_ok(),
        Err(_) => false,
    }
}

/// Fetches one file with retries, verifies it, and only then puts it in the cache.
async fn download_file(cache: &Cache, file: &ManifestFile) -> Result<(), Failure> {
    let mut last = Failure::new(FailureKind::Network, "no attempt was made");
    for attempt in 1..=MAX_ATTEMPTS {
        match fetch_verified(file).await {
            Ok(bytes) => {
                return content_cache::put(cache, &file.path, &bytes)
                    .await
                    .map_err(storage_message);
            }
            Err(failure) => last = failure,
        }
        if attempt < MAX_ATTEMPTS {
            js::sleep(backoff_ms(attempt)).await;
        }
    }
    Err(Failure::new(
        last.kind,
        format!(
            "{}: {} (after {MAX_ATTEMPTS} attempts)",
            file.path, last.message
        ),
    ))
}

async fn fetch_verified(file: &ManifestFile) -> Result<Vec<u8>, Failure> {
    let bytes = content_cache::fetch_network(&file.path)
        .await
        .map_err(|e| Failure::new(FailureKind::Network, e.to_string()))?;
    let sha = js::sha256_hex(&bytes)
        .await
        .map_err(|e| Failure::new(FailureKind::Storage, e))?;
    verify_file(file, bytes.len() as u64, &sha)
        .map_err(|e| Failure::new(FailureKind::Integrity, e.to_string()))?;
    Ok(bytes)
}

fn storage_message(message: String) -> Failure {
    let kind = if message.contains("Quota") {
        FailureKind::Quota
    } else {
        FailureKind::Storage
    };
    let text = if kind == FailureKind::Quota {
        format!("The browser ran out of storage space: {message}")
    } else {
        format!("The browser could not store the library: {message}")
    };
    Failure::new(kind, text)
}

fn storage_failure(error: store::StoreError) -> Failure {
    Failure::new(FailureKind::Storage, error.to_string())
}
