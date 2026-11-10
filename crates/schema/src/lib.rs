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
//! <content_version>/relations.json RelationIndex (schema 3)
//! <content_version>/lists.json     SpellLists (schema 3)
//! <content_version>/books.json     BookIndex (schema 3)
//! <content_version>/option-groups.json OptionGroupIndex (schema 4)
//! <content_version>/replaces.json  ReplacesIndex (schema 4)
//! <content_version>/aliases.json   AliasIndex (optional, after the canonical cutover)
//! <content_version>/backlinks.json BacklinkIndex descriptor (optional)
//! <content_version>/backlinks/<NN>.json   Vec<BacklinkTarget> shards (optional)
//! ```
//!
//! Schema 3 adds the relation, spell-list, and book-order files. They are listed in the
//! manifest and hashed like every other file. Libraries of schema 1 and 2 do not have them;
//! a reader that accepts those versions must treat the files as optional. `aliases.json`
//! is a separate optional file with its own format version, so its presence does not change
//! the schema version.
//!
//! The reverse body-link index (`backlinks.json` plus its shards) is optional and additive.
//! Its descriptor carries its own [`BACKLINK_FORMAT`], so adding or removing it does not
//! change [`SCHEMA_VERSION`]. An older library has no descriptor and the reader treats the
//! index as empty; an older client ignores the extra manifest files.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub mod books;
pub mod headings;
pub mod validate;

/// Version of the serialized content format.
///
/// 4: attached entries. `RelationKind::{Variant, Addon, Revision}` with an optional label on
/// a [`RelationEdge`], `EntryKind::ClassOption`, `ClassOptionFields` and `ArchetypeFields`
/// (each carrying `replaces` and `alters`), and the derived files `option-groups.json` and
/// `replaces.json`.
///
/// [`RuleEntry::sources`] and [`RuleEntry::aon`] are additive and optional: a library without
/// them omits the key, and an older reader ignores an unknown key, so they do not change this
/// version. The optional `replaces_features` and `alters_features` lists on the attached fields
/// are additive for the same reason.
pub const SCHEMA_VERSION: u32 = 4;

/// Name of the relation index, relative to the version directory.
pub const RELATIONS_FILE: &str = "relations.json";
/// Name of the class-option group index, relative to the version directory (schema 4).
pub const OPTION_GROUPS_FILE: &str = "option-groups.json";
/// Name of the typed replacement index, relative to the version directory (schema 4).
pub const REPLACES_FILE: &str = "replaces.json";
/// Name of the spell-list index, relative to the version directory.
pub const SPELL_LISTS_FILE: &str = "lists.json";
/// Name of the book-order index, relative to the version directory.
pub const BOOKS_FILE: &str = "books.json";
/// Name of the reverse body-link descriptor, relative to the version directory. Optional:
/// a library built before this index has no such file.
pub const BACKLINKS_FILE: &str = "backlinks.json";
/// Directory of the reverse body-link shards, relative to the version directory.
pub const BACKLINKS_DIR: &str = "backlinks";
/// Version of the reverse body-link format. Independent of [`SCHEMA_VERSION`].
pub const BACKLINK_FORMAT: u32 = 1;
/// Number of shards in the reverse body-link index. Sixteen keeps one entry view to
/// roughly 1/16 of the index while staying small enough to name each shard by two digits.
pub const BACKLINK_SHARDS: u32 = 16;
/// Name of the legacy-to-canonical alias map, relative to the version directory.
/// Optional: a library built before the canonical cutover has no such file.
pub const ALIASES_FILE: &str = "aliases.json";
/// Version of the alias file format. Independent of [`SCHEMA_VERSION`] and of the
/// search index format, as `docs/research/canonical-id-design.md` requires.
pub const ALIAS_FORMAT: u32 = 1;

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
    PrestigeClass,
    Race,
    Skill,
    Affliction,
    Trap,
    /// A character trait or drawback (schema 3).
    Trait,
    /// A racial trait of a race, standard or alternate (schema 3).
    RacialTrait,
    /// A class archetype (schema 3).
    Archetype,
    /// An option picked in place of a class feature, such as a rage power (schema 4).
    ClassOption,
}

impl EntryKind {
    pub const ALL: [EntryKind; 15] = [
        EntryKind::Rule,
        EntryKind::Spell,
        EntryKind::Feat,
        EntryKind::Creature,
        EntryKind::Item,
        EntryKind::Class,
        EntryKind::PrestigeClass,
        EntryKind::Race,
        EntryKind::Skill,
        EntryKind::Affliction,
        EntryKind::Trap,
        EntryKind::Trait,
        EntryKind::RacialTrait,
        EntryKind::Archetype,
        EntryKind::ClassOption,
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
            EntryKind::PrestigeClass => "prestige_class",
            EntryKind::Race => "race",
            EntryKind::Skill => "skill",
            EntryKind::Affliction => "affliction",
            EntryKind::Trap => "trap",
            EntryKind::Trait => "trait",
            EntryKind::RacialTrait => "racial_trait",
            EntryKind::Archetype => "archetype",
            EntryKind::ClassOption => "class_option",
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
            EntryKind::PrestigeClass => "Prestige classes",
            EntryKind::Race => "Races",
            EntryKind::Skill => "Skills",
            EntryKind::Affliction => "Afflictions",
            EntryKind::Trap => "Traps",
            EntryKind::Trait => "Traits",
            EntryKind::RacialTrait => "Racial traits",
            EntryKind::Archetype => "Archetypes",
            EntryKind::ClassOption => "Class options",
        }
    }
}

