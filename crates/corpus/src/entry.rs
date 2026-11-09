//! Entry files: `+++` TOML front matter followed by the block body.

use crate::block::{parse_blocks, write_blocks};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::tomlutil;
use cast_iron_schema::{
    Block, ChallengeRating, Citation, CreatureFields, EntryId, EntryKind, EntryReference,
    FeatFields, Inline, ItemFields, Provenance, Relation, RelationKind, RuleEntry, SpellFields,
    SpellLevel, TypedFields,
};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The corpus path of an entry, derived from the permanent `ref/` id.
pub fn entry_path(id: &EntryId) -> PathBuf {
    let rest = id.as_str().strip_prefix("ref/").unwrap_or(id.as_str());
    Path::new("entries").join(rest).with_extension("md")
}

/// The links the body holds. `RuleEntry::links` is not stored; the reader calls
/// this and the compiler must keep it in sync with the body.
pub fn derived_links(body: &[Block]) -> Vec<EntryReference> {
    let mut links = BTreeSet::new();
    collect_block_links(body, &mut links);
    links.into_iter().collect()
}

fn collect_block_links(blocks: &[Block], links: &mut BTreeSet<EntryReference>) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
                collect_inline_links(content, links);
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_block_links(item, links);
                }
            }
            Block::Table(table) => {
                collect_inline_links(&table.caption, links);
                for row in table.head.iter().chain(&table.body).chain(&table.foot) {
                    for cell in &row.cells {
                        collect_inline_links(&cell.content, links);
                    }
                }
            }
            Block::Quote { content } => collect_block_links(content, links),
            Block::StatBlock { rows, .. } => {
                for row in rows {
                    collect_inline_links(&row.value, links);
                }
            }
            Block::EntryLink { target, .. } => {
                links.insert(EntryReference {
                    target: target.clone(),
                    anchor: None,
                });
            }
        }
    }
}

fn collect_inline_links(inlines: &[Inline], links: &mut BTreeSet<EntryReference>) {
    for inline in inlines {
        match inline {
            Inline::Link { target, anchor, c } => {
                links.insert(EntryReference {
                    target: target.clone(),
                    anchor: anchor.clone(),
                });
                collect_inline_links(c, links);
            }
            Inline::Em { c } | Inline::Strong { c } | Inline::Sup { c } | Inline::Sub { c } => {
                collect_inline_links(c, links);
            }
            Inline::Text { .. } | Inline::Break => {}
        }
    }
}

pub fn write_entry(entry: &RuleEntry, config: &Config) -> String {
    let mut out = String::new();
    out.push_str("+++\n");
    out.push_str("id = ");
    out.push_str(&tomlutil::id(&entry.id));
    out.push('\n');
    out.push_str("kind = ");
    out.push_str(&tomlutil::string(entry.kind.slug()));
    out.push('\n');
    out.push_str("title = ");
    out.push_str(&tomlutil::string(&entry.title));
    out.push('\n');
    if let Some(aon) = &entry.aon {
        out.push_str("aon = ");
        out.push_str(&tomlutil::string(aon));
        out.push('\n');
    }
    write_string_array(&mut out, "source_ids", &entry.source_ids);
    write_string_array(&mut out, "section", &entry.section);
    if let Some(parent) = &entry.parent {
        out.push_str("parent = ");
        out.push_str(&tomlutil::id(parent));
        out.push('\n');
    }
    write_string_array(&mut out, "tags", &entry.tags);
    if !entry.provenance.is_empty() {
        out.push_str("provenance = [\n");
        for provenance in &entry.provenance {
            out.push_str("  { dataset = ");
            out.push_str(&tomlutil::string(&provenance.dataset));
            out.push_str(", record = ");
            out.push_str(&tomlutil::string(&provenance.record));
            out.push_str(", path = ");
            out.push_str(&tomlutil::string(&provenance.path));
            out.push_str(" },\n");
        }
        out.push_str("]\n");
    }
    // The default is declared once in `corpus.toml`; only a difference is written,
    // and an empty list is a difference.
    if entry.license_refs != config.license_refs {
        out.push_str("license_refs = ");
        out.push_str(&tomlutil::string_array(&entry.license_refs));
        out.push('\n');
    }
    let stored = stored_relations(entry);
    if !stored.is_empty() {
        out.push_str("relations = [\n");
        for relation in stored {
            out.push_str("  ");
            out.push_str(&tomlutil::relation(relation));
            out.push_str(",\n");
        }
        out.push_str("]\n");
    }
    write_sources(&mut out, &entry.sources);
    out.push_str("\n[fields]\n");
    write_fields(&mut out, &entry.fields);
    out.push_str("+++\n");

    let body = write_blocks(&entry.body);
    if !body.is_empty() {
        out.push('\n');
        out.push_str(&body);
        out.push('\n');
    }
    out
}

/// The ids a feat names as prerequisites; empty for every other kind.
fn prerequisite_feats(fields: &TypedFields) -> &[EntryId] {
    match fields {
        TypedFields::Feat(fields) => &fields.prerequisite_feats,
        _ => &[],
    }
}

/// The relations the file stores. A prerequisite relation is derived from
/// `prerequisite_feats`, so only what that list cannot say is stored: a label
/// (`or`) and a prerequisite whose target the list does not hold.
fn stored_relations(entry: &RuleEntry) -> Vec<&Relation> {
    let feats = prerequisite_feats(&entry.fields);
    entry
        .relations
        .iter()
        .filter(|relation| {
            !(relation.kind == RelationKind::Prerequisite
                && relation.label.is_none()
                && feats.contains(&relation.target))
        })
        .collect()
}

