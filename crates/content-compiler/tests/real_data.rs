//! Spot checks against the real PSRD-Data snapshot. They skip when `.cache/sources/psrd-data`
//! is missing, so CI without the dataset still passes.

mod common;

use cast_iron_content_compiler::text::{blocks_text, squash};
use common::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Turns a upstream HTML string into the plain-text pieces that must appear in the output.
/// Block-level tags split pieces, so words never merge across paragraphs.
fn pieces(html: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            for t in chars.by_ref() {
                if t == '>' {
                    break;
                }
                tag.push(t);
            }
            let name: String = tag
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if matches!(
                name.as_str(),
                "p" | "br"
                    | "div"
                    | "li"
                    | "ul"
                    | "ol"
                    | "tr"
                    | "td"
                    | "th"
                    | "table"
                    | "caption"
                    | "thead"
                    | "tbody"
                    | "h1"
                    | "h2"
                    | "h3"
            ) {
                out.push(String::new());
            }
        } else {
            out.last_mut().unwrap().push(c);
        }
    }
    out.into_iter()
        .map(|p| {
            squash(
                &p.replace("&ldquo;", "\u{201c}")
                    .replace("&rdquo;", "\u{201d}")
                    .replace("&mdash;", "\u{2014}")
                    .replace("&ndash;", "\u{2013}")
                    .replace("&times;", "\u{d7}")
                    .replace("&nbsp;", " ")
                    .replace("&apos;", "'")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&deg;", "\u{b0}")
                    .replace("&hellip;", "\u{2026}")
                    .replace("&mdash", "\u{2014}")
                    .replace("&amp;", "&"),
            )
        })
        .filter(|p| !p.is_empty())
        .collect()
}

fn strings(value: &Value, key: &str, out: &mut Vec<String>) {
    strings_skipping(value, key, &Default::default(), out)
}

/// Like `strings`, but leaves out subtrees whose `url` is another entry's record.
fn strings_skipping(
    value: &Value,
    key: &str,
    skip: &std::collections::BTreeSet<String>,
    out: &mut Vec<String>,
) {
    match value {
        Value::Object(map)
            if map
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|u| skip.contains(u)) => {}
        Value::String(s)
            if !matches!(key, "url" | "source" | "type" | "subtype" | "image" | "alt") =>
        {
            out.push(s.clone())
        }
        Value::Array(items) => items
            .iter()
            .for_each(|v| strings_skipping(v, key, skip, out)),
        Value::Object(map) => map
            .iter()
            .for_each(|(k, v)| strings_skipping(v, k, skip, out)),
        _ => {}
    }
}

fn assert_all_text_present(dataset: &Path, rel: &str, entry: &cast_iron_schema::RuleEntry) {
    let json: Value = serde_json::from_slice(&std::fs::read(dataset.join(rel)).unwrap()).unwrap();
    let mut all = Vec::new();
    strings(&json, "", &mut all);
    let haystack = squash(&format!("{} {}", entry.title, blocks_text(&entry.body)));
    let mut missing = Vec::new();
    let mut checked = 0;
    for s in &all {
        for piece in pieces(s) {
            checked += 1;
            if !haystack.contains(&piece) {
                missing.push(piece);
            }
        }
    }
    assert!(checked > 10, "{rel}: only {checked} pieces were checked");
    assert!(
        missing.is_empty(),
        "{rel}: text missing from the output: {missing:#?}"
    );
}

#[test]
fn aasimar_and_fireball_keep_all_of_their_text() {
    let dataset = workspace().join(".cache/sources/psrd-data");
    if !dataset.join("core_rulebook/spell/fireball.json").exists() {
        eprintln!("skipping: {} is not present", dataset.display());
        return;
    }
    let scratch = scratch("real-data");
    let config = scratch.join("config");
    std::fs::create_dir_all(&config).unwrap();
    for name in ["source.toml", "legal-notices.json", "id-overrides.toml"] {
        std::fs::copy(
            workspace().join("sources/psrd-data").join(name),
            config.join(name),
        )
        .unwrap();
    }
    let mut opts = options(
        &config,
        &dataset,
        &scratch.join("out"),
        &scratch.join("reports"),
    );
    opts.write_digest = true; // the scratch copy only
    let outcome = run(&opts);
    assert!(outcome.entry_count > 1000, "{}", outcome.entry_count);
    let lib = load(&scratch.join("out"));
    assert_all_text_present(
        &dataset,
        "core_rulebook/spell/fireball.json",
        lib.entry("core-rulebook/spells/fireball"),
    );
    assert_all_text_present(
        &dataset,
        "bestiary/creature/aasimar.json",
        lib.entry("bestiary/monsters/aasimar"),
    );
    assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
    // Every typed entry keeps every string that the upstream record holds.
    let mut failures = Vec::new();
    for entry in lib
        .entries
        .iter()
        .filter(|e| e.kind != cast_iron_schema::EntryKind::Rule)
    {
        let path = &entry.provenance[0].path;
        let json: Value =
            serde_json::from_slice(&std::fs::read(dataset.join(path)).unwrap()).unwrap();
        let mut all = Vec::new();
        strings(&json, "", &mut all);
        let haystack = squash(&format!("{} {}", entry.title, blocks_text(&entry.body)));
        for s in &all {
            for piece in pieces(s) {
                if !haystack.contains(&piece) {
                    failures.push(format!("{}: {piece:?}", entry.id));
                }
            }
        }
    }
    // Each rules file keeps its own text across the parent entry and its section entries.
    let typed_urls: std::collections::BTreeSet<String> = lib
        .entries
        .iter()
        .filter(|e| e.kind != cast_iron_schema::EntryKind::Rule)
        .map(|e| e.provenance[0].record.clone())
        .collect();
    for file in lib
        .entries
        .iter()
        .filter(|e| e.kind == cast_iron_schema::EntryKind::Rule && e.parent.is_none())
    {
        let path = &file.provenance[0].path;
        let json: Value =
            serde_json::from_slice(&std::fs::read(dataset.join(path)).unwrap()).unwrap();
        let mut all = Vec::new();
        strings_skipping(&json, "", &typed_urls, &mut all);
        let haystack = squash(
            &lib.entries
                .iter()
                .filter(|e| {
                    e.kind == cast_iron_schema::EntryKind::Rule && e.provenance[0].path == *path
                })
                .map(|e| format!("{} {}", e.title, blocks_text(&e.body)))
                .collect::<Vec<_>>()
                .join(" "),
        );
        for s in &all {
            for piece in pieces(s) {
                if !haystack.contains(&piece) {
                    failures.push(format!("{path}: {piece:?}"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} strings missing, first ones: {:#?}",
        failures.len(),
        &failures[..failures.len().min(25)]
    );
    // The pinned digest in the repository matches the snapshot.
    let pinned =
        std::fs::read_to_string(workspace().join("sources/psrd-data/source.toml")).unwrap();
    let computed = std::fs::read_to_string(config.join("source.toml")).unwrap();
    assert_eq!(
        pinned, computed,
        "sources/psrd-data/source.toml holds a stale input_digest"
    );
}
