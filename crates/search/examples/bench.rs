//! Native benchmark: 5,000 synthetic docs, index size, query latency.
use cast_iron_search::{DocInput, Filters, IndexBuilder, Query, SearchIndex};
use std::time::Instant;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as usize
    }
}

fn main() {
    let syl = [
        "fi", "re", "ba", "ll", "dra", "gon", "wi", "zar", "ar", "mor", "sha", "dow", "ice", "bol",
        "lan", "cer", "ing", "ten", "ta", "cle",
    ];
    let mut rng = Lcg(42);
    let vocab: Vec<String> = (0..3000)
        .map(|_| {
            (0..2 + rng.next() % 3)
                .map(|_| syl[rng.next() % syl.len()])
                .collect()
        })
        .collect();
    let kinds = ["spell", "feat", "monster", "item", "rule"];
    let sources = ["core", "apg", "bestiary", "um", "uc", "ultimate"];
    let mut b = IndexBuilder::new();
    for i in 0..5000 {
        let title: Vec<&str> = (0..1 + rng.next() % 3)
            .map(|_| vocab[rng.next() % 600].as_str())
            .collect();
        let text: Vec<&str> = (0..150 + rng.next() % 250)
            .map(|_| vocab[rng.next() % vocab.len()].as_str())
            .collect();
        b.add(DocInput {
            id: format!("doc:{i:05}"),
            title: title.join(" "),
            aliases: vec![],
            kind: kinds[rng.next() % kinds.len()].into(),
            source: sources[rng.next() % sources.len()].into(),
            section: "Synthetic".into(),
            facets: vec![("level".into(), (rng.next() % 10).to_string())],
            text: text.join(" "),
        });
    }
    let t = Instant::now();
    let idx = b.build();
    let build_ms = t.elapsed().as_millis();
    let bytes = idx.to_bytes();
    let t = Instant::now();
    let idx = SearchIndex::from_bytes(&bytes).expect("round trip");
    let load_ms = t.elapsed().as_millis();

    let mut queries: Vec<Query> = Vec::new();
    for i in 0..200 {
        let w = |k: usize| vocab[(i * 7 + k * 13) % 600].clone();
        let text = match i % 4 {
            0 => w(0),
            1 => w(1)[..w(1).len().min(3)].to_string(),
            2 => format!("{} {}", w(2), w(3)),
            _ => format!("{} {}", w(4), &w(5)[..2]),
        };
        let mut q = Query {
            text,
            ..Default::default()
        };
        if i % 5 == 0 {
            q.filters = Filters {
                kinds: vec!["spell".into()],
                ..Default::default()
            };
        }
        queries.push(q);
    }
    let mut lat: Vec<u128> = Vec::new();
    let mut total_hits = 0;
    for _ in 0..5 {
        for q in &queries {
            let t = Instant::now();
            total_hits += idx.search(q).len();
            lat.push(t.elapsed().as_micros());
        }
    }
    lat.sort();
    let p = |f: f64| lat[((lat.len() as f64 * f) as usize).min(lat.len() - 1)];
    println!(
        "results: docs={} index_bytes={} build_ms={} load_ms={} queries={} hits={} p50_us={} p95_us={}",
        idx.doc_count(),
        bytes.len(),
        build_ms,
        load_ms,
        lat.len(),
        total_hits,
        p(0.50),
        p(0.95)
    );
}
