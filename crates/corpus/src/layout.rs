//! The layout migration of corpus format 3 (#425): `books.toml` becomes one file per curated
//! book under `books/`, and the page-order books are derived by the compiler instead.
//!
//! It is a mechanical cut. A book with a printed chapter (any top-level chapter that is not one
//! of the compiler's) is curated: its file keeps the stored `[[books]]` block minus the listing
//! chapters (`Page order`, `Unpaged`, `New entries (page order)`), which the compiler rebuilds. A
//! book with no printed chapter has no file at all.
//!
//! The step is idempotent and safe to repeat after a crash: `books.toml` stays the source of truth
//! until the last step removes it, so a rerun rewrites `books/` from it. A corpus that is already
//! split has nothing to do.

use crate::books::{BOOKS_DIR, BookLayout, PAGE_ORDER_TITLE, UNPAGED_TITLE, is_reserved_chapter};
use crate::config::{Config, SPLIT_BOOKS_FORMAT, read_config, write_config};
use crate::files::{read_books, write_curated_book};
use std::fs;
use std::path::Path;

/// What a split did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SplitReport {
    /// The curated books written, in `books.toml` order.
    pub curated: Vec<String>,
    /// The books that now have no stored tree: the compiler derives them.
    pub derived: usize,
    /// Curated books whose stored `Page order` or `Unpaged` chapter was dropped. The derivation
    /// puts those entries in the book's hidden chapter instead; none exists in the pinned corpus.
    pub listing_dropped: Vec<String>,
    /// True when the corpus was split already and nothing changed.
    pub already_split: bool,
}

impl SplitReport {
    pub fn describe(&self) -> String {
        if self.already_split {
            return "the books are already split; nothing to do".to_string();
        }
        let mut text = format!(
            "split books.toml: {} curated book file(s) in {BOOKS_DIR}/, {} book(s) left to the compiler",
            self.curated.len(),
            self.derived
        );
        if !self.listing_dropped.is_empty() {
            text.push_str(&format!(
                "; dropped a stored listing chapter from {}",
                self.listing_dropped.join(", ")
            ));
        }
        text
    }
}

/// Whether a stored book is hand-curated: it has a top-level chapter the compiler does not own.
fn is_curated(chapters: &[cast_iron_schema::BookNode]) -> bool {
    chapters
        .iter()
        .any(|chapter| !is_reserved_chapter(&chapter.title))
}

/// Cuts `books.toml` of the corpus at `dir` into `books/`, then removes it. Does not touch
/// `corpus.toml`.
pub fn split_books(dir: &Path) -> Result<SplitReport, String> {
    let single = dir.join("books.toml");
    if !single.is_file() {
        return if dir.join(BOOKS_DIR).is_dir() {
            Ok(SplitReport {
                already_split: true,
                ..SplitReport::default()
            })
        } else {
            Err(format!(
                "{} has neither books.toml nor {BOOKS_DIR}/",
                dir.display()
            ))
        };
    }
    let text = fs::read_to_string(&single)
        .map_err(|e| format!("cannot read {}: {e}", single.display()))?;
    let index = read_books(Path::new("books.toml"), &text).map_err(|e| e.to_string())?;

    // Files first, in a staging directory; `books/` appears in one rename, and `books.toml`
    // goes last. A rerun after a crash starts again from `books.toml`.
    let target = dir.join(BOOKS_DIR);
    let stage = dir.join(format!("{BOOKS_DIR}.staging"));
    if stage.exists() {
        fs::remove_dir_all(&stage).map_err(|e| format!("cannot clear {}: {e}", stage.display()))?;
    }
    fs::create_dir_all(&stage).map_err(|e| format!("cannot create {}: {e}", stage.display()))?;
    let mut report = SplitReport::default();
    for mut tree in index.books {
        if !is_curated(&tree.chapters) {
            report.derived += 1;
            continue;
        }
        if tree
            .chapters
            .iter()
            .any(|chapter| chapter.title == PAGE_ORDER_TITLE || chapter.title == UNPAGED_TITLE)
        {
            report.listing_dropped.push(tree.source.clone());
        }
        tree.chapters
            .retain(|chapter| !is_reserved_chapter(&chapter.title));
        let file = stage.join(format!("{}.toml", tree.source));
        fs::write(&file, write_curated_book(&tree))
            .map_err(|e| format!("cannot write {}: {e}", file.display()))?;
        report.curated.push(tree.source.clone());
    }
    if target.exists() {
        fs::remove_dir_all(&target)
            .map_err(|e| format!("cannot replace {}: {e}", target.display()))?;
    }
    fs::rename(&stage, &target).map_err(|e| format!("cannot create {}: {e}", target.display()))?;
    fs::remove_file(&single).map_err(|e| format!("cannot remove {}: {e}", single.display()))?;
    Ok(report)
}

