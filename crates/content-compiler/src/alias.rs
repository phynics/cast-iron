//! Legacy-to-canonical identifier aliases.
//!
//! Phase D of `docs/research/canonical-id-design.md`. The canonical cutover
//! renumbers public entries under the `ref/` namespace, so every identifier that
//! a bookmark can already hold needs a reviewed disposition: either a canonical
//! target or an explicit retirement.
//!
//! Three committed artifacts form the contract:
//!
//! * `sources/canonical/legacy-ids.json` — frozen snapshot of the published
//!   identifier set with kind, title, and source. It is the totality oracle.
//! * `sources/canonical/id-map.toml` — append-only disposition of every legacy
//!   identifier: `[map]`, `[shared]`, or `[retired]`.
//! * `generated/content/<version>/aliases.json` — the runtime map, generated
//!   deterministically from the map and the canonical catalog.
//!
//! Every gate fails the command instead of guessing. A legacy identifier that is
//! missing from the map, a canonical record claimed by two identifiers without an
//! explicit `[shared]` row, a target that the pinned corpus cannot produce, and a
//! `[map]` row whose legacy title or source book does not fit the canonical record
//! are all build errors. A shared slug is never enough: the same word appears in
//! unrelated books.

use crate::pf_data::{InputRecord, Inventory};
use crate::pf_rules::RuleAllowlist;
use crate::select::sha256_hex;
use crate::slug::slug_segment;
use cast_iron_schema::EntryId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Version of the generated alias file. Independent of `SCHEMA_VERSION` and
/// `INDEX_FORMAT_VERSION`, as the design requires.
pub const ALIAS_FORMAT: u32 = 1;

/// Name of the generated alias file, relative to the content version directory.
pub const ALIASES_FILE: &str = "aliases.json";

/// Why a legacy identifier has no canonical target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementReason {
    Rights,
    OutOfScope,
    NoSource,
    Unrepresentable,
}

impl RetirementReason {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Rights => "rights",
            Self::OutOfScope => "out_of_scope",
            Self::NoSource => "no_source",
            Self::Unrepresentable => "unrepresentable",
        }
    }
}

/// The `[meta]` block. The snapshot hash binds the map to the frozen oracle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdMapMeta {
    pub format: u32,
    pub legacy_count: usize,
    pub legacy_sha256: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// One `[map]` row. Exactly one of `record`, `derived`, and `target` is present.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapRow {
    /// Upstream identity `path#key` of a native canonical record.
    #[serde(default)]
    pub record: Option<String>,
    /// Upstream identity `path#key|<child-kind>:<slug>` of a derived record.
    #[serde(default)]
    pub derived: Option<String>,
    /// An explicit canonical identifier, used when no derivation exists.
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl MapRow {
    fn dispositions(&self) -> usize {
        usize::from(self.record.is_some())
            + usize::from(self.derived.is_some())
            + usize::from(self.target.is_some())
    }
}

/// One `[shared]` row: a legacy identifier merged onto a target that at least two
/// legacy ids name.
///
/// Every legacy id that names a merged target must be a `[shared]` row, including
/// the first one. Injectivity counts claims per target, not per section, so a
/// target named by two ids fails unless each of those ids is declared here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedRow {
    pub target: String,
    pub note: String,
}

/// One `[retired]` row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetiredRow {
    pub reason: RetirementReason,
    #[serde(default)]
    pub note: Option<String>,
}

/// `sources/canonical/id-map.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdMap {
    pub meta: IdMapMeta,
    #[serde(default)]
    pub map: BTreeMap<String, MapRow>,
    #[serde(default)]
    pub shared: BTreeMap<String, SharedRow>,
    #[serde(default)]
    pub retired: BTreeMap<String, RetiredRow>,
}

impl IdMap {
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("invalid {}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let map: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        map.validate()?;
        Ok(map)
    }

    fn validate(&self) -> Result<(), String> {
        if self.meta.format != ALIAS_FORMAT {
            return Err(format!(
                "meta.format is {}; this compiler reads {ALIAS_FORMAT}",
                self.meta.format
            ));
        }
        if self.meta.legacy_sha256.len() != 64
            || !self
                .meta
                .legacy_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err("meta.legacy_sha256 must be 64 hexadecimal characters".into());
        }
        if self.meta.legacy_sha256 != self.meta.legacy_sha256.to_ascii_lowercase() {
            return Err("meta.legacy_sha256 must be lowercase".into());
        }
        for (old, row) in &self.map {
            check_old(old)?;
            if row.dispositions() != 1 {
                return Err(format!(
                    "map row {old:?} must set exactly one of record, derived, or target"
                ));
            }
            for (name, value) in [
                ("record", row.record.as_deref()),
                ("derived", row.derived.as_deref()),
                ("target", row.target.as_deref()),
            ] {
                if let Some(value) = value {
                    if value.trim().is_empty() {
                        return Err(format!("map row {old:?} has an empty {name}"));
                    }
                    if name == "target" {
                        EntryId::parse(value)
                            .map_err(|e| format!("map row {old:?} target: {e}"))?;
                    }
                }
            }
        }
        for (old, row) in &self.shared {
            check_old(old)?;
            EntryId::parse(&row.target).map_err(|e| format!("shared row {old:?} target: {e}"))?;
            if row.note.trim().is_empty() {
                return Err(format!("shared row {old:?} needs a note"));
            }
        }
        for old in self.retired.keys() {
            check_old(old)?;
        }
        for old in self.map.keys() {
            if self.shared.contains_key(old) || self.retired.contains_key(old) {
                return Err(format!(
                    "legacy id {old:?} appears in more than one section"
                ));
            }
        }
        for old in self.shared.keys() {
            if self.retired.contains_key(old) {
                return Err(format!(
                    "legacy id {old:?} appears in more than one section"
                ));
            }
        }
        Ok(())
    }
}

fn check_old(old: &str) -> Result<(), String> {
    let id = EntryId::parse(old).map_err(|e| format!("legacy id {old:?}: {e}"))?;
    if id.as_str().starts_with("ref/") {
        return Err(format!(
            "{old:?} is in the canonical namespace; only legacy ids are mapped"
        ));
    }
    Ok(())
}

/// One row of `sources/canonical/legacy-ids.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyEntry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub source: String,
}

/// `sources/canonical/legacy-ids.json`, the frozen totality oracle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacySnapshot {
    pub format: u32,
    pub count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub entries: Vec<LegacyEntry>,
}

