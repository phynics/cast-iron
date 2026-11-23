use crate::*;

fn doc(
    id: &str,
    title: &str,
    kind: &str,
    source: &str,
    text: &str,
    facets: &[(&str, &str)],
) -> DocInput {
    DocInput {
        id: id.into(),
        title: title.into(),
        aliases: vec![],
        kind: kind.into(),
        source: source.into(),
        section: "S".into(),
        facets: facets
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
        text: text.into(),
        ..DocInput::default()
    }
}

fn corpus() -> Vec<DocInput> {
    vec![
        doc(
            "spell:fireball",
            "Fireball",
            "spell",
            "core",
            "A burst of flame explodes. Deals fire damage.",
            &[
                ("school", "evocation"),
                ("class", "wizard"),
                ("class", "sorcerer"),
                ("spell_level", "3"),
            ],
        ),
        doc(
            "spell:dbf",
            "Delayed Blast Fireball",
            "spell",
            "core",
            "A delayed explosion of flame.",
            &[
                ("school", "evocation"),
                ("class", "wizard"),
                ("spell_level", "7"),
            ],
        ),
        doc(
            "spell:wall",
            "Wall of Fire",
            "spell",
            "apg",
            "Creates a wall that burns.",
            &[
                ("school", "evocation"),
                ("class", "wizard"),
                ("spell_level", "4"),
            ],
        ),
        doc(
            "spell:mm",
            "Magic Missile",
            "spell",
            "core",
            "Unlike a fireball, this never misses.",
            &[
                ("school", "evocation"),
                ("class", "wizard"),
                ("spell_level", "1"),
            ],
        ),
        doc(
            "feat:power",
            "Power Attack",
            "feat",
            "core",
            "You can make a fierce attack.",
            &[("feat_type", "combat")],
        ),
        doc(
            "feat:skill",
            "Skill Focus",
            "feat",
            "apg",
            "Pick a skill.",
            &[("feat_type", "general")],
        ),
        doc(
            "mon:wisp",
            "Will-o'-Wisp",
            "monster",
            "bestiary",
            "A glowing orb of light.",
            &[("creature_type", "aberration"), ("cr", "6")],
        ),
        doc(
            "mon:orc",
            "Orc Warrior",
            "monster",
            "bestiary",
            "A brutal humanoid.",
            &[("creature_type", "humanoid"), ("cr", "1/2")],
        ),
    ]
}

fn build(docs: Vec<DocInput>) -> SearchIndex {
    let mut b = IndexBuilder::new();
    for d in docs {
        b.add(d);
    }
    b.build()
}

fn q(text: &str) -> Query {
    Query {
        text: text.into(),
        ..Default::default()
    }
}

fn ids(idx: &SearchIndex, query: &Query) -> Vec<String> {
    idx.search(query).into_iter().map(|h| h.id).collect()
}

#[test]
fn eligible_ids_are_applied_before_limit_for_text_and_empty_queries() {
    let idx = build(vec![
        doc("item:a", "Grapple Hook", "item", "core", "A hook.", &[]),
        doc(
            "rule:b",
            "Special Attacks",
            "rule",
            "core",
            "Grapple checks use CMB.",
            &[],
        ),
        doc("item:c", "Grappling Hook", "item", "core", "A tool.", &[]),
    ]);
    let eligible = |id: &str| id == "rule:b";
    let mut query = q("grapple");
    query.limit = 1;
    assert_eq!(idx.search_eligible(&query, eligible)[0].id, "rule:b");

    query.text.clear();
    query.filters.kinds.push("item".into());
    assert!(idx.search_eligible(&query, eligible).is_empty());
    query.filters.kinds.push("rule".into());
    assert_eq!(idx.search_eligible(&query, eligible)[0].id, "rule:b");
}

#[test]
fn eligible_facets_exclude_disabled_documents() {
    let idx = build(vec![
        doc(
            "spell:a",
            "Fire",
            "spell",
            "core",
            "",
            &[("school", "evocation")],
        ),
        doc(
            "spell:b",
            "Frost",
            "spell",
            "core",
            "",
            &[("school", "abjuration")],
        ),
    ]);
    let eligible = |id: &str| id == "spell:a";
    assert_eq!(idx.facet_fields_eligible(eligible), vec!["school"]);
    assert_eq!(
        idx.facet_values_eligible("school", eligible),
        vec![("evocation".into(), 1)]
    );
}

#[test]
fn facet_ids_list_the_eligible_documents_with_a_value_ignoring_case() {
    let idx = build(vec![
        doc(
            "feat:a",
            "Alpha",
            "feat",
            "core",
            "",
            &[("feat_type", "Combat")],
        ),
        doc(
            "feat:b",
            "Beta",
            "feat",
            "core",
            "",
            &[("feat_type", "General")],
        ),
        doc(
            "feat:c",
            "Gamma",
            "feat",
            "core",
            "",
            &[("feat_type", "Combat"), ("feat_type", "Teamwork")],
        ),
        doc(
            "feat:d",
            "Delta",
            "feat",
            "ugc",
            "",
            &[("feat_type", "combat")],
        ),
    ]);
    let all = |_: &str| true;
    assert_eq!(
        idx.facet_ids_eligible("feat_type", "combat", all),
        ["feat:a", "feat:c", "feat:d"]
    );
    assert_eq!(
        idx.facet_ids_eligible("FEAT_TYPE", "Teamwork", all),
        ["feat:c"]
    );
    // A disabled book's documents are left out.
    assert_eq!(
        idx.facet_ids_eligible("feat_type", "Combat", |id| id != "feat:d"),
        ["feat:a", "feat:c"]
    );
    assert!(
        idx.facet_ids_eligible("feat_type", "Missing", all)
            .is_empty()
    );
    assert!(idx.facet_ids_eligible("school", "Combat", all).is_empty());
}

#[test]
fn exact_rule_section_beats_an_item_title_prefix_in_general_search() {
    let mut rule = doc(
        "rule:special-attacks",
        "Special Attacks",
        "rule",
        "core",
        "Grapple checks use CMB.",
        &[],
    );
    rule.aliases.push("Grapple".into());
    let idx = build(vec![
        doc(
            "item:grapple",
            "Grapple Hook",
            "item",
            "core",
            "A hook.",
            &[],
        ),
        rule,
    ]);
    assert_eq!(ids(&idx, &q("grapple"))[0], "rule:special-attacks");
}

#[test]
fn normalization() {
    assert_eq!(normalize("Will-o'-Wisp"), "will o wisp");
    assert_eq!(normalize("Dragon\u{2019}s Breath"), "dragons breath");
    assert_eq!(
        normalize("  Café — Crème\u{2013}Brûlée!! "),
        "cafe creme brulee"
    );
    assert_eq!(normalize("Fire,   Ball\n\tX"), "fire ball x");
    assert_eq!(normalize("\u{201C}Quoted\u{201D}"), "quoted");
    assert_eq!(normalize("Dragon's"), "dragons");
    assert_eq!(normalize(""), "");
    assert_eq!(normalize("1/2"), "1 2");
}

#[test]
fn tokenization() {
    assert_eq!(
        tokenize("The Will-o'-Wisp of Doom"),
        vec!["the", "will", "o", "wisp", "of", "doom"]
    );
    assert!(tokenize("  ,.; ").is_empty());
}

#[test]
fn ranking_exact_title_first() {
    let idx = build(corpus());
    let r = ids(&idx, &q("fireball"));
    assert_eq!(r, vec!["spell:fireball", "spell:dbf", "spell:mm"]);
}

#[test]
fn prefix_title_before_body() {
    let idx = build(corpus());
    let r = ids(&idx, &q("fire"));
    let pos = |id: &str| r.iter().position(|x| x == id).unwrap();
    assert!(pos("spell:wall") < pos("spell:mm"));
    assert!(pos("spell:fireball") < pos("spell:mm"));
    assert!(pos("spell:dbf") < pos("spell:mm"));
    assert_eq!(r[0], "spell:fireball");
}