/// The full relation list: one prerequisite per `prerequisite_feats` id in list
/// order (with the stored label, if any), then the stored rows.
fn merge_relations(fields: &TypedFields, stored: Vec<Relation>) -> Vec<Relation> {
    let feats = prerequisite_feats(fields);
    let mut relations: Vec<Relation> = feats
        .iter()
        .map(|feat| Relation {
            kind: RelationKind::Prerequisite,
            target: feat.clone(),
            label: stored
                .iter()
                .find(|row| row.kind == RelationKind::Prerequisite && &row.target == feat)
                .and_then(|row| row.label.clone()),
        })
        .collect();
    relations.extend(stored.into_iter().filter(|row| {
        !(row.kind == RelationKind::Prerequisite
            && row.label.is_some()
            && feats.contains(&row.target))
    }));
    relations
}

/// Writes the `[[sources]]` rows in list order. The rows are last in the header because a
/// TOML array of tables ends the top-level keys; `[fields]` follows.
fn write_sources(out: &mut String, sources: &[Citation]) {
    for source in sources {
        out.push_str("[[sources]]\npublication = ");
        out.push_str(&tomlutil::string(&source.publication));
        out.push('\n');
        if let Some(page) = &source.page {
            out.push_str("page = ");
            out.push_str(&tomlutil::string(page));
            out.push('\n');
        }
    }
}

fn write_string_array(out: &mut String, key: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(&tomlutil::string_array(values));
    out.push('\n');
}

fn write_fields(out: &mut String, fields: &TypedFields) {
    match fields {
        TypedFields::Spell(fields) => {
            out.push_str("kind = \"spell\"\n");
            out.push_str("school = ");
            out.push_str(&tomlutil::string(&fields.school));
            out.push('\n');
            write_string_array(out, "subschools", &fields.subschools);
            write_string_array(out, "descriptors", &fields.descriptors);
            if !fields.levels.is_empty() {
                out.push_str("levels = [\n");
                for level in &fields.levels {
                    out.push_str("  { class = ");
                    out.push_str(&tomlutil::string(&level.class));
                    out.push_str(", level = ");
                    out.push_str(&level.level.to_string());
                    out.push_str(" },\n");
                }
                out.push_str("]\n");
            }
            write_optional_string(out, "casting_time", &fields.casting_time);
            write_optional_string(out, "components", &fields.components);
            write_optional_string(out, "range", &fields.range);
            write_pairs(out, "effects", &fields.effects);
            write_optional_string(out, "duration", &fields.duration);
            write_optional_string(out, "saving_throw", &fields.saving_throw);
            write_optional_string(out, "spell_resistance", &fields.spell_resistance);
        }
        TypedFields::Feat(fields) => {
            out.push_str("kind = \"feat\"\n");
            write_string_array(out, "feat_types", &fields.feat_types);
            write_optional_string(out, "prerequisites", &fields.prerequisites);
            if !fields.prerequisite_feats.is_empty() {
                let values: Vec<String> = fields
                    .prerequisite_feats
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect();
                write_string_array(out, "prerequisite_feats", &values);
            }
            write_optional_string(out, "benefit", &fields.benefit);
            write_optional_string(out, "normal", &fields.normal);
            write_optional_string(out, "special", &fields.special);
        }
        TypedFields::Creature(fields) => {
            out.push_str("kind = \"creature\"\n");
            if let Some(cr) = fields.cr {
                out.push_str("cr = ");
                out.push_str(&tomlutil::string(&cr.display()));
                out.push('\n');
            }
            write_optional_string(out, "creature_type", &fields.creature_type);
            write_string_array(out, "subtypes", &fields.subtypes);
            write_optional_string(out, "size", &fields.size);
            write_optional_string(out, "alignment", &fields.alignment);
            write_optional_string(out, "ac", &fields.ac);
            write_optional_string(out, "hp", &fields.hp);
            write_optional_string(out, "fortitude", &fields.fortitude);
            write_optional_string(out, "reflex", &fields.reflex);
            write_optional_string(out, "will", &fields.will);
            write_optional_string(out, "melee", &fields.melee);
            write_optional_string(out, "ranged", &fields.ranged);
            write_optional_string(out, "special_attacks", &fields.special_attacks);
            write_optional_string(out, "special_qualities", &fields.special_qualities);
            if let Some(abilities) = &fields.abilities {
                let values: Vec<String> = abilities.to_vec();
                write_string_array(out, "abilities", &values);
            }
        }
        TypedFields::Item(fields) => {
            out.push_str("kind = \"item\"\n");
            write_optional_string(out, "category", &fields.category);
            write_optional_string(out, "price", &fields.price);
            write_optional_string(out, "weight", &fields.weight);
            write_optional_string(out, "slot", &fields.slot);
            write_optional_string(out, "aura", &fields.aura);
            write_optional_string(out, "caster_level", &fields.caster_level);
            write_pairs(out, "attributes", &fields.attributes);
        }
        TypedFields::Class(fields) => {
            out.push_str("kind = \"class\"\n");
            write_optional_string(out, "class_type", &fields.class_type);
            write_optional_string(out, "hit_die", &fields.hit_die);
            write_optional_string(out, "skill_ranks", &fields.skill_ranks);
            write_optional_string(out, "class_skills", &fields.class_skills);
        }
        TypedFields::Trait(fields) => {
            out.push_str("kind = \"trait\"\n");
            out.push_str("category = ");
            out.push_str(&tomlutil::string(&fields.category));
            out.push('\n');
            write_optional_string(out, "detail", &fields.detail);
        }
        TypedFields::RacialTrait(fields) => {
            out.push_str("kind = \"racial_trait\"\n");
            out.push_str("alternate = ");
            out.push_str(if fields.alternate { "true" } else { "false" });
            out.push('\n');
        }
        TypedFields::Archetype(fields) => {
            out.push_str("kind = \"archetype\"\n");
            write_string_array(out, "replaces", &fields.replaces);
            write_string_array(out, "alters", &fields.alters);
        }
        TypedFields::ClassOption(fields) => {
            out.push_str("kind = \"class_option\"\n");
            out.push_str("group = ");
            out.push_str(&tomlutil::string(&fields.group));
            out.push('\n');
            write_string_array(out, "replaces", &fields.replaces);
            write_string_array(out, "alters", &fields.alters);
        }
        TypedFields::None => out.push_str("kind = \"none\"\n"),
    }
}

