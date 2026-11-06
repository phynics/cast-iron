//! Repository automation. Run `cargo xtask help`.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const HELP: &str = "\
cargo xtask <command>

  fetch-sources              Download the pinned plain-text corpus (cast-iron-data) into
                             .cache/sources, verified by tree hash (see sources/corpus.toml)
  corpus-tree-hash <dir>     Print the tree hash of a directory (the pin algorithm in sources/corpus.toml)
  content                    Compile the library from the pinned corpus into generated/content;
                             fails when an id in sources/published-ids.txt stops resolving
  published-ids              Add the ids of generated/content to sources/published-ids.txt
  build-site [--public-url P]
                             Build dist/ from generated/content. The default public URL is
                             /cast-iron/, the GitHub Pages project path
  stamp-shell                Stamp dist/sw.js with the app shell file list and version
  corpus-check <dir> [--drift-report <file>]
                             Read a plain-text corpus and validate ids, paths, sources,
                             and licences; exits non-zero and names the file and line.
                             Lists fields-versus-body disagreements as warnings
  corpus-fmt <dir> [--check] Rewrite every corpus file into its canonical form, or with --check
                             list the first differing line of each file that is not canonical
  corpus-migrate <dir>       Convert a corpus from an older format (see docs/corpus-format.md)
  check                      Run fmt, clippy, tests, and the WASM build
";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("fetch-sources") => fetch_sources(),
        Some("corpus-tree-hash") => corpus_tree_hash(&args[1..]),
        Some("content") => content(&args[1..]),
        Some("published-ids") => published_ids(),
        Some("build-site") => build_site(&args[1..]),
        Some("stamp-shell") => stamp_shell(&root().join("dist")),
        Some("corpus-check") => corpus_check(&args[1..]),
        Some("corpus-fmt") => corpus_fmt(&args[1..]),
        Some("corpus-migrate") => cast_iron_corpus::cli::run_migrate(&args[1..]),
        Some("check") => check(),
        Some("help") | None => {
            print!("{HELP}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command {other:?}\n\n{HELP}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(program: &str, args: &[&str]) -> Result {
    println!("$ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|e| format!("cannot start {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed: {status}", args.join(" ")))
    }
}

/// `cargo install` writes to `~/.cargo/bin`, which is not on every PATH.
fn trunk_program() -> String {
    if Command::new("trunk").arg("--version").output().is_ok() {
        return "trunk".into();
    }
    match std::env::var("HOME") {
        Ok(home) => format!("{home}/.cargo/bin/trunk"),
        Err(_) => "trunk".into(),
    }
}

/// Reads `key = "value"` from a TOML file without a TOML parser in this tool.
fn toml_string(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_string())
    })
}

fn fetch_sources() -> Result {
    fetch_corpus()
}

/// The pinned plain-text corpus: `sources/corpus.toml`.
struct CorpusPin {
    repository: String,
    commit: String,
    tree_sha256: String,
}

fn read_corpus_pin() -> Result<CorpusPin> {
    let path = root().join("sources/corpus.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let field = |key: &str| -> Result<String> {
        toml_string(&text, key)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("sources/corpus.toml has no {key}"))
    };
    let pin = CorpusPin {
        repository: field("repository")?,
        commit: field("commit")?.to_ascii_lowercase(),
        tree_sha256: field("tree_sha256")?.to_ascii_lowercase(),
    };
    let hex_of = |value: &str, length: usize| {
        value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    if !hex_of(&pin.commit, 40) {
        return Err("sources/corpus.toml commit must be 40 hexadecimal characters".into());
    }
    if !hex_of(&pin.tree_sha256, 64) {
        return Err("sources/corpus.toml tree_sha256 must be 64 hexadecimal characters".into());
    }
    Ok(pin)
}

fn corpus_cache() -> PathBuf {
    root().join(".cache/sources/cast-iron-data")
}