#[test]
fn and_semantics_and_last_term_prefix() {
    let idx = build(corpus());
    assert_eq!(ids(&idx, &q("delayed fire")), vec!["spell:dbf"]);
    assert_eq!(ids(&idx, &q("wall fi")), vec!["spell:wall"]);
    assert!(ids(&idx, &q("fireball orc")).is_empty());
    assert!(ids(&idx, &q("zzzz")).is_empty());
    // Non-last term with no exact postings may still prefix-match.
    assert_eq!(ids(&idx, &q("dela blast")), vec!["spell:dbf"]);
    assert_eq!(ids(&idx, &q("will o wisp")), vec!["mon:wisp"]);
    assert_eq!(ids(&idx, &q("Will-o'-Wisp")), vec!["mon:wisp"]);
}

#[test]
fn exact_ranks_above_prefix_only() {
    let idx = build(vec![
        doc("a", "Firebrand", "x", "s", "", &[]),
        doc("b", "Fire", "x", "s", "", &[]),
        doc("c", "Mention", "x", "s", "fire fire", &[]),
        doc("d", "Mention Two", "x", "s", "firebrand", &[]),
    ]);
    let r = ids(&idx, &q("fire"));
    assert_eq!(r[0], "b");
    assert_eq!(r[1], "a");
    assert!(r.iter().position(|x| x == "c") < r.iter().position(|x| x == "d"));
}

#[test]
fn alias_match() {
    let mut d = doc("a", "Greater Teleport", "spell", "core", "", &[]);
    d.aliases = vec!["Jump".into()];
    let idx = build(vec![d, doc("b", "Unrelated", "spell", "core", "jump", &[])]);
    assert_eq!(ids(&idx, &q("jump"))[0], "a");
}

#[test]
fn filters() {
    let idx = build(corpus());
    let f = |kinds: &[&str], sources: &[&str], facets: &[(&str, &str)]| Filters {
        kinds: kinds.iter().map(|s| s.to_string()).collect(),
        sources: sources.iter().map(|s| s.to_string()).collect(),
        facets: facets
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
    };
    let run = |filters: Filters, text: &str| {
        ids(
            &idx,
            &Query {
                text: text.into(),
                filters,
                limit: 0,
            },
        )
    };
    assert_eq!(
        run(f(&["feat"], &[], &[]), ""),
        vec!["feat:power", "feat:skill"]
    );
    assert_eq!(
        run(f(&["feat", "monster"], &["apg"], &[]), ""),
        vec!["feat:skill"]
    );
    assert_eq!(
        run(f(&["spell"], &["core"], &[("spell_level", "3")]), ""),
        vec!["spell:fireball"]
    );
    // OR within a facet field.
    assert_eq!(
        run(
            f(&[], &[], &[("spell_level", "3"), ("spell_level", "7")]),
            ""
        ),
        vec!["spell:dbf", "spell:fireball"]
    );
    // AND across fields.
    assert_eq!(
        run(
            f(&[], &[], &[("school", "evocation"), ("class", "sorcerer")]),
            ""
        ),
        vec!["spell:fireball"]
    );
    assert_eq!(
        run(f(&[], &[], &[("feat_type", "combat")]), ""),
        vec!["feat:power"]
    );
    assert_eq!(
        run(f(&[], &[], &[("creature_type", "humanoid")]), ""),
        vec!["mon:orc"]
    );
    assert_eq!(run(f(&[], &[], &[("cr", "1/2")]), ""), vec!["mon:orc"]);
    assert_eq!(
        run(f(&[], &[], &[("CLASS", "Wizard")]), "fireball"),
        vec!["spell:fireball", "spell:dbf", "spell:mm"]
    );
    assert_eq!(
        run(f(&["spell"], &["apg"], &[]), "fireball"),
        Vec::<String>::new()
    );
    assert_eq!(
        run(f(&["spell"], &["apg"], &[]), "wall"),
        vec!["spell:wall"]
    );
}

#[test]
fn empty_query_behaviors() {
    let idx = build(corpus());
    assert!(idx.search(&q("")).is_empty());
    assert!(idx.search(&q("  -- ")).is_empty());
    let r = idx.search(&Query {
        filters: Filters {
            kinds: vec!["spell".into()],
            ..Default::default()
        },
        limit: 2,
        ..Default::default()
    });
    assert_eq!(
        r.iter().map(|h| h.title.as_str()).collect::<Vec<_>>(),
        vec!["Delayed Blast Fireball", "Fireball"]
    );
    assert!(r[0].snippet.matches.is_empty());
}

#[test]
fn limit_applies() {
    let idx = build(corpus());
    let hits = idx.search(&Query {
        text: "fire".into(),
        limit: 2,
        ..Default::default()
    });
    assert_eq!(hits.len(), 2);
}

#[test]
fn determinism_and_tiebreak() {
    let docs = corpus();
    let a = build(docs.clone()).to_bytes();
    let mut rev = docs.clone();
    rev.reverse();
    assert_eq!(a, build(rev).to_bytes());
    let mut rot = docs;
    rot.rotate_left(3);
    assert_eq!(a, build(rot).to_bytes());

    // Identical scores: break by title, then id.
    let idx = build(vec![
        doc("3", "Beta", "x", "s", "", &[]),
        doc("2", "Alpha", "x", "s", "", &[]),
        doc("1", "Alpha", "x", "s", "", &[]),
    ]);
    let hits = idx.search(&q("alpha beta gamma delta").clone());
    assert!(hits.is_empty());
    let idx = build(vec![
        doc("3", "Beta", "x", "s", "common", &[]),
        doc("2", "Alpha", "x", "s", "common", &[]),
        doc("1", "Alpha", "x", "s", "common", &[]),
    ]);
    assert_eq!(ids(&idx, &q("common")), vec!["1", "2", "3"]);
}

#[test]
fn round_trip() {
    let idx = build(corpus());
    let bytes = idx.to_bytes();
    let back = SearchIndex::from_bytes(&bytes).unwrap();
    assert_eq!(back, idx);
    assert_eq!(back.to_bytes(), bytes);
    assert_eq!(back.doc_count(), 8);
    assert_eq!(ids(&back, &q("fireball")), ids(&idx, &q("fireball")));
    let d = back.doc(0).unwrap();
    assert!(!d.id.is_empty() && !d.section.is_empty());
    assert!(back.doc(99).is_none());
}

#[test]
fn corrupt_bytes_never_panic() {
    let bytes = build(corpus()).to_bytes();
    assert!(SearchIndex::from_bytes(&[]).is_err());
    assert_eq!(
        SearchIndex::from_bytes(b"NOPE0000000000000000").unwrap_err(),
        IndexError::BadMagic
    );
    let mut v = bytes.clone();
    v[4] = 99;
    assert!(matches!(
        SearchIndex::from_bytes(&v),
        Err(IndexError::UnsupportedVersion(_))
    ));
    let mut cut = 0;
    while cut < bytes.len() {
        assert!(SearchIndex::from_bytes(&bytes[..cut]).is_err(), "cut {cut}");
        cut += 97;
    }
    // Bit flips: may decode or fail, but must not panic, including when queried.
    for i in (0..bytes.len()).step_by(7) {
        let mut v = bytes.clone();
        v[i] ^= 0xA5;
        if let Ok(idx) = SearchIndex::from_bytes(&v) {
            let _ = idx.search(&q("fire"));
            let _ = idx.facet_values("class");
        }
    }
}

#[test]
fn multibyte_highlight_ranges() {
    let text = "Café — a “crème” dish. The Fireball — très fort — burns. Naïve fireballs rule.";
    let idx = build(vec![
        doc("a", "Fireball — Café", "spell", "core", text, &[]),
        doc("b", "Other", "spell", "core", "nothing", &[]),
    ]);
    let hits = idx.search(&q("cafe fireball"));
    assert_eq!(hits.len(), 1);
    let h = &hits[0];
    let check = |s: &str, ms: &[(usize, usize)]| {
        let mut last = 0;
        for &(a, b) in ms {
            assert!(a >= last && a < b && b <= s.len());
            assert!(s.is_char_boundary(a) && s.is_char_boundary(b));
            last = b;
        }
    };
    check(&h.title, &h.title_matches);
    let t: Vec<&str> = h
        .title_matches
        .iter()
        .map(|&(a, b)| &h.title[a..b])
        .collect();
    assert_eq!(t, vec!["Fireball", "Café"]);
    assert!(h.title_matches[0].0 < h.title_matches[1].0);
    check(&h.snippet.text, &h.snippet.matches);
    let s: Vec<&str> = h
        .snippet
        .matches
        .iter()
        .map(|&(a, b)| &h.snippet.text[a..b])
        .collect();
    assert!(
        s.contains(&"Café") && s.contains(&"Fireball") && s.contains(&"fireballs"),
        "{s:?}"
    );
}

