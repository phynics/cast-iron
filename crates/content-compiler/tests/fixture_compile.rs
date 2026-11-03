mod common;

use cast_iron_content_compiler::text::blocks_text;
use cast_iron_schema::validate::validate_library;
use cast_iron_schema::{Block, EntryKind, Inline, TypedFields};
use common::*;
use std::fs;

#[test]
fn nested_prestige_class_gets_its_own_category_without_changing_its_id() {
    let scratch = scratch("fixture-prestige");
    let config = config_dir(&scratch);
    let dataset = scratch.join("dataset");
    copy_dir(&fixtures().join("dataset"), &dataset);
    let path = dataset.join("book_a/rules/combat.json");
    let mut tree: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    tree["sections"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name": "Arcane Archer", "type": "class", "subtype": "prestige",
            "source": "Core Rulebook", "url": "pfsrd://Core Rulebook/Classes/Arcane Archer",
            "hit_die": "d10", "body": "<p>Prestige class rules.</p>"
        }));
    fs::write(&path, serde_json::to_vec(&tree).unwrap()).unwrap();
    let out = scratch.join("out");
    let mut opts = options(&config, &dataset, &out, &scratch.join("reports"));
    opts.write_digest = true;
    run(&opts);
    let library = load(&out);
    let entry = library.entry("core-rulebook/classes/arcane-archer");
    assert_eq!(entry.kind, EntryKind::PrestigeClass);
    assert!(
        matches!(&entry.fields, TypedFields::Class(fields) if fields.hit_die.as_deref() == Some("d10"))
    );
    assert!(
        library
            .catalog
            .entries
            .iter()
            .any(|stub| stub.id == entry.id && stub.kind == EntryKind::PrestigeClass)
    );
    assert_eq!(
        serde_json::to_value(entry).unwrap()["kind"],
        "prestige_class"
    );
}

/// Compiles the fixture dataset, pins its digest on the first run, and returns the outcome.
fn compile_fixture(
    name: &str,
) -> (
    std::path::PathBuf,
    std::path::PathBuf,
    cast_iron_content_compiler::pipeline::Outcome,
) {
    let scratch = scratch(name);
    let config = config_dir(&scratch);
    let dataset = fixtures().join("dataset");
    let out = scratch.join("out");
    let reports = scratch.join("reports");
    let mut opts = options(&config, &dataset, &out, &reports);
    opts.write_digest = true;
    let outcome = run(&opts);
    (out, reports, outcome)
}