fn write_optional_string(out: &mut String, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        out.push_str(key);
        out.push_str(" = ");
        out.push_str(&tomlutil::string(value));
        out.push('\n');
    }
}

fn write_pairs(out: &mut String, key: &str, values: &[(String, String)]) {
    if values.is_empty() {
        return;
    }
    out.push_str(key);
    out.push_str(" = [\n");
    for (label, value) in values {
        out.push_str("  { label = ");
        out.push_str(&tomlutil::string(label));
        out.push_str(", value = ");
        out.push_str(&tomlutil::string(value));
        out.push_str(" },\n");
    }
    out.push_str("]\n");
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFront {
    id: String,
    kind: String,
    title: String,
    #[serde(default)]
    aon: Option<String>,
    #[serde(default)]
    source_ids: Vec<String>,
    #[serde(default)]
    section: Vec<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    provenance: Vec<Provenance>,
    #[serde(default)]
    license_refs: Option<Vec<String>>,
    #[serde(default)]
    relations: Vec<Relation>,
    #[serde(default)]
    sources: Vec<RawSource>,
    fields: toml::Table,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    publication: String,
    #[serde(default)]
    page: Option<String>,
}

pub fn read_entry(path: &Path, source: &str, config: &Config) -> Result<RuleEntry> {
    if let Some(index) = source.find('\r') {
        let line = 1 + source[..index]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        return Err(Error::new(
            path,
            line,
            "the file holds a carriage return (a Windows line end); save it with LF line ends. A literal carriage return in text is written `\\R`",
        ));
    }
    let lines: Vec<&str> = source.lines().collect();
    if lines.first().map(|line| line.trim_end()) != Some("+++") {
        return Err(Error::new(path, 1, "an entry file must start with `+++`"));
    }
    let close = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim_end() == "+++")
        .map(|(index, _)| index)
        .ok_or_else(|| Error::new(path, 1, "front matter is not closed with `+++`"))?;
    let front = lines[1..close].join("\n");
    let raw: RawFront = toml::from_str(&front).map_err(|error| {
        Error::new(
            path,
            2 + toml_error_line(&error, &front),
            format!("front matter: {error}"),
        )
    })?;

    let id = EntryId::parse(raw.id.clone())
        .map_err(|e| Error::new(path, key_line(&front, "id"), e.to_string()))?;
    let expected = entry_path(&id);
    if !normalized(path).ends_with(&expected) {
        return Err(Error::new(
            path,
            key_line(&front, "id"),
            format!(
                "entry id `{id}` belongs at `{}`, not `{}`",
                expected.display(),
                normalized(path).display()
            ),
        ));
    }
    let kind = EntryKind::from_slug(&raw.kind).ok_or_else(|| {
        Error::new(
            path,
            key_line(&front, "kind"),
            format!("unknown entry kind {:?}", raw.kind),
        )
    })?;
    let parent = raw
        .parent
        .as_deref()
        .map(EntryId::parse)
        .transpose()
        .map_err(|e| Error::new(path, key_line(&front, "parent"), e.to_string()))?;
    let fields = parse_fields(path, &front, &raw.fields)?;
    let relations = merge_relations(&fields, raw.relations);
    let sources: Vec<Citation> = raw
        .sources
        .into_iter()
        .map(|source| Citation {
            publication: source.publication,
            page: source.page,
        })
        .collect();

    let body_lines = &lines[close + 1..];
    let body_start = close + 2;
    let numbered: Vec<(usize, &str)> = body_lines
        .iter()
        .enumerate()
        .map(|(index, text)| (body_start + index, *text))
        .collect();
    let body = parse_blocks(path, &numbered)?;
    let links = derived_links(&body);

    Ok(RuleEntry {
        id,
        kind,
        title: raw.title,
        aon: raw.aon,
        source_ids: raw.source_ids,
        sources,
        section: raw.section,
        parent,
        body,
        fields,
        tags: raw.tags,
        links,
        provenance: raw.provenance,
        license_refs: raw
            .license_refs
            .unwrap_or_else(|| config.license_refs.clone()),
        relations,
    })
}

fn normalized(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}

