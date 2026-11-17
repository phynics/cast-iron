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
//! Review the diff of `tests/golden/synthetic-top.txt` before committing it.

use cast_iron_search::{DocInput, IndexBuilder, Query, SearchIndex};
use std::fmt::Write as _;
use std::path::Path;

const CORPUS: &str = include_str!("fixtures/synthetic-corpus.txt");
const QUERIES: &str = include_str!("fixtures/synthetic-queries.txt");
const GOLDEN_PATH: &str = "tests/golden/synthetic-top.txt";
const TOP: usize = 20;

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
    docs
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

fn render(index: &SearchIndex) -> String {
    let mut out = String::from(
        "# Top ids per query over tests/fixtures/synthetic-corpus.txt. Refresh with\n\
         # CAST_IRON_BLESS=1 cargo test -p cast-iron-search --test synthetic_ranking\n",
    );
    for text in queries() {
        writeln!(out, "{text}").unwrap();
        let hits = index.search_summaries(&Query {
            text: text.into(),
            limit: TOP,
            ..Default::default()
        });
        for hit in hits {
            writeln!(out, "  {}", hit.id).unwrap();
        }
    }
    out
}

#[test]
fn the_synthetic_ranking_matches_the_golden() {
    let got = render(&index());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN_PATH);
    if std::env::var_os("CAST_IRON_BLESS").is_some() {
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
    // A misspelled last word finds nothing.
    assert!(ids("magic missle").is_empty());
    // An exact tie goes to the lower source rank, ahead of the id order.
    assert_eq!(
        ids("distracting"),
        ["z/distracting-bolt", "a/distracting-bolt"]
    );
}
