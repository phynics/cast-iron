#![allow(dead_code)]

use cast_iron_content_compiler::pipeline::{Options, Outcome, compile};
use cast_iron_schema::{Catalog, ContentManifest, RuleEntry};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// A scratch directory under the target directory, removed first.
pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Copies the fixture config files next to a scratch `source.toml`.
pub fn config_dir(scratch: &Path) -> PathBuf {
    let dir = scratch.join("config");
    fs::create_dir_all(&dir).unwrap();
    for name in ["source.toml", "legal-notices.json", "id-overrides.toml"] {
        fs::copy(fixtures().join(name), dir.join(name)).unwrap();
    }
    dir
}

pub fn options(config: &Path, dataset: &Path, out: &Path, reports: &Path) -> Options {
    Options {
        source_config: config.join("source.toml"),
        dataset: dataset.to_path_buf(),
        out: out.to_path_buf(),
        reports: reports.to_path_buf(),
        write_digest: false,
        strict: false,
        overrides: None,
    }
}

pub fn run(opts: &Options) -> Outcome {
    compile(opts).unwrap_or_else(|f| panic!("compile failed:\n{}", f.lines.join("\n")))
}

/// Every file under `root`, relative path to bytes.
pub fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

pub struct Loaded {
    pub manifest: ContentManifest,
    pub catalog: Catalog,
    pub entries: Vec<RuleEntry>,
    pub dir: PathBuf,
}

pub fn load(out: &Path) -> Loaded {
    let manifest: ContentManifest =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    let dir = out.join(&manifest.content_version);
    let catalog: Catalog =
        serde_json::from_slice(&fs::read(dir.join("catalog.json")).unwrap()).unwrap();
    let mut entries = Vec::new();
    for file in &manifest.files {
        if file.path.contains("/entries/") {
            let shard: Vec<RuleEntry> =
                serde_json::from_slice(&fs::read(out.join(&file.path)).unwrap()).unwrap();
            entries.extend(shard);
        }
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Loaded {
        manifest,
        catalog,
        entries,
        dir,
    }
}

impl Loaded {
    pub fn entry(&self, id: &str) -> &RuleEntry {
        self.entries
            .iter()
            .find(|e| e.id.as_str() == id)
            .unwrap_or_else(|| {
                panic!(
                    "no entry {id}; have {:?}",
                    self.entries
                        .iter()
                        .map(|e| e.id.to_string())
                        .collect::<Vec<_>>()
                )
            })
    }
}
