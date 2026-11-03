//! Input selection, the input digest, and the upstream survey that feeds coverage.
//!
//! The dataset is untrusted: files are only read and parsed as JSON, never executed, and
//! symbolic links are skipped.

use crate::config::SourceConfig;
use cast_iron_schema::{EntryKind, Exclusion};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Typed { kind: EntryKind },
    Rules,
}

#[derive(Debug, Clone)]
pub struct SelectedFile {
    /// Path relative to the dataset root, with `/` separators.
    pub rel: String,
    pub sha256: String,
    pub role: Role,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct Selection {
    /// Sorted by `rel`.
    pub files: Vec<SelectedFile>,
    pub digest: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// The SHA-256 over the sorted `path:sha256\n` lines of the selected files.
pub fn input_digest<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut lines: Vec<String> = files
        .into_iter()
        .map(|(p, h)| format!("{p}:{h}\n"))
        .collect();
    lines.sort();
    sha256_hex(lines.concat().as_bytes())
}

fn safe_relative(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    let ok = !path.is_empty()
        && p.is_relative()
        && p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "selection path {path:?} must be relative and free of `..`"
        ))
    }
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("cannot read directory {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    Ok(entries)
}

/// Every `.json` file below `dir`, sorted, as paths relative to `root`. Symlinks are skipped.
pub fn list_json(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for path in read_dir_sorted(dir)? {
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            list_json(root, &path, out)?;
        } else if meta.is_file() && path.extension().is_some_and(|e| e == "json") {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push(rel);
        }
    }
    Ok(())
}

/// Reads every selected file. A configured directory or file that does not exist is a
/// configuration error, so it fails the run instead of becoming a rejection.
pub fn select(dataset: &Path, cfg: &SourceConfig) -> Result<Selection, String> {
    let mut files: BTreeMap<String, SelectedFile> = BTreeMap::new();
    for dir in &cfg.dataset.typed {
        safe_relative(dir)?;
        let kind_slug = dir.rsplit('/').next().unwrap_or_default();
        let kind = EntryKind::from_slug(kind_slug)
            .ok_or_else(|| format!("typed directory {dir:?} does not end in an entry kind"))?;
        let abs = dataset.join(dir);
        if !abs.is_dir() {
            return Err(format!("typed directory {dir} is missing from the dataset"));
        }
        let mut rels = Vec::new();
        list_json(dataset, &abs, &mut rels)?;
        for rel in rels {
            add(dataset, &mut files, rel, Role::Typed { kind })?;
        }
    }
    for file in &cfg.dataset.rules {
        safe_relative(file)?;
        if !dataset.join(file).is_file() {
            return Err(format!("rules file {file} is missing from the dataset"));
        }
        add(dataset, &mut files, file.clone(), Role::Rules)?;
    }
    let files: Vec<SelectedFile> = files.into_values().collect();
    let digest = input_digest(files.iter().map(|f| (f.rel.as_str(), f.sha256.as_str())));
    Ok(Selection { files, digest })
}

fn add(
    dataset: &Path,
    files: &mut BTreeMap<String, SelectedFile>,
    rel: String,
    role: Role,
) -> Result<(), String> {
    if files.contains_key(&rel) {
        return Err(format!("{rel} is selected twice"));
    }
    let bytes = fs::read(dataset.join(&rel)).map_err(|e| format!("cannot read {rel}: {e}"))?;
    files.insert(
        rel.clone(),
        SelectedFile {
            sha256: sha256_hex(&bytes),
            rel,
            role,
            bytes,
        },
    );
    Ok(())
}

#[derive(Debug, Default)]
pub struct Survey {
    pub exclusions: Vec<Exclusion>,
    /// Every `url` string at any depth in any upstream JSON file.
    pub urls: BTreeSet<String>,
    /// Top-level records per `source` value.
    pub source_counts: BTreeMap<String, u32>,
    /// Files that did not parse as a JSON object, outside the selection.
    pub unparsable: Vec<String>,
    pub total_files: u32,
}

impl Survey {
    pub fn has_url_or_descendant(&self, url: &str) -> bool {
        if self.urls.contains(url) {
            return true;
        }
        let prefix = format!("{url}/");
        self.urls
            .range(prefix.clone()..)
            .next()
            .is_some_and(|u| u.starts_with(&prefix))
    }
}

