//! Canonical Cast Iron content contract.
//!
//! The content compiler writes these types and the client reads them. A change
//! that breaks the serialized form must increase [`SCHEMA_VERSION`].
//!
//! Published layout, relative to the site's `content/` directory:
//!
//! ```text
//! manifest.json                    ContentManifest: the only unversioned file
//! <content_version>/catalog.json   Catalog
//! <content_version>/legal.json     LegalNotices
//! <content_version>/coverage.json  CoverageReport
//! <content_version>/search.bin     cast_iron_search::SearchIndex bytes
//! <content_version>/entries/<kind>-<NNN>.json   Vec<RuleEntry>
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub mod validate;

/// Version of the serialized content format.
pub const SCHEMA_VERSION: u32 = 1;

/// Stable canonical identifier. Lowercase `a-z0-9-` segments joined by `/`,
/// such as `core-rulebook/spells/fireball`.
///
/// Identity comes from the upstream record path, not from build order. Bookmarks
/// store this value, so a published id must never change meaning.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntryId(String);

/// The reason a string cannot be an [`EntryId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidEntryId(pub String);

impl fmt::Display for InvalidEntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid entry id: {:?}", self.0)
    }
}

impl std::error::Error for InvalidEntryId {}

impl EntryId {
    /// Parses a string, and rejects anything other than slug segments joined by `/`.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidEntryId> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.split('/').all(|segment| {
                !segment.is_empty()
                    && !segment.starts_with('-')
                    && !segment.ends_with('-')
                    && segment
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidEntryId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EntryId {
    type Error = InvalidEntryId;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<EntryId> for String {
    fn from(value: EntryId) -> Self {
        value.0
    }
}

impl fmt::Display for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Rules category. Add a variant for each new category instead of using a catch-all,
/// so that matches over this type stay exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Rule,
    Spell,
    Feat,
    Creature,
    Item,
    Class,
    Race,
    Skill,
    Affliction,
    Trap,
}

impl EntryKind {
    pub const ALL: [EntryKind; 10] = [
        EntryKind::Rule,
        EntryKind::Spell,
        EntryKind::Feat,
        EntryKind::Creature,
        EntryKind::Item,
        EntryKind::Class,
        EntryKind::Race,
        EntryKind::Skill,
        EntryKind::Affliction,
        EntryKind::Trap,
    ];

    /// Stable slug, used in shard names, routes, and search facets.
    pub fn slug(self) -> &'static str {
        match self {
            EntryKind::Rule => "rule",
            EntryKind::Spell => "spell",
            EntryKind::Feat => "feat",
            EntryKind::Creature => "creature",
            EntryKind::Item => "item",
            EntryKind::Class => "class",
            EntryKind::Race => "race",
            EntryKind::Skill => "skill",
            EntryKind::Affliction => "affliction",
            EntryKind::Trap => "trap",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.slug() == slug)
    }

    /// Plural display label.
    pub fn label(self) -> &'static str {
        match self {
            EntryKind::Rule => "Rules",
            EntryKind::Spell => "Spells",
            EntryKind::Feat => "Feats",
            EntryKind::Creature => "Creatures",
            EntryKind::Item => "Items",
            EntryKind::Class => "Classes",
            EntryKind::Race => "Races",
            EntryKind::Skill => "Skills",
            EntryKind::Affliction => "Afflictions",
            EntryKind::Trap => "Traps",
        }
    }
}

/// One canonical rules record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEntry {
    pub id: EntryId,
    pub kind: EntryKind,
    pub title: String,
    /// Publication ids. Each id must exist in [`Catalog::sources`].
    pub source_ids: Vec<String>,
    /// Breadcrumb path in the rules hierarchy, such as `["Combat", "Actions in Combat"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub section: Vec<String>,
    /// The containing rules entry, for hierarchy navigation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<EntryId>,
    pub body: Vec<Block>,
    pub fields: TypedFields,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Every resolved internal reference that the body holds, deduplicated and sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<EntryReference>,
    pub provenance: Vec<Provenance>,
    /// License ids. Each id must exist in [`LegalNotices::licenses`].
    pub license_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntryReference {
    pub target: EntryId,
    /// Heading anchor within the target entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Dataset id, such as `psrd-data`.
    pub dataset: String,
    /// Upstream record identifier, such as `pfsrd://Core Rulebook/Spells/Fireball`.
    pub record: String,
    /// Path of the upstream file, relative to the dataset root.
    pub path: String,
}

