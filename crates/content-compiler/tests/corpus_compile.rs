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
        aon: None,
        source_ids: vec!["core-rulebook".into()],
        sources: vec![],
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
        aon: None,
        entry_count: 0,
        copyright: Some("Core Rulebook. (c) Paizo Inc.".into()),
        category: Some("RPG".into()),
        publication_date: None,
        series_kind: None,
        series: None,
        series_number: None,
        season: None,
        third_party: false,
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
        section_15: vec![cast_iron_schema::CopyrightNotice {
            title: "Core Rulebook".into(),
            notice: "Core Rulebook. (c) Paizo Inc.".into(),
        }],
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
        let outcome =
            compile_corpus(&corpus, &out, &reports, None, Some("v0.1.6")).unwrap_or_else(|f| {
                panic!("{}", f.lines.join("\n"));
            });
        assert_eq!(outcome.entry_count, 2);
        assert_eq!(outcome.links.resolved, 1);
        assert_eq!(outcome.links.broken, 0);
        let manifest: ContentManifest =
            serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.content_version, outcome.content_version);
        assert_eq!(manifest.entry_count, 2);
        assert_eq!(manifest.corpus_tag.as_deref(), Some("v0.1.6"));
        for file in &manifest.files {
            assert!(out.join(&file.path).is_file(), "{}", file.path);
        }
        assert!(reports.join("coverage.md").is_file());
        assert!(reports.join("validation.json").is_file());
        assert!(reports.join("reachability.json").is_file());
        let coverage = fs::read_to_string(reports.join("coverage.md")).unwrap();
        assert!(coverage.contains("json/example.json"), "{coverage}");
        let derived: Vec<String> = manifest
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect();
        for name in ["relations.json", "option-groups.json", "replaces.json"] {
            assert!(
                derived.iter().any(|path| path.ends_with(name)),
                "{name} is not in the manifest: {derived:?}"
            );
        }
        let version = out.join(&outcome.content_version);
        assert_eq!(
            fs::read_to_string(version.join("option-groups.json")).unwrap(),
            r#"{"groups":[]}"#
        );
        assert_eq!(
            fs::read_to_string(version.join("replaces.json")).unwrap(),
            r#"{"entries":[]}"#
        );
        versions.push(outcome.content_version);
        trees.push((tree(&out), tree(&reports)));
    }
    assert_eq!(versions[0], versions[1]);
    assert_eq!(trees[0], trees[1]);
}

#[test]
fn attached_entry_indexes_are_derived_from_the_corpus() {
    let corpus = temp("attached");
    write_corpus(&corpus);
    let class_option = RuleEntry {
        id: EntryId::parse("ref/arcanedisc/example").unwrap(),
        kind: EntryKind::ClassOption,
        title: "Example Discovery".into(),
        aon: None,
        source_ids: vec!["core-rulebook".into()],
        sources: vec![],
        section: vec![],
        parent: None,
        body: vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                Inline::Strong {
                    c: vec![Inline::text("Replaces:")],
                },
                Inline::text(" Arcane bond."),
            ],
        }],
        fields: TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
            group: "discoveries".into(),
            replaces: vec!["Arcane bond".into()],
            alters: vec![],
            ..Default::default()
        }),
        tags: vec![],
        links: vec![],
        provenance: vec![],
        license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
        relations: vec![],
    };
    let archetype = RuleEntry {
        id: EntryId::parse("ref/arc-fighter/example").unwrap(),
        kind: EntryKind::Archetype,
        title: "Example Archetype".into(),
        aon: None,
        source_ids: vec!["core-rulebook".into()],
        sources: vec![],
        section: vec![],
        parent: None,
        body: vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                Inline::Strong {
                    c: vec![Inline::text("Replaces:")],
                },
                Inline::text(" Trap sense."),
            ],
        }],
        fields: TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
            replaces: vec!["Trap sense".into()],
            alters: vec![],
            ..Default::default()
        }),
        tags: vec![],
        links: vec![],
        provenance: vec![],
        license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
        relations: vec![cast_iron_schema::Relation {
            kind: cast_iron_schema::RelationKind::Archetype,
            target: EntryId::parse("ref/class/fighter").unwrap(),
            label: None,
        }],
    };
    // The archetype relation needs its class entry.
    let fighter = RuleEntry {
        id: EntryId::parse("ref/class/fighter").unwrap(),
        kind: EntryKind::Class,
        title: "Fighter".into(),
        aon: None,
        source_ids: vec!["core-rulebook".into()],
        sources: vec![],
        section: vec![],
        parent: None,
        body: vec![],
        fields: TypedFields::Class(Default::default()),
        tags: vec![],
        links: vec![],
        provenance: vec![],
        license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
        relations: vec![],
    };
    for entry in [class_option, archetype, fighter] {
        let path = corpus.join(entry_path(&entry.id));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, write_entry(&entry, &Config::default())).unwrap();
    }
    fs::write(
        corpus.join("option-groups.toml"),
        "[groups.discoveries]\nlabel = \"Discoveries\"\nsingular = \"Discovery\"\nclasses = [\"ref/class/fighter\"]\n",
    )
    .unwrap();

    let out = temp("attached-out");
    let outcome = compile_corpus(&corpus, &out, &temp("attached-reports"), None, None)
        .unwrap_or_else(|failure| panic!("{}", failure.lines.join("\n")));
    let version = out.join(&outcome.content_version);
    let groups: serde_json::Value =
        serde_json::from_slice(&fs::read(version.join("option-groups.json")).unwrap()).unwrap();
    assert_eq!(groups["groups"][0]["slug"], "discoveries");
    assert_eq!(groups["groups"][0]["entries"][0], "ref/arcanedisc/example");
    let replaces: serde_json::Value =
        serde_json::from_slice(&fs::read(version.join("replaces.json")).unwrap()).unwrap();
    assert_eq!(
        replaces["entries"],
        serde_json::json!([
            {"id": "ref/arc-fighter/example", "replaces": ["Trap sense"]},
            {"id": "ref/arcanedisc/example", "replaces": ["Arcane bond"]},
        ])
    );
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
    let failure = compile_corpus(
        &corpus,
        &temp("broken-out"),
        &temp("broken-reports"),
        None,
        None,
    )
    .expect_err("a link to a missing entry must fail");
    let text = failure.lines.join("\n");
    assert!(text.contains("grapple"), "{text}");
}

#[test]
fn a_published_id_that_stops_resolving_fails_the_build_before_anything_is_written() {
    let corpus = temp("published");
    write_corpus(&corpus);
    let ids = temp("published-ids").join("ids.txt");
    fs::write(&ids, "# header\nref/rule/combat\nref/rule/grapple\n").unwrap();
    compile_corpus(
        &corpus,
        &temp("published-ok"),
        &temp("published-ok-reports"),
        Some(&ids),
        None,
    )
    .unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));

    fs::write(&ids, "ref/rule/combat\nref/rule/dropped\n").unwrap();
    let out = temp("published-bad");
    let failure = compile_corpus(
        &corpus,
        &out,
        &temp("published-bad-reports"),
        Some(&ids),
        None,
    )
    .expect_err("a dropped published id must fail");
    let text = failure.lines.join("\n");
    assert!(text.contains("ref/rule/dropped"), "{text}");
    assert!(!out.join("manifest.json").exists());
}
