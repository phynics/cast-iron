//! The corpus build, end to end, on a tiny synthetic corpus.

use cast_iron_content_compiler::corpus_build::{compile_corpus, compile_corpus_with_slots};
use cast_iron_corpus::{
    Config, Exclusions, entry_path, write_books, write_config, write_entry, write_exclusions,
    write_history, write_legal, write_publications,
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

/// `expansion` adds a second book, `monster-handbook`, that no entry prints. `Some(true)` makes
/// the grapple rule name it in `expanded_in`; `Some(false)` leaves the rule as it was.
fn write_corpus_with(dir: &Path, expansion: Option<bool>) {
    let config = Config::default();
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
        ("books.toml", write_books(&printed_chapter())),
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
        expanded_in: vec![],
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
        race_group: None,
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
        expanded_in: vec![],
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
        race_group: None,
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
        expanded_in: vec![],
        section: vec![],
        parent: None,
        body: vec![],
        fields: TypedFields::Class(Default::default()),
        tags: vec![],
        race_group: None,
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