#[test]
fn snippet_window_and_ellipsis() {
    let long = format!(
        "{} needle {}",
        "lorem ipsum ".repeat(60),
        "dolor sit ".repeat(60)
    );
    let idx = build(vec![doc("a", "Title", "k", "s", &long, &[])]);
    let h = &idx.search(&q("needle"))[0];
    let s = &h.snippet;
    assert!(s.text.starts_with('…') && s.text.ends_with('…'));
    assert!(
        s.text.chars().count() >= 160 && s.text.chars().count() <= 230,
        "{}",
        s.text.chars().count()
    );
    assert_eq!(s.matches.len(), 1);
    assert_eq!(&s.text[s.matches[0].0..s.matches[0].1], "needle");
    // Title-only match uses the start of text, no leading ellipsis.
    let plain = long.replace("needle", "pin");
    let idx = build(vec![doc("a", "Needle", "k", "s", &plain, &[])]);
    let h = &idx.search(&q("needle"))[0];
    assert!(h.snippet.text.starts_with("lorem") && h.snippet.matches.is_empty());
    assert_eq!(h.title_matches, vec![(0, 6)]);
}

#[test]
fn stored_text_is_capped_on_char_boundary() {
    let text = "é".repeat(5000);
    let idx = build(vec![doc("a", "T", "k", "s", &text, &[])]);
    assert!(idx.texts.texts[0].len() <= 6 * 1024);
    assert!(idx.texts.texts[0].chars().all(|c| c == 'é'));
}

#[test]
fn facet_listing() {
    let idx = build(corpus());
    assert_eq!(
        idx.facet_fields(),
        vec![
            "class",
            "cr",
            "creature_type",
            "feat_type",
            "school",
            "spell_level"
        ]
    );
    assert_eq!(
        idx.facet_values("class"),
        vec![("sorcerer".to_string(), 1), ("wizard".to_string(), 4)]
    );
    assert_eq!(
        idx.facet_values("school"),
        vec![("evocation".to_string(), 4)]
    );
    assert!(idx.facet_values("nope").is_empty());
}

/// Regression: a rare completion ("firewood", in one doc) must not outrank a title
/// completion whose body has the exact query term many times.
#[test]
fn prefix_rank_is_not_decided_by_rare_completions() {
    let mut b = IndexBuilder::new();
    b.add(doc(
        "item:firewood",
        "Firewood",
        "item",
        "cr",
        "A bundle of dry wood.",
        &[],
    ));
    b.add(doc(
        "spell:fireball",
        "Fireball",
        "spell",
        "cr",
        "Deals fire damage. The fire spreads. Fire everywhere.",
        &[],
    ));
    for i in 0..20 {
        b.add(doc(
            &format!("spell:f{i}"),
            &format!("Spell {i}"),
            "spell",
            "cr",
            "fireball fireball fire",
            &[],
        ));
    }
    let idx = b.build();
    let r = ids(&idx, &q("fire"));
    assert_eq!(&r[..2], &["spell:fireball", "item:firewood"]);
}

// ---- filter counts ----------------------------------------------------------------------

fn counts_corpus() -> SearchIndex {
    build(vec![
        doc(
            "spell:a",
            "Fire Bolt",
            "spell",
            "core",
            "Flame.",
            &[("school", "evocation"), ("spell_level", "1")],
        ),
        doc(
            "spell:b",
            "Fire Wall",
            "spell",
            "apg",
            "Flame.",
            &[("school", "evocation"), ("spell_level", "4")],
        ),
        doc(
            "spell:c",
            "Fire Shield",
            "spell",
            "core",
            "Flame.",
            &[("school", "abjuration"), ("spell_level", "4")],
        ),
        doc(
            "spell:d",
            "Frost Ray",
            "spell",
            "core",
            "Cold.",
            &[("school", "evocation"), ("spell_level", "1")],
        ),
        doc(
            "feat:a",
            "Fire Mastery",
            "feat",
            "core",
            "Flame.",
            &[("feat_type", "general")],
        ),
        doc(
            "feat:b",
            "Fire Mastery",
            "feat",
            "apg",
            "Flame.",
            &[("feat_type", "combat")],
        ),
        doc("rule:a", "Cold Weather", "rule", "core", "Cold.", &[]),
    ])
}

fn mk_filters(kinds: &[&str], sources: &[&str], facets: &[(&str, &str)]) -> Filters {
    Filters {
        kinds: kinds.iter().map(|s| s.to_string()).collect(),
        sources: sources.iter().map(|s| s.to_string()).collect(),
        facets: facets
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
    }
}

fn all(_: &str) -> bool {
    true
}

#[test]
fn counts_for_an_empty_query_are_library_totals() {
    let idx = counts_corpus();
    let c = idx.filter_counts_eligible("", &Filters::default(), all);
    assert_eq!(c.total, 7);
    assert_eq!((c.kind("spell"), c.kind("feat"), c.kind("rule")), (4, 2, 1));
    assert_eq!((c.source("core"), c.source("apg")), (5, 2));
    assert_eq!(c.facet("school", "evocation"), 3);
}

#[test]
fn counts_follow_the_query_text() {
    let idx = counts_corpus();
    let c = idx.filter_counts_eligible("fire", &Filters::default(), all);
    assert_eq!(c.total, 5);
    assert_eq!((c.kind("spell"), c.kind("feat"), c.kind("rule")), (3, 2, 0));
    assert_eq!((c.source("core"), c.source("apg")), (3, 2));
    assert_eq!(c.facet("school", "evocation"), 2);
    assert_eq!(c.facet("school", "abjuration"), 1);
    // A query that matches nothing, or an unknown term, counts nothing.
    assert_eq!(
        idx.filter_counts_eligible("zzzz", &Filters::default(), all),
        FilterCounts::default()
    );
}

#[test]
fn a_kind_filter_narrows_the_others_but_not_its_own_kind_chips() {
    let idx = counts_corpus();
    let f = mk_filters(&["spell"], &[], &[]);
    let c = idx.filter_counts_eligible("fire", &f, all);
    assert_eq!(c.total, 3);
    // Kind chips ignore the kind selection, so alternatives stay visible.
    assert_eq!((c.kind("spell"), c.kind("feat")), (3, 2));
    // Sources and facets apply it.
    assert_eq!((c.source("core"), c.source("apg")), (2, 1));
    assert_eq!(c.facet("spell_level", "4"), 2);
    assert_eq!(c.facet("feat_type", "general"), 0);
}

#[test]
fn a_source_filter_narrows_kinds_and_facets() {
    let idx = counts_corpus();
    let f = mk_filters(&[], &["apg"], &[]);
    let c = idx.filter_counts_eligible("fire", &f, all);
    assert_eq!(c.total, 2);
    assert_eq!((c.kind("spell"), c.kind("feat")), (1, 1));
    // Source chips ignore the source selection.
    assert_eq!((c.source("core"), c.source("apg")), (3, 2));
    assert_eq!(c.facet("school", "evocation"), 1);
}

#[test]
fn a_facet_ignores_its_own_field_and_applies_the_rest() {
    let idx = counts_corpus();
    let f = mk_filters(
        &["spell"],
        &[],
        &[("school", "evocation"), ("spell_level", "4")],
    );
    let c = idx.filter_counts_eligible("fire", &f, all);
    assert_eq!(c.total, 1); // Fire Wall
    // Other schools stay visible for the chosen level, and other levels for the chosen school.
    assert_eq!(c.facet("school", "evocation"), 1);
    assert_eq!(c.facet("school", "abjuration"), 1);
    assert_eq!(c.facet("spell_level", "4"), 1);
    assert_eq!(c.facet("spell_level", "1"), 1);
}

// ---- entries that belong to more than one book ------------------------------------------

