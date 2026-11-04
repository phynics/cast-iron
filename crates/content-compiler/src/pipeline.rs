//! The end-to-end compile: select, parse, plan, build, validate, emit, report.

use crate::build::{self, DedupeStats, RawRules, RawTyped};
use crate::config::{self, IdOverrides, SourceConfig};
use crate::family;
use crate::legal::{self, ExtractedNotices};
use crate::model::Node;
use crate::order::{BookOrderFile, UpstreamOrder};
use crate::select::{self, Role, hex, sha256_hex};
use crate::slug::slug_segment;
use crate::text::{blocks_text, inline_text};
use cast_iron_schema::validate::{ValidationError, validate_derived, validate_library};
use cast_iron_schema::{
    BOOKS_FILE, Catalog, ContentManifest, CoverageReport, DatasetRef, EntryKind, EntryStub,
    ManifestFile, PublicationCoverage, RELATIONS_FILE, Rejection, RuleEntry, SCHEMA_VERSION,
    SPELL_LISTS_FILE, SourceInfo,
};
use cast_iron_search::{DocInput, IndexBuilder};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const SHARD_TARGET_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone)]
pub struct Options {
    pub source_config: PathBuf,
    pub dataset: PathBuf,
    pub out: PathBuf,
    pub reports: PathBuf,
    pub write_digest: bool,
    pub strict: bool,
    pub overrides: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Outcome {
    pub content_version: String,
    pub entry_count: usize,
    pub by_kind: BTreeMap<String, u32>,
    pub total_bytes: u64,
    pub largest_shard: (String, u64),
    pub largest_entry: (String, u64),
    pub links: cast_iron_schema::LinkCoverage,
    pub rejected: Vec<Rejection>,
    pub stripped: BTreeMap<String, u32>,
    pub dedupe: DedupeStats,
    pub notes: usize,
    /// Relation, spell-list, and book-order results, as written to `coverage.json`.
    pub relations: cast_iron_schema::RelationCoverage,
    /// Size in bytes of `relations.json`, `lists.json`, and `books.json`.
    pub derived_bytes: BTreeMap<String, u64>,
}

/// A failed compile. `lines` is printed one per line.
#[derive(Debug)]
pub struct Failure {
    pub lines: Vec<String>,
}

impl From<String> for Failure {
    fn from(line: String) -> Self {
        Failure { lines: vec![line] }
    }
}

pub fn compile(opts: &Options) -> Result<Outcome, Failure> {
    let cfg = SourceConfig::load(&opts.source_config)?;
    let config_dir = opts.source_config.parent().unwrap_or(Path::new("."));
    let overrides = IdOverrides::load(
        &opts
            .overrides
            .clone()
            .unwrap_or_else(|| config_dir.join("id-overrides.toml")),
    )?;
    let notices_path = config_dir.join("legal-notices.json");
    let notices = ExtractedNotices::parse(
        &fs::read_to_string(&notices_path)
            .map_err(|e| format!("cannot read {}: {e}", notices_path.display()))?,
    )?;

    let selection = select::select(&opts.dataset, &cfg)?;
    check_digest(opts, &cfg, &selection.digest)?;
    let policy = crate::policy::ContentPolicy::load(&config_dir.join("content-policy.toml"))?;

    let book_order = BookOrderFile::load(&config_dir.join("book-order.toml"))?;
    let upstream_order = UpstreamOrder::load(&config_dir.join("upstream-order.json"))?;

    // Parse at the boundary.
    let mut rejected: Vec<Rejection> = Vec::new();
    let mut unpromoted: Vec<Rejection> = Vec::new();
    let mut typed = Vec::new();
    let mut rules = Vec::new();
    for file in &selection.files {
        match Node::parse_record(&file.bytes) {
            Err(reason) => rejected.push(Rejection {
                path: file.rel.clone(),
                reason,
            }),
            Ok(mut node) => {
                if let Some(book) = file
                    .rel
                    .split('/')
                    .next()
                    .and_then(|book| cfg.dataset.books.get(book))
                {
                    normalize_sources(&mut node, book);
                }
                normalize_class_sections(&mut node, &cfg.dataset.prestige_classes);
                if !policy.retain(&mut node, &file.rel, &mut rejected) {
                    continue;
                }
                match &file.role {
                    Role::Typed {
                        kind,
                        upstream_type,
                    } => {
                        if node.kind.as_deref() != Some(*upstream_type) {
                            rejected.push(Rejection {
                                path: file.rel.clone(),
                                reason: format!(
                                    "record type {:?} does not match the {} directory",
                                    node.kind.as_deref().unwrap_or_default(),
                                    upstream_type
                                ),
                            });
                        } else if *kind == EntryKind::Archetype
                            && !is_archetype_url(node.url.as_deref().unwrap_or_default())
                        {
                            // Upstream types some class text as an archetype. It is not one,
                            // so it stays ordinary text in the tree that holds it.
                            unpromoted.push(Rejection {
                                path: file.rel.clone(),
                                reason: format!(
                                    "typed class_archetype upstream, but {:?} is not below an `... Archetypes` section; kept as text in its class",
                                    node.url.as_deref().unwrap_or_default()
                                ),
                            });
                        } else {
                            typed.push(RawTyped {
                                rel: file.rel.clone(),
                                kind: *kind,
                                node,
                            });
                        }
                    }
                    Role::Rules => rules.push(RawRules {
                        rel: file.rel.clone(),
                        node,
                    }),
                }
            }
        }
    }

    // Some typed records, notably Technology Guide's prestige class, exist
    // only inside rules trees. Promote valid typed nodes at this boundary.
    let mut known_urls: BTreeSet<String> = typed
        .iter()
        .filter_map(|raw| raw.node.url.clone())
        .collect();
    for tree in &rules {
        collect_typed_nodes(&tree.node, &tree.rel, &mut known_urls, &mut typed);
    }

    let selected: BTreeSet<String> = selection.files.iter().map(|f| f.rel.clone()).collect();
    let survey = select::survey(&opts.dataset, &cfg, &selected)?;
    let plan =
        build::plan(typed, rules, &survey, &overrides, &cfg.dataset.id).map_err(|lines| {
            let mut all = vec![format!("{} id problem(s):", lines.len())];
            all.extend(lines);
            Failure { lines: all }
        })?;
    rejected.extend(plan.rejected.iter().cloned());
    rejected.sort_by(|a, b| a.path.cmp(&b.path));
    if opts.strict && !rejected.is_empty() {
        let mut lines = vec![format!("--strict: {} rejected record(s):", rejected.len())];
        lines.extend(
            rejected
                .iter()
                .map(|r| format!("  {}: {}", r.path, r.reason)),
        );
        return Err(Failure { lines });
    }

    let approved = cfg
        .supplement
        .as_ref()
        .map(|config| crate::pf_approved::load(config, config_dir))
        .transpose()?;
    let input_digest = if let Some(extra) = &approved {
        sha256_hex(format!("{}\n{}\n", selection.digest, extra.input_digest).as_bytes())
    } else {
        selection.digest.clone()
    };
    let mut built = build::build(&plan);
    if let Some(extra) = &approved {
        let ids: BTreeSet<_> = built.entries.iter().map(|e| e.id.clone()).collect();
        for entry in &extra.entries {
            if ids.contains(&entry.id) {
                return Err(format!(
                    "approved source ID collides with published entry: {}",
                    entry.id
                )
                .into());
            }
            if entry
                .source_ids
                .iter()
                .any(|id| !built.source_titles.contains_key(id))
            {
                return Err(format!(
                    "approved entry {} requires a separately reviewed publication",
                    entry.id
                )
                .into());
            }
            if let Some(parent) = &entry.parent {
                let parent_entry = built
                    .entries
                    .iter_mut()
                    .find(|e| &e.id == parent)
                    .ok_or_else(|| {
                        format!("approved entry {} has unknown parent {parent}", entry.id)
                    })?;
                parent_entry.body.insert(
                    0,
                    cast_iron_schema::Block::EntryLink {
                        target: entry.id.clone(),
                        title: entry.title.clone(),
                    },
                );
                parent_entry.links.push(cast_iron_schema::EntryReference {
                    target: entry.id.clone(),
                    anchor: None,
                });
                parent_entry.links.sort();
                parent_entry.links.dedup();
                built.nested.insert(
                    0,
                    build::NestedLink {
                        host: parent.clone(),
                        target: entry.id.clone(),
                        trail: Vec::new(),
                        child: true,
                    },
                );
            }
            built.entries.push(entry.clone());
        }
        built.entries.sort_by(|a, b| a.id.cmp(&b.id));
    }
    let relations = crate::relations::apply(&mut built.entries, &plan, &built.nested);
    let spell_lists = crate::lists::build(&built.entries);
    let books = crate::books::build(
        &built.entries,
        &built.nested,
        book_order.as_ref(),
        &upstream_order,
    );
    let mut legal = legal::build(&cfg.dataset, &notices)?;
    if let Some(extra) = &approved {
        legal.datasets.extend(extra.notices.clone());
        for notice in &extra.copyrights {
            if !legal.section_15.contains(notice) {
                legal.section_15.push(notice.clone());
            }
        }
    }
    let facets_by_id: BTreeMap<_, _> = plan
        .typed
        .iter()
        .map(|t| (t.id.clone(), t.extracted.facets.clone()))
        .collect();

    // Shards and catalog.
    let (shard_files, shard_of, largest_entry) = shard(&built.entries)?;
    let mut sources = Vec::new();
    for (slug, raws) in &built.source_titles {
        let title = raws.iter().next().cloned().unwrap_or_default();
        sources.push(SourceInfo {
            id: slug.clone(),
            entry_count: built
                .entries
                .iter()
                .filter(|e| e.source_ids[0] == *slug)
                .count() as u32,
            copyright: legal::copyright_for(&title, &notices.section_15),
            title,
        });
    }
    sources.sort_by(|a, b| a.id.cmp(&b.id));
    let catalog = Catalog {
        sources,
        entries: built
            .entries
            .iter()
            .map(|e| EntryStub {
                id: e.id.clone(),
                kind: e.kind,
                title: e.title.clone(),
                source: e.source_ids[0].clone(),
                parent: e.parent.clone(),
                shard: shard_of[&e.id].clone(),
            })
            .collect(),
    };

    let mut errors = validate_library(&built.entries, &catalog);
    errors.extend(validate_derived(
        &built.entries,
        &relations.index,
        &books.index,
        &spell_lists.lists,
    ));
    let mut by_kind: BTreeMap<String, u32> = BTreeMap::new();
    for e in &built.entries {
        *by_kind.entry(e.kind.slug().to_string()).or_default() += 1;
    }

    let mut coverage = CoverageReport {
        input_digest,
        by_publication: publications(&survey.source_counts, &catalog),
        by_kind: by_kind.clone(),
        excluded: survey.exclusions.clone(),
        rejected: rejected.clone(),
        links: built.links.clone(),
        stripped: built.stripped.clone(),
        relations: cast_iron_schema::RelationCoverage {
            spell_lists: spell_lists.coverage.clone(),
            unpromoted: unpromoted.clone(),
            book_order: books.coverage.clone(),
            ..relations.coverage.clone()
        },
    };
    if let Some(extra) = &approved {
        coverage.excluded.extend(extra.exclusions.clone());
    }

    write_reports(
        &opts.reports,
        &errors,
        &coverage,
        &built.notes,
        &built.dedupe,
        &survey,
        &catalog,
    )?;
    if !errors.is_empty() {
        let mut lines = vec![format!("validation failed with {} error(s):", errors.len())];
        for e in errors.iter().take(50) {
            lines.push(format!("  {}: {}", e.entry, e.problem));
        }
        if errors.len() > 50 {
            lines.push(format!("  ... and {} more", errors.len() - 50));
        }
        return Err(Failure { lines });
    }

    // Search index.
    let mut index = IndexBuilder::new();
    for e in &built.entries {
        let mut aliases: BTreeSet<String> = overrides
            .aliases
            .get(e.id.as_str())
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        // A lookup such as "grapple" names a section inside Special Attacks. Use the
        // existing alias field so that these headings rank above incidental body mentions.
        if e.kind == EntryKind::Rule {
            for block in &e.body {
                if let cast_iron_schema::Block::Heading { content, .. } = block {
                    let heading = inline_text(content);
                    if !heading.is_empty() && heading != e.title {
                        aliases.insert(heading);
                    }
                }
            }
        }
        index.add(DocInput {
            id: e.id.to_string(),
            title: e.title.clone(),
            aliases: aliases.into_iter().collect(),
            kind: e.kind.slug().to_string(),
            source: e.source_ids[0].clone(),
            section: e.section.join(" > "),
            facets: facets_by_id.get(&e.id).cloned().unwrap_or_default(),
            text: blocks_text(&e.body),
        });
    }
    let search_bin = index.build().to_bytes();

    // Files, manifest, version.
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    files.insert("catalog.json".into(), json_bytes(&catalog)?);
    files.insert("legal.json".into(), json_bytes(&legal)?);
    files.insert("coverage.json".into(), json_bytes(&coverage)?);
    files.insert("search.bin".into(), search_bin);
    let mut derived_bytes = BTreeMap::new();
    for (name, bytes) in [
        (RELATIONS_FILE, json_bytes(&relations.index)?),
        (SPELL_LISTS_FILE, json_bytes(&spell_lists.lists)?),
        (BOOKS_FILE, json_bytes(&books.index)?),
    ] {
        derived_bytes.insert(name.to_string(), bytes.len() as u64);
        files.insert(name.into(), bytes);
    }
    for (path, bytes) in shard_files {
        files.insert(path, bytes);
    }
    let hashed: Vec<(String, String, u64)> = files
        .iter()
        .map(|(p, b)| (p.clone(), sha256_hex(b), b.len() as u64))
        .collect();
    let version_lines: String = hashed
        .iter()
        .map(|(p, h, _)| format!("{p}:{h}\n"))
        .collect();
    let content_version = format!("v{}", &hex(&Sha256::digest(version_lines.as_bytes()))[..16]);
    let manifest = ContentManifest {
        schema_version: SCHEMA_VERSION,
        content_version: content_version.clone(),
        total_bytes: hashed.iter().map(|(_, _, n)| n).sum(),
        files: hashed
            .iter()
            .map(|(p, h, n)| ManifestFile {
                path: format!("{content_version}/{p}"),
                sha256: h.clone(),
                bytes: *n,
            })
            .collect(),
        entry_count: built.entries.len() as u32,
        datasets: std::iter::once(DatasetRef {
            id: cfg.dataset.id.clone(),
            repository: cfg.dataset.repository.clone(),
            commit: cfg.dataset.commit.clone(),
        })
        .chain(
            approved
                .as_ref()
                .into_iter()
                .flat_map(|extra| extra.datasets.clone()),
        )
        .collect(),
    };

    prepare_out(&opts.out)?;
    for (path, bytes) in &files {
        let target = opts.out.join(&content_version).join(path);
        write_file(&target, bytes)?;
    }
    write_file(&opts.out.join("manifest.json"), &json_bytes(&manifest)?)?;

    let largest_shard = manifest
        .files
        .iter()
        .filter(|f| f.path.contains("/entries/"))
        .max_by(|a, b| a.bytes.cmp(&b.bytes).then(b.path.cmp(&a.path)))
        .map(|f| (f.path.clone(), f.bytes))
        .unwrap_or_default();
    Ok(Outcome {
        content_version,
        entry_count: built.entries.len(),
        by_kind,
        total_bytes: manifest.total_bytes,
        largest_shard,
        largest_entry,
        links: built.links,
        rejected,
        stripped: built.stripped,
        dedupe: built.dedupe,
        notes: built.notes.len(),
        relations: coverage.relations,
        derived_bytes,
    })
}

/// Archetypes live in a section whose title ends in `Archetypes`, such as
/// `.../Barbarian/Barbarian Archetypes/Breaker`.
fn is_archetype_url(url: &str) -> bool {
    let segments: Vec<&str> = url.split('/').collect();
    segments
        .len()
        .checked_sub(2)
        .and_then(|i| segments.get(i))
        .is_some_and(|parent| parent.to_lowercase().ends_with("archetypes"))
}

fn normalize_sources(node: &mut Node, inherited: &str) {
    if node
        .source
        .as_deref()
        .is_none_or(|source| source == "PFSRD" || source.is_empty())
    {
        node.source = Some(inherited.to_string());
    }
    let source = node.source.as_deref().unwrap_or(inherited);
    for child in &mut node.sections {
        normalize_sources(child, source);
    }
}

fn normalize_class_sections(node: &mut Node, urls: &[String]) {
    if node.url.as_ref().is_some_and(|url| urls.contains(url)) {
        node.kind = Some("class".into());
        node.subtype = Some("prestige".into());
    }
    for child in &mut node.sections {
        normalize_class_sections(child, urls);
    }
}

fn collect_typed_nodes(
    node: &Node,
    path: &str,
    known: &mut BTreeSet<String>,
    out: &mut Vec<RawTyped>,
) {
    if let Some(kind) = node.kind.as_deref().and_then(family::promotable_from_tree)
        && let Some(url) = &node.url
        && node.name.as_deref().is_some_and(|name| !name.is_empty())
        && known.insert(url.clone())
    {
        out.push(RawTyped {
            rel: path.to_string(),
            kind,
            node: node.clone(),
        });
    }
    for child in &node.sections {
        collect_typed_nodes(child, path, known, out);
    }
}

fn check_digest(opts: &Options, cfg: &SourceConfig, computed: &str) -> Result<(), Failure> {
    if opts.write_digest {
        let text = fs::read_to_string(&opts.source_config)
            .map_err(|e| format!("cannot read {}: {e}", opts.source_config.display()))?;
        let updated = config::rewrite_digest(&text, computed)?;
        if updated != text {
            fs::write(&opts.source_config, updated)
                .map_err(|e| format!("cannot write {}: {e}", opts.source_config.display()))?;
        }
        return Ok(());
    }
    if cfg.dataset.input_digest.is_empty() {
        return Err(format!(
            "input_digest in {} is empty. The selected inputs hash to {computed}. Review the inputs, then run again with --write-digest.",
            opts.source_config.display()
        )
        .into());
    }
    if cfg.dataset.input_digest != computed {
        return Err(Failure {
            lines: vec![
                "input digest mismatch: the selected dataset files differ from the pinned set.".into(),
                format!("  source.toml pins {}", cfg.dataset.input_digest),
                format!("  the files hash to {computed}"),
                "  Check that the dataset matches the pinned commit. If the change is intended, run again with --write-digest.".into(),
            ],
        });
    }
    Ok(())
}

fn publications(
    source_counts: &BTreeMap<String, u32>,
    catalog: &Catalog,
) -> Vec<PublicationCoverage> {
    let mut by_slug: BTreeMap<String, (String, u32, u32)> = BTreeMap::new();
    for (raw, count) in source_counts {
        let slug = slug_segment(raw);
        let entry = by_slug.entry(slug).or_insert_with(|| (raw.clone(), 0, 0));
        entry.1 += count;
    }
    for s in &catalog.sources {
        let entry = by_slug
            .entry(s.id.clone())
            .or_insert_with(|| (s.title.clone(), 0, 0));
        entry.2 = s.entry_count;
    }
    by_slug
        .into_iter()
        .map(
            |(source_id, (title, upstream_records, included_entries))| PublicationCoverage {
                source_id,
                title,
                upstream_records,
                included_entries,
            },
        )
        .collect()
}

type Shards = (
    Vec<(String, Vec<u8>)>,
    BTreeMap<cast_iron_schema::EntryId, String>,
    (String, u64),
);

/// Packs entries of each kind into shards of at most about 512 KB. Entries are already sorted
/// by id. A single entry larger than the target gets a shard of its own.
fn shard(entries: &[RuleEntry]) -> Result<Shards, String> {
    let mut files = Vec::new();
    let mut shard_of = BTreeMap::new();
    let mut largest = (String::new(), 0u64);
    for kind in EntryKind::ALL {
        let mut current: Vec<Vec<u8>> = Vec::new();
        let mut ids = Vec::new();
        let mut size = 2usize;
        let mut number = 0usize;
        let mut flush = |current: &mut Vec<Vec<u8>>,
                         ids: &mut Vec<cast_iron_schema::EntryId>,
                         size: &mut usize| {
            if current.is_empty() {
                return;
            }
            let path = format!("entries/{}-{number:03}.json", kind.slug());
            let mut bytes = Vec::with_capacity(*size);
            bytes.push(b'[');
            for (i, e) in current.iter().enumerate() {
                if i > 0 {
                    bytes.push(b',');
                }
                bytes.extend_from_slice(e);
            }
            bytes.push(b']');
            for id in ids.drain(..) {
                shard_of.insert(id, path.clone());
            }
            files.push((path, bytes));
            current.clear();
            *size = 2;
            number += 1;
        };
        for entry in entries.iter().filter(|e| e.kind == kind) {
            let bytes = serde_json::to_vec(entry).map_err(|e| e.to_string())?;
            if bytes.len() as u64 > largest.1 {
                largest = (entry.id.to_string(), bytes.len() as u64);
            }
            if !current.is_empty() && size + bytes.len() + 1 > SHARD_TARGET_BYTES {
                flush(&mut current, &mut ids, &mut size);
            }
            size += bytes.len() + 1;
            ids.push(entry.id.clone());
            current.push(bytes);
        }
        flush(&mut current, &mut ids, &mut size);
    }
    Ok((files, shard_of, largest))
}

fn json_bytes<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| e.to_string())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Empties the output directory, but only if it is empty or holds one of our manifests.
pub fn prepare_out(out: &Path) -> Result<(), String> {
    match fs::symlink_metadata(out) {
        Err(_) => {
            return fs::create_dir_all(out)
                .map_err(|e| format!("cannot create {}: {e}", out.display()));
        }
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(format!(
                "refusing to clean {}: it is a symbolic link",
                out.display()
            ));
        }
        Ok(meta) if !meta.is_dir() => {
            return Err(format!("{} exists and is not a directory", out.display()));
        }
        Ok(_) => {}
    }
    let empty = fs::read_dir(out)
        .map_err(|e| format!("cannot read {}: {e}", out.display()))?
        .next()
        .is_none();
    if !empty {
        let manifest = out.join("manifest.json");
        let ours = fs::read(&manifest)
            .ok()
            .and_then(|b| serde_json::from_slice::<ContentManifest>(&b).ok())
            .is_some();
        if !ours {
            return Err(format!(
                "refusing to clean {}: it is not empty and holds no Cast Iron manifest.json",
                out.display()
            ));
        }
    }
    fs::remove_dir_all(out).map_err(|e| format!("cannot clean {}: {e}", out.display()))?;
    fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))
}

