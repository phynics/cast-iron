//! Repository automation. Run `cargo xtask help`.

mod aon_coverage;
mod docs_index;
mod feature_map;
mod legacy_sections;

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
  published-books            Append the books of the pinned corpus that have no slot yet to
                             sources/published-books.txt (`content` fails until you do)
  legacy-sections --books <books.json> --catalog <catalog.json> [--check]
                             Write sources/legacy-sections.json from the books.json and
                             catalog.json of a published library: the stable anchor of every
                             row that its chapter pages rendered, by positional anchor. Run once,
                             for the last library that made positional anchors (corpus v0.1.31)
  aon-coverage [--check]     Write docs/research/aon-path-coverage.{md,tsv}: every canonical entry
                             of the pinned corpus without an Archives of Nethys path, with
                             per-kind matched/missing/total counts. --check writes nothing and
                             fails when the committed files differ. Reads the corpus only
  check-book-slots [--base REF]
                             Fail unless sources/published-books.txt keeps every id line that
                             REF (default origin/main) has, in place; only appends are allowed
  entries-round-trip [dir]   Read every entry shard of a compiled library (default
                             generated/content) with the client's reader, write it again and
                             fail unless the bytes match. `content` runs it after the compile
  build-site [--public-url P]
                             Build dist/ from generated/content. The default public URL is
                             /cast-iron/, the GitHub Pages project path
  search-golden              Regenerate crates/search/tests/golden/canonical-top50.txt (the top-50
                             ranking of 50 queries) from generated/content, for the pinned
                             corpus. Run it in every corpus-pin PR, after `content`, and read the
                             diff for ranking regressions. The queries are the file's `query:` lines
  size-budget                Fail when generated/content or the WASM binary in dist/ is larger than
                             sources/size-budget.toml allows (run after build-site)
  stamp-shell                Stamp dist/sw.js with the app shell file list and version
  corpus-check <dir> [--drift-report <file>]
                             Read a plain-text corpus and validate ids, paths, sources,
                             and licences; exits non-zero and names the file and line.
                             Fails on fields-versus-body disagreements (drift)
  corpus-fmt <dir> [--check] Rewrite every corpus file into its canonical form, or with --check
                             list the first differing line of each file that is not canonical
  corpus-migrate <dir>       Convert a corpus from an older format (see docs/corpus-format.md)
  corpus-migrate-layout <dir>
                             Split books.toml into books/<publication>.toml, the layout step of
                             format 3 (#425), and nothing else. Safe to repeat
  corpus-migrate-features <dir>
                             Declare corpus format 4 (feature facts, #540) in corpus.toml and
                             change nothing else. Acts only from format 3. Safe to repeat
  corpus-migrate-trades <dir> [--review <file>]
                             Lower the trades of a corpus to format 2 and write the review queue
                             (TSV, default generated/trade-review.tsv). Idempotent
  decisions-index            Render the index of docs/decisions/ (by area) into target/docs-index/decisions.md;
                             nothing is committed or overwritten
  status-index               Render the index of docs/status/ into target/docs-index/status.md
  check                      Run fmt, clippy, tests, the feature-map check, the decisions and status
                             checks, the AoN coverage check, and the WASM build
";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("fetch-sources") => fetch_sources(),
        Some("corpus-tree-hash") => corpus_tree_hash(&args[1..]),
        Some("content") => content(&args[1..]),
        Some("entries-round-trip") => match &args[1..] {
            [] => entries_round_trip(&root().join("generated/content")),
            [dir] => entries_round_trip(Path::new(dir)),
            _ => Err("usage: entries-round-trip [content dir]".into()),
        },
        Some("published-ids") => published_ids(),
        Some("published-books") => published_books(),
        Some("legacy-sections") => legacy_sections::run(&args[1..]),
        Some("aon-coverage") => aon_coverage::run(&args[1..]),
        Some("check-book-slots") => check_book_slots(&args[1..]),
        Some("build-site") => build_site(&args[1..]),
        Some("search-golden") => search_golden(&args[1..]),
        Some("size-budget") => size_budget(&args[1..]),
        Some("stamp-shell") => stamp_shell(&root().join("dist"), commit_date().as_deref()),
        Some("corpus-check") => corpus_check(&args[1..]),
        Some("corpus-fmt") => corpus_fmt(&args[1..]),
        Some("corpus-migrate") => cast_iron_corpus::cli::run_migrate(&args[1..]),
        Some("corpus-migrate-layout") => cast_iron_corpus::cli::run_migrate_layout(&args[1..]),
        Some("corpus-migrate-features") => cast_iron_corpus::cli::run_migrate_features(&args[1..]),
        Some("corpus-migrate-trades") => corpus_migrate_trades(&args[1..]),
        Some("decisions-index") => docs_index::decisions_index(&root()),
        Some("status-index") => docs_index::status_index(&root()),
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
    tag: Option<String>,
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
        tag: toml_string(&text, "tag").filter(|value| !value.is_empty()),
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
    let tag = read_corpus_pin()?.tag.unwrap_or_default();
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
            "--book-slots",
            PUBLISHED_BOOKS,
            "--legacy-sections",
            legacy_sections::TABLE,
            "--corpus-tag",
            &tag,
        ],
    )?;
    // The compiler wrote the library. Read every entry back through the client's reader.
    entries_round_trip(&root().join("generated/content"))
}

