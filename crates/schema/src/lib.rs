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
//! <content_version>/aliases.json   AliasIndex (optional, after the canonical cutover)
//! ```
//!
//! Schema 3 adds the relation, spell-list, and book-order files. They are listed in the
//! manifest and hashed like every other file. Libraries of schema 1 and 2 do not have them;
//! a reader that accepts those versions must treat the files as optional. `aliases.json`
//! is a separate optional file with its own format version, so its presence does not change
//! the schema version.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub mod validate;

/// Version of the serialized content format.
///
/// 3: new kinds `trait`, `racial_trait` and `archetype`, `RuleEntry::relations`,
/// `FeatFields::prerequisite_feats`, the typed fields of the new kinds, and the files
/// `relations.json`, `lists.json` and `books.json`.
pub const SCHEMA_VERSION: u32 = 3;

/// Name of the relation index, relative to the version directory.
pub const RELATIONS_FILE: &str = "relations.json";
/// Name of the spell-list index, relative to the version directory.
pub const SPELL_LISTS_FILE: &str = "lists.json";
/// Name of the book-order index, relative to the version directory.
pub const BOOKS_FILE: &str = "books.json";
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
}

impl EntryKind {
    pub const ALL: [EntryKind; 14] = [
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
    /// Typed edges from this entry to others (schema 3). The inverse direction is derived
    /// from [`RelationIndex`] and is not stored on the target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
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
}

impl RelationKind {
    pub const ALL: [RelationKind; 3] = [
        RelationKind::Part,
        RelationKind::Archetype,
        RelationKind::Prerequisite,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            RelationKind::Part => "part",
            RelationKind::Archetype => "archetype",
            RelationKind::Prerequisite => "prerequisite",
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
    /// Rule, race, skill, affliction, trap, and archetype entries for now.
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

/// `relations.json` (schema 3): every `Relation` of every entry, flattened so that the
/// inverse direction is a lookup over this one small file instead of a scan of all shards.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RelationIndex {
    /// Edge count per [`RelationKind::slug`].
    pub counts: BTreeMap<String, u32>,
    /// Sorted by source, kind, then target. No duplicates.
    pub edges: Vec<RelationEdge>,
}

/// `[source, kind, target]` in JSON.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationEdge(pub EntryId, pub RelationKind, pub EntryId);

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
/// empty map. `entries` and `retired` are sorted by `old`, and the format forbids chains:
/// a canonical target never appears as an alias `old`.
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
        Ok(())
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
    /// Relations, spell lists, and book order (schema 3). Absent in older reports.
    #[serde(default)]
    pub relations: RelationCoverage,
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
        let edge = RelationEdge(
            EntryId::parse("a").unwrap(),
            RelationKind::Part,
            EntryId::parse("b").unwrap(),
        );
        assert_eq!(serde_json::to_string(&edge).unwrap(), r#"["a","part","b"]"#);
    }

    #[test]
    fn new_kinds_have_slugs_and_labels() {
        for (kind, slug) in [
            (EntryKind::Trait, "trait"),
            (EntryKind::RacialTrait, "racial_trait"),
            (EntryKind::Archetype, "archetype"),
        ] {
            assert_eq!(kind.slug(), slug);
            assert_eq!(EntryKind::from_slug(slug), Some(kind));
            assert!(!kind.label().is_empty());
        }
        assert_eq!(EntryKind::ALL.len(), 14);
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
