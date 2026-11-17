//! Decisions and the status log: one Markdown file per entry, with front matter and a generated index.
//!
//! `docs/decisions/YYYY-MM-DD-<slug>.md` holds one decision and `docs/status/YYYY-MM-DD-<slug>.md`
//! one pin, release or acceptance record. Two pull requests that each add a file do not conflict
//! in the text of the entries. `cargo xtask decisions-index` and `cargo xtask status-index` write
//! the README of each directory; `cargo xtask check` fails when a README is stale, when front matter
//! is invalid, and when a relative link in the entries or in the two top pages does not resolve.
//!
//! Front matter is `key: value` lines between two `---` lines. A string and a list are JSON
//! (`title: "..."`, `issues: [394, 420]`); `date`, `status`, `area` and `kind` are bare words.
//! Decision keys: `title`, `date`, `status` (`current`, `superseded` or `partly-superseded`),
//! `superseded_by` (file names without `.md`; empty exactly when the status is `current`), `issues`
//! and `prs` (ascending numbers), `area`. Status keys: `title`, `date`, `kind`.

use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

type Result<T = ()> = std::result::Result<T, String>;

const DECISION_STATUSES: [&str; 3] = ["current", "superseded", "partly-superseded"];

/// The areas of the decisions index, in the order of its sections: key and heading.
const AREAS: [(&str, &str); 8] = [
    ("corpus", "Corpus"),
    ("content", "Content library"),
    ("search", "Search"),
    ("ui", "UI"),
    ("install-offline", "Install and offline"),
    ("releases", "Releases"),
    ("process", "Process"),
    ("licensing", "Licensing"),
];

const STATUS_KINDS: [&str; 3] = ["pin", "release", "record"];

const REPO: &str = "https://github.com/phynics/cast-iron";

#[derive(Debug, PartialEq)]
struct Decision {
    stem: String,
    title: String,
    date: String,
    status: String,
    superseded_by: Vec<String>,
    issues: Vec<u32>,
    prs: Vec<u32>,
    area: String,
}

#[derive(Debug, PartialEq)]
struct StatusEntry {
    stem: String,
    title: String,
    date: String,
    kind: String,
}

/// Splits a file into its front matter lines and the text after them.
fn split_front_matter(text: &str) -> Result<(Vec<(String, String)>, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .ok_or("the file does not start with a `---` front matter line")?;
    let end = rest
        .find("\n---\n")
        .ok_or("the front matter has no closing `---` line")?;
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in rest[..end].split('\n') {
        let (key, value) = line
            .split_once(": ")
            .ok_or_else(|| format!("front matter line {line:?} is not `key: value`"))?;
        if fields.iter().any(|(seen, _)| seen == key) {
            return Err(format!("front matter key {key:?} occurs twice"));
        }
        fields.push((key.to_string(), value.to_string()));
    }
    Ok((fields, &rest[end + "\n---\n".len()..]))
}

fn require_keys(fields: &[(String, String)], keys: &[&str]) -> Result {
    for key in keys {
        if !fields.iter().any(|(name, _)| name == key) {
            return Err(format!("front matter has no `{key}`"));
        }
    }
    for (name, _) in fields {
        if !keys.contains(&name.as_str()) {
            return Err(format!("front matter has an unknown key `{name}`"));
        }
    }
    Ok(())
}

fn field<'a>(fields: &'a [(String, String)], key: &str) -> &'a str {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map_or("", |(_, value)| value.as_str())
}

fn json_string(value: &str, key: &str) -> Result<String> {
    match serde_json::from_str::<serde_json::Value>(value) {
        Ok(serde_json::Value::String(text)) if !text.trim().is_empty() => Ok(text),
        _ => Err(format!("`{key}` must be a non-empty JSON string")),
    }
}

fn json_strings(value: &str, key: &str) -> Result<Vec<String>> {
    match serde_json::from_str::<Vec<String>>(value) {
        Ok(list) => Ok(list),
        Err(_) => Err(format!("`{key}` must be a JSON list of strings")),
    }
}