/// Reads every `entries/*.json` shard of the library in `content` as the client does
/// (`Vec<RuleEntry>`), writes it again, and fails unless the bytes are the same.
///
/// The reader of `Block`, `Inline` and `TypedFields` is written by hand and requires the tag to
/// be the first key (`crates/schema/src/tagged.rs`). Its unit tests use small values; this runs
/// it on every entry of a real library. A shard the reader refuses, or reads into something that
/// writes differently, fails here and not on a reader's device. `content` runs it on the library
/// it just compiled; `cargo xtask entries-round-trip <dir>` runs it on any library on disk, such
/// as a retained copy of a released one.
fn entries_round_trip(content: &Path) -> Result {
    let manifest_path = content.join("manifest.json");
    let manifest: cast_iron_schema::ContentManifest = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?,
    )
    .map_err(|e| format!("{} is not a content manifest: {e}", manifest_path.display()))?;
    let mut shards = 0usize;
    let mut entries = 0usize;
    for file in &manifest.files {
        if !file.path.contains("/entries/") {
            continue;
        }
        let path = content.join(&file.path);
        let written =
            fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let read: Vec<cast_iron_schema::RuleEntry> = serde_json::from_slice(&written)
            .map_err(|e| format!("{}: the entry reader refuses it: {e}", path.display()))?;
        let again = serde_json::to_vec(&read).map_err(|e| e.to_string())?;
        if again != written {
            let at = again
                .iter()
                .zip(&written)
                .position(|(a, b)| a != b)
                .unwrap_or(again.len().min(written.len()));
            return Err(format!(
                "{}: read and written again, the bytes differ from byte {at} ({} bytes on disk, {} written again)",
                path.display(),
                written.len(),
                again.len()
            ));
        }
        shards += 1;
        entries += read.len();
    }
    if shards == 0 {
        return Err(format!("{} lists no entry shard", manifest_path.display()));
    }
    println!(
        "entry round trip: {entries} entries in {shards} shards of schema {} read and written back byte for byte",
        manifest.schema_version
    );
    Ok(())
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

/// Every book that a library has shipped, one id per line; the line number is the book's
/// permanent share slot (#408). `content` fails when a book of the corpus has no line.
const PUBLISHED_BOOKS: &str = "sources/published-books.txt";

const PUBLISHED_BOOKS_HEADER: &str = "\
# Every book id of a Cast Iron library, one per line. A book's slot is its position among the
# id lines, counted from 0. Share codes (the Books page, #408) name slots, so a code keeps its
# meaning when the library gains books or changes order. See
# docs/decisions/2026-11-16-book-share-codes.md.
#
# The file only grows, and its order is the data. `cargo xtask content` fails when a book of the
# pinned corpus has no line; run `cargo xtask published-books` and commit the added lines. Never
# edit, move or delete a line: that would change what every code already shared means.
# `cargo xtask check-book-slots` enforces this by comparing the file with its version on the
# base branch (CI runs it on every pull request, and `cargo xtask check` runs it against
# origin/main). A book that leaves the library keeps its line, and a code that names it is
# skipped on the receiver.
#
# The first lines are in Books order (tier, then oldest first), so a preset is a few runs of
# neighbouring slots and its code stays short. Later books are appended oldest first.
";