/// Rewrites `corpus.toml` to `format`, keeping the defaults.
fn set_format(dir: &Path, format: u32) -> Result<(), String> {
    let path = dir.join("corpus.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let config = read_config(Path::new("corpus.toml"), &text).map_err(|e| e.to_string())?;
    fs::write(&path, write_config(&Config { format, ..config }))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// The registered migration step from format 2: splits the books and declares format 3.
pub fn migrate_to_split(dir: &Path) -> Result<(), String> {
    split_books(dir)?;
    set_format(dir, SPLIT_BOOKS_FORMAT)
}

/// `cast-iron-corpus migrate-layout`: the layout step alone. It declares format 3 only when the
/// corpus is at format 2, because a corpus at format 1 still has the trades migration to run. The
/// reader accepts the split layout in either case.
pub fn migrate_layout(dir: &Path) -> Result<String, String> {
    let report = split_books(dir)?;
    let path = dir.join("corpus.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let config = read_config(Path::new("corpus.toml"), &text).map_err(|e| e.to_string())?;
    let mut message = report.describe();
    if config.format == SPLIT_BOOKS_FORMAT - 1 {
        set_format(dir, SPLIT_BOOKS_FORMAT)?;
        message.push_str(&format!(
            "; corpus.toml now declares format {SPLIT_BOOKS_FORMAT}"
        ));
    } else if config.format < SPLIT_BOOKS_FORMAT {
        message.push_str(&format!(
            "; corpus.toml stays at format {}: run `cast-iron-corpus migrate` for the other steps",
            config.format
        ));
    }
    Ok(message)
}

/// The layout of a corpus directory, read from the disk without loading it.
pub fn layout_of(dir: &Path) -> Option<BookLayout> {
    match (
        dir.join("books.toml").is_file(),
        dir.join(BOOKS_DIR).is_dir(),
    ) {
        (true, false) => Some(BookLayout::Single),
        (false, true) => Some(BookLayout::Split),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{BookIndex, BookNode, BookTree};

    fn node(title: &str, entry: Option<&str>, children: Vec<BookNode>) -> BookNode {
        BookNode {
            title: title.into(),
            entry: entry.map(|id| cast_iron_schema::EntryId::parse(id).unwrap()),
            anchor: None,
            review: None,
            collapse: false,
            lists_children: false,
            children,
        }
    }

    fn tree(source: &str, chapters: Vec<BookNode>) -> BookTree {
        BookTree {
            source: source.into(),
            title: source.to_uppercase(),
            basis: "basis".into(),
            review: vec!["note".into()],
            chapters,
        }
    }

    fn scratch(name: &str, format: u32, books: &BookIndex) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cast-iron-layout-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("corpus.toml"),
            format!("format = {format}\n\n[defaults]\nlicense_refs = []\n"),
        )
        .unwrap();
        fs::write(dir.join("books.toml"), crate::files::write_books(books)).unwrap();
        dir
    }

    fn sample() -> BookIndex {
        BookIndex {
            books: vec![
                tree(
                    "curated",
                    vec![
                        node(
                            "Chapter 1",
                            None,
                            vec![node("A", Some("ref/rule/a"), vec![])],
                        ),
                        node(
                            "New entries (page order)",
                            None,
                            vec![node("B", Some("ref/rule/b"), vec![])],
                        ),
                    ],
                ),
                tree(
                    "listed",
                    vec![node(
                        "Page order",
                        None,
                        vec![node("C", Some("ref/rule/c"), vec![])],
                    )],
                ),
                tree("empty", vec![]),
            ],
        }
    }

    #[test]
    fn the_split_keeps_each_curated_book_without_its_catch_all_and_drops_the_listings() {
        let dir = scratch("split", 2, &sample());
        let report = split_books(&dir).unwrap();
        assert_eq!(report.curated, ["curated"]);
        assert_eq!(report.derived, 2);
        assert!(!dir.join("books.toml").exists());
        let text = fs::read_to_string(dir.join("books/curated.toml")).unwrap();
        let read = crate::files::read_curated_book(Path::new("books/curated.toml"), &text).unwrap();
        assert_eq!(read.chapters.len(), 1);
        assert_eq!(read.chapters[0].title, "Chapter 1");
        assert_eq!(read.basis, "basis");
        assert!(!dir.join("books/listed.toml").exists());
        assert!(!dir.join("books.staging").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_run_changes_nothing_and_a_crashed_run_converges() {
        let dir = scratch("again", 2, &sample());
        split_books(&dir).unwrap();
        let first = fs::read_to_string(dir.join("books/curated.toml")).unwrap();
        let again = split_books(&dir).unwrap();
        assert!(again.already_split, "{again:?}");
        assert_eq!(
            fs::read_to_string(dir.join("books/curated.toml")).unwrap(),
            first
        );
        // A crash after the books/ rename but before books.toml was removed: both exist.
        fs::write(dir.join("books.toml"), crate::files::write_books(&sample())).unwrap();
        fs::write(dir.join("books/stale.toml"), "junk").unwrap();
        split_books(&dir).unwrap();
        assert!(!dir.join("books/stale.toml").exists());
        assert!(!dir.join("books.toml").exists());
        assert_eq!(
            fs::read_to_string(dir.join("books/curated.toml")).unwrap(),
            first
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_registered_step_declares_format_3_and_the_layout_command_only_from_format_2() {
        let dir = scratch("step", 2, &sample());
        migrate_to_split(&dir).unwrap();
        let text = fs::read_to_string(dir.join("corpus.toml")).unwrap();
        assert!(text.starts_with("format = 3\n"), "{text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = scratch("layout-1", 1, &sample());
        let message = migrate_layout(&dir).unwrap();
        assert!(message.contains("stays at format 1"), "{message}");
        let text = fs::read_to_string(dir.join("corpus.toml")).unwrap();
        assert!(text.starts_with("format = 1\n"), "{text}");
        assert_eq!(layout_of(&dir), Some(BookLayout::Split));
        let _ = fs::remove_dir_all(&dir);

        let dir = scratch("layout-2", 2, &sample());
        let message = migrate_layout(&dir).unwrap();
        assert!(message.contains("now declares format 3"), "{message}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_corpus_with_no_books_at_all_is_an_error() {
        let dir =
            std::env::temp_dir().join(format!("cast-iron-layout-none-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(split_books(&dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
