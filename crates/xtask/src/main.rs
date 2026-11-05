//! Repository automation. Run `cargo xtask help`.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const HELP: &str = "\
cargo xtask <command>

  fetch-sources              Download the pinned PSRD-Data and Pathfinder Data 1e commits into
                             .cache/sources
  data-fetch                 Download the pinned Pathfinder Data 1e archive into .cache/sources/pathfinder-data-1-e
  data-audit                 Audit the Pathfinder Data 1e source
  data-import                Import the Pathfinder Data 1e source
  data-validate              Validate the Pathfinder Data 1e source
  data-report                Report Pathfinder Data 1e migration coverage
  data-legal                 Report Section 15 notice coverage for Pf Data publications
  data-alias                 Generate generated/content/<version>/aliases.json and run the alias gates
  data-alias-propose [--apply]
                             Propose dispositions for every no_source alias row; --apply rewrites id-map.toml
  data-carry-review [--apply] [--threshold N]
                             Compare carried PSRD entries with same-title Pf Data entries by
                             body similarity; --apply merges the accepted pairs into id-map.toml
  data-publications [--check]
                             Generate sources/canonical/publications.toml, or verify the committed copy
  data-test                  Test the importer and Pathfinder Data 1e source
  content [--psrd]           Compile the canonical library into generated/content (default), or
                             the PSRD-Data library into generated/psrd with --psrd
  build-site [--psrd] [--public-url P]
                             Build dist/ from generated/content (canonical by default), or from
                             generated/psrd with --psrd. The default public URL is /cast-iron/,
                             the GitHub Pages project path
  stamp-shell                Stamp dist/sw.js with the app shell file list and version
  check                      Run fmt, clippy, tests, and the WASM build
";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("fetch-sources") => fetch_sources(),
        Some("data-fetch") => data_fetch(),
        Some("data-audit") => data_tool("audit"),
        Some("data-import") => data_tool("import"),
        Some("data-validate") => data_tool("validate"),
        Some("data-report") => data_tool("report"),
        Some("data-legal") => data_tool("legal"),
        Some("data-alias") => data_alias(),
        Some("data-alias-propose") => data_alias_propose(&args[1..]),
        Some("data-carry-review") => data_carry_review(&args[1..]),
        Some("data-publications") => data_publications(&args[1..]),
        Some("data-test") => data_test(),
        Some("content") => content(&args[1..]),
        Some("build-site") => build_site(&args[1..]),
        Some("stamp-shell") => stamp_shell(&root().join("dist")),
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
    fetch_legacy_sources()?;
    data_fetch()
}

fn fetch_legacy_sources() -> Result {
    let config = fs::read_to_string(root().join("sources/psrd-data/source.toml"))
        .map_err(|e| format!("cannot read source.toml: {e}"))?;
    let commit = toml_string(&config, "commit").ok_or("source.toml has no commit")?;
    let target = root().join(".cache/sources/psrd-data");
    let stamp = target.join(".commit");
    if fs::read_to_string(&stamp).is_ok_and(|found| found == commit) {
        println!("PSRD-Data {commit} is already present");
        return Ok(());
    }
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| format!("cannot clear the cache: {e}"))?;
    }
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let archive = root().join(".cache/sources/psrd-data.tar.gz");
    let archive = archive.to_str().ok_or("non-UTF-8 path")?;
    let url = format!("https://codeload.github.com/devonjones/PSRD-Data/tar.gz/{commit}");
    run("curl", &["-fsSL", "--retry", "3", "-o", archive, &url])?;
    let target_str = target.to_str().ok_or("non-UTF-8 path")?;
    run(
        "tar",
        &["xzf", archive, "--strip-components=1", "-C", target_str],
    )?;
    fs::remove_file(archive).map_err(|e| e.to_string())?;
    // The compiler verifies input_digest, so a changed archive fails the content step.
    fs::write(&stamp, &commit).map_err(|e| e.to_string())?;
    Ok(())
}