#[test]
fn an_entry_in_two_books_matches_a_filter_and_counts_for_either() {
    let mut idx = counts_corpus();
    // `feat:a` is attributed to core and also printed in apg.
    idx.set_extra_sources([("feat:a", "apg")]);
    let by_apg = idx.search(&Query {
        text: "fire mastery".into(),
        filters: mk_filters(&[], &["apg"], &[]),
        limit: 10,
    });
    let mut found: Vec<&str> = by_apg.iter().map(|hit| hit.id.as_str()).collect();
    found.sort_unstable();
    assert_eq!(found, ["feat:a", "feat:b"]);
    // The hit keeps its own source: only the filter and the chips see the second book.
    assert!(
        by_apg
            .iter()
            .any(|h| h.id == "feat:a" && h.source == "core")
    );

    let c = idx.filter_counts_eligible("fire", &Filters::default(), all);
    assert_eq!(c.total, 5);
    assert_eq!((c.source("core"), c.source("apg")), (3, 3));
    // A source filter keeps the chips of every other book, and the entry in both stays in each.
    let c = idx.filter_counts_eligible("fire", &mk_filters(&[], &["apg"], &[]), all);
    assert_eq!(c.total, 3);
    assert_eq!((c.source("core"), c.source("apg")), (3, 3));
    // The library totals count it under both books too.
    let totals = idx.filter_counts_eligible("", &Filters::default(), all);
    assert_eq!((totals.source("core"), totals.source("apg")), (5, 3));
}

#[test]
fn extra_sources_ignore_unknown_ids_repeats_and_the_own_source_and_replace_each_call() {
    let mut idx = counts_corpus();
    idx.set_extra_sources([
        ("feat:a", "apg"),
        ("feat:a", "apg"),
        ("feat:a", "core"),
        ("nope", "apg"),
    ]);
    let c = idx.filter_counts_eligible("", &Filters::default(), all);
    assert_eq!((c.source("core"), c.source("apg")), (5, 3));
    idx.set_extra_sources([]);
    let c = idx.filter_counts_eligible("", &Filters::default(), all);
    assert_eq!((c.source("core"), c.source("apg")), (5, 2));
}

#[test]
fn the_masked_counts_and_search_see_every_member_book() {
    let mut idx = counts_corpus();
    idx.set_extra_sources([("feat:a", "apg")]);
    // The selection allows `feat:a` and `feat:b` only, as with Core off and the entry kept by apg.
    let mask = idx.eligible_mask_ids(|id| matches!(id, "feat:a" | "feat:b"));
    let counts = idx.filter_counts_masked("fire", &mk_filters(&[], &["apg"], &[]), &mask);
    assert_eq!(counts.total, 2);
    assert_eq!((counts.source("core"), counts.source("apg")), (1, 2));
    let found = idx.search_masked(
        &Query {
            text: "fire mastery".into(),
            filters: mk_filters(&[], &["apg"], &[]),
            limit: 10,
        },
        &mask,
    );
    assert_eq!(found.len(), 2);
}

#[test]
fn an_entry_kept_by_its_second_book_still_counts_when_the_first_is_inactive() {
    let mut idx = counts_corpus();
    idx.set_extra_sources([("feat:a", "apg")]);
    // The caller's eligibility (any member book active) keeps `feat:a` with only apg on.
    let apg_only = |id: &str| matches!(id, "spell:b" | "feat:b" | "feat:a");
    let c = idx.filter_counts_eligible("fire", &mk_filters(&[], &["apg"], &[]), apg_only);
    assert_eq!(c.total, 3);
    assert_eq!((c.source("core"), c.source("apg")), (1, 3));
}

#[test]
fn an_active_book_change_changes_every_count() {
    let idx = counts_corpus();
    let core_only = |id: &str| !matches!(id, "spell:b" | "feat:b");
    let c = idx.filter_counts_eligible("fire", &Filters::default(), core_only);
    assert_eq!(c.total, 3);
    assert_eq!((c.kind("spell"), c.kind("feat")), (2, 1));
    assert_eq!((c.source("core"), c.source("apg")), (3, 0));
    assert_eq!(c.facet("school", "evocation"), 1);
    assert_eq!(c.facet("feat_type", "combat"), 0);
}

#[test]
fn every_count_equals_the_result_size_of_choosing_that_chip() {
    let idx = counts_corpus();
    let size = |text: &str, f: &Filters| {
        let mut query = q(text);
        query.filters = f.clone();
        query.limit = usize::MAX;
        idx.search(&query).len() as u32
    };
    for text in ["", "fire", "cold", "mastery"] {
        for base in [
            mk_filters(&[], &[], &[]),
            mk_filters(&["spell"], &[], &[]),
            mk_filters(&[], &["core"], &[]),
            mk_filters(&["spell"], &[], &[("school", "evocation")]),
        ] {
            let c = idx.filter_counts_eligible(text, &base, all);
            if !base.is_empty() || !text.is_empty() {
                assert_eq!(c.total, size(text, &base), "{text:?} {base:?}");
            }
            for kind in ["spell", "feat", "rule"] {
                let mut f = base.clone();
                f.kinds = vec![kind.into()];
                if !f.is_empty() {
                    assert_eq!(
                        c.kind(kind),
                        size(text, &f),
                        "kind {kind} {text:?} {base:?}"
                    );
                }
            }
            for source in ["core", "apg"] {
                let mut f = base.clone();
                f.sources = vec![source.into()];
                assert_eq!(
                    c.source(source),
                    size(text, &f),
                    "source {source} {text:?} {base:?}"
                );
            }
            for (field, value) in [
                ("school", "evocation"),
                ("school", "abjuration"),
                ("spell_level", "4"),
            ] {
                let mut f = base.clone();
                f.facets.retain(|(k, _)| k != field);
                f.facets.push((field.into(), value.into()));
                assert_eq!(
                    c.facet(field, value),
                    size(text, &f),
                    "{field}={value} {text:?} {base:?}"
                );
            }
        }
    }
}

#[test]
fn top_k_matches_full_ranking_and_summaries() {
    let mut docs = Vec::new();
    for i in 0..120 {
        docs.push(doc(
            &format!("rule:item-{i:03}"),
            "Shared Word",
            if i % 3 == 0 { "rule" } else { "spell" },
            if i % 2 == 0 { "core" } else { "apg" },
            "A common token appears here.",
            &[("school", "evocation")],
        ));
    }
    let idx = build(docs);
    let full = idx.search(&Query {
        text: "common".into(),
        limit: usize::MAX,
        ..Default::default()
    });
    assert_eq!(full.len(), 120);
    for limit in [1, 2, 7, 50, 119] {
        let hits = idx.search(&Query {
            text: "common".into(),
            limit,
            ..Default::default()
        });
        let got: Vec<(String, u32)> = hits
            .iter()
            .map(|hit| (hit.id.clone(), hit.score.to_bits()))
            .collect();
        let expected: Vec<(String, u32)> = full
            .iter()
            .take(limit)
            .map(|hit| (hit.id.clone(), hit.score.to_bits()))
            .collect();
        assert_eq!(got, expected, "limit {limit}");
    }

    let summaries = idx.search_summaries(&Query {
        text: "common".into(),
        limit: 50,
        ..Default::default()
    });
    let summary_ids: Vec<&str> = summaries.iter().map(|hit| hit.id.as_str()).collect();
    let full_ids: Vec<&str> = full.iter().take(50).map(|hit| hit.id.as_str()).collect();
    assert_eq!(summary_ids, full_ids);

    let highlighter = idx.highlighter(&Query {
        text: "common".into(),
        ..Default::default()
    });
    let mut hit = summaries[0].clone().into_hit();
    idx.materialize_hit(&highlighter, &mut hit);
    assert_eq!(hit.snippet, full[0].snippet);
    assert_eq!(hit.title_matches, full[0].title_matches);
}