/// One canonical rules record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEntry {
    pub id: EntryId,
    pub kind: EntryKind,
    pub title: String,
    /// The Archives of Nethys page for this entry, as a path relative to
    /// `https://aonprd.com/`, such as `FeatDisplay.aspx?ItemName=Power%20Attack`.
    ///
    /// Optional and additive: an entry without a confident match omits the key, and an older
    /// reader ignores an unknown key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aon: Option<String>,
    /// Publication ids. Each id must exist in [`Catalog::sources`].
    pub source_ids: Vec<String>,
    /// Page citations in printed order. Optional and additive: an entry without rows omits the
    /// key, and an older reader ignores it. Each publication must also appear in
    /// [`Self::source_ids`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Citation>,
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
    /// The printed group of a `race` entry (issue #196). Optional and additive: absent from an
    /// older library, and omitted for every other kind, so schema 4 still reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub race_group: Option<RaceGroup>,
    /// Every resolved internal reference that the body holds, deduplicated and sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<EntryReference>,
    /// Where an imported entry came from. Empty for a hand-written entry.
    pub provenance: Vec<Provenance>,
    /// License ids. Each id must exist in [`LegalNotices::licenses`].
    pub license_refs: Vec<String>,
    /// Typed edges from this entry to others (schema 3). The inverse direction is derived
    /// from [`RelationIndex`] and is not stored on the target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
}

/// The printed group of a race. The Advanced Race Guide names Core, Featured, and Uncommon
/// races; every race outside those chapters is Other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaceGroup {
    /// The seven races of the Core Rulebook and the Advanced Race Guide's Core Races chapter.
    CoreRace,
    /// The Advanced Race Guide's Featured Races chapter.
    FeaturedRace,
    /// The Advanced Race Guide's Uncommon Races chapter.
    UncommonRace,
    /// A race printed outside the Advanced Race Guide's three race chapters.
    OtherRace,
}

impl RaceGroup {
    pub const ALL: [RaceGroup; 4] = [
        RaceGroup::CoreRace,
        RaceGroup::FeaturedRace,
        RaceGroup::UncommonRace,
        RaceGroup::OtherRace,
    ];

    /// The corpus and index spelling, used by the `race_group` facet.
    pub fn slug(self) -> &'static str {
        match self {
            RaceGroup::CoreRace => "core_race",
            RaceGroup::FeaturedRace => "featured_race",
            RaceGroup::UncommonRace => "uncommon_race",
            RaceGroup::OtherRace => "other_race",
        }
    }

    /// Reads a slug. `None` for a value outside [`RaceGroup::ALL`].
    pub fn parse(value: &str) -> Option<Self> {
        RaceGroup::ALL
            .into_iter()
            .find(|group| group.slug() == value)
    }
}

/// What a [`Relation`] says about its source and target.
///
/// The source is always the entry that holds the relation. Inverse questions, such as
/// "which archetypes does this class have", are answered from [`RelationIndex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    /// The source is a part of the target: a racial trait of a race.
    Part,
    /// The source is an archetype of the target class.
    Archetype,
    /// The source requires the target, such as a feat that requires another feat.
    /// The original wording stays in [`FeatFields::prerequisites`].
    Prerequisite,
    /// The source is a variant of the target spell (schema 4).
    Variant,
    /// The source is an add-on to the target entry: a mythic version, Stamina rules, or a
    /// third-party change (schema 4).
    Addon,
    /// The source is a third-party revision of the target: revised, removed, or merged (schema 4).
    Revision,
}

impl RelationKind {
    pub const ALL: [RelationKind; 6] = [
        RelationKind::Part,
        RelationKind::Archetype,
        RelationKind::Prerequisite,
        RelationKind::Variant,
        RelationKind::Addon,
        RelationKind::Revision,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            RelationKind::Part => "part",
            RelationKind::Archetype => "archetype",
            RelationKind::Prerequisite => "prerequisite",
            RelationKind::Variant => "variant",
            RelationKind::Addon => "addon",
            RelationKind::Revision => "revision",
        }
    }
}

/// One edge from the holding entry to `target`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Relation {
    pub kind: RelationKind,
    pub target: EntryId,
    /// Qualifier. `or` on a [`RelationKind::Prerequisite`] marks one alternative of an
    /// either-or requirement in the original wording; `alternate` on a [`RelationKind::Part`]
    /// marks an alternate racial trait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntryReference {
    pub target: EntryId,
    /// Heading anchor within the target entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
}

/// One printed page citation: a publication and, when the source line gives one, its page.
///
/// `page` stays a string because printed citations are not always one number: a range such as
/// `12-14` or a non-numeric page such as `inside cover` must survive as written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Citation {
    /// Publication id, which must also appear in [`RuleEntry::source_ids`].
    pub publication: String,
    /// Printed page, such as `221`, `12-14` or `inside cover`. Omitted when the citation has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
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
    /// Character traits and drawbacks (schema 3).
    Trait(TraitFields),
    /// Racial traits (schema 3).
    RacialTrait(RacialTraitFields),
    /// Archetype replacements and alterations (schema 4).
    Archetype(ArchetypeFields),
    /// Class options and the items they replace (schema 4).
    ClassOption(ClassOptionFields),
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
    /// The original prerequisite wording, kept even when parts of it are also typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<String>,
    /// Feats that the prerequisite wording names and that resolve to an entry
    /// (schema 3), sorted by id. The same edges are `Prerequisite` relations. An
    /// either-or list contributes all of its resolved feats.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisite_feats: Vec<EntryId>,
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
    /// Upstream subtype: `core`, `base`, `hybrid`, `occult`, `unchained`, `alternate`,
    /// `prestige`, or `npc`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_die: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_ranks: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_skills: Option<String>,
}