/// Appends the books of the pinned corpus that have no line in `sources/published-books.txt`,
/// oldest first (undated last, ties by id), after the lines that exist.
fn published_books() -> Result {
    verify_corpus_cache()?;
    let corpus = root().join(".cache/sources/cast-iron-data/publications.toml");
    let text = fs::read_to_string(&corpus)
        .map_err(|e| format!("cannot read {}: {e}", corpus.display()))?;
    let publications = cast_iron_corpus::read_publications(&corpus, &text)
        .map_err(|e| format!("{}: {e}", corpus.display()))?;
    let path = root().join(PUBLISHED_BOOKS);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let listed: std::collections::BTreeSet<&str> = existing
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let mut new: Vec<&cast_iron_corpus::Publication> = publications
        .iter()
        .filter(|publication| !listed.contains(publication.id.as_str()))
        .collect();
    new.sort_by(|a, b| {
        (a.publication_date.is_none(), &a.publication_date, &a.id).cmp(&(
            b.publication_date.is_none(),
            &b.publication_date,
            &b.id,
        ))
    });
    let mut text = if existing.is_empty() {
        PUBLISHED_BOOKS_HEADER.to_string()
    } else {
        existing.clone()
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    for publication in &new {
        text.push_str(&publication.id);
        text.push('\n');
    }
    fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!(
        "{PUBLISHED_BOOKS}: {} books ({} added)",
        listed.len() + new.len(),
        new.len()
    );
    Ok(())
}

/// The id lines of a `published-books.txt`: everything except blank lines and `#` comments.
fn book_slot_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// One line per way `current` breaks the slots that `base` established. Every id line of `base`
/// must be at the same position in `current`; `current` may only add lines at the end. Empty
/// when that holds, including when `current` equals `base`.
fn book_slot_problems(base: &str, current: &str) -> Vec<String> {
    let established = book_slot_lines(base);
    let now = book_slot_lines(current);
    let mut problems = Vec::new();
    for (slot, id) in established.iter().enumerate() {
        match now.get(slot) {
            Some(found) if found == id => {}
            Some(found) => {
                let moved = now.iter().position(|line| line == id);
                problems.push(match moved {
                    Some(at) => format!(
                        "slot {slot} was {id:?}, now {found:?}; {id:?} moved to slot {at} (a reorder)"
                    ),
                    // The next established line took its place: the line was deleted.
                    None if established.get(slot + 1) == Some(found) => format!(
                        "slot {slot} ({id:?}) was removed, so every later slot moved up"
                    ),
                    None => format!("slot {slot} was {id:?}, now {found:?} (a replacement)"),
                });
            }
            None => problems.push(format!("slot {slot} ({id:?}) was removed")),
        }
    }
    problems
}

/// Reads what `git ls-tree --name-only <base> -- <path>` said: `true` when the file is in the
/// base's tree, `false` when the base has no such file (the change that introduces it), and an
/// error for any git failure, with git's own message.
fn baseline_present(success: bool, stdout: &str, stderr: &str, base: &str) -> Result<bool> {
    if !success {
        return Err(format!(
            "check-book-slots: cannot list the tree of {base} ({}); fetch it, or pass a base that is here",
            stderr.trim()
        ));
    }
    Ok(!stdout.trim().is_empty())
}

/// Fails unless `sources/published-books.txt` only appends to the file at `--base` (default
/// `origin/main`). A slot is permanent: a share code names slots, so a reorder, replacement or
/// deletion would silently change what every code already shared means. A base that does not
/// have the file yet (the change that introduces it) passes. With no `--base` and no
/// `origin/main` to compare with, the check says so and passes: CI always names the base.
fn check_book_slots(args: &[String]) -> Result {
    let mut base: Option<String> = None;
    let mut rest = args.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--base" => {
                base = Some(rest.next().ok_or("--base needs a ref")?.clone());
            }
            other => {
                return Err(format!(
                    "unknown argument {other:?} for check-book-slots\nusage: check-book-slots [--base REF]"
                ));
            }
        }
    }
    let explicit = base.is_some();
    let base = base.unwrap_or_else(|| "origin/main".to_string());
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root())
            .output()
            .map_err(|e| format!("cannot start git: {e}"))
    };
    let commit = format!("{base}^{{commit}}");
    if !git(&["rev-parse", "--verify", "--quiet", &commit])?
        .status
        .success()
    {
        if explicit {
            return Err(format!(
                "check-book-slots: {base} is not a commit here; fetch it first"
            ));
        }
        println!("check-book-slots: no {base} to compare with, skipped (CI passes --base)");
        return Ok(());
    }
    let spec = format!("{base}:{PUBLISHED_BOOKS}");
    // Ask the tree whether the file exists before reading it, so that "absent at base" is the
    // only thing that passes: a git failure (a tree that is not here, a broken repository) must
    // not look like a base without the file.
    let listed = git(&["ls-tree", "--name-only", &base, "--", PUBLISHED_BOOKS])?;
    let present = baseline_present(
        listed.status.success(),
        &String::from_utf8_lossy(&listed.stdout),
        &String::from_utf8_lossy(&listed.stderr),
        &base,
    )?;
    if !present {
        println!("check-book-slots: {base} has no {PUBLISHED_BOOKS} yet; nothing to keep");
        return Ok(());
    }
    let shown = git(&["show", &spec])?;
    if !shown.status.success() {
        return Err(format!(
            "check-book-slots: cannot read {spec}: {}",
            String::from_utf8_lossy(&shown.stderr).trim()
        ));
    }
    let base_text = String::from_utf8(shown.stdout).map_err(|_| format!("{spec} is not UTF-8"))?;
    let path = root().join(PUBLISHED_BOOKS);
    let current =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let problems = book_slot_problems(&base_text, &current);
    if problems.is_empty() {
        println!(
            "check-book-slots: {PUBLISHED_BOOKS} keeps all {} slots of {base}",
            book_slot_lines(&base_text).len()
        );
        return Ok(());
    }
    let mut message = format!(
        "{PUBLISHED_BOOKS} changes slots that {base} already established; a share code names slots, so it only appends:"
    );
    for problem in problems.iter().take(20) {
        message.push_str(&format!("\n  {problem}"));
    }
    if problems.len() > 20 {
        message.push_str(&format!("\n  ... and {} more", problems.len() - 20));
    }
    Err(message)
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
    // The compiler writes the manifest only after every gate has passed, and moves the whole
    // library into place at once. This check refuses a library that is partly there or that
    // changed after the compile, whatever wrote it.
    verify_generated_content(&root().join(generated))?;
    let trunk = trunk_program();
    run(&trunk, &["build", "--release", "--public-url", &public_url])?;
    let dist = root().join("dist");
    copy_dir(&root().join(generated), &dist.join("content"))?;
    fs::write(dist.join(".nojekyll"), "").map_err(|e| e.to_string())?;
    content_address_snippets(&dist)?;
    let date = commit_date();
    stamp_shell(&dist, date.as_deref())
}

