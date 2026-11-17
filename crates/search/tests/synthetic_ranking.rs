//! Ranking golden over a small synthetic corpus checked into the repository.
//!
//! Unlike `canonical_ranking.rs`, this test needs no generated library, so it runs on every
//! `cargo test` and `cargo xtask check`. The corpus is invented text that exercises the ranking
//! rules: exact and prefix titles, aliases, the last word as a prefix, source-rank ties, and
//! stop-word-like queries. A deliberate ranking change refreshes the golden:
//!
//! ```sh
//! CAST_IRON_BLESS=1 cargo test -p cast-iron-search --test synthetic_ranking
//! ```
//!
//! Only the exact value `1` blesses, and blessing refuses to run when `CI` is set. Review the diff
//! of `tests/golden/synthetic-top.txt` before committing it.
//!
//! The index holds the documents of the fixture file and a generated crowd: 600 documents that
//! each carry one rare "zq" term and 12 that share "zqfreq". The 601 completions of "zq" exceed
//! the cap of 512, so the golden pins which completions survive. The golden also repeats every
//! query with the "apg" book switched off.

use cast_iron_search::{DocInput, EligibleMask, Filters, IndexBuilder, Query, SearchIndex};
use std::fmt::Write as _;
use std::path::Path;

const CORPUS: &str = include_str!("fixtures/synthetic-corpus.txt");
const QUERIES: &str = include_str!("fixtures/synthetic-queries.txt");
const GOLDEN_PATH: &str = "tests/golden/synthetic-top.txt";
const TOP: usize = 20;
/// Rare completions of "zq", one document each, and common documents that share one term.
const CROWD: usize = 600;
const COMMON: usize = 12;

fn documents() -> Vec<DocInput> {
    let mut docs = Vec::new();
    let mut current: Option<DocInput> = None;
    for line in CORPUS.lines() {
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            docs.extend(current.take());
            continue;
        }
        let (key, value) = line.split_once(": ").expect("key: value");
        let doc = current.get_or_insert_with(|| DocInput {
            section: "S".into(),
            ..Default::default()
        });
        match key {
            "id" => doc.id = value.into(),
            "title" => doc.title = value.into(),
            "kind" => doc.kind = value.into(),
            "source" => doc.source = value.into(),
            "alias" => doc.aliases.push(value.into()),
            "text" => doc.text = value.into(),
            other => panic!("unknown key {other}"),
        }
    }
    docs.extend(current);
    docs.extend(crowd());
    docs
}

/// A base-26 spelling of `n` with letters only, so every term is distinct and the tokenizer
/// keeps it whole.
fn letters(n: usize) -> String {
    let digit = |d: usize| (b'a' + (d % 26) as u8) as char;
    [n / 676, n / 26, n].into_iter().map(digit).collect()
}

/// More completions of "zq" than the expansion cap allows. The rare ones sort before the common
/// "zqfreq", so alphabetical truncation would lose it.
fn crowd() -> Vec<DocInput> {
    let make = |id: String, title: String, text: String| DocInput {
        id,
        title,
        kind: "item".into(),
        source: "core".into(),
        section: "S".into(),
        text,
        ..Default::default()
    };
    let rare = (0..CROWD).map(|i| {
        make(
            format!("crowd:rare-{i}"),
            format!("Rare {i}"),
            format!("zqa{}", letters(i)),
        )
    });
    let common = (0..COMMON).map(|i| {
        make(
            format!("crowd:common-{i}"),
            format!("Common {i}"),
            "zqfreq".into(),
        )
    });
    rare.chain(common).collect()
}

fn index() -> SearchIndex {
    let mut builder = IndexBuilder::new();
    for doc in documents() {
        builder.add(doc);
    }
    let mut index = builder.build();
    // "core" outranks "apg" on an exact tie, as a rulebook outranks a supplement.
    index.set_source_ranks([("core", 0), ("apg", 1)]);
    index
}

fn queries() -> impl Iterator<Item = &'static str> {
    QUERIES
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// The selection with the "apg" book switched off.
fn without_apg(index: &SearchIndex) -> EligibleMask {
    index.eligible_mask(|doc| doc.source != "apg")
}

