//! Prints top-50 ids and scores for a query list, in the golden-test format.
//!
//! ```sh
//! cargo run --release -p cast-iron-search --example dump_rankings -- \
//!   generated/content/<version>/search.bin /tmp/queries.txt
//! ```
//!
//! Used to refresh `crates/search/tests/golden/canonical-top50.txt` after a
//! deliberate ranking change. The summary path is the web view's path.
use cast_iron_search::{Query, SearchIndex};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("index path");
    let query_file = args.next().expect("query file");
    let text = std::fs::read_to_string(query_file).expect("queries");
    let bytes = std::fs::read(path).expect("read");
    let index = SearchIndex::from_bytes(&bytes).expect("decode");
    for text in text.lines().filter(|line| !line.is_empty()) {
        let hits = index.search_summaries(&Query {
            text: text.to_string(),
            limit: 50,
            ..Default::default()
        });
        println!("{text}");
        for hit in hits {
            println!("{}\t{}", hit.id, hit.score);
        }
    }
}
