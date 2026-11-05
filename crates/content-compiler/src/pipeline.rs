//! The end-to-end compile: build one source into a `Library`, then package it.
//!
//! [`build_psrd`] holds everything that is specific to the PSRD-Data source. [`package`] is the
//! source-independent tail: it validates, shards, writes reports, builds the search index, and
//! publishes a version directory and manifest.

use crate::alias::{self, LegacySnapshot};
use crate::build::{self, DedupeStats, RawRules, RawTyped};
use crate::canonical;
use crate::carry;
use crate::config::{self, IdOverrides, SourceConfig};
use crate::family;
use crate::legal::{self, ExtractedNotices};
use crate::model::Node;
use crate::order::{BookOrderFile, UpstreamOrder};
use crate::pf_data::{self, Inventory};
use crate::pf_legal;
use crate::publications::{self, Registry};
use crate::select::{self, Role, Survey, hex, sha256_hex};
use crate::slug::slug_segment;
use crate::text::{blocks_text, inline_text};
use cast_iron_schema::validate::{ValidationError, validate_derived, validate_library};
use cast_iron_schema::{
    BACKLINKS_DIR, BACKLINKS_FILE, BOOKS_FILE, Block, BookIndex, Catalog, ContentManifest,
    CopyrightNotice, CoverageReport, DatasetNotice, DatasetRef, EntryId, EntryKind, EntryStub,
    Exclusion, Inline, LegalNotices, ManifestFile, ParagraphStyle, PublicationCoverage,
    RELATIONS_FILE, Rejection, RelationIndex, RelationKind, RuleEntry, SCHEMA_VERSION,
    SPELL_LISTS_FILE, SourceInfo, SpellLists, StatRow, Table, TableCell, TableRow, TypedFields,
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

/// Inputs for the canonical (Pf Data plus PSRD carry-over) library.
///
/// The PSRD options still drive [`build_psrd`] because the PSRD snapshot is the
/// carry-over input, the source of the baseline Section 15 notices, and the
/// reviewed book titles. They are not the content being compiled.
#[derive(Debug, Clone)]
pub struct CanonicalOptions {
    pub psrd: Options,
    pub pf_dataset: PathBuf,
    pub pf_registry: PathBuf,
    pub canonical_dir: PathBuf,
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
    /// Reverse body-link coverage, as written to `coverage.json`.
    pub backlinks: cast_iron_schema::BacklinkCoverage,
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

/// The source-independent content that [`package`] writes.
///
/// A source builder fills every field, including the search aliases and facets, so that the
/// package step needs no knowledge of where the records came from.
pub struct Library {
    /// Entries, sorted by id.
    pub entries: Vec<RuleEntry>,
    pub relations: RelationIndex,
    pub spell_lists: SpellLists,
    pub books: BookIndex,
    pub legal: LegalNotices,
    /// The coverage report, as written to `coverage.json`.
    pub coverage: CoverageReport,
    /// Search aliases per entry, already sorted and deduplicated.
    pub aliases: BTreeMap<EntryId, Vec<String>>,
    /// Search facets per entry.
    pub facets: BTreeMap<EntryId, Vec<(String, String)>>,
    pub datasets: Vec<DatasetRef>,
    pub sources: Vec<SourceInfo>,
    pub notes: Vec<String>,
    pub dedupe: DedupeStats,
    /// Upstream survey, for the report totals and unparsable list.
    pub survey: Survey,
    /// Namespace that marks supplemental exclusions in the coverage report.
    pub supplement_prefix: Option<String>,
    /// The generated legacy-to-canonical map, when the builder produces one.
    /// [`package`] adds it to the version directory and the manifest so the
    /// content version covers its bytes; it is never appended after publication.
    pub aliases_file: Option<Vec<u8>>,
}

/// Builds the PSRD-Data library: select, parse, plan, build, and derive.
pub fn build_psrd(opts: &Options) -> Result<Library, Failure> {
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
        // Group by parent so multiple approved records keep a stable order instead of
        // reversing each time a link is prepended to the parent body.
        let mut children: BTreeMap<
            cast_iron_schema::EntryId,
            Vec<(cast_iron_schema::EntryId, String)>,
        > = BTreeMap::new();
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
                if !built.entries.iter().any(|e| &e.id == parent) {
                    return Err(
                        format!("approved entry {} has unknown parent {parent}", entry.id).into(),
                    );
                }
                children
                    .entry(parent.clone())
                    .or_default()
                    .push((entry.id.clone(), entry.title.clone()));
            }
            built.entries.push(entry.clone());
        }
        for (parent, links) in children {
            let parent_entry = built
                .entries
                .iter_mut()
                .find(|e| e.id == parent)
                .expect("parent checked above");
            let mut blocks: Vec<cast_iron_schema::Block> = links
                .iter()
                .map(|(target, title)| cast_iron_schema::Block::EntryLink {
                    target: target.clone(),
                    title: title.clone(),
                })
                .collect();
            blocks.append(&mut parent_entry.body);
            parent_entry.body = blocks;
            for (target, _) in &links {
                if !parent_entry.links.iter().any(|link| &link.target == target) {
                    parent_entry.links.push(cast_iron_schema::EntryReference {
                        target: target.clone(),
                        anchor: None,
                    });
                    built.links.resolved += 1;
                }
            }
            parent_entry.links.sort();
            parent_entry.links.dedup();
            for (target, _) in links {
                built.nested.push(build::NestedLink {
                    host: parent.clone(),
                    target,
                    trail: Vec::new(),
                    child: true,
                });
            }
        }
        // Links inside the approved records were validated during conversion.
        built.links.resolved += extra
            .entries
            .iter()
            .map(|entry| entry.links.len() as u32)
            .sum::<u32>();
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
    let sources = build_sources(&built, &notices);
    let by_publication = publications(&survey.source_counts, &sources);
    let mut by_kind: BTreeMap<String, u32> = BTreeMap::new();
    for e in &built.entries {
        *by_kind.entry(e.kind.slug().to_string()).or_default() += 1;
    }
    let facets: BTreeMap<EntryId, Vec<(String, String)>> = plan
        .typed
        .iter()
        .map(|t| (t.id.clone(), t.extracted.facets.clone()))
        .collect();
    // A lookup such as "grapple" names a section inside Special Attacks. Use the existing
    // alias field so that these headings rank above incidental body mentions.
    let aliases: BTreeMap<EntryId, Vec<String>> = built
        .entries
        .iter()
        .map(|e| {
            let mut names: BTreeSet<String> = overrides
                .aliases
                .get(e.id.as_str())
                .into_iter()
                .flatten()
                .cloned()
                .collect();
            if e.kind == EntryKind::Rule {
                for block in &e.body {
                    if let Block::Heading { content, .. } = block {
                        let heading = inline_text(content);
                        if !heading.is_empty() && heading != e.title {
                            names.insert(heading);
                        }
                    }
                }
            }
            (e.id.clone(), names.into_iter().collect())
        })
        .collect();

    let mut coverage = CoverageReport {
        input_digest,
        by_publication,
        by_kind,
        excluded: survey.exclusions.clone(),
        rejected,
        links: built.links.clone(),
        stripped: built.stripped.clone(),
        relations: cast_iron_schema::RelationCoverage {
            spell_lists: spell_lists.coverage.clone(),
            unpromoted,
            book_order: books.coverage.clone(),
            ..relations.coverage.clone()
        },
        backlinks: Default::default(),
    };
    let supplement_prefix = approved
        .as_ref()
        .map(|_| crate::pf_approved::COVERAGE_PREFIX.to_string());
    let datasets: Vec<DatasetRef> = std::iter::once(DatasetRef {
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
    .collect();
    if let Some(extra) = &approved {
        coverage.excluded.extend(extra.exclusions.clone());
    }

    Ok(Library {
        entries: built.entries,
        relations: relations.index,
        spell_lists: spell_lists.lists,
        books: books.index,
        legal,
        coverage,
        aliases,
        facets,
        datasets,
        sources,
        notes: built.notes,
        dedupe: built.dedupe,
        survey,
        supplement_prefix,
        aliases_file: None,
    })
}

fn build_sources(built: &build::Built, notices: &ExtractedNotices) -> Vec<SourceInfo> {
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
    sources
}

/// Validates and publishes a built [`Library`]: shards, catalog, reports, search index,
/// manifest, and the content version.
///
/// The step holds no source-specific knowledge; every alias and facet it needs is already
/// part of the library.
pub fn package(library: Library, out: &Path, reports: &Path) -> Result<Outcome, Failure> {
    let Library {
        entries,
        relations,
        spell_lists,
        books,
        legal,
        mut coverage,
        aliases,
        facets,
        datasets,
        sources,
        notes,
        dedupe,
        survey,
        supplement_prefix,
        aliases_file,
    } = library;

    // Shards and catalog.
    let (shard_files, shard_of, largest_entry) = shard(&entries)?;
    let catalog = Catalog {
        sources,
        entries: entries
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

    let mut errors = validate_library(&entries, &catalog);
    errors.extend(validate_derived(&entries, &relations, &books, &spell_lists));
    let by_kind = coverage.by_kind.clone();

    // Reverse body links. The index is optional and additive: it is written from the same
    // entries that the shards hold, so it is hashed in the manifest like every other file.
    let backlinks = crate::backlinks::build(&entries);
    let mut backlink_files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut backlink_shard_bytes = 0u64;
    for (shard, targets) in backlinks.shards.iter().enumerate() {
        let bytes = json_bytes(targets)?;
        backlink_shard_bytes += bytes.len() as u64;
        backlink_files.push((cast_iron_schema::backlink_shard_path(shard as u32), bytes));
    }
    let backlink_descriptor = json_bytes(&backlinks.index)?;
    let backlink_descriptor_bytes = backlink_descriptor.len() as u64;
    coverage.backlinks = backlinks.coverage;
    coverage.backlinks.bytes = backlink_descriptor_bytes + backlink_shard_bytes;

    write_reports(
        reports,
        &errors,
        &coverage,
        &notes,
        &dedupe,
        &survey,
        &catalog,
        supplement_prefix.as_deref(),
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
    for e in &entries {
        index.add(DocInput {
            id: e.id.to_string(),
            title: e.title.clone(),
            aliases: aliases.get(&e.id).cloned().unwrap_or_default(),
            kind: e.kind.slug().to_string(),
            source: e.source_ids[0].clone(),
            section: e.section.join(" > "),
            facets: facets.get(&e.id).cloned().unwrap_or_default(),
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
    if let Some(aliases) = aliases_file {
        files.insert(crate::alias::ALIASES_FILE.into(), aliases);
    }
    let mut derived_bytes = BTreeMap::new();
    for (name, bytes) in [
        (RELATIONS_FILE, json_bytes(&relations)?),
        (SPELL_LISTS_FILE, json_bytes(&spell_lists)?),
        (BOOKS_FILE, json_bytes(&books)?),
    ] {
        derived_bytes.insert(name.to_string(), bytes.len() as u64);
        files.insert(name.into(), bytes);
    }
    files.insert(BACKLINKS_FILE.into(), backlink_descriptor);
    for (path, bytes) in backlink_files {
        files.insert(path, bytes);
    }
    derived_bytes.insert(BACKLINKS_FILE.to_string(), backlink_descriptor_bytes);
    derived_bytes.insert(format!("{BACKLINKS_DIR}/"), backlink_shard_bytes);
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
        entry_count: entries.len() as u32,
        datasets,
    };

    prepare_out(out)?;
    for (path, bytes) in &files {
        let target = out.join(&content_version).join(path);
        write_file(&target, bytes)?;
    }
    write_file(&out.join("manifest.json"), &json_bytes(&manifest)?)?;

    let largest_shard = manifest
        .files
        .iter()
        .filter(|f| f.path.contains("/entries/"))
        .max_by(|a, b| a.bytes.cmp(&b.bytes).then(b.path.cmp(&a.path)))
        .map(|f| (f.path.clone(), f.bytes))
        .unwrap_or_default();
    Ok(Outcome {
        content_version,
        entry_count: entries.len(),
        by_kind,
        total_bytes: manifest.total_bytes,
        largest_shard,
        largest_entry,
        links: coverage.links.clone(),
        rejected: coverage.rejected.clone(),
        stripped: coverage.stripped.clone(),
        dedupe,
        notes: notes.len(),
        relations: coverage.relations.clone(),
        derived_bytes,
        backlinks: coverage.backlinks.clone(),
    })
}

/// The full compile for the configured source: build, then package.
pub fn compile(opts: &Options) -> Result<Outcome, Failure> {
    package(build_psrd(opts)?, &opts.out, &opts.reports)
}

/// The canonical compile: package the Pf Data library and its PSRD carry-over.
pub fn compile_canonical(opts: &CanonicalOptions) -> Result<Outcome, Failure> {
    package(build_canonical(opts)?, &opts.psrd.out, &opts.psrd.reports)
}

/// One Pf Data publication as the canonical build sees it.
#[derive(Debug, Clone)]
struct PublicationState {
    /// Canonical source id, shared with PSRD when the book exists there.
    source: String,
    /// Display title used by the catalog and the book view.
    title: String,
    /// Every notice that matches this publication in the union of the pinned
    /// Section 15 lists, with duplicates removed.
    notices: Vec<pf_legal::Notice>,
    /// The owner reviewed the absence of a notice and chose to ship this
    /// publication as assumed Open Game Content. Coverage stays open in #34.
    pending: bool,
}

impl PublicationState {
    /// A publication may ship when a notice covers it or the reviewed
    /// notice-pending list names it. Its records then keep the usual license
    /// references.
    fn shippable(&self) -> bool {
        !self.notices.is_empty() || self.pending
    }
}

/// The note stored with the Pf Data dataset pin on the legal page.
const PF_DATASET_NOTE: &str = "Open Game Content from the pinned Pathfinder Data 1e corpus, used under the Open Game License 1.0a and Paizo's Community Use Policy; see docs/research/pf-data-licensing.md.";

/// Builds the complete canonical library: screened Pf Data conversion, typed
/// relations, PSRD carry-over, page-ordered books, spell lists, legal notices,
/// and the generated alias map.
pub fn build_canonical(opts: &CanonicalOptions) -> Result<Library, Failure> {
    let psrd = build_psrd(&opts.psrd)?;
    let pf_config_dir = opts
        .pf_registry
        .parent()
        .ok_or_else(|| failure("the Pf Data registry has no parent directory"))?;
    let allowlist =
        crate::pf_rules::RuleAllowlist::load_optional(&pf_config_dir.join("derived-rules.json"))?;
    let inv = pf_data::inventory(&opts.pf_dataset, &opts.pf_registry)
        .map_err(|error| failure(format!("cannot read the Pf Data inventory: {error}")))?;

    // The registry is reviewed input. A new publication in the pin must be
    // reviewed and registered; it never gets an automatic source id.
    let registry = Registry::load(&opts.canonical_dir.join("publications.toml"))?;
    check_publication_registry(&registry, &inv)?;

    let psrd_config_dir = opts
        .psrd
        .source_config
        .parent()
        .ok_or_else(|| failure("the PSRD source config has no parent directory"))?;
    let section15 = pf_config_dir.join("section15");
    let legal_report = pf_legal::build(
        &inv,
        &pf_legal::Inputs {
            opengamecontent: &opts.pf_dataset.join("OPENGAMECONTENT.md"),
            psrd_notices: Some(&psrd_config_dir.join("legal-notices.json")),
            aliases: &pf_config_dir.join("notice-aliases.json"),
            out_of_scope: &pf_config_dir.join("out-of-scope.json"),
            d20pfsrd_csv: &section15.join("d20pfsrd-section15.csv"),
            d20pfsrd_provenance: &section15.join("d20pfsrd-section15.provenance.json"),
            owner_copies: &section15.join("owner-copies.json"),
            owner_provenance: &section15.join("owner-copies.provenance.json"),
            notice_pending: &section15.join("notice-pending.json"),
        },
    )
    .map_err(|error| failure(format!("cannot build the Pf Data legal report: {error}")))?;
    let pf_pin: pf_data::SourcePin = pf_data::read_json(&opts.pf_registry)
        .map_err(|error| failure(format!("cannot read the Pf Data pin: {error}")))?;
    let states = publication_states(&registry, &legal_report)?;

    let mut converted = canonical::convert_corpus(&inv, &allowlist)
        .map_err(|error| failure(format!("canonical conversion failed: {error}")))?;
    let mut rejected: Vec<Rejection> = Vec::new();
    let legally_excluded = apply_legal_screen(&mut converted.entries, &states, &mut rejected)?;
    // The earliest `‹SOURCE Book/page›` attribution is the printed starting
    // point, so it makes the primary publication. The upstream `sources` array
    // is not reliable for this: a mythic or later printing is often listed first.
    let primary_reordered =
        order_primary_sources(&mut converted.entries, &converted.pages, &states);
    for excluded in &converted.excluded {
        rejected.push(Rejection {
            path: excluded.identity.clone(),
            reason: format!(
                "converter {} exclusion{}: {}",
                excluded.category,
                excluded
                    .id
                    .as_deref()
                    .map(|id| format!(" ({id})"))
                    .unwrap_or_default(),
                excluded.reason
            ),
        });
    }

    // Relations run over the entries that survived the legal screen, so a
    // target that cannot be redistributed never becomes a dangling edge.
    let pf_relations = crate::relations::apply_canonical(&mut converted.entries, &converted.links);

    // Carry-over is explicit in the reviewed id map. Its inputs are the PSRD
    // library and the same canonical catalog the alias gate uses.
    let map = alias::IdMap::load(&opts.canonical_dir.join("id-map.toml"))?;
    let snapshot_path = opts.canonical_dir.join("legacy-ids.json");
    let snapshot_bytes = fs::read(&snapshot_path)
        .map_err(|e| format!("cannot read {}: {e}", snapshot_path.display()))?;
    let snapshot = LegacySnapshot::parse(&snapshot_bytes)
        .map_err(|e| format!("invalid {}: {e}", snapshot_path.display()))?;
    let mut catalog = alias::catalog_from_inventory(&inv, &allowlist)?;
    let emitted: BTreeSet<String> = converted
        .entries
        .iter()
        .map(|entry| entry.id.to_string())
        .collect();
    let carried =
        carry::carry_over_with_fallback(&psrd, &map, &catalog, &emitted, &legally_excluded)
            .map_err(|errors| problems("carry-over", errors))?;
    carried
        .extend_catalog(&mut catalog)
        .map_err(|errors| problems("carried catalog", errors))?;

    let mut entries = std::mem::take(&mut converted.entries);
    entries.extend(carried.records.iter().map(|record| record.entry.clone()));
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    check_unique_ids(&entries)?;

    let ids: BTreeSet<EntryId> = entries.iter().map(|entry| entry.id.clone()).collect();
    let settlement = settle_references(&mut entries, &ids);

    // Carried entries have a PSRD source but no page attribution. Giving their
    // book a display title keeps the canonical book view from showing a slug.
    let mut pages = converted.pages.clone();
    add_carried_pages(&mut pages, &carried.records, &psrd.sources);
    // Resolve the alias map first: the PSRD books' legacy ids are rewritten
    // through the same final targets that ship in aliases.json.
    let aliases = canonical_aliases(
        &entries,
        &legally_excluded,
        &map,
        &snapshot,
        &snapshot_bytes,
        &catalog,
        &carried.fallbacks,
    )?;
    let books = crate::books::build_canonical_with_psrd(
        &entries,
        &converted.links,
        &pages,
        &psrd.books,
        &aliases.targets,
        &aliases.dropped,
    );
    let spell_lists = crate::lists::build(&entries);
    let relations = crate::relations::index_over(&entries, pf_relations.coverage);
    let relation_coverage = relations.coverage.clone();

    let legal = canonical_legal(&psrd.legal, &legal_report, &pf_pin);
    let (aliases_file, alias_retirements) = (aliases.bytes, aliases.retirements);

    let sources = canonical_sources(&entries, &psrd.sources, &states);
    let by_publication =
        canonical_publications(&entries, &inv, &states, &carried.report, &sources)?;
    let mut by_kind: BTreeMap<String, u32> = BTreeMap::new();
    for entry in &entries {
        *by_kind.entry(entry.kind.slug().to_string()).or_default() += 1;
    }
    let aliases = search_aliases(&entries);

    let mut excluded = canonical_coverage_rows(
        &inv,
        &converted,
        &carried.report,
        &settlement,
        alias_retirements,
    );
    excluded.sort_by(|a, b| (&a.path, &a.reason).cmp(&(&b.path, &b.reason)));
    // A parse error can quote upstream text, markers included.
    for row in &mut rejected {
        row.reason = crate::pf_plain::plain_text(&row.reason);
    }
    rejected.sort_by(|a, b| (&a.path, &a.reason).cmp(&(&b.path, &b.reason)));
    rejected.dedup();

    let coverage = CoverageReport {
        input_digest: sha256_hex(
            format!("{}\n{}\n", psrd.coverage.input_digest, inv.input_digest).as_bytes(),
        ),
        by_publication,
        by_kind,
        excluded,
        rejected,
        links: canonical_links(&entries),
        stripped: BTreeMap::new(),
        relations: cast_iron_schema::RelationCoverage {
            spell_lists: spell_lists.coverage.clone(),
            book_order: books.coverage.clone(),
            ..relation_coverage
        },
        backlinks: Default::default(),
    };

    let mut notes = psrd.notes.clone();
    for row in &books.stats.curated {
        notes.push(format!(
            "canonical book order {}: {} chapters, {} placed via the PSRD tree, {} appended by page, {} dropped, {} duplicate placements skipped",
            row.source,
            row.chapters,
            row.placed,
            row.appended,
            row.dropped,
            row.duplicates,
        ));
        for id in &row.dropped_ids {
            notes.push(format!(
                "canonical book order {}: dropped legacy id {id}",
                row.source
            ));
        }
        for id in &row.duplicate_ids {
            notes.push(format!(
                "canonical book order {}: duplicate canonical placement {id}",
                row.source
            ));
        }
    }
    notes.push(format!(
        "canonical primary publication: {primary_reordered} multi-book entries reordered by earliest source attribution"
    ));
    notes.push(format!(
        "canonical build: {} entries ({} Pf Data, {} carried, {} fallback carried), {} rejected records, {} alias retirements ({} rights, {} unrepresentable)",
        entries.len(),
        entries.len() - carried.report.carried,
        carried.report.carried,
        carried.fallbacks.len(),
        coverage.rejected.len(),
        alias_retirements.rights + alias_retirements.unrepresentable,
        alias_retirements.rights,
        alias_retirements.unrepresentable,
    ));

    let datasets: Vec<DatasetRef> = legal
        .datasets
        .iter()
        .map(|dataset| DatasetRef {
            id: dataset.id.clone(),
            repository: dataset.repository.clone(),
            commit: dataset.commit.clone(),
        })
        .collect();
    let facets = canonical_facets(&entries);
    check_no_markup_leaks(&entries, &facets, &aliases, &coverage)?;
    Ok(Library {
        entries,
        relations: relations.index,
        spell_lists: spell_lists.lists,
        books: books.index,
        legal,
        coverage,
        aliases,
        facets,
        datasets,
        sources,
        notes,
        dedupe: DedupeStats::default(),
        survey: psrd.survey,
        supplement_prefix: None,
        aliases_file: Some(aliases_file),
    })
}

/// Search facets for the canonical library. The PSRD builder captures facets while parsing
/// upstream HTML; canonical entries are already typed, so the same values are read from
/// [`TypedFields`] instead of a second representation of the record.
///
/// Race groups are the one gap: the pin stores the group only in the upstream subtype, and
/// the canonical entry does not carry it. A race therefore has no `race_group` facet.
fn canonical_facets(entries: &[RuleEntry]) -> BTreeMap<EntryId, Vec<(String, String)>> {
    fn push(facets: &mut Vec<(String, String)>, name: &str, value: &str) {
        if value.is_empty() {
            return;
        }
        if !facets.iter().any(|(n, v)| n == name && v == value) {
            facets.push((name.to_string(), value.to_string()));
        }
    }
    let mut out = BTreeMap::new();
    for entry in entries {
        let mut facets = Vec::new();
        match &entry.fields {
            TypedFields::Spell(fields) => {
                push(
                    &mut facets,
                    "school",
                    &crate::pf_plain::school_facet(&fields.school),
                );
                for descriptor in &fields.descriptors {
                    push(&mut facets, "descriptor", descriptor);
                }
                for level in &fields.levels {
                    push(&mut facets, "class", &level.class);
                    push(&mut facets, "spell_level", &level.level.to_string());
                }
            }
            TypedFields::Feat(fields) => {
                for kind in &fields.feat_types {
                    push(&mut facets, "feat_type", kind);
                }
            }
            TypedFields::Creature(fields) => {
                if let Some(value) = &fields.creature_type {
                    push(&mut facets, "creature_type", value);
                }
                if let Some(value) = &fields.size {
                    push(&mut facets, "size", value);
                }
                if let Some(value) = &fields.alignment {
                    push(&mut facets, "alignment", value);
                }
                if let Some(cr) = fields.cr {
                    push(&mut facets, "cr", &cr.display());
                }
            }
            TypedFields::Item(fields) => {
                if let Some(value) = &fields.category {
                    push(&mut facets, "item_category", value);
                }
                if let Some(value) = fields.slot.as_deref().filter(|slot| *slot != "none") {
                    push(&mut facets, "slot", value);
                }
            }
            TypedFields::Class(fields) => {
                if let Some(value) = &fields.class_type {
                    push(&mut facets, "class_type", value);
                }
            }
            TypedFields::Trait(fields) => push(&mut facets, "trait_category", &fields.category),
            TypedFields::RacialTrait(_) | TypedFields::None => {}
        }
        // An archetype's class comes from its typed edge, not from a text field.
        if let Some(target) = entry
            .relations
            .iter()
            .find(|relation| relation.kind == RelationKind::Archetype)
            .map(|relation| relation.target.as_str())
        {
            let class = target.rsplit('/').next().unwrap_or_default();
            push(&mut facets, "archetype_class", class);
        }
        if !facets.is_empty() {
            out.insert(entry.id.clone(), facets);
        }
    }
    out
}

/// Fails the canonical build when Pathfinder Data inline markup (`‹`, `›`, `&#`) is left in
/// any compiled text: entry titles, fields, body text, facets, aliases, or the coverage
/// report. Field and body text must carry display text only (see `pf_plain`).
fn check_no_markup_leaks(
    entries: &[RuleEntry],
    facets: &BTreeMap<EntryId, Vec<(String, String)>>,
    aliases: &BTreeMap<EntryId, Vec<String>>,
    coverage: &CoverageReport,
) -> Result<(), Failure> {
    let mut lines = Vec::new();
    let mut total = 0usize;
    let mut check = |owner: &str, what: &str, json: Result<String, serde_json::Error>| {
        let json = json.unwrap_or_default();
        let leaks = crate::pf_plain::markup_leaks(&json);
        total += leaks.len();
        if let Some(first) = leaks.first()
            && lines.len() < 25
        {
            lines.push(format!(
                "  {owner} ({what}): {} leak(s), first near `{first}`",
                leaks.len()
            ));
        }
    };
    for entry in entries {
        check(entry.id.as_str(), "entry", serde_json::to_string(entry));
        if let Some(values) = facets.get(&entry.id) {
            check(entry.id.as_str(), "facets", serde_json::to_string(values));
        }
        if let Some(values) = aliases.get(&entry.id) {
            check(entry.id.as_str(), "aliases", serde_json::to_string(values));
        }
    }
    check("coverage", "report", serde_json::to_string(coverage));
    if total == 0 {
        return Ok(());
    }
    let mut all = vec![format!(
        "pathfinder data inline markup left in compiled output: {total} occurrence(s) of `‹`, `›` or `&#`"
    )];
    all.append(&mut lines);
    Err(Failure { lines: all })
}

fn failure(message: impl Into<String>) -> Failure {
    Failure {
        lines: vec![message.into()],
    }
}

fn problems(context: &str, errors: Vec<String>) -> Failure {
    let mut lines = vec![format!("{context} failed: {} problem(s):", errors.len())];
    lines.extend(errors.into_iter().map(|error| format!("  {error}")));
    Failure { lines }
}

/// Fails when the pinned inventory names a publication the registry lacks.
fn check_publication_registry(registry: &Registry, inv: &Inventory) -> Result<(), Failure> {
    let missing: Vec<String> = publications::inventory_publications(inv)
        .into_iter()
        .filter(|title| registry.get(title).is_none())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let mut lines = vec![format!(
        "{} Pf Data publication(s) are missing from {}:",
        missing.len(),
        publications::REGISTRY_PATH
    )];
    lines.extend(missing.into_iter().map(|title| format!("  {title}")));
    lines.push(
        "Run `cargo xtask data-publications`, review the rows, and commit the registry.".into(),
    );
    Err(Failure { lines })
}

/// Uses the union of the pinned notice sources from the legal report. A record
/// is kept when at least one of its publications has a notice or a reviewed
/// notice-pending decision, so an absent notice excludes the record rather than
/// shipping it without a redistribution basis.
fn publication_states(
    registry: &Registry,
    report: &pf_legal::LegalReport,
) -> Result<BTreeMap<String, PublicationState>, Failure> {
    let by_publication: BTreeMap<&str, &pf_legal::PublicationLegal> = report
        .publications
        .iter()
        .map(|publication| (publication.publication.as_str(), publication))
        .collect();
    let pending: BTreeSet<&str> = report
        .pending_publications
        .iter()
        .map(|publication| publication.publication.as_str())
        .collect();
    let mut states = BTreeMap::new();
    for (title, row) in &registry.publication {
        let mut notices = by_publication
            .get(title.as_str())
            .map(|publication| publication.notices.clone())
            .unwrap_or_default();
        notices.sort_by(|a, b| (&a.title, &a.notice).cmp(&(&b.title, &b.notice)));
        notices.dedup();
        states.insert(
            title.clone(),
            PublicationState {
                source: row.source.clone(),
                title: row.title.clone(),
                notices,
                pending: pending.contains(title.as_str()),
            },
        );
    }
    Ok(states)
}

/// Puts the publication named by the entry's earliest `‹SOURCE Book/page›`
/// attribution first. The full publication set stays intact: only the order
/// changes, so alias fit and every book chip keep their evidence.
///
/// Entries with no page attribution keep the upstream `sources` order. The
/// return value counts entries whose primary publication changed.
fn order_primary_sources(
    entries: &mut [RuleEntry],
    pages: &BTreeMap<EntryId, Vec<(String, Option<u32>)>>,
    states: &BTreeMap<String, PublicationState>,
) -> usize {
    let title_to_source: BTreeMap<&str, &str> = states
        .iter()
        .map(|(title, state)| (title.as_str(), state.source.as_str()))
        .collect();
    let mut changed = 0;
    for entry in entries {
        if entry.source_ids.len() < 2 {
            continue;
        }
        let Some(rows) = pages.get(&entry.id) else {
            continue;
        };
        let mut ordered = Vec::with_capacity(entry.source_ids.len());
        let mut seen = BTreeSet::new();
        for (title, _) in rows {
            let Some(&source) = title_to_source.get(title.as_str()) else {
                continue;
            };
            if entry.source_ids.iter().any(|id| id.as_str() == source)
                && seen.insert(source.to_string())
            {
                ordered.push(source.to_string());
            }
        }
        for source in &entry.source_ids {
            if seen.insert(source.clone()) {
                ordered.push(source.clone());
            }
        }
        if ordered != entry.source_ids {
            entry.source_ids = ordered;
            changed += 1;
        }
    }
    changed
}

/// Rewrites each converted entry's source ids through the registry and drops the
/// publications that have neither a notice nor a notice-pending decision. Returns
/// the ids that were excluded.
fn apply_legal_screen(
    entries: &mut Vec<RuleEntry>,
    states: &BTreeMap<String, PublicationState>,
    rejected: &mut Vec<Rejection>,
) -> Result<BTreeSet<String>, Failure> {
    let mut by_slug: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut shippable: BTreeMap<String, bool> = BTreeMap::new();
    for (title, state) in states {
        let sources = by_slug.entry(slug_segment(title)).or_default();
        if !sources.contains(&state.source) {
            sources.push(state.source.clone());
        }
        shippable
            .entry(state.source.clone())
            .and_modify(|value| *value |= state.shippable())
            .or_insert_with(|| state.shippable());
    }
    let mut errors = Vec::new();
    let mut excluded = BTreeSet::new();
    entries.retain_mut(|entry| {
        let mut kept = Vec::new();
        let mut seen = BTreeSet::new();
        for slug in &entry.source_ids {
            let Some(sources) = by_slug.get(slug) else {
                errors.push(format!(
                    "entry {} names publication slug {slug:?}, which has no registry row",
                    entry.id
                ));
                continue;
            };
            for source in sources {
                if shippable.get(source).copied().unwrap_or(false) && seen.insert(source.clone()) {
                    kept.push(source.clone());
                }
            }
        }
        if kept.is_empty() {
            excluded.insert(entry.id.as_str().to_string());
            rejected.push(Rejection {
                path: entry.id.to_string(),
                reason: format!(
                    "no publication with a Section 15 notice or a notice-pending decision ({}); redistribution cannot be established",
                    if entry.source_ids.is_empty() {
                        "no publication attribution".to_string()
                    } else {
                        entry.source_ids.join(", ")
                    }
                ),
            });
            false
        } else {
            entry.source_ids = kept;
            true
        }
    });
    if !errors.is_empty() {
        return Err(problems("publication source rewrite", errors));
    }
    Ok(excluded)
}

fn check_unique_ids(entries: &[RuleEntry]) -> Result<(), Failure> {
    let mut lines = Vec::new();
    for pair in entries.windows(2) {
        if pair[0].id == pair[1].id {
            lines.push(format!(
                "canonical id {} is produced by both Pf Data and carry-over",
                pair[0].id
            ));
        }
    }
    if lines.is_empty() {
        Ok(())
    } else {
        Err(Failure { lines })
    }
}

/// The canonical legal page: PSRD's notices plus every matched Pf Data notice,
/// with both dataset pins. Deduplicates by title and notice text. Publications
/// shipped under a reviewed notice-pending decision are listed separately; they
/// are not given a fabricated Section 15 line.
fn canonical_legal(
    psrd: &LegalNotices,
    report: &pf_legal::LegalReport,
    pf_pin: &pf_data::SourcePin,
) -> LegalNotices {
    let mut legal = psrd.clone();
    let mut seen: BTreeSet<(String, String)> = legal
        .section_15
        .iter()
        .map(|notice| (notice.title.clone(), notice.notice.clone()))
        .collect();
    for publication in &report.publications {
        for notice in &publication.notices {
            if seen.insert((notice.title.clone(), notice.notice.clone())) {
                legal.section_15.push(CopyrightNotice {
                    title: notice.title.clone(),
                    notice: notice.notice.clone(),
                });
            }
        }
    }
    legal.notice_pending = report
        .pending_publications
        .iter()
        .map(|publication| publication.publication.clone())
        .collect();
    legal.notice_pending.sort();
    legal.notice_pending.dedup();
    // The PSRD build may already add the Pf Data pin when the reviewed
    // supplement is configured. Keep exactly one notice per dataset id and use
    // the canonical note, which describes the whole corpus rather than one row.
    legal.datasets.retain(|dataset| dataset.id != pf_pin.id);
    legal.datasets.push(DatasetNotice {
        id: pf_pin.id.clone(),
        repository: pf_pin.repository.clone(),
        commit: pf_pin.commit.clone(),
        note: PF_DATASET_NOTE.to_string(),
    });
    legal
}

fn add_carried_pages(
    pages: &mut BTreeMap<EntryId, Vec<(String, Option<u32>)>>,
    carried: &[carry::Carried],
    sources: &[SourceInfo],
) {
    let titles: BTreeMap<&str, &str> = sources
        .iter()
        .map(|source| (source.id.as_str(), source.title.as_str()))
        .collect();
    for record in carried {
        let rows = pages.entry(record.entry.id.clone()).or_default();
        for source in &record.entry.source_ids {
            if let Some(title) = titles.get(source.as_str())
                && !rows.iter().any(|(existing, _)| existing == title)
            {
                rows.push(((*title).to_string(), None));
            }
        }
    }
}

#[derive(Debug, Default)]
struct Settlement {
    body: u32,
    parents: u32,
    relations: u32,
    prerequisites: u32,
}

impl Settlement {
    fn total(&self) -> u32 {
        self.body + self.parents + self.relations + self.prerequisites
    }
}

/// Removes references to entries the legal screen removed. The body keeps the
/// visible text, exactly as carry-over does for an unresolvable target, and the
/// count is reported in coverage. Every remaining reference names a real entry.
fn settle_references(entries: &mut [RuleEntry], ids: &BTreeSet<EntryId>) -> Settlement {
    let mut settlement = Settlement::default();
    for entry in entries.iter_mut() {
        if let Some(parent) = entry.parent.clone()
            && !ids.contains(&parent)
        {
            entry.parent = None;
            settlement.parents += 1;
        }
        let relations = std::mem::take(&mut entry.relations);
        for relation in relations {
            if ids.contains(&relation.target) {
                entry.relations.push(relation);
            } else {
                settlement.relations += 1;
            }
        }
        let body = std::mem::take(&mut entry.body);
        entry.body = settle_blocks(body, ids, &mut settlement);
        entry.links.retain(|link| ids.contains(&link.target));
        entry.links.sort();
        entry.links.dedup();
        if let TypedFields::Feat(fields) = &mut entry.fields {
            let feats = std::mem::take(&mut fields.prerequisite_feats);
            for feat in feats {
                if ids.contains(&feat) {
                    fields.prerequisite_feats.push(feat);
                } else {
                    settlement.prerequisites += 1;
                }
            }
            fields.prerequisite_feats.sort();
            fields.prerequisite_feats.dedup();
        }
    }
    settlement
}

fn settle_blocks(
    blocks: Vec<Block>,
    ids: &BTreeSet<EntryId>,
    counts: &mut Settlement,
) -> Vec<Block> {
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            Block::Heading {
                level,
                anchor,
                content,
            } => out.push(Block::Heading {
                level,
                anchor,
                content: settle_inlines(content, ids, counts),
            }),
            Block::Paragraph { style, content } => out.push(Block::Paragraph {
                style,
                content: settle_inlines(content, ids, counts),
            }),
            Block::List { ordered, items } => out.push(Block::List {
                ordered,
                items: items
                    .into_iter()
                    .map(|item| settle_blocks(item, ids, counts))
                    .collect(),
            }),
            Block::Table(table) => out.push(Block::Table(settle_table(table, ids, counts))),
            Block::Quote { content } => out.push(Block::Quote {
                content: settle_blocks(content, ids, counts),
            }),
            Block::StatBlock { title, rows } => out.push(Block::StatBlock {
                title,
                rows: rows
                    .into_iter()
                    .map(|row| StatRow {
                        label: row.label,
                        value: settle_inlines(row.value, ids, counts),
                    })
                    .collect(),
            }),
            Block::EntryLink { target, title } => {
                if ids.contains(&target) {
                    out.push(Block::EntryLink { target, title });
                } else {
                    counts.body += 1;
                    out.push(Block::Paragraph {
                        style: ParagraphStyle::Normal,
                        content: vec![Inline::Text { v: title }],
                    });
                }
            }
        }
    }
    out
}

fn settle_inlines(
    inlines: Vec<Inline>,
    ids: &BTreeSet<EntryId>,
    counts: &mut Settlement,
) -> Vec<Inline> {
    let mut out = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match inline {
            Inline::Link { target, anchor, c } => {
                if ids.contains(&target) {
                    out.push(Inline::Link {
                        target,
                        anchor,
                        c: settle_inlines(c, ids, counts),
                    });
                } else {
                    counts.body += 1;
                    out.extend(settle_inlines(c, ids, counts));
                }
            }
            Inline::Em { c } => out.push(Inline::Em {
                c: settle_inlines(c, ids, counts),
            }),
            Inline::Strong { c } => out.push(Inline::Strong {
                c: settle_inlines(c, ids, counts),
            }),
            Inline::Sup { c } => out.push(Inline::Sup {
                c: settle_inlines(c, ids, counts),
            }),
            Inline::Sub { c } => out.push(Inline::Sub {
                c: settle_inlines(c, ids, counts),
            }),
            other => out.push(other),
        }
    }
    out
}

fn settle_table(table: Table, ids: &BTreeSet<EntryId>, counts: &mut Settlement) -> Table {
    Table {
        caption: settle_inlines(table.caption, ids, counts),
        head: settle_rows(table.head, ids, counts),
        body: settle_rows(table.body, ids, counts),
        foot: settle_rows(table.foot, ids, counts),
    }
}

fn settle_rows(
    rows: Vec<TableRow>,
    ids: &BTreeSet<EntryId>,
    counts: &mut Settlement,
) -> Vec<TableRow> {
    rows.into_iter()
        .map(|row| TableRow {
            cells: row
                .cells
                .into_iter()
                .map(|cell| TableCell {
                    header: cell.header,
                    colspan: cell.colspan,
                    rowspan: cell.rowspan,
                    content: settle_inlines(cell.content, ids, counts),
                })
                .collect(),
        })
        .collect()
}

#[derive(Debug, Default, Clone, Copy)]
struct AliasRetirements {
    rights: usize,
    unrepresentable: usize,
    fallbacks: usize,
}

struct CanonicalAliases {
    bytes: Vec<u8>,
    retirements: AliasRetirements,
    /// Legacy id to the canonical target that is present in the installed library.
    targets: BTreeMap<String, String>,
    /// Legacy id to the reason it has no installed target.
    dropped: BTreeMap<String, String>,
}

/// Generates the alias map in process and retires every row whose canonical
/// target is not in the installed library. A row whose target the converter did
/// not emit falls back to its carried entry when one was produced, so no legacy
/// bookmark points outside the library and no OGC is lost. A retired row names
/// the reason instead of pointing outside the library. The alias gate still runs
/// against the full planned catalog, so a map row that names something the pin
/// cannot produce fails here rather than being silently retired.
fn canonical_aliases(
    entries: &[RuleEntry],
    legally_excluded: &BTreeSet<String>,
    map: &alias::IdMap,
    snapshot: &LegacySnapshot,
    snapshot_bytes: &[u8],
    catalog: &alias::CanonicalCatalog,
    fallbacks: &BTreeMap<String, String>,
) -> Result<CanonicalAliases, Failure> {
    let (mut file, _stats) = alias::generate(map, snapshot, snapshot_bytes, catalog)
        .map_err(|errors| problems("alias gate(s)", errors))?;
    let final_ids: BTreeSet<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
    let mut kept = Vec::new();
    let mut retirements = AliasRetirements {
        fallbacks: fallbacks.len(),
        ..AliasRetirements::default()
    };
    let mut targets = BTreeMap::new();
    let mut dropped: BTreeMap<String, String> = file
        .retired
        .iter()
        .map(|row| (row.old.clone(), row.reason.slug().to_string()))
        .collect();
    for mut entry in file.entries.drain(..) {
        if final_ids.contains(entry.target.as_str()) {
            targets.insert(entry.old.clone(), entry.target.clone());
            kept.push(entry);
            continue;
        }
        if let Some(target) = fallbacks.get(&entry.old)
            && final_ids.contains(target.as_str())
        {
            entry.target = target.clone();
            entry.basis = alias::AliasBasis::Carried;
            targets.insert(entry.old.clone(), entry.target.clone());
            kept.push(entry);
            continue;
        }
        let (reason, note) = if legally_excluded.contains(entry.target.as_str()) {
            retirements.rights += 1;
            (
                alias::RetirementReason::Rights,
                "canonical target excluded: no Section 15 notice covers its publications",
            )
        } else {
            retirements.unrepresentable += 1;
            (
                alias::RetirementReason::Unrepresentable,
                "canonical target was planned from the pin but the converter did not emit it",
            )
        };
        dropped.insert(entry.old.clone(), reason.slug().to_string());
        file.retired.push(alias::RetiredEntry {
            old: entry.old,
            reason,
            note: Some(note.into()),
        });
    }
    file.entries = kept;
    file.retired.sort_by(|a, b| a.old.cmp(&b.old));
    let bytes = alias::to_bytes(&file)?;
    Ok(CanonicalAliases {
        bytes,
        retirements,
        targets,
        dropped,
    })
}

fn search_aliases(entries: &[RuleEntry]) -> BTreeMap<EntryId, Vec<String>> {
    entries
        .iter()
        .map(|entry| {
            let mut names: BTreeSet<String> = BTreeSet::new();
            if entry.kind == EntryKind::Rule {
                for block in &entry.body {
                    if let Block::Heading { content, .. } = block {
                        let heading = inline_text(content);
                        if !heading.is_empty() && heading != entry.title {
                            names.insert(heading);
                        }
                    }
                }
            }
            (entry.id.clone(), names.into_iter().collect())
        })
        .collect()
}

fn canonical_sources(
    entries: &[RuleEntry],
    psrd_sources: &[SourceInfo],
    states: &BTreeMap<String, PublicationState>,
) -> Vec<SourceInfo> {
    let psrd_by_id: BTreeMap<&str, &SourceInfo> = psrd_sources
        .iter()
        .map(|source| (source.id.as_str(), source))
        .collect();
    let mut titles: BTreeMap<String, String> = BTreeMap::new();
    let mut notices: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    let mut pending: BTreeMap<String, String> = BTreeMap::new();
    for state in states.values() {
        titles
            .entry(state.source.clone())
            .or_insert_with(|| state.title.clone());
        if !state.notices.is_empty() {
            let slot = notices.entry(state.source.clone()).or_default();
            for notice in &state.notices {
                slot.insert((notice.title.clone(), notice.notice.clone()));
            }
        }
        if state.pending {
            pending
                .entry(state.source.clone())
                .or_insert_with(|| state.title.clone());
        }
    }
    let mut used: BTreeSet<&str> = BTreeSet::new();
    for entry in entries {
        for source in &entry.source_ids {
            used.insert(source.as_str());
        }
    }
    let mut sources = Vec::with_capacity(used.len());
    for source in used {
        let title = titles
            .get(source)
            .cloned()
            .or_else(|| psrd_by_id.get(source).map(|info| info.title.clone()))
            .unwrap_or_else(|| source.to_string());
        let copyright = notices
            .get(source)
            .and_then(|rows| (!rows.is_empty()).then(|| canonical_copyright(rows)));
        let copyright = copyright
            .or_else(|| {
                pending.get(source).map(|title| {
                    format!(
                        "{title}: Section 15 notice pending; Open Game Content used under the OGL."
                    )
                })
            })
            .or_else(|| {
                psrd_by_id
                    .get(source)
                    .and_then(|info| info.copyright.clone())
            });
        sources.push(SourceInfo {
            id: source.to_string(),
            title,
            entry_count: entries
                .iter()
                .filter(|entry| entry.source_ids.first().is_some_and(|id| id == source))
                .count() as u32,
            copyright,
        });
    }
    sources.sort_by(|a, b| a.id.cmp(&b.id));
    sources
}

/// Join every distinct matched Section 15 notice once, verbatim, with `; `.
///
/// The book title is [`SourceInfo::title`] and the page heading; adding it to
/// every notice produced strings such as "Core Rulebook. Core Rulebook, © …".
/// Several printings of one book are distinct notices and each must stay.
fn canonical_copyright(rows: &BTreeSet<(String, String)>) -> String {
    let mut seen = BTreeSet::new();
    let mut notices = Vec::new();
    for (_, notice) in rows {
        let notice = notice.trim();
        if !notice.is_empty() && seen.insert(notice.to_string()) {
            notices.push(notice.to_string());
        }
    }
    notices.join("; ")
}

fn canonical_publications(
    entries: &[RuleEntry],
    inv: &Inventory,
    states: &BTreeMap<String, PublicationState>,
    carried: &carry::CarryReport,
    sources: &[SourceInfo],
) -> Result<Vec<PublicationCoverage>, Failure> {
    let excluded: BTreeSet<(&str, &str)> = inv
        .out_of_scope
        .iter()
        .map(|record| (record.path.as_str(), record.key.as_str()))
        .collect();
    let mut upstream: BTreeMap<String, u32> = BTreeMap::new();
    for record in &inv.records {
        if excluded.contains(&(record.path.as_str(), record.key.as_str())) {
            continue;
        }
        for publication in &record.publications {
            let state = states.get(publication).ok_or_else(|| {
                failure(format!(
                    "publication {publication:?} has no registry row while counting coverage"
                ))
            })?;
            *upstream.entry(state.source.clone()).or_default() += 1;
        }
    }
    for (book, count) in &carried.by_book {
        *upstream.entry(book.clone()).or_default() += *count as u32;
    }
    let mut included: BTreeMap<String, u32> = BTreeMap::new();
    for entry in entries {
        for source in &entry.source_ids {
            *included.entry(source.clone()).or_default() += 1;
        }
    }
    let titles: BTreeMap<String, String> = sources
        .iter()
        .map(|source| (source.id.clone(), source.title.clone()))
        .chain(
            states
                .values()
                .map(|state| (state.source.clone(), state.title.clone())),
        )
        .collect();
    let mut ids: BTreeSet<String> = upstream.keys().cloned().collect();
    ids.extend(included.keys().cloned());
    Ok(ids
        .into_iter()
        .map(|source_id| PublicationCoverage {
            title: titles
                .get(&source_id)
                .cloned()
                .unwrap_or_else(|| source_id.clone()),
            upstream_records: upstream.get(&source_id).copied().unwrap_or(0),
            included_entries: included.get(&source_id).copied().unwrap_or(0),
            source_id,
        })
        .collect())
}

fn canonical_links(entries: &[RuleEntry]) -> cast_iron_schema::LinkCoverage {
    cast_iron_schema::LinkCoverage {
        resolved: entries.iter().map(|entry| entry.links.len() as u32).sum(),
        out_of_selection: 0,
        broken: 0,
    }
}

fn canonical_coverage_rows(
    inv: &Inventory,
    converted: &canonical::Converted,
    carried: &carry::CarryReport,
    settlement: &Settlement,
    alias_retirements: AliasRetirements,
) -> Vec<Exclusion> {
    let mut rows = Vec::new();
    let mut pi_by_category: BTreeMap<&str, usize> = BTreeMap::new();
    for omission in &inv.pi_omissions {
        *pi_by_category
            .entry(omission.category.as_str())
            .or_default() += 1;
    }
    if !inv.pi_omissions.is_empty() {
        rows.push(Exclusion {
            path: "pi-screen".into(),
            records: inv.pi_omissions.len() as u32,
            reason: format!(
                "Product Identity and outbound-link omissions across the in-scope inventory: {}",
                pi_by_category
                    .iter()
                    .map(|(category, count)| format!("{category} {count}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    for row in &converted.coverage {
        rows.push(Exclusion {
            path: row.record.clone(),
            records: row.occurrences as u32,
            reason: crate::pf_plain::plain_text(&format!(
                "converter coverage [{}] {}: {}",
                row.category, row.detail, row.reason
            )),
        });
    }
    let mut findings: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in &converted.findings {
        *findings.entry(finding.severity.as_str()).or_default() += 1;
    }
    if !converted.findings.is_empty() {
        rows.push(Exclusion {
            path: "converter-findings".into(),
            records: converted.findings.len() as u32,
            reason: format!(
                "converter findings, by severity: {}",
                findings
                    .iter()
                    .map(|(severity, count)| format!("{severity} {count}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    if carried.carried > 0 {
        rows.push(Exclusion {
            path: "carry-over".into(),
            records: carried.carried as u32,
            reason: format!(
                "PSRD carry-over under ref/carried/; references rewritten {}, flattened {}; {} possible duplicate review leads",
                carried.links.rewritten,
                carried.links.flattened,
                carried.possible_duplicate_count
            ),
        });
        for (book, count) in &carried.by_book {
            rows.push(Exclusion {
                path: format!("carry-over/{book}"),
                records: *count as u32,
                reason: "PSRD carry-over; no canonical Pf Data record replaces the legacy entry"
                    .into(),
            });
        }
    }
    if settlement.total() > 0 {
        rows.push(Exclusion {
            path: "legal-reference-settlement".into(),
            records: settlement.total(),
            reason: format!(
                "references to legally excluded records were flattened or dropped (body {}, parent {}, relation {}, prerequisite {})",
                settlement.body,
                settlement.parents,
                settlement.relations,
                settlement.prerequisites
            ),
        });
    }
    if alias_retirements.rights > 0 {
        rows.push(Exclusion {
            path: "alias-rights-retirements".into(),
            records: alias_retirements.rights as u32,
            reason: "legacy aliases whose canonical target has no Section 15 notice now resolve as retired for rights".into(),
        });
    }
    if alias_retirements.fallbacks > 0 {
        rows.push(Exclusion {
            path: "alias-fallback-carry-over".into(),
            records: alias_retirements.fallbacks as u32,
            reason:
                "legacy alias targets were planned from the pin but the converter did not emit them; the PSRD entries are carried under ref/carried/ so no OGC is lost"
                    .into(),
        });
    }
    if alias_retirements.unrepresentable > 0 {
        rows.push(Exclusion {
            path: "alias-unrepresentable-retirements".into(),
            records: alias_retirements.unrepresentable as u32,
            reason: "legacy aliases with no emitted target and no PSRD entry to carry now resolve as retired".into(),
        });
    }
    for record in &inv.out_of_scope {
        rows.push(Exclusion {
            path: format!("{}#{}", record.path, record.key),
            records: 1,
            reason: format!("out of scope: {}", record.reason),
        });
    }
    rows
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
    sources: &[SourceInfo],
) -> Vec<PublicationCoverage> {
    let mut by_slug: BTreeMap<String, (String, u32, u32)> = BTreeMap::new();
    for (raw, count) in source_counts {
        let slug = slug_segment(raw);
        let entry = by_slug.entry(slug).or_insert_with(|| (raw.clone(), 0, 0));
        entry.1 += count;
    }
    for s in sources {
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

/// Adds or replaces one file in an existing library and rewrites `manifest.json` with the
/// same deterministic version rule that [`compile`] uses. Returns the library version.
///
/// The alias map is generated after the content compile because it needs the pinned Pf Data
/// inventory, so this is how it joins the library that the client installs. The new version
/// directory replaces the old one; no file outside the manifest is copied.
pub fn publish_extra_file(out: &Path, name: &str, bytes: Vec<u8>) -> Result<String, String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        return Err(format!(
            "{name:?} is not a file name relative to the version directory"
        ));
    }
    let manifest: ContentManifest = serde_json::from_slice(
        &fs::read(out.join("manifest.json"))
            .map_err(|e| format!("cannot read {}: {e}", out.join("manifest.json").display()))?,
    )
    .map_err(|e| format!("invalid manifest.json: {e}"))?;
    let old_version = manifest.content_version;
    let old_prefix = format!("{old_version}/");
    let old_dir = out.join(&old_version);
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for file in &manifest.files {
        let relative = file
            .path
            .strip_prefix(&old_prefix)
            .ok_or_else(|| format!("manifest path {} is outside {old_version}", file.path))?;
        files.insert(
            relative.to_string(),
            fs::read(old_dir.join(relative))
                .map_err(|e| format!("cannot read {}: {e}", old_dir.join(relative).display()))?,
        );
    }
    files.insert(name.to_string(), bytes);

    let hashed: Vec<(String, String, u64)> = files
        .iter()
        .map(|(path, bytes)| (path.clone(), sha256_hex(bytes), bytes.len() as u64))
        .collect();
    let version_lines: String = hashed
        .iter()
        .map(|(path, hash, _)| format!("{path}:{hash}\n"))
        .collect();
    let content_version = format!("v{}", &hex(&Sha256::digest(version_lines.as_bytes()))[..16]);
    let new_dir = out.join(&content_version);
    if content_version != old_version && new_dir.exists() {
        fs::remove_dir_all(&new_dir)
            .map_err(|e| format!("cannot clear {}: {e}", new_dir.display()))?;
    }
    for (relative, bytes) in &files {
        write_file(&new_dir.join(relative), bytes)?;
    }
    let updated = ContentManifest {
        schema_version: manifest.schema_version,
        content_version: content_version.clone(),
        total_bytes: hashed.iter().map(|(_, _, bytes)| bytes).sum(),
        files: hashed
            .iter()
            .map(|(path, hash, bytes)| ManifestFile {
                path: format!("{content_version}/{path}"),
                sha256: hash.clone(),
                bytes: *bytes,
            })
            .collect(),
        entry_count: manifest.entry_count,
        datasets: manifest.datasets,
    };
    write_file(&out.join("manifest.json"), &json_bytes(&updated)?)?;
    if content_version != old_version && old_dir.exists() {
        fs::remove_dir_all(&old_dir)
            .map_err(|e| format!("cannot remove {}: {e}", old_dir.display()))?;
    }
    Ok(content_version)
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
    supplement_prefix: Option<&str>,
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
        coverage_markdown(coverage, notes, dedupe, survey, supplement_prefix).as_bytes(),
    )
}

fn coverage_markdown(
    c: &CoverageReport,
    notes: &[String],
    dedupe: &DedupeStats,
    survey: &select::Survey,
    supplement_prefix: Option<&str>,
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
    let _ = writeln!(
        s,
        "\nUpstream counts come from the PSRD-Data selection. Included entries also contain reviewed primary-source records."
    );
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
    let _ = writeln!(s, "\n## Referenced by (reverse body links)\n");
    let _ = writeln!(
        s,
        "- rows: {}\n- targets: {}\n- self-links excluded: {}\n- links that repeat a typed relation: {}\n- bytes: {}",
        c.backlinks.links,
        c.backlinks.targets,
        c.backlinks.excluded_self,
        c.backlinks.excluded_relation,
        c.backlinks.bytes
    );
    let _ = writeln!(
        s,
        "\nA self-link is dropped because the reader is already on that page. A link whose source\n\
         states a typed relation to the same target is dropped because the relation panels show\n\
         that edge. One row is kept per source and target; a source that reaches the target\n\
         through several anchors keeps no anchor."
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
    let (supplement, upstream): (Vec<_>, Vec<_>) = c
        .excluded
        .iter()
        .partition(|e| supplement_prefix.is_some_and(|prefix| e.path.starts_with(prefix)));
    let _ = writeln!(s, "\n## Excluded upstream directories\n");
    let _ = writeln!(
        s,
        "The dataset holds {} JSON files in total.\n",
        survey.total_files
    );
    let _ = writeln!(s, "| Path | Records | Reason |\n|---|---:|---|");
    for e in upstream {
        let _ = writeln!(s, "| {} | {} | {} |", e.path, e.records, e.reason);
    }
    if !supplement.is_empty() {
        let records: u64 = supplement.iter().map(|e| u64::from(e.records)).sum();
        let _ = writeln!(
            s,
            "\n## Reviewed supplemental source exclusions\n\n\
             The pinned primary inventory excludes {records} records. Approved records are listed under Entries by kind.\n"
        );
        let _ = writeln!(s, "| Path | Records | Reason |\n|---|---:|---|");
        for e in supplement {
            let _ = writeln!(s, "| {} | {} | {} |", e.path, e.records, e.reason);
        }
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
    fn publish_extra_file_joins_the_manifest_and_is_byte_identical_on_a_second_run() {
        let dir = std::env::temp_dir().join(format!("ci-publish-extra-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        prepare_out(&dir).unwrap();
        let version = "v0123456789abcdef";
        let catalog = b"{\"sources\":[],\"entries\":[]}".to_vec();
        let file = ManifestFile {
            path: format!("{version}/catalog.json"),
            sha256: sha256_hex(&catalog),
            bytes: catalog.len() as u64,
        };
        let manifest = ContentManifest {
            schema_version: SCHEMA_VERSION,
            content_version: version.into(),
            total_bytes: file.bytes,
            files: vec![file],
            entry_count: 0,
            datasets: vec![],
        };
        write_file(&dir.join(version).join("catalog.json"), &catalog).unwrap();
        write_file(&dir.join("manifest.json"), &json_bytes(&manifest).unwrap()).unwrap();

        let aliases = b"{\"format\":1}\n".to_vec();
        let first = publish_extra_file(&dir, "aliases.json", aliases.clone()).unwrap();
        assert_ne!(first, version);
        let first_manifest = fs::read(dir.join("manifest.json")).unwrap();
        let published: ContentManifest = serde_json::from_slice(&first_manifest).unwrap();
        assert_eq!(published.content_version, first);
        assert_eq!(published.files.len(), 2);
        assert!(
            published
                .files
                .iter()
                .any(|file| file.path == format!("{first}/aliases.json"))
        );
        assert!(dir.join(&first).join("aliases.json").is_file());
        assert!(
            !dir.join(version).exists(),
            "the old version directory is removed"
        );

        let second = publish_extra_file(&dir, "aliases.json", aliases).unwrap();
        assert_eq!(second, first);
        assert_eq!(fs::read(dir.join("manifest.json")).unwrap(), first_manifest);
        fs::remove_dir_all(&dir).unwrap();
    }

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
mod canonical_tests {
    use super::*;
    use cast_iron_schema::{EntryReference, Provenance};

    fn entry(id: &str, source: &str) -> RuleEntry {
        RuleEntry {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Rule,
            title: id.into(),
            source_ids: vec![source.into()],
            section: vec![],
            parent: None,
            body: vec![],
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

    fn state(source: &str, title: &str, noticed: bool) -> PublicationState {
        PublicationState {
            source: source.into(),
            title: title.into(),
            notices: noticed
                .then(|| pf_legal::Notice {
                    title: title.into(),
                    notice: "notice".into(),
                })
                .into_iter()
                .collect(),
            pending: false,
        }
    }

    #[test]
    fn legal_screen_keeps_only_noticed_publications_and_reports_the_rest() {
        let mut states = BTreeMap::new();
        states.insert(
            "Noticed Book".into(),
            state("noticed-book", "Noticed Book", true),
        );
        states.insert(
            "Unnoticed Book".into(),
            state("unnoticed-book", "Unnoticed Book", false),
        );
        let mut entries = vec![
            entry("ref/rule/both", "noticed-book"),
            entry("ref/rule/only-bad", "unnoticed-book"),
        ];
        entries[0].source_ids = vec!["noticed-book".into(), "unnoticed-book".into()];
        let mut rejected = Vec::new();
        let excluded = apply_legal_screen(&mut entries, &states, &mut rejected).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_ids, ["noticed-book"]);
        assert_eq!(excluded, BTreeSet::from(["ref/rule/only-bad".to_string()]));
        assert_eq!(rejected.len(), 1);
        assert!(rejected[0].reason.contains("Section 15"));
    }

    #[test]
    fn legal_screen_keeps_a_reviewed_notice_pending_publication() {
        let mut states = BTreeMap::new();
        let mut pending = state("pending-book", "Pending Book", false);
        pending.pending = true;
        states.insert("Pending Book".into(), pending);
        let mut entries = vec![entry("ref/rule/pending", "pending-book")];
        let mut rejected = Vec::new();
        let excluded = apply_legal_screen(&mut entries, &states, &mut rejected).unwrap();
        assert!(excluded.is_empty());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_ids, ["pending-book"]);
        assert!(rejected.is_empty());
    }

    #[test]
    fn settlement_flattens_a_link_to_a_missing_record() {
        let mut source = entry("ref/rule/host", "book");
        source.body = vec![Block::Paragraph {
            style: ParagraphStyle::Normal,
            content: vec![Inline::Link {
                target: EntryId::parse("ref/rule/missing").unwrap(),
                anchor: None,
                c: vec![Inline::Text {
                    v: "visible text".into(),
                }],
            }],
        }];
        source.links = vec![EntryReference {
            target: EntryId::parse("ref/rule/missing").unwrap(),
            anchor: None,
        }];
        let ids = BTreeSet::from([source.id.clone()]);
        let mut entries = vec![source];
        let settlement = settle_references(&mut entries, &ids);
        assert_eq!(settlement.body, 1);
        assert!(entries[0].links.is_empty());
        let Block::Paragraph { content, .. } = &entries[0].body[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(
            content,
            &[Inline::Text {
                v: "visible text".into()
            }]
        );
    }

    #[test]
    fn earliest_attribution_becomes_the_primary_publication() {
        let mut states = BTreeMap::new();
        states.insert(
            "Mythic Adventures".into(),
            state("mythic-adventures", "Mythic Adventures", true),
        );
        states.insert(
            "PRPG Core Rulebook".into(),
            state("core-rulebook", "PRPG Core Rulebook", true),
        );
        let mut entries = vec![entry("ref/spell/fireball", "mythic-adventures")];
        entries[0].source_ids = vec!["mythic-adventures".into(), "core-rulebook".into()];
        let pages = BTreeMap::from([(
            EntryId::parse("ref/spell/fireball").unwrap(),
            vec![("PRPG Core Rulebook".to_string(), Some(283))],
        )]);
        assert_eq!(order_primary_sources(&mut entries, &pages, &states), 1);
        assert_eq!(
            entries[0].source_ids,
            ["core-rulebook", "mythic-adventures"]
        );
    }

    #[test]
    fn canonical_copyright_keeps_each_notice_once_without_added_titles() {
        let rows = BTreeSet::from([
            (
                "Pathfinder RPG Core Rulebook".into(),
                "Pathfinder RPG Core Rulebook, © 2009, Paizo.".into(),
            ),
            (
                "Pathfinder RPG Core Rulebook".into(),
                "Pathfinder RPG Core Rulebook. Copyright 2009, Paizo.".into(),
            ),
            (
                "Pathfinder RPG Core Rulebook".into(),
                "Pathfinder RPG Core Rulebook, © 2009, Paizo.".into(),
            ),
            (
                "Pathfinder Roleplaying Game Core Rulebook".into(),
                "© 2010, Paizo.".into(),
            ),
        ]);
        assert_eq!(
            canonical_copyright(&rows),
            "Pathfinder RPG Core Rulebook, © 2009, Paizo.; Pathfinder RPG Core Rulebook. Copyright 2009, Paizo.; © 2010, Paizo."
        );
    }

    #[test]
    fn canonical_legal_has_one_notice_per_dataset() {
        let psrd = LegalNotices {
            licenses: vec![],
            product_identity: "pi".into(),
            community_use_notice: "cup".into(),
            section_15: vec![CopyrightNotice {
                title: "A".into(),
                notice: "one".into(),
            }],
            notice_pending: vec![],
            datasets: vec![DatasetNotice {
                id: "pathfinder-data-1-e".into(),
                repository: "r".into(),
                commit: "old".into(),
                note: "old note".into(),
            }],
        };
        let report = pf_legal::LegalReport {
            dataset: "d".into(),
            commit: "c".into(),
            in_scope_records: 0,
            publication_count: 0,
            matched_publication_count: 0,
            pending_publication_count: 0,
            unmatched_publication_count: 0,
            publications: vec![],
            pending_publications: vec![],
            excluded_publications: vec![],
            notice_sources: vec![],
            unparsed_notices: vec![],
            unmatched_notices: vec![],
        };
        let pin = pf_data::SourcePin {
            id: "pathfinder-data-1-e".into(),
            repository: "r".into(),
            commit: "new".into(),
            archive_sha256: "0".repeat(64),
            files: BTreeMap::new(),
        };
        let legal = canonical_legal(&psrd, &report, &pin);
        assert_eq!(legal.datasets.len(), 1);
        assert_eq!(legal.datasets[0].commit, "new");
        assert_eq!(legal.section_15.len(), 1);
    }

    #[test]
    fn facets_come_from_typed_fields_and_archetype_edges() {
        let mut spell = entry("ref/spell/fireball", "s");
        spell.kind = EntryKind::Spell;
        spell.fields = TypedFields::Spell(cast_iron_schema::SpellFields {
            school: "evocation".into(),
            descriptors: vec!["fire".into()],
            levels: vec![cast_iron_schema::SpellLevel {
                class: "Wizard".into(),
                level: 3,
            }],
            ..Default::default()
        });
        let mut archetype = entry("ref/arc-bard/buccaneer", "s");
        archetype.kind = EntryKind::Archetype;
        archetype.relations = vec![cast_iron_schema::Relation {
            kind: RelationKind::Archetype,
            target: EntryId::parse("ref/class/bard").unwrap(),
            label: None,
        }];
        let facets = canonical_facets(&[spell, archetype]);
        assert_eq!(
            facets[&EntryId::parse("ref/spell/fireball").unwrap()],
            vec![
                ("school".into(), "evocation".into()),
                ("descriptor".into(), "fire".into()),
                ("class".into(), "Wizard".into()),
                ("spell_level".into(), "3".into()),
            ]
        );
        assert_eq!(
            facets[&EntryId::parse("ref/arc-bard/buccaneer").unwrap()],
            vec![("archetype_class".into(), "bard".into())]
        );
    }

    #[test]
    fn school_facet_counts_a_noted_school_under_the_school() {
        let mut spell = entry("ref/spell/align-weapon", "s");
        spell.kind = EntryKind::Spell;
        spell.fields = TypedFields::Spell(cast_iron_schema::SpellFields {
            school: "transmutation [see text]".into(),
            ..Default::default()
        });
        let facets = canonical_facets(&[spell]);
        assert_eq!(
            facets[&EntryId::parse("ref/spell/align-weapon").unwrap()],
            vec![("school".into(), "transmutation".into())]
        );
    }

    #[test]
    fn markup_gate_fails_on_a_leaked_tag_and_names_the_entry() {
        let mut clean = entry("ref/rule/clean", "s");
        clean.title = "Clean".into();
        let mut leaky = entry("ref/rule/leaky", "s");
        leaky.title = "Leaky ‹rule/x›".into();
        let coverage = CoverageReport::default();
        let facets = BTreeMap::new();
        let aliases = BTreeMap::new();
        assert!(check_no_markup_leaks(&[clean.clone()], &facets, &aliases, &coverage).is_ok());
        let error =
            check_no_markup_leaks(&[clean, leaky], &facets, &aliases, &coverage).unwrap_err();
        let text = error.lines.join("\n");
        assert!(text.contains("ref/rule/leaky"), "{text}");
        assert!(
            text.contains("2 occurrence") || text.contains("1 occurrence"),
            "{text}"
        );
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
