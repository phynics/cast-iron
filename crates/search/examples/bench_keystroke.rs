//! Native timing of one keystroke's passes over a compiled library's `search.bin`, the way the
//! search page runs them: the entry search, the book-section search, the unfiltered search that
//! counts hidden matches, and the filter counts. Every third source is switched off, a stand-in
//! for the default preset, which leaves some book inactive.
//!
//! ```sh
//! cargo run --release -p cast-iron-search --example bench_keystroke -- \
//!   generated/content/<version>/search.bin [fire ma cure "magic missle"]
//! ```
//!
//! `closure` evaluates the id set for every candidate and rebuilds the mask for the counts, as
//! before. `mask` builds the mask once, as the client does per book selection, and reuses it.

use cast_iron_search::{BOOK_SECTION_KIND, Filters, Query, SearchIndex};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const REPS: usize = 7;

fn median(mut values: Vec<Duration>) -> Duration {
    values.sort();
    values[values.len() / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().expect("pass a search.bin path"));
    let mut queries: Vec<String> = args.collect();
    if queries.is_empty() {
        queries = ["fire", "ma", "cure", "magic missle"]
            .map(String::from)
            .to_vec();
    }
    let bytes = std::fs::read(&path).expect("cannot read the index");
    let index = SearchIndex::from_bytes(&bytes).expect("cannot decode the index");

    let mut sources: Vec<String> = (0..index.doc_count() as u32)
        .filter_map(|ord| index.doc(ord).map(|d| d.source.to_string()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    sources.sort();
    let off: BTreeSet<&str> = sources.iter().step_by(3).map(String::as_str).collect();
    let ids: BTreeSet<String> = (0..index.doc_count() as u32)
        .filter_map(|ord| index.doc(ord))
        .filter(|d| d.kind != BOOK_SECTION_KIND && !off.contains(d.source))
        .map(|d| d.id.to_string())
        .collect();
    println!(
        "docs={} sources={} inactive={} eligible_ids={}",
        index.doc_count(),
        sources.len(),
        off.len(),
        ids.len()
    );

    let started = Instant::now();
    let entries = index.eligible_mask_ids(|id| ids.contains(id));
    let sections = index.eligible_mask(|d| d.kind == BOOK_SECTION_KIND && !off.contains(d.source));
    println!(
        "mask build (once per selection): {:.2} ms",
        ms(started.elapsed())
    );

    println!("\nquery            closure_ms    mask_ms   (median of {REPS}, all four passes)");
    for text in &queries {
        let query = Query {
            text: text.clone(),
            limit: usize::MAX,
            ..Default::default()
        };
        let mut closure = Vec::new();
        let mut masked = Vec::new();
        let mut hits = (0, 0);
        for _ in 0..REPS {
            let t = Instant::now();
            let a = index.search_summaries_eligible(&query, |id| ids.contains(id));
            let b = index.search_summaries_docs(&query, |d| {
                d.kind == BOOK_SECTION_KIND && !off.contains(d.source)
            });
            let c = index.search_summaries(&query);
            let d = index.filter_counts_eligible(text, &Filters::default(), |id| ids.contains(id));
            closure.push(t.elapsed());
            hits.0 = a.len() + b.len() + c.len() + d.total as usize;

            let t = Instant::now();
            let a2 = index.search_summaries_masked(&query, &entries);
            let b2 = index.search_summaries_masked(&query, &sections);
            let c2 = index.search_summaries(&query);
            let d2 = index.filter_counts_masked(text, &Filters::default(), &entries);
            masked.push(t.elapsed());
            hits.1 = a2.len() + b2.len() + c2.len() + d2.total as usize;
            assert_eq!((a, b, c, d), (a2, b2, c2, d2), "{text}: results differ");
        }
        println!(
            "{text:<16} {:>10.2} {:>10.2}   hits={}",
            ms(median(closure)),
            ms(median(masked)),
            hits.0
        );
        assert_eq!(hits.0, hits.1);
    }
}