fn data_fetch() -> Result {
    let repo = root();
    let config_path = repo.join("sources/pathfinder-data-1-e/source.json");
    let config: serde_json::Value = serde_json::from_slice(
        &fs::read(&config_path)
            .map_err(|e| format!("cannot read {}: {e}", config_path.display()))?,
    )
    .map_err(|e| format!("invalid source.json: {e}"))?;
    let string_field = |key: &str| -> Result<String> {
        config
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("source.json has no valid {key}"))
    };
    let commit = string_field("commit")?;
    let repository = string_field("repository")?;
    let expected = string_field("archive_sha256")?.to_ascii_lowercase();
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("source.json archive_sha256 must be 64 hexadecimal characters".into());
    }
    let repo_url = repository.trim_end_matches('/');
    let (owner, name) = repo_url
        .strip_prefix("https://github.com/")
        .ok_or("source.json repository must be an https://github.com/ URL")?
        .split_once('/')
        .ok_or("source.json repository must include an owner and repository")?;
    if owner.is_empty() || name.is_empty() || name.contains('/') || commit.contains('/') {
        return Err("source.json repository or commit is invalid".into());
    }

    let target = repo.join(".cache/sources/pathfinder-data-1-e");
    let parent = target.parent().ok_or("invalid cache target")?;
    fs::create_dir_all(parent).map_err(|e| format!("cannot create cache parent: {e}"))?;
    let files = config
        .get("files")
        .and_then(serde_json::Value::as_object)
        .ok_or("source.json has no input hashes")?;
    if files.is_empty() {
        return Err("source.json input hashes are empty".into());
    }
    let cache_verified = files.iter().all(|(path, expected)| {
        !path.starts_with('/')
            && !path.split('/').any(|p| p == "..")
            && fs::read(target.join(path))
                .is_ok_and(|bytes| Some(hex(&Sha256::digest(&bytes)).as_str()) == expected.as_str())
    });
    if target.join("json").is_dir()
        && fs::read_to_string(target.join(".commit")).is_ok_and(|found| found == commit)
        && cache_verified
    {
        println!("Pathfinder Data 1e {commit} is already present");
        return Ok(());
    }

    let unique = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let stage = parent.join(format!(".pathfinder-data-1-e-stage-{unique}"));
    let archive = parent.join(format!(".pathfinder-data-1-e-{unique}.tar.gz"));
    let url = format!("https://codeload.github.com/{owner}/{name}/tar.gz/{commit}");
    let archive_str = archive.to_str().ok_or("non-UTF-8 archive path")?;
    let operation = (|| -> Result {
        run("curl", &["-fsSL", "--retry", "3", "-o", archive_str, &url])?;
        let bytes =
            fs::read(&archive).map_err(|e| format!("cannot read downloaded archive: {e}"))?;
        let actual = hex(&Sha256::digest(&bytes));
        if actual != expected {
            return Err(format!(
                "archive SHA256 mismatch: expected {expected}, got {actual}"
            ));
        }
        fs::create_dir(&stage).map_err(|e| format!("cannot create staging directory: {e}"))?;
        let stage_str = stage.to_str().ok_or("non-UTF-8 staging path")?;
        run(
            "tar",
            &["xzf", archive_str, "--strip-components=1", "-C", stage_str],
        )?;
        if !stage.join("json").is_dir() {
            return Err(
                "downloaded archive does not contain the expected root json/ directory".into(),
            );
        }
        fs::write(stage.join(".commit"), &commit)
            .map_err(|e| format!("cannot write cache stamp: {e}"))?;
        Ok(())
    })();
    let _ = fs::remove_file(&archive);
    if let Err(error) = operation {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }

    let backup = parent.join(format!(".pathfinder-data-1-e-old-{unique}"));
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
    println!("Fetched Pathfinder Data 1e {commit}");
    Ok(())
}

fn data_tool(command: &str) -> Result {
    run_data_binary(command)
}

fn run_data_binary(command: &str) -> Result {
    run(
        "cargo",
        &[
            "run",
            "--release",
            "-p",
            "cast-iron-content-compiler",
            "--bin",
            "cast-iron-data",
            "--",
            command,
            "--dataset",
            ".cache/sources/pathfinder-data-1-e",
            "--registry",
            "sources/pathfinder-data-1-e/source.json",
            "--out",
            "generated/migration",
            "--reports",
            "reports",
            "--psrd-dataset",
            ".cache/sources/psrd-data",
            "--psrd-config",
            "sources/psrd-data/source.toml",
        ],
    )
}