/// Checks the library in `content` against its `manifest.json`: the manifest first, then every
/// file it lists. Returns the number of files checked.
///
/// The manifest must pass the structural check that the installer applies before it downloads
/// (`cast_iron_schema::manifest_check`): a known schema version, a valid content version, safe
/// and unique paths, valid checksums, the required files, and a total that equals the sum of the
/// sizes. Each listed file must then exist with the listed size and SHA-256. This establishes that
/// the directory is the library its manifest describes. It cannot tell a library that passed the
/// compile gates from one that did not: the compiler guarantees that by writing no manifest when
/// a gate fails.
fn verify_generated_content(content: &Path) -> Result<usize> {
    let manifest_path = content.join("manifest.json");
    let manifest: cast_iron_schema::ContentManifest = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?,
    )
    .map_err(|e| format!("{} is not a content manifest: {e}", manifest_path.display()))?;
    cast_iron_schema::manifest_check::validate_manifest(&manifest).map_err(|e| {
        format!(
            "{}: {e} Run `cargo xtask content` again",
            manifest_path.display()
        )
    })?;
    for file in &manifest.files {
        let data = fs::read(content.join(&file.path)).map_err(|e| {
            format!(
                "{} lists {}, which cannot be read ({e}); run `cargo xtask content` again",
                manifest_path.display(),
                file.path
            )
        })?;
        if data.len() as u64 != file.bytes || hex(&Sha256::digest(&data)) != file.sha256 {
            return Err(format!(
                "{} does not match {}; the library is partly written or changed after the compile. Run `cargo xtask content` again",
                file.path,
                manifest_path.display()
            ));
        }
    }
    Ok(manifest.files.len())
}

/// The size limits in `sources/size-budget.toml`.
const SIZE_BUDGET: &str = "sources/size-budget.toml";

