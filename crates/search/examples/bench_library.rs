//! Native benchmark against a compiled library's `search.bin`.
//!
//! ```sh
//! cargo run --release -p cast-iron-search --example bench_library --features profile -- \
//!   generated/content/vd7596846128e349b/search.bin
//! ```
//!
//! It uses the query list from `tests/e2e/measure-library.mjs`, which is the
//! superset of the list in `tests/e2e/tests/search.spec.ts`. The default limit is
//! the one the web view uses (all matches), so the numbers include the work done
//! for the full result set, not only the visible page.

use cast_iron_search::{Query, SearchIndex, SearchTiming};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const QUERIES: &[&str] = &[
    "fire",
    "cure light wounds",
    "grapple",
    "dragon",
    "sword",
    "armor class",
    "turn undead",
    "magic missile",
    "wizard",
    "bless",
    "invisibility",
    "potion",
    "goblin",
    "attack of opportunity",
    "ranger",
    "stealth",
    "charm",
    "weapon focus",
    "fly",
    "summon",
    "acid",
    "angel",
    "animal",
    "antipaladin",
    "arcane",
    "armor",
    "arrow",
    "bard",
    "bear",
    "blade",
    "blind",
    "blood",
    "bolt",
    "bow",
    "burn",
    "chaos",
    "cleric",
    "cold",
    "combat",
    "cure",
    "darkness",
    "death",
    "demon",
    "detect",
    "divine",
    "dwarf",
    "eagle",
    "earth",
    "elemental",
    "energy",
];

const REPS: usize = 3;

fn default_index() -> Option<PathBuf> {
    let manifest = std::fs::read_to_string("generated/content/manifest.json").ok()?;
    let version = manifest.split("\"content_version\":\"").nth(1)?;
    let version = version.split('"').next()?;
    Some(PathBuf::from(format!(
        "generated/content/{version}/search.bin"
    )))
}

fn pct(values: &mut [u128], p: f64) -> u128 {
    values.sort_unstable();
    let index = ((values.len() as f64 * p).ceil() as usize).saturating_sub(1);
    values[index.min(values.len() - 1)]
}

fn ms(value: u128) -> f64 {
    value as f64 / 1_000_000.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .map(PathBuf::from)
        .or_else(default_index)
        .expect("pass a search.bin path or run from the repository root");
    let mode = args.next().unwrap_or_else(|| "web".to_string());
    let (limit, summaries) = match mode.as_str() {
        "full" => (usize::MAX, false),
        "page" => (50, false),
        "web" => (usize::MAX, true),
        other => panic!("mode must be full, page, or web; got {other}"),
    };
    const PAGE: usize = 50;

    let bytes = std::fs::read(&path).expect("cannot read the index");
    let started = Instant::now();
    let index = SearchIndex::from_bytes(&bytes).expect("cannot decode the index");
    let load = started.elapsed();
    println!(
        "index path={} bytes={} docs={} load_ms={:.2} mode={} limit={}",
        path.display(),
        bytes.len(),
        index.doc_count(),
        load.as_secs_f64() * 1000.0,
        mode,
        limit
    );

    let mut phase_names = [
        ("tokenize", Vec::new()),
        ("lookup", Vec::new()),
        ("scoring", Vec::new()),
        ("sort", Vec::new()),
        ("materialize", Vec::new()),
        ("snippets", Vec::new()),
        ("total", Vec::new()),
    ];
    let mut hit_counts = Vec::new();

    for &text in QUERIES {
        let query = Query {
            text: text.to_string(),
            limit,
            ..Default::default()
        };
        let mut timings: Vec<SearchTiming> = Vec::with_capacity(REPS);
        for _ in 0..REPS {
            if summaries {
                let (found, mut timing) = index.search_summaries_profiled(&query, |_| true);
                let page: Vec<_> = found.into_iter().take(PAGE).collect();
                let started = Instant::now();
                let mut hits: Vec<_> = page.into_iter().map(|row| row.into_hit()).collect();
                timing.materialize += started.elapsed();
                let highlighter = index.highlighter(&query);
                let started = Instant::now();
                for hit in &mut hits {
                    index.materialize_hit(&highlighter, hit);
                }
                timing.snippets += started.elapsed();
                timing.total += timing.materialize + timing.snippets;
                timings.push(timing);
            } else {
                let (_, timing) = index.search_profiled(&query, |_| true);
                timings.push(timing);
            }
        }
        let nth = |field: fn(&SearchTiming) -> Duration| {
            let mut values: Vec<u128> = timings
                .iter()
                .map(|timing| field(timing).as_nanos())
                .collect();
            pct(&mut values, 0.5)
        };
        let fields = [
            nth(|t| t.tokenize),
            nth(|t| t.lookup),
            nth(|t| t.scoring),
            nth(|t| t.sort),
            nth(|t| t.materialize),
            nth(|t| t.snippets),
            nth(|t| t.total),
        ];
        for ((_, list), value) in phase_names.iter_mut().zip(fields) {
            list.push(value);
        }
        hit_counts.push(timings[REPS - 1].hits);
        println!(
            "{text:<24} hits={:<6} tokenize={:>7.2} lookup={:>7.2} scoring={:>7.2} sort={:>7.2} materialize={:>7.2} snippets={:>7.2} total={:>7.2}",
            timings[REPS - 1].hits,
            ms(fields[0]),
            ms(fields[1]),
            ms(fields[2]),
            ms(fields[3]),
            ms(fields[4]),
            ms(fields[5]),
            ms(fields[6])
        );
    }

    println!(
        "\nphase p50 / p95 over {} queries x {} reps (ms)",
        QUERIES.len(),
        REPS
    );
    for (name, values) in &mut phase_names {
        let p50 = pct(values, 0.50);
        let p95 = pct(values, 0.95);
        let max = *values.iter().max().unwrap_or(&0);
        println!(
            "{name:<12} p50={:>8.2} p95={:>8.2} max={:>8.2}",
            ms(p50),
            ms(p95),
            ms(max)
        );
    }
    let hits: Vec<u128> = hit_counts.iter().map(|&n| n as u128).collect();
    println!(
        "hits p50={} p95={} max={}",
        pct(&mut hits.clone(), 0.50),
        pct(&mut hits.clone(), 0.95),
        hits.iter().max().copied().unwrap_or(0)
    );
}