#[test]
fn every_count_equals_the_result_size_with_several_active_facets() {
    let idx = counts_corpus();
    let size = |text: &str, f: &Filters| {
        let mut query = q(text);
        query.filters = f.clone();
        query.limit = usize::MAX;
        idx.search(&query).len() as u32
    };
    let base = mk_filters(
        &["spell"],
        &["core"],
        &[("school", "evocation"), ("spell_level", "4")],
    );
    let c = idx.filter_counts_eligible("fire", &base, all);
    assert_eq!(c.total, size("fire", &base));
    // Each chip ignores only its own group.
    for (field, value) in [
        ("school", "evocation"),
        ("school", "abjuration"),
        ("spell_level", "1"),
        ("spell_level", "4"),
    ] {
        let mut f = base.clone();
        f.facets.retain(|(k, _)| k != field);
        f.facets.push((field.into(), value.into()));
        assert_eq!(c.facet(field, value), size("fire", &f), "{field}={value}");
    }
    let mut f = base.clone();
    f.kinds = vec!["feat".into()];
    assert_eq!(c.kind("feat"), size("fire", &f));
    let mut f = base.clone();
    f.sources = vec!["apg".into()];
    assert_eq!(c.source("apg"), size("fire", &f));
}

#[test]
fn duplicate_facet_pairs_count_once() {
    let idx = build(vec![doc(
        "spell:a",
        "Fire Bolt",
        "spell",
        "core",
        "Flame.",
        &[
            ("school", "evocation"),
            ("school", "evocation"),
            ("spell_level", "1"),
        ],
    )]);
    let c = idx.filter_counts_eligible("", &Filters::default(), all);
    assert_eq!(c.facet("school", "evocation"), 1);
    assert_eq!(c.facet("spell_level", "1"), 1);
    assert_eq!(
        idx.facet_values("school"),
        vec![("evocation".to_string(), 1)]
    );
}

#[test]
fn filter_comparisons_stay_case_insensitive() {
    let idx = counts_corpus();
    let mixed = mk_filters(&["SPELL"], &["CORE"], &[("SCHOOL", "EVOCATION")]);
    let lower = mk_filters(&["spell"], &["core"], &[("school", "evocation")]);
    assert_eq!(
        idx.filter_counts_eligible("fire", &mixed, all),
        idx.filter_counts_eligible("fire", &lower, all)
    );
    assert_eq!(
        idx.search(&{
            let mut query = q("fire");
            query.filters = mixed;
            query.limit = usize::MAX;
            query
        })
        .len(),
        idx.search(&{
            let mut query = q("fire");
            query.filters = lower;
            query.limit = usize::MAX;
            query
        })
        .len()
    );
}

#[test]
fn facet_lists_match_the_documents_after_a_round_trip() {
    let idx = counts_corpus();
    let decoded = SearchIndex::from_bytes(&idx.to_bytes()).expect("round trip");
    for field in ["school", "spell_level", "feat_type"] {
        assert_eq!(decoded.facet_values(field), idx.facet_values(field));
        assert_eq!(
            decoded.facet_values_eligible(field, |id| id.starts_with("spell:")),
            idx.facet_values_eligible(field, |id| id.starts_with("spell:"))
        );
    }
    assert_eq!(
        decoded.filter_counts_eligible("fire", &Filters::default(), all),
        idx.filter_counts_eligible("fire", &Filters::default(), all)
    );
}

// --- Source rank tie-break ---------------------------------------------------------------------

/// Identical spells reprinted in three sources. The ids sort opposite to the priority, so the
/// id tie-break alone would put the reprints in the wrong order.
fn reprints() -> SearchIndex {
    build(vec![
        doc(
            "a-adventure",
            "Fireball",
            "spell",
            "ap",
            "A burst of flame.",
            &[],
        ),
        doc(
            "b-companion",
            "Fireball",
            "spell",
            "pc",
            "A burst of flame.",
            &[],
        ),
        doc(
            "c-rulebook",
            "Fireball",
            "spell",
            "core",
            "A burst of flame.",
            &[],
        ),
        doc(
            "d-unlisted",
            "Fireball",
            "spell",
            "other",
            "A burst of flame.",
            &[],
        ),
    ])
}

const RANKS: [(&str, u8); 3] = [("core", 0), ("pc", 1), ("ap", 3)];

#[test]
fn equal_scores_order_by_source_rank_before_title_and_id() {
    let mut idx = reprints();
    assert_eq!(
        ids(&idx, &q("fireball")),
        ["a-adventure", "b-companion", "c-rulebook", "d-unlisted"],
        "without ranks the order is title then id"
    );
    idx.set_source_ranks(RANKS);
    let hits = idx.search(&q("fireball"));
    assert!(hits.windows(2).all(|w| w[0].score == w[1].score));
    assert_eq!(
        hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
        ["c-rulebook", "b-companion", "a-adventure", "d-unlisted"]
    );
}

#[test]
fn a_better_score_wins_over_a_better_source_rank() {
    // The rulebook mentions the term once in a long body; the adventure titles it exactly.
    let mut idx = build(vec![
        doc(
            "rb",
            "Gear",
            "item",
            "core",
            "Long body about many things, and a single mention of lantern somewhere.",
            &[],
        ),
        doc("ap", "Lantern", "item", "ap", "Short.", &[]),
    ]);
    idx.set_source_ranks([("core", 0), ("ap", 3)]);
    let hits = idx.search(&q("lantern"));
    assert!(hits[0].score > hits[1].score);
    assert_eq!(hits[0].id, "ap");
    assert_eq!(hits[1].id, "rb");
}

#[test]
fn an_unknown_or_missing_rank_sorts_as_neutral_and_last() {
    let mut idx = reprints();
    // `other` is absent from the map; `pc` carries the neutral rank explicitly.
    idx.set_source_ranks([("core", 0), ("pc", NEUTRAL_SOURCE_RANK), ("ap", 3)]);
    assert_eq!(
        ids(&idx, &q("fireball")),
        ["c-rulebook", "a-adventure", "b-companion", "d-unlisted"]
    );
}

#[test]
fn source_ranks_apply_to_summaries_and_to_bounded_results() {
    let mut idx = reprints();
    idx.set_source_ranks(RANKS);
    let full: Vec<String> = idx
        .search(&q("fireball"))
        .into_iter()
        .map(|h| h.id)
        .collect();
    let summaries: Vec<String> = idx
        .search_summaries(&q("fireball"))
        .into_iter()
        .map(|h| h.id)
        .collect();
    assert_eq!(summaries, full);
    let mut limited = q("fireball");
    limited.limit = 2;
    assert_eq!(ids(&idx, &limited), full[..2]);
}

#[test]
fn ranked_order_is_deterministic_and_does_not_change_the_serialized_index() {
    let plain = reprints();
    let mut ranked = reprints();
    ranked.set_source_ranks(RANKS);
    assert_eq!(plain.to_bytes(), ranked.to_bytes());
    let again = {
        let mut idx = reprints();
        idx.set_source_ranks(RANKS.iter().rev().copied());
        idx
    };
    assert_eq!(ids(&ranked, &q("fireball")), ids(&again, &q("fireball")));
    // A decoded index starts with no ranks and ranks the same way once they are set again.
    let mut decoded = SearchIndex::from_bytes(&plain.to_bytes()).expect("round trip");
    assert_eq!(ids(&decoded, &q("fireball")), ids(&plain, &q("fireball")));
    decoded.set_source_ranks(RANKS);
    assert_eq!(ids(&decoded, &q("fireball")), ids(&ranked, &q("fireball")));
}

// --- Printing recency tie-break ----------------------------------------------------------------

/// The reprints, all of one product line, so only the recency can order them. Ids and titles
/// sort opposite to recency.
fn dated_reprints() -> SearchIndex {
    let mut idx = reprints();
    idx.set_source_ranks([("core", 0), ("pc", 0), ("ap", 0), ("other", 0)]);
    idx
}

