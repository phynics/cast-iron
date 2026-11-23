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
//! <content_version>/replaces.json  ReplacesIndex (schema 4 and 5; schema 6 replaces it)
//! <content_version>/trades.json    TradeIndex (schema 6)
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
pub mod manifest_check;
pub mod power;
pub mod stat_sets;
pub use stat_sets::StatSet;
mod tagged;
pub mod validate;

/// Version of the serialized content format.
///
/// 4: attached entries. `RelationKind::{Variant, Addon, Revision}` with an optional label on
/// a [`RelationEdge`], `EntryKind::ClassOption`, `ClassOptionFields` and `ArchetypeFields`
/// (each carrying `replaces` and `alters`), and the derived files `option-groups.json` and
/// `replaces.json`.
///
/// 5: book membership. [`EntryStub::books`] holds the books an entry belongs to, derived from
/// its `source_ids`; an entry is visible while any of them is active, and book placement in
/// `books.json` is presentation only (#394). A schema-4 catalog has no `books`, and the reader
/// then takes [`EntryStub::source`] as the one member book.
///
/// 6: typed trades (#450). `ArchetypeFields` and `ClassOptionFields` hold [`Trade`]s in place of
/// the `replaces`, `alters`, `replaces_features` and `alters_features` lists, and the compiler
/// writes no trades into entry shards: the derived `trades.json` ([`TradeIndex`]) carries them
/// and replaces `replaces.json`. A schema-4 or schema-5 library has `replaces.json` and no
/// `trades.json`; the reader then shows printed names only and offers no compatibility filter
/// (see [`TradeIndex::from_replaces`]).
///
/// [`RuleEntry::sources`] and [`RuleEntry::aon`] are additive and optional: a library without
/// them omits the key, and an older reader ignores an unknown key, so they do not change this
/// version.
pub const SCHEMA_VERSION: u32 = 6;

/// Name of the relation index, relative to the version directory.
pub const RELATIONS_FILE: &str = "relations.json";
/// Name of the class-option group index, relative to the version directory (schema 4).
pub const OPTION_GROUPS_FILE: &str = "option-groups.json";
/// Name of the printed replacement index of a schema-4 or schema-5 library, relative to the
/// version directory. A schema-6 library has [`TRADES_FILE`] instead.
pub const REPLACES_FILE: &str = "replaces.json";
/// Name of the typed trade index, relative to the version directory (schema 6).
pub const TRADES_FILE: &str = "trades.json";
/// Name of the spell-list index, relative to the version directory.
pub const SPELL_LISTS_FILE: &str = "lists.json";
/// Name of the book-order index, relative to the version directory.
pub const BOOKS_FILE: &str = "books.json";
/// Name of the table of old section addresses, relative to the version directory. Optional: a
/// library built before the table has no such file. See [`LegacySections`].
pub const LEGACY_SECTIONS_FILE: &str = "legacy-sections.json";
/// Version of the legacy section table. Independent of [`SCHEMA_VERSION`].
pub const LEGACY_SECTIONS_FORMAT: u32 = 1;
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
    /// Books that expand this entry without printing it (issue #458). Credit only: an entry is
    /// never a member of these books, and membership, counts, filters, search sources and page
    /// order never read the list. Optional and additive: an entry without rows omits the key,
    /// and an older reader ignores it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expanded_in: Vec<Expansion>,
    /// The stat sets of a creature that prints several (`##` headings with a `Challenge
    /// rating` row), derived by the compiler from the body and never written to the corpus.
    /// Empty for an entry with fewer than two. Optional and additive: an entry without sets
    /// omits the key, and an older reader ignores it, so `SCHEMA_VERSION` stays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stat_sets: Vec<StatSet>,
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

    /// Whether a relation of this kind may carry `label` (`None` is no label). The table is
    /// closed: an `addon` must carry exactly one of its three labels, and `variant` and
    /// `archetype` carry none.
    pub fn allows(self, label: Option<RelationLabel>) -> bool {
        use RelationLabel as L;
        match self {
            RelationKind::Prerequisite => matches!(label, None | Some(L::Or)),
            RelationKind::Part => matches!(label, None | Some(L::Alternate)),
            RelationKind::Addon => matches!(label, Some(L::Mythic | L::Stamina | L::Changes)),
            RelationKind::Revision => matches!(label, None | Some(L::Removed | L::Merged)),
            RelationKind::Variant | RelationKind::Archetype => label.is_none(),
        }
    }
}

