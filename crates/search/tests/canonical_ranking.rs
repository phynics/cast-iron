//! Ranking golden for the canonical library.
//!
//! `golden/canonical-top50.txt` records the top-50 ids of 50 short queries (the list in
//! `tests/e2e/measure-library.mjs`) against the pinned corpus. It pins ranking, not snippet text.
//! A corpus pin changes the library and so the ranking; the pin PR regenerates the golden with
//! `cargo xtask search-golden` and reads the diff for ranking regressions.
//!
//! * `the_golden_is_for_the_pinned_corpus` always runs: it compares the corpus commit that the
//!   golden records with `sources/corpus.toml`, so a pin PR that skips the refresh fails
//!   `Rust checks`.
//! * `canonical_top50_ranking_matches_the_golden` compares the rankings whenever a generated
//!   search index exists (`cargo xtask content`, or `CAST_IRON_SEARCH_INDEX`).
//! * `CAST_IRON_REQUIRE_CANONICAL=1` turns a missing index into a failure instead of a skip. The
//!   `Content compiler` job of CI sets it after `cargo xtask content`, so the comparison runs there.
//!
//! `synthetic_ranking.rs` pins the ranking rules on a checked-in corpus and needs neither.
//!
//! Format: `#` lines are comments (`# content <version>` and `# corpus <commit>` are read);
//! a `query: <text>` line starts a block; every other non-empty line is a result id. Ids are
//! never read as queries, whatever their prefix (`ref/`, `book-section/`, ...).

use cast_iron_search::{Query, SearchIndex};
use std::path::{Path, PathBuf};

const GOLDEN: &str = include_str!("golden/canonical-top50.txt");
const SAMPLE_QUERIES: usize = 50;
const TOP: usize = 50;
const REGENERATE: &str = "run `cargo xtask content`, then `cargo xtask search-golden`, \
     review the diff of crates/search/tests/golden/canonical-top50.txt for ranking regressions, \
     and commit it";

fn manifest_version(manifest: &str) -> Option<String> {
    let start = manifest.find("\"content_version\"")?;
    let rest = &manifest[start..];
    let value = rest.split('"').nth(3)?;
    Some(value.to_string())
}

/// The index of the canonical library, or `None` when it is absent and the run does not require it.
/// A required index that is missing panics, so a CI step that forgot to build the library fails
/// instead of passing without comparing anything.
fn canonical_index(test: &str) -> Option<(PathBuf, String)> {
    if let Some(found) = index_path() {
        return Some(found);
    }
    let required = std::env::var("CAST_IRON_REQUIRE_CANONICAL").is_ok_and(|value| value == "1");
    assert!(
        !required,
        "{test}: CAST_IRON_REQUIRE_CANONICAL=1 but no generated canonical search index exists; \
         run `cargo xtask content` first"
    );
    eprintln!("skipping: no generated canonical search index");
    None
}

fn index_path() -> Option<(PathBuf, String)> {
    if let Ok(path) = std::env::var("CAST_IRON_SEARCH_INDEX") {
        return Some((PathBuf::from(path), String::new()));
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = std::fs::read_to_string(root.join("generated/content/manifest.json")).ok()?;
    let version = manifest_version(&manifest)?;
    let path = root
        .join("generated/content")
        .join(&version)
        .join("search.bin");
    path.exists().then_some((path, version))
}

/// The value of a `# <key> <value>` comment line.
fn header(text: &str, key: &str) -> Option<String> {
    let prefix = format!("# {key} ");
    text.lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| value.trim().to_string())
}

fn golden_blocks(text: &str) -> Vec<(String, Vec<String>)> {
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(query) = line.strip_prefix("query: ") {
            blocks.push((query.to_string(), Vec::new()));
        } else {
            blocks
                .last_mut()
                .expect("an id before the first `query:` line")
                .1
                .push(line.to_string());
        }
    }
    blocks
}