#[allow(clippy::too_many_arguments)]
fn write_reports(
    dir: &Path,
    errors: &[ValidationError],
    coverage: &CoverageReport,
    notes: &[String],
    dedupe: &DedupeStats,
    survey: &select::Survey,
    catalog: &Catalog,
) -> Result<(), String> {
    let validation = serde_json::json!({
        "ok": errors.is_empty(),
        "error_count": errors.len(),
        "errors": errors,
        "entry_count": catalog.entries.len(),
        "rejected": coverage.rejected,
        "notes": notes,
        "dedupe": {
            "typed_duplicates_identical": dedupe.typed_identical,
            "typed_duplicates_conflicting": dedupe.typed_conflicting,
            "nested_copies_replaced_by_link_identical": dedupe.nested_replaced_identical,
            "nested_copies_replaced_by_link_conflicting": dedupe.nested_replaced_conflicting,
            "rules_children_repeated_identical": dedupe.rules_children_identical,
            "rules_children_repeated_conflicting": dedupe.rules_children_conflicting,
        },
        "unparsable_upstream_files": survey.unparsable,
    });
    let pretty = serde_json::to_string_pretty(&validation).map_err(|e| e.to_string())?;
    write_file(
        &dir.join("validation.json"),
        format!("{pretty}\n").as_bytes(),
    )?;
    write_file(
        &dir.join("coverage.md"),
        coverage_markdown(coverage, notes, dedupe, survey).as_bytes(),
    )
}