/// Compares `measured` (name, bytes) with the limits in `budget_text`. Returns one report line
/// per measure, or every overrun in one message.
fn compare_budget(
    budget_text: &str,
    measured: &[(&str, u64)],
) -> std::result::Result<Vec<String>, String> {
    let mut report = Vec::new();
    let mut over = Vec::new();
    for (name, bytes) in measured {
        let limit: u64 = toml_string(&budget_text.replace('"', ""), name)
            .and_then(|value| value.trim().replace('_', "").parse().ok())
            .ok_or_else(|| format!("{SIZE_BUDGET} has no integer `{name}`"))?;
        let percent = *bytes as f64 * 100.0 / limit.max(1) as f64;
        report.push(format!(
            "{name}: {bytes} of {limit} bytes ({percent:.1}% of the budget)"
        ));
        if *bytes > limit {
            over.push(format!(
                "{name} is {bytes} bytes, {} over the budget of {limit}",
                bytes - limit
            ));
        }
    }
    if over.is_empty() {
        Ok(report)
    } else {
        Err(format!(
            "the build is over its size budget:\n  {}\nIf the growth is intended, raise the value in {SIZE_BUDGET} in the same pull request; a reviewer approves that line (docs/releasing.md, \"Size budget\")",
            over.join("\n  ")
        ))
    }
}

/// Measures `generated/content` and the WASM binary in `dist/` against `sources/size-budget.toml`.
const SEARCH_GOLDEN: &str = "crates/search/tests/golden/canonical-top50.txt";