/// The tree hash that `sources/corpus.toml` pins: SHA-256 over one line per regular file,
/// `<sha256 of the contents>  <relative path>\n`, in byte order of the path, with `/`
/// separators and no leading `./`. It is the hash of
/// `find . -type f | sed 's|^\./||' | LC_ALL=C sort | xargs sha256sum`, so the
/// pin does not depend on archive bytes, which GitHub does not promise to keep stable.
/// A symbolic link or other special file is an error.
fn tree_hash(dir: &Path) -> Result<String> {
    let mut paths = Vec::new();
    collect_tree(dir, dir, &mut paths)?;
    paths.sort();
    let mut listing = Sha256::new();
    for (relative, path) in &paths {
        let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        listing.update(format!("{}  {relative}\n", hex(&Sha256::digest(&bytes))));
    }
    Ok(hex(&listing.finalize()))
}

fn collect_tree(base: &Path, dir: &Path, found: &mut Vec<(String, PathBuf)>) -> Result {
    for item in fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))? {
        let item = item.map_err(|e| e.to_string())?;
        let path = item.path();
        let kind = item.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            collect_tree(base, &path, found)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(base)
                .map_err(|e| e.to_string())?
                .components()
                .map(|part| {
                    part.as_os_str()
                        .to_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("non-UTF-8 path under {}", base.display()))
                })
                .collect::<Result<Vec<_>>>()?
                .join("/");
            found.push((relative, path));
        } else {
            return Err(format!(
                "{} is not a regular file or directory; the tree hash does not allow links",
                path.display()
            ));
        }
    }
    Ok(())
}

fn corpus_tree_hash(args: &[String]) -> Result {
    match args {
        [dir] => {
            println!("{}", tree_hash(Path::new(dir))?);
            Ok(())
        }
        _ => Err("usage: corpus-tree-hash <dir>".into()),
    }
}

fn tree_hash_mismatch(what: &str, expected: &str, actual: &str) -> String {
    format!(
        "{what} tree hash mismatch: sources/corpus.toml pins {expected}, found {actual}.\n\
         The pinned commit must yield exactly the pinned files. If you bumped the pin on purpose, \
         update tree_sha256 with `cargo xtask corpus-tree-hash <extracted dir>`."
    )
}

