//! Reading of the internally tagged enums ([`Block`], [`Inline`], [`TypedFields`]) without a
//! buffer.
//!
//! The derived `Deserialize` of an enum with `#[serde(tag = "...")]` accepts the tag at any
//! position. To do that it first reads the whole value into serde's private `Content` tree and
//! then reads the variant from the tree. Every type inside such an enum is then compiled for two
//! deserializers, the JSON one and the tree one, and that was 79 KB of the WASM
//! (`docs/decisions/2026-11-22-tagged-enums-read-tag-first.md`).
//!
//! The compiler writes these enums with serde, which always puts the tag first. So the reader
//! here requires the tag to be the first key, reads it, and gives the rest of the same map to the
//! variant. Nothing is buffered. The serialized form does not change; `Serialize` stays derived.
//!
//! Each struct variant has a private body struct here with the same fields and `serde`
//! attributes. Constructing the variant from the body fails to compile when the two differ in
//! fields; the tests in this module read every variant back from what `Serialize` wrote.

use crate::{
    ArchetypeFields, Block, ClassFields, ClassOptionFields, CreatureFields, EntryId, FeatFields,
    Inline, ItemFields, ParagraphStyle, RacialTraitFields, SpellFields, StatRow, Table,
    TraitFields, TypedFields,
};
use serde::de::value::MapAccessDeserializer;
use serde::de::{DeserializeSeed, Error, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

/// An enum that is a JSON object whose first key is the tag.
trait TagFirst<'de>: Sized {
    /// The key that holds the variant name.
    const TAG: &'static str;
    /// What the enum is, for an error message.
    const EXPECTING: &'static str;
    /// The variant names.
    type Tag: Deserialize<'de>;
    /// Reads the variant `tag` from the keys after the tag.
    fn body<A: MapAccess<'de>>(tag: Self::Tag, body: A) -> Result<Self, A::Error>;
}

fn read<'de, T: TagFirst<'de>, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
    deserializer.deserialize_map(TagFirstVisitor(PhantomData))
}

struct TagFirstVisitor<T>(PhantomData<T>);

impl<'de, T: TagFirst<'de>> Visitor<'de> for TagFirstVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(T::EXPECTING)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<T, A::Error> {
        match map.next_key_seed(IsKey(T::TAG))? {
            Some(true) => {}
            Some(false) => return Err(tag_not_first(T::TAG)),
            None => return Err(A::Error::missing_field(T::TAG)),
        }
        let tag = map.next_value()?;
        T::body(tag, map)
    }
}

#[cold]
fn tag_not_first<E: Error>(tag: &'static str) -> E {
    E::custom(format_args!("the key `{tag}` must come first"))
}

/// Reads a key and says whether it is the expected one.
struct IsKey(&'static str);

impl<'de> DeserializeSeed<'de> for IsKey {
    type Value = bool;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
        deserializer.deserialize_identifier(self)
    }
}

impl Visitor<'_> for IsKey {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a key")
    }

    fn visit_str<E: Error>(self, key: &str) -> Result<bool, E> {
        Ok(key == self.0)
    }
}

/// Reads a variant's fields from the keys that follow the tag. A struct ignores a key it does
/// not know, as the derived reader did.
fn rest<'de, T: Deserialize<'de>, A: MapAccess<'de>>(body: A) -> Result<T, A::Error> {
    T::deserialize(MapAccessDeserializer::new(body))
}