/// Walks the whole dataset: builds the upstream url set, per-source record counts, and the
/// exclusion list for everything the selection leaves out.
pub fn survey(
    dataset: &Path,
    cfg: &SourceConfig,
    selected: &BTreeSet<String>,
) -> Result<Survey, String> {
    let mut all = Vec::new();
    list_json(dataset, dataset, &mut all)?;
    let mut out = Survey {
        total_files: all.len() as u32,
        ..Survey::default()
    };

    // Directories that hold selection roots, so they are expanded instead of excluded whole.
    let mut roots: Vec<String> = cfg.dataset.typed.clone();
    roots.extend(cfg.dataset.rules.iter().cloned());
    let is_ancestor = |prefix: &str| roots.iter().any(|r| r.starts_with(&format!("{prefix}/")));

    let mut buckets: BTreeMap<String, (u32, String)> = BTreeMap::new();
    for rel in &all {
        let bytes = fs::read(dataset.join(rel)).map_err(|e| format!("cannot read {rel}: {e}"))?;
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => {
                if let Some(source) = value.get("source").and_then(Value::as_str) {
                    *out.source_counts.entry(source.to_string()).or_default() += 1;
                }
                collect_urls(&value, &mut out.urls);
            }
            Err(_) => {
                if !selected.contains(rel) {
                    out.unparsable.push(rel.clone());
                }
            }
        }
        if selected.contains(rel) {
            continue;
        }
        let parts: Vec<&str> = rel.split('/').collect();
        let dirs = &parts[..parts.len() - 1];
        let mut bucket = None;
        for k in 1..=dirs.len() {
            let prefix = dirs[..k].join("/");
            if is_ancestor(&prefix) || (k == 1 && dirs.len() > 1) {
                continue;
            }
            bucket = Some(prefix);
            break;
        }
        let bucket = bucket.unwrap_or_else(|| {
            if dirs.is_empty() {
                "*.json".to_string()
            } else {
                format!("{}/*.json", dirs.join("/"))
            }
        });
        let entry = buckets
            .entry(bucket.clone())
            .or_insert_with(|| (0, exclusion_reason(&bucket, cfg)));
        entry.0 += 1;
    }
    out.exclusions = buckets
        .into_iter()
        .map(|(path, (records, reason))| Exclusion {
            path,
            records,
            reason,
        })
        .collect();
    // Database files are never read; list them so that they do not disappear silently.
    let mut dbs = 0u32;
    for path in read_dir_sorted(dataset)? {
        if path.extension().is_some_and(|e| e == "db") {
            dbs += 1;
        }
    }
    if dbs > 0 {
        out.exclusions.push(Exclusion {
            path: "*.db".into(),
            records: dbs,
            reason: "SQLite databases are never opened; the legal notices were extracted ahead of time into sources/psrd-data/legal-notices.json".into(),
        });
    }
    out.exclusions.sort_by(|a, b| a.path.cmp(&b.path));
    out.unparsable.sort();
    Ok(out)
}

fn exclusion_reason(bucket: &str, cfg: &SourceConfig) -> String {
    if bucket.ends_with("/*.json") || bucket == "*.json" {
        return "not selected (files beside selected records)".into();
    }
    if let Some(parent) = bucket.rsplit_once('/').map(|(p, _)| p)
        && let Some(name) = bucket.rsplit('/').next()
    {
        let sibling = format!("{parent}/{name}.json");
        if cfg.dataset.rules.contains(&sibling) {
            return format!(
                "not selected separately: section copies of the selected {sibling} tree"
            );
        }
    }
    "no standalone category for this record family; rules-tree representations are retained where available".into()
}

fn collect_urls(value: &Value, out: &mut BTreeSet<String>) {
    let mut stack = vec![value];
    while let Some(v) = stack.pop() {
        match v {
            Value::Object(map) => {
                if let Some(Value::String(url)) = map.get("url") {
                    out.insert(url.clone());
                }
                stack.extend(map.values());
            }
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_order_independent_and_path_sensitive() {
        let a = input_digest([("a.json", "11"), ("b.json", "22")]);
        let b = input_digest([("b.json", "22"), ("a.json", "11")]);
        assert_eq!(a, b);
        assert_ne!(a, input_digest([("a.json", "11"), ("c.json", "22")]));
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn rejects_unsafe_selection_paths() {
        assert!(safe_relative("a/b").is_ok());
        assert!(safe_relative("../x").is_err());
        assert!(safe_relative("/etc").is_err());
        assert!(safe_relative("").is_err());
    }
}