/// Downloads the pinned corpus commit into `.cache/sources/cast-iron-data` and verifies the
/// tree hash. The cache is replaced only after the new tree verifies.
fn fetch_corpus() -> Result {
    let pin = read_corpus_pin()?;
    let target = corpus_cache();
    if target.is_dir() && tree_hash(&target).is_ok_and(|found| found == pin.tree_sha256) {
        println!("Corpus {} is already present", pin.commit);
        return Ok(());
    }
    let repo = pin.repository.trim_end_matches('/');
    let name = repo
        .strip_prefix("https://github.com/")
        .filter(|rest| rest.split('/').count() == 2 && !rest.contains(".."))
        .ok_or("sources/corpus.toml repository must be an https://github.com/<owner>/<name> URL")?;
    let parent = target.parent().ok_or("invalid cache target")?;
    fs::create_dir_all(parent).map_err(|e| format!("cannot create cache parent: {e}"))?;
    let unique = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let stage = parent.join(format!(".cast-iron-data-stage-{unique}"));
    let archive = parent.join(format!(".cast-iron-data-{unique}.tar.gz"));
    let url = format!("https://codeload.github.com/{name}/tar.gz/{}", pin.commit);
    let archive_str = archive.to_str().ok_or("non-UTF-8 archive path")?;
    let operation = (|| -> Result {
        run("curl", &["-fsSL", "--retry", "3", "-o", archive_str, &url])?;
        fs::create_dir(&stage).map_err(|e| format!("cannot create staging directory: {e}"))?;
        let stage_str = stage.to_str().ok_or("non-UTF-8 staging path")?;
        run(
            "tar",
            &["xzf", archive_str, "--strip-components=1", "-C", stage_str],
        )?;
        let actual = tree_hash(&stage)?;
        if actual != pin.tree_sha256 {
            return Err(tree_hash_mismatch(
                "downloaded corpus",
                &pin.tree_sha256,
                &actual,
            ));
        }
        Ok(())
    })();
    let _ = fs::remove_file(&archive);
    if let Err(error) = operation {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    let backup = parent.join(format!(".cast-iron-data-old-{unique}"));
    let had_target = target.exists();
    if had_target {
        fs::rename(&target, &backup).map_err(|e| format!("cannot preserve existing cache: {e}"))?;
    }
    if let Err(error) = fs::rename(&stage, &target) {
        if had_target {
            let _ = fs::rename(&backup, &target);
        }
        let _ = fs::remove_dir_all(&stage);
        return Err(format!("cannot activate downloaded cache: {error}"));
    }
    if had_target {
        fs::remove_dir_all(&backup)
            .map_err(|e| format!("new cache is active but old cache cleanup failed: {e}"))?;
    }
    println!("Fetched corpus {}", pin.commit);
    Ok(())
}

/// Fails unless the cached corpus is exactly the pinned tree.
fn verify_corpus_cache() -> Result {
    let pin = read_corpus_pin()?;
    let target = corpus_cache();
    if !target.is_dir() {
        return Err(
            "the pinned corpus is not in .cache/sources/cast-iron-data; run `cargo xtask fetch-sources`"
                .into(),
        );
    }
    let actual = tree_hash(&target)?;
    if actual != pin.tree_sha256 {
        return Err(format!(
            "{}\nRun `cargo xtask fetch-sources` to replace the cache.",
            tree_hash_mismatch("cached corpus", &pin.tree_sha256, &actual)
        ));
    }
    Ok(())
}

/// Compiles the library from the pinned corpus into `generated/content`.
fn content(args: &[String]) -> Result {
    if let Some(argument) = args.first() {
        return Err(format!(
            "unknown argument {argument:?} for content\nusage: content"
        ));
    }
    verify_corpus_cache()?;
    run(
        "cargo",
        &[
            "run",
            "--quiet",
            "--release",
            "--package",
            "cast-iron-content-compiler",
            "--bin",
            "cast-iron-compile",
            "--",
            "--corpus",
            ".cache/sources/cast-iron-data",
            "--out",
            "generated/content",
            "--reports",
            "generated/reports",
            "--published",
            PUBLISHED_IDS,
        ],
    )
}

/// Every id that a published library has used. `content` fails when one stops resolving.
const PUBLISHED_IDS: &str = "sources/published-ids.txt";

const PUBLISHED_IDS_HEADER: &str = "\
# Every id that a published Cast Iron library has used, one per line, sorted by byte order.
# A reader's bookmarks can hold any of them, so `cargo xtask content` fails when one no longer
# resolves to an entry, directly or through a row of `aliases.toml` in the corpus.
#
# Made by `cargo xtask published-ids` from a compiled library: every entry id in catalog.json and
# every `old` id in aliases.json. The legacy ids are those of the PSRD library; they match the
# frozen snapshot at the git tag psrd-last-buildable (sources/canonical/legacy-ids.json).
#
# The file only grows. After a library is published: run `cargo xtask content`, run
# `cargo xtask published-ids`, and commit the added lines. Never delete or edit a line.
";

/// Adds the ids of `generated/content` to `sources/published-ids.txt`, keeping the file sorted.
fn published_ids() -> Result {
    let content = root().join("generated/content");
    let manifest: serde_json::Value = read_json(&content.join("manifest.json"))?;
    let version = manifest["content_version"]
        .as_str()
        .ok_or("manifest.json has no content_version")?;
    let version_dir = content.join(version);
    let path = root().join(PUBLISHED_IDS);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut ids: std::collections::BTreeSet<String> = existing
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    let before = ids.len();
    let catalog: serde_json::Value = read_json(&version_dir.join("catalog.json"))?;
    let entries = catalog["entries"]
        .as_array()
        .ok_or("catalog.json has no entries")?;
    for entry in entries {
        ids.insert(
            entry["id"]
                .as_str()
                .ok_or("a catalog entry has no id")?
                .to_string(),
        );
    }
    let aliases_path = version_dir.join("aliases.json");
    if aliases_path.is_file() {
        let aliases: serde_json::Value = read_json(&aliases_path)?;
        for list in ["entries", "retired"] {
            for row in aliases[list].as_array().into_iter().flatten() {
                ids.insert(
                    row["old"]
                        .as_str()
                        .ok_or("an alias has no old id")?
                        .to_string(),
                );
            }
        }
    }
    let header: String = if existing.starts_with('#') {
        existing
            .lines()
            .take_while(|line| line.starts_with('#'))
            .map(|line| format!("{line}\n"))
            .collect()
    } else {
        PUBLISHED_IDS_HEADER.to_string()
    };
    let mut text = header;
    for id in &ids {
        text.push_str(id);
        text.push('\n');
    }
    fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!(
        "{PUBLISHED_IDS}: {} ids ({} added)",
        ids.len(),
        ids.len() - before
    );
    Ok(())
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    serde_json::from_slice(
        &fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", path.display()))
}

fn build_site(args: &[String]) -> Result {
    let mut public_url = "/cast-iron/".to_string();
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--public-url" => {
                public_url = rest.next().ok_or("--public-url needs a value")?.clone();
            }
            _ => return Err("usage: build-site [--public-url /path/]".into()),
        }
    }
    if !public_url.starts_with('/') || !public_url.ends_with('/') {
        return Err("--public-url must start and end with /".into());
    }
    let generated = "generated/content";
    if !root().join(generated).join("manifest.json").exists() {
        fetch_sources()?;
        content(&[])?;
    }
    let trunk = trunk_program();
    run(&trunk, &["build", "--release", "--public-url", &public_url])?;
    let dist = root().join("dist");
    copy_dir(&root().join(generated), &dist.join("content"))?;
    fs::write(dist.join(".nojekyll"), "").map_err(|e| e.to_string())?;
    content_address_snippets(&dist)?;
    stamp_shell(&dist)
}