fn parse_fields(path: &Path, front: &str, table: &toml::Table) -> Result<TypedFields> {
    let fields_line = front
        .lines()
        .position(|line| line.trim() == "[fields]")
        .map(|index| index + 2)
        .unwrap_or(2);
    let kind = table
        .get("kind")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::new(path, fields_line, "[fields] has no `kind`"))?;
    let allowed: &[&str] = match kind {
        "spell" => &[
            "kind",
            "school",
            "subschools",
            "descriptors",
            "levels",
            "casting_time",
            "components",
            "range",
            "effects",
            "duration",
            "saving_throw",
            "spell_resistance",
        ],
        "feat" => &[
            "kind",
            "feat_types",
            "prerequisites",
            "prerequisite_feats",
            "benefit",
            "normal",
            "special",
        ],
        "creature" => &[
            "kind",
            "cr",
            "creature_type",
            "subtypes",
            "size",
            "alignment",
            "ac",
            "hp",
            "fortitude",
            "reflex",
            "will",
            "melee",
            "ranged",
            "special_attacks",
            "special_qualities",
            "abilities",
        ],
        "item" => &[
            "kind",
            "category",
            "price",
            "weight",
            "slot",
            "aura",
            "caster_level",
            "attributes",
        ],
        "class" => &[
            "kind",
            "class_type",
            "hit_die",
            "skill_ranks",
            "class_skills",
        ],
        "trait" => &["kind", "category", "detail"],
        "racial_trait" => &["kind", "alternate"],
        "archetype" => &["kind", "replaces", "alters"],
        "class_option" => &["kind", "group", "replaces", "alters"],
        "none" => &["kind"],
        other => {
            return Err(Error::new(
                path,
                key_line(front, "kind"),
                format!("unknown typed-fields kind {other:?}"),
            ));
        }
    };
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(Error::new(
                path,
                key_line(front, key),
                format!("unexpected field `{key}` for `{kind}`"),
            ));
        }
    }
    let line = |key: &str| key_line(front, key);
    let string = |key: &str| required_string(path, table, key, line(key));
    let list = |key: &str| string_list(path, table, key, line(key));
    let optional = |key: &str| optional_string(path, table, key, line(key));
    Ok(match kind {
        "spell" => TypedFields::Spell(SpellFields {
            school: string("school")?,
            subschools: list("subschools")?,
            descriptors: list("descriptors")?,
            levels: parse_levels(path, table, line("levels"))?,
            casting_time: optional("casting_time")?,
            components: optional("components")?,
            range: optional("range")?,
            effects: parse_pairs(path, table, "effects", line("effects"))?,
            duration: optional("duration")?,
            saving_throw: optional("saving_throw")?,
            spell_resistance: optional("spell_resistance")?,
        }),
        "feat" => TypedFields::Feat(FeatFields {
            feat_types: list("feat_types")?,
            prerequisites: optional("prerequisites")?,
            prerequisite_feats: string_list(
                path,
                table,
                "prerequisite_feats",
                line("prerequisite_feats"),
            )?
            .into_iter()
            .map(|value| {
                EntryId::parse(value)
                    .map_err(|e| Error::new(path, line("prerequisite_feats"), e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?,
            benefit: optional("benefit")?,
            normal: optional("normal")?,
            special: optional("special")?,
        }),
        "creature" => TypedFields::Creature(Box::new(CreatureFields {
            cr: optional("cr")?
                .map(|value| {
                    ChallengeRating::parse(&value).ok_or_else(|| {
                        Error::new(
                            path,
                            line("cr"),
                            format!("unreadable challenge rating {value:?}"),
                        )
                    })
                })
                .transpose()?,
            creature_type: optional("creature_type")?,
            subtypes: list("subtypes")?,
            size: optional("size")?,
            alignment: optional("alignment")?,
            ac: optional("ac")?,
            hp: optional("hp")?,
            fortitude: optional("fortitude")?,
            reflex: optional("reflex")?,
            will: optional("will")?,
            melee: optional("melee")?,
            ranged: optional("ranged")?,
            special_attacks: optional("special_attacks")?,
            special_qualities: optional("special_qualities")?,
            abilities: parse_abilities(path, table, line("abilities"))?,
        })),
        "item" => TypedFields::Item(ItemFields {
            category: optional("category")?,
            price: optional("price")?,
            weight: optional("weight")?,
            slot: optional("slot")?,
            aura: optional("aura")?,
            caster_level: optional("caster_level")?,
            attributes: parse_pairs(path, table, "attributes", line("attributes"))?,
        }),
        "class" => TypedFields::Class(cast_iron_schema::ClassFields {
            class_type: optional("class_type")?,
            hit_die: optional("hit_die")?,
            skill_ranks: optional("skill_ranks")?,
            class_skills: optional("class_skills")?,
        }),
        "trait" => TypedFields::Trait(cast_iron_schema::TraitFields {
            category: string("category")?,
            detail: optional("detail")?,
        }),
        "racial_trait" => TypedFields::RacialTrait(cast_iron_schema::RacialTraitFields {
            alternate: table
                .get("alternate")
                .and_then(toml::Value::as_bool)
                .ok_or_else(|| {
                    Error::new(
                        path,
                        line("alternate"),
                        "racial traits need `alternate = true|false`",
                    )
                })?,
        }),
        "archetype" => TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
            replaces: list("replaces")?,
            alters: list("alters")?,
        }),
        "class_option" => TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
            group: string("group")?,
            replaces: list("replaces")?,
            alters: list("alters")?,
        }),
        "none" => TypedFields::None,
        _ => unreachable!("kind was checked above"),
    })
}

fn required_string(path: &Path, table: &toml::Table, key: &str, line: usize) -> Result<String> {
    match table.get(key) {
        Some(toml::Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(Error::new(path, line, format!("`{key}` must be a string"))),
        None => Err(Error::new(
            path,
            line,
            format!("missing required field `{key}`"),
        )),
    }
}

fn optional_string(
    path: &Path,
    table: &toml::Table,
    key: &str,
    line: usize,
) -> Result<Option<String>> {
    match table.get(key) {
        Some(toml::Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(Error::new(path, line, format!("`{key}` must be a string"))),
        None => Ok(None),
    }
}

fn string_list(path: &Path, table: &toml::Table, key: &str, line: usize) -> Result<Vec<String>> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    let toml::Value::Array(values) = value else {
        return Err(Error::new(
            path,
            line,
            format!("`{key}` must be an array of strings"),
        ));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| Error::new(path, line, format!("`{key}` must hold strings only")))
        })
        .collect()
}

