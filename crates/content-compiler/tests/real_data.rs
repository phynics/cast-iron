//! Spot checks against the real PSRD-Data snapshot. They skip when `.cache/sources/psrd-data`
//! is missing, so CI without the dataset still passes.

mod common;

use cast_iron_content_compiler::text::{blocks_text, squash};
use cast_iron_schema::TypedFields;
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
    let mut raw = String::new();
    while let Some(c) = chars.next() {
        if c == '<' {
            flush_text(&mut raw, out.last_mut().unwrap());
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
            raw.push(c);
        }
    }
    flush_text(&mut raw, out.last_mut().unwrap());
    out.into_iter()
        .map(|p| squash(&p))
        .filter(|p| !p.is_empty())
        .collect()
}

// Decode each text node before joining across inline markup. This keeps malformed
// split entities such as <i>&ldquo</i>; distinct from a valid &ldquo; entity.
fn flush_text(raw: &mut String, target: &mut String) {
    let text = std::mem::take(raw)
        .replace("&ndash1;", "\u{2013}1")
        .replace("&mdash;", "\u{2014}")
        .replace("&mdash", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
        .replace("&ndash", "\u{2013}")
        .replace("&ldquo;", "\u{201c}")
        .replace("&ldquo", "\u{201c}")
        .replace("&rdquo;", "\u{201d}")
        .replace("&rdquo", "\u{201d}");
    let fragment = scraper::Html::parse_fragment(&text);
    target.extend(
        fragment
            .root_element()
            .text()
            .flat_map(str::chars)
            .filter(|c| *c != '\u{ad}'),
    );
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

fn retained_json(dataset: &Path, rel: &str) -> Value {
    let mut node: cast_iron_content_compiler::model::Node =
        serde_json::from_slice(&std::fs::read(dataset.join(rel)).unwrap()).unwrap();
    let policy = cast_iron_content_compiler::policy::ContentPolicy::load(
        &workspace().join("sources/psrd-data/content-policy.toml"),
    )
    .unwrap();
    assert!(policy.retain(&mut node, rel, &mut Vec::new()));
    node.to_value()
}

fn record_by_url<'a>(node: &'a Value, url: &str) -> Option<&'a Value> {
    if node.get("url").and_then(Value::as_str) == Some(url) {
        return Some(node);
    }
    node.get("sections")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|child| record_by_url(child, url))
}