/// Gives every wasm-bindgen snippet directory a content-derived name.
///
/// Trunk names `snippets/<crate>-<hash>/` from the crate, not from the snippet bytes,
/// so `bridge.js` keeps one URL across builds while the hashed glue file that imports
/// it changes. Any URL-keyed cache (the browser HTTP cache, Pages' `max-age`, a shell
/// cache from an earlier build on the same origin) can then pair new glue with an old
/// bridge and fail with "Importing binding name ... is not found". Renaming the
/// directory after its contents makes the import URL change exactly when the bridge
/// does. Only the glue `import` and the `modulepreload` link in `index.html` name the
/// path; the inline boot script does not, so its CSP hash is unaffected.
fn content_address_snippets(dist: &Path) -> Result {
    let snippets = dist.join("snippets");
    if !snippets.is_dir() {
        return Ok(());
    }
    let mut renames: Vec<(String, String)> = Vec::new();
    let mut dirs: Vec<PathBuf> = fs::read_dir(&snippets)
        .map_err(|e| format!("{}: {e}", snippets.display()))?
        .map(|item| item.map(|item| item.path()).map_err(|e| e.to_string()))
        .collect::<std::result::Result<_, _>>()?;
    dirs.sort();
    for dir in dirs.into_iter().filter(|dir| dir.is_dir()) {
        let mut files = Vec::new();
        list_files(&dir, &mut files)?;
        files.sort();
        let mut digest = Sha256::new();
        for file in &files {
            let relative = file.strip_prefix(&dir).map_err(|e| e.to_string())?;
            digest.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
            digest.update(b"\0");
            digest.update(fs::read(file).map_err(|e| e.to_string())?);
            digest.update(b"\0");
        }
        let old = dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("non-UTF-8 snippet directory")?
            .to_string();
        let new = format!("{old}-{}", &hex(&digest.finalize())[..12]);
        fs::rename(&dir, snippets.join(&new)).map_err(|e| e.to_string())?;
        renames.push((format!("snippets/{old}/"), format!("snippets/{new}/")));
    }
    let mut rewritten = 0;
    for item in fs::read_dir(dist).map_err(|e| e.to_string())? {
        let path = item.map_err(|e| e.to_string())?.path();
        let text_file = path
            .extension()
            .is_some_and(|ext| ext == "js" || ext == "html");
        if !path.is_file() || !text_file || path.file_name().is_some_and(|n| n == "sw.js") {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let html = path.extension().is_some_and(|ext| ext == "html");
        let mut updated = text.clone();
        for (old, new) in &renames {
            if html {
                // index.html names the bridge only in its modulepreload href.
                updated = updated.replace(old.as_str(), new.as_str());
            } else {
                // Rewrite module specifiers only. The glue also keys the wasm imports
                // object by the original path (`imports['./snippets/<old>/...']`), and
                // that key must keep matching the module name compiled into the wasm.
                for quote in ['\'', '"'] {
                    for lead in ["from ", "import("] {
                        updated = updated.replace(
                            &format!("{lead}{quote}./{old}"),
                            &format!("{lead}{quote}./{new}"),
                        );
                    }
                }
            }
        }
        if updated != text {
            fs::write(&path, updated).map_err(|e| e.to_string())?;
            rewritten += 1;
        }
    }
    if !renames.is_empty() && rewritten == 0 {
        return Err("snippet directories were renamed but no file referenced them".into());
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result {
    if to.exists() {
        fs::remove_dir_all(to).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for item in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let item = item.map_err(|e| e.to_string())?;
        let path = item.path();
        if path.is_dir() {
            copy_dir(&path, &to.join(item.file_name()))?;
        } else {
            fs::copy(&path, to.join(item.file_name())).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn list_files(dir: &Path, found: &mut Vec<PathBuf>) -> Result {
    for item in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = item.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            list_files(&path, found)?;
        } else {
            found.push(path);
        }
    }
    Ok(())
}

/// Replaces the placeholders in `dist/sw.js`. A browser installs a new service worker
/// only when the bytes of `sw.js` change, so the version derives from the shell's bytes.
fn stamp_shell(dist: &Path) -> Result {
    let sw_path = dist.join("sw.js");
    let template =
        fs::read_to_string(&sw_path).map_err(|e| format!("{}: {e}", sw_path.display()))?;
    let mut files = Vec::new();
    list_files(dist, &mut files)?;
    let mut shell: Vec<(String, String)> = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(dist)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("non-UTF-8 path")?
            .replace('\\', "/");
        // Dotfiles (.nojekyll, Trunk's .trunk-lock) are not part of the app.
        let hidden = relative.split('/').any(|part| part.starts_with('.'));
        if relative == "sw.js" || hidden || relative.starts_with("content/") {
            continue;
        }
        let bytes = fs::read(&file).map_err(|e| e.to_string())?;
        shell.push((relative, hex(&Sha256::digest(&bytes))));
    }
    shell.sort();
    let mut digest = Sha256::new();
    for (path, sha) in &shell {
        digest.update(format!("{path}:{sha}\n"));
    }
    let version = hex(&digest.finalize())[..16].to_string();
    let names: Vec<&str> = shell.iter().map(|(path, _)| path.as_str()).collect();
    let list = serde_json::to_string(&names).map_err(|e| e.to_string())?;
    for placeholder in ["__SHELL_VERSION__", "__SHELL_FILES__"] {
        if !template.contains(placeholder) {
            return Err(format!("{placeholder} is missing from dist/sw.js"));
        }
    }
    let stamped = template
        .replace("__SHELL_VERSION__", &version)
        .replace("\"__SHELL_FILES__\"", &list);
    fs::write(&sw_path, stamped).map_err(|e| e.to_string())?;
    println!("shell {version}: {} files", names.len());
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn corpus_check(args: &[String]) -> Result {
    cast_iron_corpus::cli::run_check(args)
}

fn corpus_fmt(args: &[String]) -> Result {
    cast_iron_corpus::cli::run_fmt(args)
}

fn check() -> Result {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run(
        "cargo",
        &[
            "clippy",
            "--package",
            "cast-iron-web",
            "--target",
            "wasm32-unknown-unknown",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace"])?;
    run(
        "cargo",
        &[
            "build",
            "--package",
            "cast-iron-web",
            "--package",
            "cast-iron-search",
            "--target",
            "wasm32-unknown-unknown",
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_toml_strings() {
        let text = "[dataset]\ncommit = \"abc\"\nother = \"x\"";
        assert_eq!(toml_string(text, "commit").as_deref(), Some("abc"));
        assert_eq!(toml_string(text, "missing"), None);
    }

    fn temp_tree(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cast-iron-xtask-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("entries")).unwrap();
        fs::write(dir.join("entries/a.md"), "alpha").unwrap();
        fs::write(dir.join("b.toml"), "beta").unwrap();
        dir
    }

    #[test]
    fn tree_hash_follows_the_documented_listing() {
        let dir = temp_tree("listing");
        let a = hex(&Sha256::digest(b"alpha"));
        let b = hex(&Sha256::digest(b"beta"));
        // Byte order of the path: "b.toml" sorts before "entries/a.md".
        let listing = format!("{b}  b.toml\n{a}  entries/a.md\n");
        assert_eq!(
            tree_hash(&dir).unwrap(),
            hex(&Sha256::digest(listing.as_bytes()))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tree_hash_changes_with_content_and_with_path() {
        let dir = temp_tree("change");
        let before = tree_hash(&dir).unwrap();
        fs::write(dir.join("b.toml"), "gamma").unwrap();
        let edited = tree_hash(&dir).unwrap();
        assert_ne!(before, edited);
        fs::write(dir.join("b.toml"), "beta").unwrap();
        fs::rename(dir.join("b.toml"), dir.join("c.toml")).unwrap();
        assert_ne!(before, tree_hash(&dir).unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn tree_hash_refuses_a_symbolic_link() {
        let dir = temp_tree("link");
        std::os::unix::fs::symlink("b.toml", dir.join("link")).unwrap();
        assert!(tree_hash(&dir).unwrap_err().contains("not a regular file"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_committed_corpus_pin_is_well_formed() {
        let pin = read_corpus_pin().unwrap();
        assert_eq!(pin.commit.len(), 40);
        assert_eq!(pin.tree_sha256.len(), 64);
        assert!(pin.repository.starts_with("https://github.com/"));
    }

    #[test]
    fn content_takes_no_arguments() {
        assert!(content(&["--psrd".to_string()]).is_err());
        assert!(content(&["--from-upstream".to_string()]).is_err());
    }

    #[test]
    fn snippet_directories_take_a_content_hash_and_references_follow() {
        let build = |bridge: &str| {
            let dir = std::env::temp_dir().join(format!(
                "xtask-snip-{}-{}",
                std::process::id(),
                bridge.len()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("snippets/web-1/js")).unwrap();
            fs::write(dir.join("snippets/web-1/js/bridge.js"), bridge).unwrap();
            fs::write(
                dir.join("web-abc.js"),
                "import * as b from './snippets/web-1/js/bridge.js';\nimports['./snippets/web-1/js/bridge.js'] = b;",
            )
            .unwrap();
            fs::write(
                dir.join("index.html"),
                "<link rel=\"modulepreload\" href=\"/x/snippets/web-1/js/bridge.js\">",
            )
            .unwrap();
            fs::write(dir.join("sw.js"), "snippets/web-1/").unwrap();
            content_address_snippets(&dir).unwrap();
            let names: Vec<String> = fs::read_dir(dir.join("snippets"))
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            assert_eq!(names.len(), 1);
            let name = names[0].clone();
            assert!(name.starts_with("web-1-") && name.len() == "web-1-".len() + 12);
            let glue = fs::read_to_string(dir.join("web-abc.js")).unwrap();
            assert!(
                glue.contains(&format!("./snippets/{name}/js/bridge.js")),
                "{glue}"
            );
            let index = fs::read_to_string(dir.join("index.html")).unwrap();
            assert!(index.contains(&format!("/x/snippets/{name}/js/bridge.js")));
            // sw.js is stamped afterwards from the directory listing, not rewritten here.
            assert_eq!(
                fs::read_to_string(dir.join("sw.js")).unwrap(),
                "snippets/web-1/"
            );
            fs::remove_dir_all(&dir).unwrap();
            name
        };
        let first = build("export function a() {}");
        assert_eq!(first, build("export function a() {}"), "deterministic");
        assert_ne!(
            first,
            build("export function a() {} export function b() {}")
        );
    }

    #[test]
    fn stamping_lists_shell_files_and_excludes_content() {
        let dir = std::env::temp_dir().join(format!("xtask-stamp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("content/v1")).unwrap();
        fs::write(
            dir.join("sw.js"),
            "v='__SHELL_VERSION__';f=\"__SHELL_FILES__\";",
        )
        .unwrap();
        fs::write(dir.join("index.html"), "x").unwrap();
        fs::write(dir.join(".trunk-lock"), "").unwrap();
        fs::write(dir.join("content/v1/a.json"), "{}").unwrap();
        stamp_shell(&dir).unwrap();
        let stamped = fs::read_to_string(dir.join("sw.js")).unwrap();
        assert!(stamped.contains("f=[\"index.html\"];"), "{stamped}");
        assert!(!stamped.contains("__SHELL_VERSION__"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