fn json_numbers(value: &str, key: &str) -> Result<Vec<u32>> {
    let list = serde_json::from_str::<Vec<u32>>(value)
        .map_err(|_| format!("`{key}` must be a JSON list of numbers"))?;
    if list.contains(&0) || list.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(format!(
            "`{key}` must list positive numbers in ascending order, each once"
        ));
    }
    Ok(list)
}

/// `YYYY-MM-DD`, with a month 1 to 12 and a day 1 to 31.
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let numeric = |part: &str, width: usize| {
        part.len() == width && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    if parts.len() != 3 || !numeric(parts[0], 4) || !numeric(parts[1], 2) || !numeric(parts[2], 2) {
        return false;
    }
    let (month, day) = (
        parts[1].parse::<u32>().unwrap_or(0),
        parts[2].parse::<u32>().unwrap_or(0),
    );
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// The date of a file stem `YYYY-MM-DD-<slug>`, when the stem has that shape.
fn stem_date(stem: &str) -> Result<&str> {
    let slug_ok = |slug: &str| {
        !slug.is_empty()
            && !slug.starts_with('-')
            && !slug.ends_with('-')
            && !slug.contains("--")
            && slug
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    };
    match (stem.get(..10), stem.get(10..11), stem.get(11..)) {
        (Some(date), Some("-"), Some(slug)) if is_date(date) && slug_ok(slug) => Ok(date),
        _ => Err("the file name must be `YYYY-MM-DD-<slug>.md` with a lowercase ASCII slug".into()),
    }
}

fn check_body(body: &str) -> Result {
    match body.lines().find(|line| !line.trim().is_empty()) {
        Some(line) if line.starts_with("# ") => Ok(()),
        _ => Err("the text after the front matter must start with a `# ` heading".into()),
    }
}

fn parse_decision(stem: &str, text: &str) -> Result<Decision> {
    let date_of_name = stem_date(stem)?;
    let (fields, body) = split_front_matter(text)?;
    require_keys(
        &fields,
        &[
            "title",
            "date",
            "status",
            "superseded_by",
            "issues",
            "prs",
            "area",
        ],
    )?;
    let date = field(&fields, "date");
    if !is_date(date) || date != date_of_name {
        return Err(format!(
            "`date: {date}` must be a valid date equal to the one in the file name ({date_of_name})"
        ));
    }
    let status = field(&fields, "status");
    if !DECISION_STATUSES.contains(&status) {
        return Err(format!(
            "`status: {status}` must be one of {}",
            DECISION_STATUSES.join(", ")
        ));
    }
    let area = field(&fields, "area");
    if !AREAS.iter().any(|(key, _)| *key == area) {
        let known: Vec<&str> = AREAS.iter().map(|(key, _)| *key).collect();
        return Err(format!(
            "`area: {area}` must be one of {}",
            known.join(", ")
        ));
    }
    let superseded_by = json_strings(field(&fields, "superseded_by"), "superseded_by")?;
    if (status == "current") != superseded_by.is_empty() {
        return Err(
            "`superseded_by` must be empty when the status is `current`, and name an entry otherwise"
                .into(),
        );
    }
    check_body(body)?;
    Ok(Decision {
        stem: stem.to_string(),
        title: json_string(field(&fields, "title"), "title")?,
        date: date.to_string(),
        status: status.to_string(),
        superseded_by,
        issues: json_numbers(field(&fields, "issues"), "issues")?,
        prs: json_numbers(field(&fields, "prs"), "prs")?,
        area: area.to_string(),
    })
}

fn parse_status(stem: &str, text: &str) -> Result<StatusEntry> {
    let date_of_name = stem_date(stem)?;
    let (fields, body) = split_front_matter(text)?;
    require_keys(&fields, &["title", "date", "kind"])?;
    let date = field(&fields, "date");
    if !is_date(date) || date != date_of_name {
        return Err(format!(
            "`date: {date}` must be a valid date equal to the one in the file name ({date_of_name})"
        ));
    }
    let kind = field(&fields, "kind");
    if !STATUS_KINDS.contains(&kind) {
        return Err(format!(
            "`kind: {kind}` must be one of {}",
            STATUS_KINDS.join(", ")
        ));
    }
    check_body(body)?;
    Ok(StatusEntry {
        stem: stem.to_string(),
        title: json_string(field(&fields, "title"), "title")?,
        date: date.to_string(),
        kind: kind.to_string(),
    })
}

/// Every `*.md` file of `dir` except `README.md`, as (stem, text), in file name order.
fn read_entries(dir: &Path) -> Result<Vec<(String, String, PathBuf)>> {
    let mut found = Vec::new();
    let listing = fs::read_dir(dir).map_err(|error| format!("read {}: {error}", dir.display()))?;
    for item in listing {
        let path = item.map_err(|error| error.to_string())?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        if name == "README.md" {
            continue;
        }
        let Some(stem) = name.strip_suffix(".md") else {
            return Err(format!(
                "{}: only `.md` entry files and README.md belong in {}",
                path.display(),
                dir.display()
            ));
        };
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        found.push((stem.to_string(), text, path));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(found)
}

fn load_decisions(dir: &Path) -> Result<Vec<Decision>> {
    let mut decisions = Vec::new();
    for (stem, text, path) in read_entries(dir)? {
        decisions.push(
            parse_decision(&stem, &text).map_err(|error| format!("{}: {error}", path.display()))?,
        );
    }
    let stems: BTreeSet<&str> = decisions.iter().map(|d| d.stem.as_str()).collect();
    for decision in &decisions {
        for target in &decision.superseded_by {
            if !stems.contains(target.as_str()) {
                return Err(format!(
                    "{}/{}.md: `superseded_by` names {target:?}, which is not a file in the directory",
                    dir.display(),
                    decision.stem
                ));
            }
            if *target == decision.stem {
                return Err(format!(
                    "{}/{}.md: an entry cannot supersede itself",
                    dir.display(),
                    decision.stem
                ));
            }
        }
    }
    Ok(decisions)
}

fn load_status(dir: &Path) -> Result<Vec<StatusEntry>> {
    let mut entries = Vec::new();
    for (stem, text, path) in read_entries(dir)? {
        entries.push(
            parse_status(&stem, &text).map_err(|error| format!("{}: {error}", path.display()))?,
        );
    }
    Ok(entries)
}

/// A table cell: a pipe would end the cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

fn reference_links(issues: &[u32], prs: &[u32]) -> String {
    let mut links: Vec<String> = issues
        .iter()
        .map(|number| format!("[#{number}]({REPO}/issues/{number})"))
        .collect();
    links.extend(
        prs.iter()
            .map(|number| format!("[PR #{number}]({REPO}/pull/{number})")),
    );
    links.join(", ")
}

fn render_decisions(decisions: &[Decision]) -> String {
    let mut out = String::from(
        "# Decisions\n\n\
Generated by `cargo xtask decisions-index`. Do not edit this file.\n\n\
Each decision is one file in this directory, named `YYYY-MM-DD-<slug>.md`, with front matter and the\n\
decision text. To record a decision, add a file; `cargo xtask check` validates the front matter and fails\n\
when this index is stale. A decision that a later one replaced stays here, with its status and a link to\n\
the entry that replaced it. `docs/spec.md` stays the product specification; these entries refine it.\n\n\
Front matter: `title`, `date`, `status` (`current`, `superseded` or `partly-superseded`), `superseded_by`\n\
(file names without `.md`), `issues`, `prs`, and `area`. The areas are listed below.\n",
    );
    let title_of = |stem: &str| {
        decisions
            .iter()
            .find(|d| d.stem == stem)
            .map_or_else(|| stem.to_string(), |d| d.title.clone())
    };
    for (key, heading) in AREAS {
        let mut rows: Vec<&Decision> = decisions.iter().filter(|d| d.area == key).collect();
        if rows.is_empty() {
            continue;
        }
        rows.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.stem.cmp(&b.stem)));
        out.push_str(&format!(
            "\n## {heading}\n\n| Date | Decision | Status | Issues |\n|---|---|---|---|\n"
        ));
        for d in rows {
            let status = if d.superseded_by.is_empty() {
                d.status.clone()
            } else {
                let by: Vec<String> = d
                    .superseded_by
                    .iter()
                    .map(|stem| format!("[{}]({stem}.md)", cell(&title_of(stem))))
                    .collect();
                format!("{} by {}", d.status.replace('-', " "), by.join("; "))
            };
            out.push_str(&format!(
                "| {} | [{}]({}.md) | {} | {} |\n",
                d.date,
                cell(&d.title),
                d.stem,
                status,
                reference_links(&d.issues, &d.prs)
            ));
        }
    }
    out
}