fn parse_levels(path: &Path, table: &toml::Table, line: usize) -> Result<Vec<SpellLevel>> {
    let Some(value) = table.get("levels") else {
        return Ok(Vec::new());
    };
    let toml::Value::Array(values) = value else {
        return Err(Error::new(
            path,
            line,
            "`levels` must be an array of tables",
        ));
    };
    values
        .iter()
        .map(|value| {
            let table = value
                .as_table()
                .ok_or_else(|| Error::new(path, line, "`levels` must hold tables"))?;
            let class = table
                .get("class")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| Error::new(path, line, "a spell level needs `class`"))?
                .to_string();
            let level = table
                .get("level")
                .and_then(toml::Value::as_integer)
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(|| Error::new(path, line, "a spell level needs an integer `level`"))?;
            for key in table.keys() {
                if key != "class" && key != "level" {
                    return Err(Error::new(
                        path,
                        line,
                        format!("unexpected key `{key}` in a spell level"),
                    ));
                }
            }
            Ok(SpellLevel { class, level })
        })
        .collect()
}

fn parse_pairs(
    path: &Path,
    table: &toml::Table,
    key: &str,
    line: usize,
) -> Result<Vec<(String, String)>> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    let toml::Value::Array(values) = value else {
        return Err(Error::new(
            path,
            line,
            format!("`{key}` must be an array of `{{ label, value }}` tables"),
        ));
    };
    values
        .iter()
        .map(|value| {
            let table = value
                .as_table()
                .ok_or_else(|| Error::new(path, line, format!("`{key}` must hold tables")))?;
            let label = table
                .get("label")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| Error::new(path, line, format!("a `{key}` row needs `label`")))?
                .to_string();
            let value = table
                .get("value")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| Error::new(path, line, format!("a `{key}` row needs `value`")))?
                .to_string();
            for extra in table.keys() {
                if extra != "label" && extra != "value" {
                    return Err(Error::new(
                        path,
                        line,
                        format!("unexpected key `{extra}` in a `{key}` row"),
                    ));
                }
            }
            Ok((label, value))
        })
        .collect()
}

fn parse_abilities(path: &Path, table: &toml::Table, line: usize) -> Result<Option<[String; 6]>> {
    let Some(value) = table.get("abilities") else {
        return Ok(None);
    };
    let toml::Value::Array(values) = value else {
        return Err(Error::new(
            path,
            line,
            "`abilities` must be an array of six strings",
        ));
    };
    if values.len() != 6 {
        return Err(Error::new(
            path,
            line,
            format!("`abilities` has {} values; expected 6", values.len()),
        ));
    }
    let mut out: [String; 6] = Default::default();
    for (slot, value) in out.iter_mut().zip(values) {
        *slot = value
            .as_str()
            .ok_or_else(|| Error::new(path, line, "`abilities` must hold strings only"))?
            .to_string();
    }
    Ok(Some(out))
}

/// The 1-based file line of a top-level front-matter key.
fn key_line(front: &str, key: &str) -> usize {
    front
        .lines()
        .position(|line| {
            let line = line.trim_start();
            line == "[fields]" || line.starts_with(&format!("{key} ="))
        })
        .map(|index| index + 2)
        .unwrap_or(2)
}