/// Rewrites the canonical top-50 ranking golden from the compiled library. The queries stay: they
/// are the `query: ` lines of the file itself. The header records the corpus commit (which the
/// always-run test compares with `sources/corpus.toml`) and the content version.
fn search_golden(args: &[String]) -> Result {
    if let Some(argument) = args.first() {
        return Err(format!(
            "unknown argument {argument:?} for search-golden\nusage: search-golden"
        ));
    }
    let manifest: serde_json::Value = read_json(&root().join("generated/content/manifest.json"))
        .map_err(|e| format!("{e}\nrun `cargo xtask content` first"))?;
    let version = manifest["content_version"]
        .as_str()
        .ok_or("manifest.json has no content_version")?;
    let index = format!("generated/content/{version}/search.bin");
    let pin = read_corpus_pin()?;
    let golden = root().join(SEARCH_GOLDEN);
    println!("$ cargo run --release -p cast-iron-search --example dump_rankings");
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--release",
            "--package",
            "cast-iron-search",
            "--example",
            "dump_rankings",
            "--",
            &index,
            SEARCH_GOLDEN,
        ])
        .current_dir(root())
        .output()
        .map_err(|e| format!("cannot start cargo: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "dump_rankings failed: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let blocks = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let tag = pin.tag.as_deref().unwrap_or("untagged");
    let text = format!(
        "# Top-50 result ids for the 50 queries of tests/e2e/measure-library.mjs.\n\
         # Regenerate with `cargo xtask search-golden` (after `cargo xtask content`) in every corpus-pin PR.\n\
         # corpus {commit}\n\
         # tag {tag}\n\
         # content {version}\n\
         # Format: `query: <text>` starts a block; each other line is a result id.\n\
         {blocks}",
        commit = pin.commit
    );
    fs::write(&golden, text).map_err(|e| format!("cannot write {}: {e}", golden.display()))?;
    println!(
        "wrote {SEARCH_GOLDEN} for corpus {} ({tag}), content {version}",
        pin.commit
    );
    Ok(())
}

fn size_budget(args: &[String]) -> Result {
    if let Some(argument) = args.first() {
        return Err(format!(
            "unknown argument {argument:?} for size-budget\nusage: size-budget"
        ));
    }
    let mut library = Vec::new();
    list_files(&root().join("generated/content"), &mut library)
        .map_err(|e| format!("{e}; run `cargo xtask content` first"))?;
    let sizes = library
        .iter()
        .map(|path| {
            fs::metadata(path)
                .map(|meta| meta.len())
                .map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect::<Result<Vec<u64>>>()?;
    let mut wasm = Vec::new();
    for item in fs::read_dir(root().join("dist"))
        .map_err(|e| format!("cannot read dist: {e}; run `cargo xtask build-site` first"))?
    {
        let path = item.map_err(|e| e.to_string())?.path();
        if path.to_string_lossy().ends_with("_bg.wasm") {
            wasm.push(path);
        }
    }
    let [wasm] = wasm.as_slice() else {
        return Err(format!(
            "expected one *_bg.wasm in dist, found {}",
            wasm.len()
        ));
    };
    let wasm_bytes = fs::metadata(wasm)
        .map_err(|e| format!("{}: {e}", wasm.display()))?
        .len();
    let budget = fs::read_to_string(root().join(SIZE_BUDGET))
        .map_err(|e| format!("cannot read {SIZE_BUDGET}: {e}"))?;
    let report = compare_budget(
        &budget,
        &[
            ("library_bytes", sizes.iter().sum()),
            (
                "largest_file_bytes",
                sizes.iter().copied().max().unwrap_or(0),
            ),
            ("wasm_bytes", wasm_bytes),
        ],
    )?;
    for line in report {
        println!("{line}");
    }
    Ok(())
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
/// `date` is the ISO date of the commit being built; `None` leaves it empty, so Settings
/// shows the shell hash alone.
fn stamp_shell(dist: &Path, date: Option<&str>) -> Result {
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
    for placeholder in ["__SHELL_VERSION__", "__SHELL_FILES__", "__SHELL_DATE__"] {
        if !template.contains(placeholder) {
            return Err(format!("{placeholder} is missing from dist/sw.js"));
        }
    }
    let stamped = template
        .replace("__SHELL_VERSION__", &version)
        .replace("\"__SHELL_FILES__\"", &list)
        .replace("__SHELL_DATE__", date.unwrap_or(""));
    fs::write(&sw_path, stamped).map_err(|e| e.to_string())?;
    println!("shell {version}: {} files", names.len());
    Ok(())
}

/// The commit date of `HEAD` as ISO `YYYY-MM-DD`, or `None` when git is unavailable (a tarball
/// build). The date comes from the commit, never from the wall clock, so a build is reproducible.
fn commit_date() -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cs", "HEAD"])
        .current_dir(root())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let date = String::from_utf8(output.stdout).ok()?;
    let date = date.trim();
    is_iso_date(date).then(|| date.to_string())
}

fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
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

/// The review queue goes to `generated/trade-review.tsv` unless `--review` says otherwise, so
/// it never lands inside the corpus.
fn corpus_migrate_trades(args: &[String]) -> Result {
    let mut args = args.to_vec();
    if !args.iter().any(|arg| arg == "--review") {
        let review = root().join("generated").join("trade-review.tsv");
        args.push("--review".to_string());
        args.push(review.display().to_string());
    }
    cast_iron_corpus::cli::run_migrate_trades(&args)
}

fn check() -> Result {
    feature_map::check(&root())?;
    docs_index::check(&root())?;
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
    check_book_slots(&[])?;
    check_aon_coverage()?;
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

/// The AoN coverage report against the pinned corpus. `check` normally runs without a fetched
/// corpus (a fresh clone has no `.cache`), so the step says that it skipped rather than failing;
/// run `cargo xtask fetch-sources` to include it.
fn check_aon_coverage() -> Result {
    if !corpus_cache().is_dir() {
        println!(
            "skip: the AoN coverage report needs .cache/sources/cast-iron-data; run `cargo xtask fetch-sources` to compare it"
        );
        return Ok(());
    }
    aon_coverage::check()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "# header\nalpha\nbeta\ngamma\n";

    const BUDGET: &str =
        "# comment\nlibrary_bytes = 1_000\nlargest_file_bytes = 400\nwasm_bytes = 50\n";

    #[test]
    fn a_build_under_the_size_budget_passes() {
        let report = compare_budget(
            BUDGET,
            &[
                ("library_bytes", 1_000),
                ("largest_file_bytes", 10),
                ("wasm_bytes", 49),
            ],
        )
        .unwrap();
        assert_eq!(report.len(), 3);
        assert!(report[0].contains("100.0%"), "{report:?}");
    }

    #[test]
    fn a_build_over_the_size_budget_fails_and_names_every_overrun() {
        let error = compare_budget(
            BUDGET,
            &[
                ("library_bytes", 1_001),
                ("largest_file_bytes", 400),
                ("wasm_bytes", 90),
            ],
        )
        .unwrap_err();
        assert!(
            error.contains("library_bytes is 1001 bytes, 1 over"),
            "{error}"
        );
        assert!(error.contains("wasm_bytes is 90 bytes, 40 over"), "{error}");
        assert!(!error.contains("largest_file_bytes is"), "{error}");
        assert!(error.contains("sources/size-budget.toml"), "{error}");
    }

    #[test]
    fn a_missing_budget_line_fails() {
        let error = compare_budget("library_bytes = 5\n", &[("wasm_bytes", 1)]).unwrap_err();
        assert!(error.contains("no integer `wasm_bytes`"), "{error}");
    }

    #[test]
    fn the_committed_size_budget_names_every_measure() {
        let text = fs::read_to_string(root().join(SIZE_BUDGET)).unwrap();
        let report = compare_budget(
            &text,
            &[
                ("library_bytes", 0),
                ("largest_file_bytes", 0),
                ("wasm_bytes", 0),
            ],
        );
        assert!(report.is_ok(), "{report:?}");
    }

    /// A library on disk whose manifest lists the four required files.
    fn write_library(dir: &Path) -> cast_iron_schema::ContentManifest {
        let version = "vabc";
        let mut files = Vec::new();
        for name in cast_iron_schema::manifest_check::REQUIRED_FILES {
            let data = format!("{{\"{name}\":1}}");
            fs::create_dir_all(dir.join(version)).unwrap();
            fs::write(dir.join(version).join(name), &data).unwrap();
            files.push(cast_iron_schema::ManifestFile {
                path: format!("{version}/{name}"),
                sha256: hex(&Sha256::digest(data.as_bytes())),
                bytes: data.len() as u64,
                gzip: None,
            });
        }
        let manifest = cast_iron_schema::ContentManifest {
            schema_version: cast_iron_schema::SCHEMA_VERSION,
            content_version: version.into(),
            total_bytes: files.iter().map(|f| f.bytes).sum(),
            files,
            entry_count: 0,
            datasets: vec![],
            corpus_tag: None,
        };
        write_manifest(dir, &manifest);
        manifest
    }

    fn write_manifest(dir: &Path, manifest: &cast_iron_schema::ContentManifest) {
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_vec(manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn a_library_that_matches_its_manifest_passes() {
        let dir = temp_tree("verify-ok");
        write_library(&dir);
        assert_eq!(verify_generated_content(&dir), Ok(4));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_manifest_with_the_wrong_structure_is_refused_before_any_file_is_read() {
        let dir = temp_tree("verify-structure");
        // Not a manifest at all, and the empty file list the first version of this check accepted.
        fs::write(dir.join("manifest.json"), r#"{"files":[]}"#).unwrap();
        let error = verify_generated_content(&dir).unwrap_err();
        assert!(error.contains("is not a content manifest"), "{error}");
        let good = write_library(&dir);

        let mut empty = good.clone();
        empty.files.clear();
        empty.total_bytes = 0;
        write_manifest(&dir, &empty);
        let error = verify_generated_content(&dir).unwrap_err();
        assert!(error.contains("does not list the required file"), "{error}");

        // A required row removed together with its file.
        let mut missing = good.clone();
        let removed = missing.files.remove(0);
        missing.total_bytes -= removed.bytes;
        fs::remove_file(dir.join(&removed.path)).unwrap();
        write_manifest(&dir, &missing);
        let error = verify_generated_content(&dir).unwrap_err();
        assert!(error.contains("required file"), "{error}");
        fs::write(dir.join(&removed.path), "x").unwrap();

        let mut unsafe_path = good.clone();
        unsafe_path.files[0].path = "vabc/../x".into();
        write_manifest(&dir, &unsafe_path);
        assert!(
            verify_generated_content(&dir)
                .unwrap_err()
                .contains("invalid file path")
        );

        let mut duplicate = good.clone();
        duplicate.files.push(duplicate.files[0].clone());
        duplicate.total_bytes += duplicate.files[0].bytes;
        write_manifest(&dir, &duplicate);
        assert!(
            verify_generated_content(&dir)
                .unwrap_err()
                .contains("twice")
        );

        let mut total = good.clone();
        total.total_bytes += 1;
        write_manifest(&dir, &total);
        assert!(
            verify_generated_content(&dir)
                .unwrap_err()
                .contains("declares")
        );

        let mut version = good.clone();
        version.content_version = "ABC".into();
        write_manifest(&dir, &version);
        assert!(
            verify_generated_content(&dir)
                .unwrap_err()
                .contains("invalid version")
        );

        let mut schema = good.clone();
        schema.schema_version = cast_iron_schema::SCHEMA_VERSION + 1;
        write_manifest(&dir, &schema);
        assert!(
            verify_generated_content(&dir)
                .unwrap_err()
                .contains("content format")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_listed_file_that_changed_or_is_missing_is_refused() {
        let dir = temp_tree("verify-files");
        let manifest = write_library(&dir);
        let first = dir.join(&manifest.files[0].path);
        fs::write(&first, "[]").unwrap();
        let error = verify_generated_content(&dir).unwrap_err();
        assert!(error.contains("does not match"), "{error}");
        fs::remove_file(&first).unwrap();
        let error = verify_generated_content(&dir).unwrap_err();
        assert!(error.contains("cannot be read"), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn book_slots_may_only_be_appended() {
        // The same file, a different header, and a valid append all keep the slots.
        assert!(book_slot_problems(BASE, BASE).is_empty());
        assert!(book_slot_problems(BASE, "# other header\n\nalpha\nbeta\ngamma\n").is_empty());
        assert!(
            book_slot_problems(BASE, "# header\nalpha\nbeta\ngamma\ndelta\nepsilon\n").is_empty()
        );
        // The change that introduces the file has nothing to keep.
        assert!(book_slot_problems("", BASE).is_empty());
    }

    #[test]
    fn only_an_absent_baseline_file_passes() {
        // The tree lists the file: compare. The tree is fine but has no file: nothing to keep.
        assert_eq!(
            baseline_present(true, "sources/published-books.txt\n", "", "b"),
            Ok(true)
        );
        assert_eq!(baseline_present(true, "", "", "b"), Ok(false));
        // Any git failure is an error that carries git's message, never a pass.
        let failed =
            baseline_present(false, "", "fatal: bad object abc123\n", "abc123").unwrap_err();
        assert!(
            failed.contains("fatal: bad object abc123") && failed.contains("abc123"),
            "{failed}"
        );
    }

    #[test]
    fn a_reorder_a_replacement_and_a_deletion_are_named() {
        let reorder = book_slot_problems(BASE, "alpha\ngamma\nbeta\n");
        assert_eq!(reorder.len(), 2, "{reorder:?}");
        assert!(
            reorder[0].contains("slot 1") && reorder[0].contains("reorder"),
            "{reorder:?}"
        );
        let replacement = book_slot_problems(BASE, "alpha\nBETA\ngamma\n");
        assert_eq!(replacement.len(), 1, "{replacement:?}");
        assert!(replacement[0].contains("replacement"), "{replacement:?}");
        // Deleting a line in the middle shifts the rest: every later slot is wrong.
        let middle = book_slot_problems(BASE, "alpha\ngamma\n");
        assert!(
            middle
                .iter()
                .any(|p| p.contains("slot 2") && p.contains("removed")),
            "{middle:?}"
        );
        // Deleting the last line.
        let last = book_slot_problems(BASE, "alpha\nbeta\n");
        assert_eq!(last.len(), 1, "{last:?}");
        assert!(
            last[0].contains("gamma") && last[0].contains("removed"),
            "{last:?}"
        );
        // Deleting everything.
        assert_eq!(book_slot_problems(BASE, "# only a header\n").len(), 3);
    }

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
        assert!(pin.tag.as_deref().is_some_and(|tag| tag.starts_with('v')));
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
            "v='__SHELL_VERSION__';f=\"__SHELL_FILES__\";d='__SHELL_DATE__';",
        )
        .unwrap();
        fs::write(dir.join("index.html"), "x").unwrap();
        fs::write(dir.join(".trunk-lock"), "").unwrap();
        fs::write(dir.join("content/v1/a.json"), "{}").unwrap();
        stamp_shell(&dir, Some("2026-11-07")).unwrap();
        let stamped = fs::read_to_string(dir.join("sw.js")).unwrap();
        assert!(stamped.contains("f=[\"index.html\"];"), "{stamped}");
        assert!(stamped.contains("d='2026-11-07';"), "{stamped}");
        assert!(!stamped.contains("__SHELL_VERSION__"));
        assert!(!stamped.contains("__SHELL_DATE__"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stamping_without_a_commit_date_leaves_the_date_empty() {
        let dir = std::env::temp_dir().join(format!("xtask-stamp-nodate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("sw.js"),
            "v='__SHELL_VERSION__';f=\"__SHELL_FILES__\";d='__SHELL_DATE__';",
        )
        .unwrap();
        fs::write(dir.join("index.html"), "x").unwrap();
        stamp_shell(&dir, None).unwrap();
        let stamped = fs::read_to_string(dir.join("sw.js")).unwrap();
        assert!(stamped.contains("d='';"), "{stamped}");
        assert!(!stamped.contains("__SHELL_DATE__"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_an_iso_commit_date_is_accepted() {
        assert!(is_iso_date("2026-11-07"));
        for bad in [
            "",
            "2026-1-7",
            "2026/11/07",
            "07 Nov 2026",
            "2026-11-07T00:00",
        ] {
            assert!(!is_iso_date(bad), "{bad:?}");
        }
    }
}