/// The qualifier of a [`Relation`]. The set is closed: serde refuses any other string, and
/// [`RelationKind::allows`] says which kind may carry which label. The wire form is the
/// lowercase word, the same string the schema used before the type existed.
///
/// The variants stay in alphabetical order of their wire strings. `RelationEdge` sorts by its
/// derived `Ord`, and the index order must not change from when the label was a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationLabel {
    /// An alternate racial trait ([`RelationKind::Part`]).
    Alternate,
    /// A third-party change to the target ([`RelationKind::Addon`]).
    Changes,
    /// A revision that merges the target into another entry ([`RelationKind::Revision`]).
    Merged,
    /// A mythic version of the target ([`RelationKind::Addon`]).
    Mythic,
    /// One alternative of an either-or requirement ([`RelationKind::Prerequisite`]).
    Or,
    /// A revision that removes the target ([`RelationKind::Revision`]).
    Removed,
    /// Stamina rules for the target ([`RelationKind::Addon`]).
    Stamina,
}

impl RelationLabel {
    /// Every label, in the alphabetical order of its wire string.
    pub const ALL: [RelationLabel; 7] = [
        RelationLabel::Alternate,
        RelationLabel::Changes,
        RelationLabel::Merged,
        RelationLabel::Mythic,
        RelationLabel::Or,
        RelationLabel::Removed,
        RelationLabel::Stamina,
    ];

    /// The wire string.
    pub fn slug(self) -> &'static str {
        match self {
            RelationLabel::Alternate => "alternate",
            RelationLabel::Changes => "changes",
            RelationLabel::Merged => "merged",
            RelationLabel::Mythic => "mythic",
            RelationLabel::Or => "or",
            RelationLabel::Removed => "removed",
            RelationLabel::Stamina => "stamina",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|label| label.slug() == value)
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
    pub label: Option<RelationLabel>,
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

/// A book that expands an entry without being a member book of it (issue #458).
///
/// The expanding material has no entry of its own, so no relation can credit it. The book is
/// not in [`RuleEntry::source_ids`]; this record is a credit and never a membership.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Expansion {
    /// Publication id of the expanding book. It must exist and must not be in
    /// [`RuleEntry::source_ids`].
    pub publication: String,
    /// Printed page, as in [`Citation::page`]. Omitted when the credit has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// The entry whose own text the book expands, when the expansion is shown through another
    /// entry. It must resolve, and its `source_ids` must include [`Self::publication`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<EntryId>,
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
///
/// `Deserialize` is written by hand in `tagged.rs`: the tag `t` must be the first key.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
///
/// `Deserialize` is written by hand in `tagged.rs`: the tag `t` must be the first key.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
///
/// `Deserialize` is written by hand in `tagged.rs`: the tag `kind` must be the first key.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
            // Digits only: `u32::parse` accepts a leading `+`, and a template's `CR +1` is an
            // adjustment, not a rating.
            whole if whole.bytes().all(|b| b.is_ascii_digit()) => {
                whole.parse::<u32>().ok()?.checked_mul(24)?
            }
            _ => return None,
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

/// What an archetype or class option does to one feature of its base class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeOp {
    /// The feature is gone: another feature takes its place.
    Replaces,
    /// The feature stays, in a changed form.
    Alters,
}

impl TradeOp {
    /// The corpus key and the wire name: `replaces` or `alters`.
    pub const fn slug(self) -> &'static str {
        match self {
            TradeOp::Replaces => "replaces",
            TradeOp::Alters => "alters",
        }
    }

    /// The label of the printed line: `Replaces` or `Alters`.
    pub const fn label(self) -> &'static str {
        match self {
            TradeOp::Replaces => "Replaces",
            TradeOp::Alters => "Alters",
        }
    }
}