fn data_test() -> Result {
    run("cargo", &["test", "-p", "cast-iron-content-compiler"])?;
    run_data_binary("test")
}

/// Regenerates the legacy-to-canonical alias file. The frozen snapshot and the
/// reviewed map are committed inputs; the generated file is build output.
fn data_alias() -> Result {
    run(
        "cargo",
        &[
            "run",
            "--release",
            "-p",
            "cast-iron-content-compiler",
            "--bin",
            "cast-iron-data",
            "--",
            "alias",
            "--dataset",
            ".cache/sources/pathfinder-data-1-e",
            "--registry",
            "sources/pathfinder-data-1-e/source.json",
            "--out",
            "generated/migration",
            "--reports",
            "reports",
            "--map",
            "sources/canonical/id-map.toml",
            "--legacy",
            "sources/canonical/legacy-ids.json",
            "--content",
            "generated/content",
            "--psrd-dataset",
            ".cache/sources/psrd-data",
            "--psrd-config",
            "sources/psrd-data/source.toml",
        ],
    )
}

/// Proposes alias dispositions from the same inventory the alias gate uses. The
/// extra arguments are forwarded, so `--apply` rewrites the committed map.
fn data_alias_propose(args: &[String]) -> Result {
    let mut arguments = vec![
        "run".to_string(),
        "--release".to_string(),
        "-p".to_string(),
        "cast-iron-content-compiler".to_string(),
        "--bin".to_string(),
        "cast-iron-data".to_string(),
        "--".to_string(),
        "alias-propose".to_string(),
        "--dataset".to_string(),
        ".cache/sources/pathfinder-data-1-e".to_string(),
        "--registry".to_string(),
        "sources/pathfinder-data-1-e/source.json".to_string(),
        "--out".to_string(),
        "generated/migration".to_string(),
        "--reports".to_string(),
        "reports".to_string(),
        "--map".to_string(),
        "sources/canonical/id-map.toml".to_string(),
        "--legacy".to_string(),
        "sources/canonical/legacy-ids.json".to_string(),
    ];
    arguments.extend(args.iter().cloned());
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    run("cargo", &borrowed)
}

/// Reviews carried entries against same-title canonical entries using body text.
/// The extra arguments are forwarded, so `--apply` and `--threshold` pass through.
fn data_carry_review(args: &[String]) -> Result {
    let mut arguments = vec![
        "run".to_string(),
        "--release".to_string(),
        "-p".to_string(),
        "cast-iron-content-compiler".to_string(),
        "--bin".to_string(),
        "cast-iron-data".to_string(),
        "--".to_string(),
        "carry-review".to_string(),
        "--dataset".to_string(),
        ".cache/sources/pathfinder-data-1-e".to_string(),
        "--registry".to_string(),
        "sources/pathfinder-data-1-e/source.json".to_string(),
        "--out".to_string(),
        "generated/migration".to_string(),
        "--reports".to_string(),
        "reports".to_string(),
        "--map".to_string(),
        "sources/canonical/id-map.toml".to_string(),
        "--legacy".to_string(),
        "sources/canonical/legacy-ids.json".to_string(),
        "--canonical-dir".to_string(),
        "sources/canonical".to_string(),
    ];
    arguments.extend(args.iter().cloned());
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    run("cargo", &borrowed)
}

/// Generates or checks the reviewed canonical publication registry.
fn data_publications(args: &[String]) -> Result {
    let check = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err("usage: data-publications [--check]".into()),
    };
    let mut arguments = vec![
        "run".to_string(),
        "--release".to_string(),
        "-p".to_string(),
        "cast-iron-content-compiler".to_string(),
        "--bin".to_string(),
        "cast-iron-data".to_string(),
        "--".to_string(),
        "publications".to_string(),
        "--dataset".to_string(),
        ".cache/sources/pathfinder-data-1-e".to_string(),
        "--registry".to_string(),
        "sources/pathfinder-data-1-e/source.json".to_string(),
        "--out".to_string(),
        "generated/migration".to_string(),
        "--reports".to_string(),
        "reports".to_string(),
        "--psrd-config".to_string(),
        "sources/psrd-data/source.toml".to_string(),
        "--publications".to_string(),
        "sources/canonical/publications.toml".to_string(),
    ];
    if check {
        arguments.push("--check".to_string());
    }
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    run("cargo", &borrowed)
}