fn render(index: &SearchIndex) -> String {
    let mut out = String::from(
        "# Top ids per query over tests/fixtures/synthetic-corpus.txt and the generated crowd.\n\
         # Refresh with CAST_IRON_BLESS=1 cargo test -p cast-iron-search --test synthetic_ranking\n",
    );
    let mask = without_apg(index);
    for (heading, masked) in [("all books", false), ("apg off", true)] {
        writeln!(out, "== {heading}").unwrap();
        for text in queries() {
            writeln!(out, "{text}").unwrap();
            let query = Query {
                text: text.into(),
                limit: TOP,
                ..Default::default()
            };
            let hits = if masked {
                index.search_summaries_masked(&query, &mask)
            } else {
                index.search_summaries(&query)
            };
            for hit in hits {
                writeln!(out, "  {}", hit.id).unwrap();
            }
        }
    }
    out
}

/// Blessing needs the exact value `1` and refuses to run in CI, where a changed ranking must
/// fail instead of rewriting its own expectation.
fn blessing() -> bool {
    if std::env::var("CAST_IRON_BLESS").as_deref() != Ok("1") {
        return false;
    }
    assert!(
        std::env::var_os("CI").is_none(),
        "CAST_IRON_BLESS=1 must not run in CI"
    );
    true
}

#[test]
fn the_synthetic_ranking_matches_the_golden() {
    let got = render(&index());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN_PATH);
    if blessing() {
        std::fs::write(&path, &got).expect("write golden");
        return;
    }
    let want = std::fs::read_to_string(&path).expect("read golden");
    assert_eq!(
        got, want,
        "ranking changed; if the change is intended, refresh the golden with CAST_IRON_BLESS=1"
    );
}

#[test]
fn the_synthetic_corpus_covers_what_the_golden_claims() {
    let index = index();
    assert!(index.doc_count() >= 30, "the corpus shrank");
    let ids = |text: &str| -> Vec<String> {
        index
            .search_summaries(&Query {
                text: text.into(),
                limit: TOP,
                ..Default::default()
            })
            .into_iter()
            .map(|hit| hit.id)
            .collect()
    };
    // An exact title leads.
    assert_eq!(ids("fire")[0], "rule:fire");
    // An alias finds its document.
    assert_eq!(ids("clw")[0], "spell:cure-light-wounds");
    // A misspelled last word finds nothing, and the engine suggests the right spelling.
    assert!(ids("magic missle").is_empty());
    let all = |_: cast_iron_search::DocView<'_>| true;
    let fix = index
        .suggest_correction("magic missle", &Filters::default(), all)
        .expect("a correction for \"magic missle\"");
    assert_eq!((fix.text.as_str(), fix.distance), ("magic missile", 1));
    // An exact tie goes to the lower source rank, ahead of the id order.
    assert_eq!(
        ids("distracting"),
        ["z/distracting-bolt", "a/distracting-bolt"]
    );
}

#[test]
fn a_masked_correction_equals_the_closure_correction() {
    let index = index();
    let mask = without_apg(&index);
    let off = |doc: cast_iron_search::DocView<'_>| doc.source != "apg";
    for text in [
        "magic missle",
        "magic circel",
        "cur light wunds",
        "goblin dgo",
    ] {
        assert_eq!(
            index.suggest_correction_masked(text, &Filters::default(), &mask),
            index.suggest_correction(text, &Filters::default(), off),
            "{text}"
        );
    }
    // A name that only the switched-off book carries is never suggested.
    let circle = |mask: Option<&EligibleMask>| match mask {
        Some(mask) => index.suggest_correction_masked("magic circel", &Filters::default(), mask),
        None => index.suggest_correction("magic circel", &Filters::default(), |_| true),
    };
    assert_eq!(
        circle(None).map(|fix| fix.text),
        Some("magic circle".to_string())
    );
    assert_eq!(circle(Some(&mask)), None);
}

#[test]
fn the_crowded_prefix_keeps_the_common_completion_within_the_cap() {
    let index = index();
    let hits = index.search_summaries(&Query {
        text: "zq".into(),
        limit: usize::MAX,
        ..Default::default()
    });
    let common = hits
        .iter()
        .filter(|hit| hit.id.starts_with("crowd:common-"))
        .count();
    let rare = hits
        .iter()
        .filter(|hit| hit.id.starts_with("crowd:rare-"))
        .count();
    // 601 completions compete for 512 places. The common one stays; 511 rare ones fill the rest.
    assert_eq!((common, rare), (COMMON, 511));
    assert!(hits.iter().any(|hit| hit.id == "crowd:rare-0"));
    assert!(hits.iter().all(|hit| hit.id != "crowd:rare-599"));
}
