//! Reading a whole corpus directory.

use crate::books::{BOOKS_DIR, BookLayout, book_index};
use crate::config::{Config, SPLIT_BOOKS_FORMAT, read_config};
use crate::entry::read_entry_with_lines;
use crate::error::{Error, Result};
use crate::files::{
    Exclusions, OptionGroups, Publication, read_aliases, read_books, read_curated_book,
    read_exclusions, read_history, read_legal, read_option_groups, read_publications,
};
use cast_iron_schema::{AliasIndex, BookIndex, CoverageHistory, LegalNotices, RuleEntry};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// One entry file as read: the value, its path relative to the corpus root, and
/// the source text, which checks use to point at a line.
#[derive(Debug, Clone)]
pub struct Stored {
    pub entry: RuleEntry,
    pub path: PathBuf,
    pub text: String,
    /// The 1-based line of `text` that each top-level body block starts on, in body order. Empty
    /// when the parser did not record it (an entry built in a test).
    pub block_lines: Vec<usize>,
}

/// A corpus directory, read and parsed. The stored files keep their text so a
/// check can name the line a problem sits on.
#[derive(Debug, Clone)]
pub struct Corpus {
    pub dir: PathBuf,
    pub config: Config,
    /// Entries by permanent id.
    pub entries: BTreeMap<String, Stored>,
    pub publications: Vec<Publication>,
    /// The class-option vocabulary. Empty when the corpus has no `option-groups.toml`.
    pub option_groups: OptionGroups,
    /// The books the corpus stores: every book in the single `books.toml` layout, the hand-curated
    /// books in the split layout. [`Corpus::book_index`] is the whole index.
    pub books: BookIndex,
    /// The text of `books.toml`; empty in the split layout.
    pub books_text: String,
    pub layout: BookLayout,
    /// The text of each `books/<publication>.toml`, by publication id; empty in the single layout.
    pub book_texts: BTreeMap<String, String>,
    pub legal: LegalNotices,
    pub exclusions: Exclusions,
    /// The frozen import report. Absent in a corpus that never had an import.
    pub history: Option<CoverageHistory>,
    pub aliases: Option<AliasIndex>,
}

/// Every file below `dir`, recursively, unsorted.
pub fn list_files(dir: &Path, found: &mut Vec<PathBuf>) -> std::result::Result<(), String> {
    for item in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = item.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            list_files(&path, found)?;
        } else {
            found.push(path);
        }
    }
    Ok(())
}

fn take<T>(errors: &mut Vec<Error>, result: Result<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(error);
            None
        }
    }
}

fn read_text(dir: &Path, name: &str) -> Result<String> {
    let path = dir.join(name);
    fs::read_to_string(&path).map_err(|e| Error::new(name, 1, format!("cannot read file: {e}")))
}

/// The curated books of `dir/books`, with their texts, or the errors that stop them loading.
fn read_book_dir(
    dir: &Path,
    errors: &mut Vec<Error>,
) -> Option<(BookIndex, BTreeMap<String, String>)> {
    let books_dir = dir.join(BOOKS_DIR);
    let mut files = Vec::new();
    if let Err(message) = list_files(&books_dir, &mut files) {
        errors.push(Error::new(BOOKS_DIR, 1, message));
        return None;
    }
    files.sort();
    let mut index = BookIndex { books: Vec::new() };
    let mut texts = BTreeMap::new();
    let before = errors.len();
    for file in files {
        let relative = file.strip_prefix(dir).unwrap_or(&file).to_path_buf();
        let is_book = file.extension().is_some_and(|ext| ext == "toml")
            && file.parent() == Some(books_dir.as_path());
        if !is_book {
            errors.push(Error::new(
                &relative,
                1,
                format!("only `<publication>.toml` files belong in `{BOOKS_DIR}/`"),
            ));
            continue;
        }
        match fs::read_to_string(&file) {
            Ok(text) => match read_curated_book(&relative, &text) {
                Ok(tree) => {
                    texts.insert(tree.source.clone(), text);
                    index.books.push(tree);
                }
                Err(error) => errors.push(error),
            },
            Err(error) => errors.push(Error::new(
                &relative,
                1,
                format!("cannot read file: {error}"),
            )),
        }
    }
    (errors.len() == before).then_some((index, texts))
}