/// The library a build command selects. The canonical library is the default;
/// `--psrd` keeps the carry-over input buildable for inspection and comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LibrarySource {
    Canonical,
    Psrd,
}

fn library_source(command: &str, args: &[String]) -> Result<LibrarySource> {
    let mut selected = None;
    for arg in args {
        let source = match arg.as_str() {
            "--psrd" => LibrarySource::Psrd,
            // Kept as an explicit spelling of the default for existing callers.
            "--canonical" => LibrarySource::Canonical,
            other => {
                return Err(format!(
                    "unknown argument {other:?} for {command}\nusage: {command} [--psrd]"
                ));
            }
        };
        if selected.replace(source).is_some() {
            return Err(format!(
                "{command} accepts only one of --canonical and --psrd"
            ));
        }
    }
    Ok(selected.unwrap_or(LibrarySource::Canonical))
}

fn content(args: &[String]) -> Result {
    let canonical = library_source("content", args)? == LibrarySource::Canonical;
    let mut arguments = vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--release".to_string(),
        "--package".to_string(),
        "cast-iron-content-compiler".to_string(),
        "--bin".to_string(),
        "cast-iron-compile".to_string(),
        "--".to_string(),
        "--source-config".to_string(),
        "sources/psrd-data/source.toml".to_string(),
        "--dataset".to_string(),
        ".cache/sources/psrd-data".to_string(),
        "--out".to_string(),
        if canonical {
            "generated/content".to_string()
        } else {
            "generated/psrd".to_string()
        },
        "--reports".to_string(),
        if canonical {
            "generated/reports".to_string()
        } else {
            "generated/reports/psrd".to_string()
        },
    ];
    if canonical {
        arguments.extend(
            [
                "--canonical",
                "--pf-dataset",
                ".cache/sources/pathfinder-data-1-e",
                "--pf-registry",
                "sources/pathfinder-data-1-e/source.json",
                "--canonical-dir",
                "sources/canonical",
            ]
            .map(String::from),
        );
    }
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    run("cargo", &borrowed)
}

fn build_site(args: &[String]) -> Result {
    let mut public_url = "/cast-iron/".to_string();
    let mut source = None;
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let selected = match flag.as_str() {
            "--canonical" => Some(LibrarySource::Canonical),
            "--psrd" => Some(LibrarySource::Psrd),
            "--public-url" => {
                public_url = rest.next().ok_or("--public-url needs a value")?.clone();
                None
            }
            _ => {
                return Err("usage: build-site [--psrd] [--public-url /path/]".into());
            }
        };
        if let Some(selected) = selected
            && source.replace(selected).is_some()
        {
            return Err("build-site accepts only one of --canonical and --psrd".into());
        }
    }
    if !public_url.starts_with('/') || !public_url.ends_with('/') {
        return Err("--public-url must start and end with /".into());
    }
    // The canonical library is the default. --psrd packages generated/psrd, the
    // carry-over input build, into the same dist/.
    let canonical = source.unwrap_or(LibrarySource::Canonical) == LibrarySource::Canonical;
    let generated = if canonical {
        "generated/content"
    } else {
        "generated/psrd"
    };
    if !root().join(generated).join("manifest.json").exists() {
        fetch_sources()?;
        let psrd_args = ["--psrd".to_string()];
        content(if canonical { &[] } else { &psrd_args })?;
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

fn check() -> Result {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run(
        "python3",
        &[
            "-B",
            "-m",
            "unittest",
            "discover",
            "-s",
            "tests/scripts",
            "-v",
        ],
    )?;
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

    #[test]
    fn content_source_defaults_to_canonical() {
        let none: Vec<String> = vec![];
        assert_eq!(
            library_source("content", &none).unwrap(),
            LibrarySource::Canonical
        );
        let psrd = ["--psrd".to_string()];
        assert_eq!(
            library_source("content", &psrd).unwrap(),
            LibrarySource::Psrd
        );
        let explicit = ["--canonical".to_string()];
        assert_eq!(
            library_source("content", &explicit).unwrap(),
            LibrarySource::Canonical
        );
        let both = ["--psrd".to_string(), "--canonical".to_string()];
        assert!(library_source("content", &both).is_err());
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
