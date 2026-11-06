//! The tail of every compile: validate a built [`Library`], shard it, write the reports and the
//! search index, and publish a version directory and manifest.
//!
//! [`crate::corpus_build`] reads the corpus and fills a [`Library`]. [`package`] holds no
//! knowledge of where the records came from.

use crate::text::{blocks_text, inline_text};
use cast_iron_schema::validate::{ValidationError, validate_derived, validate_library};
use cast_iron_schema::{
    BACKLINKS_DIR, BACKLINKS_FILE, BOOKS_FILE, Block, BookIndex, Catalog, ContentManifest,
    CoverageReport, DatasetRef, EntryId, EntryKind, EntryStub, LegalNotices, ManifestFile,
    RELATIONS_FILE, Rejection, RelationIndex, RelationKind, RuleEntry, SCHEMA_VERSION,
    SPELL_LISTS_FILE, SourceInfo, SpellLists, TypedFields,
};
use cast_iron_search::{DocInput, IndexBuilder};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub const SHARD_TARGET_BYTES: usize = 512 * 1024;

/// Name of the alias file, relative to the content version directory.
pub const ALIASES_FILE: &str = "aliases.json";

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// What a compile reports to the caller.
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

/// The content that [`package`] writes.
///
/// The builder fills every field, including the search aliases and facets, so that the
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
    /// The legacy-to-canonical alias file (`aliases.json`), when the corpus has one.
    /// [`package`] adds it to the version directory and the manifest, so the content version
    /// covers its bytes.
    pub aliases_file: Option<Vec<u8>>,
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

    write_reports(reports, &errors, &coverage, &notes, &catalog)?;
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
        files.insert(ALIASES_FILE.into(), aliases);
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
        notes: notes.len(),
        relations: coverage.relations.clone(),
        derived_bytes,
        backlinks: coverage.backlinks.clone(),
    })
}

/// Search facets for the library. Entries are already typed, so the values are read from
/// [`TypedFields`] instead of a second representation of the record.
///
/// A race has no `race_group` facet: the entry does not carry its group.
pub(crate) fn canonical_facets(entries: &[RuleEntry]) -> BTreeMap<EntryId, Vec<(String, String)>> {
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
                push(&mut facets, "school", &school_facet(&fields.school));
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

/// Fails the build when inline markup from the retired import (`‹`, `›`, `&#`) is left in any
/// compiled text: entry titles, fields, body text, facets, aliases, or the coverage report.
/// Field and body text must carry display text only.
pub(crate) fn check_no_markup_leaks(
    entries: &[RuleEntry],
    facets: &BTreeMap<EntryId, Vec<(String, String)>>,
    aliases: &BTreeMap<EntryId, Vec<String>>,
    coverage: &CoverageReport,
) -> Result<(), Failure> {
    let mut lines = Vec::new();
    let mut total = 0usize;
    let mut check = |owner: &str, what: &str, json: Result<String, serde_json::Error>| {
        let json = json.unwrap_or_default();
        let leaks = markup_leaks(&json);
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
        "inline markup left in compiled output: {total} occurrence(s) of `‹`, `›` or `&#`"
    )];
    all.append(&mut lines);
    Err(Failure { lines: all })
}

pub(crate) fn failure(message: impl Into<String>) -> Failure {
    Failure {
        lines: vec![message.into()],
    }
}

