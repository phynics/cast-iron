//! The library that the plain-text corpus describes.
//!
//! [`build_from_corpus`] reads a corpus directory (`docs/corpus-format.md`) and runs the stages
//! that derive everything the corpus does not store: relations, spell lists, coverage (from the
//! entries, `exclusions.toml`, and `history.toml`), search aliases and facets, and the alias
//! file. [`crate::pipeline::package`] then writes backlinks, the catalog, the search index, and
//! the manifest, exactly as it does for the upstream builders.
//!
//! The corpus is the source of truth. This path reads no upstream dataset.

use crate::alias::ALIASES_FILE;
use crate::build::DedupeStats;
use crate::corpus_coverage;
use crate::pipeline::{
    Failure, Library, Outcome, canonical_facets, check_no_markup_leaks, failure, package,
    search_aliases,
};
use crate::relations;
use crate::select::Survey;
use crate::{lists, text};
use cast_iron_corpus::Corpus;
use cast_iron_schema::{DatasetRef, RelationCoverage, RuleEntry, SourceInfo};
use std::collections::BTreeMap;
use std::path::Path;

/// Reads the corpus at `dir` and derives the library. Every problem the corpus check finds is
/// returned together, so one run lists every file that needs a fix.
pub fn build_from_corpus(dir: &Path) -> Result<Library, Failure> {
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
            // Same bytes as `alias::to_bytes`: pretty JSON with a trailing newline.
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
    Ok(Library {
        entries,
        relations: relations.index,
        spell_lists: spell_lists.lists,
        books: corpus.books.clone(),
        legal: corpus.legal.clone(),
        coverage,
        aliases,
        facets,
        datasets,
        sources,
        notes,
        dedupe: DedupeStats::default(),
        survey: Survey::default(),
        supplement_prefix: None,
        aliases_file,
        link_pass: None,
    })
}

/// Builds the library from the corpus at `dir` and writes it to `out`, with reports in `reports`.
pub fn compile_corpus(dir: &Path, out: &Path, reports: &Path) -> Result<Outcome, Failure> {
    package(build_from_corpus(dir)?, out, reports)
}