#[test]
fn equal_scores_and_ranks_order_newest_printing_first_and_undated_last() {
    let mut idx = dated_reprints();
    assert_eq!(
        ids(&idx, &q("fireball")),
        ["a-adventure", "b-companion", "c-rulebook", "d-unlisted"],
        "without recency the order is title then id"
    );
    idx.set_printing_recency([("a-adventure", 2), ("b-companion", 7), ("c-rulebook", 5)]);
    assert_eq!(
        ids(&idx, &q("fireball")),
        ["b-companion", "c-rulebook", "a-adventure", "d-unlisted"],
        "d-unlisted is not listed, so it is undated and last"
    );
    // An explicit 0 is undated too, and equal recency falls back to title then id.
    idx.set_printing_recency([
        ("a-adventure", 4),
        ("b-companion", 4),
        ("c-rulebook", 0),
        ("d-unlisted", 0),
        ("no-such-doc", 99),
    ]);
    assert_eq!(
        ids(&idx, &q("fireball")),
        ["a-adventure", "b-companion", "c-rulebook", "d-unlisted"]
    );
}

#[test]
fn a_search_with_filters_and_no_text_orders_equal_titles_like_ranked_results() {
    let mut idx = reprints();
    let filtered = || Query {
        filters: mk_filters(&["spell"], &[], &[]),
        ..Default::default()
    };
    let by_text = |idx: &SearchIndex| ids(idx, &q("fireball"));
    let by_filter = |idx: &SearchIndex| ids(idx, &filtered());
    assert_eq!(by_filter(&idx), by_text(&idx), "no keys: title, then id");
    idx.set_source_ranks(RANKS);
    assert_eq!(by_filter(&idx), by_text(&idx), "product line first");
    idx.set_source_ranks([("core", 0), ("pc", 0), ("ap", 0), ("other", 0)]);
    idx.set_printing_recency([("a-adventure", 2), ("b-companion", 7), ("c-rulebook", 5)]);
    assert_eq!(by_filter(&idx), by_text(&idx), "then the newest printing");
    assert_eq!(by_filter(&idx)[0], "b-companion");
}

#[test]
fn source_rank_wins_over_printing_recency() {
    let mut idx = reprints();
    idx.set_source_ranks(RANKS);
    idx.set_printing_recency([("a-adventure", 9), ("b-companion", 5), ("c-rulebook", 1)]);
    assert_eq!(
        ids(&idx, &q("fireball")),
        ["c-rulebook", "b-companion", "a-adventure", "d-unlisted"]
    );
}

#[test]
fn a_better_score_wins_over_a_newer_printing() {
    let mut idx = build(vec![
        doc("old", "Lantern", "item", "core", "Short.", &[]),
        doc(
            "new",
            "Gear",
            "item",
            "core",
            "Long body about many things, and a single mention of lantern somewhere.",
            &[],
        ),
    ]);
    idx.set_printing_recency([("old", 1), ("new", 9)]);
    let hits = idx.search(&q("lantern"));
    assert!(hits[0].score > hits[1].score);
    assert_eq!(hits[0].id, "old");
}

#[test]
fn printing_recency_applies_to_summaries_and_to_bounded_results() {
    let mut idx = dated_reprints();
    idx.set_printing_recency([("a-adventure", 2), ("b-companion", 7), ("c-rulebook", 5)]);
    let full = ids(&idx, &q("fireball"));
    let summaries: Vec<String> = idx
        .search_summaries(&q("fireball"))
        .into_iter()
        .map(|h| h.id)
        .collect();
    assert_eq!(summaries, full);
    let mut limited = q("fireball");
    limited.limit = 2;
    assert_eq!(ids(&idx, &limited), full[..2]);
}

#[test]
fn printing_recency_is_deterministic_and_does_not_change_the_serialized_index() {
    let plain = dated_reprints();
    let pairs = [("a-adventure", 2), ("b-companion", 7), ("c-rulebook", 5)];
    let mut dated = dated_reprints();
    dated.set_printing_recency(pairs);
    assert_eq!(plain.to_bytes(), dated.to_bytes());
    let mut again = dated_reprints();
    again.set_printing_recency(pairs.iter().rev().copied());
    assert_eq!(ids(&dated, &q("fireball")), ids(&again, &q("fireball")));
    // A decoded index starts undated and orders the same way once the recency is set again.
    let mut decoded = SearchIndex::from_bytes(&plain.to_bytes()).expect("round trip");
    decoded.set_source_ranks([("core", 0), ("pc", 0), ("ap", 0), ("other", 0)]);
    assert_eq!(ids(&decoded, &q("fireball")), ids(&plain, &q("fireball")));
    decoded.set_printing_recency(pairs);
    assert_eq!(ids(&decoded, &q("fireball")), ids(&dated, &q("fireball")));
    // Clearing the recency restores the title and id order.
    decoded.set_printing_recency([]);
    assert_eq!(ids(&decoded, &q("fireball")), ids(&plain, &q("fireball")));
}

fn section(id: &str, title: &str, source: &str, chapter: &str, anchor: &str) -> DocInput {
    DocInput {
        id: id.into(),
        title: title.into(),
        kind: BOOK_SECTION_KIND.into(),
        source: source.into(),
        chapter: chapter.into(),
        anchor: anchor.into(),
        ..DocInput::default()
    }
}

#[test]
fn book_sections_carry_their_source_chapter_and_anchor() {
    let idx = build(vec![
        section("book-section/core/1", "Combat", "core", "combat", ""),
        section(
            "book-section/core/1-2",
            "Actions in Combat",
            "core",
            "combat",
            "toc-1-2",
        ),
        doc("ref/rule/combat", "Combat", "rule", "core", "Rules.", &[]),
    ]);
    let accept = |doc: DocView<'_>| doc.kind == BOOK_SECTION_KIND;
    let hits = idx.search_summaries_docs(&q("combat"), accept);
    // The entry and both sections share the title word; each keeps its own target.
    let sections: Vec<_> = hits
        .iter()
        .filter(|hit| hit.kind == BOOK_SECTION_KIND)
        .collect();
    assert_eq!(sections.len(), 2);
    let chapter = idx.doc(sections[0].ord).unwrap();
    assert_eq!(chapter.source, "core");
    assert_eq!(chapter.chapter, "combat");
    assert!(sections.iter().any(|hit| {
        idx.doc(hit.ord)
            .is_some_and(|doc| doc.title == "Actions in Combat" && doc.anchor == "toc-1-2")
    }));
}

#[test]
fn an_inactive_book_contributes_no_section_results() {
    let idx = build(vec![
        section("book-section/core/1", "Combat", "core", "combat", ""),
        section("book-section/apg/1", "Combat", "apg", "combat", ""),
    ]);
    let active = |source: &str| source == "core";
    let hits = idx.search_summaries_docs(&q("combat"), |doc| {
        doc.kind == BOOK_SECTION_KIND && active(doc.source)
    });
    assert_eq!(
        hits.iter()
            .map(|hit| hit.source.as_str())
            .collect::<Vec<_>>(),
        ["core"]
    );
}

#[test]
fn book_section_bytes_are_identical_whatever_the_insertion_order() {
    let docs = vec![
        section("book-section/core/1", "Combat", "core", "combat", ""),
        section(
            "book-section/core/1-2",
            "Actions in Combat",
            "core",
            "combat",
            "toc-1-2",
        ),
    ];
    let mut reversed = docs.clone();
    reversed.reverse();
    assert_eq!(build(docs).to_bytes(), build(reversed).to_bytes());
}

// ---- Did you mean ----

fn suggest(idx: &SearchIndex, text: &str) -> Option<String> {
    idx.suggest_correction(text, &Filters::default(), |_| true)
        .map(|c| c.text)
}

#[test]
fn a_misspelled_title_word_is_corrected_and_the_correction_finds_the_entry() {
    let idx = build(corpus());
    assert!(
        ids(&idx, &q("magic missle")).is_empty()
            || !ids(&idx, &q("magic missle")).contains(&"spell:mm".to_string())
    );
    let fixed = suggest(&idx, "magic missle").expect("a correction");
    assert_eq!(fixed, "magic missile");
    assert_eq!(ids(&idx, &q(&fixed))[0], "spell:mm");
}

#[test]
fn one_misspelled_word_of_several_is_the_only_one_changed() {
    let idx = build(corpus());
    assert_eq!(
        suggest(&idx, "power atack").as_deref(),
        Some("power attack")
    );
    assert_eq!(
        suggest(&idx, "SKILL  Focsu!").as_deref(),
        Some("skill focus")
    );
    // Unfinished typing counts too: the last word is compared with the start of a name.
    assert_eq!(suggest(&idx, "power atac").as_deref(), Some("power attack"));
    let fixed = idx
        .suggest_correction("power atack", &Filters::default(), |_| true)
        .unwrap();
    assert_eq!(fixed.distance, 1);
}

