//! "Did you mean" on the canonical library. Skips when no generated `search.bin` exists.
//!
//! The timing test is ignored by default because it measures, not checks. Run it in release mode
//! to get the numbers that `docs/decisions.md` records:
//!
//! ```sh
//! cargo test --release -p cast-iron-search --test canonical_correction -- --ignored --nocapture
//! ```

use cast_iron_search::{CORRECTION_MAX_HITS, Filters, Query, SearchIndex};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn index_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CAST_IRON_SEARCH_INDEX") {
        return Some(PathBuf::from(path));
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = std::fs::read_to_string(root.join("generated/content/manifest.json")).ok()?;
    let start = manifest.find("\"content_version\"")?;
    let version = manifest[start..].split('"').nth(3)?.to_string();
    let path = root
        .join("generated/content")
        .join(version)
        .join("search.bin");
    path.exists().then_some(path)
}

fn load() -> Option<SearchIndex> {
    let path = index_path().or_else(|| {
        eprintln!("skipping: no generated canonical search index");
        None
    })?;
    let bytes = std::fs::read(path).expect("read search index");
    Some(SearchIndex::from_bytes(&bytes).expect("decode search index"))
}

fn fix(index: &SearchIndex, text: &str) -> Option<String> {
    index
        .suggest_correction(text, &Filters::default(), |_| true)
        .map(|c| c.text)
}

fn hits(index: &SearchIndex, text: &str) -> Vec<String> {
    index
        .search_summaries(&Query {
            text: text.into(),
            limit: usize::MAX,
            ..Default::default()
        })
        .into_iter()
        .map(|hit| hit.title)
        .collect()
}

/// Typos of the names a reader looks up, with the title that the correction must find.
const TYPOS: &[(&str, &str, &str)] = &[
    ("magic missle", "magic missile", "Magic Missile"),
    ("magic misile", "magic missile", "Magic Missile"),
    ("fireball", "", ""),
    ("firball", "fireball", "Fireball"),
    ("fierball", "fireball", "Fireball"),
    (
        "cure lihgt wounds",
        "cure light wounds",
        "Cure Light Wounds",
    ),
    ("powr attack", "power attack", "Power Attack"),
    ("grappel", "grapple", "Grapple"),
    ("invisiblity", "invisibility", "Invisibility"),
    ("wizzard", "wizard", "Wizard"),
    ("dimention door", "dimension door", "Dimension Door"),
];

#[test]
fn typos_of_well_known_names_are_corrected() {
    let Some(index) = load() else { return };
    for &(typed, expected, title) in TYPOS {
        let got = fix(&index, typed);
        if expected.is_empty() {
            assert_eq!(got, None, "{typed:?} is spelled right");
            continue;
        }
        let got = got.unwrap_or_else(|| panic!("no correction for {typed:?}"));
        assert_eq!(got, expected, "{typed:?}");
        assert!(
            hits(&index, &got).iter().any(|found| found == title),
            "{got:?} does not find {title}"
        );
    }
}

#[test]
fn queries_with_results_are_not_corrected() {
    let Some(index) = load() else { return };
    for text in [
        "fire",
        "dragon",
        "sword",
        "armor class",
        "turn undead",
        "cure light wounds",
        "fireball",
        "magic missile",
        "grapple",
        "wizard",
        "goblin",
        "4",
        "1d6",
        "ac",
        "fly",
        "mgc",
    ] {
        assert_eq!(fix(&index, text), None, "{text:?}");
    }
}

#[test]
fn the_correction_is_the_same_on_every_run() {
    let Some(index) = load() else { return };
    let first: Vec<_> = TYPOS
        .iter()
        .map(|&(typed, ..)| fix(&index, typed))
        .collect();
    let again = SearchIndex::from_bytes(&index.to_bytes()).expect("round trip");
    let second: Vec<_> = TYPOS
        .iter()
        .map(|&(typed, ..)| fix(&again, typed))
        .collect();
    assert_eq!(first, second);
}

fn median(times: &mut [Duration]) -> Duration {
    times.sort();
    times[times.len() / 2]
}

#[test]
#[ignore = "measures time; run with --release"]
fn timing_on_the_canonical_index() {
    let Some(index) = load() else { return };
    let warm = Instant::now();
    let terms = index.warm_corrections();
    eprintln!(
        "name terms: {terms}, docs: {}, first build {:?}",
        index.doc_count(),
        warm.elapsed()
    );
    let mut all = Vec::new();
    for &(typed, expected, _) in TYPOS {
        let mut runs = Vec::new();
        let mut got = None;
        for _ in 0..15 {
            let started = Instant::now();
            got = fix(&index, typed);
            runs.push(started.elapsed());
        }
        let m = median(&mut runs);
        eprintln!(
            "{typed:<20} -> {:<22} median {m:?}",
            got.as_deref().unwrap_or("-")
        );
        if !expected.is_empty() {
            all.push(m);
        }
    }
    // The case that decides whether correction can run per keystroke: no correction exists.
    for typed in ["zzzzzzzz", "qwertyuiop asdfgh"] {
        let mut runs = Vec::new();
        for _ in 0..15 {
            let started = Instant::now();
            assert_eq!(fix(&index, typed), None);
            runs.push(started.elapsed());
        }
        eprintln!(
            "{typed:<20} -> -                      median {:?}",
            median(&mut runs)
        );
    }
    // Ordinary queries cost one search only when the caller skips the call above the limit.
    let broad = Instant::now();
    assert_eq!(fix(&index, "fire"), None);
    eprintln!("fire (strong, unguarded): {:?}", broad.elapsed());
    let worst = all.iter().max().copied().unwrap_or_default();
    eprintln!("worst median over corrected typos: {worst:?}; limit {CORRECTION_MAX_HITS} hits");
    assert!(
        worst < Duration::from_millis(250),
        "a correction took {worst:?}"
    );
}
