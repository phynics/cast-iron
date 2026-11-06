//! Reading a whole corpus directory.

use crate::entry::read_entry;
use crate::error::{Error, Result};
use crate::files::{
    Publication, read_aliases, read_books, read_coverage, read_legal, read_publications,
};
use cast_iron_schema::{AliasIndex, BookIndex, CoverageReport, LegalNotices, RuleEntry};
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
}

/// A corpus directory, read and parsed. The stored files keep their text so a
/// check can name the line a problem sits on.
#[derive(Debug, Clone)]
pub struct Corpus {
    pub dir: PathBuf,
    /// Entries by permanent id.
    pub entries: BTreeMap<String, Stored>,
    pub publications: Vec<Publication>,
    pub books: BookIndex,
    pub books_text: String,
    pub legal: LegalNotices,
    pub coverage: CoverageReport,
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

impl Corpus {
    /// Reads every file. Returns all the errors it finds, not only the first,
    /// so one run lists every file that needs a fix.
    pub fn load(dir: &Path) -> std::result::Result<Corpus, Vec<Error>> {
        let mut errors = Vec::new();
        let publications = take(
            &mut errors,
            read_text(dir, "publications.toml")
                .and_then(|text| read_publications(Path::new("publications.toml"), &text)),
        );
        let books_text = take(&mut errors, read_text(dir, "books.toml"));
        let books = books_text
            .as_deref()
            .and_then(|text| take(&mut errors, read_books(Path::new("books.toml"), text)));
        let legal = take(
            &mut errors,
            read_text(dir, "legal.toml")
                .and_then(|text| read_legal(Path::new("legal.toml"), &text)),
        );
        let coverage = take(
            &mut errors,
            read_text(dir, "coverage.toml")
                .and_then(|text| read_coverage(Path::new("coverage.toml"), &text)),
        );
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
            match read_entry(&relative, &text) {
                Ok(entry) => {
                    let id = entry.id.to_string();
                    let stored = Stored {
                        entry,
                        path: relative.clone(),
                        text,
                    };
                    if entries.insert(id.clone(), stored).is_some() {
                        errors.push(Error::new(&relative, 1, format!("duplicate entry id {id}")));
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        match (publications, books, books_text, legal, coverage, aliases) {
            (
                Some(publications),
                Some(books),
                Some(books_text),
                Some(legal),
                Some(coverage),
                Some(aliases),
            ) if errors.is_empty() => Ok(Corpus {
                dir: dir.to_path_buf(),
                entries,
                publications,
                books,
                books_text,
                legal,
                coverage,
                aliases,
            }),
            _ => Err(errors),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{entry_path, write_entry};
    use crate::files::{write_books, write_coverage, write_legal};
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
            fs::write(path, write_entry(&stored.entry)).unwrap();
        }
        fs::write(
            dir.join("publications.toml"),
            "[[publication]]\nid = \"core-rulebook\"\ntitle = \"Core Rulebook\"\n",
        )
        .unwrap();
        fs::write(dir.join("books.toml"), write_books(&source.books)).unwrap();
        fs::write(dir.join("legal.toml"), write_legal(&source.legal)).unwrap();
        fs::write(dir.join("coverage.toml"), write_coverage(&source.coverage)).unwrap();
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
}
