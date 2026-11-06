//! `cast-iron-corpus`: check and format a plain-text Cast Iron corpus.
//!
//! This binary needs only the corpus and schema crates, so a data repository can build it
//! without the web or WebAssembly toolchain. Run `cast-iron-corpus help`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cast_iron_corpus::cli::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