/// A variant without fields: any other key is ignored, as the derived reader did.
fn no_fields<'de, A: MapAccess<'de>>(mut body: A) -> Result<(), A::Error> {
    while body.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum BlockTag {
    Heading,
    Paragraph,
    List,
    Table,
    Quote,
    StatBlock,
    EntryLink,
}

#[derive(Deserialize)]
struct HeadingBody {
    level: u8,
    #[serde(default)]
    anchor: Option<String>,
    content: Vec<Inline>,
}

#[derive(Deserialize)]
struct ParagraphBody {
    #[serde(default)]
    style: ParagraphStyle,
    content: Vec<Inline>,
}

#[derive(Deserialize)]
struct ListBody {
    ordered: bool,
    items: Vec<Vec<Block>>,
}

#[derive(Deserialize)]
struct QuoteBody {
    content: Vec<Block>,
}

#[derive(Deserialize)]
struct StatBlockBody {
    #[serde(default)]
    title: Option<String>,
    rows: Vec<StatRow>,
}

#[derive(Deserialize)]
struct EntryLinkBody {
    target: EntryId,
    title: String,
}

impl<'de> TagFirst<'de> for Block {
    const TAG: &'static str = "t";
    const EXPECTING: &'static str = "a block";
    type Tag = BlockTag;

    fn body<A: MapAccess<'de>>(tag: BlockTag, body: A) -> Result<Self, A::Error> {
        Ok(match tag {
            BlockTag::Heading => {
                let HeadingBody {
                    level,
                    anchor,
                    content,
                } = rest(body)?;
                Block::Heading {
                    level,
                    anchor,
                    content,
                }
            }
            BlockTag::Paragraph => {
                let ParagraphBody { style, content } = rest(body)?;
                Block::Paragraph { style, content }
            }
            BlockTag::List => {
                let ListBody { ordered, items } = rest(body)?;
                Block::List { ordered, items }
            }
            BlockTag::Table => Block::Table(rest::<Table, A>(body)?),
            BlockTag::Quote => {
                let QuoteBody { content } = rest(body)?;
                Block::Quote { content }
            }
            BlockTag::StatBlock => {
                let StatBlockBody { title, rows } = rest(body)?;
                Block::StatBlock { title, rows }
            }
            BlockTag::EntryLink => {
                let EntryLinkBody { target, title } = rest(body)?;
                Block::EntryLink { target, title }
            }
        })
    }
}

impl<'de> Deserialize<'de> for Block {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        read(deserializer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum InlineTag {
    Text,
    Em,
    Strong,
    Sup,
    Sub,
    Break,
    Link,
}

#[derive(Deserialize)]
struct TextBody {
    v: String,
}

/// The body of `Em`, `Strong`, `Sup` and `Sub`.
#[derive(Deserialize)]
struct SpanBody {
    c: Vec<Inline>,
}

#[derive(Deserialize)]
struct LinkBody {
    target: EntryId,
    #[serde(default)]
    anchor: Option<String>,
    c: Vec<Inline>,
}

impl<'de> TagFirst<'de> for Inline {
    const TAG: &'static str = "t";
    const EXPECTING: &'static str = "an inline";
    type Tag = InlineTag;

    fn body<A: MapAccess<'de>>(tag: InlineTag, body: A) -> Result<Self, A::Error> {
        Ok(match tag {
            InlineTag::Text => {
                let TextBody { v } = rest(body)?;
                Inline::Text { v }
            }
            InlineTag::Em => Inline::Em {
                c: rest::<SpanBody, A>(body)?.c,
            },
            InlineTag::Strong => Inline::Strong {
                c: rest::<SpanBody, A>(body)?.c,
            },
            InlineTag::Sup => Inline::Sup {
                c: rest::<SpanBody, A>(body)?.c,
            },
            InlineTag::Sub => Inline::Sub {
                c: rest::<SpanBody, A>(body)?.c,
            },
            InlineTag::Break => {
                no_fields(body)?;
                Inline::Break
            }
            InlineTag::Link => {
                let LinkBody { target, anchor, c } = rest(body)?;
                Inline::Link { target, anchor, c }
            }
        })
    }
}

impl<'de> Deserialize<'de> for Inline {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        read(deserializer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum FieldsTag {
    Spell,
    Feat,
    Creature,
    Item,
    Class,
    Trait,
    RacialTrait,
    Archetype,
    ClassOption,
    None,
}

impl<'de> TagFirst<'de> for TypedFields {
    const TAG: &'static str = "kind";
    const EXPECTING: &'static str = "typed fields";
    type Tag = FieldsTag;

    fn body<A: MapAccess<'de>>(tag: FieldsTag, body: A) -> Result<Self, A::Error> {
        Ok(match tag {
            FieldsTag::Spell => TypedFields::Spell(rest::<SpellFields, A>(body)?),
            FieldsTag::Feat => TypedFields::Feat(rest::<FeatFields, A>(body)?),
            FieldsTag::Creature => TypedFields::Creature(rest::<Box<CreatureFields>, A>(body)?),
            FieldsTag::Item => TypedFields::Item(rest::<ItemFields, A>(body)?),
            FieldsTag::Class => TypedFields::Class(rest::<ClassFields, A>(body)?),
            FieldsTag::Trait => TypedFields::Trait(rest::<TraitFields, A>(body)?),
            FieldsTag::RacialTrait => TypedFields::RacialTrait(rest::<RacialTraitFields, A>(body)?),
            FieldsTag::Archetype => TypedFields::Archetype(rest::<ArchetypeFields, A>(body)?),
            FieldsTag::ClassOption => TypedFields::ClassOption(rest::<ClassOptionFields, A>(body)?),
            FieldsTag::None => {
                no_fields(body)?;
                TypedFields::None
            }
        })
    }
}

impl<'de> Deserialize<'de> for TypedFields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        read(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TableCell, TableRow};

