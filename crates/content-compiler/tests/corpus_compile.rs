//! The corpus build, end to end, on a tiny synthetic corpus.

use cast_iron_content_compiler::corpus_build::{compile_corpus, compile_corpus_with_slots};
use cast_iron_content_compiler::pipeline::Failure;
use cast_iron_corpus::{
    Config, Exclusions, SPLIT_BOOKS_FORMAT, entry_path, write_books, write_config,
    write_curated_book, write_entry, write_exclusions, write_history, write_legal,
    write_publications,
};
use cast_iron_schema::{
    Block, BookIndex, BookNode, BookTree, ContentManifest, CoverageHistory, EntryId, EntryKind,
    Exclusion, Expansion, Inline, LegalNotices, LicenseText, ParagraphStyle, RuleEntry, SourceInfo,
    TypedFields,
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
        expanded_in: vec![],
        stat_sets: vec![],
        section: vec![],
        parent: None,
        body: vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content,
        }],
        fields: TypedFields::None,
        tags: vec![],
        race_group: None,
        links,
        provenance: vec![],
        license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
        relations: vec![],
    }
}

fn node(title: &str, entry: Option<&str>, children: Vec<BookNode>) -> BookNode {
    BookNode {
        title: title.into(),
        entry: entry.map(|id| EntryId::parse(id).unwrap()),
        anchor: None,
        review: None,
        collapse: false,
        lists_children: false,
        children,
    }
}

/// A book that places both rules in a printed chapter, so each is reachable without the
/// catch-all chapter.
fn printed_chapter() -> BookIndex {
    BookIndex {
        books: vec![BookTree {
            source: "core-rulebook".into(),
            title: "Core Rulebook".into(),
            basis: String::new(),
            review: vec![],
            chapters: vec![node(
                "Combat",
                None,
                vec![
                    node("Grapple", Some("ref/rule/grapple"), vec![]),
                    node("Combat", Some("ref/rule/combat"), vec![]),
                ],
            )],
        }],
    }
}

fn write_corpus(dir: &Path) {
    write_corpus_with(dir, None);
}

/// The synthetic corpus declared as corpus format `format`.
fn write_corpus_in(dir: &Path, format: u32) {
    write_corpus_full(dir, None, format);
}

/// `expansion` adds a second book, `monster-handbook`, that no entry prints. `Some(true)` makes
/// the grapple rule name it in `expanded_in`; `Some(false)` leaves the rule as it was.
///
/// The format is the last one with a single `books.toml`, which the tests that swap the books
/// file rely on. The split layout of format 3 has its own tests below.
fn write_corpus_with(dir: &Path, expansion: Option<bool>) {
    write_corpus_full(dir, expansion, SPLIT_BOOKS_FORMAT - 1);
}

