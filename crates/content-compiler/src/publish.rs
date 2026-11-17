//! How a compiled library reaches its output directory.
//!
//! The compiler writes the whole library into a sibling staging directory, then replaces the
//! output directory in three renames and one delete:
//!
//! 1. write `<out>.staging`, the manifest last;
//! 2. rename `<out>` to `<out>.backup`, when `<out>` holds a library;
//! 3. rename `<out>.staging` to `<out>`;
//! 4. delete `<out>.backup`.
//!
//! Each step leaves a state that [`recover_out`] reads at the start of the next compile and
//! repairs, so an interrupted compile never strands a finished library and never makes the
//! output directory look foreign. A rename within one parent directory is atomic on one file
//! system. The code does not call `fsync`, so it gives no promise about a power loss: after one,
//! `cargo xtask build-site` checks every file against the manifest and asks for a new compile.

use crate::pipeline::write_file;
use cast_iron_schema::ContentManifest;
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
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

/// A library is whole when its manifest parses and every file it lists has the listed size. The
/// writer puts the manifest last, so a directory without one is partly written.
fn is_whole(dir: &Path) -> bool {
    let Some(manifest) = read_manifest(dir) else {
        return false;
    };
    manifest.files.iter().all(|file| {
        fs::metadata(dir.join(&file.path))
            .is_ok_and(|meta| meta.is_file() && meta.len() == file.bytes)
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

/// Repairs what an interrupted compile left beside `out`. Call it at the start of a compile.
///
/// | `out` | backup | staging | repair |
/// |---|---|---|---|
/// | any | none | any | remove the staging directory |
/// | library | any | any | remove the backup and the staging directory (the new library is live) |
/// | absent | any | whole | finish the handoff: staging becomes `out`, remove the backup |
/// | absent | whole | not whole | roll back: the backup becomes `out`, remove the staging directory |
/// | absent | not whole | not whole | stop and name both directories |
/// | foreign | any | any | stop: `out` is not ours |
pub fn recover_out(out: &Path) -> Result<(), String> {
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
        Slot::Library => {
            // The new library is in place. The backup is the old one, or half deleted.
            remove_dir(&staging)?;
            remove_dir(&backup)
        }
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

/// Writes the whole library into a staging directory beside `out`, then replaces `out` with it.
///
/// A failure while writing leaves `out` as it was. The manifest is the last file written into the
/// staging directory, and `out` is touched only by the two renames and the final delete.
pub fn publish_staged(
    out: &Path,
    content_version: &str,
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &[u8],
) -> Result<(), String> {
    recover_out(out)?;
    check_out(out)?;
    let staging = staging_dir(out);
    let backup = backup_dir(out);
    remove_dir(&staging)?;
    let written = (|| {
        for (path, bytes) in files {
            write_file(&staging.join(content_version).join(path), bytes)?;
        }
        write_file(&staging.join("manifest.json"), manifest)
    })();
    if let Err(e) = written {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
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
    use cast_iron_schema::{ManifestFile, SCHEMA_VERSION};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ci-publish-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A small whole library at `dir`, marked by `version`.
    fn library(dir: &Path, version: &str) {
        let body = format!("{{\"v\":\"{version}\"}}");
        write_file(&dir.join(version).join("a.json"), body.as_bytes()).unwrap();
        let manifest = ContentManifest {
            schema_version: SCHEMA_VERSION,
            content_version: version.into(),
            total_bytes: body.len() as u64,
            files: vec![ManifestFile {
                path: format!("{version}/a.json"),
                sha256: "0".repeat(64),
                bytes: body.len() as u64,
            }],
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

    /// The version of the library at `dir`.
    fn version(dir: &Path) -> String {
        read_manifest(dir).unwrap().content_version
    }

    fn files(version: &str) -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([(
            "a.json".to_string(),
            format!("{{\"v\":\"{version}\"}}").into_bytes(),
        )])
    }

    fn manifest_bytes(version: &str) -> Vec<u8> {
        let probe = temp(&format!("probe-{version}"));
        library(&probe, version);
        let bytes = fs::read(probe.join("manifest.json")).unwrap();
        fs::remove_dir_all(&probe).unwrap();
        bytes
    }

    fn only_out_remains(out: &Path) {
        assert!(!staging_dir(out).exists(), "staging is left");
        assert!(!backup_dir(out).exists(), "backup is left");
    }

    #[test]
    fn a_publish_replaces_the_library_and_leaves_no_sibling() {
        let base = temp("replace");
        let out = base.join("content");
        library(&out, "vold");
        publish_staged(&out, "vnew", &files("vnew"), &manifest_bytes("vnew")).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_after_the_backup_rename_with_a_whole_staging_library_finishes_the_handoff() {
        let base = temp("after-backup");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        library(&staging_dir(&out), "vnew");
        recover_out(&out).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_after_the_backup_rename_with_a_partial_staging_library_rolls_back() {
        let base = temp("rollback");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        // The manifest is written last, so a staging directory without one is partial.
        write_file(&staging_dir(&out).join("vnew/a.json"), b"{}").unwrap();
        recover_out(&out).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_after_the_backup_rename_with_no_staging_directory_rolls_back() {
        let base = temp("rollback-no-staging");
        let out = base.join("content");
        library(&backup_dir(&out), "vold");
        recover_out(&out).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_after_the_activation_rename_removes_the_backup() {
        let base = temp("after-activate");
        let out = base.join("content");
        library(&out, "vnew");
        library(&backup_dir(&out), "vold");
        recover_out(&out).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_during_the_backup_delete_removes_what_is_left() {
        let base = temp("half-deleted");
        let out = base.join("content");
        library(&out, "vnew");
        // A recursive delete that stopped after the manifest: files remain, no manifest.
        write_file(&backup_dir(&out).join("vold/a.json"), b"{}").unwrap();
        recover_out(&out).unwrap();
        assert_eq!(version(&out), "vnew");
        only_out_remains(&out);
    }

    #[test]
    fn an_interruption_while_staging_is_written_leaves_the_library_alone() {
        let base = temp("mid-staging");
        let out = base.join("content");
        library(&out, "vold");
        write_file(&staging_dir(&out).join("vnew/a.json"), b"{}").unwrap();
        recover_out(&out).unwrap();
        assert_eq!(version(&out), "vold");
        only_out_remains(&out);
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
                write_file(&staging_dir(out).join("vstale/a.json"), b"{}").unwrap();
            }),
        ];
        for (name, make) in states {
            let base = temp(&format!("publish-after-{}", name.replace(' ', "-")));
            let out = base.join("content");
            make(&out);
            publish_staged(&out, "vnew", &files("vnew"), &manifest_bytes("vnew"))
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
        let error = recover_out(&out).unwrap_err();
        assert!(error.contains("content.backup"), "{error}");
        assert!(out.join("keep.txt").exists());
        assert!(backup_dir(&out).exists());
    }

    #[test]
    fn two_unusable_directories_stop_the_compile_and_are_kept() {
        let base = temp("unusable");
        let out = base.join("content");
        write_file(&backup_dir(&out).join("vold/a.json"), b"{}").unwrap();
        let error = recover_out(&out).unwrap_err();
        assert!(error.contains("neither is a whole library"), "{error}");
        assert!(backup_dir(&out).exists());
    }

    #[test]
    fn publish_refuses_foreign_directories() {
        let base = temp("refuse");
        let out = base.join("content");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("keep.txt"), "x").unwrap();
        let error = publish_staged(&out, "v1", &files("v1"), b"{}").unwrap_err();
        assert!(error.contains("refusing"), "{error}");
        assert!(out.join("keep.txt").exists());
        assert!(!staging_dir(&out).exists());
        assert!(!backup_dir(&out).exists());
    }
}