/// Fields of a character trait or drawback.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TraitFields {
    /// `Combat`, `Faith`, `Magic`, `Social`, `Campaign`, `Regional`, `Religion`, `Race`,
    /// or `Drawback`.
    pub category: String,
    /// The race for a `Race` trait, such as `Half-Elf`; `Bloodline` for bloodline traits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Fields of a racial trait.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RacialTraitFields {
    /// True for an alternate racial trait, which replaces a standard trait of the race.
    pub alternate: bool,
}

/// Fields of an archetype: the printed class features it replaces and alters, in print order.
/// Empty vectors are omitted from the serialized form.
///
/// `replaces_features` and `alters_features` hold the heading anchors of the base class
/// entry for the printed names in `replaces` and `alters`, in the same order. A printed name
/// that no heading resolves is omitted from the feature list. The anchors are stable, so a
/// client can compare two archetypes, or an archetype with a class option, without parsing
/// display names.
///
/// The flat list describes a whole-feature trade. Per-power trades (#251), such as a bloodline
/// power or a domain power, will add a separate optional pair beside these fields; this list
/// keeps its meaning and stays valid.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ArchetypeFields {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alters: Vec<String>,
    /// Heading anchors in the base class entry, aligned with the confidently resolved
    /// entries of `replaces`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces_features: Vec<String>,
    /// Heading anchors in the base class entry, aligned with the confidently resolved
    /// entries of `alters`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alters_features: Vec<String>,
}

/// Fields of a class option: the option group it belongs to and the printed features it
/// replaces and alters, in print order.
///
/// `replaces_features` and `alters_features` hold heading anchors of the option group's
/// class entries, with the same resolution rule as [`ArchetypeFields`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClassOptionFields {
    /// A slug from the corpus `option-groups.toml`.
    pub group: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alters: Vec<String>,
    /// Heading anchors in the option group's class entries, aligned with the confidently
    /// resolved entries of `replaces`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces_features: Vec<String>,
    /// Heading anchors in the option group's class entries, aligned with the confidently
    /// resolved entries of `alters`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alters_features: Vec<String>,
}

/// `relations.json` (schema 3): every `Relation` of every entry, flattened so that the
/// inverse direction is a lookup over this one small file instead of a scan of all shards.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RelationIndex {
    /// Edge count per [`RelationKind::slug`].
    pub counts: BTreeMap<String, u32>,
    /// Sorted by source, kind, then target. No duplicates.
    pub edges: Vec<RelationEdge>,
}

/// `[source, kind, target]` or `[source, kind, target, label]` in JSON. The label is the
/// optional qualifier of the relation (`or`, `alternate`, `mythic`, `stamina`, `changes`,
/// `removed`, or `merged`); an edge without one keeps the three-element form so an unchanged
/// library serializes byte for byte as it did before schema 4.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationEdge(
    pub EntryId,
    pub RelationKind,
    pub EntryId,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub Option<String>,
);

impl RelationEdge {
    /// An edge with no label, in the compact three-element form.
    pub fn plain(source: EntryId, kind: RelationKind, target: EntryId) -> Self {
        Self(source, kind, target, None)
    }

    pub fn label(&self) -> Option<&str> {
        self.3.as_deref()
    }
}

/// `option-groups.json` (schema 4): the class-option groups a corpus declares, with the
/// `class_option` entries that belong to each. Sorted by slug; entries within a group are
/// sorted by id. An empty index means the corpus has no `option-groups.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OptionGroupIndex {
    pub groups: Vec<OptionGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionGroup {
    /// Slug from `option-groups.toml`, such as `arcane-discovery`.
    pub slug: String,
    /// Plural display label, such as `Arcane discoveries`.
    pub label: String,
    /// Singular display label, such as `Arcane discovery`.
    pub singular: String,
    /// Class entries that take the group, sorted by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<EntryId>,
    /// `class_option` entries in the group, sorted by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<EntryId>,
}

/// `replaces.json` (schema 4): the printed items that an archetype or class option replaces.
/// Sorted by id. The typed value also stays on the entry; this file only indexes it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReplacesIndex {
    pub entries: Vec<ReplacesRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplacesRow {
    pub id: EntryId,
    /// Printed feature names, in print order.
    pub replaces: Vec<String>,
}

/// `backlinks.json` (optional): the descriptor of the reverse body-link index.
///
/// The index answers "which entries link here" without scanning every shard. It holds one
/// row per source and target: self-links and links that duplicate a typed relation are
/// excluded, and a source that reaches one target through several anchors keeps no anchor.
/// The target rows are split across [`BacklinkIndex::shards`] files, named by
/// [`backlink_shard_path`], so one entry view loads only its bucket.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BacklinkIndex {
    pub format: u32,
    pub shards: u32,
    /// Number of source-to-target rows across all shards.
    pub links: u32,
    /// Number of target entries that have at least one backlink.
    pub targets: u32,
}

impl BacklinkIndex {
    /// Rejects a format this client does not read and a shard count it cannot address.
    pub fn validate(&self) -> Result<(), String> {
        if self.format != BACKLINK_FORMAT {
            return Err(format!(
                "backlinks.json has format {}; this client reads {BACKLINK_FORMAT}",
                self.format
            ));
        }
        if self.shards == 0 || self.shards > 100 {
            return Err(format!(
                "backlinks.json names {} shards; expected 1 to 100",
                self.shards
            ));
        }
        Ok(())
    }
}

/// One target and the entries whose bodies link to it, in a shard file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacklinkTarget(pub EntryId, pub Vec<BacklinkSource>);