fn write_corpus_full(dir: &Path, expansion: Option<bool>, format: u32) {
    let config = Config {
        format,
        ..Config::default()
    };
    let mut grapple = rule(
        "ref/rule/grapple",
        "Grapple rules.",
        Some("ref/rule/combat"),
    );
    if expansion == Some(true) {
        grapple.expanded_in = vec![Expansion {
            publication: "monster-handbook".into(),
            page: Some("18".into()),
            via: None,
        }];
    }
    for entry in [grapple, rule("ref/rule/combat", "Combat rules.", None)] {
        let path = dir.join(entry_path(&entry.id));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, write_entry(&entry, &config)).unwrap();
    }
    let mut sources = vec![SourceInfo {
        share_slot: None,
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
    if expansion.is_some() {
        let mut handbook = sources[0].clone();
        handbook.id = "monster-handbook".into();
        handbook.title = "Monster Handbook".into();
        handbook.copyright = Some("Monster Handbook. (c) Paizo Inc.".into());
        sources.push(handbook);
    }
    let legal = LegalNotices {
        licenses: vec![LicenseText {
            id: "ogl-1.0a".into(),
            title: "Open Game License 1.0a".into(),
            text: "License text.".into(),
        }],
        product_identity: "Product Identity.".into(),
        community_use_notice: "Cast Iron uses trademarks under Paizo's Community Use Policy."
            .into(),
        section_15: {
            let mut notices = vec![cast_iron_schema::CopyrightNotice {
                title: "Core Rulebook".into(),
                notice: "Core Rulebook. (c) Paizo Inc.".into(),
            }];
            if expansion.is_some() {
                notices.push(cast_iron_schema::CopyrightNotice {
                    title: "Monster Handbook".into(),
                    notice: "Monster Handbook. (c) Paizo Inc.".into(),
                });
            }
            notices
        },
        notice_pending: vec![],
        publication_licenses: vec![],
        datasets: vec![],
    };
    let exclusions = Exclusions {
        excluded: vec![Exclusion {
            path: "json/example.json".into(),
            records: 3,
            reason: "Product Identity".into(),
        }],
        rejected: vec![],
        resolved: vec![],
    };
    if format >= SPLIT_BOOKS_FORMAT {
        fs::create_dir_all(dir.join("books")).unwrap();
        for tree in printed_chapter().books {
            fs::write(
                dir.join(format!("books/{}.toml", tree.source)),
                write_curated_book(&tree),
            )
            .unwrap();
        }
    } else {
        fs::write(dir.join("books.toml"), write_books(&printed_chapter())).unwrap();
    }
    for (name, text) in [
        ("corpus.toml", write_config(&config)),
        ("publications.toml", write_publications(&sources)),
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
        for name in ["relations.json", "option-groups.json", "trades.json"] {
            assert!(
                derived.iter().any(|path| path.ends_with(name)),
                "{name} is not in the manifest: {derived:?}"
            );
        }
        assert!(
            !derived.iter().any(|path| path.ends_with("replaces.json")),
            "a schema-6 library has no replaces.json: {derived:?}"
        );
        let version = out.join(&outcome.content_version);
        assert_eq!(
            fs::read_to_string(version.join("option-groups.json")).unwrap(),
            r#"{"groups":[]}"#
        );
        assert_eq!(
            fs::read_to_string(version.join("trades.json")).unwrap(),
            r#"{"entries":[]}"#
        );
        versions.push(outcome.content_version);
        trees.push((tree(&out), tree(&reports)));
    }
    assert_eq!(versions[0], versions[1]);
    assert_eq!(trees[0], trees[1]);
}

/// Compiles `corpus` and returns `coverage.md` and `coverage.json`.
fn coverage_of(corpus: &Path, name: &str) -> Result<(String, serde_json::Value), Failure> {
    let reports = temp(&format!("{name}-reports"));
    let out = temp(&format!("{name}-out"));
    let outcome = compile_corpus(corpus, &out, &reports, None, Some("v0.1.6"))?;
    let json = fs::read(out.join(&outcome.content_version).join("coverage.json")).unwrap();
    Ok((
        fs::read_to_string(reports.join("coverage.md")).unwrap(),
        serde_json::from_slice(&json).unwrap(),
    ))
}

fn paths(json: &serde_json::Value, key: &str) -> Vec<String> {
    json[key]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| row["path"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_resolved_exclusion_moves_to_the_resolved_list_and_an_orphan_stops_the_build() {
    let corpus = temp("resolved-corpus");
    write_corpus(&corpus);
    let file = corpus.join("exclusions.toml");
    let before = fs::read_to_string(&file).unwrap();
    assert!(before.contains("json/example.json"), "{before}");
    let (listed, json) =
        coverage_of(&corpus, "excluded").unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    assert!(listed.contains("json/example.json"), "{listed}");
    // With no `resolved` row the report has no resolved section at all.
    assert!(json.get("resolved").is_none(), "{json}");
    assert!(!listed.contains("Resolved records"), "{listed}");
    assert_eq!(paths(&json, "excluded"), ["json/example.json"]);

    let row = "\n[[resolved]]\npath = \"json/example.json\"\nentries = [\"ref/rule/combat\"]\nreason = \"ships now. Refs #36.\"\n";
    fs::write(&file, format!("{before}{row}")).unwrap();
    let (after, json) =
        coverage_of(&corpus, "resolved").unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    assert!(paths(&json, "excluded").is_empty(), "{json}");
    assert_eq!(
        json["resolved"],
        serde_json::json!([{
            "path": "json/example.json",
            "was": "excluded",
            "entries": ["ref/rule/combat"],
            "reason": "ships now. Refs #36."
        }])
    );
    assert!(after.contains("## Resolved records (1)"), "{after}");
    assert!(
        after.contains(
            "- `json/example.json` (was excluded): ships now. Refs #36. Now: ref/rule/combat"
        ),
        "{after}"
    );

    // A row for a path no excluded or rejected row has is refused before anything is written.
    fs::write(
        &file,
        format!(
            "{before}{}",
            row.replace("json/example.json", "json/other.json")
        ),
    )
    .unwrap();
    let failure = coverage_of(&corpus, "orphan").expect_err("an orphan row must fail");
    assert!(
        failure.lines.join("\n").contains("supersedes nothing"),
        "{}",
        failure.lines.join("\n")
    );
}

#[test]
fn a_resolved_row_moves_a_rejected_record_to_the_resolved_list() {
    let corpus = temp("resolved-rejected-corpus");
    write_corpus(&corpus);
    let file = corpus.join("exclusions.toml");
    // The writer emits an empty `rejected = []`, which a `[[rejected]]` table cannot follow.
    let before = fs::read_to_string(&file)
        .unwrap()
        .replace("rejected = []\n", "");
    let rejected = "\n[[rejected]]\npath = \"json/domains.json#sun\"\nreason = \"converter could not read it\"\n\n[[rejected]]\npath = \"json/domains.json#moon\"\nreason = \"converter could not read it either\"\n";
    fs::write(&file, format!("{before}{rejected}")).unwrap();
    let (listed, json) =
        coverage_of(&corpus, "was-rejected").unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    assert!(listed.contains("## Rejected records (2)"), "{listed}");
    assert_eq!(
        paths(&json, "rejected"),
        ["json/domains.json#sun", "json/domains.json#moon"]
    );
    assert!(json.get("resolved").is_none(), "{json}");

    let row = "\n[[resolved]]\npath = \"json/domains.json#sun\"\nentries = [\"ref/rule/combat\"]\nreason = \"ships now. Refs #103.\"\n";
    fs::write(&file, format!("{before}{rejected}{row}")).unwrap();
    let (after, json) =
        coverage_of(&corpus, "now-resolved").unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    // The path is resolved, not rejected, the rejected count falls, and the other row stays.
    assert_eq!(
        json["resolved"],
        serde_json::json!([{
            "path": "json/domains.json#sun",
            "was": "rejected",
            "entries": ["ref/rule/combat"],
            "reason": "ships now. Refs #103."
        }])
    );
    assert_eq!(paths(&json, "rejected"), ["json/domains.json#moon"]);
    assert!(after.contains("## Rejected records (1)"), "{after}");
    assert!(
        after.contains("- `json/domains.json#moon`: converter could not read it either"),
        "{after}"
    );
    assert!(after.contains("## Resolved records (1)"), "{after}");
    assert!(
        after.contains("- `json/domains.json#sun` (was rejected): ships now. Refs #103."),
        "{after}"
    );
}

#[test]
fn one_resolved_row_does_not_hide_another_row_with_its_path() {
    let corpus = temp("resolved-shared-corpus");
    write_corpus(&corpus);
    let file = corpus.join("exclusions.toml");
    // An excluded and a rejected row share one path, like the Death domain in the pinned corpus.
    let before = fs::read_to_string(&file)
        .unwrap()
        .replace("rejected = []\n", "");
    let shared = "\n[[excluded]]\npath = \"json/domains.json#death\"\nrecords = 1\nreason = \"blog adjustment, not shipped\"\n\n[[rejected]]\npath = \"json/domains.json#death\"\nreason = \"converter native exclusion (ref/domain/death): bad directive\"\n";
    fs::write(&file, format!("{before}{shared}")).unwrap();
    let (_, json) =
        coverage_of(&corpus, "shared").unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    assert_eq!(paths(&json, "rejected"), ["json/domains.json#death"]);
    assert!(paths(&json, "excluded").contains(&"json/domains.json#death".to_string()));

    // The path alone is ambiguous: the build stops and says what to add.
    let plain = "\n[[resolved]]\npath = \"json/domains.json#death\"\nentries = [\"ref/rule/combat\"]\nreason = \"ships now. Refs #103.\"\n";
    fs::write(&file, format!("{before}{shared}{plain}")).unwrap();
    let failure = coverage_of(&corpus, "ambiguous").expect_err("an ambiguous row must fail");
    let text = failure.lines.join("\n");
    assert!(text.contains("is ambiguous"), "{text}");
    assert!(text.contains("supersedes"), "{text}");

    // Naming the rejected row resolves it and leaves the excluded row listed.
    let named = plain.replace(
        "entries",
        "supersedes = \"converter native exclusion (ref/domain/death): bad directive\"\nentries",
    );
    fs::write(&file, format!("{before}{shared}{named}")).unwrap();
    let (_, json) =
        coverage_of(&corpus, "named").unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    assert!(paths(&json, "rejected").is_empty(), "{json}");
    assert!(paths(&json, "excluded").contains(&"json/domains.json#death".to_string()));
    assert_eq!(json["resolved"][0]["was"], "rejected");
    assert_eq!(json["resolved"][0]["path"], "json/domains.json#death");
}

fn typed_entry(
    id: &str,
    kind: EntryKind,
    title: &str,
    body: Vec<Block>,
    fields: TypedFields,
    relations: Vec<cast_iron_schema::Relation>,
) -> RuleEntry {
    RuleEntry {
        id: EntryId::parse(id).unwrap(),
        kind,
        title: title.into(),
        aon: None,
        source_ids: vec!["core-rulebook".into()],
        sources: vec![],
        expanded_in: vec![],
        stat_sets: vec![],
        section: vec![],
        parent: None,
        body,
        fields,
        tags: vec![],
        race_group: None,
        links: vec![],
        provenance: vec![],
        license_refs: vec!["ogl-1.0a".into(), "legal#community-use".into()],
        relations,
    }
}

/// A class whose body holds one level-2 heading per `(anchor, text)`.
fn class_entry(id: &str, headings: &[(&str, &str)]) -> RuleEntry {
    let body = headings
        .iter()
        .map(|(anchor, text)| Block::Heading {
            level: 2,
            anchor: Some((*anchor).into()),
            content: vec![Inline::text(*text)],
        })
        .collect();
    typed_entry(
        id,
        EntryKind::Class,
        id.rsplit('/').next().unwrap(),
        body,
        TypedFields::Class(Default::default()),
        vec![],
    )
}

/// An archetype of `classes` with a printed `Replaces:` line and the given trades.
fn archetype_entry(
    id: &str,
    classes: &[&str],
    printed: &str,
    trades: Vec<cast_iron_schema::Trade>,
    legacy: cast_iron_schema::LegacyFeatureLists,
) -> RuleEntry {
    typed_entry(
        id,
        EntryKind::Archetype,
        "Example Archetype",
        vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                Inline::Strong {
                    c: vec![Inline::text("Replaces:")],
                },
                Inline::text(format!(" {printed}.")),
            ],
        }],
        TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
            features: Default::default(),
            trades,
            legacy_features: legacy,
        }),
        classes
            .iter()
            .map(|class| cast_iron_schema::Relation {
                kind: cast_iron_schema::RelationKind::Archetype,
                target: EntryId::parse(*class).unwrap(),
                label: None,
            })
            .collect(),
    )
}