fn pinned_commit() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sources/corpus.toml");
    let text = std::fs::read_to_string(path).expect("read sources/corpus.toml");
    text.lines()
        .find_map(|line| {
            let value = line
                .strip_prefix("commit")?
                .trim_start()
                .strip_prefix('=')?;
            Some(value.trim().trim_matches('"').to_string())
        })
        .expect("sources/corpus.toml has no commit")
}

#[test]
fn the_golden_parser_reads_every_id_prefix_as_an_id() {
    let text = "# content vabc\nquery: fire\nref/rule/fire\nbook-section/x/y\nref/spell/fire-ball\n\nquery: ref\nbook-section/z\n";
    assert_eq!(
        golden_blocks(text),
        vec![
            (
                "fire".to_string(),
                vec![
                    "ref/rule/fire".to_string(),
                    "book-section/x/y".to_string(),
                    "ref/spell/fire-ball".to_string()
                ]
            ),
            ("ref".to_string(), vec!["book-section/z".to_string()]),
        ]
    );
}

#[test]
fn the_golden_is_for_the_pinned_corpus() {
    let recorded = header(GOLDEN, "corpus").unwrap_or_default();
    let pinned = pinned_commit();
    assert_eq!(
        recorded, pinned,
        "crates/search/tests/golden/canonical-top50.txt was generated for corpus commit {recorded:?}, \
         but sources/corpus.toml pins {pinned}; {REGENERATE}"
    );
    assert_eq!(golden_blocks(GOLDEN).len(), SAMPLE_QUERIES);
}

#[test]
fn canonical_top50_ranking_matches_the_golden() {
    let Some((path, version)) = canonical_index("canonical_top50_ranking_matches_the_golden")
    else {
        return;
    };
    let recorded = header(GOLDEN, "content").unwrap_or_default();
    let bytes = std::fs::read(&path).expect("read search index");
    let index = SearchIndex::from_bytes(&bytes).expect("decode search index");
    let blocks = golden_blocks(GOLDEN);
    assert_eq!(blocks.len(), SAMPLE_QUERIES);
    for (query, expected) in blocks {
        let hits = index.search_summaries(&Query {
            text: query.clone(),
            limit: TOP,
            ..Default::default()
        });
        let got: Vec<String> = hits.into_iter().map(|hit| hit.id).collect();
        assert_eq!(
            got, expected,
            "ranking changed for {query:?} (golden: content {recorded}, index: content {version:?}); \
             if the library or the ranking changed on purpose, {REGENERATE}"
        );
    }
}

/// The e2e test `Equal relevance ranks the Rulebook entry before an Adventure entry` relies on
/// two entries that tie exactly for "Distracting". The tie is a property of the corpus, so pin it
/// here, where a failure is cheap to diagnose. When this fails after a content change, the e2e
/// fixture pair needs replacing with another exact tie across tiers; relevance is not at fault.
#[test]
fn the_distracting_pair_ties_exactly_in_the_canonical_index() {
    let Some((path, _)) =
        canonical_index("the_distracting_pair_ties_exactly_in_the_canonical_index")
    else {
        return;
    };
    let bytes = std::fs::read(&path).expect("read search index");
    let index = SearchIndex::from_bytes(&bytes).expect("decode search index");
    let hits = index.search_summaries(&Query {
        text: "Distracting".into(),
        limit: usize::MAX,
        ..Default::default()
    });
    let score_of = |id: &str| {
        hits.iter()
            .find(|hit| hit.id == id)
            .unwrap_or_else(|| panic!("{id} is not a hit for \"Distracting\" any more"))
            .score
    };
    let (rulebook, adventure) = (
        score_of("ref/rule/rc-distracting-visitor"),
        score_of("ref/feat/distracting-explosion"),
    );
    assert_eq!(
        rulebook.total_cmp(&adventure),
        std::cmp::Ordering::Equal,
        "the scores differ ({rulebook} vs {adventure}); the e2e tie fixture is stale"
    );
}