fn assert_all_text_present(
    dataset: &Path,
    rel: &str,
    entry: &cast_iron_schema::RuleEntry,
    typed_urls: &std::collections::BTreeSet<String>,
) {
    let json = retained_json(dataset, rel);
    let mut all = Vec::new();
    let mut replaced = typed_urls.clone();
    replaced.remove(&entry.provenance[0].record);
    strings_skipping(&json, "", &replaced, &mut all);
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
    for name in [
        "source.toml",
        "legal-notices.json",
        "id-overrides.toml",
        "content-policy.toml",
    ] {
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
    let construct =
        lib.entry("monster-codex/rules/drow/example-drow/abrakarn-viper/abrakarn-viper");
    let construction = blocks_text(&construct.body);
    assert!(construction.contains("150,000 gp"));
    assert!(construction.contains("Craft Construct"));
    let typed_urls: std::collections::BTreeSet<String> = lib
        .entries
        .iter()
        .filter(|e| e.kind != cast_iron_schema::EntryKind::Rule)
        .map(|e| e.provenance[0].record.clone())
        .collect();
    assert_all_text_present(
        &dataset,
        "core_rulebook/spell/fireball.json",
        lib.entry("core-rulebook/spells/fireball"),
        &typed_urls,
    );
    assert_all_text_present(
        &dataset,
        "bestiary/creature/aasimar.json",
        lib.entry("bestiary/monsters/aasimar"),
        &typed_urls,
    );
    assert!(
        outcome
            .rejected
            .iter()
            .all(|r| r.reason.starts_with("Product Identity:")),
        "{:?}",
        outcome.rejected
    );
    // Every typed entry keeps every string that the upstream record holds.
    let mut failures = Vec::new();
    for entry in lib
        .entries
        .iter()
        .filter(|e| e.kind != cast_iron_schema::EntryKind::Rule)
    {
        let path = &entry.provenance[0].path;
        let json = retained_json(&dataset, path);
        let node = record_by_url(&json, &entry.provenance[0].record).unwrap();
        let mut replaced = typed_urls.clone();
        replaced.remove(&entry.provenance[0].record);
        let mut all = Vec::new();
        strings_skipping(node, "", &replaced, &mut all);
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
        let json = retained_json(&dataset, path);
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

/// Relations, spell lists, and book order on the full snapshot. The numbers here are the ones
/// that docs/research/relationship-data.md reports.
#[test]
fn relationships_and_book_order_hold_on_the_real_library() {
    use cast_iron_schema::{
        BookIndex, BookNode, CoverageReport, EntryKind, RelationIndex, RelationKind, SpellLists,
    };
    use std::collections::BTreeSet;

    let dataset = workspace().join(".cache/sources/psrd-data");
    if !dataset.join("core_rulebook/spell/fireball.json").exists() {
        eprintln!("skipping: {} is not present", dataset.display());
        return;
    }
    let scratch = scratch("real-relations");
    let config = scratch.join("config");
    std::fs::create_dir_all(&config).unwrap();
    for name in [
        "source.toml",
        "legal-notices.json",
        "id-overrides.toml",
        "content-policy.toml",
        "book-order.toml",
        "upstream-order.json",
    ] {
        std::fs::copy(
            workspace().join("sources/psrd-data").join(name),
            config.join(name),
        )
        .unwrap();
    }
    run(&options(
        &config,
        &dataset,
        &scratch.join("out"),
        &scratch.join("reports"),
    ));
    let lib = load(&scratch.join("out"));
    let read = |file: &str| std::fs::read(lib.dir.join(file)).unwrap();
    let coverage: CoverageReport = serde_json::from_slice(&read("coverage.json")).unwrap();
    let index: RelationIndex = serde_json::from_slice(&read("relations.json")).unwrap();
    let books: BookIndex = serde_json::from_slice(&read("books.json")).unwrap();
    let lists: SpellLists = serde_json::from_slice(&read("lists.json")).unwrap();
    // Grapple is a heading inside Special Attacks, not a standalone entry. A rules
    // lookup must surface that subsystem ahead of incidental mentions in other bodies.
    let search = cast_iron_search::SearchIndex::from_bytes(&read("search.bin")).unwrap();
    let hits = search.search(&cast_iron_search::Query {
        text: "grapple".into(),
        filters: cast_iron_search::Filters {
            kinds: vec!["rule".into()],
            ..Default::default()
        },
        limit: 3,
    });
    assert!(
        hits.iter()
            .any(|hit| hit.id == "core-rulebook/rules/combat/special-attacks")
    );
    // The edge index holds exactly the entries' relations.
    let relation_total: usize = lib.entries.iter().map(|e| e.relations.len()).sum();
    assert_eq!(index.edges.len(), relation_total);
    assert_eq!(index.counts.values().sum::<u32>() as usize, relation_total);
    assert_eq!(coverage.relations.edges, index.counts);
    let class_ids: BTreeSet<_> = lib
        .entries
        .iter()
        .filter(|e| matches!(e.kind, EntryKind::Class | EntryKind::PrestigeClass))
        .map(|e| e.id.clone())
        .collect();
    let race_ids: BTreeSet<_> = lib
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::Race)
        .map(|e| e.id.clone())
        .collect();

    // Every archetype points at a class that exists.
    let archetypes: Vec<_> = lib
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::Archetype)
        .collect();
    assert!(archetypes.len() > 200, "{}", archetypes.len());
    for a in &archetypes {
        let classes: Vec<_> = a
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Archetype)
            .collect();
        assert_eq!(classes.len(), 1, "{} needs exactly one class", a.id);
        assert!(class_ids.contains(&classes[0].target), "{}", a.id);
    }

    // Every racial trait belongs to a race, or is reported with a reason.
    let unplaced: BTreeSet<&str> = coverage
        .relations
        .unplaced_parts
        .iter()
        .map(|u| u.entry.as_str())
        .collect();
    let traits: Vec<_> = lib
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::RacialTrait)
        .collect();
    assert!(traits.len() > 300, "{}", traits.len());
    for t in &traits {
        let part = t.relations.iter().find(|r| r.kind == RelationKind::Part);
        match part {
            Some(r) => assert!(race_ids.contains(&r.target), "{}", t.id),
            None => assert!(
                unplaced.contains(t.id.as_str()),
                "{} has no parent and no reason",
                t.id
            ),
        }
    }
    assert!(
        coverage
            .relations
            .unplaced_parts
            .iter()
            .all(|u| !u.reason.is_empty())
    );

    // The prerequisite resolution is reported, and most feat-like requirements resolve.
    let p = &coverage.relations.prerequisites;
    assert!(p.feats_with_text > 900, "{}", p.feats_with_text);
    let resolved = p.by_reading["feat"] as f64;
    let candidates = resolved + p.by_reading["other"] as f64;
    assert!(
        resolved / candidates > 0.7,
        "only {resolved} of {candidates} name-like requirements resolve"
    );
    for feat in lib.entries.iter().filter(|e| e.kind == EntryKind::Feat) {
        if let TypedFields::Feat(f) = &feat.fields {
            let from_relations: BTreeSet<_> = feat
                .relations
                .iter()
                .filter(|r| r.kind == RelationKind::Prerequisite)
                .map(|r| &r.target)
                .collect();
            assert_eq!(
                from_relations,
                f.prerequisite_feats.iter().collect(),
                "{}",
                feat.id
            );
        }
    }

    // Spell lists agree with the spells' own levels and report the classes without an entry.
    let wizard = lists.classes.iter().find(|l| l.name == "Wizard").unwrap();
    let wizard_three: Vec<&str> = wizard
        .levels
        .iter()
        .find(|l| l.level == 3)
        .unwrap()
        .spells
        .iter()
        .map(|&p| lists.spells[p as usize].0.as_str())
        .collect();
    assert!(wizard_three.contains(&"core-rulebook/spells/fireball"));
    let unmapped: Vec<&str> = coverage
        .relations
        .spell_lists
        .unmapped
        .iter()
        .map(|u| u.name.as_str())
        .collect();
    for class in ["Arcanist", "Hunter", "Elementalist Wizard"] {
        assert!(unmapped.contains(&class), "{class} is not in {unmapped:?}");
    }
    assert!(!unmapped.contains(&"Wizard"));

    // Every entry is in exactly one book tree place or listed as unplaced.
    fn collect(n: &BookNode, seen: &mut BTreeSet<String>) {
        if let (Some(e), None) = (&n.entry, &n.anchor) {
            assert!(seen.insert(e.to_string()), "{e} appears twice");
        }
        for child in &n.children {
            collect(child, seen);
        }
    }
    let mut placed = BTreeSet::new();
    for tree in &books.books {
        for chapter in &tree.chapters {
            collect(chapter, &mut placed);
        }
    }
    let unplaced_book: BTreeSet<String> = coverage
        .relations
        .book_order
        .unplaced
        .iter()
        .map(|u| u.entry.clone())
        .collect();
    for entry in &lib.entries {
        let id = entry.id.to_string();
        assert!(
            placed.contains(&id) != unplaced_book.contains(&id),
            "{id} must be either in a book or reported"
        );
    }
    assert_eq!(books.books.len(), 16);
}
