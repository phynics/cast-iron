//! How a compiled library reaches its output directory.
//!
//! One compile at a time writes to an output directory: [`OutLock::acquire`] takes a lock file
//! beside it, and recovery, writing and publication all run while the lock is held.
//!
//! The compiler writes the whole library into a sibling staging directory, then replaces the
//! output directory in two renames and one delete:
//!
//! 1. write `<out>.staging`, the manifest last (the decoded files and the stored `.gz` files both);
//! 2. rename `<out>` to `<out>.backup`, when `<out>` holds a library;
//! 3. rename `<out>.staging` to `<out>`;
//! 4. delete `<out>.backup`.
//!
//! Each step leaves a state that [`recover_out`] reads at the start of the next compile and
//! repairs. A library that was moved into the backup is never lost: recovery finishes the handoff
//! when the staging library is whole, and restores the backup when it is not. A compile that
//! stops after it completed the staging directory but before step 2 has no backup, and recovery
//! discards that staging directory; the next compile builds it again.
//!
//! A rename within one parent directory is atomic on one file system. The code does not call
//! `fsync`, so it gives no promise about a power loss: after one, `cargo xtask build-site` checks
//! every file against the manifest and asks for a new compile.

use crate::pipeline::write_file;
use cast_iron_schema::ContentManifest;
use cast_iron_schema::manifest_check::validate_manifest;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

fn sibling(out: &Path, suffix: &str) -> PathBuf {
    let name = out
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "content".into());
    out.with_file_name(format!("{name}.{suffix}"))
}

/// The directory where a library is written before it replaces `out`.
pub fn staging_dir(out: &Path) -> PathBuf {
    sibling(out, "staging")
}

/// The directory that holds the old library between the two renames.
pub fn backup_dir(out: &Path) -> PathBuf {
    sibling(out, "backup")
}

/// The lock file that one compile holds while it recovers, writes and publishes `out`.
pub fn lock_file(out: &Path) -> PathBuf {
    sibling(out, "lock")
}

/// The exclusive right to recover, write and publish one output directory.
///
/// It is a lock file `<out>.lock` holding the pid and the start time of the compile, taken with
/// `create_new`, which is atomic, so two compiles cannot both succeed. A second compile fails at
/// once and changes nothing; it does not wait. The file is removed when the value is dropped, on
/// success and on failure alike, so a finished compile leaves nothing beside the output. The
/// file holds the pid, the start time and a nonce, and the drop removes only a file that still
/// holds its own text.
///
/// A compile that is killed (SIGKILL, power loss, an abort) leaves the file, and every later
/// compile fails until a person deletes it. The message names the file and says who held it. The
/// code never judges whether the holder is still alive: deleting the lock of a running compile
/// lets two compiles write one directory.
#[derive(Debug)]
pub struct OutLock {
    out: PathBuf,
    file: PathBuf,
    /// What this lock wrote into the file. `Drop` removes the file only while it still holds
    /// this text, so it never removes a lock that a later compile created.
    text: String,
}

impl OutLock {
    /// Takes the lock for `out`, or says who holds it.
    pub fn acquire(out: &Path) -> Result<OutLock, String> {
        let file = lock_file(out);
        if let Some(parent) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        let mut handle = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
        {
            Ok(handle) => handle,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                return Err(held_message(&file));
            }
            Err(e) => return Err(format!("cannot create {}: {e}", file.display())),
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        // The nonce makes the text unique to this lock, even for two locks of one process in one
        // second: the sub-second clock and a counter that only grows.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = format!(
            "{:x}-{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let text = format!("pid={}\nstarted={now}\nnonce={nonce}\n", std::process::id());
        if let Err(e) = handle.write_all(text.as_bytes()) {
            let _ = fs::remove_file(&file);
            return Err(format!("cannot write {}: {e}", file.display()));
        }
        Ok(OutLock {
            out: out.to_path_buf(),
            file,
            text,
        })
    }

    /// The output directory that this lock covers.
    pub fn out(&self) -> &Path {
        &self.out
    }
}