#[test]
fn a_swap_of_neighbours_is_one_edit() {
    let idx = build(corpus());
    let fixed = idx.suggest_correction("firebal wal", &Filters::default(), |_| true);
    // "firebal" is the start of "fireball", so only "wal" (under four letters) would change.
    assert_eq!(fixed, None);
    let fixed = idx
        .suggest_correction("fierball", &Filters::default(), |_| true)
        .expect("a swap");
    assert_eq!((fixed.text.as_str(), fixed.distance), ("fireball", 1));
}

#[test]
fn queries_that_need_no_correction_get_none() {
    let idx = build(corpus());
    for text in [
        "fireball",         // an exact title word with strong results
        "firebal",          // the start of a title word: the reader is still typing
        "fire",             // a prefix
        "magic missile",    // exact
        "delayed fireball", // all words in a title
        "will o wisp",      // punctuation folds away
        "",                 // empty
        "   ",
        "mgc", // very short input is never changed
        "orc",
        "wil",
        "1234", // numbers
        "4d6",
        "magic 1234",
    ] {
        assert_eq!(suggest(&idx, text), None, "{text:?}");
    }
}

#[test]
fn words_that_are_not_close_to_any_name_get_none() {
    let idx = build(corpus());
    assert_eq!(suggest(&idx, "zzzzzz"), None);
    assert_eq!(suggest(&idx, "magic xylophone"), None);
    // More than three words that are not names is not a typo.
    assert_eq!(suggest(&idx, "mgaic msisile pwoer atack"), None);
    // A long query is left alone.
    assert_eq!(suggest(&idx, "a b c d e f g magic missle"), None);
}

#[test]
fn a_query_with_a_strong_result_is_left_alone_even_when_a_word_is_a_typo() {
    let mut docs = corpus();
    // Four entries hold the plural in their text, so "kobolds" is not a weak query.
    for n in 0..4 {
        docs.push(doc(
            &format!("mon:k{n}"),
            &format!("Tribe {n}"),
            "monster",
            "bestiary",
            "Kobolds dwell here.",
            &[],
        ));
    }
    docs.push(doc(
        "mon:kobold",
        "Kobold",
        "monster",
        "bestiary",
        "A small reptile.",
        &[],
    ));
    let idx = build(docs);
    assert!(
        ids(&idx, &q("kobolds")).len() > CORRECTION_MAX_HITS,
        "the fixture needs more hits than the limit"
    );
    assert_eq!(suggest(&idx, "kobolds"), None);

    // One body hit that names no title word is still weak, so it is corrected.
    let mut docs = corpus();
    docs.push(doc(
        "mon:kobold",
        "Kobold",
        "monster",
        "bestiary",
        "A small reptile.",
        &[],
    ));
    docs.push(doc(
        "mon:k",
        "Tribe",
        "monster",
        "bestiary",
        "Kobolds dwell here.",
        &[],
    ));
    let idx = build(docs);
    assert_eq!(ids(&idx, &q("kobolds")), vec!["mon:k"]);
    assert_eq!(suggest(&idx, "kobolds").as_deref(), Some("kobold"));
}

#[test]
fn a_result_that_names_every_word_is_strong() {
    let mut docs = corpus();
    // A typo in the source book: the title itself holds "missle".
    docs.push(doc(
        "spell:typo",
        "Magic Missle",
        "spell",
        "apg",
        "A misspelt page.",
        &[],
    ));
    let idx = build(docs);
    assert_eq!(suggest(&idx, "magic missle"), None);
}

#[test]
fn only_names_in_active_books_are_suggested() {
    let mut docs = corpus();
    docs.push(doc(
        "spell:mistle",
        "Magic Mistle",
        "spell",
        "apg",
        "A different spell.",
        &[],
    ));
    let idx = build(docs);
    let on = |d: DocView<'_>| d.source != "core";
    let off = |d: DocView<'_>| d.source == "core";
    // Everything on: ties on edits and document count go to the earlier term.
    assert_eq!(
        suggest(&idx, "magic missle").as_deref(),
        Some("magic missile")
    );
    // Core off: "missile" only lives there, so the other near name is offered.
    let got = idx.suggest_correction("magic missle", &Filters::default(), on);
    assert_eq!(got.map(|c| c.text).as_deref(), Some("magic mistle"));
    // Apg off: back to the Core spell.
    let got = idx.suggest_correction("magic missle", &Filters::default(), off);
    assert_eq!(got.map(|c| c.text).as_deref(), Some("magic missile"));
    // Nothing allowed: nothing to suggest.
    assert_eq!(
        idx.suggest_correction("magic missle", &Filters::default(), |_| false),
        None
    );

    // The only near name lives in a book that is off.
    let idx = build(corpus());
    let got = idx.suggest_correction("magic missle", &Filters::default(), |d| d.source != "core");
    assert_eq!(got, None);
}

#[test]
fn filters_apply_to_the_corrected_query() {
    let idx = build(corpus());
    let spells = Filters {
        kinds: vec!["spell".into()],
        ..Default::default()
    };
    let feats = Filters {
        kinds: vec!["feat".into()],
        ..Default::default()
    };
    assert_eq!(
        idx.suggest_correction("magic missle", &spells, |_| true)
            .map(|c| c.text)
            .as_deref(),
        Some("magic missile")
    );
    assert_eq!(
        idx.suggest_correction("magic missle", &feats, |_| true),
        None
    );
}

#[test]
fn equally_good_corrections_are_chosen_the_same_way_every_time() {
    let forward = vec![
        doc("a", "Bolt", "spell", "core", "x", &[]),
        doc("b", "Bolo", "item", "core", "x", &[]),
        doc("c", "Bola", "item", "core", "x", &[]),
    ];
    let mut backward = forward.clone();
    backward.reverse();
    let (one, two) = (build(forward), build(backward));
    for idx in [&one, &two, &one.clone()] {
        assert_eq!(suggest(idx, "bolx").as_deref(), Some("bola"));
        assert_eq!(suggest(idx, "bolx"), suggest(idx, "bolx"));
    }
    // A term that more documents carry wins before the alphabet does.
    let idx = build(vec![
        doc("a", "Bolt", "spell", "core", "x", &[]),
        doc("b", "Bolo", "item", "core", "x", &[]),
        doc("c", "Bolo Tie", "item", "core", "x", &[]),
    ]);
    assert_eq!(suggest(&idx, "bolx").as_deref(), Some("bolo"));
}

#[test]
fn a_correction_survives_a_round_trip_and_leaves_the_format_alone() {
    let idx = build(corpus());
    let before = idx.to_bytes();
    let first = suggest(&idx, "magic missle");
    let after = idx.to_bytes();
    assert_eq!(
        before, after,
        "asking for a correction must not change the index"
    );
    let decoded = SearchIndex::from_bytes(&after).unwrap();
    assert_eq!(decoded, idx, "the name-term cache is not part of equality");
    assert_eq!(suggest(&decoded, "magic missle"), first);
    assert_eq!(INDEX_FORMAT_VERSION, 2);
}

#[test]
fn edit_distance_counts_a_swap_as_one() {
    use crate::correct::edit_distance_for_tests as d;
    assert_eq!(d("missile", "missile", 2), Some(0));
    assert_eq!(d("missle", "missile", 2), Some(1));
    assert_eq!(d("fierball", "fireball", 2), Some(1));
    assert_eq!(d("fierbal", "fireball", 2), Some(2));
    assert_eq!(d("abcd", "wxyz", 2), None);
    assert_eq!(d("abcd", "abcdefg", 2), None);
}