fn replaces(printed: &str, anchor: Option<&str>, class: Option<&str>) -> cast_iron_schema::Trade {
    cast_iron_schema::Trade {
        by: None,
        op: cast_iron_schema::TradeOp::Replaces,
        printed: printed.into(),
        feature: anchor.map(|anchor| cast_iron_schema::FeatureRef {
            anchor: anchor.into(),
            class: class.map(|class| EntryId::parse(class).unwrap()),
            ..Default::default()
        }),
    }
}

fn write_entries(corpus: &Path, format: u32, entries: &[RuleEntry]) {
    let config = Config {
        format,
        ..Config::default()
    };
    for entry in entries {
        let path = corpus.join(entry_path(&entry.id));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, write_entry(entry, &config)).unwrap();
    }
}

/// Compiles `corpus` and returns the parsed `trades.json` and every entry shard's text.
fn compile_trades(corpus: &Path, name: &str) -> Result<(serde_json::Value, String), String> {
    let out = temp(&format!("{name}-out"));
    let outcome = compile_corpus(corpus, &out, &temp(&format!("{name}-reports")), None, None)
        .map_err(|failure| failure.lines.join("\n"))?;
    let version = out.join(&outcome.content_version);
    let trades = serde_json::from_slice(&fs::read(version.join("trades.json")).unwrap()).unwrap();
    let mut shards = String::new();
    for item in fs::read_dir(version.join("entries")).unwrap() {
        shards.push_str(&fs::read_to_string(item.unwrap().path()).unwrap());
    }
    Ok((trades, shards))
}