impl LegacySnapshot {
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::parse(&bytes).map_err(|e| format!("invalid {}: {e}", path.display()))
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let snapshot: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("legacy snapshot: {e}"))?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn validate(&self) -> Result<(), String> {
        if self.format != 1 {
            return Err(format!(
                "legacy snapshot format is {}; expected 1",
                self.format
            ));
        }
        if self.count != self.entries.len() {
            return Err(format!(
                "legacy snapshot count is {} but it holds {} entries",
                self.count,
                self.entries.len()
            ));
        }
        let mut previous: Option<&str> = None;
        for entry in &self.entries {
            let id =
                EntryId::parse(&entry.id).map_err(|e| format!("legacy snapshot entry: {e}"))?;
            if id.as_str().starts_with("ref/") {
                return Err(format!("legacy snapshot holds canonical id {:?}", entry.id));
            }
            if previous.is_some_and(|p| p >= entry.id.as_str()) {
                return Err(format!(
                    "legacy snapshot is not sorted or repeats {:?}",
                    entry.id
                ));
            }
            previous = Some(entry.id.as_str());
            for (name, value) in [
                ("kind", entry.kind.as_str()),
                ("title", entry.title.as_str()),
                ("source", entry.source.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(format!("legacy snapshot {} has an empty {name}", entry.id));
                }
            }
        }
        Ok(())
    }

    pub fn by_id(&self) -> BTreeMap<&str, &LegacyEntry> {
        self.entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry))
            .collect()
    }
}

/// One canonical record available to the cutover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalRecord {
    pub id: String,
    /// Upstream identity: `path#key`, or `path#key|<kind>:<slug>` for a child.
    pub identity: String,
    pub kind: String,
    pub title: String,
    /// Legacy book ids named by the record's publications, sorted and deduplicated.
    /// The semantic fit gate compares a legacy id's source against this set.
    pub source_ids: Vec<String>,
    /// `native`, `derived:<kind>`, or `carried`.
    pub origin: String,
}

/// Every canonical identifier the pinned corpus can produce, indexed by id and by
/// upstream identity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CanonicalCatalog {
    by_id: BTreeMap<String, CanonicalRecord>,
    by_identity: BTreeMap<String, String>,
}

