//! Ranking golden for the canonical library.
//!
//! The golden file records the top-50 ids that the previous engine produced for
//! the query list in `tests/e2e/measure-library.mjs`. It pins ranking, not
//! snippet text, so the display refactor can change materialization without
//! reordering results. When the corpus version differs the test skips: the
//! golden is evidence for the build where it was captured, not for later data.

use cast_iron_search::{Query, SearchIndex};
use std::path::{Path, PathBuf};

const GOLDEN: &str = include_str!("golden/canonical-top50.txt");
const SAMPLE_QUERIES: usize = 50;
const TOP: usize = 50;

fn manifest_version(manifest: &str) -> Option<String> {
    let start = manifest.find("\"content_version\"")?;
    let rest = &manifest[start..];
    let value = rest.split('"').nth(3)?;
    Some(value.to_string())
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

fn golden_blocks() -> Vec<(String, Vec<String>)> {
    let mut blocks = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;
    for line in GOLDEN.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("ref/") {
            current
                .as_mut()
                .expect("an id without a query")
                .1
                .push(line.to_string());
        } else {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            current = Some((line.to_string(), Vec::new()));
        }
    }
    if let Some(block) = current {
        blocks.push(block);
    }
    blocks
}

#[test]
fn canonical_top50_ranking_matches_the_previous_engine() {
    let Some((path, version)) = index_path() else {
        eprintln!("skipping: no generated canonical search index");
        return;
    };
    if !version.is_empty() && !GOLDEN.contains(&format!("content {version}")) {
        eprintln!("skipping: golden is for another content version");
        return;
    }
    let bytes = std::fs::read(&path).expect("read search index");
    let index = SearchIndex::from_bytes(&bytes).expect("decode search index");
    let blocks = golden_blocks();
    assert_eq!(blocks.len(), SAMPLE_QUERIES);
    for (query, expected) in blocks {
        let hits = index.search_summaries(&Query {
            text: query.clone(),
            limit: TOP,
            ..Default::default()
        });
        let got: Vec<String> = hits.into_iter().map(|hit| hit.id).collect();
        assert_eq!(got, expected, "ranking changed for {query:?}");
    }
}