#[test]
fn a_rare_prefix_is_a_word_however_many_names_sort_before_it() {
    // "zebr" starts "zebrzz", an allowed name. Seventy names from a turned-off book start with
    // it too and sort first, so a probe that stopped early would call "zebr" a typo and offer
    // the near name "zebu" instead.
    let mut docs = vec![
        doc("a", "Magic Zebu", "spell", "core", "x", &[]),
        doc("b", "Zebrzz Mark", "item", "core", "x", &[]),
    ];
    for n in 0..70 {
        docs.push(doc(
            &format!("off{n}"),
            &format!("Zebra{n:03}"),
            "item",
            "off",
            "x",
            &[],
        ));
    }
    let idx = build(docs);
    let on = |d: DocView<'_>| d.source != "off";
    assert_eq!(
        idx.suggest_correction("magic zebr", &Filters::default(), on),
        None
    );
    // The same query does get the near name once the prefix has no allowed carrier.
    let got = idx.suggest_correction("magic zebr", &Filters::default(), |d| d.id != "b" && on(d));
    assert_eq!(got.map(|c| c.text).as_deref(), Some("magic zebu"));
}

#[test]
fn a_number_in_any_script_is_never_rewritten() {
    let idx = build(vec![
        doc("a", "Magic 1234", "spell", "core", "x", &[]),
        doc("b", "Magic ١٢٣٥", "spell", "core", "x", &[]),
    ]);
    // Each is one edit from a name, and neither is a word.
    assert_eq!(suggest(&idx, "magic 1235"), None);
    assert_eq!(suggest(&idx, "magic ١٢٣٤"), None);
}

// ---- prefix expansion cap ---------------------------------------------------------------

/// 600 rare completions of "ma" that sort before "magic", plus one common completion. The cap
/// is 512 terms, so alphabetical truncation would drop "magic".
fn crowded_prefix() -> SearchIndex {
    let mut b = IndexBuilder::new();
    for i in 0..600 {
        // "maa" + letters: every term occurs in exactly one document.
        let term = format!("maa{}", letters(i));
        b.add(doc(
            &format!("rare:{i}"),
            &format!("Rare {i}"),
            "item",
            "core",
            &term,
            &[],
        ));
    }
    for i in 0..30 {
        b.add(doc(
            &format!("common:{i}"),
            &format!("Common {i}"),
            "spell",
            "core",
            "magic",
            &[],
        ));
    }
    b.build()
}

/// A base-26 spelling of `n` with letters only, three letters wide, so terms stay distinct.
fn letters(n: usize) -> String {
    let digit = |d: usize| (b'a' + (d % 26) as u8) as char;
    [n / 676, n / 26, n].into_iter().map(digit).collect()
}

#[test]
fn a_short_prefix_keeps_the_most_frequent_completions_not_the_first_alphabetically() {
    let idx = crowded_prefix();
    let hits = idx.search_summaries(&Query {
        text: "ma".into(),
        limit: usize::MAX,
        ..Default::default()
    });
    let common = hits.iter().filter(|h| h.id.starts_with("common:")).count();
    assert_eq!(
        common, 30,
        "the common completion \"magic\" was dropped by the cap"
    );
    // 600 rare terms compete for the 511 places that "magic" leaves. Which ones win is fixed:
    // equal frequencies keep the alphabetically earlier term.
    let rare = hits.iter().filter(|h| h.id.starts_with("rare:")).count();
    assert_eq!(rare, 511);
    assert!(hits.iter().any(|h| h.id == "rare:0"));
    assert!(hits.iter().all(|h| h.id != "rare:599"));
}

#[test]
fn prefix_truncation_is_deterministic_and_leaves_exact_terms_alone() {
    let idx = crowded_prefix();
    let query = Query {
        text: "ma".into(),
        limit: usize::MAX,
        ..Default::default()
    };
    let a = idx.search_summaries(&query);
    let b = idx.search_summaries(&query);
    assert_eq!(a, b);
    // An exact term is not part of the cap: "magic" finds all 30 whatever else "ma" completes.
    assert_eq!(
        idx.search_summaries(&Query {
            text: "magic".into(),
            limit: usize::MAX,
            ..Default::default()
        })
        .len(),
        30
    );
}

// ---- eligibility masks ------------------------------------------------------------------

#[test]
fn a_mask_is_computed_once_and_reused_by_every_pass() {
    let idx = build(vec![
        doc("spell:a", "Fire Bolt", "spell", "core", "flame", &[]),
        doc("spell:b", "Fire Wall", "spell", "apg", "flame", &[]),
        doc("spell:c", "Fire Shield", "spell", "core", "flame", &[]),
        doc("feat:a", "Fire Mastery", "feat", "core", "flame", &[]),
    ]);
    let evaluations = std::cell::Cell::new(0usize);
    let active = ["spell:a", "spell:c", "feat:a"];
    let mask = idx.eligible_mask_ids(|id| {
        evaluations.set(evaluations.get() + 1);
        active.contains(&id)
    });
    // One evaluation per document, however many searches follow.
    assert_eq!(evaluations.get(), idx.doc_count());
    assert_eq!(mask.allowed_count(), 3);
    assert!(!mask.allows_all());
    for text in ["f", "fi", "fir", "fire", "fire b", "fire bolt", "flame"] {
        let query = q(text);
        let by_mask = idx.search_summaries_masked(&query, &mask);
        let by_closure = idx.search_summaries_eligible(&query, |id| active.contains(&id));
        assert_eq!(by_mask, by_closure, "{text}");
        assert_eq!(
            idx.search_masked(&query, &mask)
                .into_iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            idx.search_eligible(&query, |id| active.contains(&id))
                .into_iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            "{text}"
        );
        assert_eq!(
            idx.filter_counts_masked(text, &Filters::default(), &mask),
            idx.filter_counts_eligible(text, &Filters::default(), |id| active.contains(&id)),
            "{text}"
        );
    }
}

#[test]
fn the_masked_calls_never_evaluate_the_selection_again() {
    let idx = build(vec![
        doc("spell:a", "Fire Bolt", "spell", "core", "flame", &[]),
        doc("spell:b", "Fire Wall", "spell", "apg", "flame", &[]),
    ]);
    let evaluations = std::cell::Cell::new(0usize);
    let mask = idx.eligible_mask_ids(|id| {
        evaluations.set(evaluations.get() + 1);
        id == "spell:a"
    });
    let built = evaluations.get();
    assert_eq!(built, 2);
    let by_closure = idx.suggest_correction("firr", &Filters::default(), |doc| doc.id == "spell:a");
    assert_eq!(by_closure.as_ref().map(|c| c.text.as_str()), Some("fire"));
    for _ in 0..10 {
        idx.search_summaries_masked(&q("fire"), &mask);
        idx.search_masked(&q("fire"), &mask);
        idx.filter_counts_masked("fire", &Filters::default(), &mask);
        let by_mask = idx.suggest_correction_masked("firr", &Filters::default(), &mask);
        assert_eq!(by_mask, by_closure);
    }
    assert_eq!(evaluations.get(), built);
}

#[test]
fn narrowing_a_mask_tests_only_the_documents_it_allows() {
    let idx = build(vec![
        doc("spell:a", "Fire Bolt", "spell", "core", "x", &[]),
        doc("spell:b", "Fire Wall", "spell", "apg", "x", &[]),
        doc("feat:a", "Fire Mastery", "feat", "core", "x", &[]),
    ]);
    let mask = idx.eligible_mask(|doc| doc.source == "core");
    let tested = std::cell::RefCell::new(Vec::new());
    let narrowed = idx.narrow_mask(&mask, |doc| {
        tested.borrow_mut().push(doc.id.to_string());
        doc.kind == "spell"
    });
    let mut tested = tested.into_inner();
    tested.sort();
    assert_eq!(tested, ["feat:a", "spell:a"]);
    assert_eq!(narrowed.allowed_count(), 1);
    let kept = (0..idx.doc_count())
        .filter(|&ord| narrowed.allows(ord))
        .map(|ord| idx.doc(ord as u32).unwrap().id)
        .collect::<Vec<_>>();
    assert_eq!(kept, ["spell:a"]);
    // A mask from another, smaller index allows nothing beyond its length.
    assert!(!narrowed.allows(99));
}

#[test]
fn an_all_allowing_mask_gives_the_library_totals() {
    let idx = counts_corpus();
    let mask = idx.eligible_mask(|_| true);
    assert!(mask.allows_all());
    assert_eq!(
        idx.filter_counts_masked("", &Filters::default(), &mask),
        idx.filter_counts_eligible("", &Filters::default(), all)
    );
}