fn render_status(entries: &[StatusEntry]) -> String {
    let mut out = String::from(
        "# Status log\n\n\
Generated by `cargo xtask status-index`. Do not edit this file.\n\n\
Each pin, release or acceptance record is one file in this directory, named `YYYY-MM-DD-<slug>.md`, with\n\
front matter (`title`, `date`, `kind`: `pin`, `release` or `record`) and the text of the record. A pin PR\n\
adds a file here. [Current state](../status.md) holds what is true now.\n\n\
| Date | Kind | Entry |\n|---|---|---|\n",
    );
    let mut rows: Vec<&StatusEntry> = entries.iter().collect();
    rows.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.stem.cmp(&b.stem)));
    for entry in rows {
        out.push_str(&format!(
            "| {} | {} | [{}]({}.md) |\n",
            entry.date,
            entry.kind,
            cell(&entry.title),
            entry.stem
        ));
    }
    out
}

fn decisions_dir(root: &Path) -> PathBuf {
    root.join("docs").join("decisions")
}

fn status_dir(root: &Path) -> PathBuf {
    root.join("docs").join("status")
}

pub fn decisions_index(root: &Path) -> Result {
    let dir = decisions_dir(root);
    let text = render_decisions(&load_decisions(&dir)?);
    fs::write(dir.join("README.md"), text).map_err(|error| error.to_string())
}