/// The feature of a base class that a [`Trade`] reaches.
///
/// `anchor` is a heading anchor of a base class entry. `class` names that entry, and the
/// compiler fills it only when the anchor exists in more than one base class of the trading
/// entry (an archetype of several classes, or a class option whose group has several classes),
/// so a one-class feature keeps no `class`. `levels` and `power` narrow the feature: the levels
/// at which the trade applies, and a named power inside a feature (a bloodline power, a domain
/// power) rather than the whole feature.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct FeatureRef {
    pub anchor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<EntryId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub levels: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<String>,
}

/// One printed trade of an archetype or class option: it replaces or alters a feature.
///
/// `printed` is the name as the book prints it, and it is always kept. `feature` is the
/// resolved reference. `None` is legal and means the trade is not resolved: a filter must never
/// treat an unresolved trade as compatible with anything.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Trade {
    pub op: TradeOp,
    pub printed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<FeatureRef>,
}

impl Trade {
    /// An unresolved trade: a printed name and nothing else.
    pub fn unresolved(op: TradeOp, printed: impl Into<String>) -> Self {
        Self {
            op,
            printed: printed.into(),
            feature: None,
        }
    }
}

/// The printed names of the trades with `op`, in order.
pub fn printed_trades(trades: &[Trade], op: TradeOp) -> Vec<String> {
    trades
        .iter()
        .filter(|trade| trade.op == op)
        .map(|trade| trade.printed.clone())
        .collect()
}

/// The heading anchors that a corpus-format-1 `replaces_features` or `alters_features` list
/// held when it did not line up one to one with the printed list. The format-1 reader keeps
/// them so that `fmt` writes the file back unchanged; the lowering to [`Trade`]s cannot say
/// which printed name each anchor belongs to. They are never serialized, and the compiler drops
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LegacyFeatureLists {
    pub replaces: Vec<String>,
    pub alters: Vec<String>,
}

impl LegacyFeatureLists {
    pub fn is_empty(&self) -> bool {
        self.replaces.is_empty() && self.alters.is_empty()
    }
}

/// Fields of an archetype: the printed class features it replaces and alters, in print order.
/// An empty `trades` is omitted from the serialized form.
///
/// The corpus holds the trades and the compiler lowers them (#450). The compiler writes none of
/// them into an entry shard: `trades.json` carries them, and a client reads it for the Replaces
/// line and for the compatibility filter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ArchetypeFields {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trades: Vec<Trade>,
    /// Corpus format 1 only; see [`LegacyFeatureLists`].
    #[serde(skip)]
    pub legacy_features: LegacyFeatureLists,
}

/// Fields of a class option: the option group it belongs to and the printed features it
/// replaces and alters, in print order. `trades` follows the rules of [`ArchetypeFields`]; a
/// feature of a class option reaches the classes of its group.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClassOptionFields {
    /// A slug from the corpus `option-groups.toml`.
    pub group: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trades: Vec<Trade>,
    /// Corpus format 1 only; see [`LegacyFeatureLists`].
    #[serde(skip)]
    pub legacy_features: LegacyFeatureLists,
}

impl TypedFields {
    /// The trades of an archetype or class option; empty for every other kind.
    pub fn trades(&self) -> &[Trade] {
        match self {
            TypedFields::Archetype(fields) => &fields.trades,
            TypedFields::ClassOption(fields) => &fields.trades,
            _ => &[],
        }
    }