impl Drop for OutLock {
    /// Removes the lock file, but only while it still holds this lock's own text. If a person
    /// deleted the file and a later compile took a lock of its own, that lock stays.
    ///
    /// One window remains: the file can be replaced between the read and the unlink. It needs a
    /// person to delete a live lock and a new compile to start in the microseconds of this call,
    /// and the lock has no other way to detect a deletion, so the window is accepted.
    fn drop(&mut self) {
        if fs::read_to_string(&self.file).is_ok_and(|now| now == self.text) {
            let _ = fs::remove_file(&self.file);
        }
    }
}

/// The failure for a lock that exists: who holds it and since when, from the file when it can be
/// read, and what to do.
fn held_message(file: &Path) -> String {
    let text = fs::read_to_string(file).unwrap_or_default();
    let field = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
            .and_then(|value| value.trim().parse::<u64>().ok())
    };
    let who = match (field("pid"), field("started")) {
        (Some(pid), Some(started)) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            format!(
                "held by pid {pid}, since {} s ago",
                now.saturating_sub(started)
            )
        }
        _ => "the holder is not named in it".to_string(),
    };
    format!(
        "another compile may be writing this output: {} exists ({who}). Nothing was changed. If no compile is running, delete {}",
        file.display(),
        file.display()
    )
}

#[derive(Debug, PartialEq, Eq)]
enum Slot {
    /// Missing, or an empty directory.
    Absent,
    /// A directory with a manifest that parses.
    Library,
    /// Anything else: a symbolic link, a file, or a directory that is not ours.
    Foreign,
}

fn slot(dir: &Path) -> Result<Slot, String> {
    let meta = match fs::symlink_metadata(dir) {
        Ok(meta) => meta,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Slot::Absent),
        Err(e) => return Err(format!("cannot read {}: {e}", dir.display())),
    };
    if !meta.is_dir() {
        return Ok(Slot::Foreign);
    }
    let empty = fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .next()
        .is_none();
    if empty {
        return Ok(Slot::Absent);
    }
    Ok(if read_manifest(dir).is_some() {
        Slot::Library
    } else {
        Slot::Foreign
    })
}

fn read_manifest(dir: &Path) -> Option<ContentManifest> {
    serde_json::from_slice(&fs::read(dir.join("manifest.json")).ok()?).ok()
}

/// Whether the file at `path` under `dir` has the size and SHA-256.
fn file_matches(dir: &Path, path: &str, bytes: u64, sha256: &str) -> bool {
    fs::read(dir.join(path)).is_ok_and(|data| {
        data.len() as u64 == bytes && format!("{:x}", Sha256::digest(&data)) == sha256
    })
}

/// A library is whole when its manifest passes the installer's structural check and every file it
/// lists has the listed size and SHA-256. A row with `gzip` lists two files: the stored `.gz` file
/// at its `path` and the decoded file at its logical path, which the staging directory holds beside
/// it. Both are checked against their hashes, and neither is decompressed. Recovery uses this
/// before it keeps one copy and discards the other, so an edited or damaged copy is not chosen
/// over a good one. The writer puts the manifest last, so a directory without one is partly
/// written.
fn is_whole(dir: &Path) -> bool {
    let Some(manifest) = read_manifest(dir) else {
        return false;
    };
    if validate_manifest(&manifest).is_err() {
        return false;
    }
    manifest.files.iter().all(|file| {
        file_matches(dir, &file.path, file.bytes, &file.sha256)
            && file.gzip.as_ref().is_none_or(|decoded| {
                file_matches(dir, file.logical_path(), decoded.bytes, &decoded.sha256)
            })
    })
}

fn remove_dir(dir: &Path) -> Result<(), String> {
    match fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("cannot remove {}: {e}", dir.display())),
    }
}

fn rename(from: &Path, to: &Path) -> Result<(), String> {
    fs::rename(from, to)
        .map_err(|e| format!("cannot move {} to {}: {e}", from.display(), to.display()))
}

