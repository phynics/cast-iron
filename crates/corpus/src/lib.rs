//! Lossless plain-text corpus format for Cast Iron.
//!
//! One entry file per [`RuleEntry`], plus a small set of stored source files
//! (`corpus.toml`, `publications.toml`, `books.toml`, `legal.toml`,
//! `exclusions.toml`, `history.toml`, `aliases.toml`). Every other compiler-derived file (relations, spell lists,
//! backlinks, catalog stubs, search index, manifest) is rebuilt by the compiler,
//! not stored.
//!
//! The format is documented in `docs/corpus-format.md`. This crate is the
//! executable definition: [`write_entry`] and [`read_entry`] must agree for
//! every value the schema can hold.
//!
//! ```text
//! entries/<id-path>.md          one RuleEntry, front matter and body
//! publications.toml             source metadata for the catalog
//! books.toml                    BookIndex (curated chapter order)
//! legal.toml                    LegalNotices
//! exclusions.toml               what the import left out, append-only
//! history.toml                  frozen import report
//! aliases.toml                  AliasIndex (optional, as in the library)
//! ```

mod block;
mod check;
mod config;
mod entry;
mod error;
mod files;
mod inline;
mod load;
#[cfg(test)]
mod testkit;
mod tomlutil;

pub use check::{Report, check, fields_kind, fields_match, heading_keys, line_of};
pub use config::{
    CORPUS_FORMAT, Config, DEFAULT_LICENSE_REFS, MIGRATE_COMMAND, read_config, write_config,
};
pub use entry::{derived_links, entry_path, read_entry, write_entry};
pub use error::{Error, Result};
pub use files::{
    Exclusions, Publication, PublicationsFile, read_aliases, read_books, read_exclusions,
    read_history, read_legal, read_publications, write_aliases, write_books, write_exclusions,
    write_history, write_legal, write_publications,
};
pub use load::{Corpus, Stored, list_files};