pub fn status_index(root: &Path) -> Result {
    let dir = status_dir(root);
    let text = render_status(&load_status(&dir)?);
    fs::write(dir.join("README.md"), text).map_err(|error| error.to_string())
}

fn check_fresh(dir: &Path, rendered: &str, command: &str) -> Result {
    let path = dir.join("README.md");
    match fs::read_to_string(&path) {
        Ok(found) if found == rendered => Ok(()),
        Ok(_) => Err(format!(
            "{} is stale. Run `cargo xtask {command}` and commit the result",
            path.display()
        )),
        Err(_) => Err(format!(
            "{} is missing. Run `cargo xtask {command}` and commit the result",
            path.display()
        )),
    }
}

/// The relative Markdown links of `text` that do not name an existing file. Code blocks and
/// inline code are skipped; links with a scheme, a bare `#anchor` and absolute paths are not files
/// of this repository.
fn broken_links(text: &str, from_dir: &Path) -> Vec<String> {
    let link = Regex::new(r"\[[^\]]*\]\(([^)\s]+)\)").expect("the link pattern is valid");
    let code_span = Regex::new(r"`[^`]*`").expect("the code span pattern is valid");
    let mut broken = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let line = code_span.replace_all(line, "");
        for captures in link.captures_iter(&line) {
            let target = &captures[1];
            let file = target.split('#').next().unwrap_or_default();
            let has_scheme = target.split_once(':').is_some_and(|(scheme, _)| {
                scheme
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'.' || b == b'-')
            });
            if file.is_empty() || has_scheme || file.starts_with('/') {
                continue;
            }
            if !from_dir.join(file).exists() {
                broken.push(target.to_string());
            }
        }
    }
    broken
}

