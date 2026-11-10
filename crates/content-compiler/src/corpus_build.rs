//! The library that the plain-text corpus describes.
//!
//! [`build_from_corpus`] reads a corpus directory (`docs/corpus-format.md`) and runs the stages
//! that derive everything the corpus does not store: relations, spell lists, coverage (from the
//! entries, `exclusions.toml`, and `history.toml`), search aliases and facets, and the alias
//! file. [`crate::pipeline::package`] then writes backlinks, the catalog, the search index, and
//! the manifest.
//!
//! The corpus is the source of truth.

use crate::corpus_coverage;
use crate::pipeline::{
    ALIASES_FILE, Failure, Library, Outcome, canonical_facets, check_no_markup_leaks, failure,
    package, search_aliases,
};
use crate::published;
use crate::relations;
use crate::{lists, text};
use cast_iron_corpus::Corpus;
use cast_iron_schema::{DatasetRef, RelationCoverage, RuleEntry, SourceInfo};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Reads the corpus at `dir` and derives the library. Every problem the corpus check finds is
/// returned together, so one run lists every file that needs a fix.
///
/// With `published` (`sources/published-ids.txt`), the build also fails when a listed id no
/// longer resolves to an entry, directly or through `aliases.toml`.
///
/// `corpus_tag` is the tag in `sources/corpus.toml`; it is display information for Settings and
/// never changes the content version.
pub fn build_from_corpus(
    dir: &Path,
    published: Option<&Path>,
    corpus_tag: Option<&str>,
) -> Result<Library, Failure> {
    let corpus = Corpus::load(dir).map_err(|errors| Failure {
        lines: std::iter::once(format!(
            "cannot read the corpus {}: {} error(s)",
            dir.display(),
            errors.len()
        ))
        .chain(errors.iter().take(50).map(ToString::to_string))
        .chain((errors.len() > 50).then(|| format!("... and {} more", errors.len() - 50)))
        .collect(),
    })?;
    let report = cast_iron_corpus::check(&corpus);
    if !report.problems.is_empty() {
        let mut lines = vec![format!(
            "the corpus check found {} problem(s):",
            report.problems.len()
        )];
        lines.extend(report.problems.iter().take(50).cloned());
        if report.problems.len() > 50 {
            lines.push(format!("... and {} more", report.problems.len() - 50));
        }
        return Err(Failure { lines });
    }
    if let Some(path) = published {
        check_published(path, &corpus)?;
    }
    let history = corpus.history.as_ref().ok_or_else(|| {
        failure(
            "the corpus has no history.toml; the coverage report needs the frozen import results",
        )
    })?;

    let mut entries: Vec<RuleEntry> = corpus
        .entries
        .values()
        .map(|stored| stored.entry.clone())
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    // The corpus has no spelling for a text-node boundary; the reader already merges them, and
    // this keeps the guarantee local to the build.
    for entry in &mut entries {
        text::normalize_text_nodes(&mut entry.body);
    }

    // `entry_count` is the number of entries whose primary publication is the source.
    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    for entry in &entries {
        if let Some(source) = entry.source_ids.first() {
            *counts.entry(source.as_str()).or_default() += 1;
        }
    }
    let mut sources: Vec<SourceInfo> = corpus
        .publications
        .iter()
        .map(|publication| {
            publication.to_source(counts.get(publication.id.as_str()).copied().unwrap_or(0))
        })
        .collect();
    sources.sort_by(|a, b| a.id.cmp(&b.id));

    let coverage = corpus_coverage::derive(
        &entries,
        &sources,
        &corpus.exclusions.excluded,
        &corpus.exclusions.rejected,
        history,
    )
    .map_err(failure)?;

    let spell_lists = lists::build(&entries);
    let relations = relations::index_over(&entries, RelationCoverage::default());
    let option_groups = crate::attached::option_groups(&corpus, &entries);
    let replaces = crate::attached::replaces(&entries);
    let aliases = search_aliases(&entries);
    let facets = canonical_facets(&entries);
    check_no_markup_leaks(&entries, &facets, &aliases, &coverage)?;

    let datasets: Vec<DatasetRef> = corpus
        .legal
        .datasets
        .iter()
        .map(|dataset| DatasetRef {
            id: dataset.id.clone(),
            repository: dataset.repository.clone(),
            commit: dataset.commit.clone(),
        })
        .collect();
    let aliases_file = corpus
        .aliases
        .as_ref()
        .map(|index| {
            // Pretty JSON with a trailing newline.
            let mut bytes = serde_json::to_vec_pretty(index).map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            Ok::<_, String>(bytes)
        })
        .transpose()
        .map_err(|e| failure(format!("cannot write {ALIASES_FILE}: {e}")))?;

    let notes = vec![format!(
        "corpus build: {} entries from {} publications; {} excluded rows, {} rejected rows; coverage derived from the entries, exclusions.toml, and history.toml",
        entries.len(),
        corpus.publications.len(),
        corpus.exclusions.excluded.len(),
        corpus.exclusions.rejected.len(),
    )];
    let mut books = corpus.books.clone();
    cast_iron_schema::books::mark_listed_children(&mut books, &entries);
    Ok(Library {
        entries,
        relations: relations.index,
        spell_lists: spell_lists.lists,
        books,
        option_groups,
        replaces,
        legal: corpus.legal.clone(),
        coverage,
        aliases,
        facets,
        datasets,
        corpus_tag: corpus_tag.map(str::to_string),
        sources,
        notes,
        aliases_file,
    })
}

/// Fails when an id in the published-id file no longer resolves in `corpus`.
fn check_published(path: &Path, corpus: &Corpus) -> Result<(), Failure> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        failure(format!(
            "cannot read the published ids {}: {e}",
            path.display()
        ))
    })?;
    let ids = published::parse(&text).map_err(|lines| Failure { lines })?;
    let entry_ids: BTreeSet<&str> = corpus.entries.keys().map(String::as_str).collect();
    let missing = published::unresolved(&ids, &entry_ids, corpus.aliases.as_ref());
    if missing.is_empty() {
        return Ok(());
    }
    let mut lines = vec![format!(
        "{} published id(s) no longer resolve; a bookmark may hold each of them:",
        missing.len()
    )];
    lines.extend(missing.iter().take(50).map(|line| format!("  {line}")));
    if missing.len() > 50 {
        lines.push(format!("  ... and {} more", missing.len() - 50));
    }
    Err(Failure { lines })
}

/// Builds the library from the corpus at `dir` and writes it to `out`, with reports in `reports`.
/// `published` is the optional published-id file (see [`build_from_corpus`]); `corpus_tag` is the
/// pinned corpus tag, also display information only.
pub fn compile_corpus(
    dir: &Path,
    out: &Path,
    reports: &Path,
    published: Option<&Path>,
    corpus_tag: Option<&str>,
) -> Result<Outcome, Failure> {
    package(build_from_corpus(dir, published, corpus_tag)?, out, reports)
}