#[test]
fn attached_entry_indexes_are_derived_from_the_corpus() {
    let corpus = temp("attached");
    write_corpus(&corpus);
    let class_option = typed_entry(
        "ref/arcanedisc/example",
        EntryKind::ClassOption,
        "Example Discovery",
        vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![
                Inline::Strong {
                    c: vec![Inline::text("Replaces:")],
                },
                Inline::text(" Arcane bond."),
            ],
        }],
        TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
            group: "discoveries".into(),
            trades: vec![replaces("Arcane bond", None, None)],
            ..Default::default()
        }),
        vec![],
    );
    let archetype = archetype_entry(
        "ref/arc-fighter/example",
        &["ref/class/fighter"],
        "Trap sense",
        // The class is written although only one class holds the anchor; the compiler drops it.
        vec![replaces(
            "Trap sense",
            Some("trap-sense"),
            Some("ref/class/fighter"),
        )],
        Default::default(),
    );
    let fighter = class_entry("ref/class/fighter", &[("trap-sense", "Trap Sense (Ex)")]);
    write_entries(&corpus, 2, &[class_option, archetype, fighter]);
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
    let trades: serde_json::Value =
        serde_json::from_slice(&fs::read(version.join("trades.json")).unwrap()).unwrap();
    assert_eq!(
        trades["entries"],
        serde_json::json!([
            {"id": "ref/arc-fighter/example", "trades": [
                {"op": "replaces", "printed": "Trap sense", "feature": {"anchor": "trap-sense"}}
            ]},
            {"id": "ref/arcanedisc/example", "trades": [
                {"op": "replaces", "printed": "Arcane bond"}
            ]},
        ])
    );
    // The shards hold no trades; the old file is gone.
    assert!(!version.join("replaces.json").exists());
    for item in fs::read_dir(version.join("entries")).unwrap() {
        let shard = fs::read_to_string(item.unwrap().path()).unwrap();
        assert!(!shard.contains("\"trades\""), "{shard}");
        assert!(!shard.contains("\"printed\""), "{shard}");
    }
}

#[test]
fn feature_facts_reach_the_library_and_the_trade_names_its_feature() {
    let corpus = temp("features");
    write_corpus_in(&corpus, 4);
    let fighter = class_entry("ref/class/fighter", &[("bravery", "Bravery (Ex)")]);
    let archetype = typed_entry(
        "ref/arc-fighter/two-handed",
        EntryKind::Archetype,
        "Two-Handed Fighter",
        vec![
            Block::Heading {
                level: 2,
                anchor: Some("shattering-strike".into()),
                content: vec![Inline::text("Shattering Strike (Ex)")],
            },
            Block::Paragraph {
                style: ParagraphStyle::Normal,
                content: vec![Inline::text(
                    "At 2nd level, a two-handed fighter gains a bonus. This ability replaces bravery.",
                )],
            },
        ],
        TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
            features: vec![cast_iron_schema::FeatureFact {
                anchor: "shattering-strike".into(),
                level: 2,
            }],
            trades: vec![cast_iron_schema::Trade {
                by: Some("shattering-strike".into()),
                ..replaces("Bravery", Some("bravery"), None)
            }],
            ..Default::default()
        }),
        vec![cast_iron_schema::Relation {
            kind: cast_iron_schema::RelationKind::Archetype,
            target: EntryId::parse("ref/class/fighter").unwrap(),
            label: None,
        }],
    );
    write_entries(&corpus, 4, &[fighter, archetype]);
    let (trades, shards) = compile_trades(&corpus, "features").unwrap();
    assert_eq!(
        trades["entries"],
        serde_json::json!([
            {"id": "ref/arc-fighter/two-handed", "trades": [
                {"op": "replaces", "printed": "Bravery", "feature": {"anchor": "bravery"}, "by": "shattering-strike"}
            ]},
        ])
    );
    // The entry shard keeps the facts; the trades stay out of it.
    assert!(
        shards.contains(r#""features":[{"anchor":"shattering-strike","level":2}]"#),
        "{shards}"
    );
    assert!(!shards.contains("\"trades\""), "{shards}");
}

#[test]
fn a_class_is_filled_only_where_the_anchor_is_ambiguous_and_a_missing_one_fails() {
    let bard = class_entry(
        "ref/class/bard",
        &[("bardic-performance", "Bardic Performance")],
    );
    let skald = class_entry(
        "ref/class/skald",
        &[("bardic-performance", "Bardic Performance")],
    );
    let classes = ["ref/class/bard", "ref/class/skald"];

    let corpus = temp("ambiguous");
    write_corpus(&corpus);
    write_entries(
        &corpus,
        2,
        &[
            bard.clone(),
            skald.clone(),
            archetype_entry(
                "ref/arc-bard/example",
                &classes,
                "Bardic Performance",
                vec![replaces(
                    "Bardic Performance",
                    Some("bardic-performance"),
                    None,
                )],
                Default::default(),
            ),
        ],
    );
    let error = compile_trades(&corpus, "ambiguous").unwrap_err();
    assert!(
        error.contains("`ref/class/bard` and `ref/class/skald` hold; add `class` to say which"),
        "{error}"
    );

    let corpus = temp("settled");
    write_corpus(&corpus);
    write_entries(
        &corpus,
        2,
        &[
            bard,
            skald,
            archetype_entry(
                "ref/arc-bard/example",
                &classes,
                "Bardic Performance",
                vec![replaces(
                    "Bardic Performance",
                    Some("bardic-performance"),
                    Some("ref/class/skald"),
                )],
                Default::default(),
            ),
        ],
    );
    let (trades, _) = compile_trades(&corpus, "settled").unwrap();
    assert_eq!(
        trades["entries"][0]["trades"][0]["feature"],
        serde_json::json!({"anchor": "bardic-performance", "class": "ref/class/skald"})
    );
}