/// `[source]` or `[source, anchor]` in JSON. The anchor is a heading in the target entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BacklinkSource(
    pub EntryId,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub Option<String>,
);

/// The shard that holds `target`. FNV-1a keeps the assignment stable across builds and
/// runtimes. Ids share long prefixes such as `ref/`, so a literal prefix would put nearly
/// every target in one shard.
pub fn backlink_shard(target: &str, shards: u32) -> u32 {
    let shards = shards.max(1);
    let mut hash: u32 = 0x811c_9dc5;
    for byte in target.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash % shards
}

/// The manifest path of one shard, relative to the version directory.
pub fn backlink_shard_path(shard: u32) -> String {
    format!("{BACKLINKS_DIR}/{shard:02}.json")
}

/// `lists.json` (schema 3): spell lists per class and level, without loading any shard.
///
/// Spells repeat across lists, so each is stored once in `spells` and the lists hold
/// positions in it. The data comes from `SpellFields::levels`, which stays the source
/// of truth; this file only indexes it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpellLists {
    /// Sorted by id.
    pub spells: Vec<SpellListSpell>,
    /// Sorted by name, ignoring case.
    pub classes: Vec<SpellListClass>,
    /// What a list row shows besides the title. **Positional:** entry `i` belongs to
    /// `spells[i]`, so the two vectors have the same order and the same length. A level's
    /// `spells` positions index both. Empty in a library built before rows carried it; the
    /// reader then shows the title alone. Anything else fails validation. Additive: it
    /// arrived under schema 3 without a [`SCHEMA_VERSION`] bump.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<SpellListDetail>,
}

/// The facts a spell-list row shows under its title. A missing fact is empty and is left
/// out of the file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpellListDetail {
    /// The school alone, lowercase, as the `school` search facet spells it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub school: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub casting_time: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub saving_throw: String,
    /// The first sentence of the description, cut at a word boundary to fit one line.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gist: String,
}

/// `[id, title]` in JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellListSpell(pub EntryId, pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellListClass {
    /// Class name as the spells name it, with one spelling per list.
    pub name: String,
    /// The class entry when one exists. Lists without an entry stay visible, such as
    /// classes from books outside the library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<EntryId>,
    /// Ascending by level.
    pub levels: Vec<SpellListLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellListLevel {
    pub level: u8,
    /// Positions in [`SpellLists::spells`], sorted by title.
    pub spells: Vec<u32>,
}

/// `books.json` (schema 3): the original chapter order of each publication, and nothing else.
/// It is not a category listing: entries appear where the printed book puts them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BookIndex {
    /// One tree per publication that has a curated chapter list, in the curated order.
    pub books: Vec<BookTree>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookTree {
    /// A [`SourceInfo::id`].
    pub source: String,
    pub title: String,
    /// Where the chapter list comes from and how sure the curator is, from `book-order.toml`.
    /// Every list is a draft until a person checks it against the printed book, and the
    /// reader shows this text next to the contents.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub basis: String,
    /// Open questions about the chapter list of this book.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review: Vec<String>,
    /// Chapters in book order.
    pub chapters: Vec<BookNode>,
}

/// A chapter, a section, or an entry in a book. A node with an `entry` opens that entry
/// (and with an `anchor` too, a heading inside it). A node without an `entry` only groups
/// its children: a chapter title, or a heading that holds nested entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookNode {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<EntryId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// Set on a chapter whose position or content is a guess, with the reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
    /// Authored in the corpus `books.toml`: show this node as one row in Contents at any level,
    /// including a top-level chapter. The row links to the node's entry when it has one, else to
    /// its chapter page. Additive and optional, so an older library reads as `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub collapse: bool,
    /// Computed by the compiler, never authored in the corpus: the node is itself an entry and
    /// that entry's body lists every entry below it. Contents and the chapter page show the node
    /// as one row and do not repeat the children. Additive and optional, so an older library
    /// reads as `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lists_children: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<BookNode>,
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
    /// The corpus tag that `sources/corpus.toml` pins, such as `v0.1.6`. Additive and
    /// optional: older libraries have no field, and older readers ignore it. It is display
    /// information only, so it does not change [`SCHEMA_VERSION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpus_tag: Option<String>,
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

/// `aliases.json`: the one-step legacy-to-canonical map.
///
/// The file is optional. An older library has no file, and the client treats it as an
/// empty map. `entries` and `retired` are sorted by `old`, each old id appears in exactly
/// one list, and no target appears as an old id, so resolution is one step. [`AliasIndex::validate`]
/// enforces all three rules before the map is used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasIndex {
    pub format: u32,
    pub legacy: AliasLegacy,
    #[serde(default)]
    pub entries: Vec<AliasEntry>,
    #[serde(default)]
    pub retired: Vec<RetiredEntry>,
}

impl Default for AliasIndex {
    fn default() -> Self {
        Self {
            format: ALIAS_FORMAT,
            legacy: AliasLegacy {
                count: 0,
                sha256: String::new(),
            },
            entries: Vec::new(),
            retired: Vec::new(),
        }
    }
}