impl CanonicalCatalog {
    /// Adds a record. A duplicate id or identity is an error: canonical derivation
    /// must stay injective, so a silent overwrite is never acceptable.
    pub fn insert(&mut self, record: CanonicalRecord) -> Result<(), String> {
        if let Some(existing) = self.by_id.get(&record.id) {
            return Err(format!(
                "canonical id {} is claimed by both {} and {}",
                record.id, existing.identity, record.identity
            ));
        }
        if let Some(existing) = self.by_identity.get(&record.identity) {
            return Err(format!(
                "upstream identity {} produces both {existing} and {}",
                record.identity, record.id
            ));
        }
        self.by_identity
            .insert(record.identity.clone(), record.id.clone());
        self.by_id.insert(record.id.clone(), record);
        Ok(())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn canonical_for_identity(&self, identity: &str) -> Option<&str> {
        self.by_identity.get(identity).map(String::as_str)
    }

    pub fn get(&self, id: &str) -> Option<&CanonicalRecord> {
        self.by_id.get(id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CanonicalRecord> {
        self.by_id.values()
    }
}

/// The canonical identifier of a Pf Data record, before the production cutover.
///
/// The design's native form is `ref/<family>/<key>`. `family` is the reviewed
/// route token from `routes.json`, which is also the record's `protocol`.
pub fn canonical_native_id(protocol: &str, key: &str) -> Result<String, String> {
    let family = slug_segment(protocol);
    let key = slug_segment(key);
    if family.is_empty() || key.is_empty() {
        return Err(format!(
            "cannot derive a canonical id from protocol {protocol:?} and key {key:?}"
        ));
    }
    let id = format!("ref/{family}/{key}");
    EntryId::parse(id.clone()).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Maps a canonical publication name (or its slug) to the legacy book id used by
/// `legacy-ids.json`.
///
/// The frozen snapshot names PSRD books (`core-rulebook`, `bestiary`,
/// `game-mastery-guide`); Pf Data names the same books with an edition prefix or a
/// joined word. These are the only reviewed spelling differences. Every other
/// publication keeps its slug, so an unknown book fails the fit gate instead of
/// being folded onto a legacy id by guesswork.
fn legacy_book_id(publication: &str) -> String {
    let slug = slug_segment(publication.trim());
    let mapped = match slug.as_str() {
        "prpg-core-rulebook" | "pathfinder-rpg-core-rulebook" => "core-rulebook",
        "pathfinder-rpg-bestiary" => "bestiary",
        "pathfinder-rpg-bestiary-2" => "bestiary-2",
        "pathfinder-rpg-bestiary-3" => "bestiary-3",
        "pathfinder-rpg-bestiary-4" => "bestiary-4",
        "gamemastery-guide" | "pathfinder-rpg-gamemastery-guide" => "game-mastery-guide",
        _ => return slug,
    };
    mapped.to_string()
}

fn book_ids<'a>(publications: impl Iterator<Item = &'a str>) -> Vec<String> {
    publications
        .map(legacy_book_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Comparison form for a title: lowercase alphanumeric words only.
///
/// Case, punctuation, and spacing differ between the two corpora; the words do
/// not. The form is deliberately strict about the words themselves, because a
/// title is half of what makes two records the same article.
pub fn normalize_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut pending_space = false;
    for ch in title.chars() {
        if ch.is_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            out.extend(ch.to_lowercase());
            pending_space = false;
        } else {
            pending_space = true;
        }
    }
    out
}

/// How a legacy title matched a canonical title, strongest evidence first.
///
/// Only the two corpora's documented spelling differences are accepted:
///
/// * `Exact` — the normalized words are identical.
/// * `Inversion` — a comma list such as `Dragon, Red` names `Red Dragon`. The
///   middle `Primal` and `Imperial` dragon-category words are dropped first, which
///   turns `Dragon, Imperial, Sky, Young` into `Young Sky Dragon`.
/// * `Parenthetical` — one title adds a trailing family/category label, as in
///   `Olethrodaemon` and `Olethrodaemon (Daemon)`. Two different parentheticals
///   never match. Only creature, trait, feat, and affliction titles may use this
///   form; an item parenthetical must be a measure or package (`Chain (10 ft.)`)
///   or the weapon/armor slot label resolved by the proposal tie-break.
/// * `InversionParenthetical` — both differences at once, as in
///   `Devil, Accuser` and `Accuser Devil (Zebub)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TitleFit {
    Exact,
    Inversion,
    Parenthetical,
    InversionParenthetical,
}

impl TitleFit {
    pub fn rank(self) -> u8 {
        match self {
            Self::Exact => 0,
            Self::Inversion => 1,
            Self::Parenthetical => 2,
            Self::InversionParenthetical => 3,
        }
    }
}

/// Kinds whose titles may add a trailing family or category label.
fn parenthetical_kind(kind: &str) -> bool {
    matches!(kind, "creature" | "trait" | "feat" | "affliction")
}

/// Words that make an item parenthetical a package or measure rather than a
/// different item. `Lock (average)` and `Tent (large)` are variants and must not
/// collapse onto a generic `Lock` or `Tent`.
fn item_parenthetical_is_measure(text: &str) -> bool {
    const MEASURE_WORDS: &[&str] = &[
        "ft",
        "foot",
        "feet",
        "in",
        "inch",
        "inches",
        "yd",
        "yard",
        "yards",
        "sq",
        "lb",
        "lbs",
        "pound",
        "pounds",
        "oz",
        "ounce",
        "ounces",
        "gallon",
        "gallons",
        "quart",
        "pint",
        "cup",
        "mug",
        "jar",
        "jug",
        "gourd",
        "bottle",
        "bottles",
        "wineskin",
        "sheet",
        "sheets",
        "bar",
        "dose",
        "doses",
        "handful",
        "handfuls",
        "vial",
        "vials",
        "flask",
        "keg",
        "barrel",
        "box",
        "case",
        "per",
        "day",
        "days",
        "hour",
        "hours",
        "minute",
        "minutes",
        "second",
        "seconds",
        "round",
        "rounds",
        "mile",
        "miles",
        "person",
        "persons",
        "block",
        "spool",
        "coil",
        "bundle",
        "set",
        "pair",
        "pack",
        "pouch",
        "sack",
        "bag",
        "cartridge",
        "bullet",
        "pellet",
        "charge",
        "use",
        "application",
        "serving",
    ];
    let mut saw_word = false;
    for token in text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        let token = token.to_ascii_lowercase();
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        // Spell levels, tech marks, and other numbered tiers are variants, not
        // measures.
        if token
            .strip_suffix("st")
            .or_else(|| token.strip_suffix("nd"))
            .or_else(|| token.strip_suffix("rd"))
            .or_else(|| token.strip_suffix("th"))
            .is_some_and(|digits| {
                !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
            })
        {
            return false;
        }
        if matches!(token.as_str(), "i" | "ii" | "iii" | "iv" | "v") {
            return false;
        }
        if !MEASURE_WORDS.contains(&token.as_str()) {
            return false;
        }
        saw_word = true;
    }
    saw_word
}

/// Splits one trailing parenthetical from a title. Nested or unterminated forms
/// are left alone.
fn split_parenthetical(title: &str) -> (&str, Option<&str>) {
    let trimmed = title.trim_end();
    if !trimmed.ends_with(')') {
        return (title, None);
    }
    let Some(start) = trimmed.rfind('(') else {
        return (title, None);
    };
    let inner = &trimmed[start + 1..trimmed.len() - 1];
    if start == 0 || inner.contains('(') || inner.contains(')') {
        return (title, None);
    }
    (trimmed[..start].trim_end(), Some(inner))
}

/// The comma-inversion spelling, with the PSRD dragon-category word removed.
fn invert_title(title: &str) -> Option<String> {
    let mut parts: Vec<&str> = title
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 {
        return None;
    }
    if parts.len() >= 3
        && matches!(
            parts[1].to_ascii_lowercase().as_str(),
            "primal" | "imperial"
        )
    {
        parts.remove(1);
    }
    Some(parts.into_iter().rev().collect::<Vec<_>>().join(" "))
}

fn title_parenthetical_allowed(kind: &str, title: &str) -> bool {
    let (_, paren) = split_parenthetical(title);
    let Some(paren) = paren else {
        return false;
    };
    if parenthetical_kind(kind) {
        return true;
    }
    if kind == "item" {
        let normalized = normalize_title(paren);
        return matches!(normalized.as_str(), "weapon" | "armor")
            || item_parenthetical_is_measure(paren);
    }
    false
}

/// The title half of the semantic fit gate. Returns the spelling difference that
/// made the titles equivalent, or `None` when they do not name the same article.
pub fn title_fit(legacy_title: &str, record_title: &str, kind: &str) -> Option<TitleFit> {
    let legacy_normalized = normalize_title(legacy_title);
    let record_normalized = normalize_title(record_title);
    if legacy_normalized == record_normalized {
        return Some(TitleFit::Exact);
    }
    if invert_title(legacy_title).is_some_and(|value| normalize_title(&value) == record_normalized)
        || invert_title(record_title)
            .is_some_and(|value| legacy_normalized == normalize_title(&value))
    {
        return Some(TitleFit::Inversion);
    }
    if !title_parenthetical_allowed(kind, legacy_title)
        && !title_parenthetical_allowed(kind, record_title)
    {
        return None;
    }
    let (legacy_base, legacy_paren) = split_parenthetical(legacy_title);
    let (record_base, record_paren) = split_parenthetical(record_title);
    if let (Some(left), Some(right)) = (legacy_paren, record_paren)
        && normalize_title(left) != normalize_title(right)
    {
        return None;
    }
    let legacy_base_normalized = normalize_title(legacy_base);
    let record_base_normalized = normalize_title(record_base);
    if legacy_base_normalized == record_base_normalized {
        return Some(TitleFit::Parenthetical);
    }
    if invert_title(legacy_base)
        .is_some_and(|value| normalize_title(&value) == record_base_normalized)
        || invert_title(record_base)
            .is_some_and(|value| legacy_base_normalized == normalize_title(&value))
    {
        return Some(TitleFit::InversionParenthetical);
    }
    None
}

/// Index forms for a title, with the spelling strength that produced each form.
/// The proposal tool and the semantic gate use this same function, so a proposed
/// row cannot pass one and fail the other.
pub(crate) fn title_forms(title: &str, kind: &str) -> BTreeMap<String, u8> {
    let mut forms = BTreeMap::new();
    let mut add = |form: String, rank: u8| {
        forms
            .entry(form)
            .and_modify(|current: &mut u8| *current = (*current).min(rank))
            .or_insert(rank);
    };
    add(normalize_title(title), TitleFit::Exact.rank());
    if let Some(inverted) = invert_title(title) {
        add(normalize_title(&inverted), TitleFit::Inversion.rank());
    }
    if title_parenthetical_allowed(kind, title) {
        let (base, _) = split_parenthetical(title);
        add(normalize_title(base), TitleFit::Parenthetical.rank());
        if let Some(inverted) = invert_title(base) {
            add(
                normalize_title(&inverted),
                TitleFit::InversionParenthetical.rank(),
            );
        }
    }
    forms
}

/// The semantic fit gate for one `[map]` row.
///
/// A generated map once paired legacy ids with canonical records that shared only a
/// slug: `environment/weather` pointed at the Ceremony spell's weather subsection,
/// and `class-archetypes/monk` at a weapon-group row. The title and the source book
/// together are the smallest reviewable evidence that the row names the same
/// article. Both must agree. `title_fit` allows only the documented spelling
/// differences between the corpora, so a title that merely shares a word still
/// fails.
fn check_semantic_fit(
    old: &str,
    legacy: &LegacyEntry,
    record: &CanonicalRecord,
    errors: &mut Vec<String>,
) {
    if !record
        .source_ids
        .iter()
        .any(|source| source == &legacy.source)
    {
        errors.push(format!(
            "map row {old:?}: legacy source {:?} is not among canonical {} sources [{}]; a shared slug is not a verified match",
            legacy.source,
            record.id,
            record.source_ids.join(", ")
        ));
    }
    if title_fit(&legacy.title, &record.title, &legacy.kind).is_none() {
        errors.push(format!(
            "map row {old:?}: legacy title {:?} does not match canonical {} title {:?}",
            legacy.title, record.id, record.title
        ));
    }
}

/// Builds the catalog from the pinned Pf Data inventory.
///
/// Native records are the ones the full import selects: in-scope records with a
/// protocol route and a description, copy, or compilation shape. Derived children
/// come from the same planners the importer runs, so the catalog cannot disagree
/// with the derivation. A record that fails conversion is still in the pinned
/// corpus; this gate asks whether the target *exists* in the pin, not whether the
/// importer has already accepted every field.
pub fn catalog_from_inventory(
    inv: &Inventory,
    allowlist: &RuleAllowlist,
) -> Result<CanonicalCatalog, String> {
    let out_of_scope: BTreeSet<(&str, &str)> = inv
        .out_of_scope
        .iter()
        .map(|record| (record.path.as_str(), record.key.as_str()))
        .collect();
    let in_scope =
        |record: &InputRecord| !out_of_scope.contains(&(record.path.as_str(), record.key.as_str()));
    let mut catalog = CanonicalCatalog::default();
    let mut selected = Vec::new();
    for record in inv.records.iter().filter(|record| in_scope(record)) {
        if ["description", "copy", "compilation"].contains(&record.shape.as_str())
            && let Some(protocol) = record.protocol.as_deref()
        {
            let id = canonical_native_id(protocol, &record.key)?;
            catalog.insert(CanonicalRecord {
                id,
                identity: format!("{}#{}", record.path, record.key),
                kind: crate::pf_data::record_kind(&record.path).slug().to_string(),
                title: title_of(record),
                source_ids: book_ids(record.publications.iter().map(String::as_str)),
                origin: "native".into(),
            })?;
            selected.push(record.clone());
        }
    }

    let mut rejections = Vec::new();
    for derivation in crate::pf_derived::plan(&selected, &mut rejections) {
        let suffix = derivation
            .provenance_record
            .split_once('|')
            .map(|(_, suffix)| suffix)
            .ok_or_else(|| format!("derived record {} has no provenance suffix", derivation.id))?;
        catalog.insert(CanonicalRecord {
            id: derivation.id.as_str().to_string(),
            identity: format!("{}|{suffix}", derivation.parent_identity),
            kind: derivation.kind.kind().slug().to_string(),
            title: derivation.title.clone(),
            source_ids: book_ids(derivation.source_ids.iter().map(String::as_str)),
            origin: format!("derived:{}", derivation.kind.slug()),
        })?;
    }

    for record in inv.records.iter().filter(|record| in_scope(record)) {
        let rules = allowlist.for_host(&record.path, &record.key);
        if rules.is_empty() {
            continue;
        }
        let Ok(lines) = description_lines(record) else {
            continue;
        };
        for extracted in crate::pf_rules::extract(&lines, rules) {
            catalog.insert(CanonicalRecord {
                id: extracted.id.clone(),
                identity: format!(
                    "{}#{}|{}:{}",
                    record.path,
                    record.key,
                    extracted.shape.slug(),
                    slug_segment(&extracted.title)
                ),
                kind: "rule".into(),
                title: extracted.title.clone(),
                source_ids: book_ids(record.publications.iter().map(String::as_str)),
                origin: "derived:rule".into(),
            })?;
        }
    }
    Ok(catalog)
}

fn title_of(record: &InputRecord) -> String {
    if record.name.is_empty() {
        record.key.clone()
    } else {
        record.name.clone()
    }
}

fn description_lines(record: &InputRecord) -> Result<Vec<String>, String> {
    record
        .raw
        .get("description")
        .and_then(serde_json::Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .ok_or_else(|| format!("{}#{} has no description", record.path, record.key))
}

/// `direct`, `derived`, `carried`, `shared`, or `renamed`, as the alias format defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasBasis {
    Direct,
    Derived,
    Carried,
    Shared,
    Renamed,
}

/// One resolved alias entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AliasEntry {
    pub old: String,
    pub target: String,
    pub basis: AliasBasis,
}

