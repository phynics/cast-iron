//! Repository automation. Run `cargo xtask help`.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const HELP: &str = "\
cargo xtask <command>

  fetch-sources              Download the pinned PSRD-Data commit into .cache/sources/psrd-data
  content                    Compile content into generated/content and generated/reports
  build-site [--public-url P]  Compile content when missing, build the app, and write dist/
                             The default public URL is /cast-iron/, the GitHub Pages project path
  stamp-shell                Stamp dist/sw.js with the app shell file list and version
  check                      Run fmt, clippy, tests, and the WASM build
";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("fetch-sources") => fetch_sources(),
        Some("content") => content(),
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

fn content() -> Result {
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
            "--source-config",
            "sources/psrd-data/source.toml",
            "--dataset",
            ".cache/sources/psrd-data",
            "--out",
            "generated/content",
            "--reports",
            "generated/reports",
        ],
    )
}

fn build_site(args: &[String]) -> Result {
    let public_url = match args {
        [] => "/cast-iron/".to_string(),
        [flag, value] if flag == "--public-url" => value.clone(),
        _ => return Err("usage: build-site [--public-url /path/]".into()),
    };
    if !public_url.starts_with('/') || !public_url.ends_with('/') {
        return Err("--public-url must start and end with /".into());
    }
    if !root().join("generated/content/manifest.json").exists() {
        fetch_sources()?;
        content()?;
    }
    let trunk = trunk_program();
    run(&trunk, &["build", "--release", "--public-url", &public_url])?;
    let dist = root().join("dist");
    copy_dir(&root().join("generated/content"), &dist.join("content"))?;
    fs::write(dist.join(".nojekyll"), "").map_err(|e| e.to_string())?;
    stamp_shell(&dist)
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