fn toml_error_line(error: &toml::de::Error, text: &str) -> usize {
    let Some(span) = error.span() else {
        return 0;
    };
    let end = span.start.min(text.len());
    text[..end].bytes().filter(|byte| *byte == b'\n').count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::write_blocks;
    use cast_iron_schema::{ParagraphStyle, StatRow, Table, TableCell, TableRow};

    fn entry() -> RuleEntry {
        RuleEntry {
            id: EntryId::parse("ref/spell/fog-cloud").unwrap(),
            kind: EntryKind::Spell,
            title: "Fog Cloud".into(),
            aon: None,
            source_ids: vec!["core-rulebook".into()],
            sources: vec![Citation {
                publication: "core-rulebook".into(),
                page: Some("221".into()),
            }],
            section: vec!["Spells".into()],
            parent: Some(EntryId::parse("ref/spell").unwrap()),
            body: vec![
                Block::Heading {
                    level: 2,
                    anchor: Some("effects".into()),
                    content: vec![Inline::text("Effects")],
                },
                Block::Paragraph {
                    style: Default::default(),
                    content: vec![
                        Inline::text("A "),
                        Inline::Em {
                            c: vec![Inline::text("dense")],
                        },
                        Inline::text(" cloud. "),
                        Inline::Strong {
                            c: vec![Inline::text("Strong")],
                        },
                        Inline::Break,
                        Inline::text("A pipe | and a star *."),
                    ],
                },
                Block::List {
                    ordered: false,
                    items: vec![
                        vec![Block::Paragraph {
                            style: Default::default(),
                            content: vec![Inline::text("one")],
                        }],
                        vec![
                            Block::Paragraph {
                                style: Default::default(),
                                content: vec![Inline::text("two")],
                            },
                            Block::List {
                                ordered: true,
                                items: vec![vec![Block::Paragraph {
                                    style: Default::default(),
                                    content: vec![Inline::text("nested")],
                                }]],
                            },
                        ],
                    ],
                },
                Block::Quote {
                    content: vec![Block::Paragraph {
                        style: Default::default(),
                        content: vec![Inline::text("quoted")],
                    }],
                },
                Block::StatBlock {
                    title: Some("Goblin".into()),
                    rows: vec![StatRow {
                        label: "AC".into(),
                        value: vec![Inline::text("15")],
                    }],
                },
                Block::EntryLink {
                    target: EntryId::parse("ref/spell/light").unwrap(),
                    title: "Light".into(),
                },
                Block::Table(Table {
                    caption: vec![Inline::text("A table")],
                    head: vec![TableRow {
                        cells: vec![TableCell {
                            header: true,
                            colspan: 1,
                            rowspan: 1,
                            content: vec![Inline::text("Name")],
                        }],
                    }],
                    body: vec![TableRow {
                        cells: vec![TableCell {
                            content: vec![Inline::text("Sword")],
                            colspan: 1,
                            rowspan: 1,
                            header: false,
                        }],
                    }],
                    foot: vec![],
                }),
            ],
            fields: TypedFields::Spell(SpellFields {
                school: "conjuration".into(),
                subschools: vec!["creation".into()],
                descriptors: vec![],
                levels: vec![SpellLevel {
                    class: "Sorcerer".into(),
                    level: 2,
                }],
                casting_time: Some("1 standard action".into()),
                components: Some("V, S".into()),
                range: Some("medium".into()),
                effects: vec![("Effect".into(), "fog".into())],
                duration: Some("1 min./level".into()),
                saving_throw: None,
                spell_resistance: None,
            }),
            tags: vec!["fog".into()],
            links: vec![],
            provenance: vec![Provenance {
                dataset: "pathfinder-data-1-e".into(),
                record: "fog_cloud".into(),
                path: "json/spells.json".into(),
            }],
            license_refs: vec!["ogl-1.0a".into()],
            relations: vec![Relation {
                kind: RelationKind::Part,
                target: EntryId::parse("ref/rule/fog").unwrap(),
                label: Some("or".into()),
            }],
        }
    }

    #[test]
    fn an_entry_round_trips_and_recomputes_links() {
        let mut entry = entry();
        let path = entry_path(&entry.id);
        let text = write_entry(&entry, &Config::default());
        entry.links = derived_links(&entry.body);
        assert_eq!(read_entry(&path, &text, &Config::default()).unwrap(), entry);
    }

    #[test]
    fn a_mismatched_path_is_rejected_with_a_line() {
        let entry = entry();
        let text = write_entry(&entry, &Config::default());
        let error = read_entry(
            Path::new("entries/spell/not-fog-cloud.md"),
            &text,
            &Config::default(),
        )
        .unwrap_err();
        assert!(error.message.contains("belongs at"), "{error}");
        assert!(error.line >= 1);
    }

    #[test]
    fn unknown_front_matter_keys_are_rejected() {
        let entry = entry();
        let text =
            write_entry(&entry, &Config::default()).replace("title = ", "bogus = true\ntitle = ");
        let error = read_entry(&entry_path(&entry.id), &text, &Config::default()).unwrap_err();
        assert!(error.message.contains("front matter"), "{error}");
    }

    #[test]
    fn unknown_typed_fields_are_rejected() {
        let entry = entry();
        let text =
            write_entry(&entry, &Config::default()).replace("school = ", "nonsense = 1\nschool = ");
        let error = read_entry(&entry_path(&entry.id), &text, &Config::default()).unwrap_err();
        assert!(error.message.contains("unexpected field"), "{error}");
    }

    #[test]
    fn empty_body_is_valid() {
        let mut entry = entry();
        entry.body.clear();
        entry.links.clear();
        let text = write_entry(&entry, &Config::default());
        assert_eq!(
            read_entry(&entry_path(&entry.id), &text, &Config::default()).unwrap(),
            entry
        );
    }

    #[test]
    fn body_text_with_leading_block_markers_survives() {
        let mut entry = entry();
        entry.body = vec![Block::Paragraph {
            style: Default::default(),
            content: vec![Inline::text("# heading\n- item\n::: fence")],
        }];
        entry.links.clear();
        let path = entry_path(&entry.id);
        assert_eq!(
            read_entry(
                &path,
                &write_entry(&entry, &Config::default()),
                &Config::default()
            )
            .unwrap(),
            entry
        );
    }

    #[test]
    fn body_writer_never_emits_a_trailing_blank_line() {
        let entry = entry();
        assert!(!write_blocks(&entry.body).ends_with('\n'));
    }

    fn round_trip_entry(entry: RuleEntry) {
        let path = entry_path(&entry.id);
        let text = write_entry(&entry, &Config::default());
        let mut expected = entry;
        expected.links = derived_links(&expected.body);
        assert_eq!(
            read_entry(&path, &text, &Config::default()).unwrap(),
            expected,
            "source:\n{text}"
        );
    }

    #[test]
    fn every_typed_fields_variant_round_trips() {
        let variants = vec![
            TypedFields::Feat(FeatFields {
                feat_types: vec!["Combat".into()],
                prerequisites: Some("Dodge".into()),
                prerequisite_feats: vec![EntryId::parse("ref/feat/dodge").unwrap()],
                benefit: Some("benefit".into()),
                normal: Some("normal".into()),
                special: Some("special".into()),
            }),
            TypedFields::Creature(Box::new(CreatureFields {
                cr: Some(ChallengeRating::parse("1/2").unwrap()),
                creature_type: Some("humanoid".into()),
                subtypes: vec!["human".into()],
                size: Some("medium".into()),
                alignment: Some("neutral".into()),
                ac: Some("12".into()),
                hp: Some("5".into()),
                fortitude: Some("+1".into()),
                reflex: Some("+2".into()),
                will: Some("+3".into()),
                melee: Some("sword".into()),
                ranged: Some("bow".into()),
                special_attacks: Some("none".into()),
                special_qualities: Some("darkvision".into()),
                abilities: Some([
                    "10".into(),
                    "11".into(),
                    "12".into(),
                    "13".into(),
                    "14".into(),
                    "15".into(),
                ]),
            })),
            TypedFields::Item(ItemFields {
                category: Some("weapon".into()),
                price: Some("2 gp".into()),
                weight: Some("1 lb.".into()),
                slot: Some("none".into()),
                aura: Some("faint".into()),
                caster_level: Some("3rd".into()),
                attributes: vec![("Special".into(), "value".into())],
            }),
            TypedFields::Class(cast_iron_schema::ClassFields {
                class_type: Some("core".into()),
                hit_die: Some("d10".into()),
                skill_ranks: Some("4".into()),
                class_skills: Some("Climb".into()),
            }),
            TypedFields::Trait(cast_iron_schema::TraitFields {
                category: "Combat".into(),
                detail: Some("Race".into()),
            }),
            TypedFields::RacialTrait(cast_iron_schema::RacialTraitFields { alternate: true }),
            TypedFields::Archetype(cast_iron_schema::ArchetypeFields {
                replaces: vec!["Trap sense".into()],
                alters: vec!["Uncanny dodge".into()],
            }),
            TypedFields::ClassOption(cast_iron_schema::ClassOptionFields {
                group: "arcane-discovery".into(),
                replaces: vec!["Arcane bond".into()],
                alters: vec!["Arcane school".into()],
            }),
            TypedFields::None,
        ];
        for fields in variants {
            let mut entry = entry();
            if let TypedFields::Feat(feat) = &fields {
                // A feat's prerequisite relations mirror its prerequisite_feats list.
                entry.relations = feat
                    .prerequisite_feats
                    .iter()
                    .map(|target| prerequisite(target.as_str(), None))
                    .chain(entry.relations)
                    .collect();
            }
            entry.fields = fields;
            round_trip_entry(entry);
        }
    }

    #[test]
    fn every_block_and_inline_form_round_trips_through_a_full_entry() {
        for body in [
            vec![Block::Paragraph {
                style: Default::default(),
                content: vec![
                    Inline::text("a * b _ c ^ d ~ e [f] (g) | h {i} <j> > k \\ l"),
                    Inline::Em {
                        c: vec![
                            Inline::text("outer"),
                            Inline::Em {
                                c: vec![Inline::text("inner")],
                            },
                        ],
                    },
                    Inline::Strong {
                        c: vec![Inline::text("strong")],
                    },
                    Inline::Sup {
                        c: vec![Inline::text("1")],
                    },
                    Inline::Sub {
                        c: vec![Inline::text("2")],
                    },
                    Inline::Break,
                    Inline::Link {
                        target: EntryId::parse("ref/spell/light").unwrap(),
                        anchor: Some("effects".into()),
                        c: vec![Inline::text("light")],
                    },
                ],
            }],
            vec![Block::Heading {
                level: 2,
                anchor: Some("anchor".into()),
                content: vec![Inline::text("heading")],
            }],
            vec![Block::List {
                ordered: true,
                items: vec![vec![Block::Paragraph {
                    style: ParagraphStyle::Indented,
                    content: vec![Inline::text("item")],
                }]],
            }],
            vec![Block::Quote {
                content: vec![Block::Paragraph {
                    style: ParagraphStyle::StatLine,
                    content: vec![Inline::text("quote")],
                }],
            }],
            vec![Block::StatBlock {
                title: None,
                rows: vec![StatRow {
                    label: "a:b".into(),
                    value: vec![Inline::text("c")],
                }],
            }],
            vec![Block::EntryLink {
                target: EntryId::parse("ref/spell/light").unwrap(),
                title: "Light".into(),
            }],
        ] {
            let mut entry = entry();
            entry.body = body;
            round_trip_entry(entry);
        }
    }

    #[test]
    fn a_front_matter_parse_error_names_a_line() {
        let entry = entry();
        let text = write_entry(&entry, &Config::default()).replace("school = ", "school = [\n");
        let error = read_entry(&entry_path(&entry.id), &text, &Config::default()).unwrap_err();
        assert!(error.line >= 2, "{error}");
    }

    #[test]
    fn unknown_kind_and_bad_id_are_rejected() {
        let entry = entry();
        let text =
            write_entry(&entry, &Config::default()).replace("kind = \"spell\"", "kind = \"nope\"");
        assert!(read_entry(&entry_path(&entry.id), &text, &Config::default()).is_err());
        let text = write_entry(&entry, &Config::default())
            .replace("id = \"ref/spell/fog-cloud\"", "id = \"Bad Id\"");
        assert!(read_entry(&entry_path(&entry.id), &text, &Config::default()).is_err());
    }

    #[test]
    fn a_body_error_names_a_line() {
        let entry = entry();
        let text =
            write_entry(&entry, &Config::default()).replace("## Effects {#effects}", "## Effects");
        assert!(read_entry(&entry_path(&entry.id), &text, &Config::default()).is_ok());
        let text = write_entry(&entry, &Config::default())
            .replace("## Effects {#effects}", "## Effects {#bad anchor}");
        let error = read_entry(&entry_path(&entry.id), &text, &Config::default()).unwrap_err();
        assert!(error.message.contains("invalid heading anchor"), "{error}");
        let text =
            write_entry(&entry, &Config::default()).replace("A pipe \\| and a star \\*.", "*open");
        let error = read_entry(&entry_path(&entry.id), &text, &Config::default()).unwrap_err();
        assert!(error.line >= 1, "{error}");
    }

    fn round_trip_with(entry: &RuleEntry, config: &Config) -> String {
        let text = write_entry(entry, config);
        let mut expected = entry.clone();
        expected.links = derived_links(&expected.body);
        assert_eq!(
            read_entry(&entry_path(&entry.id), &text, config).unwrap(),
            expected,
            "{text}"
        );
        text
    }

    #[test]
    fn default_licence_refs_are_not_written_and_a_difference_is() {
        let config = Config::default();
        let mut entry = entry();
        entry.license_refs = config.license_refs.clone();
        let text = round_trip_with(&entry, &config);
        assert!(!text.contains("license_refs"), "{text}");

        entry.license_refs = vec!["ogl-1.0a".into()];
        let text = round_trip_with(&entry, &config);
        assert!(text.contains("license_refs = [\"ogl-1.0a\"]"), "{text}");

        entry.license_refs = vec![];
        let text = round_trip_with(&entry, &config);
        assert!(text.contains("license_refs = []"), "{text}");
    }

    #[test]
    fn the_default_comes_from_the_corpus_config() {
        let config = Config {
            format: 1,
            license_refs: vec!["other".into()],
        };
        let mut entry = entry();
        entry.license_refs = vec!["other".into()];
        let text = round_trip_with(&entry, &config);
        assert!(!text.contains("license_refs"), "{text}");
        // Read with the standard defaults, the same file means something else.
        let read = read_entry(&entry_path(&entry.id), &text, &Config::default()).unwrap();
        assert_eq!(read.license_refs, Config::default().license_refs);
    }

    fn feat_with_prerequisites(relations: Vec<Relation>) -> RuleEntry {
        let mut entry = entry();
        entry.id = EntryId::parse("ref/feat/power-attack").unwrap();
        entry.kind = EntryKind::Feat;
        entry.fields = TypedFields::Feat(FeatFields {
            prerequisite_feats: vec![
                EntryId::parse("ref/feat/a").unwrap(),
                EntryId::parse("ref/feat/b").unwrap(),
            ],
            ..Default::default()
        });
        entry.relations = relations;
        entry
    }

    fn prerequisite(target: &str, label: Option<&str>) -> Relation {
        Relation {
            kind: RelationKind::Prerequisite,
            target: EntryId::parse(target).unwrap(),
            label: label.map(str::to_string),
        }
    }

    #[test]
    fn prerequisite_relations_are_derived_from_prerequisite_feats() {
        let entry = feat_with_prerequisites(vec![
            prerequisite("ref/feat/a", None),
            prerequisite("ref/feat/b", None),
        ]);
        let text = round_trip_with(&entry, &Config::default());
        assert!(!text.contains("relations"), "{text}");
        assert!(!text.contains("prerequisite\""), "{text}");
    }

    #[test]
    fn a_labelled_prerequisite_is_the_only_one_stored() {
        let entry = feat_with_prerequisites(vec![
            prerequisite("ref/feat/a", None),
            prerequisite("ref/feat/b", Some("or")),
        ]);
        let text = round_trip_with(&entry, &Config::default());
        assert!(text.contains("kind = \"prerequisite\", target = \"ref/feat/b\", label = \"or\""));
        assert!(!text.contains("target = \"ref/feat/a\""), "{text}");
    }

    #[test]
    fn other_relation_kinds_and_stray_prerequisites_stay_stored() {
        let mut feat = feat_with_prerequisites(vec![
            prerequisite("ref/feat/a", None),
            prerequisite("ref/feat/b", None),
            prerequisite("ref/feat/not-listed", None),
        ]);
        feat.relations.push(Relation {
            kind: RelationKind::Archetype,
            target: EntryId::parse("ref/class/fighter").unwrap(),
            label: None,
        });
        feat.relations.push(Relation {
            kind: RelationKind::Addon,
            target: EntryId::parse("ref/feat/power-attack").unwrap(),
            label: Some("mythic".into()),
        });
        let text = round_trip_with(&feat, &Config::default());
        assert!(text.contains("ref/feat/not-listed"), "{text}");
        assert!(text.contains("ref/class/fighter"), "{text}");
        assert!(
            text.contains(
                "kind = \"addon\", target = \"ref/feat/power-attack\", label = \"mythic\""
            ),
            "{text}"
        );
        // A non-feat has no list to derive from.
        let mut other = self::entry();
        other.relations = vec![prerequisite("ref/feat/a", None)];
        let text = round_trip_with(&other, &Config::default());
        assert!(text.contains("ref/feat/a"), "{text}");
    }

    #[test]
    fn sources_are_written_as_repeated_tables_before_fields() {
        let mut entry = entry();
        entry.sources = vec![
            Citation {
                publication: "core-rulebook".into(),
                page: Some("221".into()),
            },
            Citation {
                publication: "ultimate-combat".into(),
                page: None,
            },
        ];
        let text = round_trip_with(&entry, &Config::default());
        assert!(
            text.contains(
                "[[sources]]\npublication = \"core-rulebook\"\npage = \"221\"\n\
                 [[sources]]\npublication = \"ultimate-combat\"\n\n[fields]"
            ),
            "{text}"
        );
    }

    #[test]
    fn an_entry_without_sources_writes_no_sources_table() {
        let mut entry = entry();
        entry.sources.clear();
        let text = round_trip_with(&entry, &Config::default());
        assert!(!text.contains("[[sources]]"), "{text}");
    }

    #[test]
    fn provenance_is_optional() {
        let mut entry = entry();
        entry.provenance.clear();
        let text = round_trip_with(&entry, &Config::default());
        assert!(!text.contains("provenance"), "{text}");
    }

    #[test]
    fn aon_is_optional_and_round_trips() {
        let mut entry = entry();
        let text = round_trip_with(&entry, &Config::default());
        assert!(!text.contains("aon"), "{text}");

        entry.aon = Some("SpellDisplay.aspx?ItemName=Fog%20Cloud".into());
        let text = round_trip_with(&entry, &Config::default());
        assert!(
            text.contains("aon = \"SpellDisplay.aspx?ItemName=Fog%20Cloud\""),
            "{text}"
        );
        // The key sits with the identity fields, before the source list.
        let aon = text.find("aon = ").unwrap();
        let sources = text.find("[[sources]]").unwrap();
        assert!(aon < sources, "{text}");
    }
}