    fn id(text: &str) -> EntryId {
        EntryId::try_from(text.to_string()).unwrap()
    }

    fn text(value: &str) -> Vec<Inline> {
        vec![Inline::text(value)]
    }

    /// One value per variant. The `match` fails to compile when a variant is added, so a new
    /// variant cannot miss this list.
    fn blocks() -> Vec<Block> {
        let all = vec![
            Block::Heading {
                level: 2,
                anchor: Some("a".into()),
                content: text("h"),
            },
            Block::Heading {
                level: 3,
                anchor: None,
                content: text("h"),
            },
            Block::Paragraph {
                style: ParagraphStyle::Normal,
                content: text("p"),
            },
            Block::Paragraph {
                style: ParagraphStyle::Indented,
                content: inlines(),
            },
            Block::List {
                ordered: true,
                items: vec![vec![Block::Quote { content: vec![] }]],
            },
            Block::Table(Table {
                caption: text("c"),
                head: vec![],
                body: vec![TableRow {
                    cells: vec![TableCell {
                        header: true,
                        colspan: 2,
                        rowspan: 1,
                        content: text("x"),
                    }],
                }],
                foot: vec![],
            }),
            Block::Table(Table::default()),
            Block::Quote {
                content: vec![Block::Paragraph {
                    style: ParagraphStyle::StatLine,
                    content: text("q"),
                }],
            },
            Block::StatBlock {
                title: Some("t".into()),
                rows: vec![StatRow {
                    label: "l".into(),
                    value: text("v"),
                }],
            },
            Block::StatBlock {
                title: None,
                rows: vec![],
            },
            Block::EntryLink {
                target: id("ref/spell/fireball"),
                title: "Fireball".into(),
            },
        ];
        for block in &all {
            match block {
                Block::Heading { .. }
                | Block::Paragraph { .. }
                | Block::List { .. }
                | Block::Table(_)
                | Block::Quote { .. }
                | Block::StatBlock { .. }
                | Block::EntryLink { .. } => {}
            }
        }
        all
    }

    fn inlines() -> Vec<Inline> {
        let all = vec![
            Inline::text("t"),
            Inline::Em { c: text("e") },
            Inline::Strong { c: text("s") },
            Inline::Sup { c: text("1") },
            Inline::Sub { c: vec![] },
            Inline::Break,
            Inline::Link {
                target: id("ref/feat/dodge"),
                anchor: Some("x".into()),
                c: text("Dodge"),
            },
            Inline::Link {
                target: id("ref/feat/dodge"),
                anchor: None,
                c: vec![Inline::Em { c: text("Dodge") }],
            },
        ];
        for inline in &all {
            match inline {
                Inline::Text { .. }
                | Inline::Em { .. }
                | Inline::Strong { .. }
                | Inline::Sup { .. }
                | Inline::Sub { .. }
                | Inline::Break
                | Inline::Link { .. } => {}
            }
        }
        all
    }

    fn fields() -> Vec<TypedFields> {
        let all = vec![
            TypedFields::Spell(SpellFields {
                school: "evocation".into(),
                ..Default::default()
            }),
            TypedFields::Feat(FeatFields::default()),
            TypedFields::Creature(Box::default()),
            TypedFields::Item(ItemFields::default()),
            TypedFields::Class(ClassFields::default()),
            TypedFields::Trait(TraitFields::default()),
            TypedFields::RacialTrait(RacialTraitFields::default()),
            TypedFields::Archetype(ArchetypeFields::default()),
            TypedFields::ClassOption(ClassOptionFields::default()),
            TypedFields::None,
        ];
        for fields in &all {
            match fields {
                TypedFields::Spell(_)
                | TypedFields::Feat(_)
                | TypedFields::Creature(_)
                | TypedFields::Item(_)
                | TypedFields::Class(_)
                | TypedFields::Trait(_)
                | TypedFields::RacialTrait(_)
                | TypedFields::Archetype(_)
                | TypedFields::ClassOption(_)
                | TypedFields::None => {}
            }
        }
        all
    }