/// One retirement entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RetiredEntry {
    pub old: String,
    pub reason: RetirementReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The binding to the frozen snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyBinding {
    pub count: usize,
    pub sha256: String,
}

/// `generated/content/<version>/aliases.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AliasFile {
    pub format: u32,
    pub legacy: LegacyBinding,
    pub entries: Vec<AliasEntry>,
    pub retired: Vec<RetiredEntry>,
}

/// Counts for the command report and the release notes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AliasStats {
    pub legacy: usize,
    pub mapped: usize,
    pub shared: usize,
    pub retired: usize,
    pub direct: usize,
    pub derived: usize,
    pub carried: usize,
    pub renamed: usize,
    pub retired_by_reason: BTreeMap<String, usize>,
}

impl AliasStats {
    pub fn report(&self) -> String {
        let reasons: Vec<String> = self
            .retired_by_reason
            .iter()
            .map(|(reason, count)| format!("{reason} {count}"))
            .collect();
        format!(
            "legacy {}: mapped {} (direct {}, derived {}, carried {}, renamed {}), shared {}, retired {} ({})",
            self.legacy,
            self.mapped,
            self.direct,
            self.derived,
            self.carried,
            self.renamed,
            self.shared,
            self.retired,
            reasons.join(", ")
        )
    }
}