impl AliasIndex {
    /// Rejects a format this client does not read and an unsorted or duplicated map.
    pub fn validate(&self) -> Result<(), String> {
        if self.format != ALIAS_FORMAT {
            return Err(format!(
                "aliases.json has format {}; this client reads {ALIAS_FORMAT}",
                self.format
            ));
        }
        for pair in self.entries.windows(2) {
            if pair[0].old >= pair[1].old {
                return Err(format!(
                    "aliases.json entries are not sorted by old id: {:?}",
                    pair[1].old
                ));
            }
        }
        for pair in self.retired.windows(2) {
            if pair[0].old >= pair[1].old {
                return Err(format!(
                    "aliases.json retired rows are not sorted by old id: {:?}",
                    pair[1].old
                ));
            }
        }
        // Sortedness above makes these lookups binary searches. A target that is itself an
        // old id would make resolution a chain; an old id in both lists has two dispositions.
        for entry in &self.entries {
            if self.is_old(&entry.target) {
                return Err(format!(
                    "aliases.json target {:?} is also an old id; chains are forbidden",
                    entry.target
                ));
            }
        }
        for retired in &self.retired {
            if self
                .entries
                .binary_search_by(|entry| entry.old.cmp(&retired.old))
                .is_ok()
            {
                return Err(format!(
                    "aliases.json old id {:?} appears in both entries and retired",
                    retired.old
                ));
            }
        }
        Ok(())
    }

    /// Whether `id` has a disposition in either table. Only correct after the sortedness
    /// checks above; callers outside [`Self::validate`] hold a validated map.
    fn is_old(&self, id: &EntryId) -> bool {
        self.entries
            .binary_search_by(|entry| entry.old.cmp(id))
            .is_ok()
            || self
                .retired
                .binary_search_by(|entry| entry.old.cmp(id))
                .is_ok()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.retired.is_empty()
    }

    /// The one alias row for `id`, found by binary search over the sorted list.
    pub fn alias(&self, id: &EntryId) -> Option<&AliasEntry> {
        self.entries
            .binary_search_by(|entry| entry.old.cmp(id))
            .ok()
            .map(|index| &self.entries[index])
    }

    /// The retirement row for `id`, if the map retired it.
    pub fn retirement(&self, id: &EntryId) -> Option<&RetiredEntry> {
        self.retired
            .binary_search_by(|entry| entry.old.cmp(id))
            .ok()
            .map(|index| &self.retired[index])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasLegacy {
    pub count: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasEntry {
    pub old: EntryId,
    pub target: EntryId,
    pub basis: AliasBasis,
}

/// Why a legacy id has a canonical target. The client does not branch on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasBasis {
    Direct,
    Derived,
    Carried,
    Shared,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetiredEntry {
    pub old: EntryId,
    pub reason: RetirementReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Why a legacy id has no canonical target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// `catalog.json`: everything that browsing needs without loading entry shards.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub sources: Vec<SourceInfo>,
    /// Sorted by id.
    pub entries: Vec<EntryStub>,
}

impl Catalog {
    /// Rejects a catalog whose entries are not strictly ascending by id. The client
    /// binary-searches this list, so an unsorted or duplicated list would silently make
    /// membership answers wrong; a catalog violation is a corrupt library, not a miss.
    pub fn validate(&self) -> Result<(), String> {
        for pair in self.entries.windows(2) {
            if pair[0].id >= pair[1].id {
                return Err(format!(
                    "catalog.json entries are not sorted by unique id: {:?} then {:?}",
                    pair[0].id, pair[1].id
                ));
            }
        }
        Ok(())
    }
}

/// The optional grouping a publication belongs to in the corpus series fields (#159).
///
/// Only three lines are grouped: an Adventure Path and its issue number, a standalone
/// Module (ordered by publication date), and a Pathfinder Society season and scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SeriesKind {
    /// A volume of a Pathfinder Adventure Path, such as issue 1 of Rise of the Runelords.
    AdventurePath,
    /// A standalone Pathfinder Module.
    Module,
    /// A Pathfinder Society scenario, identified by its season and number.
    Society,
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
    /// Reviewed publication category, such as `RPG` or `Player Companion`.
    ///
    /// Optional and additive: an older catalog has no field, and an older reader ignores
    /// it. The Books page groups a source without one under `Unknown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// First publication date as an ISO 8601 calendar date (`YYYY-MM-DD`).
    ///
    /// Optional and additive, like [`Self::category`]. The ISO form sorts correctly as
    /// text, so no timestamp or locale parsing is needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<String>,
    /// The optional series line, from `series_kind` in `publications.toml`.
    ///
    /// Optional and additive, like [`Self::category`]. The three kinds are the only ones the
    /// corpus groups: [`SeriesKind::AdventurePath`], [`SeriesKind::Module`] and
    /// [`SeriesKind::Society`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_kind: Option<SeriesKind>,
    /// The Adventure Path name, such as `Rise of the Runelords`. Present only for
    /// [`SeriesKind::AdventurePath`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// The issue within the series: the issue within its Adventure Path, such as `1`, or the
    /// Society scenario, such as `3-01`. Absent for a Module, which is ordered by publication
    /// date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_number: Option<String>,
    /// The Pathfinder Society season, such as `3`. Present only for [`SeriesKind::Society`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<u16>,
    /// True when the publication is not published by Paizo, from the corpus
    /// `third_party` marker in `publications.toml`.
    ///
    /// Optional and additive, like [`Self::category`]: an older catalog has no field, and
    /// a source without it is a Paizo book. Serialized only when true.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub third_party: bool,
    /// The Archives of Nethys source page, as a path relative to `https://aonprd.com/`.
    ///
    /// Optional and additive, like [`Self::category`]. It is `SourceDisplay.aspx` with the
    /// AoN source name in `FixedSource`, and an older catalog has no field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aon: Option<String>,
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
    /// Publication titles shipped as assumed Open Game Content while their
    /// Section 15 notice is pending. Additive and optional: older libraries
    /// have no field, and older readers ignore it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notice_pending: Vec<String>,
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
    /// Relations, spell lists, and book order (schema 3). Absent in older reports.
    #[serde(default)]
    pub relations: RelationCoverage,
    /// Reverse body-link index (optional). Absent in older reports and libraries.
    #[serde(default)]
    pub backlinks: BacklinkCoverage,
}

/// The part of [`CoverageReport`] that the one-time import decided and the entries cannot
/// restate. The corpus stores it as `history.toml`. It is frozen: nothing edits it, and every
/// other part of the report is derived from the entries, the publication list, the book index,
/// and the exclusion rows.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CoverageHistory {
    /// Digest of the upstream inputs the import read.
    pub input_digest: String,
    /// Upstream record counts per publication.
    #[serde(default)]
    pub publications: Vec<UpstreamPublication>,
    /// How the import read feat prerequisite wording, over the records it converted.
    #[serde(default)]
    pub prerequisites: PrerequisiteCoverage,
    /// Racial traits and archetypes the import could not place under a parent.
    #[serde(default)]
    pub unplaced_parts: Vec<Unplaced>,
    /// Upstream records of a typed family that stayed ordinary rules text.
    #[serde(default)]
    pub unpromoted: Vec<Rejection>,
    /// How upstream page numbers placed entries in books.
    #[serde(default)]
    pub book_order: BookOrderCoverage,
}

/// One publication's upstream record count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpstreamPublication {
    pub source_id: String,
    pub upstream_records: u32,
    /// Set only when the coverage title differs from the catalogue title, or the catalogue
    /// has no such publication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// What the reverse body-link pass did, including the links it deliberately left out.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BacklinkCoverage {
    /// Source-to-target rows in the index.
    pub links: u32,
    /// Targets with at least one row.
    pub targets: u32,
    /// Links from an entry to itself. They add nothing to "Referenced by".
    pub excluded_self: u32,
    /// Links whose source already states a typed relation to the same target. The relation
    /// panels show those, so a second body-link row would duplicate them.
    pub excluded_relation: u32,
    /// Bytes of the descriptor plus every shard.
    pub bytes: u64,
}