    #[test]
    fn every_variant_reads_back_what_serialize_wrote() {
        for block in blocks() {
            let json = serde_json::to_string(&block).unwrap();
            assert!(json.starts_with(r#"{"t":""#), "{json}");
            assert_eq!(
                serde_json::from_str::<Block>(&json).unwrap(),
                block,
                "{json}"
            );
        }
        for inline in inlines() {
            let json = serde_json::to_string(&inline).unwrap();
            assert!(json.starts_with(r#"{"t":""#), "{json}");
            assert_eq!(
                serde_json::from_str::<Inline>(&json).unwrap(),
                inline,
                "{json}"
            );
        }
        for fields in fields() {
            let json = serde_json::to_string(&fields).unwrap();
            assert!(json.starts_with(r#"{"kind":""#), "{json}");
            assert_eq!(
                serde_json::from_str::<TypedFields>(&json).unwrap(),
                fields,
                "{json}"
            );
        }
    }

    #[test]
    fn the_written_form_is_the_one_of_schema_6() {
        assert_eq!(
            serde_json::to_string(&Block::Heading {
                level: 2,
                anchor: None,
                content: vec![Inline::Em { c: text("x") }, Inline::Break],
            })
            .unwrap(),
            r#"{"t":"heading","level":2,"content":[{"t":"em","c":[{"t":"text","v":"x"}]},{"t":"break"}]}"#
        );
        assert_eq!(
            serde_json::to_string(&Block::StatBlock {
                title: None,
                rows: vec![],
            })
            .unwrap(),
            r#"{"t":"stat_block","rows":[]}"#
        );
        assert_eq!(
            serde_json::to_string(&Block::Table(Table::default())).unwrap(),
            r#"{"t":"table","body":[]}"#
        );
        assert_eq!(
            serde_json::to_string(&TypedFields::None).unwrap(),
            r#"{"kind":"none"}"#
        );
    }

    #[test]
    fn a_key_that_was_left_out_takes_its_default() {
        assert_eq!(
            serde_json::from_str::<Block>(r#"{"t":"paragraph","content":[]}"#).unwrap(),
            Block::Paragraph {
                style: ParagraphStyle::Normal,
                content: vec![],
            }
        );
        assert_eq!(
            serde_json::from_str::<Inline>(r#"{"t":"link","target":"ref/feat/dodge","c":[]}"#)
                .unwrap(),
            Inline::Link {
                target: id("ref/feat/dodge"),
                anchor: None,
                c: vec![],
            }
        );
    }

    #[test]
    fn an_unknown_key_is_ignored() {
        assert_eq!(
            serde_json::from_str::<Inline>(r#"{"t":"text","later":[1,{"a":2}],"v":"x"}"#).unwrap(),
            Inline::text("x")
        );
        assert_eq!(
            serde_json::from_str::<Inline>(r#"{"t":"break","later":1}"#).unwrap(),
            Inline::Break
        );
        assert_eq!(
            serde_json::from_str::<TypedFields>(r#"{"kind":"none","later":{"a":[]}}"#).unwrap(),
            TypedFields::None
        );
    }

    #[test]
    fn the_tag_must_be_the_first_key() {
        let late = serde_json::from_str::<Inline>(r#"{"v":"x","t":"text"}"#).unwrap_err();
        assert!(
            late.to_string().contains("the key `t` must come first"),
            "{late}"
        );
        let late =
            serde_json::from_str::<TypedFields>(r#"{"school":"x","kind":"spell"}"#).unwrap_err();
        assert!(
            late.to_string().contains("the key `kind` must come first"),
            "{late}"
        );
        let none = serde_json::from_str::<Block>("{}").unwrap_err();
        assert!(none.to_string().contains("missing field `t`"), "{none}");
    }

    #[test]
    fn an_unknown_variant_and_a_missing_field_are_errors() {
        let unknown = serde_json::from_str::<Block>(r#"{"t":"video"}"#).unwrap_err();
        assert!(
            unknown.to_string().contains("unknown variant `video`"),
            "{unknown}"
        );
        let missing = serde_json::from_str::<Block>(r#"{"t":"heading","level":2}"#).unwrap_err();
        assert!(
            missing.to_string().contains("missing field `content`"),
            "{missing}"
        );
        assert!(serde_json::from_str::<Block>(r#"[1]"#).is_err());
    }
}
