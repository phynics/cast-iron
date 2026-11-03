//! Relations, spell lists, and book order on a fixture of real-shaped PSRD-Data fragments:
//! a race with racial traits, a class with an archetype, a feat chain, traits, and spells.

mod common;

use cast_iron_schema::validate::{validate_derived, validate_library};
use cast_iron_schema::{
    Block, BookIndex, BookNode, CoverageReport, EntryKind, RelationIndex, RelationKind, SpellLists,
    TypedFields,
};
use common::*;
use std::fs;
use std::path::{Path, PathBuf};

fn relations_fixture() -> PathBuf {
    fixtures().join("relations")
}

fn relation_config(scratch: &Path) -> PathBuf {
    let dir = config_dir(scratch);
    for name in ["source.toml", "book-order.toml", "upstream-order.json"] {
        fs::copy(relations_fixture().join(name), dir.join(name)).unwrap();
    }
    dir
}

struct Compiled {
    out: PathBuf,
    scratch: PathBuf,
    config: PathBuf,
    library: Loaded,
}

fn compile_relations(name: &str) -> Compiled {
    let scratch = scratch(name);
    let config = relation_config(&scratch);
    let out = scratch.join("out");
    let mut opts = options(
        &config,
        &relations_fixture().join("dataset"),
        &out,
        &scratch.join("reports"),
    );
    opts.write_digest = true;
    run(&opts);
    let library = load(&out);
    Compiled {
        out,
        scratch,
        config,
        library,
    }
}

fn read<T: serde::de::DeserializeOwned>(c: &Compiled, file: &str) -> T {
    serde_json::from_slice(&fs::read(c.library.dir.join(file)).unwrap()).unwrap()
}

fn find<'a>(nodes: &'a [BookNode], title: &str) -> &'a BookNode {
    nodes.iter().find(|n| n.title == title).unwrap_or_else(|| {
        panic!(
            "no node {title} in {:?}",
            nodes.iter().map(|n| &n.title).collect::<Vec<_>>()
        )
    })
}

