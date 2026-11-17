//! Lossless plain-text corpus format for Cast Iron.
//!
//! One entry file per [`RuleEntry`], plus a small set of stored source files
//! (`corpus.toml`, `publications.toml`, `books.toml`, `legal.toml`,
//! `exclusions.toml`, `history.toml`, `aliases.toml`, `option-groups.toml`; from format 3 the
//! curated books live in `books/<publication>.toml` and the compiler derives the others). Every other compiler-derived file (relations, spell lists,
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
//! books.toml                    BookIndex (curated chapter order; formats 1 and 2)
//! books/<publication>.toml      one curated book (format 3)
//! legal.toml                    LegalNotices
//! exclusions.toml               what the import left out, append-only
//! history.toml                  frozen import report
//! aliases.toml                  AliasIndex (optional, as in the library)
//! ```

mod block;
mod books;
mod check;
mod check_trades;
pub mod cli;
mod config;
mod drift;
mod entry;
mod error;
mod files;
mod fmt;
mod inline;
mod layout;
mod load;
mod migrate;
mod migrate_trades;
#[cfg(test)]
mod testkit;
mod tomlutil;
mod trade_match;
mod trades;

pub use books::{
    BOOKS_DIR, BookLayout, PAGE_ORDER_TITLE, UNPAGED_TITLE, book_index, is_clean_page,
    is_reserved_chapter, page_in,
};
pub use check::{Report, base_class_ids, check, fields_kind, fields_match, heading_keys, line_of};
pub use config::{
    CORPUS_FORMAT, Config, DEFAULT_LICENSE_REFS, MIGRATE_COMMAND, READABLE_FORMATS,
    SPLIT_BOOKS_FORMAT, TRADES_FORMAT, read_config, write_config,
};
pub use drift::{Disagreement, find_drift};
pub use entry::{derived_links, entry_path, read_entry, write_entry};
pub use error::{Error, Result};
pub use files::{
    Exclusions, OptionGroupDef, OptionGroups, Publication, PublicationsFile, read_aliases,
    read_books, read_curated_book, read_exclusions, read_history, read_legal, read_option_groups,
    read_publications, write_aliases, write_books, write_curated_book, write_exclusions,
    write_history, write_legal, write_option_groups, write_publication_list, write_publications,
};
pub use fmt::{Change, FmtReport, format_corpus};
pub use layout::{SplitReport, layout_of, migrate_layout, split_books};
pub use load::{Corpus, Stored, list_files};
pub use migrate::{MIGRATIONS, Migration, migrate, migrate_with};
pub use migrate_trades::{TradesReport, migrate_trades};