/// Block-level rich content. Nested upstream sections flatten into headings with anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Block {
    Heading {
        /// 2 to 6. The entry title is the only level 1 heading.
        level: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<String>,
        content: Vec<Inline>,
    },
    Paragraph {
        #[serde(default, skip_serializing_if = "is_default_style")]
        style: ParagraphStyle,
        content: Vec<Inline>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    Table(Table),
    Quote {
        content: Vec<Block>,
    },
    /// Labelled mechanical rows, such as a spell header or a creature stat block.
    StatBlock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        rows: Vec<StatRow>,
    },
    /// A rules-tree child that is its own entry. The renderer shows it as a link.
    EntryLink {
        target: EntryId,
        title: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphStyle {
    #[default]
    Normal,
    /// Upstream classes `stat-block*`: compact labelled lines.
    StatLine,
    /// Upstream class `indent`, and second-level stat lines.
    Indented,
}

fn is_default_style(style: &ParagraphStyle) -> bool {
    *style == ParagraphStyle::Normal
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatRow {
    pub label: String,
    pub value: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Table {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub caption: Vec<Inline>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub head: Vec<TableRow>,
    pub body: Vec<TableRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub foot: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableCell {
    #[serde(default, skip_serializing_if = "is_false")]
    pub header: bool,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub colspan: u16,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub rowspan: u16,
    pub content: Vec<Inline>,
}

fn is_false(value: &bool) -> bool {
    !*value
}
fn one() -> u16 {
    1
}
fn is_one(value: &u16) -> bool {
    *value == 1
}

/// Inline rich content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Inline {
    Text {
        v: String,
    },
    Em {
        c: Vec<Inline>,
    },
    Strong {
        c: Vec<Inline>,
    },
    Sup {
        c: Vec<Inline>,
    },
    Sub {
        c: Vec<Inline>,
    },
    Break,
    /// A resolved internal link. Unresolved upstream links become plain content,
    /// and the compiler reports them.
    Link {
        target: EntryId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<String>,
        c: Vec<Inline>,
    },
}

impl Inline {
    pub fn text(value: impl Into<String>) -> Self {
        Inline::Text { v: value.into() }
    }
}

/// Category-specific metadata. Text fields hold upstream wording after entity decoding,
/// because PF1e values such as `Will negates (harmless)` are not enumerable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypedFields {
    Spell(SpellFields),
    Feat(FeatFields),
    Creature(Box<CreatureFields>),
    Item(ItemFields),
    Class(ClassFields),
    /// Rule, race, skill, affliction, and trap entries for now.
    None,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SpellFields {
    pub school: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subschools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub descriptors: Vec<String>,
    pub levels: Vec<SpellLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub casting_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    /// Upstream `effects`, such as `("Target", "one creature")`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saving_throw: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_resistance: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellLevel {
    pub class: String,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FeatFields {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feat_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benefit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special: Option<String>,
}

/// Challenge rating, stored in 24ths so that every PF1e fraction sorts exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChallengeRating {
    pub twenty_fourths: u32,
}

impl ChallengeRating {
    /// Parses `1/8`, `1/6`, `1/4`, `1/3`, `1/2`, or a whole number.
    pub fn parse(value: &str) -> Option<Self> {
        let twenty_fourths = match value.trim() {
            "1/8" => 3,
            "1/6" => 4,
            "1/4" => 6,
            "1/3" => 8,
            "1/2" => 12,
            whole => whole.parse::<u32>().ok()?.checked_mul(24)?,
        };
        Some(Self { twenty_fourths })
    }

    pub fn display(self) -> String {
        match self.twenty_fourths {
            3 => "1/8".into(),
            4 => "1/6".into(),
            6 => "1/4".into(),
            8 => "1/3".into(),
            12 => "1/2".into(),
            value => (value / 24).to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreatureFields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr: Option<ChallengeRating>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creature_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtypes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fortitude: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub will: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub melee: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranged: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_attacks: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_qualities: Option<String>,
    /// The ability scores, in STR, DEX, CON, INT, WIS, CHA order. `-` where none applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abilities: Option<[String; 6]>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ItemFields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aura: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caster_level: Option<String>,
    /// Remaining upstream attributes, in upstream order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClassFields {
    /// Upstream subtype: `core`, `base`, `prestige`, or `npc`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_die: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_ranks: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_skills: Option<String>,
}

/// `content/manifest.json`. The compiler fills it deterministically: no timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentManifest {
    pub schema_version: u32,
    /// `v` plus the first 16 hex digits of the SHA-256 over the sorted `path:sha256` lines.
    /// The same outputs always give the same version.
    pub content_version: String,
    pub total_bytes: u64,
    /// Sorted by path. Paths are relative to `content/` and start with `content_version`.
    pub files: Vec<ManifestFile>,
    pub entry_count: u32,
    pub datasets: Vec<DatasetRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    /// Lowercase hexadecimal.
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetRef {
    pub id: String,
    pub repository: String,
    pub commit: String,
}

/// `catalog.json`: everything that browsing needs without loading entry shards.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub sources: Vec<SourceInfo>,
    /// Sorted by id.
    pub entries: Vec<EntryStub>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Slug, such as `core-rulebook`.
    pub id: String,
    pub title: String,
    pub entry_count: u32,
    /// The Section 15 copyright line, when the legal notices have one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryStub {
    pub id: EntryId,
    pub kind: EntryKind,
    pub title: String,
    /// The primary publication id.
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<EntryId>,
    /// Shard path relative to the version directory, such as `entries/spell-000.json`.
    pub shard: String,
}

/// `legal.json`: the data behind the attribution and legal-information page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalNotices {
    pub licenses: Vec<LicenseText>,
    /// The Product Identity declaration, verbatim.
    pub product_identity: String,
    /// The Paizo Community Use Policy notice, verbatim, with the product name filled in.
    pub community_use_notice: String,
    pub section_15: Vec<CopyrightNotice>,
    pub datasets: Vec<DatasetNotice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseText {
    /// Referenced by [`RuleEntry::license_refs`], such as `ogl-1.0a`.
    pub id: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyrightNotice {
    pub title: String,
    pub notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetNotice {
    pub id: String,
    pub repository: String,
    pub commit: String,
    pub note: String,
}

/// `coverage.json`: what was included, what was left out, and why.
/// Spec section 2 forbids exclusions that silently disappear.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CoverageReport {
    pub input_digest: String,
    pub by_publication: Vec<PublicationCoverage>,
    pub by_kind: BTreeMap<String, u32>,
    /// Upstream directories not selected for this build.
    pub excluded: Vec<Exclusion>,
    /// Records selected for this build but dropped. A non-empty list is visible in the app.
    pub rejected: Vec<Rejection>,
    pub links: LinkCoverage,
    /// Upstream content that was removed on purpose, such as images, with counts.
    pub stripped: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationCoverage {
    pub source_id: String,
    pub title: String,
    pub upstream_records: u32,
    pub included_entries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusion {
    pub path: String,
    pub records: u32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejection {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LinkCoverage {
    pub resolved: u32,
    /// The target exists upstream but is outside this build's selection.
    pub out_of_selection: u32,
    /// The target does not exist upstream, or the href is empty.
    pub broken: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_id_accepts_slug_paths_only() {
        assert!(EntryId::parse("core-rulebook/spells/fireball").is_ok());
        for bad in [
            "", "/a", "a/", "a//b", "A/b", "a b", "-a", "a-", "a/-b", "é",
        ] {
            assert!(EntryId::parse(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn entry_id_rejects_invalid_values_during_deserialization() {
        assert!(serde_json::from_str::<EntryId>("\"Bad Id\"").is_err());
    }

    #[test]
    fn kind_slugs_round_trip() {
        for kind in EntryKind::ALL {
            assert_eq!(EntryKind::from_slug(kind.slug()), Some(kind));
        }
    }

    #[test]
    fn challenge_rating_orders_fractions_before_whole_numbers() {
        let parse = |v| ChallengeRating::parse(v).unwrap();
        assert!(parse("1/8") < parse("1/6"));
        assert!(parse("1/6") < parse("1/4"));
        assert_eq!(parse("1/6").display(), "1/6");
        assert!(parse("1/2") < parse("1"));
        assert!(parse("3") < parse("12"));
        assert_eq!(parse("1/2").display(), "1/2");
        assert_eq!(parse("12").display(), "12");
        assert_eq!(ChallengeRating::parse("&mdash"), None);
    }

    #[test]
    fn rich_content_round_trips_through_json() {
        let block = Block::Paragraph {
            style: ParagraphStyle::StatLine,
            content: vec![
                Inline::text("See "),
                Inline::Link {
                    target: EntryId::parse("core-rulebook/spells/fireball").unwrap(),
                    anchor: None,
                    c: vec![Inline::Em {
                        c: vec![Inline::text("fireball")],
                    }],
                },
            ],
        };
        let json = serde_json::to_string(&block).unwrap();
        assert_eq!(serde_json::from_str::<Block>(&json).unwrap(), block);
    }
}