/// Checks that `out` is absent, empty, or holds one of our manifests, so [`publish_staged`] may
/// replace it. Writes nothing.
pub fn check_out(out: &Path) -> Result<(), String> {
    match slot(out)? {
        Slot::Absent | Slot::Library => Ok(()),
        Slot::Foreign => {
            let meta = fs::symlink_metadata(out).map_err(|e| e.to_string())?;
            Err(if meta.file_type().is_symlink() {
                format!("refusing to clean {}: it is a symbolic link", out.display())
            } else if !meta.is_dir() {
                format!("{} exists and is not a directory", out.display())
            } else {
                format!(
                    "refusing to clean {}: it is not empty and holds no Cast Iron manifest.json",
                    out.display()
                )
            })
        }
    }
}

/// Repairs what an interrupted compile left beside the output directory. Call it at the start of a
/// compile, with the lock held: only the holder may remove a staging or backup directory, because
/// they are the live writer's files while the lock is taken.
///
/// | `out` | backup | staging | repair |
/// |---|---|---|---|
/// | any | none | any | remove the staging directory |
/// | library, whole | any | any | remove the backup and the staging directory (the new library is live) |
/// | library, not whole | whole | any | roll back: the backup replaces `out`, remove the staging directory |
/// | library, not whole | not whole | any | stop and name the directories |
/// | absent | any | whole | finish the handoff: staging becomes `out`, remove the backup |
/// | absent | whole | not whole | roll back: the backup becomes `out`, remove the staging directory |
/// | absent | not whole | not whole | stop and name the directories |
/// | foreign | any | any | stop: `out` is not ours |
pub fn recover_out(lock: &OutLock) -> Result<(), String> {
    let out = lock.out();
    let staging = staging_dir(out);
    let backup = backup_dir(out);
    if fs::symlink_metadata(&backup).is_err() {
        // No handoff was under way. A staging directory is an unfinished write.
        return remove_dir(&staging);
    }
    match slot(out)? {
        Slot::Foreign => Err(format!(
            "{} and {} both exist, and {} holds no Cast Iron manifest.json; move one of them away",
            backup.display(),
            out.display(),
            out.display()
        )),
        Slot::Library if is_whole(out) => {
            // The new library is in place. The backup is the old one, or half deleted.
            remove_dir(&staging)?;
            remove_dir(&backup)
        }
        Slot::Library if is_whole(&backup) => {
            // The activated library is damaged and the backup is not: keep the good copy.
            remove_dir(&staging)?;
            remove_dir(out)?;
            rename(&backup, out)?;
            eprintln!(
                "recovered an interrupted compile: {} failed its checks, restored the previous library",
                out.display()
            );
            Ok(())
        }
        Slot::Library => Err(format!(
            "neither {} nor {} is a whole library; keep the better copy at {} and remove the other",
            out.display(),
            backup.display(),
            out.display()
        )),
        Slot::Absent if is_whole(&staging) => {
            remove_dir(out)?;
            rename(&staging, out)?;
            eprintln!(
                "recovered an interrupted compile: finished moving {} to {}",
                staging.display(),
                out.display()
            );
            remove_dir(&backup)
        }
        Slot::Absent if is_whole(&backup) => {
            remove_dir(&staging)?;
            remove_dir(out)?;
            rename(&backup, out)?;
            eprintln!(
                "recovered an interrupted compile: restored the previous library at {}",
                out.display()
            );
            Ok(())
        }
        Slot::Absent => Err(format!(
            "an interrupted compile left {} and {}, and neither is a whole library; remove them and compile again",
            backup.display(),
            staging.display()
        )),
    }
}