#[test]
fn compiles_the_fixture_into_valid_content() {
    let (out, reports, outcome) = compile_fixture("fixture-basic");
    let lib = load(&out);
    assert_eq!(validate_library(&lib.entries, &lib.catalog), vec![]);
    assert_eq!(lib.manifest.entry_count as usize, lib.entries.len());
    assert!(
        lib.manifest.content_version.starts_with('v') && lib.manifest.content_version.len() == 17
    );

    // Rejected records are reported with a reason, and nothing else is dropped.
    let rejected: Vec<_> = outcome.rejected.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        rejected,
        [
            "book_a/spell/bad_levels.json",
            "book_a/spell/truncated.json"
        ],
        "{:?}",
        outcome.rejected
    );
    assert!(outcome.rejected[0].reason.contains("levels"));
    assert!(outcome.rejected[1].reason.contains("malformed JSON"));
    let coverage = fs::read_to_string(reports.join("coverage.md")).unwrap();
    assert!(coverage.contains("bad_levels.json") && coverage.contains("truncated.json"));
    let validation: serde_json::Value =
        serde_json::from_slice(&fs::read(reports.join("validation.json")).unwrap()).unwrap();
    assert_eq!(validation["ok"], true);

    // Typed record wins over copies.
    assert_eq!(
        outcome.dedupe.typed_identical, 1,
        "fireball_copy.json repeats fireball.json"
    );
    assert_eq!(
        outcome.dedupe.nested_replaced_identical, 1,
        "{:?}",
        outcome.dedupe
    );
    assert_eq!(outcome.dedupe.nested_replaced_conflicting, 1);
    let acid = lib.entry("core-rulebook/rules/combat/goods/acid");
    assert_eq!(acid.kind, EntryKind::Item);
    assert!(acid.provenance.len() == 2, "{:?}", acid.provenance);
    assert!(
        acid.provenance
            .iter()
            .any(|p| p.path == "book_a/rules/combat.json")
    );
    assert!(outcome_note(&reports).contains("conflicting duplicate"));

    // Kinds and ids.
    let kinds: Vec<_> = lib
        .entries
        .iter()
        .map(|e| (e.id.to_string(), e.kind.slug()))
        .collect();
    for expected in [
        ("core-rulebook/spells/fireball", "spell"),
        ("core-rulebook/spells/magic-missile", "spell"),
        ("core-rulebook/feats/acrobatic", "feat"),
        ("bestiary/monsters/dog/dog", "creature"),
        ("bestiary/monsters/weird", "creature"),
        ("core-rulebook/rules/combat", "rule"),
        ("core-rulebook/rules/combat/actions", "rule"),
    ] {
        assert!(
            kinds.contains(&(expected.0.to_string(), expected.1)),
            "{expected:?} missing from {kinds:?}"
        );
    }
    assert!(
        !lib.entries
            .iter()
            .any(|e| e.id.as_str().contains("elsewhere")),
        "unselected record leaked"
    );

    // Rules tree: parent links to children, children point back, deep sections flatten.
    let parent = lib.entry("core-rulebook/rules/combat");
    assert!(parent.body.iter().any(|b| matches!(b, Block::EntryLink { target, .. } if target.as_str() == "core-rulebook/rules/combat/actions")));
    assert!(parent.body.iter().any(|b| matches!(b, Block::EntryLink { target, .. } if target.as_str() == "core-rulebook/rules/combat/goods/acid")));
    assert!(blocks_text(&parent.body).contains("Drow Monster Entry"));
    let actions = lib.entry("core-rulebook/rules/combat/actions");
    assert_eq!(
        actions.parent.as_ref().unwrap().as_str(),
        "core-rulebook/rules/combat"
    );
    let headings: Vec<(u8, String)> = actions
        .body
        .iter()
        .filter_map(|b| match b {
            Block::Heading {
                level,
                anchor,
                content,
            } => Some((
                *level,
                format!(
                    "{}:{}",
                    anchor.clone().unwrap(),
                    cast_iron_content_compiler::text::inline_text(content)
                ),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        headings,
        [
            (2, "deep:Deep".to_string()),
            (3, "deeper:Deeper".to_string())
        ]
    );
    assert!(
        actions.body.iter().any(|b| matches!(b, Block::Table(_))),
        "table without tbody survives"
    );
    assert!(actions.body.iter().any(|b| matches!(b, Block::EntryLink { target, .. } if target.as_str() == "core-rulebook/rules/combat/goods/acid")));
    // A link to a deep section resolves to entry + anchor.
    let link = actions
        .links
        .iter()
        .find(|l| l.anchor.is_some())
        .expect("deep link");
    assert_eq!(
        (link.target.as_str(), link.anchor.as_deref()),
        ("core-rulebook/rules/combat/actions", Some("deeper"))
    );

    // Links: exact, prefix, out-of-selection, broken, empty.
    let fireball = lib.entry("core-rulebook/spells/fireball");
    let targets: Vec<_> = fireball.links.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(
        targets,
        [
            "bestiary/monsters/dog/dog",
            "core-rulebook/spells/magic-missile"
        ]
    );
    assert_eq!(outcome.links.out_of_selection, 1, "{:?}", outcome.links);
    assert_eq!(outcome.links.broken, 2, "{:?}", outcome.links);
    assert_eq!(outcome.links.resolved, 3, "{:?}", outcome.links);
    let text = blocks_text(&fireball.body);
    assert!(text.contains("elsewhere") && text.contains("nothing") && text.contains("empty"));
    assert!(text.contains("\u{201c}hit\u{201d} the opening\u{2014}or else"));
    assert_eq!(outcome.stripped["img"], 1);
    assert_eq!(outcome.stripped["ignored_field:as_spell_id"], 1);

    // Typed fields.
    let TypedFields::Spell(spell) = &fireball.fields else {
        panic!()
    };
    assert_eq!(spell.school, "evocation");
    assert_eq!(spell.levels.len(), 2);
    assert_eq!(
        spell.components.as_deref(),
        Some("V, S, M (a ball of bat guano and sulfur)")
    );
    let TypedFields::Feat(feat) = &lib.entry("core-rulebook/feats/acrobatic").fields else {
        panic!()
    };
    assert_eq!(feat.feat_types, ["General"]);
    assert_eq!(feat.prerequisites.as_deref(), Some("Dex 13; unclosed bold"));
    let TypedFields::Creature(dog) = &lib.entry("bestiary/monsters/dog/dog").fields else {
        panic!()
    };
    assert_eq!(dog.cr.unwrap().display(), "1/3");
    let TypedFields::Creature(weird) = &lib.entry("bestiary/monsters/weird").fields else {
        panic!()
    };
    assert_eq!(weird.cr, None);
    let TypedFields::Item(item) = &acid.fields else {
        panic!()
    };
    assert_eq!(item.slot.as_deref(), Some("none"));
    assert_eq!(item.attributes[0].0, "Gear Type");
    assert!(!item.attributes.iter().any(|(k, _)| k == "null"));

    // Creature body keeps the spells value as rich content and the nested table spans.
    let dog_entry = lib.entry("bestiary/monsters/dog/dog");
    let Block::StatBlock { rows, .. } = &dog_entry.body[0] else {
        panic!("{:?}", dog_entry.body[0])
    };
    assert_eq!(rows[0].label, "CR");
    let sla = rows
        .iter()
        .find(|r| r.label == "Spell-like abilities")
        .unwrap();
    assert!(sla.value.iter().any(|i| matches!(i, Inline::Em { .. })));
    let table = dog_entry
        .body
        .iter()
        .find_map(|b| {
            if let Block::Table(t) = b {
                Some(t)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        (
            table.head[0].cells[0].rowspan,
            table.head[0].cells[1].colspan
        ),
        (2, 2)
    );

    // Sources and legal text.
    let core = lib
        .catalog
        .sources
        .iter()
        .find(|s| s.id == "core-rulebook")
        .unwrap();
    assert!(
        core.copyright
            .as_deref()
            .unwrap()
            .starts_with("Pathfinder Roleplaying Game Core Rulebook. ")
    );
    assert_eq!(
        lib.catalog
            .sources
            .iter()
            .find(|s| s.id == "bestiary")
            .unwrap()
            .copyright,
        None
    );
    let legal: cast_iron_schema::LegalNotices =
        serde_json::from_slice(&fs::read(lib.dir.join("legal.json")).unwrap()).unwrap();
    assert_eq!(legal.licenses[0].id, "ogl-1.0a");
    assert!(
        legal.licenses[0]
            .text
            .contains("OPEN GAME LICENSE Version 1.0a")
    );
    assert!(
        legal
            .community_use_notice
            .starts_with("Cast Iron uses trademarks")
    );
    assert_eq!(legal.product_identity, "Fixture product identity.");
    assert!(
        lib.entries
            .iter()
            .all(|e| e.license_refs == ["ogl-1.0a"] && !e.provenance.is_empty())
    );

    // Coverage lists the unselected directory and the database file, and never reads it.
    let cov: cast_iron_schema::CoverageReport =
        serde_json::from_slice(&fs::read(lib.dir.join("coverage.json")).unwrap()).unwrap();
    let excluded: Vec<_> = cov
        .excluded
        .iter()
        .map(|e| (e.path.as_str(), e.records))
        .collect();
    assert!(
        excluded.contains(&("book_b", 1)) || excluded.contains(&("book_b/spell", 1)),
        "{excluded:?}"
    );
    assert!(
        excluded.contains(&("book_a/rules/combat", 1)),
        "{excluded:?}"
    );
    assert!(excluded.contains(&("*.db", 1)), "{excluded:?}");
    assert_eq!(cov.by_kind["spell"], 2);
}

fn outcome_note(reports: &std::path::Path) -> String {
    fs::read_to_string(reports.join("validation.json")).unwrap()
}

#[test]
fn output_is_byte_identical_across_runs_and_replaces_old_output() {
    let scratch = scratch("fixture-determinism");
    let config = config_dir(&scratch);
    let dataset = fixtures().join("dataset");
    let mut opts = options(
        &config,
        &dataset,
        &scratch.join("out-a"),
        &scratch.join("reports-a"),
    );
    opts.write_digest = true;
    run(&opts);
    opts.write_digest = false;
    run(&opts); // re-running into the same, now non-empty, directory cleans it first
    let first = tree(&scratch.join("out-a"));

    let opts_b = options(
        &config,
        &dataset,
        &scratch.join("out-b"),
        &scratch.join("reports-b"),
    );
    run(&opts_b);
    assert_eq!(first, tree(&scratch.join("out-b")));
    assert_eq!(
        tree(&scratch.join("reports-a")),
        tree(&scratch.join("reports-b"))
    );
    assert!(!first.is_empty());
    // The manifest hashes what is on disk.
    let lib = load(&scratch.join("out-a"));
    for f in &lib.manifest.files {
        use sha2::{Digest, Sha256};
        let digest: String = Sha256::digest(&first[&f.path])
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, f.sha256, "{}", f.path);
        assert_eq!(first[&f.path].len() as u64, f.bytes);
    }
    assert!(lib.manifest.files.windows(2).all(|w| w[0].path < w[1].path));
}

#[test]
fn digest_mismatch_and_empty_digest_fail_clearly() {
    let scratch = scratch("fixture-digest");
    let config = config_dir(&scratch);
    let dataset = fixtures().join("dataset");
    let opts = options(
        &config,
        &dataset,
        &scratch.join("out"),
        &scratch.join("reports"),
    );
    let err = cast_iron_content_compiler::pipeline::compile(&opts).unwrap_err();
    assert!(
        err.lines[0].contains("input_digest") && err.lines[0].contains("--write-digest"),
        "{err:?}"
    );

    let mut pin = opts.clone();
    pin.write_digest = true;
    run(&pin);
    run(&opts);
    let pinned = fs::read_to_string(config.join("source.toml")).unwrap();
    fs::write(
        config.join("source.toml"),
        pinned.replace("input_digest = \"", "input_digest = \"00"),
    )
    .unwrap();
    let err = cast_iron_content_compiler::pipeline::compile(&opts).unwrap_err();
    assert!(err.lines[0].contains("digest mismatch"), "{err:?}");
}

#[test]
fn strict_mode_fails_on_rejected_records() {
    let scratch = scratch("fixture-strict");
    let config = config_dir(&scratch);
    let dataset = fixtures().join("dataset");
    let mut opts = options(
        &config,
        &dataset,
        &scratch.join("out"),
        &scratch.join("reports"),
    );
    opts.write_digest = true;
    opts.strict = true;
    let err = cast_iron_content_compiler::pipeline::compile(&opts).unwrap_err();
    assert!(
        err.lines[0].contains("--strict")
            && err.lines.iter().any(|l| l.contains("bad_levels.json")),
        "{err:?}"
    );
    assert!(!scratch.join("out").exists());
}

#[test]
fn refuses_to_clean_a_directory_that_is_not_ours() {
    let scratch = scratch("fixture-foreign-out");
    let config = config_dir(&scratch);
    let dataset = fixtures().join("dataset");
    let out = scratch.join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("precious.txt"), "keep").unwrap();
    let mut opts = options(&config, &dataset, &out, &scratch.join("reports"));
    opts.write_digest = true;
    let err = cast_iron_content_compiler::pipeline::compile(&opts).unwrap_err();
    assert!(err.lines[0].contains("refusing to clean"), "{err:?}");
    assert_eq!(
        fs::read_to_string(out.join("precious.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn slug_collisions_fail_unless_overridden() {
    let scratch = scratch("fixture-collision");
    let config = config_dir(&scratch);
    let dataset = scratch.join("dataset");
    common::copy_dir(&fixtures().join("dataset"), &dataset);
    // "Magic Missile" and "Magic  Missile!" slugify to the same id.
    fs::write(
        dataset.join("book_a/spell/magic_missile_2.json"),
        r#"{"body":"<p>x</p>","school":"evocation","source":"Core Rulebook","url":"pfsrd://Core Rulebook/Spells/Magic  Missile!","type":"spell","name":"Magic Missile Two","levels":[{"class":"Wizard","level":1}]}"#,
    )
    .unwrap();
    let mut opts = options(
        &config,
        &dataset,
        &scratch.join("out"),
        &scratch.join("reports"),
    );
    opts.write_digest = true;
    let err = cast_iron_content_compiler::pipeline::compile(&opts).unwrap_err();
    let text = err.lines.join("\n");
    assert!(
        text.contains("core-rulebook/spells/magic-missile")
            && text.contains("pfsrd://Core Rulebook/Spells/Magic Missile")
            && text.contains("Magic  Missile!"),
        "{text}"
    );

    fs::write(
        config.join("id-overrides.toml"),
        "[ids]\n\"pfsrd://Core Rulebook/Spells/Magic  Missile!\" = \"core-rulebook/spells/magic-missile-two\"\n",
    )
    .unwrap();
    run(&opts);
    let lib = load(&scratch.join("out"));
    assert_eq!(
        lib.entry("core-rulebook/spells/magic-missile-two").title,
        "Magic Missile Two"
    );
}

#[test]
fn build_order_does_not_change_ids() {
    // Renaming the files changes the order in which they are read, but not the ids.
    let scratch = scratch("fixture-order");
    let config = config_dir(&scratch);
    let dataset = scratch.join("dataset");
    common::copy_dir(&fixtures().join("dataset"), &dataset);
    let mut opts = options(
        &config,
        &dataset,
        &scratch.join("out"),
        &scratch.join("reports"),
    );
    opts.write_digest = true;
    run(&opts);
    let before: Vec<String> = load(&scratch.join("out"))
        .entries
        .iter()
        .map(|e| e.id.to_string())
        .collect();
    fs::rename(
        dataset.join("book_a/spell/magic_missile.json"),
        dataset.join("book_a/spell/aaa_magic_missile.json"),
    )
    .unwrap();
    run(&opts);
    let after: Vec<String> = load(&scratch.join("out"))
        .entries
        .iter()
        .map(|e| e.id.to_string())
        .collect();
    assert_eq!(before, after);
}

#[test]
fn deeply_nested_sections_do_not_overflow_the_stack() {
    let scratch = scratch("fixture-deep");
    let config = config_dir(&scratch);
    let dataset = scratch.join("dataset");
    common::copy_dir(&fixtures().join("dataset"), &dataset);
    let depth = 55;
    let mut json = String::new();
    for i in 0..depth {
        json.push_str(&format!(r#"{{"name":"Level {i}","type":"section","source":"Core Rulebook","body":"<p>text {i}</p>","sections":["#));
    }
    for _ in 0..depth {
        json.push_str("]}");
    }
    let tree = format!(
        r#"{{"name":"Deep","url":"pfsrd://Core Rulebook/Rules/Deep","source":"Core Rulebook","type":"section","sections":[{json}]}}"#
    );
    fs::write(dataset.join("book_a/rules/deep.json"), tree).unwrap();
    let toml = fs::read_to_string(config.join("source.toml")).unwrap();
    fs::write(
        config.join("source.toml"),
        toml.replace(
            "\"book_a/rules/combat.json\"",
            "\"book_a/rules/combat.json\", \"book_a/rules/deep.json\"",
        ),
    )
    .unwrap();
    let mut opts = options(
        &config,
        &dataset,
        &scratch.join("out"),
        &scratch.join("reports"),
    );
    opts.write_digest = true;
    run(&opts);
    let lib = load(&scratch.join("out"));
    let deep = lib.entry("core-rulebook/rules/deep/level-0");
    let levels: Vec<u8> = deep
        .body
        .iter()
        .filter_map(|b| {
            if let Block::Heading { level, .. } = b {
                Some(*level)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(levels.len(), depth - 1);
    assert_eq!(levels[0], 2);
    assert!(levels.iter().all(|l| (2..=6).contains(l)) && *levels.last().unwrap() == 6);
    assert!(blocks_text(&deep.body).contains(&format!("text {}", depth - 1)));
}
