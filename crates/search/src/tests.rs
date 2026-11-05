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