/// What the relation, spell-list, and book-order passes did, including what they could not do.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RelationCoverage {
    /// Edge count per relation kind.
    pub edges: BTreeMap<String, u32>,
    pub prerequisites: PrerequisiteCoverage,
    pub spell_lists: SpellListCoverage,
    /// Racial traits and archetypes that have no parent, with the reason.
    pub unplaced_parts: Vec<Unplaced>,
    /// Upstream records of a typed family that stay ordinary rules text, with the reason.
    pub unpromoted: Vec<Rejection>,
    pub book_order: BookOrderCoverage,
}

/// Reading of feat prerequisite wording. Every comma-separated requirement counts once;
/// an either-or list counts each alternative.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PrerequisiteCoverage {
    pub feats: u32,
    pub feats_with_text: u32,
    pub feats_with_resolved_feat: u32,
    pub requirements: u32,
    /// Requirements per reading: `feat`, `ability_score`, `base_attack_bonus`,
    /// `skill_ranks`, `level`, `class_feature`, `racial_trait`, `race`, `other`.
    pub by_reading: BTreeMap<String, u32>,
    /// Names that look like feats but match none, or more than one.
    pub ambiguous: u32,
    /// Requirements in the `other` reading, sorted by feat id.
    pub unread: Vec<UnreadRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreadRequirement {
    pub feat: EntryId,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpellListCoverage {
    pub spells: u32,
    pub classes: u32,
    /// `(class, level)` pairs in the upstream data, then after removing repeats.
    pub pairs_upstream: u32,
    pub pairs_distinct: u32,
    pub classes_with_entry: Vec<String>,
    /// Class names that no class entry carries, with their distinct spell counts.
    pub unmapped: Vec<UnmappedClass>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmappedClass {
    pub name: String,
    pub spells: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BookOrderCoverage {
    pub placed: u32,
    /// Entries that no chapter claims. Never dropped from the library.
    pub unplaced: Vec<Unplaced>,
    pub by_publication: Vec<BookPlacement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookPlacement {
    pub source_id: String,
    pub placed: u32,
    pub unplaced: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unplaced {
    pub entry: String,
    pub reason: String,
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
    fn catalog_rejects_an_unsorted_or_duplicated_entry_list() {
        let stub = |id: &str| EntryStub {
            id: EntryId::parse(id).unwrap(),
            kind: EntryKind::Rule,
            title: id.into(),
            source: "s".into(),
            parent: None,
            shard: "entries/rule-000.json".into(),
        };
        let catalog = |ids: &[&str]| Catalog {
            sources: vec![],
            entries: ids.iter().map(|id| stub(id)).collect(),
        };
        assert!(catalog(&[]).validate().is_ok());
        assert!(
            catalog(&["ref/race/trait", "ref/rule/direct", "ref/rule/shared"])
                .validate()
                .is_ok()
        );

        // The client binary-searches, so declaration order in the source is not enough.
        let unsorted = catalog(&["ref/rule/direct", "ref/rule/shared", "ref/race/trait"]);
        let message = unsorted.validate().unwrap_err();
        assert!(message.contains("not sorted by unique id"), "{message}");
        let duplicate = catalog(&["ref/rule/a", "ref/rule/a"]);
        assert!(duplicate.validate().is_err());
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
    fn relations_are_omitted_when_empty_and_edges_are_arrays() {
        let json = r#"{"id":"a","kind":"rule","title":"A","source_ids":["s"],"body":[],"fields":{"kind":"none"},"provenance":[],"license_refs":[]}"#;
        let old: RuleEntry = serde_json::from_str(json).unwrap();
        assert!(
            old.relations.is_empty(),
            "schema 1 and 2 entries have no relations"
        );
        assert!(!serde_json::to_string(&old).unwrap().contains("relations"));

        let relation = Relation {
            kind: RelationKind::Prerequisite,
            target: EntryId::parse("core-rulebook/feats/dodge").unwrap(),
            label: Some("or".into()),
        };
        assert_eq!(
            serde_json::to_string(&relation).unwrap(),
            r#"{"kind":"prerequisite","target":"core-rulebook/feats/dodge","label":"or"}"#
        );
        let edge = RelationEdge::plain(
            EntryId::parse("a").unwrap(),
            RelationKind::Part,
            EntryId::parse("b").unwrap(),
        );
        assert_eq!(serde_json::to_string(&edge).unwrap(), r#"["a","part","b"]"#);
        let labelled = RelationEdge(
            EntryId::parse("a").unwrap(),
            RelationKind::Addon,
            EntryId::parse("b").unwrap(),
            Some("mythic".into()),
        );
        assert_eq!(
            serde_json::to_string(&labelled).unwrap(),
            r#"["a","addon","b","mythic"]"#
        );
        assert_eq!(
            serde_json::from_str::<RelationEdge>(r#"["a","part","b"]"#).unwrap(),
            edge
        );
        assert_eq!(
            serde_json::from_str::<RelationEdge>(r#"["a","addon","b","mythic"]"#).unwrap(),
            labelled
        );
    }

    #[test]
    fn new_kinds_have_slugs_and_labels() {
        for (kind, slug) in [
            (EntryKind::Trait, "trait"),
            (EntryKind::RacialTrait, "racial_trait"),
            (EntryKind::Archetype, "archetype"),
            (EntryKind::ClassOption, "class_option"),
        ] {
            assert_eq!(kind.slug(), slug);
            assert_eq!(EntryKind::from_slug(slug), Some(kind));
            assert!(!kind.label().is_empty());
        }
        assert_eq!(EntryKind::ClassOption.label(), "Class options");
        assert_eq!(EntryKind::ALL.len(), 15);
        for relation in RelationKind::ALL {
            assert!(!relation.slug().is_empty());
        }
        assert_eq!(RelationKind::ALL.len(), 6);
    }

    #[test]
    fn attached_fields_omit_empty_vectors() {
        let archetype = TypedFields::Archetype(ArchetypeFields::default());
        assert_eq!(
            serde_json::to_string(&archetype).unwrap(),
            r#"{"kind":"archetype"}"#
        );
        let option = TypedFields::ClassOption(ClassOptionFields {
            group: "arcane-discovery".into(),
            replaces: vec![],
            alters: vec![],
            replaces_features: vec![],
            alters_features: vec![],
        });
        assert_eq!(
            serde_json::to_string(&option).unwrap(),
            r#"{"kind":"class_option","group":"arcane-discovery"}"#
        );
        let option = TypedFields::ClassOption(ClassOptionFields {
            group: "arcane-discovery".into(),
            replaces: vec!["Arcane bond".into()],
            alters: vec!["Arcane school".into()],
            replaces_features: vec!["arcane-bond".into()],
            alters_features: vec!["arcane-school".into()],
        });
        assert_eq!(
            serde_json::to_string(&option).unwrap(),
            r#"{"kind":"class_option","group":"arcane-discovery","replaces":["Arcane bond"],"alters":["Arcane school"],"replaces_features":["arcane-bond"],"alters_features":["arcane-school"]}"#
        );
    }

    #[test]
    fn alias_index_rejects_a_chain_a_retired_target_and_a_duplicate_old() {
        let parse = |json: &str| serde_json::from_str::<AliasIndex>(json).unwrap();

        let chain = parse(
            r#"{"format":1,"legacy":{"count":2,"sha256":""},"entries":[
                {"old":"a/x","target":"a/y","basis":"direct"},
                {"old":"a/y","target":"a/z","basis":"direct"}
            ],"retired":[]}"#,
        );
        assert!(
            chain
                .validate()
                .unwrap_err()
                .contains("chains are forbidden")
        );

        let retired_target = parse(
            r#"{"format":1,"legacy":{"count":1,"sha256":""},"entries":[
                {"old":"a/w","target":"a/x","basis":"direct"}
            ],"retired":[{"old":"a/x","reason":"no_source"}]}"#,
        );
        assert!(
            retired_target
                .validate()
                .unwrap_err()
                .contains("chains are forbidden")
        );

        let duplicate = parse(
            r#"{"format":1,"legacy":{"count":1,"sha256":""},"entries":[
                {"old":"a/x","target":"a/z","basis":"direct"}
            ],"retired":[{"old":"a/x","reason":"no_source"}]}"#,
        );
        assert!(
            duplicate
                .validate()
                .unwrap_err()
                .contains("both entries and retired")
        );

        let valid = parse(
            r#"{"format":1,"legacy":{"count":2,"sha256":""},"entries":[
                {"old":"a/x","target":"a/z","basis":"direct"}
            ],"retired":[{"old":"a/y","reason":"rights"}]}"#,
        );
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn source_metadata_is_additive_and_unknown_fields_stay_ignored() {
        // A catalog written before the metadata fields existed still opens.
        let old = r#"{"sources":[{"id":"a","title":"A","entry_count":1}],"entries":[]}"#;
        let catalog: Catalog = serde_json::from_str(old).unwrap();
        assert_eq!(catalog.sources[0].category, None);
        assert_eq!(catalog.sources[0].publication_date, None);
        assert_eq!(catalog.sources[0].series_kind, None);
        assert_eq!(catalog.sources[0].series, None);
        assert_eq!(catalog.sources[0].series_number, None);
        assert_eq!(catalog.sources[0].season, None);
        assert!(
            !catalog.sources[0].third_party,
            "an absent marker means a Paizo book"
        );
        assert_eq!(catalog.sources[0].aon, None);

        // A newer catalog carries the fields, and an older reader ignores them because
        // neither Catalog nor SourceInfo denies unknown fields.
        let new = r#"{"sources":[{"id":"a","title":"A","entry_count":1,"category":"RPG","publication_date":"2009-08-01"}],"entries":[]}"#;
        let catalog: Catalog = serde_json::from_str(new).unwrap();
        assert_eq!(catalog.sources[0].category.as_deref(), Some("RPG"));
        assert_eq!(
            catalog.sources[0].publication_date.as_deref(),
            Some("2009-08-01")
        );

        // Unset values disappear from the serialized form, so an unchanged source looks
        // the same as it did before the fields existed.
        let empty = SourceInfo {
            id: "a".into(),
            title: "A".into(),
            aon: None,
            entry_count: 0,
            copyright: None,
            category: None,
            publication_date: None,
            series_kind: None,
            series: None,
            series_number: None,
            season: None,
            third_party: false,
        };
        let json = serde_json::to_string(&empty).unwrap();
        assert!(!json.contains("category"));
        assert!(!json.contains("publication_date"));
        assert!(!json.contains("series_kind"), "{json}");
        assert!(!json.contains("series_number"), "{json}");
        assert!(!json.contains("season"), "{json}");
        assert!(!json.contains("\"third_party\""), "{json}");
        assert!(!json.contains("aon"), "{json}");
        let parsed: SourceInfo = serde_json::from_str(&json).unwrap();
        assert!(!parsed.third_party);

        // A marked source round-trips, and the field keeps the documented order: after the
        // reviewed metadata.
        let marked = SourceInfo {
            category: Some("RPG".into()),
            publication_date: Some("2011-01-01".into()),
            third_party: true,
            ..empty.clone()
        };
        let json = serde_json::to_string(&marked).unwrap();
        assert!(json.contains(r#""third_party":true"#), "{json}");
        let order = |key: &str| {
            json.find(key)
                .unwrap_or_else(|| panic!("no {key} in {json}"))
        };
        assert!(order("publication_date") < order("third_party"), "{json}");
        let read: SourceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(read, marked);

        // The series fields are additive and keep their documented order after the reviewed
        // metadata.
        let series = SourceInfo {
            category: Some("Adventure Path".into()),
            publication_date: Some("2014-09-10".into()),
            series_kind: Some(SeriesKind::AdventurePath),
            series: Some("Iron Gods".into()),
            series_number: Some("1".into()),
            ..empty.clone()
        };
        let json = serde_json::to_string(&series).unwrap();
        assert!(json.contains(r#""series_kind":"adventure-path""#), "{json}");
        let order = |key: &str| {
            json.find(&format!("\"{key}\""))
                .unwrap_or_else(|| panic!("no {key} in {json}"))
        };
        assert!(order("publication_date") < order("series_kind"), "{json}");
        assert!(order("series_kind") < order("series"), "{json}");
        assert!(order("series") < order("series_number"), "{json}");
        let read: SourceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(read, series);

        // A society source carries the season beside its scenario number.
        let society = SourceInfo {
            series_kind: Some(SeriesKind::Society),
            series_number: Some("3-01".into()),
            season: Some(3),
            ..empty
        };
        let json = serde_json::to_string(&society).unwrap();
        assert!(json.contains(r#""season":3"#), "{json}");
        assert_eq!(serde_json::from_str::<SourceInfo>(&json).unwrap(), society);
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

    #[test]
    fn backlink_rows_are_compact_and_accept_a_missing_anchor() {
        let with_anchor = BacklinkSource(
            EntryId::parse("ref/rule/a").unwrap(),
            Some("heading".into()),
        );
        assert_eq!(
            serde_json::to_string(&with_anchor).unwrap(),
            r#"["ref/rule/a","heading"]"#
        );
        let without = BacklinkSource(EntryId::parse("ref/rule/a").unwrap(), None);
        assert_eq!(
            serde_json::to_string(&without).unwrap(),
            r#"["ref/rule/a"]"#
        );
        for json in [r#"["ref/rule/a"]"#, r#"["ref/rule/a",null]"#] {
            assert_eq!(
                serde_json::from_str::<BacklinkSource>(json).unwrap(),
                without
            );
        }
    }

    #[test]
    fn backlink_shards_are_stable_and_cover_every_bucket() {
        assert_eq!(
            backlink_shard("ref/rule/a", 16),
            backlink_shard("ref/rule/a", 16)
        );
        let mut buckets: BTreeMap<u32, usize> = BTreeMap::new();
        for i in 0..512 {
            let bucket = backlink_shard(&format!("ref/rule/{i}"), BACKLINK_SHARDS);
            *buckets.entry(bucket).or_default() += 1;
        }
        assert_eq!(buckets.len(), BACKLINK_SHARDS as usize);
        assert_eq!(backlink_shard_path(3), "backlinks/03.json");
    }

    #[test]
    fn backlink_descriptor_rejects_a_foreign_format_or_shard_count() {
        let good = BacklinkIndex {
            format: BACKLINK_FORMAT,
            shards: BACKLINK_SHARDS,
            links: 1,
            targets: 1,
        };
        assert!(good.validate().is_ok());
        assert!(
            BacklinkIndex {
                format: BACKLINK_FORMAT + 1,
                ..good.clone()
            }
            .validate()
            .unwrap_err()
            .contains("format")
        );
        assert!(
            BacklinkIndex { shards: 0, ..good }
                .validate()
                .unwrap_err()
                .contains("shards")
        );
    }
}