/// Resolves every row and runs the totality, injectivity, target, and direction gates.
///
/// All problems are collected so one run shows the whole map's state. `snapshot_bytes`
/// must be the exact bytes of `legacy-ids.json`, because its hash is the oracle binding.
pub fn generate(
    map: &IdMap,
    snapshot: &LegacySnapshot,
    snapshot_bytes: &[u8],
    catalog: &CanonicalCatalog,
) -> Result<(AliasFile, AliasStats), Vec<String>> {
    let mut errors = Vec::new();
    let digest = sha256_hex(snapshot_bytes);
    if digest != map.meta.legacy_sha256 {
        errors.push(format!(
            "meta.legacy_sha256 is {} but legacy-ids.json hashes to {digest}",
            map.meta.legacy_sha256
        ));
    }
    if map.meta.legacy_count != snapshot.count {
        errors.push(format!(
            "meta.legacy_count is {} but the snapshot holds {}",
            map.meta.legacy_count, snapshot.count
        ));
    }
    let known: BTreeSet<&str> = snapshot
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    let legacy_by_id = snapshot.by_id();

    // Totality and disposition conflicts.
    for old in map
        .map
        .keys()
        .chain(map.shared.keys())
        .chain(map.retired.keys())
    {
        if !known.contains(old.as_str()) {
            errors.push(format!(
                "legacy id {old:?} is not in the frozen snapshot; the oracle cannot be extended by a map row"
            ));
        }
    }
    for entry in &snapshot.entries {
        let dispositions = usize::from(map.map.contains_key(&entry.id))
            + usize::from(map.shared.contains_key(&entry.id))
            + usize::from(map.retired.contains_key(&entry.id));
        match dispositions {
            0 => errors.push(format!(
                "legacy id {:?} is neither mapped nor retired; add a [map] or [retired] row",
                entry.id
            )),
            1 => {}
            _ => errors.push(format!(
                "legacy id {:?} appears in more than one disposition",
                entry.id
            )),
        }
    }

    // Resolution.
    let mut resolved: BTreeMap<String, (String, AliasBasis)> = BTreeMap::new();
    for (old, row) in &map.map {
        let (target, basis) = if let Some(identity) = &row.record {
            match catalog.canonical_for_identity(identity) {
                Some(id) => (id.to_string(), AliasBasis::Direct),
                None => {
                    errors.push(format!(
                        "map row {old:?}: record {identity:?} does not exist in the pinned corpus"
                    ));
                    continue;
                }
            }
        } else if let Some(identity) = &row.derived {
            match catalog.canonical_for_identity(identity) {
                Some(id) => (id.to_string(), AliasBasis::Derived),
                None => {
                    errors.push(format!(
                        "map row {old:?}: derived {identity:?} was not produced by the canonical derivation"
                    ));
                    continue;
                }
            }
        } else if let Some(target) = &row.target {
            if !catalog.contains(target) {
                errors.push(format!(
                    "map row {old:?}: target {target:?} does not exist in the pinned corpus"
                ));
                continue;
            }
            let basis = if target.starts_with("ref/carried/") {
                AliasBasis::Carried
            } else {
                AliasBasis::Renamed
            };
            (target.clone(), basis)
        } else {
            unreachable!("validated: exactly one disposition");
        };
        // The semantic fit gate: the legacy record and the canonical record must
        // describe the same book and the same title. Resolution alone cannot tell a
        // real match from a slug coincidence.
        let Some(record) = catalog.get(&target) else {
            continue;
        };
        if let Some(legacy) = legacy_by_id.get(old.as_str()) {
            check_semantic_fit(old, legacy, record, &mut errors);
        }
        resolved.insert(old.clone(), (target, basis));
    }
    for (old, row) in &map.shared {
        if !catalog.contains(&row.target) {
            errors.push(format!(
                "shared row {old:?}: target {:?} does not exist in the pinned corpus",
                row.target
            ));
            continue;
        }
        resolved.insert(old.clone(), (row.target.clone(), AliasBasis::Shared));
    }

    // Direction: no chains, and the namespaces never intersect.
    let targets: BTreeSet<&str> = resolved
        .values()
        .map(|(target, _)| target.as_str())
        .collect();
    for target in &targets {
        if known.contains(*target) {
            errors.push(format!(
                "alias target {target:?} is itself a legacy id; chains are forbidden"
            ));
        }
        if map.map.contains_key(*target) || map.shared.contains_key(*target) {
            errors.push(format!(
                "alias target {target:?} is also mapped as a legacy id; chains are forbidden"
            ));
        }
    }
    for entry in &snapshot.entries {
        if catalog.contains(&entry.id) {
            errors.push(format!(
                "legacy id {:?} is also a canonical id; the namespaces must be disjoint",
                entry.id
            ));
        }
    }

    // Injectivity: one canonical record cannot absorb two legacy ids unless every
    // row says so in `[shared]`.
    let mut claims: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (old, (target, _)) in &resolved {
        claims
            .entry(target.as_str())
            .or_default()
            .push(old.as_str());
    }
    for (target, olds) in &claims {
        if olds.len() < 2 {
            continue;
        }
        let mut sorted = olds.clone();
        sorted.sort_unstable();
        for old in &sorted {
            if !map.shared.contains_key(*old) {
                errors.push(format!(
                    "canonical record {target:?} is claimed by {} legacy ids ({}) but {old:?} is not a [shared] row",
                    sorted.len(),
                    sorted.join(", ")
                ));
            }
        }
    }
    for old in map.shared.keys() {
        if let Some((target, _)) = resolved.get(old)
            && claims
                .get(target.as_str())
                .is_some_and(|olds| olds.len() < 2)
        {
            errors.push(format!(
                "shared row {old:?} names {target:?}, but no other legacy id maps there"
            ));
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut entries: Vec<AliasEntry> = resolved
        .into_iter()
        .map(|(old, (target, basis))| AliasEntry { old, target, basis })
        .collect();
    entries.sort_by(|a, b| a.old.cmp(&b.old));
    let mut retired: Vec<RetiredEntry> = map
        .retired
        .iter()
        .map(|(old, row)| RetiredEntry {
            old: old.clone(),
            reason: row.reason,
            note: row.note.clone(),
        })
        .collect();
    retired.sort_by(|a, b| a.old.cmp(&b.old));

    let mut stats = AliasStats {
        legacy: snapshot.count,
        mapped: entries.len(),
        shared: map.shared.len(),
        retired: retired.len(),
        ..Default::default()
    };
    for entry in &entries {
        match entry.basis {
            AliasBasis::Direct => stats.direct += 1,
            AliasBasis::Derived => stats.derived += 1,
            AliasBasis::Carried => stats.carried += 1,
            AliasBasis::Renamed => stats.renamed += 1,
            AliasBasis::Shared => {}
        }
    }
    for row in &retired {
        *stats
            .retired_by_reason
            .entry(row.reason.slug().to_string())
            .or_default() += 1;
    }
    Ok((
        AliasFile {
            format: ALIAS_FORMAT,
            legacy: LegacyBinding {
                count: snapshot.count,
                sha256: map.meta.legacy_sha256.clone(),
            },
            entries,
            retired,
        },
        stats,
    ))
}

/// Serializes with a fixed field order and a trailing newline. Same inputs, same bytes.
pub fn to_bytes(file: &AliasFile) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(file).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SNAPSHOT: &str = r#"{
  "format": 1,
  "count": 3,
  "entries": [
    {"id": "bestiary/monsters/example", "kind": "creature", "title": "Example", "source": "bestiary"},
    {"id": "core-rulebook/rules/magic", "kind": "rule", "title": "Magic", "source": "core-rulebook"},
    {"id": "core-rulebook/spells/fireball", "kind": "spell", "title": "Fireball", "source": "core-rulebook"}
  ]
}"#;

    fn snapshot() -> (LegacySnapshot, Vec<u8>) {
        let bytes = SNAPSHOT.as_bytes().to_vec();
        (LegacySnapshot::parse(&bytes).unwrap(), bytes)
    }

    fn map_text(sha: &str, count: usize, body: &str) -> String {
        format!("[meta]\nformat = 1\nlegacy_count = {count}\nlegacy_sha256 = \"{sha}\"\n\n{body}")
    }

    fn catalog() -> CanonicalCatalog {
        let mut catalog = CanonicalCatalog::default();
        for (id, identity, kind, title) in [
            ("ref/main/magic", "json/main.json#magic", "rule", "Magic"),
            (
                "ref/spell/fireball",
                "json/spells.json#fireball",
                "spell",
                "Fireball",
            ),
            (
                "ref/racial-trait/dwarf/stonecunning",
                "json/races.json#dwarf|racial-trait:stonecunning",
                "racial_trait",
                "Stonecunning",
            ),
        ] {
            catalog
                .insert(CanonicalRecord {
                    id: id.into(),
                    identity: identity.into(),
                    kind: kind.into(),
                    title: title.into(),
                    source_ids: vec!["core-rulebook".into()],
                    origin: "native".into(),
                })
                .unwrap();
        }
        catalog
    }

    #[test]
    fn parses_every_section() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let text = format!(
            r#"{}

[map."core-rulebook/spells/fireball"]
derived = "json/spells.json#fireball"
note = "native identity"

[shared."core-rulebook/rules/magic"]
target = "ref/main/magic"
note = "same article"

[retired."bestiary/monsters/example"]
reason = "no_source"
note = "reviewed"
"#,
            map_text(&sha, 3, "")
        );
        let map = IdMap::parse(&text).unwrap();
        assert_eq!(map.map.len(), 1);
        assert_eq!(map.shared.len(), 1);
        assert_eq!(
            map.retired["bestiary/monsters/example"].reason,
            RetirementReason::NoSource
        );
        assert_eq!(snapshot.entries.len(), 3);
    }

    #[test]
    fn rejects_unknown_fields_and_bad_dispositions() {
        let (_, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let unknown = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\ntypo = 1\n",
        );
        assert!(
            IdMap::parse(&unknown)
                .unwrap_err()
                .contains("unknown field")
        );

        let both = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\nrecord = \"json/main.json#magic\"\n",
        );
        assert!(IdMap::parse(&both).unwrap_err().contains("exactly one"));

        let none = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\nnote = \"x\"\n",
        );
        assert!(IdMap::parse(&none).unwrap_err().contains("exactly one"));

        let reason = map_text(
            &sha,
            3,
            "[retired.\"bestiary/monsters/example\"]\nreason = \"because\"\n",
        );
        assert!(IdMap::parse(&reason).is_err());
    }

    #[test]
    fn rejects_canonical_ids_and_duplicate_dispositions() {
        let (_, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let canonical = map_text(
            &sha,
            3,
            "[map.\"ref/spell/fireball\"]\ntarget = \"ref/main/magic\"\n",
        );
        assert!(
            IdMap::parse(&canonical)
                .unwrap_err()
                .contains("canonical namespace")
        );

        let twice = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n\n[retired.\"core-rulebook/rules/magic\"]\nreason = \"no_source\"\n",
        );
        assert!(
            IdMap::parse(&twice)
                .unwrap_err()
                .contains("more than one section")
        );
    }

    #[test]
    fn snapshot_validation_is_strict() {
        let mut value: serde_json::Value = serde_json::from_str(SNAPSHOT).unwrap();
        value["count"] = json!(4);
        assert!(
            LegacySnapshot::parse(&serde_json::to_vec(&value).unwrap())
                .unwrap_err()
                .contains("count")
        );

        let unsorted = r#"{"format":1,"count":2,"entries":[{"id":"b/x","kind":"rule","title":"B","source":"s"},{"id":"a/x","kind":"rule","title":"A","source":"s"}]}"#;
        assert!(
            LegacySnapshot::parse(unsorted.as_bytes())
                .unwrap_err()
                .contains("not sorted")
        );

        let canonical = r#"{"format":1,"count":1,"entries":[{"id":"ref/x/y","kind":"rule","title":"R","source":"s"}]}"#;
        assert!(
            LegacySnapshot::parse(canonical.as_bytes())
                .unwrap_err()
                .contains("canonical id")
        );
    }

    #[test]
    fn totality_fails_when_a_legacy_id_has_no_row() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let text = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n",
        );
        let map = IdMap::parse(&text).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(
            errors
                .iter()
                .all(|e| e.contains("neither mapped nor retired"))
        );
    }

    #[test]
    fn a_map_row_outside_the_snapshot_fails() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let text = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/wish\"]\nreason = \"no_source\"\n",
        );
        let map = IdMap::parse(&text).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("not in the frozen snapshot"))
        );
    }

    #[test]
    fn target_must_exist_in_the_catalog() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let text = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/absent\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n",
        );
        let map = IdMap::parse(&text).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("does not exist in the pinned corpus"))
        );
    }

    #[test]
    fn record_and_derived_rows_resolve_through_the_catalog() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let text = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\nrecord = \"json/main.json#magic\"\n\n[map.\"core-rulebook/spells/fireball\"]\nderived = \"json/spells.json#fireball\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n",
        );
        let map = IdMap::parse(&text).unwrap();
        let (file, stats) = generate(&map, &snapshot, &bytes, &catalog()).unwrap();
        assert_eq!(stats.direct, 1);
        assert_eq!(stats.derived, 1);
        assert_eq!(stats.retired, 1);
        let targets: Vec<_> = file.entries.iter().map(|e| e.target.as_str()).collect();
        assert_eq!(targets, ["ref/main/magic", "ref/spell/fireball"]);
    }

    #[test]
    fn missing_identity_fails() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let text = map_text(
            &sha,
            3,
            "[map.\"core-rulebook/rules/magic\"]\nrecord = \"json/main.json#absent\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n",
        );
        let map = IdMap::parse(&text).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("does not exist in the pinned corpus"))
        );
    }

    #[test]
    fn two_legacy_ids_need_an_explicit_shared_row() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let body = "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n\n[map.\"core-rulebook/spells/fireball\"]\ntarget = \"ref/main/magic\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("is not a [shared] row")));

        let shared_body = "[shared.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\nnote = \"same article\"\n\n[shared.\"core-rulebook/spells/fireball\"]\ntarget = \"ref/main/magic\"\nnote = \"same article\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, shared_body)).unwrap();
        let (file, stats) = generate(&map, &snapshot, &bytes, &catalog()).unwrap();
        assert_eq!(stats.shared, 2);
        assert_eq!(
            file.entries
                .iter()
                .find(|e| e.old == "core-rulebook/spells/fireball")
                .unwrap()
                .basis,
            AliasBasis::Shared
        );
    }

    #[test]
    fn a_lonely_shared_row_fails() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let body = "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n\n[shared.\"core-rulebook/spells/fireball\"]\ntarget = \"ref/spell/fireball\"\nnote = \"not actually shared\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("no other legacy id maps there"))
        );
    }

    #[test]
    fn a_target_cannot_be_an_old_id() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let mut catalog = catalog();
        // A catalog that (incorrectly) republishes a legacy id as canonical: the
        // direction gate must refuse to alias a legacy id onto another legacy id.
        catalog
            .insert(CanonicalRecord {
                id: "core-rulebook/spells/fireball".into(),
                identity: "broken:legacy".into(),
                kind: "spell".into(),
                title: "Fireball".into(),
                source_ids: vec!["core-rulebook".into()],
                origin: "native".into(),
            })
            .unwrap();
        let body = "[map.\"core-rulebook/rules/magic\"]\ntarget = \"core-rulebook/spells/fireball\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("chains are forbidden")));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("namespaces must be disjoint"))
        );
    }

    #[test]
    fn golden_hash_mismatch_fails() {
        let (snapshot, bytes) = snapshot();
        let sha = "0".repeat(64);
        let body = "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("legacy_sha256")));

        let sha = sha256_hex(&bytes);
        let map = IdMap::parse(&map_text(&sha, 4, body)).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog()).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("legacy_count")));
    }

    #[test]
    fn output_is_sorted_deterministic_and_newline_terminated() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let body = "[map.\"core-rulebook/spells/fireball\"]\ntarget = \"ref/spell/fireball\"\n\n[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let (file, _) = generate(&map, &snapshot, &bytes, &catalog()).unwrap();
        let first = to_bytes(&file).unwrap();
        let second = to_bytes(&file).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.last(), Some(&b'\n'));
        let json: serde_json::Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(json["format"], 1);
        assert_eq!(json["legacy"]["count"], 3);
        assert_eq!(json["entries"][0]["old"], "core-rulebook/rules/magic");
        assert_eq!(json["entries"][1]["old"], "core-rulebook/spells/fireball");
        assert_eq!(json["retired"][0]["reason"], "no_source");
    }

    #[test]
    fn carried_targets_get_the_carried_basis() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let mut catalog = catalog();
        catalog
            .insert(CanonicalRecord {
                id: "ref/carried/bestiary/monsters/example".into(),
                identity: "carried:bestiary/monsters/example".into(),
                kind: "creature".into(),
                title: "Example".into(),
                source_ids: vec!["bestiary".into()],
                origin: "carried".into(),
            })
            .unwrap();
        let body = "[map.\"bestiary/monsters/example\"]\ntarget = \"ref/carried/bestiary/monsters/example\"\n\n[retired.\"core-rulebook/rules/magic\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let (file, stats) = generate(&map, &snapshot, &bytes, &catalog).unwrap();
        assert_eq!(stats.carried, 1);
        assert_eq!(file.entries[0].basis, AliasBasis::Carried);
    }

    #[test]
    fn native_and_derived_catalog_ids_stay_injective() {
        let mut catalog = CanonicalCatalog::default();
        let record = CanonicalRecord {
            id: "ref/race/dwarf".into(),
            identity: "json/races.json#dwarf".into(),
            kind: "race".into(),
            title: "Dwarf".into(),
            source_ids: vec!["core-rulebook".into()],
            origin: "native".into(),
        };
        catalog.insert(record.clone()).unwrap();
        let error = catalog.insert(record.clone()).unwrap_err();
        assert!(error.contains("claimed by both"), "{error}");

        let mut other = record;
        other.id = "ref/race/elf".into();
        assert!(catalog.insert(other).is_err());
        assert_eq!(catalog.len(), 1);
    }

    #[test]
    fn canonical_native_ids_use_the_ref_namespace() {
        assert_eq!(
            canonical_native_id("race", "Dwarf").unwrap(),
            "ref/race/dwarf"
        );
        assert_eq!(
            canonical_native_id("rule", "sanity_and_madness").unwrap(),
            "ref/rule/sanity-and-madness"
        );
        assert!(canonical_native_id("", "x").is_err());
        assert!(canonical_native_id("rule", "...").is_err());
    }

    #[test]
    fn rejects_unknown_format_versions() {
        let (_, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let text = format!(
            "[meta]\nformat = 2\nlegacy_count = 3\nlegacy_sha256 = \"{sha}\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n"
        );
        assert!(IdMap::parse(&text).unwrap_err().contains("meta.format"));

        let mut value: serde_json::Value = serde_json::from_str(SNAPSHOT).unwrap();
        value["format"] = json!(2);
        assert!(
            LegacySnapshot::parse(&serde_json::to_vec(&value).unwrap())
                .unwrap_err()
                .contains("format")
        );
    }

    fn inventory(records: Vec<InputRecord>) -> Inventory {
        Inventory {
            dataset: "pathfinder-data-1-e".into(),
            commit: "0".repeat(40),
            input_digest: "0".repeat(64),
            json_files: 1,
            input_records: records.len(),
            registered_publications: 1,
            by_file: BTreeMap::new(),
            by_shape: BTreeMap::new(),
            by_publication: BTreeMap::new(),
            directives: BTreeMap::new(),
            unregistered_publications: Vec::new(),
            out_of_scope: Vec::new(),
            pi_omissions: Vec::new(),
            records,
        }
    }

    fn rule_allowlist() -> RuleAllowlist {
        RuleAllowlist::from_json(
            br#"{
                "format": "cast-iron-derived-rules",
                "version": 1,
                "rules": [{
                    "rule": "core-rulebook/rules/host/test-rule",
                    "title": "Test Rule",
                    "host": "json/rules_test.json#host",
                    "protocol": "rule",
                    "shape": "section",
                    "heading": "Test Rule",
                    "id": "ref/rule/rule-host/test-rule"
                }]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn catalog_from_inventory_covers_native_derived_and_allowlisted_records() {
        let race = InputRecord {
            protocol: Some("race".into()),
            path: "json/races.json".into(),
            key: "dwarf".into(),
            name: "Dwarf".into(),
            shape: "description".into(),
            publications: vec!["PRPG Core Rulebook".into()],
            raw: json!({"description": ["::rtraits{plural=Dwarves main=\"~Stonecunning|Dwarves notice stonework.\"}"]}),
        };
        let host = InputRecord {
            protocol: Some("rule".into()),
            path: "json/rules_test.json".into(),
            key: "host".into(),
            name: "Host".into(),
            shape: "description".into(),
            publications: vec!["PRPG Core Rulebook".into()],
            raw: json!({"description": ["## Test Rule", "Rules-bearing text."]}),
        };
        let catalog =
            catalog_from_inventory(&inventory(vec![race, host]), &rule_allowlist()).unwrap();
        assert!(catalog.contains("ref/race/dwarf"));
        assert!(catalog.contains("ref/racial-trait/dwarf/stonecunning"));
        assert!(catalog.contains("ref/rule/rule-host/test-rule"));
        assert_eq!(
            catalog.canonical_for_identity("json/races.json#dwarf"),
            Some("ref/race/dwarf")
        );
        assert_eq!(
            catalog.canonical_for_identity("json/races.json#dwarf|racial-trait:stonecunning"),
            Some("ref/racial-trait/dwarf/stonecunning")
        );
        assert_eq!(
            catalog.canonical_for_identity("json/rules_test.json#host|section:test-rule"),
            Some("ref/rule/rule-host/test-rule")
        );
        // The fit gate compares legacy book ids, so the canonical publication name
        // is folded to the same vocabulary as `legacy-ids.json`.
        assert_eq!(
            catalog.get("ref/race/dwarf").unwrap().source_ids,
            ["core-rulebook"]
        );
        assert_eq!(
            catalog
                .get("ref/racial-trait/dwarf/stonecunning")
                .unwrap()
                .source_ids,
            ["core-rulebook"]
        );
    }

    #[test]
    fn publication_variants_fold_to_legacy_book_ids() {
        assert_eq!(legacy_book_id("PRPG Core Rulebook"), "core-rulebook");
        assert_eq!(legacy_book_id("Pathfinder RPG Bestiary"), "bestiary");
        assert_eq!(legacy_book_id("Pathfinder RPG Bestiary 2"), "bestiary-2");
        assert_eq!(legacy_book_id("GameMastery Guide"), "game-mastery-guide");
        assert_eq!(legacy_book_id("Bestiary 2"), "bestiary-2");
        assert_eq!(legacy_book_id("Ultimate Campaign"), "ultimate-campaign");
        // An unreviewed spelling is never forced onto a legacy book.
        assert_eq!(legacy_book_id("Inner Sea Races"), "inner-sea-races");
    }

    #[test]
    fn titles_compare_on_words_not_punctuation_or_case() {
        assert_eq!(normalize_title("Fungal rot"), normalize_title("Fungal Rot"));
        assert_eq!(normalize_title("Sure-Footed"), "sure footed");
        assert_eq!(normalize_title("  Low-Light   Vision "), "low light vision");
        assert_ne!(
            normalize_title("Weather"),
            normalize_title("Weather Events")
        );
    }

    #[test]
    fn title_fit_accepts_only_the_documented_spellings() {
        assert_eq!(
            title_fit("Dragon, Red", "Red Dragon", "creature"),
            Some(TitleFit::Inversion)
        );
        assert_eq!(
            title_fit(
                "Dragon, Imperial, Sky, Young",
                "Young Sky Dragon",
                "creature"
            ),
            Some(TitleFit::Inversion)
        );
        assert_eq!(
            title_fit("Olethrodaemon", "Olethrodaemon (Daemon)", "creature"),
            Some(TitleFit::Parenthetical)
        );
        assert_eq!(
            title_fit("Devil, Accuser", "Accuser Devil (Zebub)", "creature"),
            Some(TitleFit::InversionParenthetical)
        );
        assert_eq!(
            title_fit("Chain", "Chain (10 ft.)", "item"),
            Some(TitleFit::Parenthetical)
        );
        assert_eq!(title_fit("Lock", "Lock (average)", "item"), None);
        assert_eq!(
            title_fit(
                "Smart Armor (Expanded Form)",
                "Smart Armor (compact form)",
                "item"
            ),
            None
        );
        assert_eq!(
            title_fit("Malediction", "Malediction (Hero Points)", "spell"),
            None
        );
        assert_eq!(title_fit("Weather", "Weather Events", "rule"), None);
    }

    #[test]
    fn semantic_gate_rejects_a_slug_coincidence() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let mut catalog = catalog();
        // Same title, wrong book: the shape of the reported P1 rows such as
        // `environment/weather` -> the Ceremony spell's weather subsection.
        catalog
            .insert(CanonicalRecord {
                id: "ref/spell/ceremony-weather".into(),
                identity: "json/spells.json#ceremony|section:weather".into(),
                kind: "rule".into(),
                title: "Magic".into(),
                source_ids: vec!["quests-and-campaigns".into()],
                origin: "derived:rule".into(),
            })
            .unwrap();
        let body = "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/spell/ceremony-weather\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog).unwrap_err();
        let message = errors
            .iter()
            .find(|e| e.contains("legacy source"))
            .expect("the source mismatch must be reported");
        assert!(message.contains("core-rulebook"), "{message}");
        assert!(message.contains("quests-and-campaigns"), "{message}");
        assert!(message.contains("not a verified match"), "{message}");
    }

    #[test]
    fn semantic_gate_rejects_a_title_mismatch() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let mut catalog = catalog();
        // Same book, different article.
        catalog
            .insert(CanonicalRecord {
                id: "ref/main/arcane-magic".into(),
                identity: "json/main.json#arcane".into(),
                kind: "rule".into(),
                title: "Arcane Magic".into(),
                source_ids: vec!["core-rulebook".into()],
                origin: "native".into(),
            })
            .unwrap();
        let body = "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/arcane-magic\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let errors = generate(&map, &snapshot, &bytes, &catalog).unwrap_err();
        let message = errors
            .iter()
            .find(|e| e.contains("legacy title"))
            .expect("the title mismatch must be reported");
        assert!(message.contains("Arcane Magic"), "{message}");
    }

    #[test]
    fn semantic_gate_accepts_a_matching_title_and_source() {
        let (snapshot, bytes) = snapshot();
        let sha = sha256_hex(&bytes);
        let body = "[map.\"core-rulebook/rules/magic\"]\ntarget = \"ref/main/magic\"\n\n[retired.\"bestiary/monsters/example\"]\nreason = \"no_source\"\n\n[retired.\"core-rulebook/spells/fireball\"]\nreason = \"no_source\"\n";
        let map = IdMap::parse(&map_text(&sha, 3, body)).unwrap();
        let (_, stats) = generate(&map, &snapshot, &bytes, &catalog()).unwrap();
        assert_eq!(stats.mapped, 1);
        assert_eq!(stats.retired, 2);
    }

    #[test]
    fn committed_map_passes_every_gate_when_the_dataset_is_present() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dataset = root.join(".cache/sources/pathfinder-data-1-e");
        if !dataset.is_dir() {
            return;
        }
        let bytes = std::fs::read(root.join("sources/canonical/legacy-ids.json")).unwrap();
        let snapshot = LegacySnapshot::parse(&bytes).unwrap();
        let map = IdMap::load(&root.join("sources/canonical/id-map.toml")).unwrap();
        let inv = crate::pf_data::inventory(
            &dataset,
            &root.join("sources/pathfinder-data-1-e/source.json"),
        )
        .unwrap();
        let allowlist = RuleAllowlist::load_optional(
            &root.join("sources/pathfinder-data-1-e/derived-rules.json"),
        )
        .unwrap();
        let catalog = catalog_from_inventory(&inv, &allowlist).unwrap();
        let (_, stats) = generate(&map, &snapshot, &bytes, &catalog).unwrap();
        assert_eq!(
            stats.mapped + stats.retired,
            snapshot.count,
            "every legacy id has one disposition"
        );
        assert!(stats.mapped > 0);
    }

    #[test]
    fn committed_artifacts_agree_with_each_other() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let snapshot_path = root.join("sources/canonical/legacy-ids.json");
        let map_path = root.join("sources/canonical/id-map.toml");
        let bytes = std::fs::read(&snapshot_path).unwrap();
        let snapshot = LegacySnapshot::parse(&bytes).unwrap();
        let map = IdMap::load(&map_path).unwrap();
        assert_eq!(map.meta.legacy_sha256, sha256_hex(&bytes));
        assert_eq!(map.meta.legacy_count, snapshot.count);
        assert_eq!(
            snapshot.count,
            map.map.len() + map.shared.len() + map.retired.len(),
            "every legacy id needs exactly one disposition"
        );
        let ids: BTreeSet<&str> = snapshot
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        for old in map
            .map
            .keys()
            .chain(map.shared.keys())
            .chain(map.retired.keys())
        {
            assert!(ids.contains(old.as_str()), "{old} is not in the snapshot");
        }
    }
}