#[test]
fn racial_traits_belong_to_their_race_and_leave_the_race_body_as_links() {
    let c = compile_relations("rel-race");
    let lib = &c.library;
    assert_eq!(validate_library(&lib.entries, &lib.catalog), vec![]);

    let darkvision = lib.entry("core-rulebook/races/dwarves/dwarf-racial-traits/darkvision");
    assert_eq!(darkvision.kind, EntryKind::RacialTrait);
    assert!(matches!(&darkvision.fields, TypedFields::RacialTrait(f) if !f.alternate));
    assert_eq!(darkvision.relations.len(), 1);
    assert_eq!(darkvision.relations[0].kind, RelationKind::Part);
    assert_eq!(
        darkvision.relations[0].target.as_str(),
        "core-rulebook/races/dwarves"
    );

    // The race body holds links, not a second copy of the trait text.
    let dwarves = lib.entry("core-rulebook/races/dwarves");
    assert_eq!(
        dwarves.tags,
        ["standard_race"],
        "the race group separates player and monster races"
    );
    let links: Vec<&str> = dwarves
        .body
        .iter()
        .filter_map(|b| match b {
            Block::EntryLink { target, .. } => Some(target.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        links,
        [
            "core-rulebook/races/dwarves/dwarf-racial-traits/darkvision",
            "core-rulebook/races/dwarves/dwarf-racial-traits/hardy"
        ]
    );
    let text = cast_iron_content_compiler::text::blocks_text(&dwarves.body);
    assert!(!text.contains("see in the dark"), "{text}");

    // An Advanced Player's Guide alternate trait names the Gnomes of the Core Rulebook.
    let tinker = lib
        .entry("advanced-players-guide/rules/races/gnomes/alternate-racial-traits/master-tinker");
    assert!(matches!(&tinker.fields, TypedFields::RacialTrait(f) if f.alternate));
    assert_eq!(
        tinker.relations[0].target.as_str(),
        "core-rulebook/races/gnomes"
    );
    assert_eq!(tinker.relations[0].label.as_deref(), Some("alternate"));
}

#[test]
fn archetypes_point_at_their_class_and_class_text_is_not_promoted() {
    let c = compile_relations("rel-archetype");
    let lib = &c.library;
    let breaker = lib
        .entry("advanced-players-guide/rules/core-classes/barbarian/barbarian-archetypes/breaker");
    assert_eq!(breaker.kind, EntryKind::Archetype);
    assert_eq!(breaker.relations[0].kind, RelationKind::Archetype);
    assert_eq!(
        breaker.relations[0].target.as_str(),
        "core-rulebook/classes/barbarian"
    );

    // The class section links to it instead of repeating it.
    let section = lib.entry("advanced-players-guide/rules/core-classes/barbarian");
    assert!(section.links.iter().any(|l| l.target == breaker.id));
    let text = cast_iron_content_compiler::text::blocks_text(&section.body);
    assert!(!text.contains("sunder"), "{text}");

    // Antipaladin class text typed as an archetype upstream stays text and is reported.
    assert!(
        lib.entries
            .iter()
            .all(|e| e.kind != EntryKind::Archetype || e.id == breaker.id)
    );
    let coverage: CoverageReport = read(&c, "coverage.json");
    assert_eq!(coverage.relations.unpromoted.len(), 1);
    assert!(
        coverage.relations.unpromoted[0]
            .path
            .ends_with("class_archetype/paladin/class_skills.json")
    );
    let paladin_section = lib.entry("advanced-players-guide/rules/core-classes/paladin");
    let text = cast_iron_content_compiler::text::blocks_text(&paladin_section.body);
    assert!(text.contains("Bluff, Climb, and Intimidate"), "{text}");
}

#[test]
fn feat_prerequisites_resolve_into_a_chain_and_keep_the_original_text() {
    let c = compile_relations("rel-feats");
    let lib = &c.library;
    let greater = lib.entry("core-rulebook/feats/greater-bull-rush");
    let TypedFields::Feat(fields) = &greater.fields else {
        panic!()
    };
    assert_eq!(
        fields.prerequisites.as_deref(),
        Some("Str 13, Improved Bull Rush, Power Attack, base attack bonus +6.")
    );
    let ids: Vec<&str> = fields
        .prerequisite_feats
        .iter()
        .map(|i| i.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "core-rulebook/feats/improved-bull-rush",
            "core-rulebook/feats/power-attack"
        ]
    );
    assert!(
        greater
            .relations
            .iter()
            .all(|r| r.kind == RelationKind::Prerequisite && r.label.is_none())
    );

    // The wording that sits in `description`, not `body`.
    let improved = lib.entry("core-rulebook/feats/improved-iron-will");
    assert!(matches!(&improved.fields, TypedFields::Feat(f)
        if f.prerequisites.as_deref() == Some("Iron Will.")
            && f.prerequisite_feats.len() == 1));

    // An either-or list marks its alternatives; the unknown third one stays unread.
    let marked = lib.entry("core-rulebook/feats/marked-for-glory-mythic");
    assert_eq!(marked.relations.len(), 2);
    assert!(
        marked
            .relations
            .iter()
            .all(|r| r.label.as_deref() == Some("or"))
    );

    let coverage: CoverageReport = read(&c, "coverage.json");
    let p = &coverage.relations.prerequisites;
    assert_eq!(p.feats, 7);
    assert_eq!(p.feats_with_text, 5);
    assert_eq!(p.by_reading["ability_score"], 3);
    assert_eq!(p.by_reading["base_attack_bonus"], 3);
    assert_eq!(p.by_reading["feat"], 6);
    assert_eq!(p.by_reading["other"], 1);
    assert_eq!(p.unread[0].text, "Lightning Reflexes");
}

#[test]
fn traits_carry_a_category_and_the_edge_index_matches_the_entries() {
    let c = compile_relations("rel-index");
    let lib = &c.library;
    let blood = lib.entry(
        "ultimate-campaign/rules/character-background/traits/race-traits/bloodline-race-traits/blood-of-dragons",
    );
    assert!(matches!(&blood.fields, TypedFields::Trait(f)
        if f.category == "Race" && f.detail.as_deref() == Some("Bloodline")));
    let attached =
        lib.entry("ultimate-campaign/rules/character-background/traits/drawbacks/attached");
    assert!(matches!(&attached.fields, TypedFields::Trait(f) if f.category == "Drawback"));

    let index: RelationIndex = read(&c, "relations.json");
    // 3 parts (2 CRB racial traits, 1 APG), 1 archetype, 6 prerequisites.
    assert_eq!(index.counts["part"], 3);
    assert_eq!(index.counts["archetype"], 1);
    assert_eq!(index.counts["prerequisite"], 6);
    let json = fs::read_to_string(c.library.dir.join("relations.json")).unwrap();
    assert!(
        json.contains(r#"["core-rulebook/feats/greater-bull-rush","prerequisite","core-rulebook/feats/improved-bull-rush"]"#),
        "edges are [source, kind, target] arrays"
    );
    assert!(
        index.edges.windows(2).all(|w| w[0] < w[1]),
        "sorted, no duplicates"
    );
    let books: BookIndex = read(&c, "books.json");
    let lists: SpellLists = read(&c, "lists.json");
    assert_eq!(
        validate_derived(&lib.entries, &index, &books, &lists),
        vec![]
    );
}

#[test]
fn spell_lists_are_indexed_by_class_and_level_with_unmapped_classes_visible() {
    let c = compile_relations("rel-lists");
    let lists: SpellLists = read(&c, "lists.json");
    let titles = |class: &str, level: u8| -> Vec<String> {
        let list = lists.classes.iter().find(|l| l.name == class).unwrap();
        list.levels
            .iter()
            .find(|l| l.level == level)
            .map(|l| {
                l.spells
                    .iter()
                    .map(|&p| lists.spells[p as usize].1.clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    assert_eq!(titles("Wizard", 3), ["Fireball"]);
    assert_eq!(titles("Wizard", 2), ["Acid Arrow"]);
    assert_eq!(titles("Sorcerer", 3), ["Fireball"]);
    let wizard = lists.classes.iter().find(|l| l.name == "Wizard").unwrap();
    assert_eq!(
        wizard.class.as_ref().unwrap().as_str(),
        "core-rulebook/classes/wizard"
    );
    // Sorcerer has no class entry in this fixture, and Hunter never does in this library.
    let hunter = lists.classes.iter().find(|l| l.name == "Hunter").unwrap();
    assert!(hunter.class.is_none());
    assert_eq!(
        titles("Hunter", 3),
        ["Fireball"],
        "the repeated upstream pair counts once"
    );

    let coverage: CoverageReport = read(&c, "coverage.json");
    let s = &coverage.relations.spell_lists;
    assert_eq!((s.spells, s.classes), (2, 3));
    assert_eq!((s.pairs_upstream, s.pairs_distinct), (6, 5));
    assert_eq!(s.classes_with_entry, ["Wizard"]);
    let unmapped: Vec<&str> = s.unmapped.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(unmapped, ["Hunter", "Sorcerer"]);
}

#[test]
fn the_book_view_follows_chapters_nesting_and_upstream_order() {
    let c = compile_relations("rel-books");
    let books: BookIndex = read(&c, "books.json");
    let core = books
        .books
        .iter()
        .find(|b| b.source == "core-rulebook")
        .unwrap();
    let chapter_titles: Vec<&str> = core.chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(chapter_titles, ["Races", "Classes", "Feats", "Spells"]);
    // The draft status of the curated list reaches the reader.
    assert_eq!(core.basis, "Fixture.");
    assert_eq!(core.review, ["Fixture question."]);
    assert_eq!(
        core.chapters[0].review.as_deref(),
        Some("Fixture position is a guess.")
    );
    assert_eq!(core.chapters[1].review, None);

    // Races follow upstream-order.json (Elves, Dwarves, Gnomes), not the alphabet.
    let races = find(&core.chapters, "Races");
    let order: Vec<&str> = races.children.iter().map(|n| n.title.as_str()).collect();
    assert_eq!(order, ["Elves", "Dwarves", "Gnomes"]);
    // Racial traits sit under their race, below the heading that holds them.
    let heading = &find(&races.children, "Dwarves").children[0];
    assert_eq!(heading.entry, None);
    assert_eq!(heading.anchor.as_deref(), Some("dwarf-racial-traits"));
    let traits: Vec<&str> = heading.children.iter().map(|n| n.title.as_str()).collect();
    assert_eq!(traits, ["Darkvision", "Hardy"]);
    // Spells: upstream order again (Fireball before Acid Arrow).
    let spells: Vec<&str> = find(&core.chapters, "Spells")
        .children
        .iter()
        .map(|n| n.title.as_str())
        .collect();
    assert_eq!(spells, ["Fireball", "Acid Arrow"]);

    // The Advanced Player's Guide chapter is the rules tree itself; the archetype is nested
    // in the class section, under its archetypes heading.
    let apg = books
        .books
        .iter()
        .find(|b| b.source == "advanced-players-guide")
        .unwrap();
    let classes = find(&apg.chapters, "Core Classes");
    assert_eq!(
        classes.entry.as_ref().unwrap().as_str(),
        "advanced-players-guide/rules/core-classes"
    );
    let barbarian = find(&classes.children, "Barbarian");
    let archetypes = &barbarian.children[0];
    assert_eq!(archetypes.anchor.as_deref(), Some("barbarian-archetypes"));
    assert_eq!(archetypes.children[0].title, "Breaker");

    // Every entry is placed exactly once or reported.
    let coverage: CoverageReport = read(&c, "coverage.json");
    let mut seen = std::collections::BTreeSet::new();
    fn collect(n: &BookNode, seen: &mut std::collections::BTreeSet<String>) {
        if let (Some(e), None) = (&n.entry, &n.anchor) {
            assert!(seen.insert(e.to_string()), "{e} appears twice");
        }
        for child in &n.children {
            collect(child, seen);
        }
    }
    for tree in &books.books {
        for chapter in &tree.chapters {
            collect(chapter, &mut seen);
        }
    }
    let unplaced: Vec<&str> = coverage
        .relations
        .book_order
        .unplaced
        .iter()
        .map(|u| u.entry.as_str())
        .collect();
    assert_eq!(seen.len() + unplaced.len(), c.library.entries.len());
    assert_eq!(coverage.relations.book_order.placed as usize, seen.len());
    assert!(unplaced.is_empty(), "{unplaced:?}");
}

#[test]
fn entries_no_chapter_claims_are_reported_and_kept() {
    let c = compile_relations("rel-unplaced");
    let trimmed = fs::read_to_string(c.config.join("book-order.toml"))
        .unwrap()
        .replace(
            "[[book.chapter]]\ntitle = \"Feats\"\nmatch = [{ url = \"pfsrd://Core Rulebook/Feats\" }]\n",
            "",
        );
    fs::write(c.config.join("book-order.toml"), trimmed).unwrap();
    let out = c.scratch.join("out2");
    run(&options(
        &c.config,
        &relations_fixture().join("dataset"),
        &out,
        &c.scratch.join("reports2"),
    ));
    let library = load(&out);
    assert_eq!(
        library.entries.len(),
        c.library.entries.len(),
        "nothing is dropped"
    );
    let coverage: CoverageReport =
        serde_json::from_slice(&fs::read(library.dir.join("coverage.json")).unwrap()).unwrap();
    let unplaced = &coverage.relations.book_order.unplaced;
    assert_eq!(unplaced.len(), 7, "the seven feats");
    assert!(
        unplaced
            .iter()
            .all(|u| u.entry.starts_with("core-rulebook/feats/"))
    );
    assert!(unplaced[0].reason.contains("no chapter"));
}

#[test]
fn relation_output_is_byte_identical_across_runs() {
    let c = compile_relations("rel-determinism-a");
    let again = scratch("rel-determinism-b");
    let config = relation_config(&again);
    fs::copy(c.config.join("source.toml"), config.join("source.toml")).unwrap();
    let out = again.join("out");
    run(&options(
        &config,
        &relations_fixture().join("dataset"),
        &out,
        &again.join("reports"),
    ));
    assert_eq!(tree(&c.out), tree(&out));
    for file in ["relations.json", "lists.json", "books.json"] {
        let name = format!("{}/{file}", c.library.manifest.content_version);
        assert!(
            c.library.manifest.files.iter().any(|f| f.path == name),
            "{name} is in the manifest"
        );
    }
    assert_eq!(c.library.manifest.schema_version, 3);
}

#[test]
fn a_book_order_file_with_an_unknown_kind_fails_the_compile() {
    let c = compile_relations("rel-bad-order");
    let bad = fs::read_to_string(c.config.join("book-order.toml"))
        .unwrap()
        .replace(
            "match = [{ url = \"pfsrd://Core Rulebook/Races\" }]",
            "match = [{ url = \"pfsrd://Core Rulebook/Races\", kinds = [\"vehicle\"] }]",
        );
    fs::write(c.config.join("book-order.toml"), bad).unwrap();
    let error = cast_iron_content_compiler::pipeline::compile(&options(
        &c.config,
        &relations_fixture().join("dataset"),
        &c.scratch.join("out3"),
        &c.scratch.join("reports3"),
    ))
    .unwrap_err();
    assert!(error.lines[0].contains("unknown kind"), "{error:?}");
}
