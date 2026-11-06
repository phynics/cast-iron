//! The corpus build, end to end, on a tiny synthetic corpus.

use cast_iron_content_compiler::corpus_build::compile_corpus;
use cast_iron_corpus::{
    Config, Exclusions, entry_path, write_books, write_config, write_entry, write_exclusions,
    write_history, write_legal, write_publications,
};
use cast_iron_schema::{
    Block, BookIndex, ContentManifest, CoverageHistory, EntryId, EntryKind, Exclusion, Inline,
    LegalNotices, LicenseText, ParagraphStyle, RuleEntry, SourceInfo, TypedFields,
};
use std::fs;
use std::path::{Path, PathBuf};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ci-corpus-compile-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn rule(id: &str, text: &str, link: Option<&str>) -> RuleEntry {
    let mut content = vec![Inline::text(text)];
    let mut links = vec![];
    if let Some(target) = link {
        let target = EntryId::parse(target).unwrap();
        content.push(Inline::Link {
            target: target.clone(),
            anchor: None,
            c: vec![Inline::text("see")],
        });
        links.push(cast_iron_schema::EntryReference {
            target,
            anchor: None,
        });
    }
    RuleEntry {
        id: EntryId::parse(id).unwrap(),
        kind: EntryKind::Rule,
        title: id.rsplit('/').next().unwrap().to_string(),
        source_ids: vec!["core-rulebook".into()],
        section: vec![],
        parent: None,
        body: vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }],
        fields: TypedFields::None,
        tags: vec![],
        links,
        provenance: vec![],
        license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
        relations: vec![],
    }
}

fn write_corpus(dir: &Path) {
    let config = Config::default();
    for entry in [
        rule(
            "ref/rule/grapple",
            "Grapple rules.",
            Some("ref/rule/combat"),
        ),
        rule("ref/rule/combat", "Combat rules.", None),
    ] {
        let path = dir.join(entry_path(&entry.id));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, write_entry(&entry, &config)).unwrap();
    }
    let sources = [SourceInfo {
        id: "core-rulebook".into(),
        title: "Core Rulebook".into(),
        entry_count: 0,
        copyright: Some("Core Rulebook. (c) Paizo Inc.".into()),
        category: Some("RPG".into()),
        publication_date: None,
    }];
    let legal = LegalNotices {
        licenses: vec![LicenseText {
            id: "ogl-1.0a".into(),
            title: "Open Game License 1.0a".into(),
            text: "License text.".into(),
        }],
        product_identity: "Product Identity.".into(),
        community_use_notice: "Cast Iron uses trademarks under Paizo's Community Use Policy."
            .into(),
        section_15: vec![],
        notice_pending: vec![],
        datasets: vec![],
    };
    let exclusions = Exclusions {
        excluded: vec![Exclusion {
            path: "json/example.json".into(),
            records: 3,
            reason: "Product Identity".into(),
        }],
        rejected: vec![],
    };
    for (name, text) in [
        ("corpus.toml", write_config(&config)),
        ("publications.toml", write_publications(&sources)),
        ("books.toml", write_books(&BookIndex::default())),
        ("legal.toml", write_legal(&legal)),
        ("exclusions.toml", write_exclusions(&exclusions)),
        (
            "history.toml",
            write_history(&CoverageHistory {
                input_digest: "0".repeat(64),
                ..Default::default()
            }),
        ),
    ] {
        fs::write(dir.join(name), text).unwrap();
    }
}

/// Every file under `dir` with its bytes, in path order.
fn tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for item in fs::read_dir(dir).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let relative = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                out.push((relative, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[test]
fn a_corpus_compiles_to_a_verified_library_with_the_same_bytes_every_time() {
    let corpus = temp("corpus");
    write_corpus(&corpus);
    let mut versions = Vec::new();
    let mut trees = Vec::new();
    for run in 0..2 {
        let out = temp(&format!("out{run}"));
        let reports = temp(&format!("reports{run}"));
        let outcome = compile_corpus(&corpus, &out, &reports).unwrap_or_else(|f| {
            panic!("{}", f.lines.join("\n"));
        });
        assert_eq!(outcome.entry_count, 2);
        assert_eq!(outcome.links.resolved, 1);
        assert_eq!(outcome.links.broken, 0);
        let manifest: ContentManifest =
            serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.content_version, outcome.content_version);
        assert_eq!(manifest.entry_count, 2);
        for file in &manifest.files {
            assert!(out.join(&file.path).is_file(), "{}", file.path);
        }
        assert!(reports.join("coverage.md").is_file());
        assert!(reports.join("validation.json").is_file());
        let coverage = fs::read_to_string(reports.join("coverage.md")).unwrap();
        assert!(coverage.contains("json/example.json"), "{coverage}");
        versions.push(outcome.content_version);
        trees.push((tree(&out), tree(&reports)));
    }
    assert_eq!(versions[0], versions[1]);
    assert_eq!(trees[0], trees[1]);
}

#[test]
fn a_broken_link_in_the_corpus_stops_the_build_and_names_the_file() {
    let corpus = temp("broken");
    write_corpus(&corpus);
    let entry = rule("ref/rule/grapple", "Grapple.", Some("ref/rule/missing"));
    fs::write(
        corpus.join(entry_path(&entry.id)),
        write_entry(&entry, &Config::default()),
    )
    .unwrap();
    let failure = compile_corpus(&corpus, &temp("broken-out"), &temp("broken-reports"))
        .expect_err("a link to a missing entry must fail");
    let text = failure.lines.join("\n");
    assert!(text.contains("grapple"), "{text}");
}