/// Writes the whole library into a staging directory beside the output directory, then replaces
/// the output directory with it. The caller holds the lock. `layers` are the files to write, each
/// map by the path under the version directory: the decoded files and the stored `.gz` files.
///
/// A failure while writing leaves `out` as it was. The manifest is the last file written into the
/// staging directory, and `out` is touched only by the two renames and the final delete.
pub fn publish_staged(
    lock: &OutLock,
    content_version: &str,
    layers: &[&BTreeMap<String, Vec<u8>>],
    manifest: &[u8],
) -> Result<(), String> {
    let out = lock.out();
    recover_out(lock)?;
    check_out(out)?;
    let staging = staging_dir(out);
    let backup = backup_dir(out);
    remove_dir(&staging)?;
    let written = (|| {
        for (path, bytes) in layers.iter().flat_map(|layer| layer.iter()) {
            write_file(&staging.join(content_version).join(path), bytes)?;
        }
        write_file(&staging.join("manifest.json"), manifest)
    })();
    if let Err(e) = written {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    let replaces = slot(out)? == Slot::Library;
    if replaces {
        rename(out, &backup)?;
    } else {
        remove_dir(out)?; // an empty directory
    }
    if let Err(e) = rename(&staging, out) {
        if replaces {
            let _ = fs::rename(&backup, out);
        }
        return Err(e);
    }
    if replaces && let Err(e) = remove_dir(&backup) {
        // The new library is live. The next compile removes the backup.
        eprintln!("warning: {e}; the next compile removes it");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::manifest_check::REQUIRED_FILES;
    use cast_iron_schema::{Decoded, ManifestFile, SCHEMA_VERSION};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ci-publish-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The files of a small whole library, marked by `version`.
    fn files(version: &str) -> BTreeMap<String, Vec<u8>> {
        REQUIRED_FILES
            .iter()
            .map(|name| {
                (
                    name.to_string(),
                    format!("{{\"{name}\":\"{version}\"}}").into_bytes(),
                )
            })
            .collect()
    }

    fn manifest_bytes(version: &str) -> Vec<u8> {
        let files = files(version);
        let manifest = ContentManifest {
            schema_version: SCHEMA_VERSION,
            content_version: version.into(),
            total_bytes: files.values().map(|b| b.len() as u64).sum(),
            files: files
                .iter()
                .map(|(name, bytes)| ManifestFile {
                    path: format!("{version}/{name}"),
                    sha256: format!("{:x}", Sha256::digest(bytes)),
                    bytes: bytes.len() as u64,
                    gzip: None,
                })
                .collect(),
            entry_count: 0,
            datasets: vec![],
            corpus_tag: None,
        };
        serde_json::to_vec(&manifest).unwrap()
    }

    /// A whole library at `dir`, marked by `version`.
    fn library(dir: &Path, version: &str) {
        for (name, bytes) in files(version) {
            write_file(&dir.join(version).join(name), &bytes).unwrap();
        }
        write_file(&dir.join("manifest.json"), &manifest_bytes(version)).unwrap();
    }

    /// The version of the library at `dir`.
    fn version(dir: &Path) -> String {
        read_manifest(dir).unwrap().content_version
    }

    /// Takes the lock for `out` and, as a compile would, recovers.
    fn locked(out: &Path) -> OutLock {
        OutLock::acquire(out).unwrap()
    }

    fn only_out_remains(out: &Path) {
        assert!(!staging_dir(out).exists(), "staging is left");
        assert!(!backup_dir(out).exists(), "backup is left");
    }

    /// Every path under `dir` with its size, so a test can show that nothing changed.
    fn listing(dir: &Path) -> Vec<(String, u64)> {
        fn walk(base: &Path, dir: &Path, found: &mut Vec<(String, u64)>) {
            let Ok(items) = fs::read_dir(dir) else { return };
            for item in items {
                let path = item.unwrap().path();
                let meta = fs::symlink_metadata(&path).unwrap();
                found.push((
                    path.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    meta.len(),
                ));
                if meta.is_dir() {
                    walk(base, &path, found);
                }
            }
        }
        let mut found = Vec::new();
        walk(dir, dir, &mut found);
        found.sort();
        found
    }

    #[test]
    fn a_publish_replaces_the_library_and_leaves_no_sibling() {
        let base = temp("replace");
        let out = base.join("content");
        library(&out, "vold");
        let lock = locked(&out);
        publish_staged(&lock, "vnew", &[&files("vnew")], &manifest_bytes("vnew")).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
        drop(lock);
        assert!(!lock_file(&out).exists(), "the lock outlived its holder");
    }

    #[test]
    fn an_interruption_after_the_backup_rename_with_a_whole_staging_library_finishes_the_handoff() {
        let base = temp("after-backup");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        library(&staging_dir(&out), "vnew");
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_after_the_backup_rename_with_a_partial_staging_library_rolls_back() {
        let base = temp("rollback");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        // The manifest is written last, so a staging directory without one is partial.
        write_file(&staging_dir(&out).join("vnew/catalog.json"), b"{}").unwrap();
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_after_the_backup_rename_with_no_staging_directory_rolls_back() {
        let base = temp("rollback-no-staging");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_after_the_activation_rename_removes_the_backup() {
        let base = temp("after-activate");
        let out = base.join("content");
        library(&out, "vnew");
        library(&backup_dir(&out), "vold");
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_during_the_backup_delete_removes_what_is_left() {
        let base = temp("half-deleted");
        let out = base.join("content");
        library(&out, "vnew");
        // A recursive delete that stopped after the manifest: files remain, no manifest.
        write_file(&backup_dir(&out).join("vold/catalog.json"), b"{}").unwrap();
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_while_staging_is_written_leaves_the_library_alone() {
        let base = temp("mid-staging");
        let out = base.join("content");
        library(&out, "vold");
        write_file(&staging_dir(&out).join("vnew/catalog.json"), b"{}").unwrap();
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);
    }

    #[test]
    fn an_activated_library_that_fails_its_checks_does_not_replace_a_good_backup() {
        // Same-length damage: the size matches the manifest, the SHA-256 does not.
        let base = temp("damaged-live");
        let out = base.join("content");
        library(&out, "vnew");
        library(&backup_dir(&out), "vold");
        let victim = out.join("vnew/catalog.json");
        let mut bytes = fs::read(&victim).unwrap();
        bytes[2] ^= 1;
        fs::write(&victim, bytes).unwrap();
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);

        // A manifest with an empty file list passes a size check and fails the structural one.
        let base = temp("empty-live");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        write_file(
            &out.join("manifest.json"),
            br#"{"schema_version":4,"content_version":"vnew","total_bytes":0,"files":[],"entry_count":0,"datasets":[]}"#,
        )
        .unwrap();
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);

        // When neither copy is whole, both stay for a person to choose.
        let base = temp("both-damaged");
        let out = base.join("content");
        library(&out, "vnew");
        library(&backup_dir(&out), "vold");
        fs::write(out.join("vnew/catalog.json"), b"x").unwrap();
        fs::write(backup_dir(&out).join("vold/catalog.json"), b"x").unwrap();
        let error = recover_out(&locked(&out)).unwrap_err();
        assert!(error.contains("neither"), "{error}");
        assert!(out.exists() && backup_dir(&out).exists());
    }

    #[test]
    fn a_staging_library_with_wrong_bytes_is_not_activated() {
        let base = temp("damaged-staging");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        library(&staging_dir(&out), "vnew");
        let victim = staging_dir(&out).join("vnew/search.bin");
        let mut bytes = fs::read(&victim).unwrap();
        bytes[2] ^= 1;
        fs::write(&victim, bytes).unwrap();
        recover_out(&locked(&out)).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);
    }

    /// A library of schema 8 at `dir`: each required file as a decoded file and a stored `.gz`
    /// file (its bytes are not real gzip, because `is_whole` hashes and never decodes).
    fn gzip_library(dir: &Path, version: &str) {
        let mut rows = Vec::new();
        for (name, decoded) in files(version) {
            let stored = [b"stored:".as_slice(), &decoded].concat();
            write_file(&dir.join(version).join(&name), &decoded).unwrap();
            write_file(&dir.join(version).join(format!("{name}.gz")), &stored).unwrap();
            rows.push(ManifestFile {
                path: format!("{version}/{name}.gz"),
                sha256: format!("{:x}", Sha256::digest(&stored)),
                bytes: stored.len() as u64,
                gzip: Some(Decoded {
                    sha256: format!("{:x}", Sha256::digest(&decoded)),
                    bytes: decoded.len() as u64,
                }),
            });
        }
        let manifest = ContentManifest {
            schema_version: SCHEMA_VERSION,
            content_version: version.into(),
            total_bytes: rows.iter().map(|row| row.bytes).sum(),
            files: rows,
            entry_count: 0,
            datasets: vec![],
            corpus_tag: None,
        };
        write_file(
            &dir.join("manifest.json"),
            &serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn a_library_with_gzip_rows_is_whole_only_when_both_layers_match() {
        let base = temp("both-layers");
        let out = base.join("content");
        gzip_library(&out, "vgz");
        assert!(is_whole(&out));

        // The stored file is damaged (same length).
        let stored = out.join("vgz/catalog.json.gz");
        let mut bytes = fs::read(&stored).unwrap();
        bytes[9] ^= 1;
        fs::write(&stored, &bytes).unwrap();
        assert!(!is_whole(&out), "a damaged stored file passed");
        bytes[9] ^= 1;
        fs::write(&stored, &bytes).unwrap();
        assert!(is_whole(&out));

        // The decoded file is damaged (same length).
        let decoded = out.join("vgz/catalog.json");
        let mut bytes = fs::read(&decoded).unwrap();
        bytes[2] ^= 1;
        fs::write(&decoded, &bytes).unwrap();
        assert!(!is_whole(&out), "a damaged decoded file passed");

        // The decoded file is missing.
        fs::remove_file(&decoded).unwrap();
        assert!(!is_whole(&out), "a missing decoded file passed");
    }

    #[test]
    fn a_publish_writes_both_layers() {
        let base = temp("write-layers");
        let out = base.join("content");
        let decoded = files("vgz");
        let stored: BTreeMap<String, Vec<u8>> = decoded
            .iter()
            .map(|(name, bytes)| {
                (
                    format!("{name}.gz"),
                    [b"stored:".as_slice(), bytes].concat(),
                )
            })
            .collect();
        let lock = locked(&out);
        publish_staged(&lock, "vgz", &[&decoded, &stored], b"{}").unwrap();
        for name in decoded.keys() {
            assert!(out.join("vgz").join(name).is_file(), "{name}");
            assert!(
                out.join("vgz").join(format!("{name}.gz")).is_file(),
                "{name}.gz"
            );
        }
    }

    #[test]
    fn a_publish_after_each_interruption_ends_with_the_new_library() {
        type State = (&'static str, fn(&Path));
        let states: [State; 4] = [
            ("backup and whole staging", |out| {
                library(&backup_dir(out), "vold");
                library(&staging_dir(out), "vstale");
            }),
            ("backup only", |out| library(&backup_dir(out), "vold")),
            ("library and backup", |out| {
                library(out, "vold");
                library(&backup_dir(out), "volder");
            }),
            ("library and staging", |out| {
                library(out, "vold");
                write_file(&staging_dir(out).join("vstale/catalog.json"), b"{}").unwrap();
            }),
        ];
        for (name, make) in states {
            let base = temp(&format!("publish-after-{}", name.replace(' ', "-")));
            let out = base.join("content");
            make(&out);
            let lock = locked(&out);
            publish_staged(&lock, "vnew", &[&files("vnew")], &manifest_bytes("vnew"))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(version(&out), "vnew", "{name}");
            only_out_remains(&out);
        }
    }

    #[test]
    fn a_backup_beside_a_foreign_directory_stops_the_compile() {
        let base = temp("foreign");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("keep.txt"), "x").unwrap();
        let error = recover_out(&locked(&out)).unwrap_err();
        assert!(error.contains("content.backup"), "{error}");
        assert!(out.join("keep.txt").exists());
        assert!(backup_dir(&out).exists());
    }

    #[test]
    fn two_unusable_directories_stop_the_compile_and_are_kept() {
        let base = temp("unusable");
        let out = base.join("content");
        write_file(&backup_dir(&out).join("vold/catalog.json"), b"{}").unwrap();
        let error = recover_out(&locked(&out)).unwrap_err();
        assert!(error.contains("neither is a whole library"), "{error}");
        assert!(backup_dir(&out).exists());
    }

    #[test]
    fn publish_refuses_foreign_directories() {
        let base = temp("refuse");
        let out = base.join("content");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("keep.txt"), "x").unwrap();
        let lock = locked(&out);
        let error = publish_staged(&lock, "v1", &[&files("v1")], b"{}").unwrap_err();
        assert!(error.contains("refusing"), "{error}");
        assert!(out.join("keep.txt").exists());
        assert!(!staging_dir(&out).exists());
        assert!(!backup_dir(&out).exists());
    }

    #[test]
    fn a_held_lock_blocks_a_second_compile_and_touches_no_directory() {
        let base = temp("held");
        let out = base.join("content");
        library(&out, "vold");
        // The holder is part way through its write: a staging directory with some files.
        let first = locked(&out);
        write_file(&staging_dir(&out).join("vnew/catalog.json"), b"{}").unwrap();
        let before = listing(&base);

        let error = OutLock::acquire(&out).unwrap_err();
        assert!(error.contains("another compile may be writing"), "{error}");
        assert!(
            error.contains(&format!("held by pid {}", std::process::id())),
            "{error}"
        );
        assert!(error.contains("Nothing was changed"), "{error}");
        assert_eq!(
            listing(&base),
            before,
            "the blocked compile changed something"
        );
        assert_eq!(version(&out), "vold");

        // After the holder finishes, the next compile takes the lock.
        drop(first);
        assert!(!lock_file(&out).exists());
        drop(OutLock::acquire(&out).unwrap());
    }

    #[test]
    fn a_leftover_lock_fails_with_who_held_it_and_how_to_remove_it() {
        let base = temp("leftover");
        let out = base.join("content");
        library(&out, "vold");
        // What a compile that was killed leaves behind.
        fs::write(lock_file(&out), "pid=4242\nstarted=1\n").unwrap();
        let before = listing(&base);
        let error = OutLock::acquire(&out).unwrap_err();
        assert!(error.contains("held by pid 4242"), "{error}");
        assert!(error.contains("Nothing was changed"), "{error}");
        assert!(
            error.contains(&format!(
                "If no compile is running, delete {}",
                lock_file(&out).display()
            )),
            "{error}"
        );
        assert_eq!(listing(&base), before);

        // A lock that names nobody (killed while writing it) fails the same way.
        fs::write(lock_file(&out), "").unwrap();
        let error = OutLock::acquire(&out).unwrap_err();
        assert!(error.contains("the holder is not named"), "{error}");
        assert!(error.contains("delete"), "{error}");

        // After the person deletes it, the next compile runs.
        fs::remove_file(lock_file(&out)).unwrap();
        drop(OutLock::acquire(&out).unwrap());
        assert!(!lock_file(&out).exists());
    }

    #[test]
    fn a_dropped_lock_does_not_remove_the_lock_of_a_later_compile() {
        let base = temp("drop-own");
        let out = base.join("content");
        let first = locked(&out);
        // A person deletes the lock of the first compile, believing it dead, and a second compile
        // takes its own lock.
        fs::remove_file(lock_file(&out)).unwrap();
        let second = locked(&out);
        let held = fs::read_to_string(lock_file(&out)).unwrap();
        assert_ne!(first.text, second.text);
        // The first compile ends. Its drop must leave the second's lock in place.
        drop(first);
        assert_eq!(fs::read_to_string(lock_file(&out)).unwrap(), held);
        let error = OutLock::acquire(&out).unwrap_err();
        assert!(error.contains("another compile may be writing"), "{error}");
        // The second ends normally and removes its own lock.
        drop(second);
        assert!(!lock_file(&out).exists());
    }

    #[test]
    fn a_dropped_lock_whose_file_is_gone_leaves_nothing_to_remove() {
        let base = temp("drop-gone");
        let out = base.join("content");
        let lock = locked(&out);
        fs::remove_file(lock_file(&out)).unwrap();
        drop(lock);
        assert!(!lock_file(&out).exists());
    }
}