fn coverage_markdown(
    c: &CoverageReport,
    notes: &[String],
    dedupe: &DedupeStats,
    survey: &select::Survey,
) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "# Coverage report\n");
    let _ = writeln!(s, "Input digest: `{}`\n", c.input_digest);
    let _ = writeln!(s, "## Entries by kind\n\n| Kind | Entries |\n|---|---:|");
    for (k, n) in &c.by_kind {
        let _ = writeln!(s, "| {k} | {n} |");
    }
    let _ = writeln!(
        s,
        "\n## Publications\n\n| Source | Title | Upstream records | Included entries |\n|---|---|---:|---:|"
    );
    for p in &c.by_publication {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            p.source_id, p.title, p.upstream_records, p.included_entries
        );
    }
    let _ = writeln!(s, "\n## Rejected records ({})\n", c.rejected.len());
    if c.rejected.is_empty() {
        let _ = writeln!(s, "None.");
    }
    for r in &c.rejected {
        let _ = writeln!(s, "- `{}`: {}", r.path, r.reason);
    }
    let _ = writeln!(s, "\n## Links\n");
    let _ = writeln!(
        s,
        "- resolved: {}\n- out of selection: {}\n- broken: {}",
        c.links.resolved, c.links.out_of_selection, c.links.broken
    );
    let _ = writeln!(s, "\n## Duplicates\n");
    let _ = writeln!(
        s,
        "- typed records repeated, identical: {}",
        dedupe.typed_identical
    );
    let _ = writeln!(
        s,
        "- typed records repeated, conflicting: {}",
        dedupe.typed_conflicting
    );
    let _ = writeln!(
        s,
        "- rules-tree copies replaced by an entry link, identical: {}",
        dedupe.nested_replaced_identical
    );
    let _ = writeln!(
        s,
        "- rules-tree copies replaced by an entry link, conflicting: {}",
        dedupe.nested_replaced_conflicting
    );
    let _ = writeln!(
        s,
        "- rules sections repeated across selected files, identical: {}",
        dedupe.rules_children_identical
    );
    let _ = writeln!(
        s,
        "- rules sections repeated across selected files, conflicting: {}",
        dedupe.rules_children_conflicting
    );
    let _ = writeln!(
        s,
        "\n## Stripped or unwrapped content\n\n| What | Count |\n|---|---:|"
    );
    for (k, n) in &c.stripped {
        let _ = writeln!(s, "| {k} | {n} |");
    }
    let _ = writeln!(s, "\n## Notes ({})\n", notes.len());
    for n in notes.iter().take(50) {
        let _ = writeln!(s, "- {n}");
    }
    if notes.len() > 50 {
        let _ = writeln!(s, "- ... {} more in validation.json", notes.len() - 50);
    }
    let _ = writeln!(s, "\n## Excluded upstream directories\n");
    let _ = writeln!(
        s,
        "The dataset holds {} JSON files in total.\n",
        survey.total_files
    );
    let _ = writeln!(s, "| Path | Records | Reason |\n|---|---:|---|");
    for e in &c.excluded {
        let _ = writeln!(s, "| {} | {} | {} |", e.path, e.records, e.reason);
    }
    if !survey.unparsable.is_empty() {
        let _ = writeln!(s, "\n## Unparsable upstream files outside the selection\n");
        for p in &survey.unparsable {
            let _ = writeln!(s, "- {p}");
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_out_refuses_foreign_directories() {
        let dir = std::env::temp_dir().join(format!("ci-prepare-out-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        prepare_out(&dir).unwrap(); // creates
        fs::write(dir.join("keep.txt"), "x").unwrap();
        let err = prepare_out(&dir).unwrap_err();
        assert!(err.contains("refusing"), "{err}");
        assert!(dir.join("keep.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod shard_tests {
    use super::*;
    use cast_iron_schema::{Block, EntryId, Inline, Provenance, TypedFields};

    fn entry(id: &str, kind: EntryKind, bytes: usize) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind,
            title: id.into(),
            source_ids: vec!["s".into()],
            section: vec![],
            parent: None,
            body: vec![Block::Paragraph {
                style: Default::default(),
                content: vec![Inline::text("x".repeat(bytes))],
            }],
            fields: TypedFields::None,
            tags: vec![],
            links: vec![],
            provenance: vec![Provenance {
                dataset: "d".into(),
                record: "r".into(),
                path: "p".into(),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: vec![],
        }
    }

    #[test]
    fn shards_stay_under_target_and_keep_id_order() {
        let entries: Vec<RuleEntry> = (0..10)
            .map(|i| entry(&format!("a/e{i}"), EntryKind::Rule, 200_000))
            .chain([entry("a/spell", EntryKind::Spell, 10)])
            .collect();
        let (files, shard_of, largest) = shard(&entries).unwrap();
        let names: Vec<_> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            names,
            [
                "entries/rule-000.json",
                "entries/rule-001.json",
                "entries/rule-002.json",
                "entries/rule-003.json",
                "entries/rule-004.json",
                "entries/spell-000.json"
            ]
        );
        assert!(files.iter().all(|(_, b)| b.len() <= SHARD_TARGET_BYTES));
        assert_eq!(
            shard_of[&EntryId::parse("a/e0").unwrap()],
            "entries/rule-000.json"
        );
        assert_eq!(
            shard_of[&EntryId::parse("a/e9").unwrap()],
            "entries/rule-004.json"
        );
        assert!(largest.1 > 200_000);
        let parsed: Vec<RuleEntry> = serde_json::from_slice(&files[0].1).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn a_single_oversized_entry_gets_its_own_shard() {
        let entries = vec![
            entry("a/big", EntryKind::Rule, 700_000),
            entry("a/small", EntryKind::Rule, 10),
        ];
        let (files, _, _) = shard(&entries).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].1.len() > SHARD_TARGET_BYTES);
    }
}
