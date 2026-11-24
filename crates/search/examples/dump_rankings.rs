//! Prints the ranking blocks of the canonical top-50 golden.
//!
//! ```sh
//! cargo run --release -p cast-iron-search --example dump_rankings -- \
//!   generated/content/<version>/search.bin crates/search/tests/golden/canonical-top50.txt
//! ```
//!
//! The queries are the `query: <text>` lines of the golden file named by the second argument;
//! the output is the same blocks with fresh results, ids only. `cargo xtask search-golden` runs
//! this and adds the header lines. The summary path is the web view's path.
use cast_iron_search::{Query, SearchIndex};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("index path");
    let golden = args.next().expect("golden file");
    let text = std::fs::read_to_string(golden).expect("golden");
    let bytes = std::fs::read(path).expect("read");
    let index = SearchIndex::from_bytes(&bytes).expect("decode");
    for text in text.lines().filter_map(|line| line.strip_prefix("query: ")) {
        let hits = index.search_summaries(&Query {
            text: text.to_string(),
            limit: 50,
            ..Default::default()
        });
        println!("query: {text}");
        for hit in hits {
            println!("{}", hit.id);
        }
    }
}