pub(crate) fn search_aliases(entries: &[RuleEntry]) -> BTreeMap<EntryId, Vec<String>> {
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

fn write_reports(
    dir: &Path,
    errors: &[ValidationError],
    coverage: &CoverageReport,
    notes: &[String],
    catalog: &Catalog,
) -> Result<(), String> {
    let validation = serde_json::json!({
        "ok": errors.is_empty(),
        "error_count": errors.len(),
        "errors": errors,
        "entry_count": catalog.entries.len(),
        "rejected": coverage.rejected,
        "notes": notes,
    });
    let pretty = serde_json::to_string_pretty(&validation).map_err(|e| e.to_string())?;
    write_file(
        &dir.join("validation.json"),
        format!("{pretty}\n").as_bytes(),
    )?;
    write_file(
        &dir.join("coverage.md"),
        coverage_markdown(coverage, notes).as_bytes(),
    )
}

fn coverage_markdown(c: &CoverageReport, notes: &[String]) -> String {
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
        "\n## Publications\n\n| Source | Title | Imported records | Included entries |\n|---|---|---:|---:|"
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
    let _ = writeln!(s, "\n## Excluded from the import\n");
    let _ = writeln!(s, "| Path | Records | Reason |\n|---|---:|---|");
    for e in &c.excluded {
        let _ = writeln!(s, "| {} | {} | {} |", e.path, e.records, e.reason);
    }
    s
}

/// Where inline markup from the retired import is left in `text`: each `‹`, `›` or `&#`, with
/// a few characters of context. Empty when the text is clean.
fn markup_leaks(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, _) in text
        .match_indices(['‹', '›'])
        .chain(text.match_indices("&#"))
    {
        let from = text[..index]
            .char_indices()
            .rev()
            .nth(30)
            .map_or(0, |(i, _)| i);
        let to = text[index..]
            .char_indices()
            .nth(40)
            .map_or(text.len(), |(i, _)| index + i);
        found.push(text[from..to].replace('\n', " "));
    }
    found
}

/// The facet value for a spell school: the school word alone, lowercased. A parenthetical
/// subschool and a bracketed note both end the value, so `Conjuration (summoning) [see text]`
/// counts under `conjuration` and `multiple (see text)` under `multiple`. Facet values are
/// exact-match filter and route keys; the client capitalizes them for display.
///
/// Only the school field carries these notes. Do not use this for other facets, where a
/// bracket or parenthesis can be part of the value.
pub(crate) fn school_facet(school: &str) -> String {
    let end = school.find(['[', '(']).unwrap_or(school.len());
    school[..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{Inline, Provenance};

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

    #[test]
    fn facets_come_from_typed_fields_and_archetype_edges() {
        let mut spell = entry("ref/spell/fireball", EntryKind::Spell, 0);
        spell.fields = TypedFields::Spell(cast_iron_schema::SpellFields {
            school: "evocation".into(),
            descriptors: vec!["fire".into()],
            levels: vec![cast_iron_schema::SpellLevel {
                class: "Wizard".into(),
                level: 3,
            }],
            ..Default::default()
        });
        let mut archetype = entry("ref/arc-bard/buccaneer", EntryKind::Archetype, 0);
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
        let mut spell = entry("ref/spell/align-weapon", EntryKind::Spell, 0);
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
    fn school_facet_drops_the_bracketed_note_and_the_subschool() {
        assert_eq!(school_facet("transmutation [see text]"), "transmutation");
        assert_eq!(school_facet("Transmutation [see text]"), "transmutation");
        assert_eq!(school_facet("evocation"), "evocation");
        assert_eq!(school_facet("  conjuration   [healing]  "), "conjuration");
        for (school, facet) in [
            ("conjuration (summoning) [see text]", "conjuration"),
            ("Conjuration (Creation Or Calling)", "conjuration"),
            ("multiple (see text)", "multiple"),
        ] {
            assert_eq!(school_facet(school), facet, "{school}");
        }
    }

    #[test]
    fn markup_leaks_reports_each_marker_with_context() {
        assert!(markup_leaks("clean [text] & more").is_empty());
        assert_eq!(markup_leaks("a ‹b› c &#91& d").len(), 3);
    }

    #[test]
    fn markup_gate_fails_on_a_leaked_tag_and_names_the_entry() {
        let mut clean = entry("ref/rule/clean", EntryKind::Rule, 0);
        clean.title = "Clean".into();
        let mut leaky = entry("ref/rule/leaky", EntryKind::Rule, 0);
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