#[test]
fn a_format_1_corpus_keeps_every_printed_name_and_resolves_only_what_lines_up() {
    let bard = class_entry(
        "ref/class/bard",
        &[
            ("bardic-performance", "Bardic Performance"),
            ("bravery", "Bravery"),
        ],
    );
    let skald = class_entry(
        "ref/class/skald",
        &[("bardic-performance", "Bardic Performance")],
    );
    let corpus = temp("format-1");
    write_corpus_in(&corpus, 1);
    write_entries(
        &corpus,
        1,
        &[
            bard,
            skald,
            // Lines up, one class holds each anchor: both resolve.
            archetype_entry(
                "ref/arc-bard/aligned",
                &["ref/class/bard"],
                "Bravery",
                vec![replaces("Bravery", Some("bravery"), None)],
                Default::default(),
            ),
            // Lines up, but two classes hold the anchor and format 1 cannot name one: unresolved.
            archetype_entry(
                "ref/arc-bard/shared",
                &["ref/class/bard", "ref/class/skald"],
                "Bardic Performance",
                vec![replaces(
                    "Bardic Performance",
                    Some("bardic-performance"),
                    None,
                )],
                Default::default(),
            ),
            // A composite name is partly another feature, so it is not resolved to the anchor
            // that starts it; the name that is exactly an anchor resolves.
            archetype_entry(
                "ref/arc-bard/composite",
                &["ref/class/bard"],
                "Bardic Performance and a feat, Bravery",
                vec![
                    replaces("Bardic Performance and a feat", None, None),
                    replaces("Bravery", None, None),
                ],
                cast_iron_schema::LegacyFeatureLists {
                    replaces: vec!["bardic-performance".into(), "bravery".into()],
                    alters: vec![],
                },
            ),
        ],
    );
    let (trades, shards) = compile_trades(&corpus, "format-1").unwrap();
    assert_eq!(
        trades["entries"],
        serde_json::json!([
            {"id": "ref/arc-bard/aligned", "trades": [
                {"op": "replaces", "printed": "Bravery", "feature": {"anchor": "bravery"}}
            ]},
            {"id": "ref/arc-bard/composite", "trades": [
                {"op": "replaces", "printed": "Bardic Performance and a feat"},
                {"op": "replaces", "printed": "Bravery", "feature": {"anchor": "bravery"}}
            ]},
            {"id": "ref/arc-bard/shared", "trades": [
                {"op": "replaces", "printed": "Bardic Performance"}
            ]},
        ])
    );
    assert!(!shards.contains("\"trades\""), "{shards}");
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

#[test]
fn a_book_gets_its_slot_from_the_published_books_file_and_a_new_book_fails_the_build() {
    let corpus = temp("slots");
    write_corpus(&corpus);
    let slots = temp("slots-file").join("books.txt");
    // A book that left the library keeps its line; the slot is the line among the id lines.
    fs::write(&slots, "# header\nremoved-book\ncore-rulebook\n").unwrap();
    let out = temp("slots-out");
    compile_corpus_with_slots(
        &corpus,
        &out,
        &temp("slots-reports"),
        None,
        Some(&slots),
        None,
        None,
    )
    .unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    let manifest: ContentManifest =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    let catalog: cast_iron_schema::Catalog = serde_json::from_slice(
        &fs::read(out.join(&manifest.content_version).join("catalog.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(catalog.sources[0].id, "core-rulebook");
    assert_eq!(catalog.sources[0].share_slot, Some(1));

    // Without the file the catalog carries no slot, as before.
    let plain = temp("slots-plain");
    compile_corpus(&corpus, &plain, &temp("slots-plain-reports"), None, None).unwrap();
    let manifest: ContentManifest =
        serde_json::from_slice(&fs::read(plain.join("manifest.json")).unwrap()).unwrap();
    let catalog: cast_iron_schema::Catalog = serde_json::from_slice(
        &fs::read(plain.join(&manifest.content_version).join("catalog.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(catalog.sources[0].share_slot, None);

    // A book with no line stops the build, names the book and the command, and writes nothing.
    fs::write(&slots, "removed-book\n").unwrap();
    let bad = temp("slots-bad");
    let failure = compile_corpus_with_slots(
        &corpus,
        &bad,
        &temp("slots-bad-reports"),
        None,
        Some(&slots),
        None,
        None,
    )
    .expect_err("a book without a slot must fail");
    let text = failure.lines.join("\n");
    assert!(text.contains("core-rulebook"), "{text}");
    assert!(text.contains("cargo xtask published-books"), "{text}");
    assert!(!bad.join("manifest.json").exists());
}

#[test]
fn a_failed_gate_leaves_the_previous_library_unchanged() {
    let corpus = temp("gate");
    write_corpus(&corpus);
    let out = temp("gate-out");
    let reports = temp("gate-reports");
    compile_corpus(&corpus, &out, &reports, None, None)
        .unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    let before = tree(&out);
    assert!(!before.is_empty());

    // Take the rules out of their printed chapter: no route reaches them without the
    // catch-all chapter, which the reachability gate refuses. The gate runs after the library
    // is derived, so the compiler holds every file before it fails.
    fs::write(
        corpus.join("books.toml"),
        write_books(&BookIndex::default()),
    )
    .unwrap();
    let failure = compile_corpus(&corpus, &out, &reports, None, None)
        .expect_err("an unreachable entry must fail the build");
    let text = failure.lines.join("\n");
    assert!(text.contains("unreachable"), "{text}");
    assert_eq!(tree(&out), before, "the failed compile changed the library");
    assert!(
        !out.with_file_name(format!(
            "{}.staging",
            out.file_name().unwrap().to_string_lossy()
        ))
        .exists(),
        "the failed compile left a staging directory"
    );
    // The reports of the failed run stay, for the author to read.
    let report = fs::read_to_string(reports.join("reachability.json")).unwrap();
    assert!(report.contains("\"unreachable\": 2"), "{report}");

    // A first compile that fails writes no manifest at all.
    let fresh = temp("gate-fresh");
    let fresh_out = fresh.join("content");
    compile_corpus(&corpus, &fresh_out, &reports, None, None).expect_err("must fail");
    assert!(!fresh_out.exists());
}

#[test]
fn a_library_is_moved_into_place_whole() {
    let corpus = temp("staged");
    write_corpus(&corpus);
    let out = temp("staged-out");
    let outcome = compile_corpus(&corpus, &out, &temp("staged-reports"), None, None).unwrap();
    // A second compile replaces the first one; nothing of the old version stays behind.
    fs::write(out.join("stale.txt"), "x").unwrap();
    let again = compile_corpus(&corpus, &out, &temp("staged-reports2"), None, None).unwrap();
    assert_eq!(again.content_version, outcome.content_version);
    assert!(!out.join("stale.txt").exists());
    let manifest: ContentManifest =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    for file in &manifest.files {
        assert!(out.join(&file.path).is_file(), "{}", file.path);
    }
}

/// The sibling of `out` that the compiler uses, named by its suffix.
fn beside(out: &Path, suffix: &str) -> PathBuf {
    out.with_file_name(format!(
        "{}.{suffix}",
        out.file_name().unwrap().to_string_lossy()
    ))
}

#[test]
fn a_compile_recovers_from_an_interrupted_handoff() {
    let corpus = temp("recover");
    write_corpus(&corpus);
    let base = temp("recover-base");
    let reports = temp("recover-reports");

    // Step 2 done, step 3 not: the old library sits in the backup and `out` is missing.
    let out = base.join("after-backup");
    let first = compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    fs::rename(&out, beside(&out, "backup")).unwrap();
    let again = compile_corpus(&corpus, &out, &reports, None, None)
        .unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
    assert_eq!(again.content_version, first.content_version);
    assert!(out.join("manifest.json").is_file());
    assert!(!beside(&out, "backup").exists() && !beside(&out, "staging").exists());

    // The same state, and the gate fails: the compile still restores the previous library.
    let out = base.join("gate-after-backup");
    compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    let before = tree(&out);
    fs::rename(&out, beside(&out, "backup")).unwrap();
    fs::write(
        corpus.join("books.toml"),
        write_books(&BookIndex::default()),
    )
    .unwrap();
    compile_corpus(&corpus, &out, &reports, None, None).expect_err("the gate must fail");
    assert_eq!(tree(&out), before, "the previous library was not restored");
    assert!(!beside(&out, "backup").exists());
    fs::write(corpus.join("books.toml"), write_books(&printed_chapter())).unwrap();

    // Step 3 done, step 4 not: a new library and a leftover backup.
    let out = base.join("after-activate");
    compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    fs::create_dir_all(beside(&out, "backup").join("vold")).unwrap();
    fs::write(beside(&out, "backup").join("vold/a.json"), "{}").unwrap();
    compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    assert!(out.join("manifest.json").is_file());
    assert!(!beside(&out, "backup").exists());

    // A staging directory that a crash left behind.
    let out = base.join("leftover-staging");
    compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    fs::create_dir_all(beside(&out, "staging").join("vold")).unwrap();
    compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    assert!(!beside(&out, "staging").exists());
}

#[test]
fn a_second_compile_cannot_clean_a_live_writers_staging_directory() {
    use cast_iron_content_compiler::publish::{OutLock, lock_file, staging_dir};
    let corpus = temp("live-writer");
    write_corpus(&corpus);
    let out = temp("live-writer-base").join("content");
    let reports = temp("live-writer-reports");
    compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    let good = tree(&out);

    // Compile A holds the lock and is part way through its write: a staging directory with files
    // but no manifest yet.
    let writer = OutLock::acquire(&out).unwrap();
    let staging = staging_dir(&out);
    fs::create_dir_all(staging.join("vnew")).unwrap();
    fs::write(staging.join("vnew/catalog.json"), "{}").unwrap();
    let staged = tree(&staging);

    // Compile B fails its gates, and compile C would pass them. Neither runs, and neither
    // touches the staging directory or the library.
    fs::write(
        corpus.join("books.toml"),
        write_books(&BookIndex::default()),
    )
    .unwrap();
    let failing = compile_corpus(&corpus, &out, &reports, None, None).expect_err("B must stop");
    assert!(
        failing
            .lines
            .join("\n")
            .contains("another compile may be writing")
    );
    fs::write(corpus.join("books.toml"), write_books(&printed_chapter())).unwrap();
    let passing = compile_corpus(&corpus, &out, &reports, None, None).expect_err("C must stop");
    assert!(
        passing
            .lines
            .join("\n")
            .contains("another compile may be writing")
    );
    assert_eq!(
        tree(&staging),
        staged,
        "a blocked compile changed the live staging"
    );
    assert_eq!(tree(&out), good, "a blocked compile changed the library");

    // A finishes, and the next compile takes the lock and succeeds. Neither leaves a lock file.
    drop(writer);
    assert!(!lock_file(&out).exists());
    compile_corpus(&corpus, &out, &reports, None, None).unwrap();
    assert!(!lock_file(&out).exists());
    assert!(!staging.exists());
}

/// Compiles `corpus` into a fresh directory and returns the library files by path.
fn compile_tree(corpus: &Path, name: &str) -> Vec<(String, Vec<u8>)> {
    let out = temp(&format!("{name}-out"));
    let reports = temp(&format!("{name}-reports"));
    compile_corpus(corpus, &out, &reports, None, Some("v0.1.6")).unwrap_or_else(|f| {
        panic!("{}", f.lines.join("\n"));
    });
    let manifest: ContentManifest =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    let version = out.join(&manifest.content_version);
    tree(&version)
}

#[test]
fn a_book_that_only_expands_an_entry_is_not_a_member_of_that_book() {
    let with = temp("expanded-with");
    let without = temp("expanded-without");
    write_corpus_with(&with, Some(true));
    write_corpus_with(&without, Some(false));
    let with_files = compile_tree(&with, "expanded-with");
    let without_files = compile_tree(&without, "expanded-without");

    // Everything the library derives from membership is identical with and without the credit:
    // the catalog (book counts and each entry's books), the search index, and every other file.
    // Only the shard that stores the credited entry may differ.
    let paths = |files: &[(String, Vec<u8>)]| files.iter().map(|f| f.0.clone()).collect::<Vec<_>>();
    assert_eq!(paths(&with_files), paths(&without_files));
    let differing: Vec<&str> = with_files
        .iter()
        .zip(&without_files)
        .filter(|(a, b)| a.1 != b.1)
        .map(|(a, _)| a.0.as_str())
        .collect();
    assert!(
        differing.iter().all(|path| path.starts_with("entries/")),
        "only an entry shard may differ: {differing:?}"
    );
    assert!(
        !differing.is_empty(),
        "the credit must reach the stored entry"
    );

    let catalog: cast_iron_schema::Catalog = serde_json::from_slice(
        &with_files
            .iter()
            .find(|f| f.0 == "catalog.json")
            .expect("catalog.json")
            .1,
    )
    .unwrap();
    let handbook = catalog
        .sources
        .iter()
        .find(|source| source.id == "monster-handbook")
        .expect("the book is listed");
    assert_eq!(handbook.entry_count, 0);
    let core = catalog
        .sources
        .iter()
        .find(|source| source.id == "core-rulebook")
        .unwrap();
    assert_eq!(core.entry_count, 2);
    let grapple = catalog
        .entries
        .iter()
        .find(|entry| entry.id.as_str() == "ref/rule/grapple")
        .unwrap();
    assert_eq!(grapple.source, "core-rulebook");
    assert!(grapple.books.is_empty(), "{:?}", grapple.books);
}

#[test]
fn compiling_the_same_corpus_with_expansions_twice_is_byte_identical() {
    let corpus = temp("expanded-twice");
    write_corpus_with(&corpus, Some(true));
    assert_eq!(
        compile_tree(&corpus, "expanded-twice-a"),
        compile_tree(&corpus, "expanded-twice-b")
    );
}

/// The compiled `books.json` of a library.
fn compiled_books(out: &Path) -> BookIndex {
    let manifest: ContentManifest =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    let file = manifest
        .files
        .iter()
        .find(|file| file.path.ends_with("/books.json"))
        .expect("books.json is in the manifest");
    serde_json::from_slice(&fs::read(out.join(&file.path)).unwrap()).unwrap()
}

#[test]
fn a_split_corpus_compiles_its_curated_book_and_derives_the_others_with_the_same_bytes() {
    let corpus = temp("split");
    write_corpus_full(&corpus, Some(false), SPLIT_BOOKS_FORMAT);
    // `monster-handbook` has no curated file and no member: the compiler still gives it a book.
    let mut trees = Vec::new();
    for run in 0..2 {
        let out = temp(&format!("split-out{run}"));
        let reports = temp(&format!("split-reports{run}"));
        compile_corpus(&corpus, &out, &reports, None, None)
            .unwrap_or_else(|f| panic!("{}", f.lines.join("\n")));
        trees.push(tree(&out));
        if run == 0 {
            let books = compiled_books(&out);
            let sources: Vec<_> = books.books.iter().map(|b| b.source.as_str()).collect();
            // Curated first, then the derived book.
            assert_eq!(sources, ["core-rulebook", "monster-handbook"]);
            assert_eq!(books.books[0].chapters, printed_chapter().books[0].chapters);
            assert!(books.books[1].chapters.is_empty());
        }
    }
    assert_eq!(trees[0], trees[1], "two compiles must write the same bytes");
}

#[test]
fn a_book_without_a_curated_file_is_derived_from_the_pages_and_reaches_every_member() {
    let corpus = temp("split-derived");
    write_corpus_full(&corpus, None, SPLIT_BOOKS_FORMAT);
    fs::remove_file(corpus.join("books/core-rulebook.toml")).unwrap();
    let out = temp("split-derived-out");
    let reports = temp("split-derived-reports");
    compile_corpus(&corpus, &out, &reports, None, None)
        .unwrap_or_else(|f| panic!("a derived book is reachable: {}", f.lines.join("\n")));
    let book = &compiled_books(&out).books[0];
    assert_eq!(book.title, "Core Rulebook");
    assert_eq!(
        book.chapters
            .iter()
            .map(|c| c.title.as_str())
            .collect::<Vec<_>>(),
        ["Unpaged"]
    );
    let ids: Vec<_> = book.chapters[0]
        .children
        .iter()
        .map(|node| node.entry.as_ref().unwrap().as_str())
        .collect();
    // Neither rule has a page; kind is equal, so the lowercased title decides.
    assert_eq!(ids, ["ref/rule/combat", "ref/rule/grapple"]);
}

#[test]
fn a_curated_book_gets_the_catch_all_chapter_for_the_entries_it_does_not_place() {
    let corpus = temp("split-catch-all");
    write_corpus_full(&corpus, None, SPLIT_BOOKS_FORMAT);
    let mut tree_without_grapple = printed_chapter().books.remove(0);
    tree_without_grapple.chapters = vec![node(
        "Combat",
        None,
        vec![node("Combat", Some("ref/rule/combat"), vec![])],
    )];
    fs::write(
        corpus.join("books/core-rulebook.toml"),
        write_curated_book(&tree_without_grapple),
    )
    .unwrap();
    let out = temp("split-catch-all-out");
    let reports = temp("split-catch-all-reports");
    // The rule the curated tree leaves out is only in the hidden chapter, which the gate refuses.
    let failure = compile_corpus(&corpus, &out, &reports, None, None)
        .expect_err("a rule that only the catch-all chapter reaches fails the gate");
    assert!(
        failure.lines.join("\n").contains("unreachable"),
        "{failure:?}"
    );
    let reachability = fs::read_to_string(reports.join("reachability.json")).unwrap();
    assert!(reachability.contains("ref/rule/grapple"), "{reachability}");
}

fn stat_set_blocks(title: &str, anchor: &str, cr: &str, size: &str) -> Vec<Block> {
    let row = |label: &str, value: &str| cast_iron_schema::StatRow {
        label: label.into(),
        value: vec![Inline::text(value)],
    };
    vec![
        Block::Heading {
            level: 2,
            anchor: Some(anchor.into()),
            content: vec![Inline::text(title)],
        },
        Block::Heading {
            level: 3,
            anchor: None,
            content: vec![Inline::text("Statistics")],
        },
        Block::StatBlock {
            title: None,
            rows: vec![
                row("Challenge rating", cr),
                row("Size", size),
                row("Type", "outsider"),
            ],
        },
    ]
}

#[test]
fn a_creature_with_several_stat_sets_stores_them_and_a_single_set_stores_none() {
    let corpus = temp("stat-sets");
    write_corpus(&corpus);
    let fields = |cr: &str| {
        TypedFields::Creature(Box::new(cast_iron_schema::CreatureFields {
            cr: cast_iron_schema::ChallengeRating::parse(cr),
            ..Default::default()
        }))
    };
    let mut body = stat_set_blocks("Small Air Elemental", "stat-small", "1", "Small");
    body.extend(stat_set_blocks(
        "Large Air Elemental",
        "stat-large",
        "5",
        "Large",
    ));
    let many = typed_entry(
        "ref/monster/air-elemental",
        EntryKind::Creature,
        "Air Elemental",
        body,
        fields("1"),
        vec![],
    );
    let single = typed_entry(
        "ref/monster/ettercap",
        EntryKind::Creature,
        "Ettercap",
        stat_set_blocks("Ettercap", "stat-ettercap", "3", "Medium"),
        fields("3"),
        vec![],
    );
    // The same two sections on a trap are no stat sets: only a creature has them.
    let mut template_body = stat_set_blocks("Young", "stat-young", "1", "Small");
    template_body.extend(stat_set_blocks("Giant", "stat-giant", "5", "Large"));
    let template = typed_entry(
        "ref/trap/young-creature",
        EntryKind::Trap,
        "Young Creature",
        template_body,
        TypedFields::None,
        vec![],
    );
    write_entries(&corpus, 2, &[many, single, template]);
    let files = compile_tree(&corpus, "stat-sets");
    let entries: Vec<RuleEntry> = files
        .iter()
        .filter(|(path, _)| path.starts_with("entries/"))
        .flat_map(|(_, bytes)| serde_json::from_slice::<Vec<RuleEntry>>(bytes).unwrap())
        .collect();
    let find = |id: &str| {
        entries
            .iter()
            .find(|entry| entry.id.as_str() == id)
            .unwrap()
    };
    let sets = &find("ref/monster/air-elemental").stat_sets;
    assert_eq!(sets.len(), 2, "{sets:?}");
    assert_eq!(sets[0].title, "Small Air Elemental");
    assert_eq!(sets[0].anchor, "stat-small");
    assert_eq!(sets[0].cr, cast_iron_schema::ChallengeRating::parse("1"));
    assert_eq!(sets[1].size.as_deref(), Some("Large"));
    assert_eq!(sets[1].cr, cast_iron_schema::ChallengeRating::parse("5"));
    assert!(
        find("ref/trap/young-creature").stat_sets.is_empty(),
        "a non-creature never derives stat sets"
    );
    assert!(
        find("ref/monster/ettercap").stat_sets.is_empty(),
        "one set means no list"
    );

    // The search index finds the creature under every set's rating, and a creature with one
    // set only under its own.
    let bytes = &files
        .iter()
        .find(|(path, _)| path == "search.bin")
        .unwrap()
        .1;
    let index = cast_iron_search::SearchIndex::from_bytes(bytes).unwrap();
    let under = |cr: &str| {
        let mut ids = index.facet_ids_eligible("cr", cr, |_| true);
        ids.sort_unstable();
        ids
    };
    assert_eq!(under("1"), ["ref/monster/air-elemental"]);
    assert_eq!(under("5"), ["ref/monster/air-elemental"]);
    assert_eq!(under("3"), ["ref/monster/ettercap"]);
}