    /// The trade list of an archetype or class option, for the compiler to lower or clear.
    pub fn trades_mut(&mut self) -> Option<&mut Vec<Trade>> {
        match self {
            TypedFields::Archetype(fields) => Some(&mut fields.trades),
            TypedFields::ClassOption(fields) => Some(&mut fields.trades),
            _ => None,
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")] pub Option<RelationLabel>,
);

impl RelationEdge {
    /// An edge with no label, in the compact three-element form.
    pub fn plain(source: EntryId, kind: RelationKind, target: EntryId) -> Self {
        Self(source, kind, target, None)
    }

    pub fn label(&self) -> Option<RelationLabel> {
        self.3
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

/// `replaces.json` (schema 4 and 5): the printed items that an archetype or class option
/// replaces. Sorted by id. A schema-6 library has [`TradeIndex`] instead; the reader keeps this
/// type to open an older library.
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

/// `trades.json` (schema 6): the trades of every archetype and class option that has any, one
/// row per entry, sorted by id. An entry without trades has no row. Derived from the corpus,
/// and the only place the compiler writes trades (the entry shards hold none).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TradeIndex {
    pub entries: Vec<TradeRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeRow {
    pub id: EntryId,
    /// In print order: the replaced features, then the altered ones.
    pub trades: Vec<Trade>,
}

impl TradeIndex {
    /// The row of an entry. The rows are sorted by id.
    pub fn row(&self, id: &EntryId) -> Option<&TradeRow> {
        self.entries
            .binary_search_by(|row| row.id.cmp(id))
            .ok()
            .map(|index| &self.entries[index])
    }

    /// Reads the `replaces.json` of a schema-4 or schema-5 library as trades: every item is a
    /// `Replaces` trade with a printed name and no feature, because that file holds neither
    /// anchors nor `alters`. The client then shows the names and offers no filter.
    pub fn from_replaces(index: ReplacesIndex) -> Self {
        let mut entries: Vec<TradeRow> = index
            .entries
            .into_iter()
            .filter(|row| !row.replaces.is_empty())
            .map(|row| TradeRow {
                id: row.id,
                trades: row
                    .replaces
                    .into_iter()
                    .map(|printed| Trade::unresolved(TradeOp::Replaces, printed))
                    .collect(),
            })
            .collect();
        cast_iron_sort::sort_by(&mut entries, |a, b| a.id.cmp(&b.id));
        Self { entries }
    }
}

impl TradeRow {
    /// The printed names of the trades with `op`, in order.
    pub fn printed(&self, op: TradeOp) -> Vec<&str> {
        self.trades
            .iter()
            .filter(|trade| trade.op == op)
            .map(|trade| trade.printed.as_str())
            .collect()
    }
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

/// `legacy-sections.json`: what each positional section address that a published library made
/// stands for now. A book-section anchor used to be the row's position (`toc-2-26`), so a pin
/// that inserted a row moved every later address to another section. A chapter page now names a
/// row by its identity (`books::section_anchors`), and this table keeps the old addresses
/// working: it maps the positional anchor of each row of the last published tree to the stable
/// anchor of the same row. A row that has left the book has no stable anchor in the current tree,
/// and its old address opens its chapter. Additive and optional; the table is frozen, because no
/// later library makes a positional anchor. `sources/legacy-sections.json` holds it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LegacySections {
    /// [`LEGACY_SECTIONS_FORMAT`].
    pub format: u32,
    /// By book source: old positional anchor to stable anchor.
    pub books: BTreeMap<String, BTreeMap<String, String>>,
    /// By book source: the old positional anchor of each top-level Contents row (`toc-3`) to the
    /// permanent id of its chapter page. Contents addresses (`?c=3.2`) name chapter rows too, and
    /// `books` holds only the rows below them. Optional: a table made before Contents addresses
    /// followed the row has none, and its chapter addresses open the top of Contents.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub chapters: BTreeMap<String, BTreeMap<String, String>>,
}

/// The rows of [`LegacySections`] that belong to one book.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LegacyBook {
    /// Old positional anchor of a row below chapter level to its stable anchor.
    pub sections: BTreeMap<String, String>,
    /// Old positional anchor of a chapter row to its chapter id.
    pub chapters: BTreeMap<String, String>,
}

impl LegacySections {
    /// The rows of `source`. A book the table does not mention has none.
    pub fn book(&self, source: &str) -> LegacyBook {
        LegacyBook {
            sections: self.books.get(source).cloned().unwrap_or_default(),
            chapters: self.chapters.get(source).cloned().unwrap_or_default(),
        }
    }
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
        for stub in &self.entries {
            if stub.books.is_empty() {
                continue;
            }
            let distinct = stub
                .books
                .iter()
                .enumerate()
                .all(|(i, book)| !stub.books[..i].contains(book));
            if stub.books.len() < 2 || stub.books[0] != stub.source || !distinct {
                return Err(format!(
                    "catalog.json entry {:?} has malformed member books {:?}",
                    stub.id, stub.books
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

/// The most book slots a share code can describe (#408). `sources/published-books.txt` may not
/// list more books than this.
pub const MAX_SHARE_SLOTS: u32 = 4096;

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
    /// The book's permanent slot for share codes (#408): its line number in
    /// `sources/published-books.txt`, counted from 0. A code names slots, not ids and not catalog
    /// order, so it keeps its meaning when books are added or reordered. The compiler assigns it
    /// from the append-only file and fails when a book has no line, so a library from this
    /// compiler has one on every source.
    ///
    /// Optional and additive, like [`Self::category`]: an older catalog has no field and an
    /// older reader ignores it, so `SCHEMA_VERSION` stays. A reader that finds a source
    /// without a slot cannot share a set and hides the control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_slot: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryStub {
    pub id: EntryId,
    pub kind: EntryKind,
    pub title: String,
    /// The primary publication id.
    pub source: String,
    /// The books the entry belongs to: its distinct `source_ids`, in order, so the first is
    /// [`Self::source`]. Omitted when the entry belongs to `source` alone, and absent from a
    /// schema-4 catalog; read it through [`Self::member_books`]. Compute it with
    /// [`Self::encode_books`] so every writer agrees.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub books: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<EntryId>,
    /// Shard path relative to the version directory, such as `entries/spell-000.json`.
    pub shard: String,
}

impl EntryStub {
    /// The stored form of [`Self::books`] for an entry whose publications are `source_ids`:
    /// the distinct ids in order, or empty when there is only one.
    pub fn encode_books(source_ids: &[String]) -> Vec<String> {
        let mut books: Vec<String> = Vec::with_capacity(source_ids.len());
        for id in source_ids {
            if !books.contains(id) {
                books.push(id.clone());
            }
        }
        if books.len() < 2 { Vec::new() } else { books }
    }

    /// Every book the entry belongs to, the primary one first. Never empty.
    pub fn member_books(&self) -> &[String] {
        if self.books.is_empty() {
            std::slice::from_ref(&self.source)
        } else {
            &self.books
        }
    }
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
    /// Per-publication licensing bases that differ from the legacy OGL default.
    ///
    /// Optional and additive. Older readers ignore these records and still display the
    /// Community Use notice and global license text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub publication_licenses: Vec<PublicationLicense>,
    pub datasets: Vec<DatasetNotice>,
}

/// A source's reviewed legal basis, named here so the Legal page can identify it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationLicense {
    /// Publication id from the source catalogue.
    pub publication_id: String,
    /// Exact source title, checked against the catalogue when compiling the corpus.
    pub title: String,
    /// License ids from the legal notice table, including the special
    /// `legal#community-use` reference when applicable.
    pub license_refs: Vec<String>,
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
    /// Earlier `excluded` or `rejected` rows that a later `resolved` row superseded, because
    /// the record now ships. Absent when there are none, so a report without any is unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved: Vec<ResolvedExclusion>,
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

/// An `excluded` or `rejected` row that a `resolved` row superseded: the record at `path` now
/// ships as `entries`. The report lists it here instead of under `excluded` or `rejected`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedExclusion {
    pub path: String,
    /// The kind of the superseded row: `excluded` or `rejected`.
    pub was: String,
    /// Entry ids that now hold the record.
    pub entries: Vec<String>,
    /// Why the row no longer applies, as the `resolved` row states it.
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
    /// `publication_licenses` is additive (AGENTS rule 4 needs no `SCHEMA_VERSION` change): a
    /// library without it serializes without the key, and an older library reads as empty.
    #[test]
    fn legal_notices_without_publication_licenses_are_unchanged() {
        let old = r#"{"product_identity":"p","community_use_notice":"c","licenses":[],"section_15":[],"datasets":[]}"#;
        let legal: LegalNotices = serde_json::from_str(old).unwrap();
        assert!(legal.publication_licenses.is_empty());
        let same = |text: &str, legal: &LegalNotices| {
            let expected: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(serde_json::to_value(legal).unwrap(), expected);
        };
        same(old, &legal);

        let newer = r#"{"product_identity":"p","community_use_notice":"c","licenses":[],"section_15":[],"publication_licenses":[{"publication_id":"a","title":"A","license_refs":["legal#community-use"]}],"datasets":[]}"#;
        let legal: LegalNotices = serde_json::from_str(newer).unwrap();
        same(newer, &legal);
    }

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
            books: vec![],
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
    fn member_books_are_the_distinct_source_ids_and_a_lone_source_is_omitted() {
        let ids = |list: &[&str]| list.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        assert!(EntryStub::encode_books(&ids(&[])).is_empty());
        assert!(EntryStub::encode_books(&ids(&["core"])).is_empty());
        // A repeated source is one book; it does not make a second member.
        assert!(EntryStub::encode_books(&ids(&["core", "core"])).is_empty());
        assert_eq!(
            EntryStub::encode_books(&ids(&["core", "um", "core", "uc"])),
            ids(&["core", "um", "uc"])
        );

        let mut stub = EntryStub {
            id: EntryId::parse("ref/rule/a").unwrap(),
            kind: EntryKind::Rule,
            title: "a".into(),
            source: "core".into(),
            books: vec![],
            parent: None,
            shard: "entries/rule-000.json".into(),
        };
        // A schema-4 stub has no `books`, and the primary source is the one member.
        assert_eq!(stub.member_books(), ["core".to_string()]);
        stub.books = ids(&["core", "um"]);
        assert_eq!(stub.member_books(), ids(&["core", "um"]));
    }

    #[test]
    fn catalog_rejects_malformed_member_books() {
        let stub = |books: &[&str]| EntryStub {
            id: EntryId::parse("ref/rule/a").unwrap(),
            kind: EntryKind::Rule,
            title: "a".into(),
            source: "core".into(),
            books: books.iter().map(|id| id.to_string()).collect(),
            parent: None,
            shard: "entries/rule-000.json".into(),
        };
        let catalog = |books: &[&str]| Catalog {
            sources: vec![],
            entries: vec![stub(books)],
        };
        assert!(catalog(&[]).validate().is_ok());
        assert!(catalog(&["core", "um"]).validate().is_ok());
        for bad in [&["core"][..], &["um", "core"], &["core", "um", "um"]] {
            let message = catalog(bad).validate().unwrap_err();
            assert!(message.contains("member books"), "{message}");
        }
    }

    #[test]
    fn challenge_rating_rejects_a_sign_and_empty_text() {
        assert_eq!(ChallengeRating::parse("+1"), None);
        assert_eq!(ChallengeRating::parse("-1"), None);
        assert_eq!(ChallengeRating::parse(""), None);
        assert_eq!(ChallengeRating::parse(" 5 ").unwrap().twenty_fourths, 120);
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
            label: Some(RelationLabel::Or),
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
            Some(RelationLabel::Mythic),
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
    fn labels_use_their_wire_strings_and_refuse_unknown_ones() {
        for label in RelationLabel::ALL {
            let json = format!("\"{}\"", label.slug());
            assert_eq!(serde_json::to_string(&label).unwrap(), json);
            assert_eq!(serde_json::from_str::<RelationLabel>(&json).unwrap(), label);
            assert_eq!(RelationLabel::from_slug(label.slug()), Some(label));
        }
        assert_eq!(RelationLabel::ALL.len(), 7);
        let mut by_slug = RelationLabel::ALL;
        by_slug.sort_by_key(|label| label.slug());
        assert_eq!(by_slug, RelationLabel::ALL, "ALL is alphabetical");
        let mut by_ord = RelationLabel::ALL;
        by_ord.sort();
        assert_eq!(
            by_ord,
            RelationLabel::ALL,
            "Ord matches the old string order"
        );
        assert_eq!(RelationLabel::from_slug("Or"), None);
        for bad in ["\"unknown\"", "\"\"", "\"Mythic\"", "null", "1"] {
            assert!(serde_json::from_str::<RelationLabel>(bad).is_err(), "{bad}");
        }
        assert!(
            serde_json::from_str::<Relation>(r#"{"kind":"part","target":"a","label":"bogus"}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<RelationEdge>(r#"["a","part","b","bogus"]"#).is_err());
    }

    #[test]
    fn every_kind_allows_exactly_its_closed_set_of_labels() {
        use RelationKind as K;
        use RelationLabel as L;
        let allowed = |kind: K| -> Vec<Option<L>> {
            let mut all = vec![None];
            all.extend(L::ALL.into_iter().map(Some));
            all.into_iter()
                .filter(|label| kind.allows(*label))
                .collect()
        };
        assert_eq!(allowed(K::Prerequisite), [None, Some(L::Or)]);
        assert_eq!(allowed(K::Part), [None, Some(L::Alternate)]);
        assert_eq!(
            allowed(K::Addon),
            [Some(L::Changes), Some(L::Mythic), Some(L::Stamina)]
        );
        assert_eq!(
            allowed(K::Revision),
            [None, Some(L::Merged), Some(L::Removed)]
        );
        assert_eq!(allowed(K::Variant), [None]);
        assert_eq!(allowed(K::Archetype), [None]);
        assert_eq!(K::ALL.len(), 6, "a new kind needs a row in this test");
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
    fn attached_fields_omit_empty_vectors_and_never_serialize_the_legacy_lists() {
        let archetype = TypedFields::Archetype(ArchetypeFields::default());
        assert_eq!(
            serde_json::to_string(&archetype).unwrap(),
            r#"{"kind":"archetype"}"#
        );
        let option = TypedFields::ClassOption(ClassOptionFields {
            group: "arcane-discovery".into(),
            ..Default::default()
        });
        assert_eq!(
            serde_json::to_string(&option).unwrap(),
            r#"{"kind":"class_option","group":"arcane-discovery"}"#
        );
        let option = TypedFields::ClassOption(ClassOptionFields {
            group: "arcane-discovery".into(),
            trades: vec![
                Trade {
                    op: TradeOp::Replaces,
                    printed: "Arcane bond".into(),
                    feature: Some(FeatureRef {
                        anchor: "arcane-bond".into(),
                        ..Default::default()
                    }),
                },
                Trade::unresolved(TradeOp::Alters, "Arcane school"),
            ],
            legacy_features: LegacyFeatureLists {
                replaces: vec!["kept-out".into()],
                alters: vec![],
            },
        });
        let json = serde_json::to_string(&option).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"class_option","group":"arcane-discovery","trades":[{"op":"replaces","printed":"Arcane bond","feature":{"anchor":"arcane-bond"}},{"op":"alters","printed":"Arcane school"}]}"#
        );
        let back: TypedFields = serde_json::from_str(&json).unwrap();
        assert_eq!(back.trades().len(), 2);
        assert!(match &back {
            TypedFields::ClassOption(fields) => fields.legacy_features.is_empty(),
            _ => false,
        });
        // A schema-5 shard still names the old lists; the reader ignores them.
        let old = r#"{"kind":"archetype","replaces":["Bravery"],"replaces_features":["bravery"]}"#;
        let old: TypedFields = serde_json::from_str(old).unwrap();
        assert!(old.trades().is_empty());
    }

    #[test]
    fn a_feature_reference_serializes_its_narrowing_keys_only_when_set() {
        let reference = FeatureRef {
            anchor: "bloodline-powers".into(),
            class: Some(EntryId::parse("ref/class/sorcerer").unwrap()),
            levels: vec![1, 9],
            power: Some("Acidic ray".into()),
        };
        let json = serde_json::to_string(&reference).unwrap();
        assert_eq!(
            json,
            r#"{"anchor":"bloodline-powers","class":"ref/class/sorcerer","levels":[1,9],"power":"Acidic ray"}"#
        );
        assert_eq!(
            serde_json::from_str::<FeatureRef>(&json).unwrap(),
            reference
        );
    }

    #[test]
    fn an_older_replaces_file_reads_as_unresolved_replaces_trades() {
        let id = |value: &str| EntryId::parse(value).unwrap();
        let legacy = ReplacesIndex {
            entries: vec![
                ReplacesRow {
                    id: id("ref/archetype/a"),
                    replaces: vec!["Bravery".into(), "Trap sense".into()],
                },
                ReplacesRow {
                    id: id("ref/archetype/b"),
                    replaces: vec![],
                },
            ],
        };
        let trades = TradeIndex::from_replaces(legacy);
        assert_eq!(trades.entries.len(), 1);
        let row = trades.row(&id("ref/archetype/a")).unwrap();
        assert_eq!(row.printed(TradeOp::Replaces), ["Bravery", "Trap sense"]);
        assert!(row.printed(TradeOp::Alters).is_empty());
        assert!(row.trades.iter().all(|trade| trade.feature.is_none()));
        assert!(trades.row(&id("ref/archetype/b")).is_none());
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
            share_slot: None,
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