impl Corpus {
    /// The whole book index: the stored books as they are in the single layout, and in the split
    /// layout the curated books with their catch-all chapters plus every derived book.
    pub fn book_index(&self) -> BookIndex {
        match self.layout {
            BookLayout::Single => self.books.clone(),
            BookLayout::Split => book_index(&self.books.books, &self.publications, &self.entries),
        }
    }

    /// Reads every file. Returns all the errors it finds, not only the first,
    /// so one run lists every file that needs a fix.
    pub fn load(dir: &Path) -> std::result::Result<Corpus, Vec<Error>> {
        let mut errors = Vec::new();
        let config = take(
            &mut errors,
            read_text(dir, "corpus.toml")
                .and_then(|text| read_config(Path::new("corpus.toml"), &text)),
        );
        let publications = take(
            &mut errors,
            read_text(dir, "publications.toml")
                .and_then(|text| read_publications(Path::new("publications.toml"), &text)),
        );
        let option_groups = if dir.join("option-groups.toml").is_file() {
            take(
                &mut errors,
                read_text(dir, "option-groups.toml")
                    .and_then(|text| read_option_groups(Path::new("option-groups.toml"), &text)),
            )
            .map(Some)
        } else {
            Some(Some(OptionGroups::default()))
        };
        // The layout is read from the disk. A corpus of format 3 or above must hold the split one.
        let has_single = dir.join("books.toml").is_file();
        let has_split = dir.join(BOOKS_DIR).is_dir();
        let layout = match (has_single, has_split) {
            (true, false) => Some(BookLayout::Single),
            (false, true) => Some(BookLayout::Split),
            (true, true) => {
                errors.push(Error::new(
                    "books.toml",
                    1,
                    format!(
                        "the corpus holds both `books.toml` and `{BOOKS_DIR}/`; keep one layout. `cast-iron-corpus migrate` finishes a half-done split by rewriting `{BOOKS_DIR}/` from `books.toml` and removing it"
                    ),
                ));
                None
            }
            (false, false) => {
                errors.push(Error::new(
                    "books.toml",
                    1,
                    format!("the corpus has neither `books.toml` nor a `{BOOKS_DIR}/` directory"),
                ));
                None
            }
        };
        if let (Some(config), Some(BookLayout::Single)) = (&config, layout)
            && config.format >= SPLIT_BOOKS_FORMAT
        {
            errors.push(Error::new(
                "books.toml",
                1,
                format!(
                    "format {} stores the curated books in `{BOOKS_DIR}/`, not in `books.toml`; run `{}`",
                    config.format,
                    crate::config::MIGRATE_COMMAND
                ),
            ));
        }
        let (books, books_text, book_texts) = match layout {
            Some(BookLayout::Single) => {
                let text = take(&mut errors, read_text(dir, "books.toml"));
                let books = text
                    .as_deref()
                    .and_then(|text| take(&mut errors, read_books(Path::new("books.toml"), text)));
                (books, text, Some(BTreeMap::new()))
            }
            Some(BookLayout::Split) => match read_book_dir(dir, &mut errors) {
                Some((index, texts)) => (Some(index), Some(String::new()), Some(texts)),
                None => (None, None, None),
            },
            None => (None, None, None),
        };
        let legal = take(
            &mut errors,
            read_text(dir, "legal.toml")
                .and_then(|text| read_legal(Path::new("legal.toml"), &text)),
        );
        let exclusions = take(
            &mut errors,
            read_text(dir, "exclusions.toml")
                .and_then(|text| read_exclusions(Path::new("exclusions.toml"), &text)),
        );
        let history = if dir.join("history.toml").is_file() {
            take(
                &mut errors,
                read_text(dir, "history.toml")
                    .and_then(|text| read_history(Path::new("history.toml"), &text)),
            )
            .map(Some)
        } else {
            Some(None)
        };
        let aliases = if dir.join("aliases.toml").is_file() {
            take(
                &mut errors,
                read_text(dir, "aliases.toml")
                    .and_then(|text| read_aliases(Path::new("aliases.toml"), &text)),
            )
            .map(Some)
        } else {
            Some(None)
        };

        let read_config_value = config.clone().unwrap_or_default();
        let mut files = Vec::new();
        if let Err(message) = list_files(&dir.join("entries"), &mut files) {
            errors.push(Error::new("entries", 1, message));
        }
        files.sort();
        let mut entries: BTreeMap<String, Stored> = BTreeMap::new();
        for file in &files {
            if file.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let relative = match file.strip_prefix(dir) {
                Ok(relative) => relative.to_path_buf(),
                Err(_) => file.clone(),
            };
            let text = match fs::read_to_string(file) {
                Ok(text) => text,
                Err(error) => {
                    errors.push(Error::new(
                        &relative,
                        1,
                        format!("cannot read file: {error}"),
                    ));
                    continue;
                }
            };
            match read_entry_with_lines(&relative, &text, &read_config_value) {
                Ok((entry, block_lines)) => {
                    let id = entry.id.to_string();
                    let stored = Stored {
                        entry,
                        path: relative.clone(),
                        text,
                        block_lines,
                    };
                    if entries.insert(id.clone(), stored).is_some() {
                        errors.push(Error::new(&relative, 1, format!("duplicate entry id {id}")));
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        match (
            config,
            publications,
            option_groups,
            books,
            books_text,
            book_texts,
            layout,
            legal,
            exclusions,
            history,
            aliases,
        ) {
            (
                Some(config),
                Some(publications),
                Some(Some(option_groups)),
                Some(books),
                Some(books_text),
                Some(book_texts),
                Some(layout),
                Some(legal),
                Some(exclusions),
                Some(history),
                Some(aliases),
            ) if errors.is_empty() => Ok(Corpus {
                dir: dir.to_path_buf(),
                config,
                entries,
                publications,
                option_groups,
                books,
                books_text,
                layout,
                book_texts,
                legal,
                exclusions,
                history,
                aliases,
            }),
            _ => Err(errors),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, write_config};
    use crate::entry::{entry_path, write_entry};
    use crate::files::{write_books, write_exclusions, write_legal};
    use crate::testkit::{corpus, paragraph, rule};
    use cast_iron_schema::Inline;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cast-iron-corpus-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_corpus(dir: &Path, source: &Corpus) {
        for stored in source.entries.values() {
            let path = dir.join(entry_path(&stored.entry.id));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, write_entry(&stored.entry, &source.config)).unwrap();
        }
        fs::write(dir.join("corpus.toml"), write_config(&source.config)).unwrap();
        fs::write(
            dir.join("publications.toml"),
            "[[publication]]\nid = \"core-rulebook\"\ntitle = \"Core Rulebook\"\n",
        )
        .unwrap();
        fs::write(dir.join("books.toml"), write_books(&source.books)).unwrap();
        fs::write(dir.join("legal.toml"), write_legal(&source.legal)).unwrap();
        fs::write(
            dir.join("exclusions.toml"),
            write_exclusions(&source.exclusions),
        )
        .unwrap();
    }

    #[test]
    fn a_written_corpus_loads_back_and_keeps_every_entry_text() {
        let dir = temp_dir("load");
        let source = corpus(vec![rule(
            "ref/rule/a",
            vec![paragraph(vec![Inline::text("hello")])],
        )]);
        write_corpus(&dir, &source);
        let loaded = Corpus::load(&dir).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        let stored = &loaded.entries["ref/rule/a"];
        assert_eq!(stored.entry, source.entries["ref/rule/a"].entry);
        assert_eq!(stored.path, Path::new("entries/rule/a.md"));
        assert!(stored.text.contains("hello"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_reports_every_bad_file_not_only_the_first() {
        let dir = temp_dir("errors");
        let source = corpus(vec![rule("ref/rule/a", vec![])]);
        write_corpus(&dir, &source);
        fs::write(dir.join("entries/rule/a.md"), "no front matter").unwrap();
        fs::create_dir_all(dir.join("entries/rule")).unwrap();
        fs::write(dir.join("entries/rule/b.md"), "also bad").unwrap();
        let errors = Corpus::load(&dir).unwrap_err();
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors.iter().all(|error| error.line >= 1));
        let _ = fs::remove_dir_all(&dir);
    }

    /// A split corpus: one curated book (`core-rulebook`) written by the migration, format 3.
    fn split_corpus(name: &str) -> PathBuf {
        use cast_iron_schema::{BookIndex, BookNode, BookTree};
        let dir = temp_dir(name);
        let mut source = corpus(vec![
            rule("ref/rule/a", vec![paragraph(vec![Inline::text("a")])]),
            rule("ref/rule/b", vec![paragraph(vec![Inline::text("b")])]),
        ]);
        source.books = BookIndex {
            books: vec![BookTree {
                source: "core-rulebook".into(),
                title: "Core Rulebook".into(),
                basis: String::new(),
                review: vec![],
                chapters: vec![BookNode {
                    title: "Chapter 1".into(),
                    entry: Some(cast_iron_schema::EntryId::parse("ref/rule/a").unwrap()),
                    anchor: None,
                    review: None,
                    collapse: false,
                    lists_children: false,
                    children: vec![],
                }],
            }],
        };
        write_corpus(&dir, &source);
        crate::layout::migrate_to_split(&dir).unwrap();
        dir
    }

    #[test]
    fn a_split_corpus_loads_its_curated_books_and_derives_the_index() {
        let dir = split_corpus("split-load");
        let loaded = Corpus::load(&dir).unwrap();
        assert_eq!(loaded.layout, BookLayout::Split);
        assert_eq!(loaded.config.format, 3);
        assert_eq!(loaded.books.books.len(), 1);
        assert!(loaded.book_texts["core-rulebook"].contains("[[books]]"));
        let index = loaded.book_index();
        let titles: Vec<_> = index.books[0]
            .chapters
            .iter()
            .map(|c| c.title.as_str())
            .collect();
        // `b` is no chapter's entry, so it lands in the compiler's hidden chapter.
        assert_eq!(titles, ["Chapter 1", "New entries (page order)"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_two_layouts_never_mix_and_format_3_needs_the_split_one() {
        let dir = split_corpus("split-mixed");
        fs::write(dir.join("books.toml"), "").unwrap();
        let errors = Corpus::load(&dir).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("both `books.toml`")),
            "{errors:?}"
        );
        fs::remove_file(dir.join("books.toml")).unwrap();

        fs::write(dir.join("books/stray.txt"), "x").unwrap();
        let errors = Corpus::load(&dir).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("only `<publication>.toml`")),
            "{errors:?}"
        );
        fs::remove_file(dir.join("books/stray.txt")).unwrap();

        let text = fs::read_to_string(dir.join("books/core-rulebook.toml")).unwrap();
        fs::write(dir.join("books/other.toml"), &text).unwrap();
        let errors = Corpus::load(&dir).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("file name is the publication id")),
            "{errors:?}"
        );
        fs::remove_file(dir.join("books/other.toml")).unwrap();
        fs::write(dir.join("books/core-rulebook.toml"), "books = []\n").unwrap();
        let errors = Corpus::load(&dir).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("exactly one [[books]] block")),
            "{errors:?}"
        );
        let _ = fs::remove_dir_all(&dir);

        // Format 3 with a single books.toml is refused and names the command.
        let dir = temp_dir("single-format-3");
        let source = corpus(vec![rule("ref/rule/a", vec![])]);
        write_corpus(&dir, &source);
        fs::write(dir.join("corpus.toml"), write_config(&Config::default())).unwrap();
        let errors = Corpus::load(&dir).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("stores the curated books")),
            "{errors:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