fn check_links(root: &Path) -> Result {
    let mut problems = Vec::new();
    let mut files: Vec<PathBuf> = vec![
        root.join("docs").join("decisions.md"),
        root.join("docs").join("status.md"),
    ];
    for dir in [decisions_dir(root), status_dir(root)] {
        let listing =
            fs::read_dir(&dir).map_err(|error| format!("read {}: {error}", dir.display()))?;
        for item in listing {
            let path = item.map_err(|error| error.to_string())?.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    for path in files {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let dir = path.parent().unwrap_or(root);
        for target in broken_links(&text, dir) {
            problems.push(format!(
                "{}: the link ({target}) does not resolve",
                path.display()
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// `cargo xtask check` step: valid front matter, fresh indexes, resolving links.
pub fn check(root: &Path) -> Result {
    let decisions = load_decisions(&decisions_dir(root))?;
    check_fresh(
        &decisions_dir(root),
        &render_decisions(&decisions),
        "decisions-index",
    )?;
    let status = load_status(&status_dir(root))?;
    check_fresh(&status_dir(root), &render_status(&status), "status-index")?;
    check_links(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision_text(extra: &str) -> String {
        format!(
            "---\ntitle: \"A title\"\ndate: 2026-11-17\nstatus: current\nsuperseded_by: []\nissues: [3, 9]\nprs: []\narea: ui\n{extra}---\n\n# A title, 2026-11-17\n\nText.\n"
        )
    }

    const STEM: &str = "2026-11-17-a-title";

    #[test]
    fn a_valid_decision_parses() {
        let parsed = parse_decision(STEM, &decision_text("")).unwrap();
        assert_eq!(parsed.title, "A title");
        assert_eq!(parsed.issues, vec![3, 9]);
        assert!(parsed.superseded_by.is_empty());
    }

    #[test]
    fn a_title_keeps_quotes_backticks_and_unicode() {
        let text = decision_text("").replace("\"A title\"", "\"Book \\\"x\\\" `source_ids` é\"");
        assert_eq!(
            parse_decision(STEM, &text).unwrap().title,
            "Book \"x\" `source_ids` é"
        );
    }

    #[test]
    fn invalid_front_matter_is_named() {
        let cases = [
            (
                decision_text("").replace("status: current", "status: old"),
                "status",
            ),
            (decision_text("").replace("area: ui", "area: web"), "area"),
            (
                decision_text("").replace("date: 2026-11-17", "date: 2026-11-16"),
                "date",
            ),
            (
                decision_text("").replace("date: 2026-11-17", "date: 2026-13-17"),
                "date",
            ),
            (decision_text("").replace("[3, 9]", "[9, 3]"), "ascending"),
            (decision_text("").replace("[3, 9]", "[3, 3]"), "ascending"),
            (decision_text("").replace("[3, 9]", "[0]"), "ascending"),
            (decision_text("").replace("[3, 9]", "[\"3\"]"), "numbers"),
            (decision_text("").replace("\"A title\"", "A title"), "title"),
            (decision_text("").replace("\"A title\"", "\"\""), "title"),
            (decision_text("kind: pin\n"), "unknown key"),
            (decision_text("").replace("prs: []\n", ""), "no `prs`"),
            (
                decision_text("").replace("area: ui\n", "area: ui\narea: ui\n"),
                "twice",
            ),
            (
                decision_text("").replace("superseded_by: []", "superseded_by: [\"x\"]"),
                "superseded_by",
            ),
            (
                decision_text("").replace("status: current", "status: superseded"),
                "superseded_by",
            ),
            (
                decision_text("").replace("# A title, 2026-11-17", "A title"),
                "heading",
            ),
            (decision_text("").replacen("---\n", "", 1), "front matter"),
        ];
        for (text, needle) in cases {
            let error = parse_decision(STEM, &text).unwrap_err();
            assert!(error.contains(needle), "{needle:?} not in {error:?}");
        }
    }

    #[test]
    fn a_file_name_must_carry_a_date_and_a_clean_slug() {
        for stem in [
            "a-title",
            "2026-11-17",
            "2026-11-17-",
            "2026-11-17-A-title",
            "2026-11-17-a--title",
            "2026-11-17-a_title",
            "2026-13-17-a-title",
        ] {
            assert!(stem_date(stem).is_err(), "{stem} should be refused");
        }
        assert_eq!(stem_date("2026-11-17-a-title-2"), Ok("2026-11-17"));
    }

    fn write_tree(name: &str, decisions: &[(&str, String)], status: &[(&str, String)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cast-iron-docs-index-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(decisions_dir(&root)).unwrap();
        fs::create_dir_all(status_dir(&root)).unwrap();
        for (stem, text) in decisions {
            fs::write(decisions_dir(&root).join(format!("{stem}.md")), text).unwrap();
        }
        for (stem, text) in status {
            fs::write(status_dir(&root).join(format!("{stem}.md")), text).unwrap();
        }
        fs::write(root.join("docs").join("decisions.md"), "# Decisions\n").unwrap();
        fs::write(root.join("docs").join("status.md"), "# Status\n").unwrap();
        root
    }

    fn status_text() -> String {
        "---\ntitle: \"Corpus pinned to v1\"\ndate: 2026-11-16\nkind: pin\n---\n\n# Corpus pinned to v1, 2026-11-16\n\nText.\n"
            .to_string()
    }

    #[test]
    fn a_status_entry_is_checked_like_a_decision() {
        assert!(parse_status("2026-11-16-corpus-pinned-to-v1", &status_text()).is_ok());
        let bad = status_text().replace("kind: pin", "kind: log");
        assert!(
            parse_status("2026-11-16-corpus-pinned-to-v1", &bad)
                .unwrap_err()
                .contains("kind")
        );
    }

    #[test]
    fn an_index_is_fresh_only_after_it_is_generated() {
        let root = write_tree(
            "fresh",
            &[(STEM, decision_text(""))],
            &[("2026-11-16-corpus-pinned-to-v1", status_text())],
        );
        assert!(check(&root).unwrap_err().contains("decisions-index"));
        decisions_index(&root).unwrap();
        assert!(check(&root).unwrap_err().contains("status-index"));
        status_index(&root).unwrap();
        check(&root).unwrap();
        // A new decision makes the committed index stale.
        let newer = decision_text("").replace("A title", "Another");
        fs::write(decisions_dir(&root).join("2026-11-17-another.md"), newer).unwrap();
        assert!(check(&root).unwrap_err().contains("stale"));
        decisions_index(&root).unwrap();
        check(&root).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_index_groups_by_area_and_links_what_replaced_an_entry() {
        let old = decision_text("")
            .replace("status: current", "status: partly-superseded")
            .replace("superseded_by: []", "superseded_by: [\"2026-11-18-newer\"]");
        let newer = decision_text("")
            .replace("A title", "Newer | one")
            .replace("2026-11-17", "2026-11-18")
            .replace("area: ui", "area: search")
            .replace("[3, 9]", "[]")
            .replace("prs: []", "prs: [12]");
        let root = write_tree("grouped", &[(STEM, old), ("2026-11-18-newer", newer)], &[]);
        let index = render_decisions(&load_decisions(&decisions_dir(&root)).unwrap());
        assert!(index.contains("## Search"), "{index}");
        assert!(index.find("## Search").unwrap() < index.find("## UI").unwrap());
        assert!(index.contains(
            "| 2026-11-17 | [A title](2026-11-17-a-title.md) | partly superseded by [Newer \\| one](2026-11-18-newer.md) | [#3](https://github.com/phynics/cast-iron/issues/3), [#9](https://github.com/phynics/cast-iron/issues/9) |"
        ));
        assert!(index.contains("[PR #12](https://github.com/phynics/cast-iron/pull/12)"));
        assert!(!index.contains("## Corpus"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_superseding_entry_must_exist_and_not_be_the_entry_itself() {
        let missing = decision_text("")
            .replace("status: current", "status: superseded")
            .replace("superseded_by: []", "superseded_by: [\"2026-11-18-gone\"]");
        let root = write_tree("missing", &[(STEM, missing)], &[]);
        assert!(
            load_decisions(&decisions_dir(&root))
                .unwrap_err()
                .contains("not a file")
        );
        let itself = decision_text("")
            .replace("status: current", "status: superseded")
            .replace("superseded_by: []", &format!("superseded_by: [\"{STEM}\"]"));
        fs::write(decisions_dir(&root).join(format!("{STEM}.md")), itself).unwrap();
        assert!(
            load_decisions(&decisions_dir(&root))
                .unwrap_err()
                .contains("itself")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn links_must_resolve_but_code_and_urls_are_not_files() {
        let root = write_tree("links", &[], &[]);
        let dir = decisions_dir(&root);
        fs::write(dir.join("exists.md"), "x").unwrap();
        let text = "See [one](exists.md#a), [two](missing.md), [three](https://example.org/x.md), \
[four](#here), `[five](nothing.md)`.\n```\n[six](nothing.md)\n```\n[seven](../decisions.md)\n";
        assert_eq!(broken_links(text, &dir), vec!["missing.md".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_committed_entries_and_indexes_are_valid_and_fresh() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        check(&root).unwrap();
    }
}
